# fake-github API coverage

This document tracks which GitHub REST API endpoints `continuous-release/fake-github`
implements. The fake is a **test double / lightweight emulator**: it covers the
subset needed by the continuous-release toolkit and harness, plus a broad set of
general-purpose endpoints. Coverage against the *entire* GitHub REST API is
therefore still partial, but the release-relevant surface is effectively
complete.

Source of truth for the client side: `continuous-release/toolkit` `src/release.rs`
(`ReleaseClient`) and `continuous-release/harness` `src/github.rs`. Source of
truth for the server side: `src/lib.rs` (`handle_request`).

## Status legend

- **Full** — implemented and exercised by an integration test in `tests/api.rs`.
- **Partial** — implemented but with known limitations.
- **Missing** — not implemented.

## Implemented endpoints

### Auth / users / orgs

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/user` | Full | Resolves from bearer token; defaults to `octocat`. |
| GET | `/users/{username}` | Full | Synthetic if unknown. |
| GET | `/orgs/{org}` | Full | Synthetic if unknown. |
| GET | `/orgs/{org}/rulesets` | Full | |
| POST | `/orgs/{org}/rulesets` | Full | |

### Repository

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}` | Full | Synthetic metadata. |
| GET | `/repos/{owner}/{repo}/branches` | Full | |
| GET | `/repos/{owner}/{repo}/branches/{branch}` | Full | 200/404. |
| GET | `/repos/{owner}/{repo}/branches/{branch}/protection` | Full | |
| PUT | `/repos/{owner}/{repo}/branches/{branch}/protection` | Full | |
| DELETE | `/repos/{owner}/{repo}/branches/{branch}/protection` | Full | |
| GET | `/repos/{owner}/{repo}/contents/{path}` | Full | Base64 round-trip. 404 when absent. |
| PUT/POST | `/repos/{owner}/{repo}/contents/{path}` | Full | Create/update. |
| DELETE | `/repos/{owner}/{repo}/contents/{path}` | Full | |
| GET | `/repos/{owner}/{repo}/events` | Full | Recorded events. |
| GET | `/repos/{owner}/{repo}/environments` | Full | |
| PUT | `/repos/{owner}/{repo}/environments` | Full | |
| GET | `/repos/{owner}/{repo}/rulesets` | Full | |
| POST | `/repos/{owner}/{repo}/rulesets` | Full | |
| GET | `/repos/{owner}/{repo}/rulesets/{id}` | Full | |
| PUT | `/repos/{owner}/{repo}/rulesets/{id}` | Full | |
| DELETE | `/repos/{owner}/{repo}/rulesets/{id}` | Full | |
| GET | `/repos/{owner}/{repo}/hooks` | Full | |
| POST | `/repos/{owner}/{repo}/hooks` | Full | |
| DELETE | `/repos/{owner}/{repo}/hooks/{id}` | Full | |
| GET | `/repos/{owner}/{repo}/merge-queue` | Full | |
| POST | `/repos/{owner}/{repo}/merge-queue` | Full | Enqueue PR. |

### Releases

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/releases/tags/{tag}` | Full | 200/404. Tag is URL-decoded. |
| GET | `/repos/{owner}/{repo}/releases` | Partial | `per_page`/pagination ignored. |
| POST | `/repos/{owner}/{repo}/releases` | Full | `tag_name`, `target_commitish`, `name`, `body`, `draft`, `prerelease`. 422 `already_exists`. |
| PATCH | `/repos/{owner}/{repo}/releases/{id}` | Full | `name`, `body`, `draft`, `prerelease`. 404 when absent. |
| DELETE | `/repos/{owner}/{repo}/releases/{id}` | Full | 204. |

### Release assets

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/releases/{id}/assets` | Full | 200/404. |
| GET | `/repos/{owner}/{repo}/releases/assets/{id}` | Full | Download bytes. |
| POST | upload URL (`/upload/{id}?name=...`) | Full | Replaces an existing asset with the same name. 404 when release absent. |
| DELETE | `/repos/{owner}/{repo}/releases/assets/{id}` | Full | 204. |

### Git data

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/git/refs` | Full | Lists all refs (paginated). |
| GET | `/repos/{owner}/{repo}/git/refs/tags` | Full | Paginated. |
| GET | `/repos/{owner}/{repo}/git/refs/heads` | Full | Paginated. |
| GET | `/repos/{owner}/{repo}/git/ref/tags/{tag}` | Full | Lightweight tag; object type `commit`. 404 when absent. |
| POST | `/repos/{owner}/{repo}/git/refs` | Full | Creates `refs/tags/{tag}`. 422 when exists. |
| DELETE | `/repos/{owner}/{repo}/git/refs/tags/{tag}` | Full | 204. |
| DELETE | `/repos/{owner}/{repo}/git/refs/heads/{branch}` | Full | 204. |
| GET | `/repos/{owner}/{repo}/git/tags/{sha}` | Full | Annotated tag objects; peels to the target commit. 404 when absent. |
| GET | `/repos/{owner}/{repo}/commits` | Full | Returns seeded commits. Paginated. |
| GET | `/repos/{owner}/{repo}/commits/{sha}` | Full | 200/404. |
| GET | `/repos/{owner}/{repo}/git/trees/{sha}` | Full | Stub (empty tree). |
| GET | `/repos/{owner}/{repo}/git/blobs/{sha}` | Full | Stub. |

### Issues

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/issues` | Full | Paginated. |
| GET | `/repos/{owner}/{repo}/issues/{number}` | Full | 200/404. |
| POST | `/repos/{owner}/{repo}/issues` | Full | `title`, `body`, `labels`. |
| PATCH | `/repos/{owner}/{repo}/issues/{number}` | Full | `title`, `body`, `state`, `labels`. 404 when absent. |
| GET | `/repos/{owner}/{repo}/issues/{number}/comments` | Full | 200/404 (issue). |
| POST | `/repos/{owner}/{repo}/issues/{number}/comments` | Full | 201/404 (issue). |

