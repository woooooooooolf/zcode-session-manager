//! Unit tests on throwaway fake databases. Never touch real ZCode data.

use crate::backup;
use crate::compat;
use crate::integrity;
use crate::paths::Paths;
use crate::store::Store;
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
CREATE TABLE permission (id TEXT PRIMARY KEY, project_id TEXT, data TEXT);
";

const TI_SCHEMA: &str = "
CREATE TABLE tasks (task_id TEXT PRIMARY KEY, title TEXT,
    pinned INTEGER DEFAULT 0, archived INTEGER DEFAULT 0, deleted INTEGER DEFAULT 0);
CREATE TABLE task_group_members (id TEXT PRIMARY KEY, task_id TEXT);
CREATE TABLE automation_runs (run_id TEXT PRIMARY KEY, session_id TEXT);
CREATE TABLE off_peak_tasks (off_peak_task_id TEXT PRIMARY KEY, session_id TEXT);
CREATE TABLE automations (automation_id TEXT PRIMARY KEY, target_task_id TEXT);
";

const ROOT: &str = "sess_root0000-0000-0000-000000000001";
const CHILD: &str = "sess_child000-0000-0000-000000000002";
const KEEP: &str = "sess_keep0000-0000-0000-000000000003";
const GHOST: &str = "sess_ghost00-0000-0000-000000000004";

struct Fixture {
    #[allow(dead_code)]
    root: PathBuf,
    paths: Paths,
    store: Store,
}

fn make_fixture() -> Fixture {
    crate::set_zcode_check_disabled(true);
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
    con.execute("INSERT INTO permission VALUES ('perm1', 'proj1', '{}')", []).unwrap();
    con.close().unwrap();

    let v2 = root.join("v2");
    std::fs::create_dir_all(&v2).unwrap();
    let con = Connection::open(v2.join("tasks-index.sqlite")).unwrap();
    con.execute_batch(TI_SCHEMA).unwrap();
    for (id, archived) in [(ROOT, 1), (CHILD, 0), (KEEP, 0), (GHOST, 1)] {
        con.execute(
            "INSERT INTO tasks (task_id, title, archived) VALUES (?1, ?2, ?3)",
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
    Fixture { root, paths, store }
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

    let con = Connection::open(fx.paths.db_path.clone()).unwrap();
    assert_eq!(count(&con, "SELECT COUNT(*) FROM session"), 1);
    assert_eq!(count(&con, "SELECT COUNT(*) FROM permission"), 1);
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
    let backup_dir = backup::new_backup_dir(&fx.paths).unwrap();
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
