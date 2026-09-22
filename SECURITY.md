# Security Policy

## Reporting a vulnerability

Please report security issues privately via GitHub Security Advisories
("Report a vulnerability" on the repo's Security tab) rather than a public issue.

## Scope notes

- This plugin never stores GitHub tokens itself. Auth is delegated to the `gh`
  CLI (or Zed's MCP OAuth flow), which own credential storage.
- The `ghpr-mcp` server runs locally and only talks to GitHub's API on your behalf.
