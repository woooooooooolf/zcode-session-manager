"""Generate a fictional demo ZCode data directory for the README screenshots.

The schema mirrors the REAL ZCode 3.16.2 (desktop 3.16.2.8769 / engine 0.16.9)
layout verified on 2026-10-20 — same table sets, same column names, message
JSON in the real shape (role + semantics.origin + source), soft-deleted
session included so every list badge can appear, plus the privacy-side data
(visualizations, migration backups, checkpoints, …). All content is fiction.

Usage:
    python tools/make_demo_data.py [--lang zh|en] [--out DIR]

Default output lives inside the repo at demo-data/zcode (gitignored — it is
regenerated wholesale on every run, default path included). Nothing ever
touches real ZCode data.
"""

import argparse
import json
import os
import random
import shutil
import sqlite3
import time

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

NOW = int(time.time() * 1000)
H = 3600 * 1000
D = 24 * H


def ts(y, m, d, hh=10, mm=30):
    import datetime
    return int(datetime.datetime(y, m, d, hh, mm).timestamp() * 1000)


# key, title, project, parent, created, updated, msgs, archived, pinned, kb, deleted
TITLES = {
    "zh": [
        ("perf",   "首页性能优化调研",            r"C:\Users\demo\workspace\default", None,      ts(2026, 8, 20), NOW - 3 * H, 210, 0, 1, 5600, 0),
        ("meet",   "会议纪要要点提取",            r"C:\Users\demo\workspace\default", None,      ts(2026, 8, 26), NOW - 2 * H,  12, 0, 0,   30, 0),
        ("backup", "数据备份方案对比",            r"C:\Users\demo\workspace\default", None,      ts(2026, 6, 15), NOW - 5 * H,  38, 0, 0,  340, 0),
        ("travel", "旅行行程规划助手测试",        r"C:\Users\demo\workspace\default", None,      ts(2026, 7,  2), NOW - 1 * D,  64, 0, 0,  800, 0),
        ("fwp",    "嵌入式固件版本升级排期",      r"D:\work\embedded-fw",             None,      ts(2025, 11, 4), ts(2026, 1, 12), 156, 1, 0, 3900, 0),
        ("fws",    "固件升级排期-子任务拆解",     r"D:\work\embedded-fw",             "fwp",     ts(2026, 1, 12), ts(2026, 1, 20),  31, 0, 1,   80, 0),
        ("repq",   "季度经营分析报告整理",        r"C:\Users\demo\projects\report",   None,      ts(2025, 9, 18), ts(2025, 12, 8), 87, 1, 0, 1400, 0),
        ("repc",   "客户反馈数据清洗脚本",        r"C:\Users\demo\projects\report",   None,      ts(2025, 6, 10), ts(2025, 8, 22), 42, 1, 0,  240, 0),
        ("week",   "周报自动汇总工具",            r"C:\Users\demo\projects\report",   None,      ts(2025, 4, 25), ts(2025, 7, 30), 23, 1, 0,   64, 0),
        ("sq",     "SQLite 学习笔记",             r"C:\Users\demo\workspace\default", None,      ts(2025, 3, 14), ts(2025, 5,  2),  9, 1, 0,   12, 0),
        ("draft",  "旧版迁移方案草稿",            r"C:\Users\demo\workspace\default", None,      ts(2025, 5, 20), ts(2025, 6,  1), 18, 0, 0,  210, 1),
    ],
    "en": [
        ("perf",   "Homepage performance research",   r"C:\Users\demo\workspace\default", None,      ts(2026, 8, 20), NOW - 3 * H, 210, 0, 1, 5600, 0),
        ("meet",   "Meeting notes extraction",        r"C:\Users\demo\workspace\default", None,      ts(2026, 8, 26), NOW - 2 * H,  12, 0, 0,   30, 0),
        ("backup", "Backup strategy comparison",      r"C:\Users\demo\workspace\default", None,      ts(2026, 6, 15), NOW - 5 * H,  38, 0, 0,  340, 0),
        ("travel", "Travel planner assistant test",   r"C:\Users\demo\workspace\default", None,      ts(2026, 7,  2), NOW - 1 * D,  64, 0, 0,  800, 0),
        ("fwp",    "Firmware upgrade scheduling",     r"D:\work\embedded-fw",             None,      ts(2025, 11, 4), ts(2026, 1, 12), 156, 1, 0, 3900, 0),
        ("fws",    "Firmware plan - subtask breakdown", r"D:\work\embedded-fw",           "fwp",     ts(2026, 1, 12), ts(2026, 1, 20),  31, 0, 1,   80, 0),
        ("repq",   "Quarterly business report",       r"C:\Users\demo\projects\report",   None,      ts(2025, 9, 18), ts(2025, 12, 8), 87, 1, 0, 1400, 0),
        ("repc",   "Customer feedback cleanup script", r"C:\Users\demo\projects\report",  None,      ts(2025, 6, 10), ts(2025, 8, 22), 42, 1, 0,  240, 0),
        ("week",   "Weekly report summarizer",        r"C:\Users\demo\projects\report",   None,      ts(2025, 4, 25), ts(2025, 7, 30), 23, 1, 0,   64, 0),
        ("sq",     "SQLite study notes",              r"C:\Users\demo\workspace\default", None,      ts(2025, 3, 14), ts(2025, 5,  2),  9, 1, 0,   12, 0),
        ("draft",  "Legacy migration draft",          r"C:\Users\demo\workspace\default", None,      ts(2025, 5, 20), ts(2025, 6,  1), 18, 0, 0,  210, 1),
    ],
}

