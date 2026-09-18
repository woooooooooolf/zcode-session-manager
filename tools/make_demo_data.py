"""Generate a fictional demo ZCode data directory for the README screenshots.

Creates the minimal schema the app expects (db.sqlite: session/message/part;
tasks-index.sqlite: tasks + friends) populated with made-up sessions whose
dates span a long range, plus rollout files for plausible sizes.

Usage:
    python tools/make_demo_data.py [--lang zh|en] [--out DIR]

The data directory itself is created at a fabricated, neutral location so the
in-app path display looks right in screenshots (C:\\Users\\demo\\.zcode when
creatable, otherwise .\\target\\demo-zcode). Nothing touches real ZCode data.
"""

import argparse
import json
import os
import random
import sqlite3
import time

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

NOW = int(time.time() * 1000)
H = 3600 * 1000
D = 24 * H


def ts(y, m, d, hh=10, mm=30):
    import datetime
    return int(datetime.datetime(y, m, d, hh, mm).timestamp() * 1000)


TITLES = {
    "zh": [
        # key, title, project, parent, created, updated, msgs, archived, pinned, kb
        ("perf",   "首页性能优化调研",            r"C:\Users\demo\workspace\default", None,      ts(2026, 8, 20), NOW - 3 * H, 210, 0, 1, 5600),
        ("meet",   "会议纪要要点提取",            r"C:\Users\demo\workspace\default", None,      ts(2026, 8, 26), NOW - 2 * H,  12, 0, 0,   30),
        ("backup", "数据备份方案对比",            r"C:\Users\demo\workspace\default", None,      ts(2026, 6, 15), NOW - 5 * H,  38, 0, 0,  340),
        ("travel", "旅行行程规划助手测试",        r"C:\Users\demo\workspace\default", None,      ts(2026, 7,  2), NOW - 1 * D,  64, 0, 0,  800),
        ("fwp",    "嵌入式固件版本升级排期",      r"D:\work\embedded-fw",             None,      ts(2025, 11, 4), ts(2026, 1, 12), 156, 1, 0, 3900),
        ("fws",    "固件升级排期-子任务拆解",     r"D:\work\embedded-fw",             "fwp",     ts(2026, 1, 12), ts(2026, 1, 20),  31, 0, 1,   80),
        ("repq",   "季度经营分析报告整理",        r"C:\Users\demo\projects\report",   None,      ts(2025, 9, 18), ts(2025, 12, 8), 87, 1, 0, 1400),
        ("repc",   "客户反馈数据清洗脚本",        r"C:\Users\demo\projects\report",   None,      ts(2025, 6, 10), ts(2025, 8, 22), 42, 1, 0,  240),
        ("week",   "周报自动汇总工具",            r"C:\Users\demo\projects\report",   None,      ts(2025, 4, 25), ts(2025, 7, 30), 23, 1, 0,   64),
        ("sq",     "SQLite 学习笔记",             r"C:\Users\demo\workspace\default", None,      ts(2025, 3, 14), ts(2025, 5,  2),  9, 1, 0,   12),
    ],
    "en": [
        ("perf",   "Homepage performance research",   r"C:\Users\demo\workspace\default", None,      ts(2026, 8, 20), NOW - 3 * H, 210, 0, 1, 5600),
        ("meet",   "Meeting notes extraction",        r"C:\Users\demo\workspace\default", None,      ts(2026, 8, 26), NOW - 2 * H,  12, 0, 0,   30),
        ("backup", "Backup strategy comparison",      r"C:\Users\demo\workspace\default", None,      ts(2026, 6, 15), NOW - 5 * H,  38, 0, 0,  340),
        ("travel", "Travel planner assistant test",   r"C:\Users\demo\workspace\default", None,      ts(2026, 7,  2), NOW - 1 * D,  64, 0, 0,  800),
        ("fwp",    "Firmware upgrade scheduling",     r"D:\work\embedded-fw",             None,      ts(2025, 11, 4), ts(2026, 1, 12), 156, 1, 0, 3900),
        ("fws",    "Firmware plan - subtask breakdown", r"D:\work\embedded-fw",           "fwp",     ts(2026, 1, 12), ts(2026, 1, 20),  31, 0, 1,   80),
        ("repq",   "Quarterly business report",       r"C:\Users\demo\projects\report",   None,      ts(2025, 9, 18), ts(2025, 12, 8), 87, 1, 0, 1400),
        ("repc",   "Customer feedback cleanup script", r"C:\Users\demo\projects\report",  None,      ts(2025, 6, 10), ts(2025, 8, 22), 42, 1, 0,  240),
        ("week",   "Weekly report summarizer",        r"C:\Users\demo\projects\report",   None,      ts(2025, 4, 25), ts(2025, 7, 30), 23, 1, 0,   64),
        ("sq",     "SQLite study notes",              r"C:\Users\demo\workspace\default", None,      ts(2025, 3, 14), ts(2025, 5,  2),  9, 1, 0,   12),
    ],
}

