# fake-github API coverage

This document tracks which GitHub REST API endpoints `continuous-release/fake-github`
implements. The fake is a **test double**, not a full GitHub emulator: it covers
the subset needed by the continuous-release toolkit and harness, plus a few
general-purpose endpoints. Coverage against the *entire* GitHub REST API is
therefore intentionally small.

Source of truth for the client side: `continuous-release/toolkit` `src/release.rs`
(`ReleaseClient`). Source of truth for the server side: `src/lib.rs`
(`handle_request`).

## Status legend

- **Full** — implemented and exercised by an integration test in `tests/api.rs`.
- **Partial** — implemented but with known limitations.
- **Missing** — not implemented.

## Implemented endpoints (Full unless noted)

### Repository

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}` | Full | Synthetic metadata (name, full_name, owner, default_branch). |

### Releases

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/releases/tags/{tag}` | Full | 200/404. Tag is URL-decoded. |
| GET | `/repos/{owner}/{repo}/releases` | Partial | Returns all releases; `per_page`/pagination ignored. |
| GET | `/repos/{owner}/{repo}/releases/{id}` | Missing | Not used by the toolkit. |
| POST | `/repos/{owner}/{repo}/releases` | Full | `tag_name`, `target_commitish`, `name`, `body`, `draft`, `prerelease`. 422 `already_exists` on duplicate tag. |
| PATCH | `/repos/{owner}/{repo}/releases/{id}` | Full | `name`, `body`, `draft`, `prerelease`. 404 when absent. |
| DELETE | `/repos/{owner}/{repo}/releases/{id}` | Full | 204. |

### Release assets

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/releases/{id}/assets` | Full | 200/404. |
| POST | upload URL (`/upload/{id}?name=...`) | Full | Replaces an existing asset with the same name. 404 when release absent. |
| DELETE | `/repos/{owner}/{repo}/releases/assets/{id}` | Full | 204. Removes from all releases. |

### Git data

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/git/ref/tags/{tag}` | Full | Lightweight tag; object type `commit`. 404 when absent. |
| POST | `/repos/{owner}/{repo}/git/refs` | Full | Creates `refs/tags/{tag}`. 422 when the ref already exists. |
| DELETE | `/repos/{owner}/{repo}/git/refs/tags/{tag}` | Full | 204. |
| GET | `/repos/{owner}/{repo}/git/tags/{sha}` | Partial | Always 404 (annotated tags are not modelled). |
| GET | `/repos/{owner}/{repo}/commits` | Full | Returns seeded commits. |
| GET | `/repos/{owner}/{repo}/commits/{sha}` | Full | 200/404. |

### Issues

| Method | Path | Status | Notes |
| --- | --- | --- | --- |
| GET | `/repos/{owner}/{repo}/issues` | Full | |
| GET | `/repos/{owner}/{repo}/issues/{number}` | Full | 200/404. |
| POST | `/repos/{owner}/{repo}/issues` | Full | `title`, `body`, `labels`. |
| PATCH | `/repos/{owner}/{repo}/issues/{number}` | Full | `title`, `body`, `state`, `labels`. 404 when absent. |
| GET | `/repos/{owner}/{repo}/issues/{number}/comments` | Full | 200/404 (issue). |
| POST | `/repos/{owner}/{repo}/issues/{number}/comments` | Full | 201/404 (issue). |

## Not implemented (GitHub REST API surface)

The following domains are **not** implemented. They are listed so callers know
what to expect; contributions adding them (with tests) are welcome.

- Actions (workflows, runs, artifacts, secrets, variables)
- Activity (events, notifications, starring, watching)
- Apps / GitHub Apps / installations
- Billing
- Checks (check runs, check suites)
- Code scanning / secret scanning
- Codespaces
- Collaborators / invitations
- Commits (commit comments, statuses, branches) — only basic commit lookup
- Dependabot
- Deployments / environments
- Emojis
- Gists
- Git database (blobs, trees) — only tag refs and commits
- Gitignore / licenses
- Interactions
- Issues (reactions, timelines, assignees, milestones) — only CRUD + comments
- Markdown
- Meta / rate limit / versions
- Migrations
- Organizations / teams / members
- Packages
- Projects (classic & beta)
- Pull requests (and reviews, comments, requested reviewers)
- Reactions
- Repositories (branches, contents, forks, tags listing, topics, webhooks, etc.)
- Search
- Teams (discussions)
- Users
- Enterprise administration

## Known limitations

- No pagination (`Link` header; `page`/`per_page` ignored).
- No annotated tags.
- No repository metadata beyond a synthetic object.
- No rate limiting, conditional requests (ETag), or authentication checks.
- Upload URL is a synthetic `/upload/{id}` path rather than a real
  `uploads.github.com` host; the toolkit derives it from `upload_url`, so this
  is transparent.
- Commits are recorded only when a tag is seeded or created; arbitrary commit
  history is not modelled.

## Test coverage

Integration tests live in `tests/api.rs` and exercise every **Full** endpoint
above against a running `FakeGitHub` instance. Run with `cargo test`.
