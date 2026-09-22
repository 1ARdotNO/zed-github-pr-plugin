# Security posture

A living summary of the plugin's security model. For how to report issues, see
[`SECURITY.md`](../SECURITY.md).

## Trust boundaries

| Boundary | Notes |
|----------|-------|
| Zed ⇄ `ghpr-mcp` | stdio (JSON-RPC) between Zed and a local child process. No network socket. |
| `ghpr-mcp` ⇄ `gh` | `ghpr-mcp` invokes the `gh` CLI; `gh` holds credentials and talks to GitHub. |
| `gh` ⇄ GitHub | HTTPS to the GitHub API, authenticated with the user's stored token. |

## Credential handling

The plugin holds **no** long-lived secrets. It never reads, logs, or persists
GitHub tokens — auth is entirely delegated to `gh` (or, later, Zed's MCP OAuth
flow). Multi-account selection uses `gh`'s existing account store.

## Input handling

Tool arguments from the Agent Panel are passed to `gh` as **separate argv
elements** (never interpolated into a shell string), so PR filters can't inject
shell commands. `ghpr-mcp` spawns `gh` directly without a shell.

## Open items

- Native OAuth device flow (alternative to `gh`) — will add a token-lifetime and
  scope review when implemented.
- Notification transport (OS-native vs. Agent-Panel) — revisit data-exposure notes
  once chosen.
