//! zsm-core: ZCode session storage access, compatibility checks, backups,
//! cascade deletion and integrity verification. UI-agnostic.

pub mod backup;
pub mod compat;
pub mod disk;
pub mod error;
pub mod integrity;
pub mod paths;
pub mod privacy;
pub mod store;
#[cfg(test)]
mod tests;
pub mod util;

pub use compat::CompatReport;
pub use error::{Error, Result};
pub use paths::Paths;
pub use store::{DeletePlan, DeleteResult, RunningPolicy, SessionDetail, SessionSummary, Store};

use std::sync::atomic::{AtomicI8, Ordering};

static ZCODE_RUNNING_OVERRIDE: AtomicI8 = AtomicI8::new(-1);

/// Test hook: `None` probes the real system, `Some(_)` forces the answer.
pub fn set_zcode_running_override(v: Option<bool>) {
    ZCODE_RUNNING_OVERRIDE.store(
        match v {
            None => -1,
            Some(false) => 0,
            Some(true) => 1,
        },
        Ordering::SeqCst,
    );
}

/// Is any ZCode process running? (name contains "zcode", our own process excluded)
pub fn zcode_running() -> bool {
    match ZCODE_RUNNING_OVERRIDE.load(Ordering::SeqCst) {
        0 => return false,
        1 => return true,
        _ => {}
    }
    use sysinfo::System;
    let sys = System::new_all();
    let self_pid = std::process::id();
    sys.processes().iter().any(|(pid, p)| {
        pid.as_u32() != self_pid
            && p.name().to_string_lossy().to_lowercase().contains("zcode")
    })
}
