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
- [ ] Repo automerge infra: enable auto-merge + branch protection (required checks)
- [ ] Wire the MCP binary auto-download into the extension (GitHub releases)

## Phase 1 — GitHub auth + accounts
- [ ] MVP auth by reusing `gh` CLI stored credentials (multi-account already there)
- [ ] `list_accounts` tool — enumerate `gh auth status` accounts
- [ ] Account switching per request (choose between different git accounts)
- [ ] Native OAuth device flow as an alternative to `gh` (Zed MCP OAuth/DCR)

## Phase 2 — PR views + filters
- [ ] `list_pull_requests` tool — repo/org scoped
- [ ] Filters: author, review-requested, assignee, state, label, draft, checks
- [ ] Named views (e.g. "needs my review", "mine", "failing checks")
- [ ] `pr_detail` tool — description, checks, reviews, files

## Phase 3 — Notifications
- [ ] Poll PR state; detect updates (new review, CI status change, new commit)
- [ ] Surface updates through the Agent Panel / OS notification from the server
- [ ] Notification filter config (which repos/events/authors)
- [ ] User settings schema for filters + polling interval

## Phase 4 — Polish
- [ ] Slash commands (`/prs`, `/pr`) for quick access in the assistant
- [ ] Docs site / screenshots
- [ ] Publish to Zed extension registry
