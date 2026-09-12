use std::path::{Path, PathBuf};

/// Resolved locations of everything zsm touches, derived from one root directory.
///
/// Layout (verified against ZCode desktop, 2026-09):
///   <zcode>/cli/db/db.sqlite          session metadata + content
///   <zcode>/v2/tasks-index.sqlite     desktop index (archived/pinned flags)
///   <zcode>/cli/{rollout,exec,artifacts,image-cache,agents}/...  per-session disk data
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Paths {
    pub zcode_dir: PathBuf,
    pub cli_dir: PathBuf,
    pub db_path: PathBuf,
    pub tasks_db: PathBuf,
}

impl Paths {
    pub fn from_zcode_dir(dir: &Path) -> Self {
        Self {
            zcode_dir: dir.to_path_buf(),
            cli_dir: dir.join("cli"),
            db_path: dir.join("cli").join("db").join("db.sqlite"),
            tasks_db: dir.join("v2").join("tasks-index.sqlite"),
        }
    }

    pub fn default_zcode_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".zcode"))
    }

    /// A directory counts as a ZCode data dir if either known database is present.
    pub fn looks_valid(dir: &Path) -> bool {
        dir.join("cli").join("db").join("db.sqlite").is_file()
            || dir.join("v2").join("tasks-index.sqlite").is_file()
    }
}
