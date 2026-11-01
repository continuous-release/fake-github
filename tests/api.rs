use fake_github::{FakeGitHub, User};
use reqwest::blocking::Client;
use serde_json::json;

fn base_url(fake: &mut FakeGitHub) -> String {
    fake.start().expect("start")
}

fn client() -> Client {
    Client::new()
}

#[test]
fn get_release_by_tag_returns_404_when_absent() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client()
        .get(format!("{url}/repos/o/r/releases/tags/v1.2.3"))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn create_release_then_get_by_tag() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let create = client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&json!({
            "tag_name": "v1.2.3",
            "target_commitish": "abc",
            "name": "v1.2.3",
            "body": "notes",
            "draft": true,
            "prerelease": false
        }))
        .send()
        .unwrap();
    assert_eq!(create.status().as_u16(), 201);
    let created: serde_json::Value = create.json().unwrap();
    assert_eq!(created["tag_name"], "v1.2.3");

    let get = client()
        .get(format!("{url}/repos/o/r/releases/tags/v1.2.3"))
        .send()
        .unwrap();
    assert_eq!(get.status().as_u16(), 200);
    let got: serde_json::Value = get.json().unwrap();
    assert_eq!(got["tag_name"], "v1.2.3");
    assert_eq!(got["draft"], true);
    assert_eq!(got["body"], "notes");
    fake.stop();
}

#[test]
fn create_duplicate_release_returns_422() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let body = json!({"tag_name": "v1.2.3", "target_commitish": "abc"});
    let first = client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&body)
        .send()
        .unwrap();
    assert_eq!(first.status().as_u16(), 201);
    let second = client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&body)
        .send()
        .unwrap();
    assert_eq!(second.status().as_u16(), 422);
    fake.stop();
}

#[test]
fn list_releases_returns_all() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    for tag in ["v1.0.0", "v2.0.0"] {
        client()
            .post(format!("{url}/repos/o/r/releases"))
            .json(&json!({"tag_name": tag}))
            .send()
            .unwrap();
    }
    let resp = client()
        .get(format!("{url}/repos/o/r/releases"))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let list: Vec<serde_json::Value> = resp.json().unwrap();
    assert_eq!(list.len(), 2);
    fake.stop();
}

#[test]
fn update_release_changes_fields() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created: serde_json::Value = client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&json!({"tag_name": "v1.2.3"}))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let id = created["id"].as_u64().unwrap();
    let updated = client()
        .patch(format!("{url}/repos/o/r/releases/{id}"))
        .json(&json!({"name": "new name", "body": "new body", "draft": false, "prerelease": true}))
        .send()
        .unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let got: serde_json::Value = updated.json().unwrap();
    assert_eq!(got["name"], "new name");
    assert_eq!(got["body"], "new body");
    assert_eq!(got["draft"], false);
    assert_eq!(got["prerelease"], true);
    fake.stop();
}

#[test]
fn update_missing_release_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client()
        .patch(format!("{url}/repos/o/r/releases/999"))
        .json(&json!({"name": "x"}))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn delete_release_removes_it() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created: serde_json::Value = client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&json!({"tag_name": "v1.2.3"}))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let id = created["id"].as_u64().unwrap();
    let del = client()
        .delete(format!("{url}/repos/o/r/releases/{id}"))
        .send()
        .unwrap();
    assert_eq!(del.status().as_u16(), 204);
    let get = client()
        .get(format!("{url}/repos/o/r/releases/tags/v1.2.3"))
        .send()
        .unwrap();
    assert_eq!(get.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn list_assets_empty_initially() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created: serde_json::Value = client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&json!({"tag_name": "v1.2.3"}))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let id = created["id"].as_u64().unwrap();
    let resp = client()
        .get(format!("{url}/repos/o/r/releases/{id}/assets"))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let assets: Vec<serde_json::Value> = resp.json().unwrap();
    assert!(assets.is_empty());
    fake.stop();
}