DB_SCHEMA = """
CREATE TABLE session (id TEXT PRIMARY KEY, parent_id TEXT, directory TEXT, title TEXT,
    task_type TEXT DEFAULT 'interactive', time_created INTEGER, time_updated INTEGER);
CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT, sequence INTEGER, data TEXT);
CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT, session_id TEXT, sequence INTEGER, data TEXT);
"""

TI_SCHEMA = """
CREATE TABLE tasks (task_id TEXT PRIMARY KEY, title TEXT,
    pinned INTEGER DEFAULT 0, archived INTEGER DEFAULT 0, deleted INTEGER DEFAULT 0,
    updated_at INTEGER DEFAULT 0);
CREATE TABLE task_group_members (id TEXT PRIMARY KEY, task_id TEXT);
CREATE TABLE automation_runs (run_id TEXT PRIMARY KEY, session_id TEXT);
CREATE TABLE off_peak_tasks (off_peak_task_id TEXT PRIMARY KEY, session_id TEXT);
CREATE TABLE automations (automation_id TEXT PRIMARY KEY, target_task_id TEXT, enabled INTEGER DEFAULT 1);
"""


def sid(key):
    return f"sess-demo-{key}-000000000000"


def pick_data_root():
    """Fabricated, neutral-looking locations for the demo data."""
    candidates = [r"C:\Users\demo\.zcode", r"D:\zsm-demo\zcode"]
    for c in candidates:
        try:
            os.makedirs(os.path.join(c, "cli", "db"), exist_ok=True)
            return c
        except OSError:
            continue
    fallback = os.path.join(BASE, "target", "demo-zcode")
    os.makedirs(os.path.join(fallback, "cli", "db"), exist_ok=True)
    return fallback


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang", choices=["zh", "en"], default="zh")
    ap.add_argument("--out", default="")
    args = ap.parse_args()

    cli = os.path.join(args.out, "cli") if args.out else os.path.join(pick_data_root(), "cli")
    v2 = os.path.join(args.out, "v2") if args.out else os.path.join(pick_data_root(), "v2")
    if args.out:
        # an explicit --out target is a throwaway fabrication dir: start clean
        import shutil
        shutil.rmtree(args.out, ignore_errors=True)
    if args.out:
        os.makedirs(os.path.join(cli, "db"), exist_ok=True)
    os.makedirs(v2, exist_ok=True)
    os.makedirs(os.path.join(cli, "rollout"), exist_ok=True)
    data_root = os.path.dirname(cli)

    db = os.path.join(cli, "db", "db.sqlite")
    con = sqlite3.connect(db)
    con.execute("PRAGMA journal_mode=MEMORY")
    con.executescript(DB_SCHEMA)
    for key, title, project, parent, created, updated, msgs, _a, _p, _kb in TITLES[args.lang]:
        s = sid(key)
        con.execute(
            "INSERT INTO session (id, parent_id, directory, title, task_type, time_created, time_updated)"
            " VALUES (?,?,?,?,?,?,?)",
            (s, sid(parent) if parent else None, project, title, "interactive", created, updated),
        )
        rows = [
            (f"m{i}-{s}", s, i, json.dumps({"role": "user" if i % 2 else "assistant"}))
            for i in range(msgs)
        ]
        con.executemany("INSERT INTO message (id, session_id, sequence, data) VALUES (?,?,?,?)", rows)
    con.commit()
    con.close()

    ti = os.path.join(v2, "tasks-index.sqlite")
    con = sqlite3.connect(ti)
    con.execute("PRAGMA journal_mode=MEMORY")
    con.executescript(TI_SCHEMA)
    for key, title, _p, parent, _c, updated, _m, archived, pinned, _kb in TITLES[args.lang]:
        con.execute(
            "INSERT INTO tasks (task_id, title, pinned, archived, deleted, updated_at) VALUES (?,?,?,?,0,?)",
            (sid(key), title, pinned, archived, updated),
        )
    con.commit()
    con.close()

    rng = random.Random(42)
    for row in TITLES[args.lang]:
        key, kb = row[0], row[9]
        with open(os.path.join(cli, "rollout", f"model-io-{sid(key)}.jsonl"), "wb") as f:
            f.write(bytes(rng.getrandbits(8) for _ in range(kb * 1024)))

    privacy_demo(cli, v2, args.lang)

    print(f"demo data ({args.lang}) written to {data_root}")


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


def privacy_demo(cli, v2, lang):
    """Fabricate the non-session privacy data: workspace-snapshot upload
    traces, telemetry ids, logs, crash dumps, agent memories and the
    desktop setting file — everything the Privacy tab inventories."""
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