# ---------------------------------------------------------------- db.sqlite --
# Column names/order mirror the real 3.16.2 layout (engine 0.16.9, migrations
# 0001..0024 applied). Types are approximations — the tooling only cares about
# names. Both workflow-journal generations exist in the real schema and stay
# empty there; same here.
DB_SCHEMA = """
CREATE TABLE session (
    id TEXT PRIMARY KEY, project_id TEXT, workspace_id TEXT, parent_id TEXT,
    slug TEXT, directory TEXT, path TEXT, title TEXT, version TEXT,
    share_url TEXT, summary_additions INTEGER, summary_deletions INTEGER,
    summary_files INTEGER, summary_diffs INTEGER, revert TEXT, permission TEXT,
    time_created INTEGER, time_updated INTEGER, time_compacting INTEGER,
    time_archived INTEGER, task_type TEXT, title_source TEXT,
    title_message_id TEXT, time_title_updated INTEGER, trace_id TEXT,
    time_deleted INTEGER, last_outcome TEXT);
CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT, time_created INTEGER,
    time_updated INTEGER, data TEXT, sequence INTEGER);
CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT, session_id TEXT,
    time_created INTEGER, time_updated INTEGER, data TEXT, sequence INTEGER);
CREATE TABLE session_entry (id TEXT PRIMARY KEY, session_id TEXT, type TEXT,
    time_created INTEGER, time_updated INTEGER, data TEXT);
CREATE TABLE session_input (id TEXT PRIMARY KEY, session_id TEXT, kind TEXT,
    delivery TEXT, payload TEXT, admitted_sequence INTEGER,
    promoted_sequence INTEGER, promoted_message_id TEXT, status TEXT,
    status_reason TEXT, time_created INTEGER, time_updated INTEGER);
CREATE TABLE session_target (session_id TEXT, target_id TEXT, objective TEXT,
    status TEXT, token_budget INTEGER, tokens_used INTEGER,
    time_used_seconds INTEGER, time_created INTEGER, time_updated INTEGER,
    summary_title TEXT, active_input_id TEXT, active_run_started_at INTEGER,
    active_run_last_seen_at INTEGER);
CREATE TABLE todo (session_id TEXT, content TEXT);
CREATE TABLE tool_usage (id TEXT PRIMARY KEY, session_id TEXT, tool_name TEXT,
    time_created INTEGER);
CREATE TABLE turn_usage (session_id TEXT, turn_id TEXT, output_tokens INTEGER);
CREATE TABLE model_usage (id TEXT PRIMARY KEY, session_id TEXT, model_id TEXT,
    input_tokens INTEGER, output_tokens INTEGER);
CREATE TABLE input_history (id TEXT PRIMARY KEY, project_id TEXT,
    session_id TEXT, text TEXT, kind TEXT, time_created INTEGER, attachments TEXT);
CREATE TABLE session_task_link (id TEXT PRIMARY KEY, root_workflow_run_id TEXT,
    parent_link_id TEXT, activity_id TEXT, parent_session_id TEXT,
    child_session_id TEXT, role TEXT, depth INTEGER, path TEXT, phase TEXT,
    label TEXT, agent_type TEXT, model TEXT, status TEXT, time_created INTEGER,
    time_updated INTEGER);
CREATE TABLE workflow_definition (id TEXT PRIMARY KEY, name TEXT, source TEXT,
    trusted INTEGER, enabled INTEGER, script_path TEXT, script_hash TEXT,
    meta_json TEXT, time_created INTEGER, time_updated INTEGER, scope TEXT);
CREATE TABLE workflow_run (id TEXT PRIMARY KEY, definition_id TEXT, name TEXT,
    kind TEXT, parent_session_id TEXT, cwd TEXT, script_path TEXT,
    script_hash TEXT, args_json TEXT, args_hash TEXT, status TEXT,
    current_phase TEXT, budget_total INTEGER, budget_spent INTEGER,
    stats_json TEXT, failure_json TEXT, time_created INTEGER,
    time_started INTEGER, time_updated INTEGER, time_completed INTEGER);
CREATE TABLE workflow_event (id TEXT PRIMARY KEY, run_id TEXT, sequence INTEGER,
    type TEXT, phase TEXT, activity_id TEXT, payload_json TEXT, time_created INTEGER);
CREATE TABLE workflow_activity (id TEXT PRIMARY KEY, run_id TEXT,
    parent_activity_id TEXT, call_index INTEGER, call_path TEXT, attempt INTEGER,
    type TEXT, phase TEXT, label TEXT, input_hash TEXT, prompt TEXT,
    opts_json TEXT, status TEXT, child_session_id TEXT, result_json TEXT,
    error_json TEXT, time_created INTEGER, time_started INTEGER,
    time_updated INTEGER, time_completed INTEGER);
CREATE TABLE dwf_run (id TEXT PRIMARY KEY, parent_session_id TEXT, cwd TEXT,
    name TEXT, script_text TEXT, script_hash TEXT, args_json TEXT,
    tool_call_id TEXT, resumed_from TEXT, caps_max_concurrency INTEGER,
    spent_tokens INTEGER, status TEXT, result_json TEXT, failure_json TEXT,
    time_created INTEGER, time_updated INTEGER);
CREATE TABLE dwf_actor (id TEXT PRIMARY KEY, run_id TEXT, site_id TEXT,
    ordinal INTEGER, name TEXT, persona_json TEXT, resolved_model TEXT,
    session_id TEXT, time_created INTEGER, time_updated INTEGER);
CREATE TABLE dwf_node (id TEXT PRIMARY KEY, run_id TEXT, site_id TEXT,
    ordinal INTEGER, kind TEXT, actor_site_id TEXT, actor_ordinal INTEGER,
    actor_seq INTEGER, input_hash TEXT, input_json TEXT, status TEXT,
    result_json TEXT, error_json TEXT, stats_json TEXT,
    message_boundary INTEGER, artifact_id TEXT, time_created INTEGER,
    time_updated INTEGER);
CREATE TABLE dwf_event (id TEXT PRIMARY KEY, run_id TEXT, sequence INTEGER,
    type TEXT, payload_json TEXT, time_created INTEGER);
CREATE TABLE permission (project_id TEXT PRIMARY KEY, time_created INTEGER,
    time_updated INTEGER, data TEXT);
CREATE TABLE local_setting (scope TEXT, scope_id TEXT, namespace TEXT, key TEXT,
    value TEXT, schema_version INTEGER, time_created INTEGER, time_updated INTEGER);
CREATE TABLE zcode_database_identity (scope TEXT PRIMARY KEY, identity TEXT);
CREATE TABLE schema_migration (id TEXT PRIMARY KEY, checksum TEXT,
    app_version TEXT, time_applied INTEGER);
"""

