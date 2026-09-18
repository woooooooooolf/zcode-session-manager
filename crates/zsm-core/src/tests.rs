//! Unit tests on throwaway fake databases. Never touch real ZCode data.

use crate::backup;
use crate::compat;
use crate::integrity;
use crate::paths::Paths;
use crate::store::{RunningPolicy, Store};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let d = std::env::temp_dir().join(format!("zsm-core-test-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

const DB_SCHEMA: &str = "
CREATE TABLE session (id TEXT PRIMARY KEY, parent_id TEXT, directory TEXT, title TEXT,
    task_type TEXT DEFAULT 'interactive', time_created INTEGER, time_updated INTEGER);
CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT, sequence INTEGER, data TEXT);
CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT, session_id TEXT, sequence INTEGER, data TEXT);
CREATE TABLE session_entry (id TEXT PRIMARY KEY, session_id TEXT, type TEXT, data TEXT);
CREATE TABLE session_input (id TEXT PRIMARY KEY, session_id TEXT, kind TEXT);
CREATE TABLE session_target (session_id TEXT, target_id TEXT);
CREATE TABLE todo (session_id TEXT, content TEXT);
CREATE TABLE tool_usage (id TEXT PRIMARY KEY, session_id TEXT, tool_name TEXT);
CREATE TABLE turn_usage (session_id TEXT, turn_id TEXT, output_tokens INTEGER);
CREATE TABLE model_usage (id TEXT PRIMARY KEY, session_id TEXT, model_id TEXT, input_tokens INTEGER, output_tokens INTEGER);
CREATE TABLE input_history (id TEXT PRIMARY KEY, project_id TEXT, session_id TEXT, text TEXT);
CREATE TABLE session_task_link (id TEXT PRIMARY KEY, parent_session_id TEXT, child_session_id TEXT);
CREATE TABLE workflow_run (id TEXT PRIMARY KEY, parent_session_id TEXT);
CREATE TABLE workflow_event (id TEXT PRIMARY KEY, run_id TEXT);
CREATE TABLE workflow_activity (id TEXT PRIMARY KEY, run_id TEXT);
CREATE TABLE dwf_run (id TEXT PRIMARY KEY, parent_session_id TEXT);
CREATE TABLE dwf_node (id INTEGER PRIMARY KEY, run_id TEXT);
CREATE TABLE dwf_event (id INTEGER PRIMARY KEY, run_id TEXT);
CREATE TABLE dwf_actor (id INTEGER PRIMARY KEY, run_id TEXT, session_id TEXT);
CREATE TABLE permission (id TEXT PRIMARY KEY, project_id TEXT, data TEXT);
";

const TI_SCHEMA: &str = "
CREATE TABLE tasks (task_id TEXT PRIMARY KEY, title TEXT,
    pinned INTEGER DEFAULT 0, archived INTEGER DEFAULT 0, deleted INTEGER DEFAULT 0,
    updated_at INTEGER DEFAULT 0);
CREATE TABLE task_group_members (id TEXT PRIMARY KEY, task_id TEXT);
CREATE TABLE automation_runs (run_id TEXT PRIMARY KEY, session_id TEXT);
CREATE TABLE off_peak_tasks (off_peak_task_id TEXT PRIMARY KEY, session_id TEXT);
CREATE TABLE automations (automation_id TEXT PRIMARY KEY, target_task_id TEXT, enabled INTEGER DEFAULT 1);
";

const ROOT: &str = "sess_root0000-0000-0000-000000000001";
const CHILD: &str = "sess_child000-0000-0000-000000000002";
const KEEP: &str = "sess_keep0000-0000-0000-000000000003";
const GHOST: &str = "sess_ghost00-0000-0000-000000000004";

/// The running-override hook is a process-global; tests that set different
/// values must not run concurrently.
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct Fixture {
    #[allow(dead_code)]
    root: PathBuf,
    paths: Paths,
    store: Store,
    _guard: std::sync::MutexGuard<'static, ()>,
}

