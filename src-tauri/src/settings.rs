//! Persisted user settings (JSON in the OS config dir).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Idle threshold bounds for limited mode: 1 minute .. 365 days (in minutes).
pub const IDLE_MIN: u32 = 1;
pub const IDLE_MAX: u32 = 525_600;
const fn default_idle_minutes() -> u32 {
    60
}

/// ZCode process polling interval bounds (seconds).
pub const POLL_MIN: u32 = 1;
pub const POLL_MAX: u32 = 60;
const fn default_poll_seconds() -> u32 {
    4
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Absolute path of the ZCode data directory (the `.zcode` folder).
    /// None = auto-detect from the user home.
    pub zcode_dir: Option<String>,
    /// "zh" | "en"
    pub language: String,
    /// "light" | "dark" | "hc"
    pub theme: String,
    /// Limited-mode idle threshold, stored canonically in minutes.
    #[serde(default = "default_idle_minutes")]
    pub idle_minutes: u32,
    /// UI polling interval for the ZCode process probe, in seconds.
    #[serde(default = "default_poll_seconds")]
    pub poll_seconds: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            zcode_dir: None,
            language: String::new(), // empty = follow system locale
            theme: String::new(),    // empty = follow system preference
            idle_minutes: default_idle_minutes(),
            poll_seconds: default_poll_seconds(),
        }
    }
}

impl Settings {
    pub fn idle_minutes_clamped(&self) -> i64 {
        self.idle_minutes.clamp(IDLE_MIN, IDLE_MAX) as i64
    }

    pub fn poll_seconds_clamped(&self) -> u32 {
        self.poll_seconds.clamp(POLL_MIN, POLL_MAX)
    }
}

pub struct SettingsStore {
    path: PathBuf,
    lock: Mutex<Settings>,
}

impl SettingsStore {
    pub fn new() -> Self {
        let dir = dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("zsm");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("settings.json");
        let settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Self {
            path,
            lock: Mutex::new(settings),
        }
    }

    pub fn load(&self) -> Settings {
        self.lock.lock().unwrap().clone()
    }

    pub fn update(&self, f: impl FnOnce(&mut Settings)) {
        let mut guard = self.lock.lock().unwrap();
        f(&mut guard);
        let _ = std::fs::write(
            &self.path,
            serde_json::to_string_pretty(&*guard).unwrap_or_else(|_| "{}".into()),
        );
    }
}
