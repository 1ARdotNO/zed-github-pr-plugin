# zed-github-pr

**Manage GitHub pull requests from inside Zed.**

A Zed extension that ships an MCP context server for GitHub PRs: list and filter
pending PRs, inspect a PR in detail, and switch between your GitHub accounts —
all from Zed's Agent Panel. Notifications on PR updates are on the roadmap.

> Status: **early, usable.** PR listing/filtering/detail and multi-account work
> today. See [`TODO.md`](TODO.md) for the roadmap and [`DESIGN.md`](DESIGN.md) for
> why it's built as an MCP server.

## Why an MCP server?

Zed extensions can't draw custom panels. They *can* register a **context server**
that the Agent Panel talks to. So all the PR logic lives in a small native binary
(`ghpr-mcp`) that the extension launches. See [`DESIGN.md`](DESIGN.md).

## Requirements

- [Zed](https://zed.dev)
- [Rust toolchain](https://rustup.rs) (to build the server)
- [`gh`](https://cli.github.com), authenticated: `gh auth login`. `gh` owns
  credential storage and multi-account support.

## Install

```sh
# 1. Authenticate GitHub (once). Add more accounts anytime with the same command.
gh auth login

# 2. Build and install the MCP server onto your PATH (~/.cargo/bin).
cargo install --path crates/ghpr-mcp

# 3. Install the extension in Zed:
#    Extensions → Install Dev Extension → select this repository's root folder.
```

The extension launches `ghpr-mcp` from your `PATH`. (Auto-downloading a prebuilt
binary from releases is planned; for now the `cargo install` step above is how you
get it.)

## Use

Open Zed's **Agent Panel** and ask, e.g.:

- "List the open PRs that need my review."
- "Show open PRs authored by me across my accounts."
- "Show PR #42 in detail."

The agent calls these tools:

| Tool | What it does |
|------|--------------|
| `list_pull_requests` | List/filter PRs. Params: `repo`, `state`, `author`, `assignee`, `label`, `search`, `view`, `limit`, `account`. |
| `pr_detail` | One PR's body, reviews, checks, and changed files. Params: `number` (required), `repo`, `account`. |
| `list_accounts` | The GitHub accounts `gh` is signed into. |
| `notification_settings` | Show the effective notification config. |

**Named views** (the `view` param): `needs-my-review`, `mine`, `assigned-to-me`,
`involves-me` — each expands to a GitHub search and can be combined with `search`.

**Multiple accounts:** pass `account: "<login>"` to scope a request to one of your
`gh` accounts (see `list_accounts`); it's resolved to that account's token for that
call only, without changing your active `gh` account.

## Terminal (CLI) mode

The same binary works directly from a terminal — no Zed, no AI. Run it with a
subcommand (with no subcommand it speaks MCP, which is how the extension uses it):

```sh
ghpr-mcp prs --view needs-my-review     # filtered PR list
ghpr-mcp prs --repo owner/name --state all --account other-login
ghpr-mcp pr 42                          # one PR in detail (JSON)
ghpr-mcp accounts                       # authenticated gh accounts
```

## Configuration

Notification behaviour is configured in JSON at `$GHPR_CONFIG`, or
`~/.config/ghpr-mcp/config.json`. Any omitted field uses its default:

```json
{
  "poll_interval_secs": 120,
  "comment_cooldown_secs": 300,
  "repos": ["owner/name"],
  "suppress_self": true,
  "exclude_bots": true,
  "silenced_users": ["some-bot"],
  "events": { "approved_ready": true, "new_comment": true, "new_commit": true }
}
```

Run the `notification_settings` tool to see the effective, defaults-merged config.
(The polling/delivery engine that consumes this is in progress — see the roadmap.)

## Development

```sh
cargo build --workspace                 # build the MCP server
cargo test --workspace                  # tests
cargo clippy --all-targets -- -D warnings
cargo build --target wasm32-wasip1 --manifest-path crates/zed-ghpr/Cargo.toml
```

## License

MIT — see [`LICENSE`](LICENSE).