#[test]
fn list_assets_missing_release_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client()
        .get(format!("{url}/repos/o/r/releases/999/assets"))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn upload_asset_and_list_it() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created: serde_json::Value = client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&json!({"tag_name": "v1.2.3"}))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let id = created["id"].as_u64().unwrap();
    let upload_url = created["upload_url"].as_str().unwrap();
    let base = upload_url.split('{').next().unwrap();
    let mut u = reqwest::Url::parse(base).unwrap();
    u.query_pairs_mut().append_pair("name", "artifact.txt");
    let upload = client()
        .post(u)
        .body(b"hello".to_vec())
        .header("Content-Type", "application/octet-stream")
        .send()
        .unwrap();
    assert_eq!(upload.status().as_u16(), 201);
    let asset: serde_json::Value = upload.json().unwrap();
    assert_eq!(asset["name"], "artifact.txt");
    assert_eq!(asset["size"], 5);

    let list = client()
        .get(format!("{url}/repos/o/r/releases/{id}/assets"))
        .send()
        .unwrap();
    let assets: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0]["name"], "artifact.txt");
    fake.stop();
}

#[test]
fn upload_asset_replaces_same_name() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created: serde_json::Value = client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&json!({"tag_name": "v1.2.3"}))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let id = created["id"].as_u64().unwrap();
    let upload_url = created["upload_url"].as_str().unwrap();
    let base = upload_url.split('{').next().unwrap();
    for body in [b"a".to_vec(), b"bb".to_vec()] {
        let mut u = reqwest::Url::parse(base).unwrap();
        u.query_pairs_mut().append_pair("name", "a.txt");
        client().post(u).body(body).send().unwrap();
    }
    let list = client()
        .get(format!("{url}/repos/o/r/releases/{id}/assets"))
        .send()
        .unwrap();
    let assets: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0]["size"], 2);
    fake.stop();
}

#[test]
fn upload_asset_missing_release_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let mut u = reqwest::Url::parse(&format!("{url}/upload/999")).unwrap();
    u.query_pairs_mut().append_pair("name", "a.txt");
    let resp = client().post(u).body(b"x".to_vec()).send().unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn delete_asset_removes_it() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created: serde_json::Value = client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&json!({"tag_name": "v1.2.3"}))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let id = created["id"].as_u64().unwrap();
    let upload_url = created["upload_url"].as_str().unwrap();
    let base = upload_url.split('{').next().unwrap();
    let mut u = reqwest::Url::parse(base).unwrap();
    u.query_pairs_mut().append_pair("name", "a.txt");
    let asset: serde_json::Value = client().post(u).body(b"x".to_vec()).send().unwrap().json().unwrap();
    let asset_id = asset["id"].as_u64().unwrap();

    let del = client()
        .delete(format!("{url}/repos/o/r/releases/assets/{asset_id}"))
        .send()
        .unwrap();
    assert_eq!(del.status().as_u16(), 204);

    let list = client()
        .get(format!("{url}/repos/o/r/releases/{id}/assets"))
        .send()
        .unwrap();
    let assets: Vec<serde_json::Value> = list.json().unwrap();
    assert!(assets.is_empty());
    fake.stop();
}

#[test]
fn git_ref_tag_returns_sha() {
    let mut fake = FakeGitHub::new();
    fake.seed_tag("v1.2.3", "deadbeef");
    let url = base_url(&mut fake);
    let resp = client()
        .get(format!("{url}/repos/o/r/git/ref/tags/v1.2.3"))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["object"]["sha"], "deadbeef");
    assert_eq!(body["object"]["type"], "commit");
    fake.stop();
}

#[test]
fn git_ref_tag_missing_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client()
        .get(format!("{url}/repos/o/r/git/ref/tags/v9.9.9"))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn create_tag_ref_then_get() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created = client()
        .post(format!("{url}/repos/o/r/git/refs"))
        .json(&json!({"ref": "refs/tags/v1.2.3", "sha": "abc123"}))
        .send()
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let get = client()
        .get(format!("{url}/repos/o/r/git/ref/tags/v1.2.3"))
        .send()
        .unwrap();
    assert_eq!(get.status().as_u16(), 200);
    let body: serde_json::Value = get.json().unwrap();
    assert_eq!(body["object"]["sha"], "abc123");
    fake.stop();
}