# The real migration ledger as of 3.16.2 (id, engine version).
DB_MIGRATIONS = [
    ("0001_base_session_store", "0.2.0"), ("0002_local_setting", "0.2.0"),
    ("0003_backfill_permission_local_setting", "0.2.0"),
    ("0004_session_target", "0.7.0"), ("0005_session_target_accounting", "0.7.0"),
    ("0006_input_history_attachments", "0.11.0"),
    ("0007_workflow_script_runtime", "0.13.0"),
    ("0008_workflow_definition_scope", "0.13.0"),
    ("0009_session_title_metadata", "0.14.0"),
    ("0010_usage_observability", "0.15.0"),
    ("0011_session_target_summary_title", "0.15.0"),
    ("0012_session_trace_id", "0.15.0"),
    ("0013_session_target_active_run_accounting", "0.15.0"),
    ("0014_message_part_sequence", "0.15.0"),
    ("0015_message_part_sequence_backfill_and_guard", "0.15.2"),
    ("0016_session_input_ledger", "0.15.2"),
    ("0017_session_input_start_now_delivery", "0.15.2"),
    ("0018_session_input_failed_status", "0.15.2"),
    ("0019_dwf_journal", "0.16.5"), ("0020_provider_model_selection", "0.16.5"),
    ("0021_official_glm_selection", "0.16.5"),
    ("0022_backfilled_session_reasoning", "0.16.5"),
    ("0023_session_soft_delete_outcome", "0.16.9"),
    ("0024_session_database_identity", "0.16.9"),
]

