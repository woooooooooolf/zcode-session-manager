//! Tauri backend: commands exposed to the UI + persisted settings.

mod settings;

use serde::Serialize;
use settings::{Settings, SettingsStore};
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use zsm_core::{CompatReport, DeletePlan, DeleteResult, Paths, SessionDetail, SessionSummary, Store};

#[derive(Clone)]
pub struct AppMgr {
    inner: Arc<AppInner>,
}

struct AppInner {
    settings: SettingsStore,
}

impl AppMgr {
    fn settings(&self) -> Settings {
        self.inner.settings.load()
    }

    fn store(&self) -> Result<Store, ApiError> {
        let dir = self
            .settings()
            .zcode_dir
            .or_else(|| zsm_core::Paths::default_zcode_dir().map(|p| p.to_string_lossy().to_string()))
            .ok_or_else(|| ApiError::simple("no_dir", "could not determine the ZCode data directory"))?;
        let dir = std::path::PathBuf::from(&dir);
        if !Paths::looks_valid(&dir) {
            return Err(ApiError::simple(
                "invalid_dir",
                "the configured directory does not look like a ZCode data directory",
            ));
        }
        Ok(Store::new(Paths::from_zcode_dir(&dir)))
    }
}

// ---------------- error payload ----------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// stable code the frontend maps to a localized message
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problems: Option<Vec<String>>,
}

impl ApiError {
    pub fn simple(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            problems: None,
        }
    }
}

impl From<zsm_core::Error> for ApiError {
    fn from(e: zsm_core::Error) -> Self {
        let (code, problems) = match &e {
            zsm_core::Error::ZcodeRunning => ("zcode_running", None),
            zsm_core::Error::Compat { problems } => ("compat", Some(problems.clone())),
            zsm_core::Error::Corruption { details } => ("corruption", Some(details.clone())),
            zsm_core::Error::DbNotFound(_) => ("db_not_found", None),
            zsm_core::Error::Io(_) => ("io", None),
            zsm_core::Error::Sql(_) => ("sqlite", None),
            zsm_core::Error::Json(_) => ("json", None),
            zsm_core::Error::Other(_) => ("other", None),
        };
        Self {
            code: code.into(),
            message: e.to_string(),
            problems,
        }
    }
}

// ---------------- shared output types ----------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStatusOut {
    pub db: String,
    pub state: String,
    pub detail: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateOut {
    pub settings: Settings,
    pub detected_default: Option<String>,
    pub db_path: Option<String>,
    pub cli_dir: Option<String>,
    pub backups_dir: Option<String>,
    pub compat: Option<CompatReport>,
    pub zcode_running: bool,
    pub integrity: Vec<DbStatusOut>,
    pub app_version: String,
}

fn build_state(mgr: &AppMgr) -> AppStateOut {
    let settings = mgr.settings();
    let detected = Paths::default_zcode_dir();
    let dir = settings
        .zcode_dir
        .clone()
        .map(std::path::PathBuf::from)
        .or_else(|| detected.clone());
    let mut out = AppStateOut {
        settings,
        detected_default: detected.map(|p| p.to_string_lossy().to_string()),
        db_path: None,
        cli_dir: None,
        backups_dir: None,
        compat: None,
        zcode_running: zsm_core::zcode_running(),
        integrity: vec![],
        app_version: env!("CARGO_PKG_VERSION").into(),
    };
    if let Some(dir) = dir {
        if Paths::looks_valid(&dir) {
            let paths = Paths::from_zcode_dir(&dir);
            out.db_path = Some(paths.db_path.to_string_lossy().to_string());
            out.cli_dir = Some(paths.cli_dir.to_string_lossy().to_string());
            out.backups_dir = Some(zsm_core::backup::backups_base(&paths).to_string_lossy().to_string());
            out.compat = Some(zsm_core::compat::check(&paths));
            out.integrity = vec![
                DbStatusOut {
                    db: "db.sqlite".into(),
                    state: integrity_state(&paths.db_path),
                    detail: String::new(),
                },
                DbStatusOut {
                    db: "tasks-index.sqlite".into(),
                    state: integrity_state(&paths.tasks_db),
                    detail: String::new(),
                },
            ];
        }
    }
    out
}

fn integrity_state(path: &std::path::Path) -> String {
    zsm_core::integrity::check_db(path).state
}

// ---------------- commands ----------------

#[tauri::command]
async fn app_state(mgr: State<'_, AppMgr>) -> Result<AppStateOut, ApiError> {
    let mgr = mgr.inner().clone();
    tauri::async_runtime::spawn_blocking(move || Ok(build_state(&mgr)))
        .await
        .map_err(|e| ApiError::simple("join", &e.to_string()))?
}