#[test]
fn create_duplicate_tag_ref_returns_422() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    fake.seed_tag("v1.2.3", "abc");
    let resp = client()
        .post(format!("{url}/repos/o/r/git/refs"))
        .json(&json!({"ref": "refs/tags/v1.2.3", "sha": "def"}))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 422);
    fake.stop();
}

#[test]
fn delete_tag_ref_removes_it() {
    let mut fake = FakeGitHub::new();
    fake.seed_tag("v1.2.3", "abc");
    let url = base_url(&mut fake);
    let del = client()
        .delete(format!("{url}/repos/o/r/git/refs/tags/v1.2.3"))
        .send()
        .unwrap();
    assert_eq!(del.status().as_u16(), 204);
    let get = client()
        .get(format!("{url}/repos/o/r/git/ref/tags/v1.2.3"))
        .send()
        .unwrap();
    assert_eq!(get.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn annotated_tag_lookup_returns_404() {
    let mut fake = FakeGitHub::new();
    fake.seed_tag("v1.2.3", "abc");
    let url = base_url(&mut fake);
    let resp = client()
        .get(format!("{url}/repos/o/r/git/tags/abc"))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn url_encoded_tag_round_trips() {
    let mut fake = FakeGitHub::new();
    let tag = "release/1.0.0";
    fake.seed_tag(tag, "abc");
    let url = base_url(&mut fake);
    let resp = client()
        .get(format!("{url}/repos/o/r/git/ref/tags/release%2F1.0.0"))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    fake.stop();
}

#[test]
fn fake_list_releases_and_tags_methods() {
    let mut fake = FakeGitHub::new();
    fake.seed_tag("v1.0.0", "abc");
    let url = base_url(&mut fake);
    // create a release so list_releases has data
    client()
        .post(format!("{url}/repos/o/r/releases"))
        .json(&json!({"tag_name": "v1.0.0"}))
        .send()
        .unwrap();
    assert_eq!(fake.list_releases().len(), 1);
    assert!(fake.list_tags().contains(&"v1.0.0".to_string()));
    fake.stop();
}

#[test]
fn get_repository_metadata() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/octo/hello")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["name"], "hello");
    assert_eq!(body["full_name"], "octo/hello");
    assert_eq!(body["default_branch"], "main");
    fake.stop();
}

#[test]
fn list_commits_empty_initially() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/o/r/commits")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let list: Vec<serde_json::Value> = resp.json().unwrap();
    assert!(list.is_empty());
    fake.stop();
}

#[test]
fn seed_tag_records_commit_and_get_commit() {
    let mut fake = FakeGitHub::new();
    fake.seed_tag("v1.0.0", "deadbeef");
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/o/r/commits/deadbeef")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["sha"], "deadbeef");
    let list = client().get(format!("{url}/repos/o/r/commits")).send().unwrap();
    let commits: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(commits.len(), 1);
    fake.stop();
}

#[test]
fn get_missing_commit_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/o/r/commits/doesnotexist")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn create_issue_and_get() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created = client()
        .post(format!("{url}/repos/o/r/issues"))
        .json(&json!({"title": "bug", "body": "details", "labels": ["bug"]}))
        .send()
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let issue: serde_json::Value = created.json().unwrap();
    let number = issue["number"].as_u64().unwrap();
    assert_eq!(issue["title"], "bug");
    assert_eq!(issue["state"], "open");

    let got = client().get(format!("{url}/repos/o/r/issues/{number}")).send().unwrap();
    assert_eq!(got.status().as_u16(), 200);
    let got: serde_json::Value = got.json().unwrap();
    assert_eq!(got["title"], "bug");
    fake.stop();
}