# ------------------------------------------------------- tasks-index.sqlite --
TI_SCHEMA = """
CREATE TABLE tasks (
    workspace_key TEXT, workspace_path TEXT, workspace_identity TEXT,
    task_id TEXT PRIMARY KEY, title TEXT, task_status TEXT, provider TEXT,
    mode TEXT, model TEXT, migration_source TEXT, forked_from_task_id TEXT,
    created_at INTEGER, updated_at INTEGER, unread_at INTEGER,
    last_unread_at INTEGER, pinned INTEGER DEFAULT 0,
    archived INTEGER DEFAULT 0, deleted INTEGER DEFAULT 0,
    title_overridden INTEGER DEFAULT 0, meta_json TEXT, searchable_text TEXT,
    cron_automation_id TEXT, off_peak_task_id TEXT, host_id TEXT,
    root_path TEXT, project_id TEXT, execution_directories TEXT,
    sort_order INTEGER, cli_title TEXT, handoff_pending INTEGER DEFAULT 0,
    cli_title_source TEXT, cli_created_at INTEGER, cli_updated_at INTEGER,
    cli_last_outcome TEXT, cli_deleted INTEGER DEFAULT 0);
CREATE TABLE task_group_members (group_id TEXT, workspace_key TEXT,
    workspace_path TEXT, workspace_identity TEXT, task_id TEXT, sort_order INTEGER,
    added_at INTEGER, created_at INTEGER, updated_at INTEGER);
CREATE TABLE task_group_admissions (workspace_key TEXT, task_id TEXT,
    system_kind TEXT, user_organized INTEGER, creation_id TEXT);
CREATE TABLE task_group_view_node_orders (node_type TEXT, node_key TEXT,
    sort_order INTEGER, created_at INTEGER, updated_at INTEGER);
CREATE TABLE task_group_workspace_bootstraps (workspace_key TEXT, group_id TEXT,
    created_at INTEGER, updated_at INTEGER);
CREATE TABLE task_groups (group_id TEXT PRIMARY KEY, title TEXT, color TEXT,
    created_at INTEGER, updated_at INTEGER);
CREATE TABLE automations (
    automation_id TEXT PRIMARY KEY, title TEXT, cron_expr TEXT, prompt TEXT,
    model TEXT, provider TEXT, mode TEXT, thought_level TEXT, workspace_key TEXT,
    workspace_path TEXT, workspace_identity TEXT, target_task_id TEXT,
    bot_delivery_target TEXT, location_kind TEXT, recurring INTEGER,
    max_runs INTEGER, end_at INTEGER, schedule_rule TEXT,
    schedule_edited_by_user INTEGER, run_count INTEGER,
    scheduled_run_count INTEGER, enabled INTEGER DEFAULT 1,
    lifecycle_status TEXT, next_run_at INTEGER, last_run_at INTEGER,
    running INTEGER, claimed_at INTEGER, dispatch_status TEXT,
    dispatch_attempts INTEGER, retry_at INTEGER, last_error TEXT,
    created_at INTEGER, updated_at INTEGER, model_selection TEXT,
    project_id TEXT, host_id TEXT, additional_directories TEXT);
CREATE TABLE automation_runs (run_id TEXT PRIMARY KEY, automation_id TEXT,
    workspace_key TEXT, scheduled_at INTEGER, trigger TEXT, dispatch_status TEXT,
    outcome TEXT, session_id TEXT, error TEXT, attempts INTEGER,
    created_at INTEGER, updated_at INTEGER, model_selection TEXT);
CREATE TABLE off_peak_tasks (
    off_peak_task_id TEXT PRIMARY KEY, server_ticket_id TEXT, title TEXT,
    conversation_id TEXT, session_id TEXT, prompt TEXT, permission_mode TEXT,
    model TEXT, thought_level TEXT, workspace_key TEXT, workspace_path TEXT,
    workspace_identity TEXT, status TEXT, queued_at INTEGER, started_at INTEGER,
    ended_at INTEGER, failure_reason TEXT, files_changed TEXT, settled_at INTEGER,
    history_deleted_at INTEGER, registered_at INTEGER, schedulable INTEGER,
    queue_position INTEGER, next_poll_at INTEGER, claim_running INTEGER,
    claimed_at INTEGER, attempt_count INTEGER, last_error TEXT,
    created_at INTEGER, updated_at INTEGER, model_selection TEXT,
    project_id TEXT, host_id TEXT, additional_directories TEXT);
CREATE TABLE hosts (host_id TEXT PRIMARY KEY, kind TEXT, target_json TEXT,
    membership_id TEXT, position INTEGER, last_connection_status TEXT,
    last_connection_error TEXT, last_opened_at INTEGER);
CREATE TABLE projects (project_id TEXT PRIMARY KEY, host_id TEXT, name TEXT,
    name_overridden INTEGER, position INTEGER, pinned INTEGER DEFAULT 0,
    pinned_at INTEGER, deleted INTEGER DEFAULT 0, deleted_at INTEGER,
    created_at INTEGER, updated_at INTEGER);
CREATE TABLE project_roots (project_id TEXT, root_path TEXT, position INTEGER,
    is_primary INTEGER);
CREATE TABLE remote_project_entries (entry_id TEXT PRIMARY KEY, host_id TEXT,
    remote_project_id TEXT, cached_name TEXT, cached_roots_json TEXT,
    legacy_workspace_json TEXT, last_synced_at INTEGER, initial_position INTEGER);
CREATE TABLE tasks_schema_migration (id TEXT PRIMARY KEY, checksum TEXT,
    time_applied INTEGER);
CREATE TABLE schema_migration (id TEXT PRIMARY KEY, checksum TEXT,
    app_version TEXT, applied_at INTEGER);
"""

TI_MIGRATIONS = [
    ("0001_adopt_task_schema", ts(2026, 1, 5)),
    ("0002_provider_selection", ts(2026, 6, 12)),
    ("0003_official_glm_selection", ts(2026, 6, 12)),
    ("0004_project_execution_scopes", NOW - 3 * D),
    ("0006_task_group_admissions", NOW - 3 * D),
]

# Fabricated host/projects: one local host, one project per demo workspace.
PROJECTS = [
    # project_id, host, name, root_path
    ("5e2f0a1c-3b7d-4c8e-9f60-1a2b3c4d5e6f", "local", "default", r"C:\Users\demo\workspace\default"),
    ("8a4c2e9d-6f1b-4d3a-b5c7-2e8f9a0b1c2d", "local", "embedded-fw", r"D:\work\embedded-fw"),
    ("c9d1e3f2-7a4b-4e5c-8d6f-3b0a1c2d3e4f", "local", "report", r"C:\Users\demo\projects\report"),
]


def sid(key):
    return f"sess-demo-{key}-000000000000"


