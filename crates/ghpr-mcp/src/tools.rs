//! MCP tool definitions and dispatch.

use serde_json::{json, Value};

use crate::{gh, notify};

/// A reusable schema fragment: the optional account selector.
fn account_prop() -> Value {
    json!({ "type": "string", "description": "gh account login to scope this request to (see list_accounts)" })
}

/// Tool schemas advertised via `tools/list`.
pub fn list() -> Vec<Value> {
    vec![
        json!({
            "name": "list_pull_requests",
            "description": "List GitHub pull requests, filtered. Defaults to open PRs in the current repo.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo": { "type": "string", "description": "owner/name; omit to use the current repo" },
                    "state": { "type": "string", "enum": ["open", "closed", "merged", "all"], "default": "open" },
                    "author": { "type": "string", "description": "filter by author login (or @me)" },
                    "assignee": { "type": "string", "description": "filter by assignee login (or @me)" },
                    "label": { "type": "string" },
                    "search": { "type": "string", "description": "gh search query, e.g. 'review-requested:@me'" },
                    "view": { "type": "string", "enum": ["needs-my-review", "mine", "assigned-to-me", "involves-me"], "description": "named preset; merges with search" },
                    "limit": { "type": "integer", "default": 30 },
                    "account": account_prop()
                }
            }
        }),
        json!({
            "name": "pr_detail",
            "description": "Show one pull request in detail: body, reviews, checks, changed files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "number": { "type": "integer", "description": "the PR number" },
                    "repo": { "type": "string", "description": "owner/name; omit to use the current repo" },
                    "account": account_prop()
                },
                "required": ["number"]
            }
        }),
        json!({
            "name": "list_accounts",
            "description": "List the GitHub accounts the gh CLI is authenticated as (for choosing between git accounts).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "notification_settings",
            "description": "Show the effective PR-notification config (defaults merged with the user's config file).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "config file path; defaults to $GHPR_CONFIG or ~/.config/ghpr-mcp/config.json" }
                }
            }
        }),
    ]
}

/// Dispatch a `tools/call` request.
pub fn call(params: Option<&Value>) -> Result<Value, String> {
    let params = params.ok_or("missing params")?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("missing tool name")?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let account = args.get("account").and_then(Value::as_str);

    let text = match name {
        "list_pull_requests" => gh::run_as(&gh::pr_list_args(&args), account),
        "pr_detail" => match gh::pr_view_args(&args) {
            Ok(a) => gh::run_as(&a, account),
            Err(e) => Err(e),
        },
        "list_accounts" => gh::run(&["auth".into(), "status".into()]),
        "notification_settings" => {
            let path = args.get("path").and_then(Value::as_str);
            notify::NotifyConfig::load(&notify::config_path(path))
                .and_then(|c| serde_json::to_string_pretty(&c).map_err(|e| e.to_string()))
        }
        other => return Err(format!("unknown tool: {other}")),
    };

    Ok(match text {
        Ok(out) => json!({ "content": [{ "type": "text", "text": out }] }),
        Err(err) => json!({ "content": [{ "type": "text", "text": err }], "isError": true }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_expected_tools() {
        let names: Vec<_> = list()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect();
        assert!(names.contains(&"list_pull_requests".to_string()));
        assert!(names.contains(&"pr_detail".to_string()));
        assert!(names.contains(&"list_accounts".to_string()));
    }

    #[test]
    fn unknown_tool_errors() {
        let params = json!({ "name": "does_not_exist" });
        assert!(call(Some(&params)).is_err());
    }

    #[test]
    fn pr_detail_without_number_is_a_tool_error_result() {
        // Missing `number` surfaces as an isError result, not a dispatch error.
        let params = json!({ "name": "pr_detail", "arguments": {} });
        let out = call(Some(&params)).unwrap();
        assert_eq!(out["isError"], true);
    }
}
