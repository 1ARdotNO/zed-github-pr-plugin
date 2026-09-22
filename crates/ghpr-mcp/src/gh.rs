//! Thin wrapper over the `gh` CLI. Auth and multi-account are gh's job.

use std::process::Command;

use serde_json::Value;

/// Build the argument vector for `gh pr list` from tool params. Pure, so it's
/// unit-testable without invoking `gh`.
pub fn pr_list_args(params: &Value) -> Vec<String> {
    let mut args = vec!["pr".into(), "list".into()];

    if let Some(repo) = params.get("repo").and_then(Value::as_str) {
        args.push("--repo".into());
        args.push(repo.into());
    }
    // Default to open PRs; accept open|closed|merged|all.
    let state = params
        .get("state")
        .and_then(Value::as_str)
        .unwrap_or("open");
    args.push("--state".into());
    args.push(state.into());

    if let Some(author) = params.get("author").and_then(Value::as_str) {
        args.push("--author".into());
        args.push(author.into());
    }
    if let Some(assignee) = params.get("assignee").and_then(Value::as_str) {
        args.push("--assignee".into());
        args.push(assignee.into());
    }
    if let Some(label) = params.get("label").and_then(Value::as_str) {
        args.push("--label".into());
        args.push(label.into());
    }
    if let Some(search) = params.get("search").and_then(Value::as_str) {
        args.push("--search".into());
        args.push(search.into());
    }
    let limit = params.get("limit").and_then(Value::as_i64).unwrap_or(30);
    args.push("--limit".into());
    args.push(limit.to_string());

    args.push("--json".into());
    args.push(PR_FIELDS.into());
    args
}

const PR_FIELDS: &str =
    "number,title,author,url,state,isDraft,createdAt,updatedAt,reviewDecision,labels,headRefName";

/// Run `gh` with the given args, returning stdout on success or an error string.
pub fn run(args: &[String]) -> Result<String, String> {
    let output = Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("failed to run gh (is it installed and on PATH?): {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_to_open_prs_with_json_fields() {
        let args = pr_list_args(&json!({}));
        assert!(args.windows(2).any(|w| w == ["--state", "open"]));
        assert!(args.iter().any(|a| a == "--json"));
        assert!(args.windows(2).any(|w| w == ["--limit", "30"]));
    }

    #[test]
    fn threads_filters_through() {
        let args = pr_list_args(&json!({
            "repo": "1ARdotNO/zed-github-pr-plugin",
            "author": "1ardotno",
            "state": "all",
            "limit": 5
        }));
        assert!(args
            .windows(2)
            .any(|w| w == ["--repo", "1ARdotNO/zed-github-pr-plugin"]));
        assert!(args.windows(2).any(|w| w == ["--author", "1ardotno"]));
        assert!(args.windows(2).any(|w| w == ["--state", "all"]));
        assert!(args.windows(2).any(|w| w == ["--limit", "5"]));
    }
}