def pick_data_root():
    """Repo-relative by default so the dataset stays with the project; a
    neutral-looking override stays possible for screenshot footers."""
    # repo-relative first: keeps the dataset self-contained with the project
    # (survives relocation of the whole repo; see demo-data/ in .gitignore).
    # For a neutral-looking path in screenshot footers, pass --out explicitly.
    root = os.path.join(BASE, "demo-data", "zcode")
    os.makedirs(os.path.join(root, "cli", "db"), exist_ok=True)
    return root


def message_data(i, lang):
    """Real-shaped message JSON: the app's visibility filter keys off
    semantics.origin (real_user vs agent_runtime) and source."""
    if i % 2 == 0:
        return {
            "role": "user",
            "semantics": {"origin": "real_user"},
            "time": NOW - i * 60_000,
        }
    return {
        "role": "assistant",
        "semantics": {"origin": "agent_runtime"},
        "model": "glm-4.7",
        "modelId": "glm-4.7",
        "providerId": "bigmodel",
        "finish": "stop",
        "time": NOW - i * 60_000,
        "tokens": {"input": 512 + i, "output": 64 + i},
        "tools": [],
    }


PART_TEXT = {
    "zh": ("帮我把这个项目的关键结论整理成要点。", "已完成：以下是要点整理与建议。"),
    "en": ("Summarize the key conclusions of this project.", "Done: here is the summary and suggestions."),
}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang", choices=["zh", "en"], default="zh")
    ap.add_argument("--out", default="")
    args = ap.parse_args()

    data_root = args.out if args.out else pick_data_root()
    cli = os.path.join(data_root, "cli")
    v2 = os.path.join(data_root, "v2")
    # always start clean — the dataset is regenerated wholesale
    shutil.rmtree(data_root, ignore_errors=True)
    os.makedirs(os.path.join(cli, "db"), exist_ok=True)
    os.makedirs(v2, exist_ok=True)

    build_content_db(os.path.join(cli, "db", "db.sqlite"), args.lang)
    build_index_db(os.path.join(v2, "tasks-index.sqlite"), args.lang)

    rng = random.Random(42)
    titles = TITLES[args.lang]
    os.makedirs(os.path.join(cli, "rollout"), exist_ok=True)
    for row in titles:
        key, kb = row[0], row[9]
        with open(os.path.join(cli, "rollout", f"model-io-{sid(key)}.jsonl"), "wb") as f:
            f.write(bytes(rng.getrandbits(8) for _ in range(kb * 1024)))

    disk_demo(cli, rng)
    privacy_demo(cli, v2, args.lang)

    print(f"demo data ({args.lang}) written to {data_root}")


def build_content_db(db, lang):
    con = sqlite3.connect(db)
    con.execute("PRAGMA journal_mode=MEMORY")
    con.executescript(DB_SCHEMA)

    for i, (key, title, project, parent, created, updated, msgs, archived, _p, _kb, deleted) in enumerate(TITLES[lang]):
        s = sid(key)
        trace = f"trc-{key}-0000000000000000000000000{i}"
        con.execute(
            "INSERT INTO session (id, project_id, workspace_id, parent_id, slug, directory,"
            " path, title, version, summary_additions, summary_deletions, summary_files,"
            " permission, time_created, time_updated, time_archived, task_type,"
            " title_source, time_title_updated, trace_id, time_deleted, last_outcome)"
            " VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (s, None, None, sid(parent) if parent else None, key, project, project,
             title, "2", 42 + i, 7, 5, "default", created, updated,
             updated if archived else None,
             "subagent_child" if parent else "interactive",
             "model", updated, trace,
             updated if deleted else None, "completed"),
        )
        for j in range(msgs):
            mid = f"m{j}-{s}"
            t = created + j * 60_000
            con.execute(
                "INSERT INTO message (id, session_id, time_created, time_updated, data, sequence)"
                " VALUES (?,?,?,?,?,?)",
                (mid, s, t, t, json.dumps(message_data(j, lang)), j),
            )
            if j < 6:  # a few visible parts are enough for the viewer demo
                con.execute(
                    "INSERT INTO part (id, message_id, session_id, time_created, time_updated, data, sequence)"
                    " VALUES (?,?,?,?,?,?,?)",
                    (f"p{j}-{s}", mid, s, t, t,
                     json.dumps({"type": "text",
                                 "text": PART_TEXT[lang][j % 2]}), 0),
                )
        # usage/ledger rows for a couple of sessions, in the real shape
        if key in ("perf", "repq", "fwp"):
            con.execute("INSERT INTO todo VALUES (?,?)", (s, "[ ] demo todo"))
            con.execute("INSERT INTO turn_usage VALUES (?,?,?)", (s, f"turn-{key}", 1200 + i))
            con.execute("INSERT INTO model_usage VALUES (?,?,?,?,?)", (f"mu-{key}", s, "glm-4.7", 900 * msgs, 180 * msgs))
            con.execute("INSERT INTO tool_usage VALUES (?,?,?,?)",
                        (f"tu-{key}", s, "Read", created))
            con.execute("INSERT INTO input_history VALUES (?,?,?,?,?,?,?)",
                        (f"ih-{key}", project, s, PART_TEXT[lang][0], "user", created, "[]"))
            con.execute("INSERT INTO session_entry VALUES (?,?,?,?,?,?)",
                        (f"se-{key}", s, "compaction", created, updated, "{}"))

    # parent/child link for the fork pair, in the real table shape
    fwp, fws = sid("fwp"), sid("fws")
    con.execute(
        "INSERT INTO session_task_link (id, parent_session_id, child_session_id, role, depth,"
        " status, time_created, time_updated) VALUES ('link-1',?,?,?,0,'completed',?,?)",
        (fwp, fws, "fork", ts(2026, 1, 12), ts(2026, 1, 20)),
    )
    con.execute(
        "INSERT INTO workflow_definition (id, name, source, trusted, enabled, script_path,"
        " time_created, time_updated, scope) VALUES ('wd-1','demo-flow','project',1,0,"
        "'D:\\work\\embedded-fw\\.zcode/workflows/demo.ts',?,?, 'project')",
        (ts(2025, 12, 1), ts(2025, 12, 1)),
    )
    con.execute("INSERT INTO permission VALUES (?,?,?,?)",
                (r"C:\Users\demo\workspace\default", ts(2026, 8, 20), ts(2026, 8, 20), "{}"))
    con.execute("INSERT INTO local_setting VALUES (?,?,?,?,?,?,?,?)",
                ("session", "global", "demo", "flag", "1", 1, NOW, NOW))
    con.executemany(
        "INSERT INTO schema_migration VALUES (?,?,?,?)",
        [(mid, f"{'a1' * 32}", ver, ts(2026, 1, 5) + k) for k, (mid, ver) in enumerate(DB_MIGRATIONS)],
    )
    con.commit()
    con.close()


