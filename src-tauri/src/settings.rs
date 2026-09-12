//! Persisted user settings (JSON in the OS config dir).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            zcode_dir: None,
            language: String::new(), // empty = follow system locale
            theme: String::new(),    // empty = follow system preference
        }
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
