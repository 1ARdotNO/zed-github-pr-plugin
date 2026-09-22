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

    for (key, flag) in [
        ("author", "--author"),
        ("assignee", "--assignee"),
        ("label", "--label"),
        ("search", "--search"),
    ] {
        if let Some(v) = params.get(key).and_then(Value::as_str) {
            args.push(flag.into());
            args.push(v.into());
        }
    }
    let limit = params.get("limit").and_then(Value::as_i64).unwrap_or(30);
    args.push("--limit".into());
    args.push(limit.to_string());

    args.push("--json".into());
    args.push(PR_LIST_FIELDS.into());
    args
}

/// Build the argument vector for `gh pr view <number>`. Returns an error string
/// when the required `number` is missing.
pub fn pr_view_args(params: &Value) -> Result<Vec<String>, String> {
    let number = params
        .get("number")
        .and_then(Value::as_i64)
        .ok_or("missing required integer param: number")?;

    let mut args = vec!["pr".into(), "view".into(), number.to_string()];
    if let Some(repo) = params.get("repo").and_then(Value::as_str) {
        args.push("--repo".into());
        args.push(repo.into());
    }
    args.push("--json".into());
    args.push(PR_DETAIL_FIELDS.into());
    Ok(args)
}

/// `gh auth token --user <login>` — prints that account's token without changing
/// the globally-active account, so we can scope a request per account.
pub fn token_args(account: &str) -> Vec<String> {
    vec![
        "auth".into(),
        "token".into(),
        "--user".into(),
        account.into(),
    ]
}

const PR_LIST_FIELDS: &str =
    "number,title,author,url,state,isDraft,createdAt,updatedAt,reviewDecision,labels,headRefName";

const PR_DETAIL_FIELDS: &str = "number,title,body,author,state,isDraft,url,reviewDecision,mergeStateStatus,statusCheckRollup,reviews,files,additions,deletions,labels,headRefName,baseRefName,createdAt,updatedAt";

/// Run `gh` with the given args, returning stdout on success or an error string.
pub fn run(args: &[String]) -> Result<String, String> {
    run_env(args, &[])
}

/// Run `gh` with extra environment variables (used to scope a request to a
/// specific account via `GH_TOKEN`).
pub fn run_env(args: &[String], env: &[(String, String)]) -> Result<String, String> {
    let mut cmd = Command::new("gh");
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("failed to run gh (is it installed and on PATH?): {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Run `gh` for a request, optionally scoped to `account`. When an account is
/// given we resolve its token and pass it via `GH_TOKEN` for this call only.
pub fn run_as(args: &[String], account: Option<&str>) -> Result<String, String> {
    match account {
        None => run(args),
        Some(acct) => {
            let token = run(&token_args(acct))?.trim().to_string();
            run_env(args, &[("GH_TOKEN".to_string(), token)])
        }
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

    #[test]
    fn pr_view_needs_a_number() {
        assert!(pr_view_args(&json!({})).is_err());
    }

    #[test]
    fn pr_view_builds_positional_and_repo() {
        let args = pr_view_args(&json!({ "number": 7, "repo": "o/n" })).unwrap();
        assert_eq!(args[0..3], ["pr", "view", "7"]);
        assert!(args.windows(2).any(|w| w == ["--repo", "o/n"]));
        assert!(args.iter().any(|a| a == "--json"));
    }

    #[test]
    fn token_args_target_the_account() {
        assert_eq!(
            token_args("octocat"),
            ["auth", "token", "--user", "octocat"]
        );
    }
}