#[test]
fn list_issues_returns_all() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    for title in ["a", "b"] {
        client().post(format!("{url}/repos/o/r/issues")).json(&json!({"title": title})).send().unwrap();
    }
    let resp = client().get(format!("{url}/repos/o/r/issues")).send().unwrap();
    let issues: Vec<serde_json::Value> = resp.json().unwrap();
    assert_eq!(issues.len(), 2);
    fake.stop();
}

#[test]
fn get_missing_issue_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/o/r/issues/999")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn update_issue_changes_fields() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let issue: serde_json::Value = client()
        .post(format!("{url}/repos/o/r/issues"))
        .json(&json!({"title": "bug", "body": "details"}))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let number = issue["number"].as_u64().unwrap();
    let updated = client()
        .patch(format!("{url}/repos/o/r/issues/{number}"))
        .json(&json!({"title": "fixed", "state": "closed", "labels": ["done"]}))
        .send()
        .unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let got: serde_json::Value = updated.json().unwrap();
    assert_eq!(got["title"], "fixed");
    assert_eq!(got["state"], "closed");
    fake.stop();
}

#[test]
fn update_missing_issue_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client()
        .patch(format!("{url}/repos/o/r/issues/999"))
        .json(&json!({"title": "x"}))
        .send()
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn issue_comments_create_and_list() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let issue: serde_json::Value = client()
        .post(format!("{url}/repos/o/r/issues"))
        .json(&json!({"title": "bug"}))
        .send()
        .unwrap()
        .json()
        .unwrap();
    let number = issue["number"].as_u64().unwrap();

    let c = client()
        .post(format!("{url}/repos/o/r/issues/{number}/comments"))
        .json(&json!({"body": "a comment"}))
        .send()
        .unwrap();
    assert_eq!(c.status().as_u16(), 201);

    let list = client()
        .get(format!("{url}/repos/o/r/issues/{number}/comments"))
        .send()
        .unwrap();
    assert_eq!(list.status().as_u16(), 200);
    let comments: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["body"], "a comment");
    fake.stop();
}

#[test]
fn issue_comments_on_missing_issue_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let get = client().get(format!("{url}/repos/o/r/issues/999/comments")).send().unwrap();
    assert_eq!(get.status().as_u16(), 404);
    let post = client()
        .post(format!("{url}/repos/o/r/issues/999/comments"))
        .json(&json!({"body": "x"}))
        .send()
        .unwrap();
    assert_eq!(post.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn get_authenticated_user_defaults_to_octocat() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/user")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["login"], "octocat");
    fake.stop();
}

#[test]
fn get_user_by_username() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/users/alice")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["login"], "alice");
    fake.stop();
}

#[test]
fn get_org() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/orgs/continuous-release")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["login"], "continuous-release");
    fake.stop();
}

#[test]
fn pagination_slices_issues() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    for i in 0..5 {
        client().post(format!("{url}/repos/o/r/issues")).json(&json!({"title": format!("issue {i}")})).send().unwrap();
    }
    let resp = client().get(format!("{url}/repos/o/r/issues?per_page=2&page=2")).send().unwrap();
    let issues: Vec<serde_json::Value> = resp.json().unwrap();
    assert_eq!(issues.len(), 2);
    fake.stop();
}

#[test]
fn authenticated_user_resolves_from_token() {
    let mut fake = FakeGitHub::new();
    fake.add_user(User { login: "alice".into(), id: 42, name: Some("Alice".into()), email: None, avatar_url: None }, Some("secret-token"));
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/user")).header("Authorization", "Bearer secret-token").send().unwrap();
    let body: serde_json::Value = resp.json().unwrap();
    assert_eq!(body["login"], "alice");
    assert_eq!(body["id"], 42);
    fake.stop();
}

#[test]
fn list_branches_includes_main() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/o/r/branches")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let branches: Vec<serde_json::Value> = resp.json().unwrap();
    assert!(branches.iter().any(|b| b["name"] == "main"));
    fake.stop();
}

