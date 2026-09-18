//! Privacy cleaner: inventory and removal of the user data ZCode keeps
//! outside the session databases — workspace repo-snapshot checkpoints,
//! telemetry device state, desktop/CLI logs, crash dumps, the embedded
//! Chromium profile and the updater cache.
//!
//! Layout verified against a real install on 2026-09-18:
//!   <zcode>/v2/checkpoints/<hash>/          repo-snapshot manifests + upload state
//!   <zcode>/v2/telemetry-state.json         telemetry device id (deviceMid)
//!   <zcode>/v2/logs/                        desktop logs (contain full settings dumps)
//!   <zcode>/v2/crash/                       crash dumps
//!   <zcode>/cli/log/                        CLI jsonl logs
//!   <zcode>/cli/memories/                   agent long-term memory about the user
//!   %APPDATA%/ZCode/session/                embedded Chromium profile (cookies, caches)
//!   %APPDATA%/ZCode/zcode-data-size-telemetry.json
//!   %APPDATA%/ZCode/rum-electron-store/     real-user-monitoring store
//!   %LOCALAPPDATA%/@zcodedesktop-updater/   downloaded installers
//!   <zcode>/v2/setting.json                 recentProjects / lastWorkspaceSession lists
//!
//! Never touched: credentials.json, v2/config.json (API keys — not this
//! tool's business to rotate), the session databases (see `store`), and
//! zsm's own backup directory.

use crate::disk;
use crate::{Error, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Where every privacy-relevant location lives. The two desktop-app paths
/// are user-profile fixed and independent of the configured zcode dir.
#[derive(Debug, Clone)]
pub struct PrivacyPaths {
    pub zcode: Option<PathBuf>,
    pub roaming_zcode: Option<PathBuf>,
    pub local_updater: Option<PathBuf>,
}

impl PrivacyPaths {
    /// `zcode_dir` is only trusted after passing the same structural check
    /// the session store uses — a wrong/custom folder must never expose
    /// `v2/*` or `cli/*` paths for cleaning (宁缺毋滥: rather clean nothing
    /// than delete from a directory we are not sure is ZCode's).
    pub fn discover(zcode_dir: Option<&Path>) -> Self {
        let zcode = zcode_dir
            .filter(|d| crate::paths::Paths::looks_valid(d))
            .map(|p| p.to_path_buf());
        Self {
            zcode,
            roaming_zcode: dirs::config_dir().map(|d| d.join("ZCode")),
            local_updater: dirs::data_local_dir().map(|d| d.join("@zcodedesktop-updater")),
        }
    }

    fn setting_json(&self) -> Option<PathBuf> {
        self.zcode.as_ref().map(|z| z.join("v2").join("setting.json"))
    }
}

/// One cleanable category. Titles/descriptions are i18n keys resolved by the
/// frontend (`privacy.cat.<id>` / `privacy.cat.<id>.desc`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyEntry {
    pub id: &'static str,
    pub risk: &'static str, // "high" | "medium" | "low"
    pub present: bool,
    pub bytes: u64,
    pub files: u64,
    /// Display-only sample of the concrete locations involved.
    pub paths: Vec<String>,
}

/// Display order. `recent_projects` edits JSON fields instead of deleting
/// files, so it is absent from `entry_targets`.
pub const CATEGORY_IDS: &[&str] = &[
    "repo_snapshots",
    "telemetry_state",
    "desktop_logs",
    "cli_logs",
    "crash_reports",
    "browser_profile",
    "recent_projects",
    "updater_cache",
    "agent_memories",
];

/// Current values of the upload/indexing toggles in `v2/setting.json`.
/// `None` = file missing or key absent (ZCode then applies its defaults).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacySwitches {
    pub setting_path: Option<String>,
    pub optimize_agent_experience: Option<bool>,
    pub repo_snapshot_indexing: Option<bool>,
    pub instant_grep_indexing: Option<bool>,
}

fn entry_targets(p: &PrivacyPaths, id: &str) -> Vec<PathBuf> {
    let z = |rel: &[&str]| -> Option<PathBuf> {
        p.zcode.as_ref().map(|z| z.join(rel.iter().collect::<PathBuf>()))
    };
    let r = |rel: &[&str]| -> Option<PathBuf> {
        p.roaming_zcode.as_ref().map(|z| z.join(rel.iter().collect::<PathBuf>()))
    };
    match id {
        "repo_snapshots" => vec![z(&["v2", "checkpoints"])],
        "telemetry_state" => vec![
            z(&["v2", "telemetry-state.json"]),
            r(&["zcode-data-size-telemetry.json"]),
            r(&["rum-electron-store"]),
        ],
        "desktop_logs" => vec![z(&["v2", "logs"])],
        "cli_logs" => vec![z(&["cli", "log"])],
        "crash_reports" => vec![z(&["v2", "crash"])],
        "browser_profile" => vec![r(&["session"])],
        "updater_cache" => vec![p.local_updater.clone()],
        "agent_memories" => vec![z(&["cli", "memories"])],
        _ => vec![],
    }
    .into_iter()
    .flatten()
    .collect()
}