def build_index_db(ti, lang):
    con = sqlite3.connect(ti)
    con.execute("PRAGMA journal_mode=MEMORY")
    con.executescript(TI_SCHEMA)

    proj_by_root = {root: pid for pid, _h, _n, root in PROJECTS}
    con.execute("INSERT INTO hosts VALUES ('local','local',NULL,NULL,0,NULL,NULL,?)", (ts(2026, 1, 5),))
    for pos, (pid, _host, name, root) in enumerate(PROJECTS):
        con.execute(
            "INSERT INTO projects VALUES (?,?,?,?,0,0,NULL,0,NULL,?,?)",
            (pid, "local", name, pos, ts(2026, 1, 5), ts(2026, 1, 5)),
        )
        con.execute("INSERT INTO project_roots VALUES (?,?,?,1)", (pid, root, pos))

    for i, (key, title, project, parent, created, updated, _m, archived, pinned, _kb, deleted) in enumerate(TITLES[lang]):
        pid = proj_by_root[project]
        con.execute(
            "INSERT INTO tasks (workspace_key, workspace_path, task_id, title, task_status,"
            " provider, mode, model, created_at, updated_at, pinned, archived, deleted,"
            " searchable_text, host_id, root_path, project_id, sort_order, cli_title,"
            " cli_title_source, cli_created_at, cli_updated_at, cli_deleted)"
            " VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (project, project, sid(key), title, "completed", "bigmodel", "default",
             "glm-4.7", created, updated, pinned, archived, deleted, title,
             "local", project, pid, i, title, "model", created, updated, 0),
        )
        con.execute(
            "INSERT INTO task_group_admissions (workspace_key, task_id, system_kind, user_organized)"
            " VALUES (?,?,NULL,1)",
            (project, sid(key)),
        )

    # one custom group with one member + the view orders, like a real install
    con.execute("INSERT INTO task_groups VALUES ('grp-1',?,NULL,?,?)",
                ("报表专项" if lang == "zh" else "Reports", ts(2025, 9, 20), ts(2025, 9, 20)))
    con.execute("INSERT INTO task_group_members (group_id, workspace_key, task_id, sort_order, created_at, updated_at)"
                " VALUES ('grp-1',?,?,0,?,?)",
                (r"C:\Users\demo\projects\report", sid("repq"), ts(2025, 9, 20), ts(2025, 9, 20)))
    for j, (node, node_key) in enumerate([("project", "grp-1"), ("task", sid("repq")), ("task", sid("week"))]):
        con.execute("INSERT INTO task_group_view_node_orders VALUES (?,?,?,?,?)",
                    (node, node_key, j, NOW - 3 * D, NOW - 3 * D))

    # one enabled cron automation targeting an old archived session, and a run
    auto_title, auto_prompt = (("每周数据汇总提醒", "汇总本周 report 项目的进展并输出摘要。")
                               if lang == "zh"
                               else ("Weekly data digest", "Summarize this week's progress in the report project."))
    con.execute(
        "INSERT INTO automations (automation_id, title, cron_expr, prompt, model, provider,"
        " workspace_key, workspace_path, target_task_id, location_kind, recurring, enabled,"
        " lifecycle_status, created_at, updated_at, host_id)"
        " VALUES ('auto-0001',?, '0 9 * * 1', ?, 'glm-4.7','bigmodel', ?, ?, ?, 'workspace',"
        " 1, 1, 'active', ?, ?, 'local')",
        (auto_title, auto_prompt, r"C:\Users\demo\projects\report",
         r"C:\Users\demo\projects\report", sid("week"), ts(2025, 7, 1), NOW - 2 * D),
    )
    con.execute(
        "INSERT INTO automation_runs (run_id, automation_id, workspace_key, scheduled_at,"
        " trigger, dispatch_status, outcome, session_id, attempts, created_at, updated_at)"
        " VALUES ('run-0001','auto-0001',?, ?, 'cron','dispatched','completed',?,1,?,?)",
        (r"C:\Users\demo\projects\report", NOW - 2 * D, sid("week"), NOW - 2 * D, NOW - 2 * D),
    )

    con.executemany(
        "INSERT INTO tasks_schema_migration VALUES (?,?,?)",
        [(mid, "b2" * 32, t) for mid, t in TI_MIGRATIONS],
    )
    con.executemany(
        "INSERT INTO schema_migration VALUES (?,?,?,?)",
        [("0004_project_execution_scopes", "c3" * 32, "3.16.2", NOW - 3 * D),
         ("0005_task_group_views", "c3" * 32, "3.16.2", NOW - 3 * D),
         ("0006_task_group_admissions", "c3" * 32, "3.16.2", NOW - 3 * D)],
    )
    con.commit()
    con.close()


