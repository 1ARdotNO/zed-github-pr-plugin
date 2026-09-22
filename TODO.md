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
- [x] osv-scanner: ignore low/unmaintained transitive advisories from ratatui
      (lru, paste) via `osv-scanner.toml`; Trivy still gates HIGH/CRITICAL
- [ ] Drop the osv ignores once ratatui ships fixed lru/paste (revisit 2027-03)
- [ ] Make MegaLinter a required check once it's reliably green across a few runs
- [x] Release workflow — tag `vX.Y.Z` builds `ghpr-mcp` for linux/macOS (x64+arm64)
      and publishes to GitHub Releases (taiki-e actions, mirrors wyrm)
- [x] Extension-side: download the release binary per-platform (linux/macOS via
      `latest_github_release` + `download_file`), falling back to `ghpr-mcp` on PATH

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
- [x] Poll PR state + detect updates — `watch` command diffs snapshots against a
      state file each cycle (review decision, checks status, commits, comments)
- [x] Config foundation — `NotifyConfig` (poll interval, cooldown, repos,
      suppress-self, exclude-bots, silence list, event toggles) + serde load +
      `notification_settings` tool (defaults merged with the user's file)
- [x] Notification filter/detection logic — `detect` + `should_notify` + `Cooldown`
- [x] Deliver via OS notification — `watch` fires native desktop notifications
      (macOS osascript / Linux notify-send), gated by `desktop_notifications`
- [x] Comment/commit attribution — `gh pr list` returns `comments` + `headRefOid`
      in one call; snapshots carry comment count, head SHA, last commenter (+bot)

Event types to notify on:
- [x] PR approved and ready to merge (review APPROVED + checks not failing/pending)
- [x] New comment on a PR — fires with the commenter attributed (only comment
      events carry an actor, so filters apply correctly)

Noise controls (esp. for comments):
- [x] Comment cooldown/debounce — `Cooldown` coalesces same actor+PR within a window
- [x] Never notify on the user's own actions — `suppress_self` + `self_login`
- [x] Exclude bot / GitHub App comments — `exclude_bots` on `actor_is_bot`
- [x] Per-user silence list — `silenced_users`

## Phase 4 — PR review shortcut
- [x] "Review this PR" action — `review_pr` MCP tool + `review <n>` CLI: gathers
      title/body/diff and renders the review prompt for the Agent Panel (or piping)
- [x] Config field — `review.prompt_template` with {repo}/{number}/{title}/{body}/{diff}

## Phase 5 — Polish
- [x] README: install, setup, and usage (tools, views, accounts, config)
- [ ] Slash commands (`/prs`, `/pr`) for quick access in the assistant
- [x] GitHub Pages docs site — `docs/index.html` (install, tools, CLI/TUI,
      notification config) + `pages.yml` deploy; Pages enabled (Actions source)
- [ ] Docs polish: screenshots/GIFs of the TUI + Agent Panel
- [ ] Publish to Zed extension registry

## Phase 6 — Direct interaction (no AI)
Zed extensions can't draw UI, so direct interaction lives in the `ghpr-mcp` binary
itself — same core logic, non-AI front doors.
- [x] CLI mode — dual-mode binary: no args → MCP stdio; `prs`/`pr`/`accounts`
      subcommands → direct terminal output, reusing the gh-backed logic
- [x] TUI dashboard (ratatui) `ghpr-mcp tui` — PR table with color-coded checks,
      approval state, age, +/- diff stats (data layer unit-tested)
- [x] TUI keybindings: j/k + arrows nav, g/G edges, Enter=open in browser,
      r=refresh, ?=help, q/Esc=quit
- [x] TUI: check pass/total counts (e.g. 5/7) in the checks column
- [x] TUI auto-refresh — `tui_refresh_secs` config (clamped ≥30s), resets on `r`
- [x] TUI sortable columns — `s` cycles number/age/checks/size
- [x] TUI merge-state column — conflict / behind / draft (from mergeable +
      mergeStateStatus), color-coded
- [ ] TUI polish: rate-limit awareness
      (ref: https://github.com/steffen-karlsson/githappens)
- [x] Easy in-Zed access to the TUI — `.zed/tasks.json` "GitHub PRs" task runs
      `ghpr-mcp tui` (repo inferred from the worktree); README documents a keybind
      to open it in Zed's integrated terminal