### Labels & milestones

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/labels` | Full | |
| GET | `/repos/{owner}/{repo}/labels/{name}` | Full | 200/404. |
| POST | `/repos/{owner}/{repo}/labels` | Full | |
| DELETE | `/repos/{owner}/{repo}/labels/{name}` | Full | 204. |
| GET | `/repos/{owner}/{repo}/milestones` | Full | |
| GET | `/repos/{owner}/{repo}/milestones/{number}` | Full | 200/404. |
| POST | `/repos/{owner}/{repo}/milestones` | Full | |
| PATCH | `/repos/{owner}/{repo}/milestones/{number}` | Full | |
| DELETE | `/repos/{owner}/{repo}/milestones/{number}` | Full | 204. |

### Pull requests

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/pulls` | Full | |
| GET | `/repos/{owner}/{repo}/pulls/{number}` | Full | 200/404. |
| POST | `/repos/{owner}/{repo}/pulls` | Full | |
| PATCH | `/repos/{owner}/{repo}/pulls/{number}` | Full | |
| PUT | `/repos/{owner}/{repo}/pulls/{number}/merge` | Full | |
| GET | `/repos/{owner}/{repo}/pulls/{number}/reviews` | Full | |
| POST | `/repos/{owner}/{repo}/pulls/{number}/reviews` | Full | |
| GET | `/repos/{owner}/{repo}/pulls/{number}/comments` | Full | |
| POST | `/repos/{owner}/{repo}/pulls/{number}/comments` | Full | |

### Commit statuses & check runs

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/commits/{sha}/statuses` | Full | |
| GET | `/repos/{owner}/{repo}/commits/{sha}/status` | Full | Combined status. |
| POST | `/repos/{owner}/{repo}/statuses/{sha}` | Full | |
| GET | `/repos/{owner}/{repo}/commits/{sha}/check-runs` | Full | |
| POST | `/repos/{owner}/{repo}/check-runs` | Full | |

### Deployments

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/deployments` | Full | |
| POST | `/repos/{owner}/{repo}/deployments` | Full | |
| GET | `/repos/{owner}/{repo}/deployments/{id}/statuses` | Full | Stub (empty). |
| POST | `/repos/{owner}/{repo}/deployments/{id}/statuses` | Full | |

### GitHub Actions

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/actions/workflows` | Full | |
| POST | `/repos/{owner}/{repo}/actions/workflows/{id}/dispatches` | Full | |
| GET | `/repos/{owner}/{repo}/actions/runs` | Full | |
| GET | `/repos/{owner}/{repo}/actions/workflows/{workflow}/runs` | Full | Filtered by workflow filename. |
| GET | `/repos/{owner}/{repo}/actions/runs/{id}` | Full | |
| POST | `/repos/{owner}/{repo}/actions/runs/{id}/rerun` | Full | |
| GET | `/repos/{owner}/{repo}/actions/runs/{id}/jobs` | Full | Stub (empty). |
| GET | `/repos/{owner}/{repo}/actions/runs/{id}/artifacts` | Full | Stub (empty). |

### Merge queue

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/merge-queue` | Full | |
| POST | `/repos/{owner}/{repo}/merge-queue` | Full | Enqueue PR. |

### Pagination

List endpoints accept `page` and `per_page` query params and return a sliced
page. `Link` headers are emitted for releases, commits, issues, and git refs.

### Other

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/events` | Full | Recorded events. |
| GET | `/search/issues` | Full | Basic title match. |
| POST | `/graphql` | Partial | Stub returning viewer/repository/search shapes. |
| GET | `/rate_limit` | Full | |
| GET | `/meta` | Full | |
| GET | `/octocat` | Full | |

## Not implemented (GitHub REST API surface)

The following domains are **not** implemented. Contributions adding them (with
tests) are welcome.

- Actions: secrets, variables, runners, caches, artifacts upload/download
- Activity: notifications, starring, watching
- Apps / GitHub Apps / installations
- Checks: check suites (only check runs)
- Code scanning / secret scanning
- Collaborators / invitations
- Commit comments / statuses contexts
- Dependabot
- Gists
- Git database: blobs/trees creation (only lookup)
- Issues: reactions, timelines, assignees, milestones linking, search advanced
- Organizations: members, teams
- Packages (GitHub Packages registry)
- Projects
- Reactions
- Repositories: branches protection advanced, contents history, forks, topics, webhooks delivery
- Search: code, commits, repositories, users
- Users: followers, gists
- Webhooks: actual delivery (best-effort POST on release/issue/PR/workflow events only)
- Enterprise administration

## Test coverage

Integration tests live in `tests/api.rs` (69 tests) and exercise most **Full**
endpoints against a running `FakeGitHub` instance. Run with `cargo test`.
