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
    ] {
        if let Some(v) = params.get(key).and_then(Value::as_str) {
            args.push(flag.into());
            args.push(v.into());
        }
    }

    // A named `view` expands to a search qualifier and merges with any explicit
    // `search`, so callers can combine a preset with extra filters.
    let view = params
        .get("view")
        .and_then(Value::as_str)
        .and_then(view_qualifier);
    let explicit = params.get("search").and_then(Value::as_str);
    let search = [view, explicit]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    if !search.is_empty() {
        args.push("--search".into());
        args.push(search);
    }

    let limit = params.get("limit").and_then(Value::as_i64).unwrap_or(30);
    args.push("--limit".into());
    args.push(limit.to_string());

    args.push("--json".into());
    args.push(PR_LIST_FIELDS.into());
    args
}

/// Map a named view to a GitHub search qualifier. Unknown views yield `None`,
/// so they're simply ignored rather than erroring.
pub fn view_qualifier(view: &str) -> Option<&'static str> {
    match view {
        "needs-my-review" => Some("review-requested:@me"),
        "mine" => Some("author:@me"),
        "assigned-to-me" => Some("assignee:@me"),
        "involves-me" => Some("involves:@me"),
        _ => None,
    }
}

/// Combine a named view and/or explicit search into one account-wide GitHub
/// search qualifier. Falls back to `involves:@me` so "global, no filter" still
/// means "PRs that concern me" rather than every PR on GitHub.
pub fn global_qualifier(view: Option<&str>, search: Option<&str>) -> String {
    let parts = [
        view.and_then(view_qualifier).map(str::to_string),
        search.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if parts.is_empty() {
        "involves:@me".to_string()
    } else {
        parts.join(" ")
    }
}

/// Build `gh search prs` args for an account-wide (all repos) search. The
/// qualifier is a GitHub search fragment (e.g. `author:@me`).
pub fn pr_search_args(qualifier: &str, state: &str, limit: i64) -> Vec<String> {
    vec![
        "search".into(),
        "prs".into(),
        qualifier.into(),
        "--state".into(),
        state.into(),
        "--limit".into(),
        limit.to_string(),
        "--json".into(),
        "number,title,author,isDraft,state,repository,url".into(),
    ]
}

/// Distinct `owner/name` repos from a `gh search prs` result, in first-seen
/// order. Used to fan a rich per-repo `gh pr list` out across all matches.
pub fn repos_from_search(json: &str) -> Vec<String> {
    let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    for pr in &arr {
        if let Some(name) = pr["repository"]["nameWithOwner"].as_str() {
            if !out.iter().any(|r| r == name) {
                out.push(name.to_string());
            }
        }
    }
    out
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

/// Build the argument vector for `gh pr diff <number>`.
pub fn pr_diff_args(number: i64, repo: Option<&str>) -> Vec<String> {
    let mut args = vec!["pr".into(), "diff".into(), number.to_string()];
    if let Some(r) = repo {
        args.push("--repo".into());
        args.push(r.into());
    }
    args
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
    fn view_preset_becomes_a_search_qualifier() {
        let args = pr_list_args(&json!({ "view": "needs-my-review" }));
        assert!(args
            .windows(2)
            .any(|w| w == ["--search", "review-requested:@me"]));
    }

    #[test]
    fn view_and_explicit_search_merge() {
        let args = pr_list_args(&json!({ "view": "mine", "search": "is:draft" }));
        assert!(args
            .windows(2)
            .any(|w| w == ["--search", "author:@me is:draft"]));
    }

    #[test]
    fn unknown_view_is_ignored() {
        let args = pr_list_args(&json!({ "view": "bogus" }));
        assert!(!args.iter().any(|a| a == "--search"));
    }

    #[test]
    fn pr_view_needs_a_number() {
        assert!(pr_view_args(&json!({})).is_err());
    }

    #[test]
    fn global_qualifier_defaults_to_involving_me() {
        assert_eq!(global_qualifier(None, None), "involves:@me");
        assert_eq!(global_qualifier(Some("bogus"), None), "involves:@me");
    }

    #[test]
    fn global_qualifier_merges_view_and_search() {
        assert_eq!(global_qualifier(Some("mine"), None), "author:@me");
        assert_eq!(
            global_qualifier(Some("mine"), Some("is:draft")),
            "author:@me is:draft"
        );
        assert_eq!(global_qualifier(None, Some("label:bug")), "label:bug");
    }

    #[test]
    fn pr_search_args_targets_search_prs() {
        let args = pr_search_args("author:@me", "open", 50);
        assert_eq!(args[0..3], ["search", "prs", "author:@me"]);
        assert!(args.windows(2).any(|w| w == ["--state", "open"]));
        assert!(args.windows(2).any(|w| w == ["--limit", "50"]));
    }

    #[test]
    fn repos_from_search_is_unique_and_ordered() {
        let json = r#"[
            {"repository": {"nameWithOwner": "o/a"}},
            {"repository": {"nameWithOwner": "o/b"}},
            {"repository": {"nameWithOwner": "o/a"}}
        ]"#;
        assert_eq!(repos_from_search(json), vec!["o/a", "o/b"]);
        assert!(repos_from_search("not json").is_empty());
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
