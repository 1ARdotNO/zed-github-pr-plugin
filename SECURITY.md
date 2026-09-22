# Security Policy

## Reporting a vulnerability

Please report security issues **privately** — do not open a public issue for
anything exploitable.

- Preferred: use GitHub's **[Report a vulnerability](https://github.com/1ARdotNO/zed-github-pr-plugin/security/advisories/new)**
  (Security → Advisories) — private vulnerability reporting is enabled on this repo.
- We aim to acknowledge reports within a few days and to coordinate a fix and
  disclosure timeline with you.

Please include: affected version/commit, a description, reproduction steps, and
impact. Proof-of-concept code is welcome, but never test against systems, repos,
or data you don't own.

## Supported versions

This plugin is pre-1.0; only the latest `main` is supported. Fixes land on `main`
and in the next tagged release.

## Scope & handling notes

- **Credentials.** The plugin never stores GitHub tokens itself. Authentication is
  delegated to the [`gh`](https://cli.github.com) CLI (or Zed's MCP OAuth flow),
  which own credential storage. Treat a compromised `gh` config or OS keychain as a
  credential exposure — outside this project's control.
- **Local execution.** `ghpr-mcp` runs locally as a child process of Zed and only
  talks to GitHub's API on your behalf. It shells out to `gh`; it does not open
  network listeners.
- **Data movement.** The server reads PR metadata for the repos/accounts you query
  and returns it to Zed's Agent Panel. It does not send your data anywhere else.
- **Automated hardening.** Dependencies and GitHub Actions are monitored by
  Dependabot alerts + Renovate; every change is gated by CI (fmt, clippy, tests,
  coverage, CodeQL, Trivy, MegaLinter) before merge.

## What is not a vulnerability

- Findings that require an already-compromised host, `gh` config, or OS keychain.
- Issues in the `gh` CLI or GitHub's API themselves — report those upstream.
- Rate-limiting or quota behaviour inherited from the GitHub API.
