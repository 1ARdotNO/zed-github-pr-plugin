//! ghpr-mcp — a GitHub-PR helper with two front doors:
//!
//! - **no subcommand**: an MCP server over stdio (newline-delimited JSON-RPC),
//!   which is how the Zed extension launches it.
//! - **a subcommand** (`prs`, `pr`, `accounts`): direct CLI output, no AI.
//!
//! Auth/accounts are delegated to the `gh` CLI for the MVP; see DESIGN.md.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use clap::Parser;

mod cli;
mod gh;
mod notify;
mod rpc;
mod tools;

const PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Parser)]
#[command(
    name = "ghpr-mcp",
    about = "GitHub PRs from Zed (MCP) or the terminal (CLI)."
)]
struct Args {
    #[command(subcommand)]
    cmd: Option<cli::Cmd>,
}

fn main() -> ExitCode {
    match Args::parse().cmd {
        None => {
            run_mcp();
            ExitCode::SUCCESS
        }
        Some(cmd) => match cli::run(cmd) {
            Ok(text) => {
                println!("{text}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

/// The MCP stdio loop: one JSON-RPC message per line in, responses out.
fn run_mcp() {
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