fn measure(targets: &[PathBuf]) -> (u64, u64, bool) {
    let mut bytes = 0u64;
    let mut files = 0u64;
    let mut present = false;
    for t in targets {
        if t.is_dir() {
            let (b, f) = disk::walk(t);
            bytes += b;
            files += f;
            present = true;
        } else if t.is_file() {
            bytes += t.metadata().map(|m| m.len()).unwrap_or(0);
            files += 1;
            present = true;
        }
    }
    (bytes, files, present)
}

/// Inventory all categories. Missing locations report as absent; nothing
/// errors out so the UI can always render a full list.
pub fn scan(p: &PrivacyPaths) -> Vec<PrivacyEntry> {
    CATEGORY_IDS
        .iter()
        .map(|id| {
            if *id == "recent_projects" {
                scan_recent_projects(p)
            } else {
                let targets = entry_targets(p, id);
                let (bytes, files, present) = measure(&targets);
                PrivacyEntry {
                    id,
                    risk: risk_of(id),
                    present,
                    bytes,
                    files,
                    paths: targets.iter().map(|t| t.to_string_lossy().to_string()).collect(),
                }
            }
        })
        .collect()
}

fn risk_of(id: &str) -> &'static str {
    match id {
        "repo_snapshots" | "browser_profile" | "agent_memories" => "high",
        "updater_cache" => "low",
        _ => "medium",
    }
}

fn scan_recent_projects(p: &PrivacyPaths) -> PrivacyEntry {
    let mut entry = PrivacyEntry {
        id: "recent_projects",
        risk: "medium",
        present: false,
        bytes: 0,
        files: 0,
        paths: vec![],
    };
    if let Some(path) = p.setting_json() {
        entry.paths.push(path.to_string_lossy().to_string());
        if path.is_file() {
            entry.bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
            entry.files = 1;
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(
                &std::fs::read_to_string(&path).unwrap_or_default(),
            ) {
                let has = ["recentProjects", "lastWorkspaceSession"].iter().any(|k| {
                    v.get(k).and_then(|a| a.as_array()).map(|a| !a.is_empty()).unwrap_or(false)
                });
                entry.present = has;
            }
        }
    }
    entry
}

// ---------------- cleaning ----------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanOutcome {
    pub id: String,
    pub ok: bool,
    pub bytes: u64,
    pub files: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanReport {
    pub outcomes: Vec<CleanOutcome>,
    pub total_bytes: u64,
    pub total_files: u64,
}

/// Remove the selected categories. Refuses to run while ZCode is open —
/// deleting caches under a live Electron app corrupts its state. Unknown
/// category ids are ignored (whitelist), never interpreted as paths.
pub fn clean(p: &PrivacyPaths, ids: &[String]) -> Result<CleanReport> {
    if crate::zcode_running() {
        return Err(Error::ZcodeRunning);
    }
    let mut report = CleanReport { outcomes: vec![], total_bytes: 0, total_files: 0 };
    for id in ids.iter().filter(|id| CATEGORY_IDS.contains(&id.as_str())) {
        let outcome = if id == "recent_projects" {
            clean_recent_projects(p)
        } else {
            let targets = entry_targets(p, id);
            let (bytes, files, _) = measure(&targets);
            let mut errors = vec![];
            for t in &targets {
                if t.is_dir() {
                    if let Ok(rd) = std::fs::read_dir(t) {
                        let children: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                        let (_, _, errs) = disk::remove(&children);
                        errors.extend(errs);
                    }
                } else if t.is_file() {
                    if let Err(e) = std::fs::remove_file(t) {
                        errors.push(format!("{}: {e}", t.display()));
                    }
                }
            }
            CleanOutcome { id: id.clone(), ok: errors.is_empty(), bytes, files, errors }
        };
        report.total_bytes += outcome.bytes;
        report.total_files += outcome.files;
        report.outcomes.push(outcome);
    }
    Ok(report)
}

