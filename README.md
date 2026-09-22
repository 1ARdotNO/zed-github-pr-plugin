# zed-github-pr

**Manage GitHub pull requests from inside Zed.**

A Zed extension that ships an MCP context server for GitHub PRs: list and filter
pending PRs, switch between your GitHub accounts, and get notified when a PR you
care about changes — all surfaced in Zed's Agent Panel.

> Status: **early bootstrap.** See [`TODO.md`](TODO.md) for the roadmap and
> [`DESIGN.md`](DESIGN.md) for why it's built as an MCP server.

## Why an MCP server?

Zed extensions can't draw custom panels. They *can* register a **context server**
that the Agent Panel talks to. So all the PR logic lives in a small native binary
(`ghpr-mcp`) that the extension launches. See [`DESIGN.md`](DESIGN.md).

## Features (planned)

- **PR views & filters** — pending PRs by author, review-requested, checks state,
  labels; named views like "needs my review".
- **Multiple accounts** — reuses `gh` CLI credentials; switch between accounts.
- **Notifications** — get pinged when a tracked PR gets a review, a new commit, or
  a CI status change, with per-repo/event filters.

## Requirements

- [Zed](https://zed.dev)
- [`gh`](https://cli.github.com) authenticated (`gh auth login`) — used for auth
  and multi-account support in the MVP.

## Development

```sh
cargo build --workspace          # build the extension + MCP server
cargo test --workspace           # tests
```

Then install the extension as a dev extension in Zed (Extensions → Install Dev
Extension → this folder).

## License

MIT — see [`LICENSE`](LICENSE).
