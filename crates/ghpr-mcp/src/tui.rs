//! Terminal dashboard (ratatui) for open PRs — the non-AI, keyboard-driven front
//! door. Data/format helpers are pure and tested; the event loop is thin.

use std::io::IsTerminal;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use serde_json::Value;

use crate::{cli, gh, notify};

/// One row of the dashboard, derived from `gh pr list` JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrRow {
    pub number: i64,
    /// `owner/name`; empty in single-repo mode where the repo is implicit.
    pub repo: String,
    pub title: String,
    pub author: String,
    pub approval: String, // APPROVED | CHANGES_REQUESTED | REVIEW_REQUIRED | ""
    pub checks: String,   // passing | failing | pending | ""
    pub checks_passed: u32,
    pub checks_total: u32,
    /// Short merge-state label: conflict | behind | draft | "".
    pub merge: String,
    pub age: String,
    /// Seconds since last update; kept for sorting (the string is display-only).
    pub age_secs: i64,
    pub additions: i64,
    pub deletions: i64,
    pub url: String,
}

const LIST_FIELDS: &str = "number,title,author,reviewDecision,statusCheckRollup,mergeable,mergeStateStatus,updatedAt,additions,deletions,url";

/// A short, actionable merge-state label from `mergeable` + `mergeStateStatus`.
pub fn merge_label(mergeable: &str, merge_state: &str) -> &'static str {
    if mergeable == "CONFLICTING" {
        "conflict"
    } else if merge_state == "BEHIND" {
        "behind"
    } else if merge_state == "DRAFT" {
        "draft"
    } else {
        "-"
    }
}

/// Color for a merge-state label.
pub fn merge_color(label: &str) -> Color {
    match label {
        "conflict" => Color::Red,
        "behind" => Color::Yellow,
        _ => Color::DarkGray,
    }
}

/// Seconds since `updated` (RFC3339); `i64::MAX` if unparseable (sorts last).
pub fn age_seconds(updated: &str, now: DateTime<Utc>) -> i64 {
    match DateTime::parse_from_rfc3339(updated) {
        Ok(then) => (now - then.with_timezone(&Utc)).num_seconds().max(0),
        Err(_) => i64::MAX,
    }
}

/// Relative age like `2d`, `5h`, `3m` from an RFC3339 timestamp.
pub fn fmt_age(updated: &str, now: DateTime<Utc>) -> String {
    let secs = age_seconds(updated, now);
    if secs == i64::MAX {
        String::new()
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

/// How the dashboard is sorted; cycled with the `s` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Number,
    Age,
    Checks,
    Churn,
}

impl SortKey {
    fn next(self) -> Self {
        match self {
            SortKey::Number => SortKey::Age,
            SortKey::Age => SortKey::Checks,
            SortKey::Checks => SortKey::Churn,
            SortKey::Churn => SortKey::Number,
        }
    }

    fn label(self) -> &'static str {
        match self {
            SortKey::Number => "number",
            SortKey::Age => "age",
            SortKey::Checks => "checks",
            SortKey::Churn => "size",
        }
    }
}

/// Rank checks so the most-actionable sort first: failing < pending < passing < none.
fn checks_rank(checks: &str) -> u8 {
    match checks {
        "failing" => 0,
        "pending" => 1,
        "passing" => 2,
        _ => 3,
    }
}

/// Sort rows in place by the given key (newest/oldest/most-actionable/largest first).
pub fn apply_sort(rows: &mut [PrRow], key: SortKey) {
    use std::cmp::Reverse;
    match key {
        SortKey::Number => rows.sort_by_key(|r| Reverse(r.number)),
        SortKey::Age => rows.sort_by_key(|r| Reverse(r.age_secs)),
        SortKey::Checks => rows.sort_by_key(|r| checks_rank(&r.checks)),
        SortKey::Churn => rows.sort_by_key(|r| Reverse(r.additions + r.deletions)),
    }
}

/// Parse `gh api rate_limit` into `(remaining, limit)` for the GraphQL API that
/// `gh pr list` uses. The rate_limit endpoint itself doesn't count against it.
pub fn parse_rate_limit(json: &str) -> Option<(u64, u64)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let g = &v["resources"]["graphql"];
    Some((g["remaining"].as_u64()?, g["limit"].as_u64()?))
}