/// Empty `recentProjects` / `lastWorkspaceSession` while preserving every
/// other key in `v2/setting.json`.
fn clean_recent_projects(p: &PrivacyPaths) -> CleanOutcome {
    let mut outcome = CleanOutcome { id: "recent_projects".into(), ok: false, bytes: 0, files: 0, errors: vec![] };
    let Some(path) = p.setting_json() else {
        outcome.errors.push("v2/setting.json: path unknown".into());
        return outcome;
    };
    if !path.is_file() {
        outcome.ok = true; // nothing recorded — nothing to clean
        return outcome;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            outcome.errors.push(format!("{}: {e}", path.display()));
            return outcome;
        }
    };
    outcome.bytes = text.len() as u64;
    outcome.files = 1;
    let mut v = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(v) => v,
        Err(e) => {
            outcome.errors.push(format!("{}: {e}", path.display()));
            return outcome;
        }
    };
    let had_any = ["recentProjects", "lastWorkspaceSession"]
        .iter()
        .any(|k| v.get(k).and_then(|a| a.as_array()).map(|a| !a.is_empty()).unwrap_or(false));
    if !had_any {
        outcome.ok = true;
        return outcome;
    }
    if let Some(obj) = v.as_object_mut() {
        obj.insert("recentProjects".into(), serde_json::json!([]));
        obj.insert("lastWorkspaceSession".into(), serde_json::json!([]));
    }
    match serde_json::to_string_pretty(&v) {
        Ok(new_text) => match std::fs::write(&path, new_text) {
            Ok(()) => outcome.ok = true,
            Err(e) => outcome.errors.push(format!("{}: {e}", path.display())),
        },
        Err(e) => outcome.errors.push(format!("{}: {e}", path.display())),
    }
    outcome
}

// ---------------- upload toggles ----------------

/// Read the three upload/indexing toggles from `v2/setting.json`.
pub fn switches(p: &PrivacyPaths) -> PrivacySwitches {
    let mut sw = PrivacySwitches {
        setting_path: None,
        optimize_agent_experience: None,
        repo_snapshot_indexing: None,
        instant_grep_indexing: None,
    };
    let Some(path) = p.setting_json() else { return sw };
    sw.setting_path = Some(path.to_string_lossy().to_string());
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&path).unwrap_or_default())
    else {
        return sw;
    };
    let get = |k: &str| v.get(k).and_then(|b| b.as_bool());
    sw.optimize_agent_experience = get("optimizeAgentExperienceEnabled");
    sw.repo_snapshot_indexing = get("repoSnapshotIndexingEnabled");
    sw.instant_grep_indexing = get("instantGrepIndexingEnabled");
    sw
}

/// Force the three toggles off. Requires ZCode to be closed — it rewrites
/// `setting.json` on exit, which would silently undo the change.
pub fn apply_switches(p: &PrivacyPaths) -> Result<PrivacySwitches> {
    if crate::zcode_running() {
        return Err(Error::ZcodeRunning);
    }
    let Some(path) = p.setting_json() else {
        return Err(Error::Other("v2/setting.json: path unknown".into()));
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
    let mut v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
    let obj = v
        .as_object_mut()
        .ok_or_else(|| Error::Other(format!("{}: not a JSON object", path.display())))?;
    obj.insert("optimizeAgentExperienceEnabled".into(), serde_json::json!(false));
    obj.insert("repoSnapshotIndexingEnabled".into(), serde_json::json!(false));
    obj.insert("instantGrepIndexingEnabled".into(), serde_json::json!(false));
    let new_text = serde_json::to_string_pretty(&v)
        .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
    std::fs::write(&path, new_text)
        .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
    Ok(switches(p))
}

// ---------------- workspace snapshot report ----------------

/// One file entry from a repo-snapshot manifest.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotFile {
    pub path: String,
    pub size_bytes: u64,
}

/// Upload evidence for one workspace, rebuilt from `v2/checkpoints/`.
///
/// `recorded_at_ms` is when the encrypted package was built (the upload
/// happens right after); `accepted` compares the server-accepted manifest
/// hash against the latest one, `failure_count` is the persisted retry
/// counter. `files` comes from the manifest on disk — the accepted one when
/// available, otherwise the newest.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotReport {
    pub workspace_path: String,
    pub checkpoint_dir: String,
    pub recorded_at_ms: Option<u64>,
    pub state_modified_ms: Option<u64>,
    pub workspace_bytes: Option<u64>,
    pub encrypted_bytes: Option<u64>,
    pub manifest_hash: Option<String>,
    pub accepted: Option<bool>,
    pub failure_count: Option<u64>,
    pub manifest_on_disk: bool,
    pub files: Vec<SnapshotFile>,
    pub files_bytes: u64,
    pub git_files: u64,
    pub extra_files: u64,
    pub extra_bytes: u64,
}

