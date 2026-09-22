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
- [ ] Named views (e.g. "needs my review", "mine", "failing checks")
- [x] `pr_detail` tool — body, reviews, checks (statusCheckRollup), files

## Phase 3 — Notifications
- [ ] Poll PR state; detect updates (new review, CI status change, new commit)
- [ ] Surface updates through the Agent Panel / OS notification from the server
- [ ] Notification filter config (which repos/events/authors)
- [ ] User settings schema for filters + polling interval

## Phase 4 — Polish
- [ ] Slash commands (`/prs`, `/pr`) for quick access in the assistant
- [ ] Docs site / screenshots
- [ ] Publish to Zed extension registry
