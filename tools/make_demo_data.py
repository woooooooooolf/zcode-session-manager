"""Generate a fictional demo ZCode data directory for the README screenshot.

Creates target/demo-zcode/ with the minimal schema the app expects
(db.sqlite: session/message/part; tasks-index.sqlite: tasks + friends)
populated with made-up sessions, plus rollout files for plausible sizes.
Nothing here touches real ZCode data.
"""

import json
import os
import random
import sqlite3
import time

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(BASE, "target", "demo-zcode")
CLI = os.path.join(OUT, "cli")
V2 = os.path.join(OUT, "v2")

NOW = int(time.time() * 1000)
H = 3600 * 1000
D = 24 * H

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

# key, title, project, parent_key, created, updated, msgs, archived, pinned, rollout_kb
SESSIONS = [
    ("perf",   "首页性能优化调研",           r"C:\Users\demo\workspace\default",  None,      NOW - 6 * D,  NOW - 3 * H, 210, 0, 1, 5600),
    ("meet",   "会议纪要要点提取",           r"C:\Users\demo\workspace\default",  None,      NOW - 2 * D,  NOW - 2 * H,  12, 0, 0,   30),
    ("backup", "数据备份方案对比",           r"C:\Users\demo\workspace\default",  None,      NOW - 4 * D,  NOW - 5 * H,  38, 0, 0,  340),
    ("travel", "旅行行程规划助手测试",       r"C:\Users\demo\workspace\default",  None,      NOW - 9 * D,  NOW - 1 * D,  64, 0, 0,  800),
    ("repq",   "季度经营分析报告整理",       r"C:\Users\demo\projects\report",    None,      NOW - 45 * D, NOW - 30 * D, 87, 1, 0, 1400),
    ("repc",   "客户反馈数据清洗脚本",       r"C:\Users\demo\projects\report",    None,      NOW - 40 * D, NOW - 28 * D, 42, 1, 0,  240),
    ("fwp",    "嵌入式固件版本升级排期",     r"D:\work\embedded-fw",              None,      NOW - 38 * D, NOW - 20 * D, 156, 1, 0, 3900),
    ("fws",    "固件升级排期-子任务拆解",    r"D:\work\embedded-fw",              "fwp",     NOW - 20 * D, NOW - 19 * D, 31, 0, 0,   80),
    ("week",   "周报自动汇总工具",           r"C:\Users\demo\projects\report",    None,      NOW - 33 * D, NOW - 26 * D, 23, 1, 0,   64),
    ("sq",     "SQLite 学习笔记",            r"C:\Users\demo\workspace\default",  None,      NOW - 60 * D, NOW - 50 * D,  9, 1, 0,   12),
]


def sid(key):
    return f"sess-demo-{key}-000000000000"


def main():
    for d in (os.path.join(CLI, "db"), V2, os.path.join(CLI, "rollout")):
        os.makedirs(d, exist_ok=True)

    db = os.path.join(CLI, "db", "db.sqlite")
    con = sqlite3.connect(db)
    con.execute("PRAGMA journal_mode=MEMORY")
    con.executescript(DB_SCHEMA)
    for key, title, project, parent, created, updated, msgs, _a, _p, _kb in SESSIONS:
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

    ti = os.path.join(V2, "tasks-index.sqlite")
    con = sqlite3.connect(ti)
    con.execute("PRAGMA journal_mode=MEMORY")
    con.executescript(TI_SCHEMA)
    for key, title, _p, parent, _c, updated, _m, archived, pinned, _kb in SESSIONS:
        con.execute(
            "INSERT INTO tasks (task_id, title, pinned, archived, deleted, updated_at) VALUES (?,?,?,?,0,?)",
            (sid(key), title, pinned, archived, updated),
        )
    con.commit()
    con.close()

    rng = random.Random(42)
    for key, *_rest, kb in [(s[0], s[9]) for s in SESSIONS]:
        with open(os.path.join(CLI, "rollout", f"model-io-{sid(key)}.jsonl"), "wb") as f:
            f.write(bytes(rng.getrandbits(8) for _ in range(kb * 1024)))

    print("demo data written to", OUT)


if __name__ == "__main__":
    main()
