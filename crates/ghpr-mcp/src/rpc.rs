//! JSON-RPC dispatch for the MCP methods we implement.

use serde_json::{json, Value};

use crate::{tools, PROTOCOL_VERSION};

/// Handle one JSON-RPC request line. Returns the serialized response, or `None`
/// for notifications (no `id`), which get no reply.
pub fn handle(line: &str) -> Option<String> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
    };

    let id = req.get("id").cloned();
    let method = req
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    // Notifications (no id) are acknowledged silently.
    let id = id?;

    let result = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "ghpr-mcp", "version": env!("CARGO_PKG_VERSION") }
        })),
        "tools/list" => Ok(json!({ "tools": tools::list() })),
        "tools/call" => tools::call(req.get("params")),
        _ => return Some(error(id, -32601, &format!("method not found: {method}"))),
    };

    Some(match result {
        Ok(value) => success(id, value),
        Err(msg) => error(id, -32000, &msg),
    })
}

fn success(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn error(id: Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_reports_protocol_and_server() {
        let out = handle(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(v["result"]["serverInfo"]["name"], "ghpr-mcp");
    }

    #[test]
    fn notifications_get_no_reply() {
        assert!(handle(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    }

    #[test]
    fn unknown_method_is_an_error() {
        let out = handle(r#"{"jsonrpc":"2.0","id":2,"method":"nope"}"#).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["error"]["code"], -32601);
    }

    #[test]
    fn tools_list_is_nonempty() {
        let out = handle(r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert!(v["result"]["tools"].as_array().unwrap().len() >= 2);
    }
}
