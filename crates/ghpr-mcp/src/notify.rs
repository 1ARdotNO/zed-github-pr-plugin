//! Notification settings model. The poller (next step) consumes this to decide
//! what to surface; today the `notification_settings` tool loads and validates it.

use std::collections::HashMap;

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
    /// Fire native desktop notifications (in addition to stdout) from `watch`.
    pub desktop_notifications: bool,
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
            desktop_notifications: true,
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

/// A point-in-time view of a PR, diffed across polls to detect events. Serialized
/// to a small state file so a poll cycle can compare against the previous run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrSnapshot {
    pub number: u64,
    /// "APPROVED" | "CHANGES_REQUESTED" | "REVIEW_REQUIRED" | ""
    pub review_decision: String,
    /// "passing" | "failing" | "pending" | ""
    pub checks: String,
    pub head_sha: String,
    pub comment_count: u64,
    /// Login of the actor behind the most recent change, when known.
    pub last_actor: String,
    pub last_actor_is_bot: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    ApprovedReady,
    NewReview,
    NewComment,
    CiStatus,
    NewCommit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub pr_number: u64,
    pub actor: String,
    pub actor_is_bot: bool,
}

/// Diff two snapshots of the same PR into the events they imply.
pub fn detect(prev: &PrSnapshot, curr: &PrSnapshot) -> Vec<Event> {
    let mut out = Vec::new();
    let mk = |kind| Event {
        kind,
        pr_number: curr.number,
        actor: curr.last_actor.clone(),
        actor_is_bot: curr.last_actor_is_bot,
    };

    if prev.review_decision != curr.review_decision {
        match curr.review_decision.as_str() {
            // Approved *and* not blocked by checks == ready to merge.
            "APPROVED" if curr.checks != "failing" && curr.checks != "pending" => {
                out.push(mk(EventKind::ApprovedReady))
            }
            "CHANGES_REQUESTED" => out.push(mk(EventKind::NewReview)),
            _ => {}
        }
    }
    if !prev.head_sha.is_empty() && prev.head_sha != curr.head_sha {
        out.push(mk(EventKind::NewCommit));
    }
    if !curr.checks.is_empty() && prev.checks != curr.checks {
        out.push(mk(EventKind::CiStatus));
    }
    if curr.comment_count > prev.comment_count {
        out.push(mk(EventKind::NewComment));
    }
    out
}

/// Apply the config's noise controls to decide whether an event notifies.
pub fn should_notify(e: &Event, c: &NotifyConfig) -> bool {
    let enabled = match e.kind {
        EventKind::ApprovedReady => c.events.approved_ready,
        EventKind::NewReview => c.events.new_review,
        EventKind::NewComment => c.events.new_comment,
        EventKind::CiStatus => c.events.ci_status,
        EventKind::NewCommit => c.events.new_commit,
    };
    if !enabled {
        return false;
    }
    if c.suppress_self && !e.actor.is_empty() && c.self_login.as_deref() == Some(e.actor.as_str()) {
        return false;
    }
    if c.exclude_bots && e.actor_is_bot {
        return false;
    }
    if c.silenced_users.iter().any(|u| u == &e.actor) {
        return false;
    }
    true
}

/// Debounce: suppresses repeats of the same key within a time window. Used to
/// coalesce a burst of comments from one actor into a single notification.
#[derive(Default)]
pub struct Cooldown {
    last: HashMap<String, u64>,
    window_secs: u64,
}

impl Cooldown {
    pub fn new(window_secs: u64) -> Self {
        Self {
            last: HashMap::new(),
            window_secs,
        }
    }

    /// True if `key` is allowed at time `now` (first time, or outside the window);
    /// records the time when allowed.
    pub fn allow(&mut self, key: &str, now: u64) -> bool {
        match self.last.get(key) {
            Some(&t) if now.saturating_sub(t) < self.window_secs => false,
            _ => {
                self.last.insert(key.to_string(), now);
                true
            }
        }
    }
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

    fn snap(number: u64) -> PrSnapshot {
        PrSnapshot {
            number,
            ..Default::default()
        }
    }

    #[test]
    fn detects_approved_ready_only_when_checks_ok() {
        let prev = snap(1);
        let mut curr = snap(1);
        curr.review_decision = "APPROVED".into();
        curr.checks = "passing".into();
        assert_eq!(detect(&prev, &curr)[0].kind, EventKind::ApprovedReady);

        // Failing checks means approved but not ready — no ApprovedReady event.
        curr.checks = "failing".into();
        assert!(!detect(&prev, &curr)
            .iter()
            .any(|e| e.kind == EventKind::ApprovedReady));
    }

    #[test]
    fn detects_new_commit_and_ci_and_comment() {
        let mut prev = snap(2);
        prev.head_sha = "aaa".into();
        prev.checks = "pending".into();
        let mut curr = prev.clone();
        curr.head_sha = "bbb".into();
        curr.checks = "passing".into();
        curr.comment_count = 1;
        let kinds: Vec<_> = detect(&prev, &curr).into_iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&EventKind::NewCommit));
        assert!(kinds.contains(&EventKind::CiStatus));
        assert!(kinds.contains(&EventKind::NewComment));
    }

    #[test]
    fn no_change_yields_no_events() {
        let s = snap(3);
        assert!(detect(&s, &s).is_empty());
    }

    fn comment_event(actor: &str, is_bot: bool) -> Event {
        Event {
            kind: EventKind::NewComment,
            pr_number: 1,
            actor: actor.into(),
            actor_is_bot: is_bot,
        }
    }

    #[test]
    fn should_notify_applies_noise_controls() {
        let mut cfg = NotifyConfig {
            self_login: Some("me".into()),
            ..Default::default()
        };

        assert!(should_notify(&comment_event("someone", false), &cfg));
        assert!(!should_notify(&comment_event("me", false), &cfg)); // suppress self
        assert!(!should_notify(&comment_event("dependabot", true), &cfg)); // exclude bots

        cfg.silenced_users = vec!["noisy".into()];
        assert!(!should_notify(&comment_event("noisy", false), &cfg)); // silenced

        cfg.events.new_comment = false;
        assert!(!should_notify(&comment_event("someone", false), &cfg)); // kind disabled
    }

    #[test]
    fn cooldown_suppresses_within_window() {
        let mut cd = Cooldown::new(300);
        assert!(cd.allow("pr1:bob", 0)); // first is allowed
        assert!(!cd.allow("pr1:bob", 100)); // within 300s → suppressed
        assert!(cd.allow("pr1:bob", 400)); // window elapsed → allowed again
        assert!(cd.allow("pr1:alice", 100)); // different key unaffected
    }
}