#[tauri::command]
async fn set_zcode_dir(path: String, mgr: State<'_, AppMgr>) -> Result<AppStateOut, ApiError> {
    let dir = std::path::PathBuf::from(&path);
    if !Paths::looks_valid(&dir) {
        return Err(ApiError::simple(
            "invalid_dir",
            "the directory does not look like a ZCode data directory (need cli/db/db.sqlite or v2/tasks-index.sqlite)",
        ));
    }
    let mgr = mgr.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        mgr.inner.settings.update(|s| {
            s.zcode_dir = Some(path.clone());
        });
        Ok(build_state(&mgr))
    })
    .await
    .map_err(|e| ApiError::simple("join", &e.to_string()))?
}

#[tauri::command]
async fn detect_zcode_dir(mgr: State<'_, AppMgr>) -> Result<AppStateOut, ApiError> {
    let mgr = mgr.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let detected = Paths::default_zcode_dir();
        match detected {
            Some(d) if Paths::looks_valid(&d) => {
                let val = d.to_string_lossy().to_string();
                mgr.inner.settings.update(|s| s.zcode_dir = Some(val));
            }
            _ => {
                return Err(ApiError::simple(
                    "detect_failed",
                    "could not find a ZCode data directory under the user home",
                ));
            }
        }
        Ok(build_state(&mgr))
    })
    .await
    .map_err(|e| ApiError::simple("join", &e.to_string()))?
}

#[tauri::command]
fn set_prefs(
    language: Option<String>,
    theme: Option<String>,
    mgr: State<AppMgr>,
) -> Settings {
    mgr.inner.settings.update(|s| {
        if let Some(l) = language {
            s.language = l;
        }
        if let Some(t) = theme {
            s.theme = t;
        }
    });
    mgr.settings()
}

#[tauri::command]
async fn scan(mgr: State<'_, AppMgr>) -> Result<AppStateOut, ApiError> {
    let mgr = mgr.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        // rebuild state (compat + integrity) together with the session list
        Ok(build_state(&mgr))
    })
    .await
    .map_err(|e| ApiError::simple("join", &e.to_string()))?
}

#[tauri::command]
async fn list_sessions(mgr: State<'_, AppMgr>) -> Result<Vec<SessionSummary>, ApiError> {
    let store = mgr.store()?;
    tauri::async_runtime::spawn_blocking(move || store.list_sessions().map_err(ApiError::from))
        .await
        .map_err(|e| ApiError::simple("join", &e.to_string()))?
}

#[tauri::command]
async fn session_detail(id: String, mgr: State<'_, AppMgr>) -> Result<Option<SessionDetail>, ApiError> {
    let store = mgr.store()?;
    tauri::async_runtime::spawn_blocking(move || {
        store.session_detail(&id).map_err(ApiError::from)
    })
    .await
    .map_err(|e| ApiError::simple("join", &e.to_string()))?
}

#[tauri::command]
async fn delete_plan(ids: Vec<String>, mgr: State<'_, AppMgr>) -> Result<DeletePlan, ApiError> {
    let store = mgr.store()?;
    tauri::async_runtime::spawn_blocking(move || {
        store.plan_delete(&ids).map_err(ApiError::from)
    })
    .await
    .map_err(|e| ApiError::simple("join", &e.to_string()))?
}

#[tauri::command]
async fn delete_execute(ids: Vec<String>, mgr: State<'_, AppMgr>) -> Result<DeleteResult, ApiError> {
    let store = mgr.store()?;
    tauri::async_runtime::spawn_blocking(move || {
        store.execute_delete(&ids).map_err(ApiError::from)
    })
    .await
    .map_err(|e| ApiError::simple("join", &e.to_string()))?
}

#[tauri::command]
async fn pick_folder(app: AppHandle) -> Result<Option<String>, ApiError> {
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app.dialog().file().blocking_pick_folder();
        Ok(picked.map(|p| p.to_string()))
    })
    .await
    .map_err(|e| ApiError::simple("join", &e.to_string()))?
}

#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppMgr {
            inner: Arc::new(AppInner {
                settings: SettingsStore::new(),
            }),
        })
        .invoke_handler(tauri::generate_handler![
            app_state,
            set_zcode_dir,
            detect_zcode_dir,
            set_prefs,
            scan,
            list_sessions,
            session_detail,
            delete_plan,
            delete_execute,
            pick_folder,
            reveal_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
