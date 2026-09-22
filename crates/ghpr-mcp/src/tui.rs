//! Terminal dashboard (ratatui) for open PRs — the non-AI, keyboard-driven front
//! door. Data/format helpers are pure and tested; the event loop is thin.

use std::io::IsTerminal;
use std::time::Duration;

use chrono::{DateTime, Utc};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use serde_json::Value;

use crate::{cli, gh};

/// One row of the dashboard, derived from `gh pr list` JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrRow {
    pub number: i64,
    pub title: String,
    pub author: String,
    pub approval: String, // APPROVED | CHANGES_REQUESTED | REVIEW_REQUIRED | ""
    pub checks: String,   // passing | failing | pending | ""
    pub age: String,
    pub additions: i64,
    pub deletions: i64,
    pub url: String,
}

const LIST_FIELDS: &str =
    "number,title,author,reviewDecision,statusCheckRollup,updatedAt,additions,deletions,url";

/// Relative age like `2d`, `5h`, `3m` from an RFC3339 timestamp.
pub fn fmt_age(updated: &str, now: DateTime<Utc>) -> String {
    let Ok(then) = DateTime::parse_from_rfc3339(updated) else {
        return String::new();
    };
    let secs = (now - then.with_timezone(&Utc)).num_seconds().max(0);
    if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

/// Parse `gh pr list` JSON into dashboard rows.
pub fn build_rows(json: &str, now: DateTime<Utc>) -> Result<Vec<PrRow>, String> {
    let arr: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let arr = arr.as_array().ok_or("expected a JSON array of PRs")?;
    Ok(arr
        .iter()
        .map(|pr| PrRow {
            number: pr["number"].as_i64().unwrap_or(0),
            title: pr["title"].as_str().unwrap_or("").trim().to_string(),
            author: pr["author"]["login"].as_str().unwrap_or("?").to_string(),
            approval: pr["reviewDecision"].as_str().unwrap_or("").to_string(),
            checks: cli::checks_from_rollup(&pr["statusCheckRollup"]),
            age: fmt_age(pr["updatedAt"].as_str().unwrap_or(""), now),
            additions: pr["additions"].as_i64().unwrap_or(0),
            deletions: pr["deletions"].as_i64().unwrap_or(0),
            url: pr["url"].as_str().unwrap_or("").to_string(),
        })
        .collect())
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

struct App {
    repo: String,
    account: Option<String>,
    rows: Vec<PrRow>,
    state: TableState,
    help: bool,
    status: String,
}

impl App {
    fn fetch(repo: &str, account: Option<&str>) -> Result<Vec<PrRow>, String> {
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
            LIST_FIELDS,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let json = gh::run_as(&args, account)?;
        build_rows(&json, Utc::now())
    }

    fn refresh(&mut self) {
        match Self::fetch(&self.repo, self.account.as_deref()) {
            Ok(rows) => {
                self.rows = rows;
                self.status = format!("{} open PRs", self.rows.len());
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
pub fn run(repo: &str, account: Option<&str>) -> Result<String, String> {
    if !std::io::stdout().is_terminal() {
        return Err("the dashboard needs an interactive terminal".to_string());
    }
    let mut app = App {
        repo: repo.to_string(),
        account: account.map(String::from),
        rows: Vec::new(),
        state: TableState::default(),
        help: false,
        status: "loading…".to_string(),
    };
    app.refresh();
    if !app.rows.is_empty() {
        app.state.select(Some(0));
    }

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    result.map(|_| String::new()).map_err(|e| e.to_string())
}

fn event_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    loop {
        terminal.draw(|f| render(f, app))?;
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
                KeyCode::Char('r') | KeyCode::Char('R') => app.refresh(),
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

    let title = Line::from(format!(" GitHub PRs — {} ({}) ", app.repo, app.status)).bold();
    f.render_widget(Paragraph::new(title), chunks[0]);

    let header = Row::new(["#", "checks", "review", "age", "±", "title", "author"])
        .style(Style::default().add_modifier(Modifier::BOLD));
    let rows = app.rows.iter().map(|r| {
        Row::new(vec![
            Cell::from(r.number.to_string()),
            Cell::from("●").style(Style::default().fg(checks_color(&r.checks))),
            Cell::from(approval_label(&r.approval))
                .style(Style::default().fg(approval_color(&r.approval))),
            Cell::from(r.age.clone()),
            Cell::from(format!("+{} -{}", r.additions, r.deletions)),
            Cell::from(r.title.clone()),
            Cell::from(r.author.clone()),
        ])
    });
    let widths = [
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(5),
        Constraint::Length(12),
        Constraint::Min(20),
        Constraint::Length(16),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_stateful_widget(table, chunks[1], &mut app.state);

    let footer = if app.help {
        " j/k move · g/G top/bottom · Enter open · r refresh · ? help · q quit "
    } else {
        " j/k move · Enter open · r refresh · ? help · q quit "
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
    }

    #[test]
    fn colors_and_labels() {
        assert_eq!(checks_color("failing"), Color::Red);
        assert_eq!(approval_color("APPROVED"), Color::Green);
        assert_eq!(approval_label("CHANGES_REQUESTED"), "changes");
        assert_eq!(approval_label(""), "-");
    }
}
