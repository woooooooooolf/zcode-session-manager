use std::path::{Path, PathBuf};

/// Per-session disk artifacts under the cli dir (verified layout).
pub fn targets(cli_dir: &Path, sid: &str) -> Vec<PathBuf> {
    vec![
        cli_dir.join("agents").join(sid),
        cli_dir.join("artifacts").join(sid),
        cli_dir.join("exec").join(sid),
        cli_dir.join("exec").join("bash-startup").join(sid),
        cli_dir.join("image-cache").join(sid),
        cli_dir.join("rollout").join(format!("model-io-{sid}.jsonl")),
    ]
}

pub fn usage(cli_dir: &Path, sid: &str) -> u64 {
    targets(cli_dir, sid).iter().map(|p| size_of(p)).sum()
}

pub fn size_of(path: &Path) -> u64 {
    if path.is_dir() {
        walk(path).0
    } else if path.is_file() {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    }
}

/// Returns (bytes, files).
pub fn walk(path: &Path) -> (u64, u64) {
    let mut bytes = 0u64;
    let mut files = 0u64;
    if let Ok(rd) = std::fs::read_dir(path) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let (b, f) = walk(&p);
                bytes += b;
                files += f;
            } else if let Ok(md) = entry.metadata() {
                bytes += md.len();
                files += 1;
            }
        }
    }
    (bytes, files)
}

/// Delete the given paths. Returns (files_removed, bytes_freed, errors).
pub fn remove(targets: &[PathBuf]) -> (u64, u64, Vec<String>) {
    let mut files_removed = 0u64;
    let mut bytes = 0u64;
    let mut errors = Vec::new();
    for t in targets {
        if !t.exists() {
            continue;
        }
        let (b, f) = walk(t);
        let res = if t.is_dir() {
            std::fs::remove_dir_all(t)
        } else {
            std::fs::remove_file(t)
        };
        match res {
            Ok(()) => {
                bytes += b;
                files_removed += f.max(1);
            }
            Err(e) => errors.push(format!("{}: {e}", t.display())),
        }
    }
    (files_removed, bytes, errors)
}
