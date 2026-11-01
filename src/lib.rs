use std::collections::{BTreeMap, HashMap};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use tiny_http::{Header, Request, Response, Server, StatusCode};
use url::Url;

/// A user in the fake GitHub.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

/// An organization in the fake GitHub.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Organization {
    pub login: String,
    pub id: u64,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// A repository in the fake GitHub.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: String,
    pub private: bool,
    pub default_branch: String,
    pub description: Option<String>,
}

/// A release asset reported by the fake GitHub server.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
}

/// A release reported by the fake GitHub server.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub prerelease: bool,
    pub body: Option<String>,
    pub assets: Vec<ReleaseAsset>,
}

/// A GitHub issue reported by the fake server.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IssueComment {
    pub id: u64,
    pub body: String,
    pub user: String,
}

/// A label in the fake GitHub.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Label {
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

/// A milestone in the fake GitHub.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Milestone {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub description: Option<String>,
}

/// A commit status in the fake GitHub.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CommitStatus {
    pub state: String,
    pub context: String,
    pub description: Option<String>,
}

/// A pull request in the fake GitHub.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub id: u64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: String,
    pub head: String,
    pub base: String,
    pub draft: bool,
    pub merged: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeAsset {
    id: u64,
    name: String,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeRelease {
    id: u64,
    tag_name: String,
    name: String,
    body: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<FakeAsset>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeCommit {
    sha: String,
    message: String,
    parents: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeIssue {
    id: u64,
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    user: String,
    labels: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeIssueComment {
    id: u64,
    body: String,
    user: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeLabel {
    name: String,
    color: String,
    description: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeMilestone {
    id: u64,
    number: u64,
    title: String,
    state: String,
    description: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeCommitStatus {
    id: u64,
    state: String,
    description: Option<String>,
    context: String,
    target_url: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeWebhook {
    id: u64,
    url: String,
    events: Vec<String>,
    active: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeRuleset {
    id: u64,
    name: String,
    target: String,
    enforcement: String,
    conditions: serde_json::Value,
    rules: serde_json::Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeCheckRun {
    id: u64,
    name: String,
    head_sha: String,
    status: String,
    conclusion: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeDeployment {
    id: u64,
    sha: String,
    ref_name: String,
    environment: String,
    description: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeWorkflow {
    id: u64,
    name: String,
    path: String,
    state: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeWorkflowRun {
    id: u64,
    name: String,
    workflow_id: u64,
    head_sha: String,
    head_branch: String,
    status: String,
    conclusion: Option<String>,
    event: String,
    run_number: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakeBranchProtection {
    required_status_checks: Vec<String>,
    enforce_admins: bool,
    required_pull_request_reviews: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakePull {
    id: u64,
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    user: String,
    head: String,
    base: String,
    draft: bool,
    merged: bool,
    mergeable: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakePullComment {
    id: u64,
    body: String,
    user: String,
    path: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct FakePullReview {
    id: u64,
    body: Option<String>,
    state: String,
    user: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct RepoState {
    tags: HashMap<String, String>,
    commits: HashMap<String, FakeCommit>,
    releases: Vec<FakeRelease>,
    issues: Vec<FakeIssue>,
    issue_comments: HashMap<u64, Vec<FakeIssueComment>>,
    next_issue_number: u64,
    branches: Vec<String>,
    contents: HashMap<String, Vec<u8>>,
    labels: Vec<FakeLabel>,
    milestones: Vec<FakeMilestone>,
    next_milestone_number: u64,
    commit_statuses: HashMap<String, Vec<FakeCommitStatus>>,
    check_runs: HashMap<String, Vec<FakeCheckRun>>,
    deployments: Vec<FakeDeployment>,
    next_deployment_id: u64,
    workflows: Vec<FakeWorkflow>,
    workflow_runs: Vec<FakeWorkflowRun>,
    next_workflow_run_number: u64,
    branch_protection: HashMap<String, FakeBranchProtection>,
    rulesets: Vec<FakeRuleset>,
    merge_queue: Vec<String>,
    events: Vec<Value>,
    webhooks: Vec<FakeWebhook>,
    next_webhook_id: u64,
    pulls: Vec<FakePull>,
    pull_comments: HashMap<u64, Vec<FakePullComment>>,
    pull_reviews: HashMap<u64, Vec<FakePullReview>>,
    next_pull_number: u64,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct FakeState {
    users: HashMap<String, User>,
    orgs: HashMap<String, Organization>,
    org_rulesets: HashMap<String, Vec<FakeRuleset>>,
    repos: HashMap<String, RepoState>,
    tokens: HashMap<String, String>, // token -> user login
    next_id: u64,
}

impl FakeState {
    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn repo_key(owner: &str, name: &str) -> String {
        format!("{owner}/{name}")
    }
    fn repo(&mut self, owner: &str, name: &str) -> &mut RepoState {
        let key = Self::repo_key(owner, name);
        let rs = self.repos.entry(key).or_default();
        if rs.branches.is_empty() {
            rs.branches.push("main".to_string());
        }
        rs
    }

    fn ensure_repo(&mut self, owner: &str, name: &str) -> String {
        let key = format!("{owner}/{name}");
        if !self.repos.contains_key(&key) {
            let id = self.id();
            let _ = id;
        }
        key
    }

    fn user_for_token(&self, token: &str) -> Option<String> {
        self.tokens.get(token).cloned()
    }
}

fn repo_key_from_segments(segments: &[String]) -> Option<String> {
    if segments.len() >= 3 && segments[0] == "repos" {
        Some(format!("{}/{}", segments[1], segments[2]))
    } else {
        None
    }
}

/// The subset of the GitHub API, in memory. Suitable as a test double and
/// lightweight emulator.
pub struct FakeGitHub {
    state: Arc<Mutex<FakeState>>,
    server: Option<Arc<Server>>,
    thread: Option<JoinHandle<()>>,
    url: String,
    default_owner: String,
    default_repo: String,
}

impl FakeGitHub {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(FakeState::default())),
            server: None,
            thread: None,
            url: String::new(),
            default_owner: "o".to_string(),
            default_repo: "r".to_string(),
        }
    }

    /// Create a fake GitHub with a default repository owner/name.
    pub fn with_default_repo(owner: &str, repo: &str) -> Self {
        let mut s = Self::new();
        s.default_owner = owner.to_string();
        s.default_repo = repo.to_string();
        s
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

    /// Save the current state to a JSON file.
    pub fn save_state(&self, path: &std::path::Path) -> Result<()> {
        let state = self.state.lock().expect("fake state poisoned");
        let text = serde_json::to_string_pretty(&*state).context("serializing state")?;
        drop(state);
        std::fs::write(path, text).context("writing state file")?;
        Ok(())
    }

    /// Replace the current state from a JSON file.
    pub fn load_state(&self, path: &std::path::Path) -> Result<()> {
        let text = std::fs::read_to_string(path).context("reading state file")?;
        let parsed: FakeState = serde_json::from_str(&text).context("parsing state file")?;
        let mut state = self.state.lock().expect("fake state poisoned");
        *state = parsed;
        Ok(())
    }

    // --- user / org management ---

    pub fn add_user(&self, user: User, token: Option<&str>) {
        let mut state = self.state.lock().expect("fake state poisoned");
        if let Some(token) = token {
            state.tokens.insert(token.to_string(), user.login.clone());
        }
        state.users.insert(user.login.clone(), user);
    }

    pub fn add_org(&self, org: Organization) {
        let mut state = self.state.lock().expect("fake state poisoned");
        state.orgs.insert(org.login.clone(), org);
    }

    pub fn create_repo(&self, owner: &str, name: &str) {
        let mut state = self.state.lock().expect("fake state poisoned");
        state.repo(owner, name);
    }

    // --- default-repo helpers (backward compat) ---

    fn default_key(&self) -> String {
        format!("{}/{}", self.default_owner, self.default_repo)
    }

    pub fn list_releases(&self) -> Vec<Release> {
        let state = self.state.lock().expect("fake state poisoned");
        let key = format!("{}/{}", self.default_owner, self.default_repo);
        state
            .repos
            .get(&key)
            .map(|rs| {
                rs.releases
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
            })
            .unwrap_or_default()
    }

    pub fn list_tags(&self) -> Vec<String> {
        let state = self.state.lock().expect("fake state poisoned");
        let key = format!("{}/{}", self.default_owner, self.default_repo);
        state
            .repos
            .get(&key)
            .map(|rs| rs.tags.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn seed_tag(&self, tag: &str, sha: &str) {
        let mut state = self.state.lock().expect("fake state poisoned");
        let key = format!("{}/{}", self.default_owner, self.default_repo);
        let rs = state.repos.entry(key).or_default();
        rs.tags.insert(tag.to_string(), sha.to_string());
        rs.commits
            .entry(sha.to_string())
            .or_insert_with(|| FakeCommit {
                sha: sha.to_string(),
                message: format!("commit {sha}"),
                parents: Vec::new(),
            });
    }

    pub fn list_issues(&self) -> Vec<Issue> {
        let state = self.state.lock().expect("fake state poisoned");
        let key = format!("{}/{}", self.default_owner, self.default_repo);
        state
            .repos
            .get(&key)
            .map(|rs| {
                rs.issues
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
            })
            .unwrap_or_default()
    }

    pub fn list_commits(&self) -> Vec<String> {
        let state = self.state.lock().expect("fake state poisoned");
        let key = format!("{}/{}", self.default_owner, self.default_repo);
        state
            .repos
            .get(&key)
            .map(|rs| rs.commits.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for FakeGitHub {
    fn default() -> Self {
        Self::new()
    }
}

// --- pagination helper ---

fn parse_page(url: &Url) -> (usize, usize) {
    let page = url
        .query_pairs()
        .find(|(k, _)| k == "page")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(1)
        .max(1);
    let per_page = url
        .query_pairs()
        .find(|(k, _)| k == "per_page")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(30)
        .clamp(1, 100);
    (page, per_page)
}

fn paginate<T: Clone>(items: &[T], url: &Url) -> (Vec<T>, Option<String>) {
    let (page, per_page) = parse_page(url);
    let total = items.len();
    let start = (page - 1) * per_page;
    let end = (start + per_page).min(total);
    let slice = if start < total {
        items[start..end].to_vec()
    } else {
        Vec::new()
    };
    // Build a Link header (RFC 5988) when there is more than one page.
    let total_pages = if total == 0 { 1 } else { (total + per_page - 1) / per_page };
    if total_pages <= 1 {
        return (slice, None);
    }
    let base = format!("{}?{}", url.path(), url.query().unwrap_or(""));
    // strip existing page param crudely by rebuilding
    let mut links = Vec::new();
    if page > 1 {
        links.push(format!("<{}?page={}&per_page={}>; rel=\"prev\"", base.split('?').next().unwrap(), page - 1, per_page));
    }
    if page < total_pages {
        links.push(format!("<{}?page={}&per_page={}>; rel=\"next\"", base.split('?').next().unwrap(), page + 1, per_page));
    }
    links.push(format!("<{}?page=1&per_page={}>; rel=\"first\"", base.split('?').next().unwrap(), per_page));
    links.push(format!("<{}?page={}&per_page={}>; rel=\"last\"", base.split('?').next().unwrap(), total_pages, per_page));
    (slice, Some(links.join(", ")))
}

fn send(request: Request, status: u16, body: Option<Value>) -> Result<()> {
    send_with(request, status, body, &[])
}

fn send_with(request: Request, status: u16, body: Option<Value>, extra: &[(&str, &str)]) -> Result<()> {
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
    for (k, v) in extra {
        if let Ok(h) = Header::from_bytes(k.as_bytes(), v.as_bytes()) {
            response = response.with_header(h);
        }
    }
    request.respond(response)?;
    Ok(())
}

fn decode(segment: &str) -> String {
    percent_decode_str(segment).decode_utf8_lossy().to_string()
}

fn bearer_token(request: &Request) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Authorization"))
        .and_then(|h| {
            let v = h.value.to_string();
            v.strip_prefix("Bearer ").map(|s| s.to_string()).or(Some(v))
        })
}


fn deliver_webhook(url_str: &str, payload: &Value) {
    // Best-effort delivery; ignore errors.
    let Ok(parsed) = Url::parse(url_str) else { return; };
    let host = parsed.host_str().unwrap_or("").to_string();
    if host.is_empty() { return; }
    let port = parsed.port().unwrap_or(80);
    let path = if parsed.path().is_empty() { "/" } else { parsed.path() };
    let body = serde_json::to_string(payload).unwrap_or_default();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    use std::io::Write;
    use std::net::TcpStream;
    if let Ok(mut stream) = TcpStream::connect((host.as_str(), port)) {
        let _ = stream.write_all(request.as_bytes());
        let _ = stream.flush();
    }
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

    let token = bearer_token(&request);

    // --- Auth / users ---

    // GET /user
    if segments == ["user"] && method == "GET" {
        let state = state.lock().expect("fake state poisoned");
        let login = token
            .as_deref()
            .and_then(|t| state.user_for_token(t))
            .unwrap_or_else(|| "octocat".to_string());
        let user = state.users.get(&login).cloned().unwrap_or(User {
            login: login.clone(),
            id: 1,
            name: Some(login.clone()),
            email: None,
            avatar_url: None,
        });
        drop(state);
        return send(
            request,
            200,
            Some(json!({
                "login": user.login,
                "id": user.id,
                "name": user.name,
                "email": user.email,
                "avatar_url": user.avatar_url
            })),
        );
    }

    // GET /users/{username}
    if segments.len() == 2 && segments[0] == "users" && method == "GET" {
        let username = &segments[1];
        let state = state.lock().expect("fake state poisoned");
        let user = state.users.get(username).cloned().unwrap_or(User {
            login: username.clone(),
            id: 1,
            name: Some(username.clone()),
            email: None,
            avatar_url: None,
        });
        drop(state);
        return send(
            request,
            200,
            Some(json!({
                "login": user.login,
                "id": user.id,
                "name": user.name,
                "email": user.email,
                "avatar_url": user.avatar_url
            })),
        );
    }

    // GET /orgs/{org}
    if segments.len() == 2 && segments[0] == "orgs" && method == "GET" {
        let orgname = &segments[1];
        let state = state.lock().expect("fake state poisoned");
        let org = state.orgs.get(orgname).cloned().unwrap_or(Organization {
            login: orgname.clone(),
            id: 1,
            name: Some(orgname.clone()),
            description: None,
        });
        drop(state);
        return send(
            request,
            200,
            Some(json!({
                "login": org.login,
                "id": org.id,
                "name": org.name,
                "description": org.description
            })),
        );
    }

    // --- Repository metadata ---

    // GET /repos/{owner}/{repo}
    if segments.len() == 3 && segments[0] == "repos" && method == "GET" {
        let owner = &segments[1];
        let repo = &segments[2];
        let state = state.lock().expect("fake state poisoned");
        let _ = state.repos.contains_key(&format!("{owner}/{repo}"));
        drop(state);
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

    // --- Asset upload (synthetic host) ---
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
        // find repo containing this release
        let mut found = None;
        for (_key, rs) in state.repos.iter_mut() {
            if let Some(rel) = rs.releases.iter_mut().find(|r| r.id == release_id) {
                let bytes = std::mem::take(&mut body);
                let size = bytes.len();
                rel.assets.retain(|a| a.name != name);
                rel.assets.push(FakeAsset {
                    id: asset_id,
                    name: name.clone(),
                    bytes,
                });
                found = Some((asset_id, name.clone(), size));
                break;
            }
        }
        drop(state);
        match found {
            Some((asset_id, name, size)) => {
                return send(
                    request,
                    201,
                    Some(json!({
                        "id": asset_id,
                        "name": name,
                        "size": size,
                        "content_type": "application/octet-stream",
                        "browser_download_url": ""
                    })),
                );
            }
            None => {
                return send(
                    request,
                    404,
                    Some(json!({ "message": "no release to upload to" })),
                );
            }
        }
    }

    // Need repo key for scoped operations.
    let repo_key = repo_key_from_segments(&segments);

    // --- Releases by tag ---
    if method == "GET"
        && segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "releases"
        && segments[4] == "tags"
    {
        let tag = &segments[5];
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let response = state.repos.get(&key).and_then(|rs| {
            rs.releases
                .iter()
                .find(|r| r.tag_name == *tag)
                .map(|r| release_json(r, base_url))
        });
        drop(state);
        return match response {
            Some(response) => send(request, 200, Some(response)),
            None => send(request, 404, Some(json!({ "message": "Not Found" }))),
        };
    }

    // --- Releases list/create ---
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "releases" {
        let key = repo_key.clone().unwrap();
        if method == "GET" {
            let state = state.lock().expect("fake state poisoned");
            let releases: Vec<Value> = state
                .repos
                .get(&key)
                .map(|rs| rs.releases.iter().map(|r| release_json(r, base_url)).collect())
                .unwrap_or_default();
            drop(state);
            let (page_items, link) = paginate(&releases, &url);
            if let Some(link) = link {
                return send_with(request, 200, Some(Value::Array(page_items)), &[("Link", &link)]);
            }
            return send(request, 200, Some(Value::Array(page_items)));
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
            let id = state.id();
            let rs = state.repos.entry(key.clone()).or_default();
            if rs.releases.iter().any(|r| r.tag_name == tag) {
                drop(state);
                return send(request, 422, Some(json!({ "message": "already_exists" })));
            }
            if !rs.tags.contains_key(&tag) {
                if let Some(target) = body.get("target_commitish").and_then(Value::as_str) {
                    rs.tags.insert(tag.clone(), target.to_string());
                    rs.commits.entry(target.to_string()).or_insert_with(|| FakeCommit {
                        sha: target.to_string(),
                        message: format!("commit {target}"),
                        parents: Vec::new(),
                    });
                }
            }
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
            let event = json!({"type": "ReleaseEvent", "repo": key, "payload": {"action": "created", "release": {"tag_name": tag}}});
            {
                let rs = state.repos.get_mut(&key).expect("repo exists");
                rs.releases.push(release);
                rs.events.push(event.clone());
            }
            let hooks: Vec<(String, Vec<String>)> = state.repos.get(&key).map(|r| r.webhooks.iter().map(|h| (h.url.clone(), h.events.clone())).collect()).unwrap_or_default();
            drop(state);
            for (url, events) in hooks {
                if events.is_empty() || events.iter().any(|e| e == "release" || e == "*") {
                    let payload = event.clone();
                    std::thread::spawn(move || deliver_webhook(&url, &payload));
                }
            }
            return send(request, 201, Some(response));
        }
    }

    // --- Single release PATCH/DELETE ---
    if segments.len() == 5
        && segments[0] == "repos"
        && segments[3] == "releases"
        && (method == "PATCH" || method == "DELETE")
    {
        let key = repo_key.clone().unwrap();
        let id: u64 = segments[4].parse()?;
        if method == "PATCH" {
            let body = parse_json_body(&body)?;
            let mut state = state.lock().expect("fake state poisoned");
            let rs = state.repos.get_mut(&key);
            let Some(rs) = rs else {
                drop(state);
                return send(request, 404, Some(json!({ "message": "Not Found" })));
            };
            let Some(release) = rs.releases.iter_mut().find(|r| r.id == id) else {
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
        } else {
            let mut state = state.lock().expect("fake state poisoned");
            if let Some(rs) = state.repos.get_mut(&key) {
                rs.releases.retain(|r| r.id != id);
            }
            drop(state);
            return send(request, 204, None);
        }
    }

    // --- Release assets list ---
    if segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "releases"
        && segments[5] == "assets"
        && method == "GET"
    {
        let key = repo_key.clone().unwrap();
        let id: u64 = segments[4].parse()?;
        let state = state.lock().expect("fake state poisoned");
        let rs = state.repos.get(&key);
        let Some(rs) = rs else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        let Some(release) = rs.releases.iter().find(|r| r.id == id) else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        let assets: Vec<Value> = release.assets.iter().map(asset_json).collect();
        drop(state);
        return send(request, 200, Some(Value::Array(assets)));
    }

    // --- Asset delete ---
    if segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "releases"
        && segments[4] == "assets"
        && method == "DELETE"
    {
        let id: u64 = segments[5].parse()?;
        let mut state = state.lock().expect("fake state poisoned");
        for rs in state.repos.values_mut() {
            for release in &mut rs.releases {
                release.assets.retain(|a| a.id != id);
            }
        }
        drop(state);
        return send(request, 204, None);
    }

    // --- Git refs: tag ---
    // GET /repos/{owner}/{repo}/git/ref/tags/{tag}
    if segments.len() == 7
        && segments[0] == "repos"
        && segments[3] == "git"
        && segments[4] == "ref"
        && segments[5] == "tags"
        && method == "GET"
    {
        let key = repo_key.clone().unwrap();
        let tag = &segments[6];
        let state = state.lock().expect("fake state poisoned");
        let sha = state.repos.get(&key).and_then(|rs| rs.tags.get(tag).cloned());
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

    // DELETE /repos/{owner}/{repo}/git/refs/tags/{tag}
    if segments.len() == 7
        && segments[0] == "repos"
        && segments[3] == "git"
        && segments[4] == "refs"
        && segments[5] == "tags"
        && method == "DELETE"
    {
        let key = repo_key.clone().unwrap();
        let tag = &segments[6];
        let mut state = state.lock().expect("fake state poisoned");
        if let Some(rs) = state.repos.get_mut(&key) {
            rs.tags.remove(tag);
        }
        drop(state);
        return send(request, 204, None);
    }

    // POST /repos/{owner}/{repo}/git/refs
    if segments.len() == 5
        && segments[0] == "repos"
        && segments[3] == "git"
        && segments[4] == "refs"
        && method == "POST"
    {
        let key = repo_key.clone().unwrap();
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
        let rs = state.repos.entry(key).or_default();
        if rs.tags.contains_key(&tag) {
            drop(state);
            return send(
                request,
                422,
                Some(json!({ "message": "Reference already exists" })),
            );
        }
        rs.tags.insert(tag.clone(), sha.clone());
        rs.commits.entry(sha.clone()).or_insert_with(|| FakeCommit {
            sha: sha.clone(),
            message: format!("commit {sha}"),
            parents: Vec::new(),
        });
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


    // Branches list
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "branches" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let branches = state.repos.get(&key).map(|rs| rs.branches.clone()).unwrap_or_else(|| vec!["main".to_string()]);
        drop(state);
        let arr: Vec<Value> = branches.iter().map(|b| json!({"name": b, "commit": {"sha": "0000000000000000000000000000000000000000"}})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Branch get
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "branches" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let name = &segments[4];
        let state = state.lock().expect("fake state poisoned");
        let exists = state.repos.get(&key).map(|rs| rs.branches.iter().any(|b| b == name)).unwrap_or(false) || name == "main";
        drop(state);
        if exists {
            return send(request, 200, Some(json!({"name": name, "commit": {"sha": "0000000000000000000000000000000000000000"}})));
        } else {
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        }
    }

    // Contents get
    if segments.len() >= 4 && segments[0] == "repos" && segments[3] == "contents" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let path = segments[4..].join("/");
        let state = state.lock().expect("fake state poisoned");
        let content = state.repos.get(&key).and_then(|rs| rs.contents.get(&path).cloned());
        drop(state);
        match content {
            Some(bytes) => {
                use base64::Engine as _;
                let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                return send(request, 200, Some(json!({
                    "name": path.rsplit('/').next().unwrap_or(&path),
                    "path": path,
                    "sha": "0000000000000000000000000000000000000000",
                    "type": "file",
                    "content": encoded,
                    "encoding": "base64"
                })));
            }
            None => {
                return send(request, 404, Some(json!({ "message": "Not Found" })));
            }
        }
    }
    // Contents create/update
    if segments.len() >= 4 && segments[0] == "repos" && segments[3] == "contents" && (method == "PUT" || method == "POST") {
        let key = repo_key.clone().unwrap();
        let path = segments[4..].join("/");
        let body = parse_json_body(&body)?;
        let content_b64 = body.get("content").and_then(Value::as_str).unwrap_or("");
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD.decode(content_b64).unwrap_or_default();
        let mut state = state.lock().expect("fake state poisoned");
        let rs = state.repos.entry(key).or_default();
        rs.contents.insert(path.clone(), bytes);
        drop(state);
        return send(request, 200, Some(json!({"content": {"path": path, "sha": "0000000000000000000000000000000000000000"}})));
    }
    // Contents delete
    if segments.len() >= 4 && segments[0] == "repos" && segments[3] == "contents" && method == "DELETE" {
        let key = repo_key.clone().unwrap();
        let path = segments[4..].join("/");
        let mut state = state.lock().expect("fake state poisoned");
        if let Some(rs) = state.repos.get_mut(&key) {
            rs.contents.remove(&path);
        }
        drop(state);
        return send(request, 200, Some(json!({"content": null})));
    }

    // Pulls list
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "pulls" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let pulls: Vec<Value> = state.repos.get(&key).map(|rs| rs.pulls.iter().map(|p| json!({"id": p.id, "number": p.number, "title": p.title, "body": p.body, "state": p.state, "user": {"login": p.user}, "head": {"ref": p.head}, "base": {"ref": p.base}, "draft": p.draft, "merged": p.merged, "mergeable": p.mergeable})).collect()).unwrap_or_default();
        drop(state);
        return send(request, 200, Some(Value::Array(pulls)));
    }
    // Pull get
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "pulls" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let pr = state.repos.get(&key).and_then(|rs| rs.pulls.iter().find(|p| p.number == number).cloned());
        drop(state);
        match pr {
            Some(p) => return send(request, 200, Some(json!({"id": p.id, "number": p.number, "title": p.title, "body": p.body, "state": p.state, "user": {"login": p.user}, "head": {"ref": p.head}, "base": {"ref": p.base}, "draft": p.draft, "merged": p.merged, "mergeable": p.mergeable}))),
            None => return send(request, 404, Some(json!({"message": "Not Found"}))),
        }
    }
    // Pull create
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "pulls" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let body = parse_json_body(&body)?;
        let title = body.get("title").and_then(Value::as_str).unwrap_or("").to_string();
        let pr_body = body.get("body").and_then(Value::as_str).map(|s| s.to_string());
        let head = body.get("head").and_then(Value::as_str).unwrap_or("").to_string();
        let base = body.get("base").and_then(Value::as_str).unwrap_or("main").to_string();
        let draft = body.get("draft").and_then(Value::as_bool).unwrap_or(false);
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.entry(key).or_default();
        rs.next_pull_number += 1;
        let number = rs.next_pull_number;
        let pr = FakePull { id, number, title: title.clone(), body: pr_body.clone(), state: "open".to_string(), user: "octocat".to_string(), head: head.clone(), base: base.clone(), draft, merged: false, mergeable: true };
        let response = json!({"id": id, "number": number, "title": title, "body": pr_body, "state": "open", "user": {"login": "octocat"}, "head": {"ref": head}, "base": {"ref": base}, "draft": draft, "merged": false, "mergeable": true});
        rs.pulls.push(pr);
        rs.events.push(json!({"type": "PullRequestEvent", "payload": {"action": "opened", "pull_request": {"number": number, "title": title}}}));
        drop(state);
        return send(request, 201, Some(response));
    }
    // Pull update
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "pulls" && method == "PATCH" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let mut state = state.lock().expect("fake state poisoned");
        let rs = state.repos.get_mut(&key);
        let Some(rs) = rs else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        let Some(pr) = rs.pulls.iter_mut().find(|p| p.number == number) else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        if let Some(t) = body.get("title").and_then(Value::as_str) { pr.title = t.to_string(); }
        if let Some(b) = body.get("body").and_then(Value::as_str) { pr.body = Some(b.to_string()); }
        if let Some(st) = body.get("state").and_then(Value::as_str) { pr.state = st.to_string(); }
        let response = json!({"id": pr.id, "number": pr.number, "title": pr.title, "body": pr.body, "state": pr.state, "user": {"login": pr.user}, "head": {"ref": pr.head}, "base": {"ref": pr.base}, "draft": pr.draft, "merged": pr.merged, "mergeable": pr.mergeable});
        drop(state);
        return send(request, 200, Some(response));
    }
    // Pull merge
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "pulls" && segments[5] == "merge" && method == "PUT" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let mut state = state.lock().expect("fake state poisoned");
        let rs = state.repos.get_mut(&key);
        let Some(rs) = rs else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        let Some(pr) = rs.pulls.iter_mut().find(|p| p.number == number) else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        pr.merged = true;
        pr.state = "closed".to_string();
        drop(state);
        return send(request, 200, Some(json!({"merged": true, "message": "Pull Request successfully merged"})));
    }
    // Pull reviews list
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "pulls" && segments[5] == "reviews" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let reviews = state.repos.get(&key).and_then(|rs| rs.pull_reviews.get(&number).cloned()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = reviews.iter().map(|r| json!({"id": r.id, "body": r.body, "state": r.state, "user": {"login": r.user}})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Pull review create
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "pulls" && segments[5] == "reviews" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let state_str = body.get("event").and_then(Value::as_str).unwrap_or("COMMENT").to_string();
        let review_body = body.get("body").and_then(Value::as_str).map(|s| s.to_string());
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.get_mut(&key);
        let Some(rs) = rs else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        if !rs.pulls.iter().any(|p| p.number == number) { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); }
        rs.pull_reviews.entry(number).or_default().push(FakePullReview { id, body: review_body.clone(), state: state_str.clone(), user: "octocat".to_string() });
        drop(state);
        return send(request, 200, Some(json!({"id": id, "body": review_body, "state": state_str, "user": {"login": "octocat"}})));
    }
    // Pull comments list
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "pulls" && segments[5] == "comments" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let comments = state.repos.get(&key).and_then(|rs| rs.pull_comments.get(&number).cloned()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = comments.iter().map(|c| json!({"id": c.id, "body": c.body, "user": {"login": c.user}, "path": c.path})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Pull comment create
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "pulls" && segments[5] == "comments" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let text = body.get("body").and_then(Value::as_str).unwrap_or("").to_string();
        let path = body.get("path").and_then(Value::as_str).map(|s| s.to_string());
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.get_mut(&key);
        let Some(rs) = rs else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        if !rs.pulls.iter().any(|p| p.number == number) { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); }
        rs.pull_comments.entry(number).or_default().push(FakePullComment { id, body: text.clone(), user: "octocat".to_string(), path: path.clone() });
        drop(state);
        return send(request, 201, Some(json!({"id": id, "body": text, "user": {"login": "octocat"}, "path": path})));
    }


    // Labels list
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "labels" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let labels = state.repos.get(&key).map(|rs| rs.labels.clone()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = labels.iter().map(|l| json!({"name": l.name, "color": l.color, "description": l.description})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Label create
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "labels" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let body = parse_json_body(&body)?;
        let name = body.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let color = body.get("color").and_then(Value::as_str).unwrap_or("ededed").to_string();
        let description = body.get("description").and_then(Value::as_str).map(|s| s.to_string());
        let mut state = state.lock().expect("fake state poisoned");
        let rs = state.repos.entry(key).or_default();
        rs.labels.push(FakeLabel { name: name.clone(), color: color.clone(), description: description.clone() });
        drop(state);
        return send(request, 201, Some(json!({"name": name, "color": color, "description": description})));
    }
    // Label get
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "labels" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let name = &segments[4];
        let state = state.lock().expect("fake state poisoned");
        let label = state.repos.get(&key).and_then(|rs| rs.labels.iter().find(|l| l.name == *name).cloned());
        drop(state);
        match label {
            Some(l) => return send(request, 200, Some(json!({"name": l.name, "color": l.color, "description": l.description}))),
            None => return send(request, 404, Some(json!({"message": "Not Found"}))),
        }
    }
    // Label delete
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "labels" && method == "DELETE" {
        let key = repo_key.clone().unwrap();
        let name = &segments[4];
        let mut state = state.lock().expect("fake state poisoned");
        if let Some(rs) = state.repos.get_mut(&key) {
            rs.labels.retain(|l| l.name != *name);
        }
        drop(state);
        return send(request, 204, None);
    }

    // Milestones list
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "milestones" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let ms = state.repos.get(&key).map(|rs| rs.milestones.clone()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = ms.iter().map(|m| json!({"number": m.number, "title": m.title, "state": m.state, "description": m.description})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Milestone create
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "milestones" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let body = parse_json_body(&body)?;
        let title = body.get("title").and_then(Value::as_str).unwrap_or("").to_string();
        let description = body.get("description").and_then(Value::as_str).map(|s| s.to_string());
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.entry(key).or_default();
        rs.next_milestone_number += 1;
        let number = rs.next_milestone_number;
        rs.milestones.push(FakeMilestone { id, number, title: title.clone(), state: "open".to_string(), description: description.clone() });
        drop(state);
        return send(request, 201, Some(json!({"number": number, "title": title, "state": "open", "description": description})));
    }
    // Milestone get
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "milestones" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let m = state.repos.get(&key).and_then(|rs| rs.milestones.iter().find(|m| m.number == number).cloned());
        drop(state);
        match m {
            Some(m) => return send(request, 200, Some(json!({"number": m.number, "title": m.title, "state": m.state, "description": m.description}))),
            None => return send(request, 404, Some(json!({"message": "Not Found"}))),
        }
    }
    // Milestone update
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "milestones" && method == "PATCH" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let mut state = state.lock().expect("fake state poisoned");
        let rs = state.repos.get_mut(&key);
        let Some(rs) = rs else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        let Some(m) = rs.milestones.iter_mut().find(|m| m.number == number) else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        if let Some(t) = body.get("title").and_then(Value::as_str) { m.title = t.to_string(); }
        if let Some(st) = body.get("state").and_then(Value::as_str) { m.state = st.to_string(); }
        if let Some(d) = body.get("description").and_then(Value::as_str) { m.description = Some(d.to_string()); }
        let resp = json!({"number": m.number, "title": m.title, "state": m.state, "description": m.description});
        drop(state);
        return send(request, 200, Some(resp));
    }
    // Milestone delete
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "milestones" && method == "DELETE" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let mut state = state.lock().expect("fake state poisoned");
        if let Some(rs) = state.repos.get_mut(&key) {
            rs.milestones.retain(|m| m.number != number);
        }
        drop(state);
        return send(request, 204, None);
    }


    // Commit statuses list: GET /repos/{owner}/{repo}/commits/{sha}/statuses
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "commits" && segments[5] == "statuses" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let sha = &segments[4];
        let state = state.lock().expect("fake state poisoned");
        let statuses = state.repos.get(&key).and_then(|rs| rs.commit_statuses.get(sha).cloned()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = statuses.iter().map(|st| json!({"id": st.id, "state": st.state, "description": st.description, "context": st.context, "target_url": st.target_url})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Combined status: GET /repos/{owner}/{repo}/commits/{sha}/status
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "commits" && segments[5] == "status" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let sha = &segments[4];
        let state = state.lock().expect("fake state poisoned");
        let statuses = state.repos.get(&key).and_then(|rs| rs.commit_statuses.get(sha).cloned()).unwrap_or_default();
        drop(state);
        let overall = if statuses.iter().any(|s| s.state == "failure" || s.state == "error") { "failure" } else if statuses.iter().any(|s| s.state == "pending") { "pending" } else { "success" };
        return send(request, 200, Some(json!({"state": overall, "statuses": statuses.iter().map(|st| json!({"state": st.state, "context": st.context})).collect::<Vec<_>>()})));
    }
    // Create commit status: POST /repos/{owner}/{repo}/statuses/{sha}
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "statuses" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let sha = &segments[4];
        let body = parse_json_body(&body)?;
        let st_state = body.get("state").and_then(Value::as_str).unwrap_or("pending").to_string();
        let description = body.get("description").and_then(Value::as_str).map(|s| s.to_string());
        let context = body.get("context").and_then(Value::as_str).unwrap_or("default").to_string();
        let target_url = body.get("target_url").and_then(Value::as_str).map(|s| s.to_string());
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.entry(key).or_default();
        rs.commit_statuses.entry(sha.to_string()).or_default().push(FakeCommitStatus { id, state: st_state.clone(), description: description.clone(), context: context.clone(), target_url: target_url.clone() });
        drop(state);
        return send(request, 201, Some(json!({"id": id, "state": st_state, "description": description, "context": context, "target_url": target_url})));
    }
    // Webhooks list
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "hooks" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let hooks = state.repos.get(&key).map(|rs| rs.webhooks.clone()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = hooks.iter().map(|h| json!({"id": h.id, "config": {"url": h.url}, "events": h.events, "active": h.active})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Webhook create
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "hooks" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let body = parse_json_body(&body)?;
        let url = body.get("config").and_then(|c| c.get("url")).and_then(Value::as_str).unwrap_or("").to_string();
        let events = body.get("events").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_else(|| vec!["push".to_string()]);
        let active = body.get("active").and_then(Value::as_bool).unwrap_or(true);
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.entry(key).or_default();
        rs.webhooks.push(FakeWebhook { id, url: url.clone(), events: events.clone(), active });
        drop(state);
        return send(request, 201, Some(json!({"id": id, "config": {"url": url}, "events": events, "active": active})));
    }
    // Webhook delete
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "hooks" && method == "DELETE" {
        let key = repo_key.clone().unwrap();
        let id: u64 = segments[4].parse().unwrap_or(0);
        let mut state = state.lock().expect("fake state poisoned");
        if let Some(rs) = state.repos.get_mut(&key) {
            rs.webhooks.retain(|h| h.id != id);
        }
        drop(state);
        return send(request, 204, None);
    }

    // Search issues (very basic)
    if segments.len() == 3 && segments[0] == "search" && segments[1] == "issues" && method == "GET" {
        let q = url.query_pairs().find(|(k, _)| k == "q").map(|(_, v)| v.to_string()).unwrap_or_default();
        let state = state.lock().expect("fake state poisoned");
        let mut items = Vec::new();
        for rs in state.repos.values() {
            for issue in &rs.issues {
                if q.is_empty() || issue.title.contains(&q) {
                    items.push(json!({"number": issue.number, "title": issue.title, "state": issue.state}));
                }
            }
        }
        drop(state);
        return send(request, 200, Some(json!({"total_count": items.len(), "items": items})));
    }


    // Check runs list: GET /repos/{owner}/{repo}/commits/{sha}/check-runs
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "commits" && segments[5] == "check-runs" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let sha = &segments[4];
        let state = state.lock().expect("fake state poisoned");
        let runs = state.repos.get(&key).and_then(|rs| rs.check_runs.get(sha).cloned()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = runs.iter().map(|r| json!({"id": r.id, "name": r.name, "head_sha": r.head_sha, "status": r.status, "conclusion": r.conclusion})).collect();
        return send(request, 200, Some(json!({"total_count": arr.len(), "check_runs": arr})));
    }
    // Create check run: POST /repos/{owner}/{repo}/check-runs
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "check-runs" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let body = parse_json_body(&body)?;
        let name = body.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let head_sha = body.get("head_sha").and_then(Value::as_str).unwrap_or("").to_string();
        let status = body.get("status").and_then(Value::as_str).unwrap_or("queued").to_string();
        let conclusion = body.get("conclusion").and_then(Value::as_str).map(|s| s.to_string());
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.entry(key).or_default();
        rs.check_runs.entry(head_sha.clone()).or_default().push(FakeCheckRun { id, name: name.clone(), head_sha: head_sha.clone(), status: status.clone(), conclusion: conclusion.clone() });
        drop(state);
        return send(request, 201, Some(json!({"id": id, "name": name, "head_sha": head_sha, "status": status, "conclusion": conclusion})));
    }
    // Deployments list
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "deployments" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let deps = state.repos.get(&key).map(|rs| rs.deployments.clone()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = deps.iter().map(|d| json!({"id": d.id, "sha": d.sha, "ref": d.ref_name, "environment": d.environment, "description": d.description})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Create deployment
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "deployments" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let body = parse_json_body(&body)?;
        let sha = body.get("sha").and_then(Value::as_str).unwrap_or("").to_string();
        let ref_name = body.get("ref").and_then(Value::as_str).unwrap_or("main").to_string();
        let environment = body.get("environment").and_then(Value::as_str).unwrap_or("production").to_string();
        let description = body.get("description").and_then(Value::as_str).map(|s| s.to_string());
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.entry(key).or_default();
        rs.deployments.push(FakeDeployment { id, sha: sha.clone(), ref_name: ref_name.clone(), environment: environment.clone(), description: description.clone() });
        drop(state);
        return send(request, 201, Some(json!({"id": id, "sha": sha, "ref": ref_name, "environment": environment, "description": description})));
    }
    // Deployment statuses
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "deployments" && segments[5] == "statuses" && method == "GET" {
        return send(request, 200, Some(Value::Array(vec![])));
    }
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "deployments" && segments[5] == "statuses" && method == "POST" {
        let body = parse_json_body(&body)?;
        let state_str = body.get("state").and_then(Value::as_str).unwrap_or("success").to_string();
        return send(request, 201, Some(json!({"state": state_str})));
    }


    // List workflows
    if segments.len() == 4 && segments[0] == "repos" && segments[2] == segments[2] && segments[3] == "workflows" && segments[1] == segments[1] && method == "GET" && segments[0] == "repos" {
        // path: /repos/{owner}/{repo}/actions/workflows actually has 6 segments; handle below
    }
    // Actions workflows: GET /repos/{owner}/{repo}/actions/workflows
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "actions" && segments[4] == "workflows" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let workflows = state.repos.get(&key).map(|rs| rs.workflows.clone()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = workflows.iter().map(|w| json!({"id": w.id, "name": w.name, "path": w.path, "state": w.state})).collect();
        return send(request, 200, Some(json!({"total_count": arr.len(), "workflows": arr})));
    }
    // Workflow dispatch: POST /repos/{owner}/{repo}/actions/workflows/{id}/dispatches
    if segments.len() == 7 && segments[0] == "repos" && segments[3] == "actions" && segments[4] == "workflows" && segments[6] == "dispatches" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let workflow_id: u64 = segments[5].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let ref_name = body.get("ref").and_then(Value::as_str).unwrap_or("main").to_string();
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.entry(key).or_default();
        let run_number = rs.next_workflow_run_number + 1;
        rs.next_workflow_run_number = run_number;
        let workflow_name = rs.workflows.iter().find(|w| w.id == workflow_id).map(|w| w.name.clone()).unwrap_or_else(|| "workflow".to_string());
        rs.workflow_runs.push(FakeWorkflowRun { id, name: workflow_name, workflow_id, head_sha: "0000000000000000000000000000000000000000".to_string(), head_branch: ref_name, status: "queued".to_string(), conclusion: None, event: "workflow_dispatch".to_string(), run_number });
        drop(state);
        return send(request, 204, None);
    }
    // List workflow runs
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "actions" && segments[4] == "runs" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let runs = state.repos.get(&key).map(|rs| rs.workflow_runs.clone()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = runs.iter().map(|r| json!({"id": r.id, "name": r.name, "head_branch": r.head_branch, "status": r.status, "conclusion": r.conclusion, "event": r.event, "run_number": r.run_number})).collect();
        return send(request, 200, Some(json!({"total_count": arr.len(), "workflow_runs": arr})));
    }
    // Get workflow run
    if segments.len() == 7 && segments[0] == "repos" && segments[3] == "actions" && segments[4] == "runs" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let run_id: u64 = segments[6].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let run = state.repos.get(&key).and_then(|rs| rs.workflow_runs.iter().find(|r| r.id == run_id).cloned());
        drop(state);
        match run {
            Some(r) => return send(request, 200, Some(json!({"id": r.id, "name": r.name, "head_branch": r.head_branch, "status": r.status, "conclusion": r.conclusion, "event": r.event, "run_number": r.run_number}))),
            None => return send(request, 404, Some(json!({"message": "Not Found"}))),
        }
    }
    // Rerun workflow run
    if segments.len() == 8 && segments[0] == "repos" && segments[3] == "actions" && segments[4] == "runs" && segments[7] == "rerun" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let run_id: u64 = segments[6].parse().unwrap_or(0);
        let mut state = state.lock().expect("fake state poisoned");
        if let Some(rs) = state.repos.get_mut(&key) {
            if let Some(run) = rs.workflow_runs.iter_mut().find(|r| r.id == run_id) {
                run.status = "queued".to_string();
                run.conclusion = None;
            }
        }
        drop(state);
        return send(request, 201, None);
    }
    // List jobs for a run
    if segments.len() == 8 && segments[0] == "repos" && segments[3] == "actions" && segments[4] == "runs" && segments[6] == "jobs" && method == "GET" {
        return send(request, 200, Some(json!({"total_count": 0, "jobs": []})));
    }
    // List artifacts for a run
    if segments.len() == 8 && segments[0] == "repos" && segments[3] == "actions" && segments[4] == "runs" && segments[6] == "artifacts" && method == "GET" {
        return send(request, 200, Some(json!({"total_count": 0, "artifacts": []})));
    }

    // Branch protection get
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "branches" && segments[5] == "protection" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let branch = &segments[4];
        let state = state.lock().expect("fake state poisoned");
        let prot = state.repos.get(&key).and_then(|rs| rs.branch_protection.get(branch).cloned());
        drop(state);
        match prot {
            Some(p) => return send(request, 200, Some(json!({"required_status_checks": {"contexts": p.required_status_checks}, "enforce_admins": {"enabled": p.enforce_admins}, "required_pull_request_reviews": {"required_approving_review_count": if p.required_pull_request_reviews {1} else {0}}}))),
            None => return send(request, 404, Some(json!({"message": "Not Found"}))),
        }
    }
    // Branch protection update
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "branches" && segments[5] == "protection" && method == "PUT" {
        let key = repo_key.clone().unwrap();
        let branch = segments[4].clone();
        let body = parse_json_body(&body)?;
        let contexts = body.get("required_status_checks").and_then(|c| c.get("contexts")).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect()).unwrap_or_default();
        let enforce = body.get("enforce_admins").and_then(Value::as_bool).unwrap_or(false);
        let pr_reviews = body.get("required_pull_request_reviews").is_some();
        let mut state = state.lock().expect("fake state poisoned");
        let rs = state.repos.entry(key).or_default();
        rs.branch_protection.insert(branch, FakeBranchProtection { required_status_checks: contexts, enforce_admins: enforce, required_pull_request_reviews: pr_reviews });
        drop(state);
        return send(request, 200, Some(json!({"ok": true})));
    }
    // Branch protection delete
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "branches" && segments[5] == "protection" && method == "DELETE" {
        let key = repo_key.clone().unwrap();
        let branch = &segments[4];
        let mut state = state.lock().expect("fake state poisoned");
        if let Some(rs) = state.repos.get_mut(&key) {
            rs.branch_protection.remove(branch);
        }
        drop(state);
        return send(request, 204, None);
    }

    // Merge queue: GET /repos/{owner}/{repo}/merge-queue
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "merge-queue" && segments[4] == "entries" && method == "GET" {
        // not standard; skip
    }
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "merge-queue" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let q = state.repos.get(&key).map(|rs| rs.merge_queue.clone()).unwrap_or_default();
        drop(state);
        return send(request, 200, Some(json!({"entries": q})));
    }
    // Enqueue PR into merge queue
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "merge-queue" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let body = parse_json_body(&body)?;
        let pr = body.get("pull_request").and_then(Value::as_str).or_else(|| body.get("pull_request_number").map(|_| "")).unwrap_or("").to_string();
        let mut state = state.lock().expect("fake state poisoned");
        let rs = state.repos.entry(key).or_default();
        if !pr.is_empty() { rs.merge_queue.push(pr); }
        drop(state);
        return send(request, 200, Some(json!({"ok": true})));
    }


    // Repo rulesets list: GET /repos/{owner}/{repo}/rulesets
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "rulesets" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let rsets = state.repos.get(&key).map(|rs| rs.rulesets.clone()).unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = rsets.iter().map(|r| json!({"id": r.id, "name": r.name, "target": r.target, "enforcement": r.enforcement, "conditions": r.conditions, "rules": r.rules})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Repo ruleset create
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "rulesets" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let body = parse_json_body(&body)?;
        let name = body.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let target = body.get("target").and_then(Value::as_str).unwrap_or("branch").to_string();
        let enforcement = body.get("enforcement").and_then(Value::as_str).unwrap_or("active").to_string();
        let conditions = body.get("conditions").cloned().unwrap_or(json!({}));
        let rules = body.get("rules").cloned().unwrap_or(json!([]));
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.entry(key).or_default();
        rs.rulesets.push(FakeRuleset { id, name: name.clone(), target: target.clone(), enforcement: enforcement.clone(), conditions: conditions.clone(), rules: rules.clone() });
        drop(state);
        return send(request, 201, Some(json!({"id": id, "name": name, "target": target, "enforcement": enforcement, "conditions": conditions, "rules": rules})));
    }
    // Repo ruleset get
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "rulesets" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let id: u64 = segments[4].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let rset = state.repos.get(&key).and_then(|rs| rs.rulesets.iter().find(|r| r.id == id).cloned());
        drop(state);
        match rset {
            Some(r) => return send(request, 200, Some(json!({"id": r.id, "name": r.name, "target": r.target, "enforcement": r.enforcement, "conditions": r.conditions, "rules": r.rules}))),
            None => return send(request, 404, Some(json!({"message": "Not Found"}))),
        }
    }
    // Repo ruleset update
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "rulesets" && method == "PUT" {
        let key = repo_key.clone().unwrap();
        let id: u64 = segments[4].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let mut state = state.lock().expect("fake state poisoned");
        let rs = state.repos.get_mut(&key);
        let Some(rs) = rs else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        let Some(rset) = rs.rulesets.iter_mut().find(|r| r.id == id) else { drop(state); return send(request, 404, Some(json!({"message":"Not Found"}))); };
        if let Some(n) = body.get("name").and_then(Value::as_str) { rset.name = n.to_string(); }
        if let Some(e) = body.get("enforcement").and_then(Value::as_str) { rset.enforcement = e.to_string(); }
        if let Some(c) = body.get("conditions") { rset.conditions = c.clone(); }
        if let Some(r) = body.get("rules") { rset.rules = r.clone(); }
        let resp = json!({"id": rset.id, "name": rset.name, "target": rset.target, "enforcement": rset.enforcement, "conditions": rset.conditions, "rules": rset.rules});
        drop(state);
        return send(request, 200, Some(resp));
    }
    // Repo ruleset delete
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "rulesets" && method == "DELETE" {
        let key = repo_key.clone().unwrap();
        let id: u64 = segments[4].parse().unwrap_or(0);
        let mut state = state.lock().expect("fake state poisoned");
        if let Some(rs) = state.repos.get_mut(&key) {
            rs.rulesets.retain(|r| r.id != id);
        }
        drop(state);
        return send(request, 204, None);
    }

    // Org rulesets list: GET /orgs/{org}/rulesets
    if segments.len() == 3 && segments[0] == "orgs" && segments[2] == "rulesets" && method == "GET" {
        let org = &segments[1];
        let state = state.lock().expect("fake state poisoned");
        let rsets = state.org_rulesets.get(org).cloned().unwrap_or_default();
        drop(state);
        let arr: Vec<Value> = rsets.iter().map(|r| json!({"id": r.id, "name": r.name, "target": r.target, "enforcement": r.enforcement, "conditions": r.conditions, "rules": r.rules})).collect();
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Org ruleset create
    if segments.len() == 3 && segments[0] == "orgs" && segments[2] == "rulesets" && method == "POST" {
        let org = segments[1].clone();
        let body = parse_json_body(&body)?;
        let name = body.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let target = body.get("target").and_then(Value::as_str).unwrap_or("branch").to_string();
        let enforcement = body.get("enforcement").and_then(Value::as_str).unwrap_or("active").to_string();
        let conditions = body.get("conditions").cloned().unwrap_or(json!({}));
        let rules = body.get("rules").cloned().unwrap_or(json!([]));
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        state.org_rulesets.entry(org).or_default().push(FakeRuleset { id, name: name.clone(), target: target.clone(), enforcement: enforcement.clone(), conditions: conditions.clone(), rules: rules.clone() });
        drop(state);
        return send(request, 201, Some(json!({"id": id, "name": name, "target": target, "enforcement": enforcement, "conditions": conditions, "rules": rules})));
    }


    // Events list (repo)
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "events" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let events = state.repos.get(&key).map(|rs| rs.events.clone()).unwrap_or_default();
        drop(state);
        return send(request, 200, Some(Value::Array(events)));
    }
    // Org events
    if segments.len() == 3 && segments[0] == "orgs" && segments[2] == "events" && method == "GET" {
        return send(request, 200, Some(Value::Array(vec![])));
    }
    // User events
    if segments.len() == 3 && segments[0] == "users" && segments[2] == "events" && method == "GET" {
        return send(request, 200, Some(Value::Array(vec![])));
    }
    // Environments list
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "environments" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let _ = state.repos.get(&key);
        drop(state);
        return send(request, 200, Some(json!({"total_count": 0, "environments": []})));
    }
    // Create environment
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "environments" && method == "PUT" {
        let body = parse_json_body(&body)?;
        let name = body.get("name").and_then(Value::as_str).unwrap_or("production").to_string();
        return send(request, 200, Some(json!({"name": name, "protection_rules": []})));
    }
    // Git trees get
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "git" && segments[4] == "trees" && method == "GET" {
        let sha = &segments[5];
        return send(request, 200, Some(json!({"sha": sha, "tree": [], "truncated": false})));
    }
    // Git blobs get
    if segments.len() == 6 && segments[0] == "repos" && segments[3] == "git" && segments[4] == "blobs" && method == "GET" {
        let sha = &segments[5];
        return send(request, 200, Some(json!({"sha": sha, "content": "", "encoding": "base64", "size": 0})));
    }
    // Rate limit
    if segments == ["rate_limit"] && method == "GET" {
        return send(request, 200, Some(json!({"resources": {"core": {"limit": 5000, "remaining": 4999, "reset": 0}}})));
    }
    // Meta
    if segments == ["meta"] && method == "GET" {
        return send(request, 200, Some(json!({"verifiable_password_authentication": true})));
    }
    // Octocat
    if segments == ["octocat"] && method == "GET" {
        return send(request, 200, Some(json!({"message": "Emulated by fake-github"})));
    }


    // GraphQL endpoint
    if (segments == ["graphql"] && (method == "POST" || method == "GET")) {
        let body_text = String::from_utf8_lossy(&body).to_string();
        let query = if method == "POST" {
            // try to parse JSON body
            serde_json::from_str::<Value>(&body_text)
                .ok()
                .and_then(|v| v.get("query").and_then(Value::as_str).map(|s| s.to_string()))
                .unwrap_or(body_text.clone())
        } else {
            url.query_pairs().find(|(k,_)| k == "query").map(|(_,v)| v.to_string()).unwrap_or_default()
        };
        // Minimal handling: return viewer + repository stub
        let data = if query.contains("viewer") {
            json!({"viewer": {"login": "octocat", "id": "1"}})
        } else if query.contains("repository") {
            json!({"repository": {"name": "r", "owner": {"login": "o"}, "issues": {"totalCount": 0, "nodes": []}, "pullRequests": {"totalCount": 0, "nodes": []}}})
        } else if query.contains("search") {
            json!({"search": {"issueCount": 0, "nodes": []}})
        } else {
            json!({})
        };
        return send(request, 200, Some(json!({"data": data})));
    }

    // Annotated tag lookup: always 404
    if segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "git"
        && segments[4] == "tags"
        && method == "GET"
    {
        return send(request, 404, Some(json!({ "message": "Not Found" })));
    }

    // Commits list/get
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "commits" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let commits: Vec<Value> = state
            .repos
            .get(&key)
            .map(|rs| {
                rs.commits
                    .values()
                    .map(|c| {
                        json!({"sha": c.sha, "commit": {"message": c.message}, "parents": c.parents.iter().map(|p| json!({"sha": p})).collect::<Vec<_>>()})
                    })
                    .collect()
            })
            .unwrap_or_default();
        drop(state);
        let (page_items, link) = paginate(&commits, &url);
        if let Some(link) = link {
            return send_with(request, 200, Some(Value::Array(page_items)), &[("Link", &link)]);
        }
        return send(request, 200, Some(Value::Array(page_items)));
    }
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "commits" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let sha = &segments[4];
        let state = state.lock().expect("fake state poisoned");
        let commit = state.repos.get(&key).and_then(|rs| rs.commits.get(sha).cloned());
        drop(state);
        match commit {
            Some(c) => {
                return send(
                    request,
                    200,
                    Some(json!({"sha": c.sha, "commit": {"message": c.message}, "parents": c.parents.iter().map(|p| json!({"sha": p})).collect::<Vec<_>>()})),
                );
            }
            None => {
                return send(request, 404, Some(json!({ "message": "Not Found" })));
            }
        }
    }

    // Issues list
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "issues" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let state = state.lock().expect("fake state poisoned");
        let issues: Vec<Value> = state
            .repos
            .get(&key)
            .map(|rs| {
                rs.issues
                    .iter()
                    .map(|i| {
                        json!({"id": i.id, "number": i.number, "title": i.title, "body": i.body, "state": i.state, "user": {"login": i.user}, "labels": i.labels.iter().map(|l| json!({"name": l})).collect::<Vec<_>>()})
                    })
                    .collect()
            })
            .unwrap_or_default();
        drop(state);
        let (page_items, link) = paginate(&issues, &url);
        if let Some(link) = link {
            return send_with(request, 200, Some(Value::Array(page_items)), &[("Link", &link)]);
        }
        return send(request, 200, Some(Value::Array(page_items)));
    }
    // Issue get
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "issues" && method == "GET" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let issue = state.repos.get(&key).and_then(|rs| rs.issues.iter().find(|i| i.number == number).cloned());
        drop(state);
        match issue {
            Some(i) => {
                return send(
                    request,
                    200,
                    Some(json!({"id": i.id, "number": i.number, "title": i.title, "body": i.body, "state": i.state, "user": {"login": i.user}, "labels": i.labels.iter().map(|l| json!({"name": l})).collect::<Vec<_>>()})),
                );
            }
            None => {
                return send(request, 404, Some(json!({ "message": "Not Found" })));
            }
        }
    }
    // Issue create
    if segments.len() == 4 && segments[0] == "repos" && segments[3] == "issues" && method == "POST" {
        let key = repo_key.clone().unwrap();
        let body = parse_json_body(&body)?;
        let title = body.get("title").and_then(Value::as_str).unwrap_or("").to_string();
        let issue_body = body.get("body").and_then(Value::as_str).map(|s| s.to_string());
        let labels: Vec<String> = body
            .get("labels")
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_str).map(String::from).collect())
            .unwrap_or_default();
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.entry(key).or_default();
        rs.next_issue_number += 1;
        let number = rs.next_issue_number;
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
            "labels": labels.iter().map(|l| json!({"name": l})).collect::<Vec<_>>()
        });
        rs.issues.push(issue);
        rs.events.push(json!({"type": "IssuesEvent", "payload": {"action": "opened", "issue": {"number": number, "title": title}}}));
        drop(state);
        return send(request, 201, Some(response));
    }
    // Issue update
    if segments.len() == 5 && segments[0] == "repos" && segments[3] == "issues" && method == "PATCH" {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let mut state = state.lock().expect("fake state poisoned");
        let rs = state.repos.get_mut(&key);
        let Some(rs) = rs else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        let Some(issue) = rs.issues.iter_mut().find(|i| i.number == number) else {
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
    // Issue comments list
    if segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "issues"
        && segments[5] == "comments"
        && method == "GET"
    {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let state = state.lock().expect("fake state poisoned");
        let rs = state.repos.get(&key);
        let Some(rs) = rs else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        if !rs.issues.iter().any(|i| i.number == number) {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        }
        let comments = rs.issue_comments.get(&number).cloned().unwrap_or_default();
        let arr: Vec<Value> = comments
            .iter()
            .map(|c| json!({"id": c.id, "body": c.body, "user": {"login": c.user}}))
            .collect();
        drop(state);
        return send(request, 200, Some(Value::Array(arr)));
    }
    // Issue comment create
    if segments.len() == 6
        && segments[0] == "repos"
        && segments[3] == "issues"
        && segments[5] == "comments"
        && method == "POST"
    {
        let key = repo_key.clone().unwrap();
        let number: u64 = segments[4].parse().unwrap_or(0);
        let body = parse_json_body(&body)?;
        let text = body.get("body").and_then(Value::as_str).unwrap_or("").to_string();
        let mut state = state.lock().expect("fake state poisoned");
        let id = state.id();
        let rs = state.repos.get_mut(&key);
        let Some(rs) = rs else {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        };
        if !rs.issues.iter().any(|i| i.number == number) {
            drop(state);
            return send(request, 404, Some(json!({ "message": "Not Found" })));
        }
        rs.issue_comments
            .entry(number)
            .or_default()
            .push(FakeIssueComment {
                id,
                body: text.clone(),
                user: "octocat".to_string(),
            });
        drop(state);
        return send(request, 201, Some(json!({"id": id, "body": text, "user": {"login": "octocat"}})));
    }

    // Users list? GET /users
    // (not implemented)

    send(
        request,
        404,
        Some(json!({ "message": format!("unhandled {method} {path}") })),
    )
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
        "assets": release.assets.iter().map(asset_json).collect::<Vec<_>>(),
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
