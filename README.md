# fake-github

A small in-memory GitHub API server used as a test double by the
continuous-release toolkit and harness.

It implements the subset of the GitHub REST API that `cr` uses: tags, releases,
assets, drafts, and prereleases. Unlike Node-only mock libraries, it is a real
HTTP server, so it can back Rust, TypeScript, Python, or any other client.

## Library

```rust
use fake_github::FakeGitHub;

let mut fake = FakeGitHub::new();
let url = fake.start().unwrap();
// point your client at `url`
fake.seed_tag("v1.2.3", "abc123");
// ... run code under test ...
fake.stop();
```

## Binary

```sh
cargo run --bin fake-github
# prints http://127.0.0.1:<port> and serves until stdin closes
```