fn make_fixture() -> Fixture {
    let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::set_zcode_running_override(Some(false));
    let root = temp_dir();
    let cli = root.join("cli");
    std::fs::create_dir_all(cli.join("db")).unwrap();

    let con = Connection::open(cli.join("db").join("db.sqlite")).unwrap();
    con.execute_batch(DB_SCHEMA).unwrap();
    con.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
    for (id, parent) in [(ROOT, None), (CHILD, Some(ROOT)), (KEEP, None)] {
        con.execute(
            "INSERT INTO session (id, parent_id, directory, title, time_created, time_updated) \
             VALUES (?1, ?2, ?3, ?4, 1000, 2000)",
            rusqlite::params![id, parent, "D:\\proj\\demo", format!("title-{id}")],
        )
        .unwrap();
    }
    for (i, sid) in [ROOT, ROOT, ROOT, CHILD, CHILD, KEEP].iter().enumerate() {
        con.execute(
            "INSERT INTO message (id, session_id, sequence, data) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![format!("m{i}"), sid, i as i64, r#"{"role":"user","source":"real_user"}"#],
        )
        .unwrap();
        con.execute(
            "INSERT INTO part (id, message_id, session_id, sequence, data) VALUES (?1, ?2, ?3, 0, ?4)",
            rusqlite::params![format!("p{i}"), format!("m{i}"), sid, r#"{"type":"text","text":"hi"}"#],
        )
        .unwrap();
    }
    con.execute("INSERT INTO todo VALUES (?1, 't')", [ROOT]).unwrap();
    con.execute("INSERT INTO model_usage VALUES ('mu1', ?1, 'glm', 1, 2)", [ROOT]).unwrap();
    con.execute("INSERT INTO session_task_link VALUES ('l1', ?1, ?2)", rusqlite::params![ROOT, CHILD])
        .unwrap();
    con.execute("INSERT INTO workflow_run VALUES ('wr1', ?1)", [ROOT]).unwrap();
    con.execute("INSERT INTO workflow_event VALUES ('we1', 'wr1')", []).unwrap();
    con.execute("INSERT INTO workflow_activity VALUES ('wa1', 'wr1')", []).unwrap();
    // dwf journal: dwf1 belongs to ROOT (actor names CHILD as its session);
    // dwf2 belongs to KEEP and must survive a ROOT-tree delete untouched
    con.execute("INSERT INTO dwf_run VALUES ('dwf1', ?1)", [ROOT]).unwrap();
    con.execute("INSERT INTO dwf_node VALUES (1, 'dwf1')", []).unwrap();
    con.execute("INSERT INTO dwf_event VALUES (1, 'dwf1')", []).unwrap();
    con.execute("INSERT INTO dwf_actor VALUES (1, 'dwf1', ?1)", [CHILD]).unwrap();
    con.execute("INSERT INTO dwf_run VALUES ('dwf2', ?1)", [KEEP]).unwrap();
    con.execute("INSERT INTO dwf_actor VALUES (2, 'dwf2', ?1)", [KEEP]).unwrap();
    con.execute("INSERT INTO permission VALUES ('perm1', 'proj1', '{}')", []).unwrap();
    con.close().unwrap();

    let v2 = root.join("v2");
    std::fs::create_dir_all(&v2).unwrap();
    let con = Connection::open(v2.join("tasks-index.sqlite")).unwrap();
    con.execute_batch(TI_SCHEMA).unwrap();
    for (id, archived) in [(ROOT, 1), (CHILD, 0), (KEEP, 0), (GHOST, 1)] {
        con.execute(
            "INSERT INTO tasks (task_id, title, archived, updated_at) VALUES (?1, ?2, ?3, 1000)",
            rusqlite::params![id, format!("title-{id}"), archived],
        )
        .unwrap();
    }
    con.execute("INSERT INTO task_group_members VALUES ('gm1', ?1)", [ROOT]).unwrap();
    con.close().unwrap();

    // disk artifacts
    let rollout = cli.join("rollout");
    std::fs::create_dir_all(&rollout).unwrap();
    std::fs::write(rollout.join(format!("model-io-{ROOT}.jsonl")), "x".repeat(1024)).unwrap();
    let exec = cli.join("exec").join(ROOT);
    std::fs::create_dir_all(&exec).unwrap();
    std::fs::write(exec.join("out.txt"), "y".repeat(512)).unwrap();
    let art = cli.join("artifacts").join(ROOT);
    std::fs::create_dir_all(&art).unwrap();
    std::fs::write(art.join("a.bin"), "z".repeat(256)).unwrap();
    let keep_img = cli.join("image-cache").join(KEEP);
    std::fs::create_dir_all(&keep_img).unwrap();
    std::fs::write(keep_img.join("k.png"), "k".repeat(64)).unwrap();

    let paths = Paths::from_zcode_dir(&root);
    let store = Store::new(paths.clone());
    Fixture {
        root,
        paths,
        store,
        _guard: guard,
    }
}

fn count(con: &Connection, sql: &str) -> i64 {
    con.query_row(sql, [], |r| r.get(0)).unwrap()
}

#[test]
fn list_sessions_flags_and_ghost() {
    let fx = make_fixture();
    let mut sessions = fx.store.list_sessions().unwrap();
    sessions.sort_by(|a, b| a.id.cmp(&b.id));

    let root = sessions.iter().find(|s| s.id == ROOT).unwrap();
    assert!(root.archived);
    assert_eq!(root.message_count, Some(3));
    assert_eq!(root.child_count, 1);
    assert!(root.disk_bytes >= 1024 + 512 + 256);

    let keep = sessions.iter().find(|s| s.id == KEEP).unwrap();
    assert!(!keep.archived);

    let ghost = sessions.iter().find(|s| s.id == GHOST).unwrap();
    assert!(ghost.ghost);
    assert_eq!(ghost.message_count, None);
    assert_eq!(sessions.len(), 4);
}

#[test]
fn plan_delete_touches_nothing() {
    let fx = make_fixture();
    let plan = fx.store.plan_delete(&[ROOT.to_string()]).unwrap();
    assert!(plan.all_ids.contains(&ROOT.to_string()));
    assert!(plan.all_ids.contains(&CHILD.to_string()));
    assert_eq!(plan.all_ids.len(), 2);
    assert_eq!(plan.counts.iter().find(|c| c.table == "message").unwrap().rows, 5);
    assert_eq!(plan.counts.iter().find(|c| c.table == "session").unwrap().rows, 2);
    assert_eq!(plan.counts.iter().find(|c| c.table == "dwf_run").unwrap().rows, 1);
    assert_eq!(plan.counts.iter().find(|c| c.table == "dwf_actor").unwrap().rows, 1);
    assert!(plan.disk_bytes > 0);

    // nothing changed
    let con = Connection::open(fx.paths.db_path.clone()).unwrap();
    assert_eq!(count(&con, "SELECT COUNT(*) FROM session"), 3);
    assert!(fx
        .paths
        .cli_dir
        .join("rollout")
        .join(format!("model-io-{ROOT}.jsonl"))
        .exists());
}

#[test]
fn execute_delete_cascades_content_disk_index() {
    let fx = make_fixture();
    let result = fx.store.execute_delete(&[ROOT.to_string()]).unwrap();

    // content: root + child gone, keep + shared tables preserved
    let deleted = &result.deleted;
    assert_eq!(deleted.iter().find(|c| c.table == "message").unwrap().rows, 5);
    assert_eq!(deleted.iter().find(|c| c.table == "session").unwrap().rows, 2);
    assert_eq!(deleted.iter().find(|c| c.table == "session_task_link").unwrap().rows, 1);
    assert_eq!(deleted.iter().find(|c| c.table == "workflow_run").unwrap().rows, 1);
    assert_eq!(deleted.iter().find(|c| c.table == "dwf_run").unwrap().rows, 1);
    assert_eq!(deleted.iter().find(|c| c.table == "dwf_node").unwrap().rows, 1);
    assert_eq!(deleted.iter().find(|c| c.table == "dwf_actor").unwrap().rows, 1);

    let con = Connection::open(fx.paths.db_path.clone()).unwrap();
    assert_eq!(count(&con, "SELECT COUNT(*) FROM session"), 1);
    assert_eq!(count(&con, "SELECT COUNT(*) FROM permission"), 1);
    // dwf journal of the surviving session stays intact
    assert_eq!(count(&con, "SELECT COUNT(*) FROM dwf_run"), 1);
    assert_eq!(count(&con, "SELECT COUNT(*) FROM dwf_actor"), 1);
    assert_eq!(count(&con, "SELECT COUNT(*) FROM dwf_node"), 0);
    con.close().unwrap();

    // disk: root artifacts gone, keep's image-cache preserved
    assert!(!fx.paths.cli_dir.join("rollout").join(format!("model-io-{ROOT}.jsonl")).exists());
    assert!(!fx.paths.cli_dir.join("exec").join(ROOT).exists());
    assert!(!fx.paths.cli_dir.join("artifacts").join(ROOT).exists());
    assert!(fx.paths.cli_dir.join("image-cache").join(KEEP).exists());

    // index: root+child tasks rows gone, keep+ghost remain
    let con = Connection::open(fx.paths.tasks_db.clone()).unwrap();
    assert_eq!(count(&con, "SELECT COUNT(*) FROM tasks"), 2);
    assert_eq!(count(&con, "SELECT COUNT(*) FROM task_group_members"), 0);
    con.close().unwrap();

    // backup contains everything needed for a restore
    let backup_dir = PathBuf::from(&result.backup_dir);
    assert!(backup_dir.join("db.sqlite").is_file());
    assert!(backup_dir.join("tasks-index.sqlite").is_file());
    assert!(backup_dir.join("manifest.json").is_file());
    assert!(backup_dir.join("disk").join("rollout").join(format!("model-io-{ROOT}.jsonl")).is_file());

    assert!(result.integrity.passed);
    assert!(!result.integrity.restored);
}

#[test]
fn ghost_cleanup_removes_index_rows_only() {
    let fx = make_fixture();
    let result = fx.store.execute_delete(&[GHOST.to_string()]).unwrap();
    assert_eq!(result.deleted.iter().find(|c| c.table == "session").unwrap().rows, 0);
    assert_eq!(result.index.iter().find(|c| c.table == "tasks").unwrap().rows, 1);
    let con = Connection::open(fx.paths.tasks_db.clone()).unwrap();
    assert_eq!(count(&con, "SELECT COUNT(*) FROM tasks"), 3);
}

#[test]
fn restore_from_backup_recovers_corrupted_db() {
    let fx = make_fixture();
    // build a backup the same way execute_delete does
    let backup_dir = backup::new_backup_dir(&fx.paths, None).unwrap();
    backup::copy_database(&fx.paths.db_path, &backup_dir).unwrap();
    backup::copy_database(&fx.paths.tasks_db, &backup_dir).unwrap();
    let records =
        backup::copy_session_disk(&fx.paths.cli_dir, &[ROOT.to_string()], &backup_dir).unwrap();

    // corrupt the live db
    std::fs::write(&fx.paths.db_path, b"not a sqlite file at all").unwrap();
    let status = integrity::check_db(&fx.paths.db_path);
    assert!(!status.is_ok());

    backup::restore_from_backup(&backup_dir, &fx.paths, &records).unwrap();
    assert!(integrity::check_db(&fx.paths.db_path).is_ok());
    let con = Connection::open(fx.paths.db_path.clone()).unwrap();
    assert_eq!(count(&con, "SELECT COUNT(*) FROM session"), 3);
}

#[test]
fn compat_check_passes_on_valid_layout_and_fails_on_drift() {
    let fx = make_fixture();
    let report = compat::check(&fx.paths);
    assert!(report.ok, "problems: {:?}", report.problems);
    assert!(report.db_present);
    assert!(report.tasks_present);

    // schema drift: drop a required column
    let broken = temp_dir();
    let cli = broken.join("cli");
    std::fs::create_dir_all(cli.join("db")).unwrap();
    let con = Connection::open(cli.join("db").join("db.sqlite")).unwrap();
    con.execute_batch(
        "CREATE TABLE session (id TEXT PRIMARY KEY, parent_id TEXT, title TEXT); \
         CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT); \
         CREATE TABLE part (id TEXT PRIMARY KEY, session_id TEXT);",
    )
    .unwrap();
    con.close().unwrap();
    let report = compat::check(&Paths::from_zcode_dir(&broken));
    assert!(!report.ok);
    assert!(report.problems.iter().any(|p| p.contains("directory")));
}

// ---------------- limited mode (delete while ZCode runs) ----------------

#[test]
fn limited_mode_allows_archived_idle_tree_and_rejects_unarchived_roots() {
    let fx = make_fixture();
    crate::set_zcode_running_override(Some(true));
    // fixture: ROOT archived + old, CHILD (non-archived fork) old, KEEP not archived
    let policy = RunningPolicy::Limited { idle_minutes: 60 };

    // archived root with its non-archived child: allowed (children exempt)
    let r = fx
        .store
        .execute_delete_with_policy(&[ROOT.to_string()], policy)
        .unwrap();
    assert_eq!(r.deleted.iter().find(|c| c.table == "session").unwrap().rows, 2);

    // unarchived root: rejected with reason
    let err = fx
        .store
        .execute_delete_with_policy(&[KEEP.to_string()], policy)
        .unwrap_err();
    match err {
        crate::Error::Limited { violations } => {
            assert_eq!(violations, vec![(KEEP.to_string(), "not_archived")]);
        }
        other => panic!("expected Limited, got {other:?}"),
    }
}

#[test]
fn limited_mode_rejects_recent_and_automation_referenced() {
    let fx = make_fixture();
    crate::set_zcode_running_override(Some(true));
    let policy = RunningPolicy::Limited { idle_minutes: 60 };

    // freshly updated session -> too_recent (archived alone is not enough)
    let con = Connection::open(fx.paths.db_path.clone()).unwrap();
    con.execute(
        "UPDATE session SET time_updated = ?1 WHERE id = ?2",
        rusqlite::params![chrono::Utc::now().timestamp_millis(), ROOT],
    )
    .unwrap();
    con.close().unwrap();
    let err = fx
        .store
        .execute_delete_with_policy(&[ROOT.to_string()], policy)
        .unwrap_err();
    match err {
        crate::Error::Limited { violations } => {
            assert_eq!(violations, vec![(ROOT.to_string(), "too_recent")]);
        }
        other => panic!("expected Limited, got {other:?}"),
    }

    // make ROOT idle again before probing the automation rule
    let con = Connection::open(fx.paths.db_path.clone()).unwrap();
    con.execute("UPDATE session SET time_updated = 2000 WHERE id = ?1", [ROOT]).unwrap();
    con.close().unwrap();

    // enabled automation referencing the root -> automation_ref
    let con = Connection::open(fx.paths.tasks_db.clone()).unwrap();
    con.execute(
        "INSERT INTO automations (automation_id, target_task_id) VALUES ('a1', ?1)",
        [ROOT],
    )
    .unwrap();
    con.close().unwrap();
    let err = fx
        .store
        .execute_delete_with_policy(&[ROOT.to_string()], policy)
        .unwrap_err();
    match err {
        crate::Error::Limited { violations } => {
            assert_eq!(violations, vec![(ROOT.to_string(), "automation_ref")]);
        }
        other => panic!("expected Limited, got {other:?}"),
    }

    // disabled automation -> no violation, delete proceeds
    let con = Connection::open(fx.paths.tasks_db.clone()).unwrap();
    con.execute("UPDATE automations SET enabled = 0 WHERE automation_id = 'a1'", []).unwrap();
    con.close().unwrap();
    assert!(fx
        .store
        .execute_delete_with_policy(&[ROOT.to_string()], policy)
        .is_ok());
}

#[test]
fn limited_mode_threshold_bounds_are_enforced() {
    let fx = make_fixture();
    crate::set_zcode_running_override(Some(true));
    // 30 minutes ago: inside a 60-minute threshold -> too_recent
    let con = Connection::open(fx.paths.db_path.clone()).unwrap();
    con.execute(
        "UPDATE session SET time_updated = ?1 WHERE id = ?2",
        rusqlite::params![chrono::Utc::now().timestamp_millis() - 30 * 60_000, ROOT],
    )
    .unwrap();
    con.close().unwrap();
    let err = fx
        .store
        .execute_delete_with_policy(&[ROOT.to_string()], RunningPolicy::Limited { idle_minutes: 60 })
        .unwrap_err();
    match err {
        crate::Error::Limited { violations } => {
            assert_eq!(violations, vec![(ROOT.to_string(), "too_recent")]);
        }
        other => panic!("expected Limited, got {other:?}"),
    }
    // with a 20-minute threshold the same session counts as idle -> passes
    assert!(fx
        .store
        .execute_delete_with_policy(&[ROOT.to_string()], RunningPolicy::Limited { idle_minutes: 20 })
        .is_ok());
}

#[test]
fn refuse_and_full_mode_interplay() {
    // running + Refuse -> refused
    let fx = make_fixture();
    crate::set_zcode_running_override(Some(true));
    assert!(matches!(
        fx.store.execute_delete_with_policy(&[GHOST.to_string()], RunningPolicy::Refuse),
        Err(crate::Error::ZcodeRunning)
    ));
    // not running + Refuse -> full delete works
    crate::set_zcode_running_override(Some(false));
    assert!(fx
        .store
        .execute_delete_with_policy(&[GHOST.to_string()], RunningPolicy::Refuse)
        .is_ok());
    // not running + Limited behaves identically (policy only consulted while running)
    drop(fx); // release TEST_LOCK before the next fixture acquires it
    let fx2 = make_fixture();
    assert!(fx2
        .store
        .execute_delete_with_policy(
            &[GHOST.to_string()],
            RunningPolicy::Limited { idle_minutes: 60 }
        )
        .is_ok());
}

// ---------------- privacy cleaner ----------------

fn privacy_fixture() -> (PathBuf, crate::privacy::PrivacyPaths) {
    let root = temp_dir();
    let v2 = root.join("v2");
    std::fs::create_dir_all(v2.join("checkpoints").join("abc123")).unwrap();
    std::fs::create_dir_all(v2.join("logs")).unwrap();
    std::fs::create_dir_all(v2.join("crash").join("live")).unwrap();
    std::fs::create_dir_all(root.join("cli").join("log")).unwrap();
    std::fs::write(v2.join("checkpoints").join("abc123").join("state.json"), "{}").unwrap();
    std::fs::write(v2.join("telemetry-state.json"), "{\"deviceMid\":\"x\"}").unwrap();
    std::fs::write(v2.join("logs").join("2026.log"), "log").unwrap();
    std::fs::write(v2.join("crash").join("live").join("a.dmp"), "dump").unwrap();
    std::fs::write(root.join("cli").join("log").join("zcode.jsonl"), "{}").unwrap();
    std::fs::write(
        v2.join("setting.json"),
        r#"{"recentProjects":["D:\\proj"],"lastWorkspaceSession":[{"workspacePath":"D:\\proj"}],"locale":"zh-CN"}"#,
    )
    .unwrap();
    // roaming / local desktop paths stay unwired so tests never touch real data
    let paths = crate::privacy::PrivacyPaths {
        zcode: Some(root.clone()),
        roaming_zcode: None,
        local_updater: None,
    };
    (root, paths)
}

#[test]
fn privacy_scan_and_clean_roundtrip() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::set_zcode_running_override(Some(false));
    let (root, paths) = privacy_fixture();

    let entries = crate::privacy::scan(&paths);
    let by_id = |id: &str| entries.iter().find(|e| e.id == id).unwrap();
    assert!(by_id("repo_snapshots").present);
    assert_eq!(by_id("repo_snapshots").files, 1);
    assert!(by_id("telemetry_state").present);
    assert!(by_id("desktop_logs").present);
    assert!(by_id("cli_logs").present);
    assert!(by_id("crash_reports").present);
    assert!(by_id("recent_projects").present, "{:?}", by_id("recent_projects"));
    assert!(!by_id("browser_profile").present);
    assert!(!by_id("updater_cache").present);

    let ids: Vec<String> = ["repo_snapshots", "telemetry_state", "desktop_logs", "cli_logs", "crash_reports"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let report = crate::privacy::clean(&paths, &ids).unwrap();
    assert!(report.outcomes.iter().all(|o| o.ok), "{:?}", report.outcomes);
    // directory shells are kept, contents removed
    assert!(root.join("v2/checkpoints").read_dir().unwrap().next().is_none());
    assert!(root.join("v2/logs").read_dir().unwrap().next().is_none());
    assert!(root.join("v2/crash").read_dir().unwrap().next().is_none());
    assert!(!root.join("v2/telemetry-state.json").exists());
}

#[test]
fn privacy_clean_refuses_while_zcode_runs() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::set_zcode_running_override(Some(true));
    let (_root, paths) = privacy_fixture();
    assert!(matches!(
        crate::privacy::clean(&paths, &["cli_logs".into()]),
        Err(crate::Error::ZcodeRunning)
    ));
    assert!(matches!(
        crate::privacy::apply_switches(&paths),
        Err(crate::Error::ZcodeRunning)
    ));
    crate::set_zcode_running_override(Some(false));
}

