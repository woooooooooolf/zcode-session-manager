//! Compatibility check: does the ZCode database layout match what we know?
//! A failed check blocks all destructive operations.

use crate::paths::Paths;
use crate::util::{columns, existing_tables, open_ro};
use rusqlite::Connection;
use std::collections::HashSet;

/// db.sqlite tables deleted by session_id. Optional at runtime — missing tables
/// are skipped during deletion — but a *present* table must have `session_id`.
pub const CHILD_TABLES: &[&str] = &[
    "message",
    "part",
    "session_entry",
    "session_input",
    "session_target",
    "todo",
    "tool_usage",
    "turn_usage",
    "model_usage",
    "input_history",
];

const SESSION_REQUIRED_COLS: &[&str] = &[
    "id",
    "parent_id",
    "directory",
    "title",
    "task_type",
    "time_created",
    "time_updated",
];

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatReport {
    pub db_present: bool,
    pub tasks_present: bool,
    /// true when `problems` is empty; destructive ops require this.
    pub ok: bool,
    pub problems: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn check(paths: &Paths) -> CompatReport {
    let mut problems = Vec::new();
    let mut warnings = Vec::new();

    // ---------- db.sqlite ----------
    let db_present = paths.db_path.is_file();
    if !db_present {
        problems.push("db.sqlite: cli/db/db.sqlite not found".into());
    } else {
        match open_ro(&paths.db_path) {
            Err(e) => problems.push(format!("db.sqlite: cannot open ({e})")),
            Ok(con) => check_content_db(&con, &mut problems),
        }
    }

    // ---------- tasks-index.sqlite ----------
    let tasks_present = paths.tasks_db.is_file();
    if !tasks_present {
        warnings
            .push("tasks-index: v2/tasks-index.sqlite not found — archive flags unavailable".into());
    } else {
        match open_ro(&paths.tasks_db) {
            Err(e) => problems.push(format!("tasks-index: cannot open ({e})")),
            Ok(con) => check_index_db(&con, &mut problems),
        }
    }

    CompatReport {
        db_present,
        tasks_present,
        ok: problems.is_empty(),
        problems,
        warnings,
    }
}

fn check_content_db(con: &Connection, problems: &mut Vec<String>) {
    let tables = existing_tables(con);
    for req in ["session", "message", "part"] {
        if !tables.contains(req) {
            problems.push(format!("db.sqlite: missing required table `{req}`"));
        }
    }
    if tables.contains("session") {
        let cols = columns(con, "session");
        for req in SESSION_REQUIRED_COLS {
            if !cols.contains(*req) {
                problems.push(format!("db.sqlite: `session` is missing column `{req}`"));
            }
        }
    }
    for t in ["message", "part"] {
        if tables.contains(t) && !columns(con, t).contains("session_id") {
            problems.push(format!("db.sqlite: `{t}` is missing column `session_id`"));
        }
    }
    for t in CHILD_TABLES {
        if tables.contains(*t) && !columns(con, t).contains("session_id") {
            problems.push(format!("db.sqlite: `{t}` is missing column `session_id`"));
        }
    }
    // dwf_* workflow journal (ZCode 0.16.5+): linked to sessions via
    // dwf_run.parent_session_id and dwf_actor.session_id, children via run_id.
    for (t, col) in [
        ("dwf_run", "parent_session_id"),
        ("dwf_actor", "session_id"),
        ("dwf_node", "run_id"),
        ("dwf_event", "run_id"),
    ] {
        if tables.contains(t) && !columns(con, t).contains(col) {
            problems.push(format!("db.sqlite: `{t}` is missing column `{col}`"));
        }
    }
}

fn check_index_db(con: &Connection, problems: &mut Vec<String>) {
    let tables = existing_tables(con);
    if !tables.contains("tasks") {
        problems.push("tasks-index: missing table `tasks`".into());
        return;
    }
    let cols = columns(con, "tasks");
    for req in ["task_id", "archived"] {
        if !cols.contains(req) {
            problems.push(format!("tasks-index: `tasks` is missing column `{req}`"));
        }
    }
}

/// All tables we might touch that actually exist in the content db.
pub fn available_child_tables(con: &Connection) -> HashSet<String> {
    let tables = existing_tables(con);
    CHILD_TABLES
        .iter()
        .filter(|t| tables.contains(**t))
        .map(|t| t.to_string())
        .collect()
}
