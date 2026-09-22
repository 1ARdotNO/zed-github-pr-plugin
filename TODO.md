# TODO — zed-github-pr-plugin

Canonical backlog for this project. Every instruction lands here first, then gets
worked in the order that makes sense. Checked = done, `~` = in progress.

## How we work (the loop)
- Instructions become TODO items here before anything else.
- Work items in dependency order, not arrival order.
- Each iteration: do the work, commit to a branch, push, open a PR with auto-merge.
  CI must pass; it automerges when green. If CI fails, **fix the real problem** —
  never disable or weaken a test to go green.
- Periodically check GitHub issues on `1ARdotNO/zed-github-pr-plugin`; act only on
  issues opened by **1ardotno** (ignore everyone else's).
- CI (fmt, clippy, tests, coverage, Trivy, MegaLinter, CodeQL) is the safety net;
  Renovate automerges dependency PRs that stay green.

## Phase 0 — Bootstrap  ~
- [x] Study wyrm (Zed extension) + jync (renovate/CI) as the reference setup
- [x] Confirm Zed extension capability envelope (MCP context server is the path)
- [x] Set git remote to the GitHub repo
- [x] Scaffold repo: LICENSE, README, SECURITY, CONTRIBUTING, .gitignore
- [x] CI/security: ci, coverage, Trivy, CodeQL, MegaLinter workflows
- [x] Aggressive Renovate automerge config (mirrors wyrm/jync)
- [x] Issue templates + `config.yml`
- [x] DESIGN.md — architecture + Zed API constraints
- [x] First commit + push (seed main)
- [x] MVP MCP server: `list_pull_requests` + `list_accounts` via `gh` (tested)
- [x] Repo automerge infra: auto-merge + branch protection (required `build` check)
- [x] Private vulnerability reporting enabled on the repo
- [x] Responsible-disclosure security page (SECURITY.md + posture), jync-style
- [x] Green CI: commit `zed-ghpr` Cargo.lock (wasm `--locked`); drop flaky/redundant
      grype; enable Discussions so the issue-template link resolves
- [x] Add `extension (wasm)` to required checks (gate = build + wasm)
- [x] Harden CI checkouts (`persist-credentials: false`) — real hardening; kept
- [x] Disable bundled zizmor (v1.25.0 crashes its artipacked audit on ci.yml)
- [ ] Re-enable ACTION_ZIZMOR when MegaLinter bundles a fixed zizmor
- [ ] Make MegaLinter a required check once it's reliably green across a few runs
- [ ] Wire the MCP binary auto-download into the extension (GitHub releases)

## Phase 1 — GitHub auth + accounts
- [x] MVP auth by reusing `gh` CLI stored credentials (multi-account already there)
- [x] `list_accounts` tool — enumerate `gh auth status` accounts
- [x] Account switching per request — `account` param resolves a per-call `GH_TOKEN`
      via `gh auth token --user`, no global state change
- [ ] Native OAuth device flow as an alternative to `gh` (Zed MCP OAuth/DCR)

## Phase 2 — PR views + filters
- [x] `list_pull_requests` tool — repo/org scoped
- [x] Filters: author, review-requested (via `search`), assignee, state, label, limit
- [x] Named views — `view` preset (needs-my-review, mine, assigned-to-me,
      involves-me) expands to a search qualifier, merges with `search`
- [x] `pr_detail` tool — body, reviews, checks (statusCheckRollup), files

## Phase 3 — Notifications
- [ ] Poll PR state; detect updates (new review, CI status change, new commit)
- [ ] Surface updates through the Agent Panel / OS notification from the server
- [x] Config foundation — `NotifyConfig` (poll interval, cooldown, repos,
      suppress-self, exclude-bots, silence list, event toggles) + serde load +
      `notification_settings` tool (defaults merged with the user's file)
- [ ] Notification filter/detection logic on top of the config (poller feeds it)

Event types to notify on:
- [ ] PR approved and ready to merge (approving review + mergeable/clean checks)
- [ ] New comment on a PR (review comment or issue comment)

Noise controls (esp. for comments):
- [ ] Comment cooldown/debounce — coalesce a burst of comments from one actor
      into a single notification within a window (configurable)
- [ ] Never notify on the user's own actions — suppress events the active
      account authored
- [ ] Exclude bot / GitHub App comments (opt-in; match `author.type == "Bot"` /
      app slugs), configurable
- [ ] Per-user silence list — mute comment notifications from specific logins

## Phase 4 — PR review shortcut
- [ ] "Review this PR with Claude" action — an MCP tool / slash command
      (`review_pr <n>`) that gathers the PR diff + changed files and starts a
      review in Zed's Agent Panel (the button-equivalent; extensions can't add
      real PR-UI buttons, see DESIGN.md).
- [ ] Config field: user-defined review prompt template for that shortcut
      (with placeholders like {repo}, {number}, {title}, {diff}).

## Phase 5 — Polish
- [x] README: install, setup, and usage (tools, views, accounts, config)
- [ ] Slash commands (`/prs`, `/pr`) for quick access in the assistant
- [ ] Docs site / screenshots
- [ ] Publish to Zed extension registry

## Phase 6 — Direct interaction (no AI)
Zed extensions can't draw UI, so direct interaction lives in the `ghpr-mcp` binary
itself — same core logic, non-AI front doors.
- [ ] CLI mode — subcommands on `ghpr-mcp` (e.g. `ghpr-mcp prs --view needs-my-review`,
      `ghpr-mcp pr 42`, `ghpr-mcp accounts`) reusing the existing gh/notify code
- [ ] TUI dashboard (ratatui), inspired by githappens — sortable PR table with:
      color-coded merge-readiness (green/yellow/red), check pass/total (e.g. 5/7),
      approval state, branch-up-to-date, PR age, +/- diff stats
- [ ] TUI keybindings: j/k + arrows nav, g/G edges, Enter=open in browser,
      r=refresh, R=force refetch, ?=help, q/Esc=quit
- [ ] TUI auto-refresh (configurable, min 30s) + rate-limit awareness
      (ref: https://github.com/steffen-karlsson/githappens)
