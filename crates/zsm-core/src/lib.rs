//! zsm-core: ZCode session storage access, compatibility checks, backups,
//! cascade deletion and integrity verification. UI-agnostic.

pub mod backup;
pub mod compat;
pub mod disk;
pub mod error;
pub mod integrity;
pub mod paths;
pub mod store;
#[cfg(test)]
mod tests;
pub mod util;

pub use error::{Error, Result};
pub use paths::Paths;
pub use store::{DeletePlan, DeleteResult, SessionDetail, SessionSummary, Store};

use std::sync::atomic::{AtomicBool, Ordering};

static ZCODE_CHECK_DISABLED: AtomicBool = AtomicBool::new(false);

/// Test hook: unit tests run on machines where ZCode may well be open.
pub fn set_zcode_check_disabled(v: bool) {
    ZCODE_CHECK_DISABLED.store(v, Ordering::SeqCst);
}

pub fn zcode_check_disabled() -> bool {
    ZCODE_CHECK_DISABLED.load(Ordering::SeqCst)
}

/// Is any ZCode process running? (name contains "zcode", our own process excluded)
pub fn zcode_running() -> bool {
    use sysinfo::System;
    let sys = System::new_all();
    let self_pid = std::process::id();
    sys.processes().iter().any(|(pid, p)| {
        pid.as_u32() != self_pid && p.name().to_lowercase().contains("zcode")
    })
}
