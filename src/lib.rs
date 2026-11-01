use std::collections::HashMap;

use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use tiny_http::{Header, Request, Response, Server, StatusCode};
use url::Url;

/// A release asset reported by the fake GitHub server.
#[derive(Clone, Debug, Default)]
pub struct ReleaseAsset {
    pub name: String,
}

/// A release reported by the fake GitHub server.
#[derive(Clone, Debug, Default)]
pub struct Release {
    pub tag_name: String,
    pub prerelease: bool,
    pub body: Option<String>,
    pub assets: Vec<ReleaseAsset>,
}

/// A GitHub issue reported by the fake server.
#[derive(Clone, Debug, Default)]
pub struct Issue {
    pub number: u64,
    pub id: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: String,
    pub labels: Vec<String>,
}

/// A comment on a fake issue.
#[derive(Clone, Debug, Default)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub user: String,
}

#[derive(Clone, Debug)]
struct FakeAsset {
    id: u64,
    name: String,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct FakeRelease {
    id: u64,
    tag_name: String,
    name: String,
    body: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<FakeAsset>,
}

#[derive(Clone, Debug)]
struct FakeCommit {
    sha: String,
    message: String,
    parents: Vec<String>,
}

#[derive(Clone, Debug)]
struct FakeIssue {
    id: u64,
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    user: String,
    labels: Vec<String>,
}

#[derive(Clone, Debug)]
struct FakeIssueComment {
    id: u64,
    body: String,
    user: String,
}

#[derive(Default)]
struct FakeState {
    tags: HashMap<String, String>,
    commits: HashMap<String, FakeCommit>,
    releases: Vec<FakeRelease>,
    issues: Vec<FakeIssue>,
    issue_comments: HashMap<u64, Vec<FakeIssueComment>>,
    next_id: u64,
    next_issue_number: u64,
}

impl FakeState {
    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}

fn release_json(release: &FakeRelease, base_url: &str) -> Value {
    json!({
        "id": release.id,
        "tag_name": release.tag_name,
        "name": release.name,
        "body": release.body,
        "draft": release.draft,
        "prerelease": release.prerelease,
        "html_url": format!("{base_url}/local/releases/{}", release.tag_name),
        "upload_url": format!("{base_url}/upload/{}{{?name,label}}", release.id),
        "assets": release
            .assets
            .iter()
            .map(asset_json)
            .collect::<Vec<_>>(),
    })
}

fn parse_json_body(body: &[u8]) -> Result<Value> {
    if body.is_empty() {
        Ok(Value::Object(Default::default()))
    } else {
        Ok(serde_json::from_slice(body)?)
    }
}

fn asset_json(asset: &FakeAsset) -> Value {
    json!({
        "id": asset.id,
        "name": asset.name,
        "size": asset.bytes.len(),
        "content_type": "application/octet-stream",
        "browser_download_url": "",
    })
}

/// The subset of the GitHub API the toolkit uses, in memory. It makes the
/// release contract testable without a network or a GitHub repository, while
/// the live harness remains the fidelity check.
pub struct FakeGitHub {
    state: Arc<Mutex<FakeState>>,
    server: Option<Arc<Server>>,
    thread: Option<JoinHandle<()>>,
    url: String,
}

impl FakeGitHub {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState::default())),
            server: None,
            thread: None,
            url: String::new(),
        }
    }

    pub fn start(&mut self) -> Result<String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).context("binding the fake GitHub")?;
        let address = listener.local_addr()?;
        let server = Arc::new(
            Server::from_listener(listener, None)
                .map_err(|error| anyhow::anyhow!("starting the fake GitHub: {error}"))?,
        );
        self.url = format!("http://127.0.0.1:{}", address.port());

        let state = Arc::clone(&self.state);
        let base_url = self.url.clone();
        let running = Arc::clone(&server);
        self.thread = Some(thread::spawn(move || {
            for request in running.incoming_requests() {
                if let Err(error) = handle_request(&state, request, &base_url) {
                    eprintln!("fake: {error:#}");
                }
            }
        }));
        self.server = Some(server);
        Ok(self.url.clone())
    }

    pub fn stop(&mut self) {
        if let Some(server) = self.server.take() {
            server.unblock();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn list_releases(&self) -> Vec<Release> {
        let state = self.state.lock().expect("fake state poisoned");
        state
            .releases
            .iter()
            .map(|release| Release {
                tag_name: release.tag_name.clone(),
                prerelease: release.prerelease,
                body: Some(release.body.clone()),
                assets: release
                    .assets
                    .iter()
                    .map(|asset| ReleaseAsset {
                        name: asset.name.clone(),
                    })
                    .collect(),
            })
            .collect()
    }

    pub fn list_tags(&self) -> Vec<String> {
        let state = self.state.lock().expect("fake state poisoned");
        state.tags.keys().cloned().collect()
    }

    pub fn list_issues(&self) -> Vec<Issue> {
        let state = self.state.lock().expect("fake state poisoned");
        state
            .issues
            .iter()
            .map(|issue| Issue {
                number: issue.number,
                id: issue.id,
                title: issue.title.clone(),
                body: issue.body.clone(),
                state: issue.state.clone(),
                user: issue.user.clone(),
                labels: issue.labels.clone(),
            })
            .collect()
    }

    pub fn list_commits(&self) -> Vec<String> {
        let state = self.state.lock().expect("fake state poisoned");
        state.commits.keys().cloned().collect()
    }

    pub fn seed_tag(&self, tag: &str, sha: &str) {
        let mut state = self.state.lock().expect("fake state poisoned");
        state.tags.insert(tag.to_string(), sha.to_string());
        state
            .commits
            .entry(sha.to_string())
            .or_insert_with(|| FakeCommit {
                sha: sha.to_string(),
                message: format!("commit {sha}"),
                parents: Vec::new(),
            });
    }
}

impl Default for FakeGitHub {
    fn default() -> Self {
        Self::new()
    }
}

fn send(request: Request, status: u16, body: Option<Value>) -> Result<()> {
    let (text, is_json) = match body {
        Some(body) => (serde_json::to_string(&body)?, true),
        None => (String::new(), false),
    };
    let mut response = Response::from_string(text).with_status_code(StatusCode(status));
    if is_json {
        response = response.with_header(
            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                .expect("valid header"),
        );
    }
    request.respond(response)?;
    Ok(())
}

fn decode(segment: &str) -> String {
    percent_decode_str(segment).decode_utf8_lossy().to_string()
}

fn handle_request(
    state: &Arc<Mutex<FakeState>>,
    mut request: Request,
    base_url: &str,
) -> Result<()> {
    let url = Url::parse(&format!("http://localhost{}", request.url()))?;
    let method = request.method().as_str().to_string();
    let path = url.path().to_string();
    if std::env::var_os("HARNESS_DEBUG").is_some() {
        eprintln!("fake: {method} {path}");
    }

    let segments: Vec<String> = url
        .path_segments()
        .map(|segments| segments.map(decode).collect())
        .unwrap_or_default();

    let mut body = Vec::new();
    request.as_reader().read_to_end(&mut body)?;

    // Asset upload: POST /upload/{release_id}?name=...
    if method == "POST" && segments.first().map(String::as_str) == Some("upload") {
        let release_id: u64 = segments
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("missing release id"))?
            .parse()?;
        let name = url
            .query_pairs()
            .find(|(key, _)| key == "name")
            .map(|(_, value)| value.to_string())
            .unwrap_or_else(|| "asset".to_string());
        let mut state = state.lock().expect("fake state poisoned");
        let asset_id = state.id();
        let Some(release) = state
            .releases
            .iter_mut()
            .find(|release| release.id == release_id)
        else {
            drop(state);
            return send(
                request,
                404,
                Some(json!({ "message": "no release to upload to" })),
            );
        };
        release.assets.retain(|asset| asset.name != name);
        release.assets.push(FakeAsset {
            id: asset_id,
            name,
            bytes: std::mem::take(&mut body),
        });
        let asset = release.assets.last().expect("just pushed");
        let response = asset_json(asset);
        drop(state);
        return send(request, 201, Some(response));
    }

    // Release by tag: GET /repos/{owner}/{repo}/releases/tags/{tag}
    if method == "GET"
        && segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "releases"
        && segments[4] == "tags"
    {
        let tag = &segments[5];
        let state = state.lock().expect("fake state poisoned");
        let response = state
            .releases
            .iter()
            .find(|release| &release.tag_name == tag)
            .map(|release| release_json(release, base_url));
        drop(state);
        return match response {
            Some(response) => send(request, 200, Some(response)),
            None => send(request, 404, Some(json!({ "message": "Not Found" }))),
        };
    }

    // Releases: GET/POST /repos/{owner}/{repo}/releases
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "releases" {
        if method == "GET" {
            let state = state.lock().expect("fake state poisoned");
            let releases: Vec<Value> = state
                .releases
                .iter()
                .map(|release| release_json(release, base_url))
                .collect();
            drop(state);
            return send(request, 200, Some(Value::Array(releases)));
        }
        if method == "POST" {
            let body = parse_json_body(&body)?;
            let tag = body
                .get("tag_name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if tag.is_empty() {
                return send(
                    request,
                    422,
                    Some(json!({ "message": "tag_name is required" })),
                );
            }
            let mut state = state.lock().expect("fake state poisoned");
            if state.releases.iter().any(|release| release.tag_name == tag) {
                drop(state);
                return send(request, 422, Some(json!({ "message": "already_exists" })));
            }
            if !state.tags.contains_key(&tag) {
                if let Some(target) = body.get("target_commitish").and_then(Value::as_str) {
                    state.tags.insert(tag.clone(), target.to_string());
                }
            }
            let id = state.id();
            let release = FakeRelease {
                id,
                tag_name: tag.clone(),
                name: body
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(&tag)
                    .to_string(),
                body: body
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                draft: body.get("draft").and_then(Value::as_bool).unwrap_or(false),
                prerelease: body
                    .get("prerelease")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                assets: Vec::new(),
            };
            let response = release_json(&release, base_url);
            state.releases.push(release);
            drop(state);
            return send(request, 201, Some(response));
        }
    }

    // A single release: PATCH/DELETE /repos/{owner}/{repo}/releases/{id}
    if segments.len() == 5
        && segments[0] == "repos"
        && segments[3] == "releases"
        && method == "PATCH"
    {
        let id: u64 = segments[4].parse()?;
        let body = parse_json_body(&body)?;
        let mut state = state.lock().expect("fake state poisoned");
        let Some(release) = state.releases.iter_mut().find(|release| release.id == id) else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        if let Some(name) = body.get("name").and_then(Value::as_str) {
            release.name = name.to_string();
        }
        if let Some(value) = body.get("body").and_then(Value::as_str) {
            release.body = value.to_string();
        }
        if let Some(value) = body.get("draft").and_then(Value::as_bool) {
            release.draft = value;
        }
        if let Some(value) = body.get("prerelease").and_then(Value::as_bool) {
            release.prerelease = value;
        }
        let response = release_json(release, base_url);
        drop(state);
        return send(request, 200, Some(response));
    }

    if segments.len() == 5
        && segments[0] == "repos"
        && segments[3] == "releases"
        && method == "DELETE"
    {
        let id: u64 = segments[4].parse()?;
        let mut state = state.lock().expect("fake state poisoned");
        state.releases.retain(|release| release.id != id);
        drop(state);
        return send(request, 204, None);
    }

    // Assets: GET /repos/{owner}/{repo}/releases/{id}/assets
    if segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "releases"
        && segments[5] == "assets"
        && method == "GET"
    {
        let id: u64 = segments[4].parse()?;
        let state = state.lock().expect("fake state poisoned");
        let Some(release) = state.releases.iter().find(|release| release.id == id) else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        let assets: Vec<Value> = release.assets.iter().map(asset_json).collect();
        drop(state);
        return send(request, 200, Some(Value::Array(assets)));
    }

    // Asset delete: DELETE /repos/{owner}/{repo}/releases/assets/{id}
    if segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "releases"
        && segments[4] == "assets"
        && method == "DELETE"
    {
        let id: u64 = segments[5].parse()?;
        let mut state = state.lock().expect("fake state poisoned");
        for release in &mut state.releases {
            release.assets.retain(|asset| asset.id != id);
        }
        drop(state);
        return send(request, 204, None);
    }

    // Tag ref: GET /repos/{owner}/{repo}/git/ref/tags/{tag}
    if segments.len() == 7
        && segments[0] == "repos"
        && segments[3] == "git"
        && segments[4] == "ref"
        && segments[5] == "tags"
        && method == "GET"
    {
        let tag = &segments[6];
        let state = state.lock().expect("fake state poisoned");
        let sha = state.tags.get(tag).cloned();
        drop(state);
        return match sha {
            Some(sha) => send(
                request,
                200,
                Some(json!({
                    "ref": format!("refs/tags/{tag}"),
                    "object": { "sha": sha, "type": "commit" },
                })),
            ),
            None => send(request, 404, Some(json!({ "message": "Not Found" }))),
        };
    }

    // Tag ref delete: DELETE /repos/{owner}/{repo}/git/refs/tags/{tag}
    if segments.len() == 7
        && segments[0] == "repos"
        && segments[3] == "git"
        && segments[4] == "refs"
        && segments[5] == "tags"
        && method == "DELETE"
    {
        let tag = &segments[6];
        let mut state = state.lock().expect("fake state poisoned");
        state.tags.remove(tag);
        drop(state);
        return send(request, 204, None);
    }

    // Tag ref create: POST /repos/{owner}/{repo}/git/refs
    if segments.len() == 5
        && segments[0] == "repos"
        && segments[3] == "git"
        && segments[4] == "refs"
        && method == "POST"
    {
        let body = parse_json_body(&body)?;
        let reference = body
            .get("ref")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let sha = body
            .get("sha")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let tag = reference
            .strip_prefix("refs/tags/")
            .unwrap_or_default()
            .to_string();
        if tag.is_empty() || sha.is_empty() {
            return send(request, 422, Some(json!({ "message": "invalid ref" })));
        }
        let mut state = state.lock().expect("fake state poisoned");
        if state.tags.contains_key(&tag) {
            drop(state);
            return send(
                request,
                422,
                Some(json!({ "message": "Reference already exists" })),
            );
        }
        state.tags.insert(tag, sha.clone());
        drop(state);
        return send(
            request,
            201,
            Some(json!({
                "ref": reference,
                "object": { "sha": sha, "type": "commit" },
            })),
        );
    }

    // Repository metadata: GET /repos/{owner}/{repo}
    if segments.len() == 3 && segments[0] == "repos" && method == "GET" {
        let owner = &segments[1];
        let repo = &segments[2];
        return send(
            request,
            200,
            Some(json!({
                "id": 1,
                "name": repo,
                "full_name": format!("{owner}/{repo}"),
                "owner": { "login": owner },
                "private": false,
                "html_url": format!("https://github.com/{owner}/{repo}"),
                "default_branch": "main"
            })),
        );
    }

    // Commits: GET /repos/{owner}/{repo}/commits
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "commits" && method == "GET" {
        let state = state.lock().expect("fake state poisoned");
        let commits: Vec<Value> = state
            .commits
            .values()
            .map(|commit| json!({
                "sha": commit.sha,
                "commit": { "message": commit.message },
                "parents": commit.parents.iter().map(|p| json!({"sha": p})).collect::<Vec<_>>()
            }))
            .collect();
        drop(state);
        return send(request, 200, Some(Value::Array(commits)));
    }

    // Commit: GET /repos/{owner}/{repo}/commits/{sha}
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "commits" && method == "GET" {
        let sha = &segments[4];
        let state = state.lock().expect("fake state poisoned");
        let commit = state.commits.get(sha).cloned();
        drop(state);
        match commit {
            Some(commit) => {
                return send(
                    request,
                    200,
                    Some(json!({
                        "sha": commit.sha,
                        "commit": { "message": commit.message },
                        "parents": commit.parents.iter().map(|p| json!({"sha": p})).collect::<Vec<_>>()
                    })),
                );
            }
            None => {
                return send(request, 404, Some(json!({ "message": "Not Found" })));
            }
        }
    }

    // Issues: GET /repos/{owner}/{repo}/issues
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "issues" && method == "GET" {
        let state = state.lock().expect("fake state poisoned");
        let issues: Vec<Value> = state
            .issues
            .iter()
            .map(|issue| json!({
                "id": issue.id,
                "number": issue.number,
                "title": issue.title,
                "body": issue.body,
                "state": issue.state,
                "user": { "login": issue.user },
                "labels": issue.labels.iter().map(|l| json!({"name": l})).collect::<Vec<_>>()
            }))
            .collect();
        drop(state);
        return send(request, 200, Some(Value::Array(issues)));
    }

    // Issue: GET /repos/{owner}/{repo}/issues/{number}
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "issues" && method == "GET" {
        let number: u64 = segments[4].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let issue = state.issues.iter().find(|i| i.number == number).cloned();
        drop(state);
        match issue {
            Some(issue) => {
                return send(
                    request,
                    200,
                    Some(json!({
                        "id": issue.id,
                        "number": issue.number,
                        "title": issue.title,
                        "body": issue.body,
                        "state": issue.state,
                        "user": { "login": issue.user },
                        "labels": issue.labels.iter().map(|l| json!({"name": l})).collect::<Vec<_>>()
                    })),
                );
            }
            None => {
                return send(request, 404, Some(json!({ "message": "Not Found" })));
            }
        }
    }

    // Issue create: POST /repos/{owner}/{repo}/issues
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "issues" && method == "POST" {
        let body = parse_json_body(&body)?;
        let title = body.get("title").and_then(Value::as_str).unwrap_or("").to_string();
        let issue_body = body.get("body").and_then(Value::as_str).map(|s| s.to_string());
        let labels: Vec<String> = body
            .get("labels")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
            .unwrap_or_default();
        let mut state = state.lock().expect("fake state poisoned");
        state.next_issue_number += 1;
        let number = state.next_issue_number;
        let id = state.id();
        let issue = FakeIssue {
            id,
            number,
            title: title.clone(),
            body: issue_body.clone(),
            state: "open".to_string(),
            user: "octocat".to_string(),
            labels: labels.clone(),
        };
        let response = json!({
            "id": id,
            "number": number,
            "title": title,
            "body": issue_body,
            "state": "open",
            "user": { "login": "octocat" },
            "labels": labels.iter().map(|l: &String| json!({"name": l})).collect::<Vec<_>>()
        });
        state.issues.push(issue);
        drop(state);
        return send(request, 201, Some(response));
    }

    // Issue update: PATCH /repos/{owner}/{repo}/issues/{number}
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "issues" && method == "PATCH" {
        let number: u64 = segments[4].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let mut state = state.lock().expect("fake state poisoned");
        let Some(issue) = state.issues.iter_mut().find(|i| i.number == number) else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        if let Some(title) = body.get("title").and_then(Value::as_str) {
            issue.title = title.to_string();
        }
        if let Some(value) = body.get("body").and_then(Value::as_str) {
            issue.body = Some(value.to_string());
        }
        if let Some(value) = body.get("state").and_then(Value::as_str) {
            issue.state = value.to_string();
        }
        if let Some(labels) = body.get("labels").and_then(Value::as_array) {
            issue.labels = labels.iter().filter_map(Value::as_str).map(String::from).collect();
        }
        let response = json!({
            "id": issue.id,
            "number": issue.number,
            "title": issue.title,
            "body": issue.body,
            "state": issue.state,
            "user": { "login": issue.user },
            "labels": issue.labels.iter().map(|l| json!({"name": l})).collect::<Vec<_>>()
        });
        drop(state);
        return send(request, 200, Some(response));
    }

    // Issue comments: GET /repos/{owner}/{repo}/issues/{number}/comments
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "issues" && segments[5] == "comments" && method == "GET" {
        let number: u64 = segments[4].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let issue = state.issues.iter().find(|i| i.number == number);
        let Some(issue) = issue else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        let comments = state.issue_comments.get(&issue.number).cloned().unwrap_or_default();
        let arr: Vec<Value> = comments.iter().map(|c| json!({"id": c.id, "body": c.body, "user": {"login": c.user}})).collect();
        drop(state);
        return send(request, 200, Some(Value::Array(arr)));
    }

    // Issue comment create: POST /repos/{owner}/{repo}/issues/{number}/comments
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "issues" && segments[5] == "comments" && method == "POST" {
        let number: u64 = segments[4].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let text = body.get("body").and_then(Value::as_str).unwrap_or("").to_string();
        let mut state = state.lock().expect("fake state poisoned");
        let issue = state.issues.iter().find(|i| i.number == number).cloned();
        let Some(_) = issue else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        let id = state.id();
        let comment = FakeIssueComment { id, body: text.clone(), user: "octocat".to_string() };
        state.issue_comments.entry(number).or_default().push(comment);
        drop(state);
        return send(request, 201, Some(json!({"id": id, "body": text, "user": {"login": "octocat"}})));
    }

    // Annotated tag lookup: the toolkit only needs to know it is not there.
    if segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "git"
        && segments[4] == "tags"
        && method == "GET"
    {
        return send(request, 404, Some(json!({ "message": "Not Found" })));
    }

    send(
        request,
        404,
        Some(json!({ "message": format!("unhandled {method} {path}") })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_github_starts_and_stops() {
        let mut fake = FakeGitHub::new();
        let url = fake.start().unwrap();
        assert!(url.starts_with("http://127.0.0.1:"));
        fake.seed_tag("v1.2.3", "abc");
        assert_eq!(fake.list_tags(), vec!["v1.2.3"]);
        fake.stop();
    }
}