/// Parse `gh pr list` JSON into dashboard rows.
pub fn build_rows(json: &str, now: DateTime<Utc>) -> Result<Vec<PrRow>, String> {
    let arr: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let arr = arr.as_array().ok_or("expected a JSON array of PRs")?;
    Ok(arr
        .iter()
        .map(|pr| PrRow {
            number: pr["number"].as_i64().unwrap_or(0),
            repo: String::new(),
            title: pr["title"].as_str().unwrap_or("").trim().to_string(),
            author: pr["author"]["login"].as_str().unwrap_or("?").to_string(),
            approval: pr["reviewDecision"].as_str().unwrap_or("").to_string(),
            checks: cli::checks_from_rollup(&pr["statusCheckRollup"]),
            checks_passed: cli::check_counts(&pr["statusCheckRollup"]).0,
            checks_total: cli::check_counts(&pr["statusCheckRollup"]).1,
            merge: merge_label(
                pr["mergeable"].as_str().unwrap_or(""),
                pr["mergeStateStatus"].as_str().unwrap_or(""),
            )
            .to_string(),
            age: fmt_age(pr["updatedAt"].as_str().unwrap_or(""), now),
            age_secs: age_seconds(pr["updatedAt"].as_str().unwrap_or(""), now),
            additions: pr["additions"].as_i64().unwrap_or(0),
            deletions: pr["deletions"].as_i64().unwrap_or(0),
            url: pr["url"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

/// Short repo label for the dashboard: drop the owner (usually constant) and
/// keep `name`, which reads cleaner in a narrow column.
pub fn short_repo(name_with_owner: &str) -> String {
    name_with_owner
        .rsplit_once('/')
        .map(|(_, name)| name)
        .unwrap_or(name_with_owner)
        .to_string()
}

/// Color for a checks state: green/red/yellow, else gray.
pub fn checks_color(checks: &str) -> Color {
    match checks {
        "passing" => Color::Green,
        "failing" => Color::Red,
        "pending" => Color::Yellow,
        _ => Color::DarkGray,
    }
}

/// Color for an approval state.
pub fn approval_color(approval: &str) -> Color {
    match approval {
        "APPROVED" => Color::Green,
        "CHANGES_REQUESTED" => Color::Red,
        _ => Color::DarkGray,
    }
}

/// Short label for an approval state.
pub fn approval_label(approval: &str) -> &str {
    match approval {
        "APPROVED" => "approved",
        "CHANGES_REQUESTED" => "changes",
        "REVIEW_REQUIRED" => "review",
        _ => "-",
    }
}

/// Cap on repos fanned out in account-wide mode — bounds the per-refresh `gh`
/// calls (one per repo) so a user in dozens of repos can't exhaust rate limit.
const MAX_GLOBAL_REPOS: usize = 25;

struct App {
    /// `None` means "the current repo" — gh infers it from the working directory.
    repo: Option<String>,
    account: Option<String>,
    /// Account-wide mode: list PRs across all repos via `gh search prs`.
    global: bool,
    /// Named view (e.g. `mine`) driving the account-wide search qualifier.
    view: Option<String>,
    rows: Vec<PrRow>,
    state: TableState,
    help: bool,
    status: String,
    sort: SortKey,
}

impl App {
    /// Build a `gh pr list` arg vector with the rich dashboard fields. When
    /// `repo`/`search` are set they scope the query; otherwise gh infers the
    /// repo from the working directory.
    fn list_args(repo: Option<&str>, search: Option<&str>) -> Vec<String> {
        let mut args: Vec<String> = ["pr", "list", "--state", "open", "--limit", "100", "--json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        args.push(LIST_FIELDS.to_string());
        if let Some(r) = repo {
            args.push("--repo".to_string());
            args.push(r.to_string());
        }
        if let Some(s) = search {
            args.push("--search".to_string());
            args.push(s.to_string());
        }
        args
    }

    fn fetch(repo: Option<&str>, account: Option<&str>) -> Result<Vec<PrRow>, String> {
        let json = gh::run_as(&Self::list_args(repo, None), account)?;
        build_rows(&json, Utc::now())
    }

    /// Account-wide fetch: discover matching repos via `gh search prs`, then run
    /// the rich per-repo query for each so checks/review/merge columns survive
    /// (the search API doesn't return those rollups). A failed repo is skipped.
    fn fetch_global(view: Option<&str>, account: Option<&str>) -> Result<Vec<PrRow>, String> {
        let qualifier = gh::global_qualifier(view, None);
        let search_json = gh::run_as(&gh::pr_search_args(&qualifier, "open", 100), account)?;
        let repos = gh::repos_from_search(&search_json);
        let now = Utc::now();
        let mut rows = Vec::new();
        for repo in repos.iter().take(MAX_GLOBAL_REPOS) {
            let args = Self::list_args(Some(repo), Some(&qualifier));
            let Ok(json) = gh::run_as(&args, account) else {
                continue;
            };
            if let Ok(mut rs) = build_rows(&json, now) {
                for r in &mut rs {
                    r.repo = repo.clone();
                }
                rows.append(&mut rs);
            }
        }
        Ok(rows)
    }

    fn repo_label(&self) -> String {
        if self.global {
            format!(
                "all repos · {}",
                gh::global_qualifier(self.view.as_deref(), None)
            )
        } else {
            self.repo.as_deref().unwrap_or("current repo").to_string()
        }
    }

    /// Best-effort GraphQL rate-limit `(remaining, limit)`; `None` on any failure.
    fn fetch_rate(account: Option<&str>) -> Option<(u64, u64)> {
        let out = gh::run_as(&["api".to_string(), "rate_limit".to_string()], account).ok()?;
        parse_rate_limit(&out)
    }

    fn refresh(&mut self) {
        let fetched = if self.global {
            Self::fetch_global(self.view.as_deref(), self.account.as_deref())
        } else {
            Self::fetch(self.repo.as_deref(), self.account.as_deref())
        };
        match fetched {
            Ok(mut rows) => {
                apply_sort(&mut rows, self.sort);
                self.rows = rows;
                let rate = Self::fetch_rate(self.account.as_deref())
                    .map(|(r, l)| format!(" · API {r}/{l}"))
                    .unwrap_or_default();
                self.status = format!(
                    "{} open PRs · sort: {}{rate}",
                    self.rows.len(),
                    self.sort.label()
                );
                if self.state.selected().unwrap_or(0) >= self.rows.len() {
                    self.state
                        .select(if self.rows.is_empty() { None } else { Some(0) });
                }
            }
            Err(e) => self.status = format!("error: {e}"),
        }
    }

    fn step(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let cur = self.state.selected().unwrap_or(0) as isize;
        let next = (cur + delta).rem_euclid(self.rows.len() as isize);
        self.state.select(Some(next as usize));
    }

    fn jump(&mut self, last: bool) {
        if self.rows.is_empty() {
            return;
        }
        self.state
            .select(Some(if last { self.rows.len() - 1 } else { 0 }));
    }

    fn open_selected(&mut self) {
        if let Some(row) = self.state.selected().and_then(|i| self.rows.get(i)) {
            if !row.url.is_empty() {
                let _ = open_in_browser(&row.url);
            }
        }
    }

    fn cycle_sort(&mut self) {
        self.sort = self.sort.next();
        apply_sort(&mut self.rows, self.sort);
        self.status = format!("{} open PRs · sort: {}", self.rows.len(), self.sort.label());
        if !self.rows.is_empty() {
            self.state.select(Some(0));
        }
    }

    /// Flip between the current repo and account-wide (all repos) and refetch.
    fn toggle_global(&mut self) {
        self.global = !self.global;
        self.status = "loading…".to_string();
        self.refresh();
        self.state
            .select(if self.rows.is_empty() { None } else { Some(0) });
    }
}

/// Open a URL in the default browser (best-effort, per platform).
fn open_in_browser(url: &str) -> std::io::Result<()> {
    let (prog, arg): (&str, &[&str]) = if cfg!(target_os = "macos") {
        ("open", &[])
    } else if cfg!(target_os = "windows") {
        ("cmd", &["/C", "start"])
    } else {
        ("xdg-open", &[])
    };
    std::process::Command::new(prog)
        .args(arg)
        .arg(url)
        .status()
        .map(|_| ())
}

/// Launch the interactive dashboard. Returns when the user quits.
pub fn run(
    repo: Option<&str>,
    account: Option<&str>,
    global: bool,
    view: Option<&str>,
) -> Result<String, String> {
    if !std::io::stdout().is_terminal() {
        return Err("the dashboard needs an interactive terminal".to_string());
    }
    let mut app = App {
        repo: repo.map(String::from),
        account: account.map(String::from),
        global,
        view: view.map(String::from),
        rows: Vec::new(),
        state: TableState::default(),
        help: false,
        status: "loading…".to_string(),
        sort: SortKey::Number,
    };
    app.refresh();
    if !app.rows.is_empty() {
        app.state.select(Some(0));
    }

    // Auto-refresh interval from config (min 30s), used by the event loop.
    let refresh = Duration::from_secs(
        notify::NotifyConfig::load(&notify::config_path(None))
            .unwrap_or_default()
            .tui_refresh_secs
            .max(30),
    );

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app, refresh);
    ratatui::restore();
    result.map(|_| String::new()).map_err(|e| e.to_string())
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    refresh: Duration,
) -> std::io::Result<()> {
    let mut last_refresh = Instant::now();
    loop {
        terminal.draw(|f| render(f, app))?;
        if last_refresh.elapsed() >= refresh {
            app.refresh();
            last_refresh = Instant::now();
        }
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('j') | KeyCode::Down => app.step(1),
                KeyCode::Char('k') | KeyCode::Up => app.step(-1),
                KeyCode::Char('g') => app.jump(false),
                KeyCode::Char('G') => app.jump(true),
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    app.refresh();
                    last_refresh = Instant::now();
                }
                KeyCode::Char('s') => app.cycle_sort(),
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    app.toggle_global();
                    last_refresh = Instant::now();
                }
                KeyCode::Char('?') => app.help = !app.help,
                KeyCode::Enter => app.open_selected(),
                _ => {}
            }
        }
    }
}

fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(f.area());

    let title = Line::from(format!(
        " GitHub PRs — {} ({}) ",
        app.repo_label(),
        app.status
    ))
    .bold();
    f.render_widget(Paragraph::new(title), chunks[0]);

    // Account-wide mode adds a repo column, since a bare number isn't unique
    // across repos; single-repo mode omits it.
    let global = app.global;
    let mut header_cells = vec!["#"];
    if global {
        header_cells.push("repo");
    }
    header_cells.extend(["checks", "review", "state", "age", "±", "title", "author"]);
    let header = Row::new(header_cells).style(Style::default().add_modifier(Modifier::BOLD));
    let rows = app.rows.iter().map(|r| {
        let checks_cell = if r.checks_total > 0 {
            format!("{}/{}", r.checks_passed, r.checks_total)
        } else {
            "–".to_string()
        };
        let mut cells = vec![Cell::from(r.number.to_string())];
        if global {
            cells.push(Cell::from(short_repo(&r.repo)));
        }
        cells.extend([
            Cell::from(checks_cell).style(Style::default().fg(checks_color(&r.checks))),
            Cell::from(approval_label(&r.approval))
                .style(Style::default().fg(approval_color(&r.approval))),
            Cell::from(r.merge.clone()).style(Style::default().fg(merge_color(&r.merge))),
            Cell::from(r.age.clone()),
            Cell::from(format!("+{} -{}", r.additions, r.deletions)),
            Cell::from(r.title.clone()),
            Cell::from(r.author.clone()),
        ]);
        Row::new(cells)
    });
    let mut widths = vec![Constraint::Length(6)];
    if global {
        widths.push(Constraint::Length(20));
    }
    widths.extend([
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(5),
        Constraint::Length(12),
        Constraint::Min(20),
        Constraint::Length(16),
    ]);
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_stateful_widget(table, chunks[1], &mut app.state);

    let footer = if app.help {
        " j/k move · g/G top/bottom · Enter open · s sort · a all-repos · r refresh · ? help · q quit "
    } else {
        " j/k move · Enter open · s sort · a all-repos · r refresh · ? help · q quit "
    };
    f.render_widget(
        Paragraph::new(Line::from(footer)).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-09-22T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn fmt_age_buckets() {
        assert_eq!(fmt_age("2026-09-22T11:30:00Z", now()), "30m");
        assert_eq!(fmt_age("2026-09-22T07:00:00Z", now()), "5h");
        assert_eq!(fmt_age("2026-09-20T12:00:00Z", now()), "2d");
        assert_eq!(fmt_age("not-a-date", now()), "");
    }

    #[test]
    fn build_rows_maps_fields() {
        let json = r#"[
            {"number": 7, "title": " Fix ", "author": {"login": "alice"},
             "reviewDecision": "APPROVED",
             "statusCheckRollup": [{"status":"COMPLETED","conclusion":"SUCCESS"}],
             "updatedAt": "2026-09-22T11:00:00Z", "additions": 10, "deletions": 2,
             "url": "https://x/7"}
        ]"#;
        let rows = build_rows(json, now()).unwrap();
        assert_eq!(rows[0].number, 7);
        assert_eq!(rows[0].title, "Fix");
        assert_eq!(rows[0].author, "alice");
        assert_eq!(rows[0].checks, "passing");
        assert_eq!(rows[0].age, "1h");
        assert_eq!(rows[0].additions, 10);
        // repo is only populated in account-wide mode, where fetch_global tags it.
        assert_eq!(rows[0].repo, "");
    }

    #[test]
    fn short_repo_drops_owner() {
        assert_eq!(short_repo("nellotech/argocd-manifests"), "argocd-manifests");
        assert_eq!(short_repo("bare"), "bare");
    }

    #[test]
    fn colors_and_labels() {
        assert_eq!(checks_color("failing"), Color::Red);
        assert_eq!(approval_color("APPROVED"), Color::Green);
        assert_eq!(approval_label("CHANGES_REQUESTED"), "changes");
        assert_eq!(approval_label(""), "-");
    }

    fn row(number: i64, age_secs: i64, checks: &str, churn: i64) -> PrRow {
        PrRow {
            number,
            repo: String::new(),
            title: String::new(),
            author: String::new(),
            approval: String::new(),
            checks: checks.to_string(),
            checks_passed: 0,
            checks_total: 0,
            merge: String::new(),
            age: String::new(),
            age_secs,
            additions: churn,
            deletions: 0,
            url: String::new(),
        }
    }

    #[test]
    fn sort_by_each_key() {
        let base = || {
            vec![
                row(1, 100, "passing", 5),
                row(3, 300, "failing", 1),
                row(2, 200, "pending", 9),
            ]
        };

        let mut r = base();
        apply_sort(&mut r, SortKey::Number);
        assert_eq!(r.iter().map(|x| x.number).collect::<Vec<_>>(), [3, 2, 1]);

        r = base();
        apply_sort(&mut r, SortKey::Age); // oldest (largest age_secs) first
        assert_eq!(r.iter().map(|x| x.number).collect::<Vec<_>>(), [3, 2, 1]);

        r = base();
        apply_sort(&mut r, SortKey::Checks); // failing < pending < passing
        assert_eq!(
            r.iter().map(|x| x.checks.as_str()).collect::<Vec<_>>(),
            ["failing", "pending", "passing"]
        );

        r = base();
        apply_sort(&mut r, SortKey::Churn); // largest first
        assert_eq!(r.iter().map(|x| x.number).collect::<Vec<_>>(), [2, 1, 3]);
    }

    #[test]
    fn sort_key_cycles() {
        assert_eq!(SortKey::Number.next(), SortKey::Age);
        assert_eq!(SortKey::Churn.next(), SortKey::Number);
    }

    #[test]
    fn rate_limit_parses_graphql() {
        let json = r#"{"resources": {"graphql": {"remaining": 4980, "limit": 5000}}}"#;
        assert_eq!(parse_rate_limit(json), Some((4980, 5000)));
        assert_eq!(parse_rate_limit("{}"), None);
        assert_eq!(parse_rate_limit("nope"), None);
    }

    #[test]
    fn merge_label_prioritises_conflict() {
        assert_eq!(merge_label("CONFLICTING", "BEHIND"), "conflict");
        assert_eq!(merge_label("MERGEABLE", "BEHIND"), "behind");
        assert_eq!(merge_label("MERGEABLE", "DRAFT"), "draft");
        assert_eq!(merge_label("MERGEABLE", "CLEAN"), "-");
        assert_eq!(merge_color("conflict"), Color::Red);
        assert_eq!(merge_color("behind"), Color::Yellow);
    }
}