#[test]
fn get_branch_main() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/o/r/branches/main")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    fake.stop();
}

#[test]
fn get_missing_branch_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/o/r/branches/nope")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn contents_round_trip() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"hello");
    let put = client()
        .put(format!("{url}/repos/o/r/contents/README.md"))
        .json(&json!({"content": b64}))
        .send()
        .unwrap();
    assert_eq!(put.status().as_u16(), 200);

    let get = client().get(format!("{url}/repos/o/r/contents/README.md")).send().unwrap();
    assert_eq!(get.status().as_u16(), 200);
    let body: serde_json::Value = get.json().unwrap();
    assert_eq!(body["encoding"], "base64");
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, body["content"].as_str().unwrap().replace('\n', "")).unwrap();
    assert_eq!(decoded, b"hello");
    fake.stop();
}

#[test]
fn get_missing_content_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/o/r/contents/nope.txt")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn create_and_get_pull() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created = client()
        .post(format!("{url}/repos/o/r/pulls"))
        .json(&json!({"title": "my pr", "body": "desc", "head": "feature", "base": "main"}))
        .send()
        .unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let pr: serde_json::Value = created.json().unwrap();
    let number = pr["number"].as_u64().unwrap();

    let got = client().get(format!("{url}/repos/o/r/pulls/{number}")).send().unwrap();
    assert_eq!(got.status().as_u16(), 200);
    let got: serde_json::Value = got.json().unwrap();
    assert_eq!(got["title"], "my pr");
    assert_eq!(got["head"]["ref"], "feature");
    fake.stop();
}

#[test]
fn list_pulls() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    client().post(format!("{url}/repos/o/r/pulls")).json(&json!({"title": "a", "head": "a", "base": "main"})).send().unwrap();
    client().post(format!("{url}/repos/o/r/pulls")).json(&json!({"title": "b", "head": "b", "base": "main"})).send().unwrap();
    let resp = client().get(format!("{url}/repos/o/r/pulls")).send().unwrap();
    let pulls: Vec<serde_json::Value> = resp.json().unwrap();
    assert_eq!(pulls.len(), 2);
    fake.stop();
}

#[test]
fn update_pull() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let pr: serde_json::Value = client().post(format!("{url}/repos/o/r/pulls")).json(&json!({"title": "old", "head": "f", "base": "main"})).send().unwrap().json().unwrap();
    let number = pr["number"].as_u64().unwrap();
    let updated = client().patch(format!("{url}/repos/o/r/pulls/{number}")).json(&json!({"title": "new", "state": "closed"})).send().unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let got: serde_json::Value = updated.json().unwrap();
    assert_eq!(got["title"], "new");
    assert_eq!(got["state"], "closed");
    fake.stop();
}

#[test]
fn merge_pull() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let pr: serde_json::Value = client().post(format!("{url}/repos/o/r/pulls")).json(&json!({"title": "p", "head": "f", "base": "main"})).send().unwrap().json().unwrap();
    let number = pr["number"].as_u64().unwrap();
    let merged = client().put(format!("{url}/repos/o/r/pulls/{number}/merge")).send().unwrap();
    assert_eq!(merged.status().as_u16(), 200);
    let body: serde_json::Value = merged.json().unwrap();
    assert_eq!(body["merged"], true);
    let got = client().get(format!("{url}/repos/o/r/pulls/{number}")).send().unwrap();
    let got: serde_json::Value = got.json().unwrap();
    assert_eq!(got["merged"], true);
    assert_eq!(got["state"], "closed");
    fake.stop();
}

#[test]
fn get_missing_pull_returns_404() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().get(format!("{url}/repos/o/r/pulls/999")).send().unwrap();
    assert_eq!(resp.status().as_u16(), 404);
    fake.stop();
}

