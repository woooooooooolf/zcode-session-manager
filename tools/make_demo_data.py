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
        ("fws",    "固件升级排期-子任务拆解",     r"D:\work\embedded-fw",             "fwp",     ts(2026, 1, 12), ts(2026, 1, 20),  31, 0, 0,   80),
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
        ("fws",    "Firmware plan - subtask breakdown", r"D:\work\embedded-fw",           "fwp",     ts(2026, 1, 12), ts(2026, 1, 20),  31, 0, 0,   80),
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

    print(f"demo data ({args.lang}) written to {data_root}")


if __name__ == "__main__":
    main()
