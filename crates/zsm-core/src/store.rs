//! Session store: listing, detail view, cascade planning and deletion.

use crate::backup;
use crate::compat::{self, CHILD_TABLES};
use crate::disk;
use crate::error::{Error, Result};
use crate::integrity::{self, DbStatus};
use crate::paths::Paths;
use crate::util::{existing_tables, open_ro, open_rw, qm};
use rusqlite::{params_from_iter, TransactionBehavior};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Message sources that are synthetic runtime input, not real conversation.
const SYNTHETIC_SOURCES: &[&str] = &[
    "todo_reminder",
    "compaction",
    "queued_input",
    "background_notification",
    "system",
    "tool_result_auto",
    "context_snapshot",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub task_type: String,
    pub project: String,
    pub parent_id: Option<String>,
    pub created_ms: i64,
    pub updated_ms: i64,
    pub message_count: Option<i64>,
    pub child_count: i64,
    pub archived: bool,
    pub pinned: bool,
    /// Present in the tasks index but with no content row left in db.sqlite.
    pub ghost: bool,
    pub disk_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Msg {
    pub id: String,
    pub role: Option<String>,
    pub visible: bool,
    pub source: String,
    pub parts: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub id: String,
    pub title: String,
    pub directory: String,
    pub task_type: String,
    pub created_ms: i64,
    pub updated_ms: i64,
    pub messages: Vec<Msg>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowCount {
    pub table: String,
    pub rows: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdMeta {
    pub id: String,
    pub title: String,
    pub ghost: bool,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePlan {
    pub roots: Vec<String>,
    pub all_ids: Vec<String>,
    pub metas: Vec<IdMeta>,
    pub counts: Vec<RowCount>,
    pub disk_bytes: u64,
    pub backups_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskOutcome {
    pub files: u64,
    pub bytes: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbCheck {
    pub db: String,
    pub status: DbStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityOutcome {
    pub passed: bool,
    /// true when a failed check triggered an automatic restore from backup
    pub restored: bool,
    pub details: Vec<DbCheck>,
    pub restore_log: Vec<String>,
    pub restore_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResult {
    pub backup_dir: String,
    pub roots: Vec<String>,
    pub all_ids: Vec<String>,
    pub deleted: Vec<RowCount>,
    pub disk: DiskOutcome,
    pub index: Vec<RowCount>,
    pub warnings: Vec<String>,
    pub integrity: IntegrityOutcome,
}

pub struct Store {
    pub paths: Paths,
}

/// How to behave when ZCode is running.
#[derive(Debug, Clone, Copy)]
pub enum RunningPolicy {
    /// Never delete while ZCode is running.
    Refuse,
    /// Limited mode: only sessions that are idle longer than `idle_minutes`
    /// and not referenced by enabled automations; root sessions must also be
    /// archived. Enforced per-plan before anything is touched.
    Limited { idle_minutes: i64 },
}

impl Store {
    pub fn new(paths: Paths) -> Self {
        Self { paths }
    }

    // ---------------- listing ----------------

    /// Epoch-ms column reader that tolerates REAL-typed timestamps — SQLite is
    /// dynamically typed and third-party data may store ms as float.
    fn col_ms(r: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<i64> {
        match r.get::<_, Option<i64>>(idx) {
            Ok(v) => Ok(v.unwrap_or(0)),
            Err(_) => r
                .get::<_, Option<f64>>(idx)
                .map(|v| v.unwrap_or(0.0) as i64),
        }
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let con = open_ro(&self.paths.db_path)?;
        let mut stmt = con.prepare(
            "SELECT s.id, s.title, s.task_type, s.directory, s.parent_id, \
                    s.time_created, s.time_updated, \
                    (SELECT COUNT(*) FROM message m WHERE m.session_id = s.id), \
                    (SELECT COUNT(*) FROM session c WHERE c.parent_id = s.id) \
             FROM session s",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(SessionSummary {
                id: r.get(0)?,
                title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                task_type: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                project: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                parent_id: r.get(4)?,
                created_ms: Self::col_ms(r, 5)?,
                updated_ms: Self::col_ms(r, 6)?,
                message_count: r.get(7).ok(),
                child_count: r.get::<_, Option<i64>>(8)?.unwrap_or(0),
                archived: false,
                pinned: false,
                ghost: false,
                disk_bytes: 0,
            })
        })?;
        let mut out: Vec<SessionSummary> = Vec::new();
        for r in rows {
            match r {
                Ok(s) => out.push(s),
                // a row that fails mapping must not vanish silently
                Err(e) => eprintln!("[zsm] list row skipped: {e}"),
            }
        }
        let flags = self.index_flags();
        let mut seen: HashSet<String> = out.iter().map(|s| s.id.clone()).collect();
        if let Some(map) = &flags {
            for (id, (archived, pinned, _updated)) in map {
                if let Some(s) = out.iter_mut().find(|s| &s.id == id) {
                    s.archived = *archived;
                    s.pinned = *pinned;
                } else {
                    seen.insert(id.clone());
                    out.push(SessionSummary {
                        id: id.clone(),
                        title: String::new(), // filled below as ghost
                        task_type: String::new(),
                        project: String::new(),
                        parent_id: None,
                        created_ms: 0,
                        updated_ms: 0,
                        message_count: None,
                        child_count: 0,
                        archived: *archived,
                        pinned: *pinned,
                        ghost: true,
                        disk_bytes: 0,
                    });
                }
            }
        }
        for s in &mut out {
            if s.ghost {
                s.title = String::new(); // frontend renders the ghost label
            }
            s.disk_bytes = disk::usage(&self.paths.cli_dir, &s.id);
        }
        let _ = seen;
        Ok(out)
    }

    /// task_id → (archived, pinned, updated_at) from the tasks index; None when unavailable.
    /// Falls back to a literal 0 for updated_at if that column is absent, so a
    /// benign drift never disables the archived/pinned flags wholesale.
    fn index_flags(&self) -> Option<HashMap<String, (bool, bool, i64)>> {
        if !self.paths.tasks_db.is_file() {
            return None;
        }
        let con = open_ro(&self.paths.tasks_db).ok()?;
        if !crate::util::has_table(&con, "tasks") {
            return None;
        }
        let has_updated = crate::util::columns(&con, "tasks").contains("updated_at");
        let sql = if has_updated {
            "SELECT task_id, archived, pinned, updated_at FROM tasks"
        } else {
            "SELECT task_id, archived, pinned, 0 FROM tasks"
        };
        let mut stmt = con.prepare(sql).ok()?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0) != 0,
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0) != 0,
                    r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                ))
            })
            .ok()?;
        Some(
            rows.filter_map(|r| r.ok().map(|(id, a, p, u)| (id, (a, p, u))))
                .collect(),
        )
    }

    pub fn session_detail(&self, id: &str) -> Result<Option<SessionDetail>> {
        let con = open_ro(&self.paths.db_path)?;
        let meta = con
            .query_row(
                "SELECT id, title, directory, task_type, time_created, time_updated \
                 FROM session WHERE id = ?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                        r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                        r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                        r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                        r.get::<_, Option<i64>>(5)?.unwrap_or(0),
                    ))
                },
            )
            .optional()?;
        let (id, title, directory, task_type, created_ms, updated_ms) = match meta {
            Some(m) => m,
            None => return Ok(None),
        };

        let mut msgs = con
            .prepare("SELECT id, data FROM message WHERE session_id = ?1 ORDER BY sequence")?;
        let msg_rows = msgs
            .query_map([id.as_str()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        let mut parts_stmt = con
            .prepare("SELECT message_id, data FROM part WHERE session_id = ?1 ORDER BY sequence")?;
        let mut parts_by_msg: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        for p in parts_stmt
            .query_map([id.as_str()], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
        {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&p.1) {
                parts_by_msg.entry(p.0).or_default().push(v);
            }
        }

        let mut messages = Vec::new();
        for (mid, data) in msg_rows {
            let d: serde_json::Value =
                serde_json::from_str(&data).unwrap_or(serde_json::Value::Null);
            let role = d.get("role").and_then(|v| v.as_str()).map(|s| s.to_string());
            let sem = d
                .pointer("/semantics/origin")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let src = d.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let visible = match role.as_deref() {
                Some("user") => {
                    sem == "real_user"
                        || (!SYNTHETIC_SOURCES.contains(&src) && sem != "agent_runtime")
                }
                _ => true,
            };
            messages.push(Msg {
                id: mid.clone(),
                role,
                visible,
                source: if src.is_empty() { sem.to_string() } else { src.to_string() },
                parts: parts_by_msg.remove(&mid).unwrap_or_default(),
            });
        }

        Ok(Some(SessionDetail {
            id,
            title,
            directory,
            task_type,
            created_ms,
            updated_ms,
            messages,
        }))
    }

    // ---------------- cascade & plan ----------------

    /// roots plus every descendant session (BFS over session.parent_id).
    pub fn cascade_ids(&self, roots: &[String]) -> Result<Vec<String>> {
        let con = open_ro(&self.paths.db_path)?;
        let mut all: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut frontier: Vec<String> = roots.to_vec();
        while !frontier.is_empty() {
            let batch: Vec<String> = frontier
                .iter()
                .filter(|i| !seen.contains(*i))
                .cloned()
                .collect();
            if batch.is_empty() {
                break;
            }
            for b in &batch {
                seen.insert(b.clone());
                all.push(b.clone());
            }
            let sql = format!(
                "SELECT id FROM session WHERE parent_id IN ({})",
                qm(batch.len())
            );
            let mut stmt = con.prepare(&sql)?;
            frontier = stmt
                .query_map(params_from_iter(batch.iter()), |r| r.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .collect();
        }
        Ok(all)
    }

    pub fn plan_delete(&self, roots: &[String]) -> Result<DeletePlan> {
        let all_ids = self.cascade_ids(roots)?;
        let con = open_ro(&self.paths.db_path)?;
        let tables = existing_tables(&con);
        let mut counts: Vec<RowCount> = Vec::new();

        for t in CHILD_TABLES {
            if tables.contains(*t) {
                let sql = format!(
                    "SELECT COUNT(*) FROM {t} WHERE session_id IN ({})",
                    qm(all_ids.len())
                );
                let n = con.query_row(&sql, params_from_iter(all_ids.iter()), |r| r.get(0))?;
                counts.push(RowCount {
                    table: t.to_string(),
                    rows: n,
                });
            }
        }
        if tables.contains("session_task_link") {
            let sql = format!(
                "SELECT COUNT(*) FROM session_task_link WHERE parent_session_id IN ({0}) \
                 OR child_session_id IN ({0})",
                qm(all_ids.len())
            );
            let n = con.query_row(&sql, params_from_iter(all_ids.iter().chain(all_ids.iter())), |r| {
                r.get(0)
            })?;
            counts.push(RowCount {
                table: "session_task_link".into(),
                rows: n,
            });
        }
        if tables.contains("workflow_run") {
            let sql = format!(
                "SELECT id FROM workflow_run WHERE parent_session_id IN ({})",
                qm(all_ids.len())
            );
            let run_ids: Vec<String> = con
                .prepare(&sql)?
                .query_map(params_from_iter(all_ids.iter()), |r| r.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            if !run_ids.is_empty() {
                counts.push(RowCount {
                    table: "workflow_run".into(),
                    rows: run_ids.len() as i64,
                });
                for t in ["workflow_event", "workflow_activity"] {
                    if tables.contains(t) {
                        let sql = format!(
                            "SELECT COUNT(*) FROM {t} WHERE run_id IN ({})",
                            qm(run_ids.len())
                        );
                        let n = con.query_row(&sql, params_from_iter(run_ids.iter()), |r| r.get(0))?;
                        counts.push(RowCount {
                            table: t.to_string(),
                            rows: n,
                        });
                    }
                }
            }
        }
        {
            let sql = format!("SELECT COUNT(*) FROM session WHERE id IN ({})", qm(all_ids.len()));
            let n = con.query_row(&sql, params_from_iter(all_ids.iter()), |r| r.get(0))?;
            counts.push(RowCount {
                table: "session".into(),
                rows: n,
            });
        }

        let known: HashSet<&str> = tables.iter().map(|s| s.as_str()).collect();
        let mut metas = Vec::new();
        if !all_ids.is_empty() {
            let sql = format!(
                "SELECT id, title FROM session WHERE id IN ({})",
                qm(all_ids.len())
            );
            let rows: HashMap<String, String> = con
                .prepare(&sql)?
                .query_map(params_from_iter(all_ids.iter()), |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();
            let flags = self.index_flags().unwrap_or_default();
            for id in &all_ids {
                let title = rows.get(id).cloned();
                let _ = known;
                metas.push(IdMeta {
                    id: id.clone(),
                    title: title.unwrap_or_default(),
                    ghost: !rows.contains_key(id),
                    archived: flags.get(id).map(|f| f.0).unwrap_or(false),
                });
            }
        }

        let disk_bytes = all_ids.iter().map(|id| disk::usage(&self.paths.cli_dir, id)).sum();

        Ok(DeletePlan {
            roots: roots.to_vec(),
            all_ids,
            metas,
            counts,
            disk_bytes,
            backups_dir: backup::backups_base(&self.paths).to_string_lossy().to_string(),
        })
    }

    // ---------------- deletion ----------------

    pub fn execute_delete(&self, roots: &[String]) -> Result<DeleteResult> {
        self.execute_delete_with_policy(roots, RunningPolicy::Refuse)
    }

    pub fn execute_delete_with_policy(
        &self,
        roots: &[String],
        policy: RunningPolicy,
    ) -> Result<DeleteResult> {
        let report = compat::check(&self.paths);
        if !report.ok {
            return Err(Error::Compat {
                problems: report.problems,
            });
        }
        if !self.paths.db_path.is_file() {
            return Err(Error::DbNotFound(
                self.paths.db_path.to_string_lossy().to_string(),
            ));
        }

        let pre = self.integrity_snapshot();
        let bad: Vec<String> = pre
            .iter()
            .filter(|c| !c.status.is_ok())
            .map(|c| format!("{}: {}", c.db, c.status.detail))
            .collect();
        if !bad.is_empty() {
            return Err(Error::Corruption { details: bad });
        }

        let plan = self.plan_delete(roots)?;
        if plan.all_ids.is_empty() {
            return Err(Error::Other("no sessions matched the selection".into()));
        }

        if crate::zcode_running() {
            match policy {
                RunningPolicy::Refuse => return Err(Error::ZcodeRunning),
                RunningPolicy::Limited { idle_minutes } => {
                    let violations =
                        self.limited_violations(&plan.all_ids, &plan.roots, idle_minutes)?;
                    if !violations.is_empty() {
                        return Err(Error::Limited { violations });
                    }
                }
            }
        }

        // 1. backup (simple copy, timestamped dir)
        let backup_dir = backup::new_backup_dir(&self.paths)?;
        backup::copy_database(&self.paths.db_path, &backup_dir)?;
        backup::copy_database(&self.paths.tasks_db, &backup_dir)?;
        let records =
            backup::copy_session_disk(&self.paths.cli_dir, &plan.all_ids, &backup_dir)?;
        backup::write_manifest(
            &backup_dir,
            &serde_json::json!({
                "createdAt": chrono::Local::now().to_rfc3339(),
                "roots": plan.roots,
                "allIds": plan.all_ids,
                "dbPath": self.paths.db_path,
                "tasksDb": self.paths.tasks_db,
                "cliDir": self.paths.cli_dir,
                "disk": records,
            }),
        )?;

        // 2. content
        let deleted = self.delete_content(&plan.all_ids)?;

        // 3. disk
        let targets: Vec<PathBuf> = plan
            .all_ids
            .iter()
            .flat_map(|id| disk::targets(&self.paths.cli_dir, id))
            .collect();
        let (files, bytes, errors) = disk::remove(&targets);
        let disk_outcome = DiskOutcome { files, bytes, errors };

        // 4. index last: a mid-run failure then leaves harmless orphans only
        let (index, mut warnings) = self.delete_index(&plan.all_ids)?;

        // 5. integrity verify + auto-restore on failure
        let mut after = self.integrity_snapshot();
        let mut restored = false;
        let mut restore_log = Vec::new();
        let mut restore_errors = Vec::new();
        if after.iter().any(|c| !c.status.is_ok()) {
            match backup::restore_from_backup(&backup_dir, &self.paths, &records) {
                Ok(log) => {
                    restored = true;
                    restore_log = log;
                }
                Err(e) => restore_errors.push(e.to_string()),
            }
            after = self.integrity_snapshot();
            if restored && after.iter().any(|c| !c.status.is_ok()) {
                restore_errors.push("restore completed but integrity still failing".into());
            }
        }

        let _ = &mut warnings;
        Ok(DeleteResult {
            backup_dir: backup_dir.to_string_lossy().to_string(),
            roots: plan.roots,
            all_ids: plan.all_ids,
            deleted,
            disk: disk_outcome,
            index,
            warnings,
            integrity: IntegrityOutcome {
                passed: after.iter().all(|c| c.status.is_ok()),
                restored,
                details: after,
                restore_log,
                restore_errors,
            },
        })
    }

    /// Eligibility when ZCode is running (limited mode):
    /// - every id in the plan must be idle longer than `idle_minutes`
    ///   (freshness = max(db session.time_updated, tasks-index updated_at))
    /// - every id must not be referenced by an enabled automation
    /// - root sessions must additionally be archived; cascade children are
    ///   exempt because ZCode does not propagate the archived flag to forks —
    ///   requiring it would make most archived trees with forks undeletable
    fn limited_violations(
        &self,
        all_ids: &[String],
        roots: &[String],
        idle_minutes: i64,
    ) -> Result<Vec<(String, &'static str)>> {
        let mut violations = Vec::new();

        let mut updated: HashMap<String, i64> = HashMap::new();
        if !all_ids.is_empty() {
            let con = open_ro(&self.paths.db_path)?;
            let sql = format!(
                "SELECT id, time_updated FROM session WHERE id IN ({})",
                qm(all_ids.len())
            );
            let mut stmt = con.prepare(&sql)?;
            let rows = stmt.query_map(params_from_iter(all_ids.iter()), |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                ))
            })?;
            for row in rows.flatten() {
                updated.insert(row.0, row.1);
            }
        }

        let idx = self.index_flags();

        let mut auto_refs: HashSet<String> = HashSet::new();
        if self.paths.tasks_db.is_file() {
            if let Ok(tcon) = open_ro(&self.paths.tasks_db) {
                if crate::util::has_table(&tcon, "automations") {
                    let cols = crate::util::columns(&tcon, "automations");
                    let sql = if cols.contains("enabled") {
                        "SELECT target_task_id FROM automations WHERE enabled = 1"
                    } else {
                        "SELECT target_task_id FROM automations"
                    };
                    if let Ok(mut stmt) = tcon.prepare(sql) {
                        if let Ok(rows) = stmt.query_map([], |r| r.get::<_, Option<String>>(0)) {
                            for id in rows.flatten().flatten() {
                                auto_refs.insert(id);
                            }
                        }
                    }
                }
            }
        }

        let cutoff = chrono::Utc::now().timestamp_millis() - idle_minutes.saturating_mul(60_000);
        let roots_set: HashSet<&String> = roots.iter().collect();
        for id in all_ids {
            let sess_updated = updated.get(id).copied();
            let (archived, idx_updated) = idx
                .as_ref()
                .and_then(|m| m.get(id))
                .map(|(a, _p, u)| (*a, Some(*u)))
                .unwrap_or((false, None));
            let effective = sess_updated.or(idx_updated).unwrap_or(0);
            if effective > cutoff {
                violations.push((id.clone(), "too_recent"));
                continue;
            }
            if auto_refs.contains(id) {
                violations.push((id.clone(), "automation_ref"));
                continue;
            }
            if roots_set.contains(id) && !archived {
                violations.push((id.clone(), "not_archived"));
            }
        }
        Ok(violations)
    }

    fn integrity_snapshot(&self) -> Vec<DbCheck> {
        vec![
            DbCheck {
                db: "db.sqlite".into(),
                status: integrity::check_db(&self.paths.db_path),
            },
            DbCheck {
                db: "tasks-index.sqlite".into(),
                status: integrity::check_db(&self.paths.tasks_db),
            },
        ]
    }

    fn delete_content(&self, ids: &[String]) -> Result<Vec<RowCount>> {
        let mut con = open_rw(&self.paths.db_path)?;
        let tables = existing_tables(&con);
        let tx = con.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut out = Vec::new();

        for t in CHILD_TABLES {
            if tables.contains(*t) {
                let sql = format!("DELETE FROM {t} WHERE session_id IN ({})", qm(ids.len()));
                let n = tx.execute(&sql, params_from_iter(ids.iter()))?;
                out.push(RowCount {
                    table: t.to_string(),
                    rows: n as i64,
                });
            }
        }
        if tables.contains("session_task_link") {
            let sql = format!(
                "DELETE FROM session_task_link WHERE parent_session_id IN ({0}) \
                 OR child_session_id IN ({0})",
                qm(ids.len())
            );
            let n = tx.execute(&sql, params_from_iter(ids.iter().chain(ids.iter())))?;
            out.push(RowCount {
                table: "session_task_link".into(),
                rows: n as i64,
            });
        }
        if tables.contains("workflow_run") {
            let sql = format!(
                "SELECT id FROM workflow_run WHERE parent_session_id IN ({})",
                qm(ids.len())
            );
            let run_ids: Vec<String> = tx
                .prepare(&sql)?
                .query_map(params_from_iter(ids.iter()), |r| r.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            if !run_ids.is_empty() {
                for t in ["workflow_event", "workflow_activity"] {
                    if tables.contains(t) {
                        let sql = format!(
                            "DELETE FROM {t} WHERE run_id IN ({})",
                            qm(run_ids.len())
                        );
                        let n = tx.execute(&sql, params_from_iter(run_ids.iter()))?;
                        out.push(RowCount {
                            table: t.to_string(),
                            rows: n as i64,
                        });
                    }
                }
                let sql = format!("DELETE FROM workflow_run WHERE id IN ({})", qm(run_ids.len()));
                let n = tx.execute(&sql, params_from_iter(run_ids.iter()))?;
                out.push(RowCount {
                    table: "workflow_run".into(),
                    rows: n as i64,
                });
            }
        }
        let sql = format!("DELETE FROM session WHERE id IN ({})", qm(ids.len()));
        let n = tx.execute(&sql, params_from_iter(ids.iter()))?;
        out.push(RowCount {
            table: "session".into(),
            rows: n as i64,
        });

        tx.commit()?;
        let _ = con.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        Ok(out)
    }

    fn delete_index(&self, ids: &[String]) -> Result<(Vec<RowCount>, Vec<String>)> {
        let mut warnings = Vec::new();
        if !self.paths.tasks_db.is_file() {
            warnings.push("tasks index missing — index cleanup skipped".into());
            return Ok((Vec::new(), warnings));
        }
        let mut con = open_rw(&self.paths.tasks_db)?;
        let tables = existing_tables(&con);
        let tx = con.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut out = Vec::new();

        for (t, col) in [
            ("task_group_members", "task_id"),
            ("automation_runs", "session_id"),
            ("off_peak_tasks", "session_id"),
        ] {
            if tables.contains(t) {
                let sql = format!("DELETE FROM {t} WHERE {col} IN ({})", qm(ids.len()));
                let n = tx.execute(&sql, params_from_iter(ids.iter()))?;
                out.push(RowCount {
                    table: t.to_string(),
                    rows: n as i64,
                });
            }
        }
        if tables.contains("automations") {
            let sql = format!(
                "SELECT COUNT(*) FROM automations WHERE target_task_id IN ({})",
                qm(ids.len())
            );
            let n: i64 = tx.query_row(&sql, params_from_iter(ids.iter()), |r| r.get(0))?;
            if n > 0 {
                warnings.push(format!(
                    "{n} automation config(s) still reference the deleted sessions"
                ));
            }
        }
        if tables.contains("tasks") {
            let sql = format!("DELETE FROM tasks WHERE task_id IN ({})", qm(ids.len()));
            let n = tx.execute(&sql, params_from_iter(ids.iter()))?;
            out.push(RowCount {
                table: "tasks".into(),
                rows: n as i64,
            });
        }

        tx.commit()?;
        let _ = con.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        Ok((out, warnings))
    }
}

use rusqlite::OptionalExtension;
