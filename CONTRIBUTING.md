# Contributing

## Setup

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Ground rules

- CI (fmt, clippy `-D warnings`, tests, coverage, Trivy, CodeQL, MegaLinter) must
  be green. Renovate automerges dependency PRs once green.
- Keep the WASM extension crate (`zed-ghpr`) thin — logic belongs in `ghpr-mcp`.
- Commits: conventional-ish, imperative mood.

## Architecture

See [`DESIGN.md`](DESIGN.md) before adding features.
