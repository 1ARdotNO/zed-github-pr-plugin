//! Notification settings model. The poller (next step) consumes this to decide
//! what to surface; today the `notification_settings` tool loads and validates it.

use serde::{Deserialize, Serialize};

/// Which PR event kinds produce a notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EventToggles {
    pub approved_ready: bool,
    pub new_comment: bool,
    pub new_review: bool,
    pub ci_status: bool,
    pub new_commit: bool,
}

impl Default for EventToggles {
    fn default() -> Self {
        Self {
            approved_ready: true,
            new_comment: true,
            new_review: true,
            ci_status: true,
            new_commit: true,
        }
    }
}

/// User notification configuration. Unknown fields are ignored and any omitted
/// field falls back to its default, so partial configs are valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotifyConfig {
    pub poll_interval_secs: u64,
    /// Coalesce a burst of comments from one actor within this window.
    pub comment_cooldown_secs: u64,
    pub repos: Vec<String>,
    /// The active account's login; events it authored are suppressed when
    /// `suppress_self` is set. Resolved from `gh` at runtime when omitted.
    pub self_login: Option<String>,
    pub suppress_self: bool,
    pub exclude_bots: bool,
    pub silenced_users: Vec<String>,
    pub events: EventToggles,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 120,
            comment_cooldown_secs: 300,
            repos: Vec::new(),
            self_login: None,
            suppress_self: true,
            exclude_bots: true,
            silenced_users: Vec::new(),
            events: EventToggles::default(),
        }
    }
}

impl NotifyConfig {
    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("invalid notification config: {e}"))
    }

    /// Load from `path`; a missing file yields defaults (not an error).
    pub fn load(path: &str) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(s) => Self::from_json(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("cannot read {path}: {e}")),
        }
    }
}

/// Resolve the config path: explicit override, else `$GHPR_CONFIG`, else
/// `$HOME/.config/ghpr-mcp/config.json`.
pub fn config_path(override_path: Option<&str>) -> String {
    if let Some(p) = override_path {
        return p.to_string();
    }
    if let Ok(p) = std::env::var("GHPR_CONFIG") {
        return p;
    }
    let home = std::env::var("HOME").unwrap_or_default();
    format!("{home}/.config/ghpr-mcp/config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = NotifyConfig::default();
        assert_eq!(c.poll_interval_secs, 120);
        assert!(c.suppress_self && c.exclude_bots);
        assert!(c.events.approved_ready && c.events.new_comment);
        assert!(c.silenced_users.is_empty());
    }

    #[test]
    fn partial_json_overrides_only_named_fields() {
        let c = NotifyConfig::from_json(
            r#"{"suppress_self": false, "silenced_users": ["dependabot"]}"#,
        )
        .unwrap();
        assert!(!c.suppress_self); // overridden
        assert!(c.exclude_bots); // still the default
        assert_eq!(c.silenced_users, vec!["dependabot".to_string()]);
        assert_eq!(c.poll_interval_secs, 120); // still the default
    }

    #[test]
    fn unknown_fields_are_ignored() {
        assert!(NotifyConfig::from_json(r#"{"totally_unknown": 42}"#).is_ok());
    }

    #[test]
    fn nested_event_toggle_overrides() {
        let c = NotifyConfig::from_json(r#"{"events": {"new_commit": false}}"#).unwrap();
        assert!(!c.events.new_commit);
        assert!(c.events.approved_ready); // sibling stays default
    }

    #[test]
    fn config_path_prefers_override() {
        assert_eq!(config_path(Some("/tmp/x.json")), "/tmp/x.json");
    }
}