def disk_demo(cli, rng):
    """Per-session artifact dirs in the real layout: exec/<sid> (+bash-startup),
    artifacts/<sid>, image-cache/<sid>, agents/<sid>."""
    for key, files in [
        ("perf", [("exec", "out.txt", 3 * 1024), ("exec", "trace.json", 12 * 1024),
                  ("artifacts", "plot.svg", 40 * 1024), ("image-cache", "shot.png", 90 * 1024)]),
        ("fwp", [("exec", "plan.md", 2 * 1024), ("artifacts", "schedule.csv", 8 * 1024)]),
        ("fws", [("agents", "agent-1.log", 1 * 1024)]),
        ("draft", [("exec", "notes.txt", 700)]),
    ]:
        for sub, name, size in files:
            d = os.path.join(cli, sub, sid(key))
            os.makedirs(d, exist_ok=True)
            with open(os.path.join(d, name), "wb") as f:
                f.write(bytes(rng.getrandbits(8) for _ in range(size)))
    os.makedirs(os.path.join(cli, "exec", "bash-startup", sid("perf")), exist_ok=True)
    with open(os.path.join(cli, "exec", "bash-startup", sid("perf"), "env.json"), "w") as f:
        f.write("{}\n")


FAKE_MANIFEST_FILES = [
    (".git/HEAD", 21), (".git/config", 401), (".git/description", 73),
    (".git/FETCH_HEAD", 114), (".git/COMMIT_EDITMSG", 256),
    (".git/hooks/applypatch-msg.sample", 478), (".git/hooks/commit-msg.sample", 896),
    (".git/objects/pack/pack-3f2a9c.idx", 53412),
    (".git/objects/pack/pack-3f2a9c.pack", 382104),
    ("README.md", 1834), ("Makefile", 987), ("src/main.c", 15320),
    ("src/driver/uart.c", 9821), ("src/driver/uart.h", 2140),
    ("firmware/boot.bin", 65536), ("tools/flash.py", 2210),
]

FAKE_LOG_LINE = "[{t}] [info] [host] demo log line for screenshots — settings snapshot written, workspace scanned\n"


def _fake_log(path, lines):
    with open(path, "w", encoding="utf-8") as f:
        for i in range(lines):
            f.write(FAKE_LOG_LINE.format(t=f"2026-09-1{6 + i % 3} 2{i % 4}:0{i % 6}:00.000"))


def _fake_sqlite_bytes(rng, size):
    return b"SQLite format 3\x00" + bytes(rng.getrandbits(8) for _ in range(size))


