//! Direct CLI mode — the non-AI front door. Reuses the same gh-backed logic the
//! MCP tools use, so `ghpr-mcp prs`/`pr`/`accounts` work without Zed or an agent.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clap::Subcommand;
use serde_json::{json, Value};

use crate::{gh, notify};

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
    /// Watch a repo's open PRs and print notification-worthy changes each poll.
    Watch {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        account: Option<String>,
        /// Run a single poll cycle and exit (default: loop).
        #[arg(long)]
        once: bool,
        /// State file for previous snapshots (default: ~/.cache/ghpr-mcp/...).
        #[arg(long)]
        state_file: Option<String>,
    },
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
        Cmd::Watch {
            repo,
            account,
            once,
            state_file,
        } => run_watch(&repo, account.as_deref(), once, state_file),
    }
}

/// The notification poller: each cycle diffs current open PRs against the last
/// snapshot and prints the events that pass the user's filters.
fn run_watch(
    repo: &str,
    account: Option<&str>,
    once: bool,
    state_file: Option<String>,
) -> Result<String, String> {
    let cfg = notify::NotifyConfig::load(&notify::config_path(None))?;
    let state_path = state_file.unwrap_or_else(|| watch_state_path(repo));
    let mut cooldown = notify::Cooldown::new(cfg.comment_cooldown_secs);

    loop {
        let cycle = watch_cycle(repo, account, &state_path, &cfg, &mut cooldown)?;
        if !cycle.is_empty() {
            println!("{cycle}");
        }
        if once {
            return Ok(String::new());
        }
        std::thread::sleep(Duration::from_secs(cfg.poll_interval_secs.max(30)));
    }
}

fn watch_cycle(
    repo: &str,
    account: Option<&str>,
    state_path: &str,
    cfg: &notify::NotifyConfig,
    cooldown: &mut notify::Cooldown,
) -> Result<String, String> {
    let args: Vec<String> = [
        "pr",
        "list",
        "--repo",
        repo,
        "--state",
        "open",
        "--limit",
        "100",
        "--json",
        "number,reviewDecision,statusCheckRollup",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let json = gh::run_as(&args, account)?;
    let current = snapshots_from_list(&json)?;

    let previous = load_state(state_path);
    let now = now_secs();
    let mut lines = String::new();
    let mut next: HashMap<u64, notify::PrSnapshot> = HashMap::new();

    for snap in &current {
        next.insert(snap.number, snap.clone());
        // Skip PRs we've never seen — don't notify on the baseline snapshot.
        let Some(prev) = previous.get(&snap.number) else {
            continue;
        };
        for ev in notify::detect(prev, snap) {
            if !notify::should_notify(&ev, cfg) {
                continue;
            }
            // Debounce comment bursts from the same actor.
            if ev.kind == notify::EventKind::NewComment {
                let key = format!("{}:{}", ev.pr_number, ev.actor);
                if !cooldown.allow(&key, now) {
                    continue;
                }
            }
            lines.push_str(&format_event(repo, &ev));
            lines.push('\n');
        }
    }

    save_state(state_path, &next)?;
    Ok(lines.trim_end().to_string())
}

fn format_event(repo: &str, e: &notify::Event) -> String {
    let what = match e.kind {
        notify::EventKind::ApprovedReady => "approved & ready to merge",
        notify::EventKind::NewReview => "changes requested",
        notify::EventKind::NewComment => "new comment",
        notify::EventKind::CiStatus => "checks status changed",
        notify::EventKind::NewCommit => "new commit",
    };
    let by = if e.actor.is_empty() {
        String::new()
    } else {
        format!(" by @{}", e.actor)
    };
    format!("{repo}#{} — {what}{by}", e.pr_number)
}

/// Parse `gh pr list --json number,reviewDecision,statusCheckRollup` into snapshots.
pub fn snapshots_from_list(json: &str) -> Result<Vec<notify::PrSnapshot>, String> {
    let arr: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let arr = arr.as_array().ok_or("expected a JSON array of PRs")?;
    Ok(arr
        .iter()
        .map(|pr| notify::PrSnapshot {
            number: pr["number"].as_u64().unwrap_or(0),
            review_decision: pr["reviewDecision"].as_str().unwrap_or("").to_string(),
            checks: checks_from_rollup(&pr["statusCheckRollup"]),
            ..Default::default()
        })
        .collect())
}

/// Reduce a `statusCheckRollup` array to `passing` | `failing` | `pending` | "".
pub fn checks_from_rollup(rollup: &Value) -> String {
    let Some(entries) = rollup.as_array() else {
        return String::new();
    };
    let (mut any_pass, mut any_fail, mut any_pending) = (false, false, false);
    for e in entries {
        // Check runs carry status/conclusion; status contexts carry state.
        let status = e["status"].as_str().unwrap_or("");
        let outcome = e["conclusion"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or_else(|| e["state"].as_str())
            .unwrap_or("");
        match outcome {
            "SUCCESS" | "NEUTRAL" | "SKIPPED" => any_pass = true,
            "FAILURE" | "ERROR" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED"
            | "STARTUP_FAILURE" => any_fail = true,
            _ if !status.is_empty() && status != "COMPLETED" => any_pending = true,
            "PENDING" | "EXPECTED" | "" => any_pending = true,
            _ => {}
        }
    }
    if any_fail {
        "failing".into()
    } else if any_pending {
        "pending".into()
    } else if any_pass {
        "passing".into()
    } else {
        String::new()
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn watch_state_path(repo: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let slug = repo.replace('/', "_");
    format!("{home}/.cache/ghpr-mcp/watch-{slug}.json")
}

fn load_state(path: &str) -> HashMap<u64, notify::PrSnapshot> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(path: &str, state: &HashMap<u64, notify::PrSnapshot>) -> Result<(), String> {
    if let Some(dir) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string(state).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
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

    #[test]
    fn rollup_failing_wins_over_pending_and_passing() {
        let rollup = json!([
            {"status": "COMPLETED", "conclusion": "SUCCESS"},
            {"status": "IN_PROGRESS", "conclusion": null},
            {"status": "COMPLETED", "conclusion": "FAILURE"}
        ]);
        assert_eq!(checks_from_rollup(&rollup), "failing");
    }

    #[test]
    fn rollup_all_success_is_passing_and_empty_is_blank() {
        let ok = json!([{"status": "COMPLETED", "conclusion": "SUCCESS"}]);
        assert_eq!(checks_from_rollup(&ok), "passing");
        assert_eq!(checks_from_rollup(&json!([])), "");
        // Legacy status contexts use `state`.
        assert_eq!(
            checks_from_rollup(&json!([{"state": "PENDING"}])),
            "pending"
        );
    }

    #[test]
    fn snapshots_parse_decision_and_checks() {
        let json = r#"[
            {"number": 5, "reviewDecision": "APPROVED", "statusCheckRollup": [{"status":"COMPLETED","conclusion":"SUCCESS"}]}
        ]"#;
        let snaps = snapshots_from_list(json).unwrap();
        assert_eq!(snaps[0].number, 5);
        assert_eq!(snaps[0].review_decision, "APPROVED");
        assert_eq!(snaps[0].checks, "passing");
    }
}