fn mtime_ms(path: &Path) -> Option<u64> {
    path.metadata()
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn json_file(path: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Parse a manifest's `files` array into (entries, total bytes, .git count).
fn parse_manifest_files(v: &serde_json::Value) -> (Vec<SnapshotFile>, u64, u64) {
    let mut files = vec![];
    let mut bytes = 0u64;
    let mut git = 0u64;
    for f in v.get("files").and_then(|a| a.as_array()).into_iter().flatten() {
        let Some(path) = f.get("path").and_then(|p| p.as_str()) else { continue };
        let size = f.get("sizeBytes").and_then(|s| s.as_u64()).unwrap_or(0);
        if path.starts_with(".git/") || path == ".git" {
            git += 1;
        }
        bytes += size;
        files.push(SnapshotFile { path: path.to_string(), size_bytes: size });
    }
    (files, bytes, git)
}

/// All workspace snapshot reports, newest first. Missing/undecodable parts
/// degrade to `None`/empty instead of failing the whole report.
pub fn snapshot_report(p: &PrivacyPaths) -> Vec<SnapshotReport> {
    let Some(checkpoints) = p.zcode.as_ref().map(|z| z.join("v2").join("checkpoints")) else {
        return vec![];
    };
    let mut out: Vec<SnapshotReport> = std::fs::read_dir(&checkpoints)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let dir = e.path();
            let state_path = dir.join("state.json");
            let state = json_file(&state_path).unwrap_or(serde_json::Value::Null);

            let get_u64 = |k: &str| state.get(k).and_then(|v| v.as_u64());
            let get_str = |k: &str| state.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
            let last = state.get("lastCompressedSize");
            let recorded_at_ms = last.and_then(|l| l.get("recordedAt")).and_then(|v| v.as_u64());
            let manifest_hash = last
                .and_then(|l| l.get("manifestHash"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| get_str("lastAcceptedManifestHash"));
            let accepted_hash = get_str("lastAcceptedManifestHash");
            let accepted = match (&manifest_hash, &accepted_hash) {
                (Some(a), Some(b)) => Some(a == b),
                _ => None,
            };

            // manifest on disk: the accepted one when present, else newest by createdAt
            let manifests_dir = dir.join("manifests");
            let mut candidates: Vec<(u64, std::path::PathBuf)> = std::fs::read_dir(&manifests_dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json"))
                .map(|p| {
                    let created = json_file(&p)
                        .and_then(|v| v.get("createdAt").and_then(|c| c.as_u64()))
                        .unwrap_or(0);
                    (created, p)
                })
                .collect();
            candidates.sort_by(|a, b| b.0.cmp(&a.0));
            let chosen = accepted_hash
                .as_ref()
                .and_then(|h| {
                    let p = manifests_dir.join(format!("{h}.json"));
                    p.is_file().then_some((u64::MAX, p))
                })
                .or_else(|| candidates.into_iter().next());

            let (files, files_bytes, git_files, manifest_on_disk) = match chosen
                .as_ref()
                .and_then(|(_, p)| json_file(p))
            {
                Some(v) => {
                    let (files, bytes, git) = parse_manifest_files(&v);
                    (files, bytes, git, true)
                }
                None => (vec![], 0, 0, false),
            };

            let mut extra_files = 0u64;
            let mut extra_bytes = 0u64;
            if let Some(extra_dir) = dir.join("extra-manifests").read_dir().ok() {
                for ep in extra_dir.flatten().map(|e| e.path()) {
                    if let Some(v) = json_file(&ep) {
                        if let Some(stats) = v.get("stats") {
                            extra_files += stats.get("includedFileCount").and_then(|x| x.as_u64()).unwrap_or(0);
                            extra_bytes += stats.get("includedBytes").and_then(|x| x.as_u64()).unwrap_or(0);
                        } else if let Some((f, b, _)) = v
                            .get("groups")
                            .and_then(|g| g.as_array())
                            .map(|groups| {
                                groups
                                    .iter()
                                    .flat_map(|g| g.get("files").and_then(|a| a.as_array()).into_iter().flatten())
                                    .fold((0u64, 0u64, ()), |(n, b, ()), f| {
                                        (n + 1, b + f.get("sizeBytes").and_then(|s| s.as_u64()).unwrap_or(0), ())
                                    })
                            })
                        {
                            extra_files += f;
                            extra_bytes += b;
                        }
                    }
                }
            }

            SnapshotReport {
                workspace_path: get_str("workspacePath").unwrap_or_default(),
                checkpoint_dir: dir.to_string_lossy().to_string(),
                recorded_at_ms,
                state_modified_ms: mtime_ms(&state_path),
                workspace_bytes: last.and_then(|l| l.get("workspaceSizeBytes")).and_then(|v| v.as_u64()),
                encrypted_bytes: last.and_then(|l| l.get("encryptedSizeBytes")).and_then(|v| v.as_u64()),
                manifest_hash,
                accepted,
                failure_count: get_u64("failureCount"),
                manifest_on_disk,
                files,
                files_bytes,
                git_files,
                extra_files,
                extra_bytes,
            }
        })
        .collect();
    out.sort_by(|a, b| b.recorded_at_ms.cmp(&a.recorded_at_ms));
    out
}
