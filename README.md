# fake-github

An in-memory GitHub REST (and partial GraphQL) API server used as a test
double and lightweight emulator for the continuous-release toolkit, its
harness, and any other client.

It is a real HTTP server, so it can back Rust, TypeScript, Python, or any
other language client. State is held in memory and can be persisted to JSON.

## Library

```rust
use fake_github::FakeGitHub;

let mut fake = FakeGitHub::new();
let url = fake.start().unwrap();
// point your client at `url`
fake.seed_tag("v1.2.3", "abc123");
// ... run code under test ...
fake.save_state(std::path::Path::new("state.json")).unwrap();
fake.stop();
```

Multi-repo, users, and orgs:

```rust
let mut fake = FakeGitHub::with_default_repo("octo", "hello");
fake.add_user(fake_github::User { login: "alice".into(), ..Default::default() }, Some("token"));
fake.create_repo("octo", "other");
```

## Binary

```sh
cargo run --bin fake-github
# prints http://127.0.0.1:<port>

# Load seed state and save on exit:
cargo run --bin fake-github -- --state seed.json --save out.json
```

## Covered API surface

See [docs/api-coverage.md](docs/api-coverage.md). Summary:

- Repository metadata, branches, contents
- Releases, release assets
- Git data: refs, commits, trees, blobs
- Issues, labels, milestones, comments
- Pull requests, reviews, comments, merge
- Commit statuses, check runs
- Deployments
- GitHub Actions: workflows, runs, dispatch, rerun
- Branch protection and rulesets (repo + org)
- Webhooks (with best-effort delivery)
- Merge queue
- Users, organizations
- Events, environments, search
- Rate limit, meta, octocat, GraphQL stub

## Persistence

State serializes to JSON via `save_state`/`load_state`, so an emulator instance
can be seeded once and reused.

## Tests

```sh
cargo test
```
12 versions of incremental expansion; 69 integration tests cover the
implemented endpoints.
