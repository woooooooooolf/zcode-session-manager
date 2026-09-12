//! Simple-copy backups with timestamped directory names.
//! Everything needed to undo a deletion lives in one backup dir:
//!   <zcode>/zsm-backups/<timestamp>/
//!     db.sqlite / tasks-index.sqlite (+ -wal if it existed)
//!     disk/<path relative to cli dir>       per-session disk artifacts
//!     manifest.json                          what was backed up and where it came from

use crate::disk;
use crate::paths::Paths;
use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn backups_base(paths: &Paths) -> PathBuf {
    paths.zcode_dir.join("zsm-backups")
}

pub fn new_backup_dir(paths: &Paths) -> io::Result<PathBuf> {
    let base = backups_base(paths);
    fs::create_dir_all(&base)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut dir = base.join(&stamp);
    let mut n = 1;
    while dir.exists() {
        dir = base.join(format!("{stamp}-{n}"));
        n += 1;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn sidecar(p: &Path, suffix: &str) -> PathBuf {
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    p.with_file_name(format!("{name}{suffix}"))
}

/// Copy a sqlite database (best-effort WAL checkpoint first) plus its -wal sidecar.
pub fn copy_database(src: &Path, dest_dir: &Path) -> io::Result<()> {
    if !src.is_file() {
        return Ok(());
    }
    if let Ok(con) = Connection::open(src) {
        let _ = con.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
    }
    let name = src.file_name().expect("db file has a name").to_owned();
    fs::copy(src, dest_dir.join(&name))?;
    let wal = sidecar(src, "-wal");
    if wal.is_file() {
        fs::copy(&wal, dest_dir.join(sidecar(&PathBuf::from(&name), "-wal").file_name().unwrap()))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskRecord {
    /// absolute original path
    pub source: String,
    /// path inside the backup dir, relative
    pub backup_rel: String,
    pub is_dir: bool,
}

/// Copy every disk artifact of the given sessions into `<backup>/disk/<rel-to-cli>`.
pub fn copy_session_disk(
    cli_dir: &Path,
    session_ids: &[String],
    backup_dir: &Path,
) -> io::Result<Vec<DiskRecord>> {
    let mut records = Vec::new();
    let disk_root = backup_dir.join("disk");
    for sid in session_ids {
        for target in disk::targets(cli_dir, sid) {
            if !target.exists() {
                continue;
            }
            let rel = target
                .strip_prefix(cli_dir)
                .unwrap_or(&target)
                .to_string_lossy()
                .to_string();
            let dest = disk_root.join(&rel);
            if target.is_dir() {
                copy_dir_recursive(&target, &dest)?;
            } else {
                if let Some(p) = dest.parent() {
                    fs::create_dir_all(p)?;
                }
                fs::copy(&target, &dest)?;
            }
            records.push(DiskRecord {
                source: target.to_string_lossy().to_string(),
                backup_rel: format!("disk/{rel}"),
                is_dir: target.is_dir(),
            });
        }
    }
    Ok(records)
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

pub fn write_manifest(backup_dir: &Path, manifest: &serde_json::Value) -> io::Result<()> {
    fs::write(
        backup_dir.join("manifest.json"),
        serde_json::to_string_pretty(manifest).unwrap_or_else(|_| "{}".into()),
    )
}

pub fn read_manifest(backup_dir: &Path) -> Option<serde_json::Value> {
    let text = fs::read_to_string(backup_dir.join("manifest.json")).ok()?;
    serde_json::from_str(&text).ok()
}

/// Undo a deletion: copy databases and disk artifacts back from a backup dir.
/// Returns a human-readable action log. Fails hard (Err) only on io errors.
pub fn restore_from_backup(
    backup_dir: &Path,
    paths: &Paths,
    records: &[DiskRecord],
) -> crate::Result<Vec<String>> {
    let mut log = Vec::new();

    for name in ["db.sqlite", "tasks-index.sqlite"] {
        let backup_file = backup_dir.join(name);
        let live = if name == "db.sqlite" {
            &paths.db_path
        } else {
            &paths.tasks_db
        };
        if !backup_file.is_file() {
            log.push(format!("backup copy of {name} missing — skipped"));
            continue;
        }
        for suffix in ["-wal", "-shm"] {
            let sc = sidecar(live, suffix);
            if sc.exists() {
                fs::remove_file(&sc)?;
                log.push(format!("removed stale sidecar {}", sc.display()));
            }
        }
        fs::copy(&backup_file, live)?;
        log.push(format!("restored {}", live.display()));
        let backup_wal = backup_dir.join(format!("{name}-wal"));
        if backup_wal.is_file() {
            fs::copy(&backup_wal, sidecar(live, "-wal"))?;
            log.push("restored -wal sidecar".into());
        }
    }

    for r in records {
        let src = backup_dir.join(&r.backup_rel);
        let dst = PathBuf::from(&r.source);
        if !src.exists() {
            continue;
        }
        if r.is_dir {
            copy_dir_recursive(&src, &dst)?;
        } else {
            if let Some(p) = dst.parent() {
                fs::create_dir_all(p)?;
            }
            fs::copy(&src, &dst)?;
        }
        log.push(format!("restored {}", dst.display()));
    }

    Ok(log)
}