#[test]
fn create_pull_review() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let pr: serde_json::Value = client().post(format!("{url}/repos/o/r/pulls")).json(&json!({"title": "p", "head": "f", "base": "main"})).send().unwrap().json().unwrap();
    let number = pr["number"].as_u64().unwrap();
    let review = client().post(format!("{url}/repos/o/r/pulls/{number}/reviews")).json(&json!({"event": "APPROVE", "body": "lgtm"})).send().unwrap();
    assert_eq!(review.status().as_u16(), 200);
    let list = client().get(format!("{url}/repos/o/r/pulls/{number}/reviews")).send().unwrap();
    let reviews: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(reviews.len(), 1);
    assert_eq!(reviews[0]["state"], "APPROVE");
    fake.stop();
}

#[test]
fn create_pull_comment() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let pr: serde_json::Value = client().post(format!("{url}/repos/o/r/pulls")).json(&json!({"title": "p", "head": "f", "base": "main"})).send().unwrap().json().unwrap();
    let number = pr["number"].as_u64().unwrap();
    let c = client().post(format!("{url}/repos/o/r/pulls/{number}/comments")).json(&json!({"body": "nit", "path": "src/lib.rs"})).send().unwrap();
    assert_eq!(c.status().as_u16(), 201);
    let list = client().get(format!("{url}/repos/o/r/pulls/{number}/comments")).send().unwrap();
    let comments: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["body"], "nit");
    fake.stop();
}

#[test]
fn save_and_load_state_round_trips() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    client().post(format!("{url}/repos/o/r/issues")).json(&json!({"title": "persisted"})).send().unwrap();
    let tmp = std::env::temp_dir().join("fake-github-state.json");
    fake.save_state(&tmp).unwrap();
    fake.stop();

    let mut fake2 = FakeGitHub::new();
    fake2.load_state(&tmp).unwrap();
    assert_eq!(fake2.list_issues().len(), 1);
    assert_eq!(fake2.list_issues()[0].title, "persisted");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn labels_crud() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created = client().post(format!("{url}/repos/o/r/labels")).json(&json!({"name": "bug", "color": "ff0000", "description": "a bug"})).send().unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let got = client().get(format!("{url}/repos/o/r/labels/bug")).send().unwrap();
    assert_eq!(got.status().as_u16(), 200);
    let list = client().get(format!("{url}/repos/o/r/labels")).send().unwrap();
    let labels: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(labels.len(), 1);
    let del = client().delete(format!("{url}/repos/o/r/labels/bug")).send().unwrap();
    assert_eq!(del.status().as_u16(), 204);
    fake.stop();
}

#[test]
fn milestones_crud() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created = client().post(format!("{url}/repos/o/r/milestones")).json(&json!({"title": "v1", "description": "first"})).send().unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let m: serde_json::Value = created.json().unwrap();
    let num = m["number"].as_u64().unwrap();
    let got = client().get(format!("{url}/repos/o/r/milestones/{num}")).send().unwrap();
    assert_eq!(got.status().as_u16(), 200);
    let updated = client().patch(format!("{url}/repos/o/r/milestones/{num}")).json(&json!({"state": "closed"})).send().unwrap();
    assert_eq!(updated.status().as_u16(), 200);
    let del = client().delete(format!("{url}/repos/o/r/milestones/{num}")).send().unwrap();
    assert_eq!(del.status().as_u16(), 204);
    fake.stop();
}

#[test]
fn commit_status_create_and_combined() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().post(format!("{url}/repos/o/r/statuses/abc123")).json(&json!({"state": "success", "context": "ci", "description": "ok"})).send().unwrap();
    assert_eq!(resp.status().as_u16(), 201);
    let list = client().get(format!("{url}/repos/o/r/commits/abc123/statuses")).send().unwrap();
    let statuses: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(statuses.len(), 1);
    let combined = client().get(format!("{url}/repos/o/r/commits/abc123/status")).send().unwrap();
    let body: serde_json::Value = combined.json().unwrap();
    assert_eq!(body["state"], "success");
    fake.stop();
}

