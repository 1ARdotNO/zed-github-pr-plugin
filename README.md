# zed-github-pr

**Manage GitHub pull requests from inside Zed.**

A Zed extension that ships an MCP context server for GitHub PRs: list and filter
pending PRs, inspect a PR in detail, switch between your GitHub accounts, get
desktop notifications on updates, and open a review — from Zed's Agent Panel, a
terminal CLI, or an interactive dashboard.

📖 **Docs: <https://1ardotno.github.io/zed-github-pr-plugin/>**

> Status: **usable.** PR listing/filtering/detail, multi-account, notifications,
> review shortcut, CLI, and TUI all work today. See [`TODO.md`](TODO.md) for the
> roadmap and [`DESIGN.md`](DESIGN.md) for why it's built as an MCP server.

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

On linux/macOS the extension **auto-downloads** the matching `ghpr-mcp` binary
from GitHub Releases, so step 2 is optional there once a release is published. It
falls back to `ghpr-mcp` on your `PATH` (the `cargo install` above) when no release
asset fits — which also covers Windows and running from source.

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
ghpr-mcp review 42 --repo owner/name    # print a review prompt (title+body+diff)
ghpr-mcp tui                            # interactive PR dashboard (repo inferred from cwd)
ghpr-mcp watch --repo owner/name        # poll and notify on PR changes (--once for one cycle)
```

`tui` is a keyboard-driven dashboard of open PRs (color-coded checks, approval,
age, diff stats): `j`/`k` or arrows to move, `g`/`G` for top/bottom, `Enter` to
open in the browser, `r` to refresh, `?` for help, `q` to quit. With no `--repo`
it uses the current repo (gh infers it from the working directory).

### Dashboard inside Zed

Zed extensions can't draw panels, but you can run the dashboard in Zed's built-in
terminal as a persistent, one-key panel. This repo ships a [`.zed/tasks.json`](.zed/tasks.json)
task named **"GitHub PRs"** (runs `ghpr-mcp tui` in the worktree, so it targets
whatever repo you have open). Bind it to a key in your Zed `keymap.json`:

```json
[{ "bindings": { "cmd-shift-g": ["task::Spawn", { "task_name": "GitHub PRs" }] } }]
```

Now `cmd-shift-g` opens the PR dashboard in Zed's terminal. (Extensions can't spawn
terminals themselves; Zed's task system is the supported path — see the roadmap.)

`watch` diffs each poll against a saved snapshot, fires notifications for
notification-worthy changes (approved & ready, checks status, …), and applies your
config's noise controls (suppress-self, exclude-bots, silence list, comment cooldown).

`review` (and the `review_pr` MCP tool) render `review.prompt_template` from your
config — placeholders `{repo}`, `{number}`, `{title}`, `{body}`, `{diff}`. In Zed,
ask the agent to "review PR 42" and it runs the assembled prompt; from a terminal,
pipe the output into your reviewer of choice.

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
  "desktop_notifications": true,
  "tui_refresh_secs": 60,
  "events": { "approved_ready": true, "new_comment": true, "new_commit": true },
  "review": { "prompt_template": "Review {repo}#{number} — {title}\n{diff}" }
}
```

`ghpr-mcp watch` fires native desktop notifications (macOS `osascript`, Linux
`notify-send`) for each change that passes your filters; set
`desktop_notifications: false` for stdout only. Zed has no extension notification
API, so these are OS-native. Run the `notification_settings` tool to see the
effective, defaults-merged config.

## Development

```sh
cargo build --workspace                 # build the MCP server
cargo test --workspace                  # tests
cargo clippy --all-targets -- -D warnings
cargo build --target wasm32-wasip1 --manifest-path crates/zed-ghpr/Cargo.toml
```

## License

MIT — see [`LICENSE`](LICENSE).