def privacy_demo(cli, v2, lang):
    """Fabricate the non-session privacy data: workspace-snapshot upload
    traces, telemetry ids, logs, crash dumps, agent memories, session
    visualizations, pre-migration backups and the desktop setting file —
    everything the Privacy tab inventories."""
    # two workspaces with repo-snapshot checkpoints (upload evidence)
    for ws, cdir, enc, ws_bytes, failures in [
        (r"D:\work\embedded-fw", "5f3a9c21d7e8", 77_560, 2_264_294, 2),
        (r"C:\Users\demo\projects\report", "a71c40e8b9d2", 389_051, 689_990, None),
    ]:
        cp = os.path.join(v2, "checkpoints", cdir)
        man = os.path.join(cp, "manifests")
        extra = os.path.join(cp, "extra-manifests")
        os.makedirs(man, exist_ok=True)
        os.makedirs(extra, exist_ok=True)
        mh = "c82c618db02167686012de2b59abdda1deae45a9a6fc6766b052b65352f22afb"
        with open(os.path.join(man, mh + ".json"), "w", encoding="utf-8") as f:
            json.dump({
                "schema": "repo_snapshot_manifest/v2",
                "workspaceKey": ws,
                "createdAt": NOW - 5 * D,
                "files": [{"path": p, "sizeBytes": s} for p, s in FAKE_MANIFEST_FILES],
            }, f)
        with open(os.path.join(extra, "3fbd07e9c73740ab98cb2f06c97abca5721b10d24a7fc8e5883206f4b1bbdcdb.json"), "w") as f:
            json.dump({
                "schema": "repo_snapshot_extra_manifest/v1",
                "createdAt": NOW - 5 * D,
                "groups": [{"groupId": "global-configs", "changePolicy": "rare",
                            "files": [{"path": "settings.behavior.json", "sizeBytes": 760,
                                       "source": "app-memory:global-settings"}]}],
                "stats": {"includedFileCount": 1, "includedBytes": 760},
            }, f)
        state = {
            "workspacePath": ws,
            "workspaceKey": ws,
            "lastCompressedSize": {
                "encryptedSizeBytes": enc,
                "workspaceSizeBytes": ws_bytes,
                "manifestHash": mh,
                "recordedAt": NOW - 5 * D,
            },
            "lastAcceptedManifestHash": mh,
            "lastAcceptedManifestPath": os.path.join(man, mh + ".json"),
        }
        if failures is not None:
            state["failureCount"] = failures
        with open(os.path.join(cp, "state.json"), "w") as f:
            json.dump(state, f)

    with open(os.path.join(v2, "telemetry-state.json"), "w") as f:
        json.dump({"deviceMid": "5b7d2f10-9c3e-4a8b-8f2d-6e1c0a4b9d37",
                   "lastDailyActiveDate": "2026-09-18"}, f)

    os.makedirs(os.path.join(v2, "logs"), exist_ok=True)
    _fake_log(os.path.join(v2, "logs", "2026-09-17.log"), 900)
    _fake_log(os.path.join(v2, "logs", "2026-09-18.log"), 600)

    os.makedirs(os.path.join(v2, "crash", "live"), exist_ok=True)
    with open(os.path.join(v2, "crash", "live", "b0f2c1a4.dmp"), "wb") as f:
        f.write(bytes(random.Random(7).getrandbits(8) for _ in range(120 * 1024)))

    os.makedirs(os.path.join(cli, "log"), exist_ok=True)
    _fake_log(os.path.join(cli, "log", "zcode-2026-09-17.jsonl"), 1200)
    _fake_log(os.path.join(cli, "log", "zcode-2026-09-18.jsonl"), 800)

    mem = os.path.join(cli, "memories", "projects", "demo")
    os.makedirs(mem, exist_ok=True)
    with open(os.path.join(cli, "memories", "MEMORY.md"), "w", encoding="utf-8") as f:
        f.write("# Memory Index\n\n- [Demo project](projects/demo/demo.md) — fictional\n")
    with open(os.path.join(mem, "demo.md"), "w", encoding="utf-8") as f:
        f.write("---\nname: demo\ndescription: fictional memory for screenshots\n---\nDemo content.\n")

    # Gen UI visualization fragments (privacy category session_visualizations)
    rng = random.Random(11)
    chart = ("9b0077688759c96e2c3c0c6e00b7fbfc78a5c1e702b26316ef7472f33d727007",
             "<html><body><svg width='320' height='180'><!-- demo chart --></svg></body></html>")
    for h in ("0a039e8968c925ed753fbbbc29f823ca8cb687afaa2cc7417d3c837085e79670",):
        os.makedirs(os.path.join(v2, "visualizations", h), exist_ok=True)  # empty like real
    d = os.path.join(v2, "visualizations", chart[0])
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "uart-baud-tree.html"), "w", encoding="utf-8") as f:
        f.write(chart[1])

    # pre-migration backups (privacy category migration_backups), one set per
    # database like the real 3.16.2 upgrade leaves behind
    for name, migs in [
        ("cli-1792484364119-c292f57f-939e-42dd-86b9-35058ec229fc",
         ["0023_session_soft_delete_outcome", "0024_session_database_identity"]),
        ("tasks_index-1792484363233-c1639138-5edf-4fe8-8aeb-d5cfcac53f93",
         ["0004_project_execution_scopes"]),
        ("tasks_schema-1792484363181-eb153960-0226-4e08-a405-0c56cecb0269", []),
    ]:
        d = os.path.join(os.path.dirname(cli), "backup", name)
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, "tables.sqlite"), "wb") as f:
            f.write(_fake_sqlite_bytes(rng, 24 * 1024))
        with open(os.path.join(d, "backup.json"), "w", encoding="utf-8") as f:
            json.dump({"version": 1, "database": name.split("-")[0], "migrations": migs,
                       "tables": [{"name": "session", "columns": ["id", "title"]}]}, f)

    with open(os.path.join(v2, "setting.json"), "w", encoding="utf-8") as f:
        json.dump({
            "recentProjects": [r"D:\work\embedded-fw", r"C:\Users\demo\projects\report"],
            "lastWorkspaceSession": [
                {"kind": "local", "workspacePath": r"D:\work\embedded-fw", "workspacePurpose": "project"},
            ],
            "locale": "zh-CN" if lang == "zh" else "en-US",
            "optimizeAgentExperienceEnabled": False,
            "repoSnapshotIndexingEnabled": True,
            "instantGrepIndexingEnabled": False,
        }, f)


if __name__ == "__main__":
    main()
