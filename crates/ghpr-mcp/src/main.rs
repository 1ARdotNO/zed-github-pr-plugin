//! ghpr-mcp — a minimal MCP (Model Context Protocol) server for GitHub PRs.
//!
//! Transport: newline-delimited JSON-RPC 2.0 over stdio (the MCP stdio transport).
//! Auth/accounts are delegated to the `gh` CLI for the MVP; see DESIGN.md.

use std::io::{self, BufRead, Write};

mod gh;
mod rpc;
mod tools;

const PROTOCOL_VERSION: &str = "2024-11-05";

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            Ok(_) => continue,
            Err(_) => break,
        };
        if let Some(response) = rpc::handle(&line) {
            let _ = writeln!(out, "{response}");
            let _ = out.flush();
        }
    }
}