#[test]
fn privacy_recent_projects_and_switches() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::set_zcode_running_override(Some(false));
    let (root, paths) = privacy_fixture();

    let report = crate::privacy::clean(&paths, &["recent_projects".into()]).unwrap();
    assert!(report.outcomes[0].ok, "{:?}", report.outcomes);
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("v2/setting.json")).unwrap()).unwrap();
    assert_eq!(v["recentProjects"].as_array().unwrap().len(), 0);
    assert_eq!(v["lastWorkspaceSession"].as_array().unwrap().len(), 0);
    assert_eq!(v["locale"], "zh-CN"); // untouched keys survive

    assert_eq!(crate::privacy::switches(&paths).optimize_agent_experience, None);
    std::fs::write(
        root.join("v2/setting.json"),
        r#"{"optimizeAgentExperienceEnabled":true,"repoSnapshotIndexingEnabled":true}"#,
    )
    .unwrap();
    let sw = crate::privacy::apply_switches(&paths).unwrap();
    assert_eq!(sw.optimize_agent_experience, Some(false));
    assert_eq!(sw.repo_snapshot_indexing, Some(false));
    assert_eq!(sw.instant_grep_indexing, Some(false));
}

#[test]
fn privacy_snapshot_report_parses_checkpoints() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp_dir();
    let cp = root.join("v2").join("checkpoints").join("37107b86eb71");
    std::fs::create_dir_all(cp.join("manifests")).unwrap();
    std::fs::create_dir_all(cp.join("extra-manifests")).unwrap();
    std::fs::write(
        cp.join("state.json"),
        r#"{"workspacePath":"D:\\ws","workspaceKey":"D:\\ws","lastCompressedSize":{"encryptedSizeBytes":77560,"workspaceSizeBytes":2264294,"manifestHash":"abc","recordedAt":1789654247740},"lastAcceptedManifestHash":"abc","failureCount":2}"#,
    )
    .unwrap();
    std::fs::write(
        cp.join("manifests").join("abc.json"),
        r#"{"schema":"repo_snapshot_manifest/v2","workspaceKey":"D:\\ws","createdAt":1789654247740,"files":[{"path":".git/HEAD","sizeBytes":21},{"path":".git/config","sizeBytes":401},{"path":"src/main.rs","sizeBytes":500}]}"#,
    )
    .unwrap();
    std::fs::write(
        cp.join("extra-manifests").join("3fb.json"),
        r#"{"schema":"repo_snapshot_extra_manifest/v1","createdAt":1789654247740,"groups":[{"groupId":"global-configs","files":[{"path":"settings.behavior.json","sizeBytes":760}]}],"stats":{"includedFileCount":1,"includedBytes":760}}"#,
    )
    .unwrap();

    let paths = crate::privacy::PrivacyPaths { zcode: Some(root.clone()), roaming_zcode: None, local_updater: None };
    let reps = crate::privacy::snapshot_report(&paths);
    assert_eq!(reps.len(), 1);
    let r = &reps[0];
    assert_eq!(r.workspace_path, "D:\\ws");
    assert_eq!(r.recorded_at_ms, Some(1789654247740));
    assert_eq!(r.workspace_bytes, Some(2264294));
    assert_eq!(r.encrypted_bytes, Some(77560));
    assert_eq!(r.accepted, Some(true));
    assert_eq!(r.failure_count, Some(2));
    assert!(r.manifest_on_disk);
    assert_eq!(r.files.len(), 3);
    assert_eq!(r.git_files, 2);
    assert_eq!(r.files_bytes, 21 + 401 + 500);
    assert_eq!(r.extra_files, 1);
    assert_eq!(r.extra_bytes, 760);
    assert!(r.state_modified_ms.is_some());
}

#[test]
fn privacy_discover_rejects_non_zcode_dirs() {
    // a random folder must never expose cli/* or v2/* paths for cleaning
    let empty = temp_dir();
    let p = crate::privacy::PrivacyPaths::discover(Some(&empty));
    assert!(p.zcode.is_none());

    let real = temp_dir();
    std::fs::create_dir_all(real.join("v2")).unwrap();
    std::fs::write(real.join("v2").join("tasks-index.sqlite"), "x").unwrap();
    let p2 = crate::privacy::PrivacyPaths::discover(Some(&real));
    assert!(p2.zcode.is_some());
}

#[test]
fn privacy_clean_ignores_unknown_ids() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    crate::set_zcode_running_override(Some(false));
    let (root, paths) = privacy_fixture();
    let report = crate::privacy::clean(
        &paths,
        &["../../secrets".into(), "sessions_db".into(), "".into()],
    )
    .unwrap();
    assert!(report.outcomes.is_empty());
    // nothing at all was touched
    assert!(root.join("v2/setting.json").is_file());
    assert!(root.join("v2/telemetry-state.json").is_file());
    assert!(root.join("cli/log/zcode.jsonl").is_file());
}
