//! MCP tool definitions and dispatch.

use serde_json::{json, Value};

use crate::gh;

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
                    "limit": { "type": "integer", "default": 30 }
                }
            }
        }),
        json!({
            "name": "list_accounts",
            "description": "List the GitHub accounts the gh CLI is authenticated as (for choosing between git accounts).",
            "inputSchema": { "type": "object", "properties": {} }
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

    let text = match name {
        "list_pull_requests" => gh::run(&gh::pr_list_args(&args)),
        "list_accounts" => gh::run(&["auth".into(), "status".into()]),
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
        assert!(names.contains(&"list_accounts".to_string()));
    }

    #[test]
    fn unknown_tool_errors() {
        let params = json!({ "name": "does_not_exist" });
        assert!(call(Some(&params)).is_err());
    }
}
