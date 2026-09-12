//! SQLite integrity verification.

use crate::util::open_ro;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStatus {
    /// "ok" | "corrupt" | "unavailable"
    pub state: String,
    pub detail: String,
}

impl DbStatus {
    pub fn ok() -> Self {
        Self {
            state: "ok".into(),
            detail: String::new(),
        }
    }
    pub fn is_ok(&self) -> bool {
        self.state == "ok"
    }
}

pub fn check_db(path: &Path) -> DbStatus {
    if !path.is_file() {
        return DbStatus {
            state: "unavailable".into(),
            detail: "file not found".into(),
        };
    }
    match open_ro(path) {
        Err(e) => DbStatus {
            state: "unavailable".into(),
            detail: e.to_string(),
        },
        Ok(con) => match con.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0)) {
            Ok(s) if s == "ok" => DbStatus::ok(),
            Ok(s) => DbStatus {
                state: "corrupt".into(),
                detail: s,
            },
            Err(e) => DbStatus {
                state: "corrupt".into(),
                detail: e.to_string(),
            },
        },
    }
}
