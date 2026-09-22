//! Direct CLI mode — the non-AI front door. Reuses the same gh-backed logic the
//! MCP tools use, so `ghpr-mcp prs`/`pr`/`accounts` work without Zed or an agent.

use clap::Subcommand;
use serde_json::{json, Value};

use crate::gh;

#[derive(Subcommand)]
pub enum Cmd {
    /// List pull requests (defaults to open PRs in the current repo).
    Prs {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long, default_value = "open")]
        state: String,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        search: Option<String>,
        /// Named preset: needs-my-review | mine | assigned-to-me | involves-me
        #[arg(long)]
        view: Option<String>,
        #[arg(long, default_value_t = 30)]
        limit: i64,
        /// gh account login to scope this request to.
        #[arg(long)]
        account: Option<String>,
    },
    /// Show one pull request in detail (raw JSON).
    Pr {
        number: i64,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        account: Option<String>,
    },
    /// List the GitHub accounts gh is authenticated as.
    Accounts,
}

/// Run a CLI subcommand, returning the text to print or an error.
pub fn run(cmd: Cmd) -> Result<String, String> {
    match cmd {
        Cmd::Prs {
            repo,
            state,
            author,
            assignee,
            label,
            search,
            view,
            limit,
            account,
        } => {
            let mut params = json!({ "state": state, "limit": limit });
            for (k, v) in [
                ("repo", repo),
                ("author", author),
                ("assignee", assignee),
                ("label", label),
                ("search", search),
                ("view", view),
            ] {
                if let Some(v) = v {
                    params[k] = Value::String(v);
                }
            }
            let out = gh::run_as(&gh::pr_list_args(&params), account.as_deref())?;
            format_pr_list(&out)
        }
        Cmd::Pr {
            number,
            repo,
            account,
        } => {
            let mut params = json!({ "number": number });
            if let Some(r) = repo {
                params["repo"] = Value::String(r);
            }
            let args = gh::pr_view_args(&params)?;
            gh::run_as(&args, account.as_deref())
        }
        Cmd::Accounts => gh::run(&["auth".into(), "status".into()]),
    }
}

/// Render the JSON from `gh pr list` as a compact one-line-per-PR table.
pub fn format_pr_list(json: &str) -> Result<String, String> {
    let prs: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let arr = prs.as_array().ok_or("expected a JSON array of PRs")?;
    if arr.is_empty() {
        return Ok("No pull requests.".to_string());
    }
    let mut out = String::new();
    for pr in arr {
        let num = pr["number"].as_i64().unwrap_or(0);
        let title = pr["title"].as_str().unwrap_or("").trim();
        let author = pr["author"]["login"].as_str().unwrap_or("?");
        let tag = if pr["isDraft"].as_bool().unwrap_or(false) {
            "DRAFT"
        } else {
            pr["state"].as_str().unwrap_or("")
        };
        out.push_str(&format!("#{num:<5} {tag:<8} {title}  (@{author})\n"));
    }
    Ok(out.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_a_pr_list() {
        let json = r#"[
            {"number": 7, "title": "Fix login", "author": {"login": "alice"}, "state": "OPEN", "isDraft": false},
            {"number": 9, "title": "WIP cache", "author": {"login": "bob"}, "state": "OPEN", "isDraft": true}
        ]"#;
        let out = format_pr_list(json).unwrap();
        assert!(out.contains("#7"));
        assert!(out.contains("Fix login"));
        assert!(out.contains("(@alice)"));
        assert!(out.contains("DRAFT")); // draft overrides state
    }

    #[test]
    fn empty_list_is_friendly() {
        assert_eq!(format_pr_list("[]").unwrap(), "No pull requests.");
    }

    #[test]
    fn bad_json_errors() {
        assert!(format_pr_list("not json").is_err());
    }
}