#[test]
fn webhook_create_list_delete() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created = client().post(format!("{url}/repos/o/r/hooks")).json(&json!({"config": {"url": "http://example.com/hook"}, "events": ["push"], "active": true})).send().unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let h: serde_json::Value = created.json().unwrap();
    let id = h["id"].as_u64().unwrap();
    let list = client().get(format!("{url}/repos/o/r/hooks")).send().unwrap();
    let hooks: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(hooks.len(), 1);
    let del = client().delete(format!("{url}/repos/o/r/hooks/{id}")).send().unwrap();
    assert_eq!(del.status().as_u16(), 204);
    fake.stop();
}

#[test]
fn check_run_create_and_list() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created = client().post(format!("{url}/repos/o/r/check-runs")).json(&json!({"name": "build", "head_sha": "abc", "status": "completed", "conclusion": "success"})).send().unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let list = client().get(format!("{url}/repos/o/r/commits/abc/check-runs")).send().unwrap();
    let body: serde_json::Value = list.json().unwrap();
    assert_eq!(body["total_count"], 1);
    fake.stop();
}

#[test]
fn deployment_create_and_list() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created = client().post(format!("{url}/repos/o/r/deployments")).json(&json!({"sha": "abc", "ref": "main", "environment": "prod"})).send().unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let list = client().get(format!("{url}/repos/o/r/deployments")).send().unwrap();
    let deps: Vec<serde_json::Value> = list.json().unwrap();
    assert_eq!(deps.len(), 1);
    fake.stop();
}

#[test]
fn ruleset_crud() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let created = client().post(format!("{url}/repos/o/r/rulesets")).json(&json!({"name": "protect", "target": "branch", "enforcement": "active", "conditions": {}, "rules": []})).send().unwrap();
    assert_eq!(created.status().as_u16(), 201);
    let r: serde_json::Value = created.json().unwrap();
    let id = r["id"].as_u64().unwrap();
    let got = client().get(format!("{url}/repos/o/r/rulesets/{id}")).send().unwrap();
    assert_eq!(got.status().as_u16(), 200);
    let del = client().delete(format!("{url}/repos/o/r/rulesets/{id}")).send().unwrap();
    assert_eq!(del.status().as_u16(), 204);
    fake.stop();
}

#[test]
fn branch_protection_round_trip() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let put = client().put(format!("{url}/repos/o/r/branches/main/protection")).json(&json!({"required_status_checks": {"contexts": ["ci"]}, "enforce_admins": true, "required_pull_request_reviews": {}})).send().unwrap();
    assert_eq!(put.status().as_u16(), 200);
    let got = client().get(format!("{url}/repos/o/r/branches/main/protection")).send().unwrap();
    assert_eq!(got.status().as_u16(), 200);
    let del = client().delete(format!("{url}/repos/o/r/branches/main/protection")).send().unwrap();
    assert_eq!(del.status().as_u16(), 204);
    fake.stop();
}

#[test]
fn graphql_endpoint() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    let resp = client().post(format!("{url}/graphql")).json(&json!({"query": "{ viewer { login } }"})).send().unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().unwrap();
    assert!(body.get("data").is_some());
    fake.stop();
}

#[test]
fn rate_limit_and_meta() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    assert_eq!(client().get(format!("{url}/rate_limit")).send().unwrap().status().as_u16(), 200);
    assert_eq!(client().get(format!("{url}/meta")).send().unwrap().status().as_u16(), 200);
    assert_eq!(client().get(format!("{url}/octocat")).send().unwrap().status().as_u16(), 200);
    fake.stop();
}

#[test]
fn events_recorded_on_issue_create() {
    let mut fake = FakeGitHub::new();
    let url = base_url(&mut fake);
    client().post(format!("{url}/repos/o/r/issues")).json(&json!({"title": "bug"})).send().unwrap();
    let events = client().get(format!("{url}/repos/o/r/events")).send().unwrap();
    let ev: Vec<serde_json::Value> = events.json().unwrap();
    assert!(!ev.is_empty());
    fake.stop();
}
