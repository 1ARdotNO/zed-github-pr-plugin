# Design

## The constraint that shapes everything

Zed extensions compile to a WebAssembly component and can only provide a fixed set
of capabilities: **languages, language servers, debuggers, themes, icon themes,
snippets, and MCP (context) servers**. There is **no public API** for custom UI
panels, background timers, or OS notifications from an extension.

So a "GitHub PR manager" cannot be a bespoke Zed panel today. The viable, idiomatic
path is a **context server (MCP)** that the extension registers, surfaced in Zed's
**Agent Panel** — the same mechanism the official GitHub MCP extension uses.

## Architecture

```
┌─────────────────────────────┐
│ Zed                         │
│  Agent Panel  ◀── tools ────┼──┐
└─────────────────────────────┘  │ stdio (JSON-RPC / MCP)
        ▲ registers              │
        │                        ▼
┌───────┴────────┐      ┌─────────────────────┐
│ zed-ghpr (WASM)│──────▶ ghpr-mcp (native)   │
│ extension.toml │ spawn│ GitHub PR logic      │
└────────────────┘      └──────────┬──────────┘
                                   │
                          gh CLI / GitHub API
```

- **`crates/zed-ghpr`** — the WASM extension. Its only job is to tell Zed how to
  launch the MCP server (`context_server_command`). Later it will download the
  right `ghpr-mcp` binary from GitHub releases per-platform.
- **`crates/ghpr-mcp`** — the native MCP server. All GitHub logic lives here:
  auth, account selection, PR listing/filtering, polling for updates. Being a
  native binary (not WASM) it can shell out, poll, and — for notifications — talk
  to the OS directly.

## Auth (why `gh` first)

`gh` already stores OAuth credentials for **multiple accounts** and refreshes them.
Reusing it gives us working auth and multi-account switching on day one, with no
token UI to build. Native OAuth device flow (via Zed's MCP OAuth/DCR support) is a
later alternative for users without `gh`.

## Notifications

Zed exposes **no notification API** to extensions or MCP/context servers — the
`zed_extension_api` 0.7 WIT has no toast/message/alert function, and Zed doesn't
render MCP server→client notifications (logging/progress/`list_changed`) as UI
toasts. The Agent Panel is pull-based, not a push surface. So there's no way for the
extension to raise a Zed-native alert.

The native `ghpr-mcp watch` process is the only surface that can *push* one: it
polls PR state and fires **OS notifications** (macOS `osascript`, Linux
`notify-send`), filtered by the user's config. If Zed ever adds a notification API,
switch to it (see TODO).

## Open decisions
- Binary distribution: GitHub releases + extension-side download vs. `cargo install`.
- Notification transport: OS-native vs. Agent-Panel-only.
