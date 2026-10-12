use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const STATE_FILE: &str = "nova-state.json";
const BACKUP_FILE: &str = "nova-state.backup.json";
const TEMP_FILE: &str = "nova-state.tmp.json";
const MIGRATION_MARKER: &str = ".v7-alpha6.1-appdata-migration-complete";
const MAX_STATE_BYTES: usize = 64 * 1024 * 1024;

/*
 * V7 alpha 6.1 durable native memory
 * ----------------------------------
 * Restore the proven V5 persistence contract: Nova's ONE authoritative native
 * state lives in Tauri's stable app_data_dir(), whose identity is
 * `com.qtech.nova`. The application version must never be part of this path.
 *
 * The V6.2.15/V7 custom %LOCALAPPDATA%\\QTech\\Nova\\UserData directory is now
 * a migration SOURCE only. On the first alpha 6.1 load, all known valid native
 * snapshots are compared by persistence.savedAt (then file mtime), the newest
 * one is committed into app_data_dir(), and migration is marked complete.
 *
 * After that point there is one writer and one native authority again.
 */
fn persistent_state_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve Nova app-data directory: {error}"))
}

fn state_paths_for_dir(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    (
        dir.join(STATE_FILE),
        dir.join(BACKUP_FILE),
        dir.join(TEMP_FILE),
    )
}

fn persistent_state_paths(app: &AppHandle) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    Ok(state_paths_for_dir(&persistent_state_dir(app)?))
}

fn legacy_state_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::<PathBuf>::new();

    // V6.2.15 through V7 alpha 6 custom authority. Alpha 6.1 imports it once,
    // then retires it as an active persistence destination.
    #[cfg(target_os = "windows")]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let base = PathBuf::from(&local);
            dirs.push(base.join("QTech").join("Nova").join("UserData"));
            dirs.push(base.join("Nova"));
            dirs.push(base.join("com.qtech.nova"));
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let base = PathBuf::from(appdata);
            dirs.push(base.join("Nova"));
            dirs.push(base.join("com.qtech.nova"));
        }
    }

    // Earlier/native variants and the old local-data fallback remain read-only
    // migration candidates. app_data_dir itself is added separately as primary.
    if let Ok(dir) = app.path().app_local_data_dir() {
        dirs.push(dir.clone());
        dirs.push(dir.join("UserData"));
    }

    let primary = persistent_state_dir(app).ok();
    let mut unique = Vec::new();
    for dir in dirs {
        if primary.as_ref().map(|p| p == &dir).unwrap_or(false) {
            continue;
        }
        if !unique.iter().any(|existing: &PathBuf| existing == &dir) {
            unique.push(dir);
        }
    }
    unique
}

fn migration_marker_path(dir: &Path) -> PathBuf {
    dir.join(MIGRATION_MARKER)
}

fn mark_migration_complete(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir)
        .map_err(|error| format!("Could not create Nova app-data directory: {error}"))?;
    let marker = migration_marker_path(dir);
    if !marker.exists() {
        write_and_sync(
            &marker,
            b"Nova V7 alpha 6.1 app-data persistence migration checked.\n",
        )?;
    }
    Ok(())
}

fn read_text(path: &Path) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("Could not read Nova state from {}: {error}", path.display()))?;
    // Invalid UTF-8 is a corrupt snapshot, not a reason to skip the valid backup.
    // Treat it as unusable state and continue recovery.
    Ok(String::from_utf8(bytes).ok())
}

fn valid_state_json(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .map(|value| value.is_object())
        .unwrap_or(false)
}

fn state_saved_at(raw: &str) -> u64 {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| value.get("persistence")?.get("savedAt")?.as_u64())
        .unwrap_or(0)
}

fn load_state_from_paths(state_path: &Path, backup_path: &Path) -> Result<Option<String>, String> {
    if let Some(raw) = read_text(state_path)? {
        if !raw.trim().is_empty() && valid_state_json(&raw) {
            return Ok(Some(raw));
        }
    }
    if let Some(raw) = read_text(backup_path)? {
        if !raw.trim().is_empty() && valid_state_json(&raw) {
            return Ok(Some(raw));
        }
    }
    Ok(None)
}

fn write_and_sync(path: &Path, payload: &[u8]) -> Result<(), String> {
    let mut file = File::create(path)
        .map_err(|error| format!("Could not create {}: {error}", path.display()))?;
    file.write_all(payload)
        .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("Could not flush {} to disk: {error}", path.display()))?;
    Ok(())
}

fn sync_existing_file(path: &Path) -> Result<(), String> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("Could not open {} for flush: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("Could not flush {} to disk: {error}", path.display()))
}

fn save_state_to_paths(
    state_path: &Path,
    backup_path: &Path,
    temp_path: &Path,
    payload: &str,
) -> Result<(), String> {
    if payload.len() > MAX_STATE_BYTES {
        return Err(format!(
            "Nova state is too large to persist safely ({} bytes).",
            payload.len()
        ));
    }
    if payload.trim().is_empty() {
        return Err("Nova state payload was empty.".to_string());
    }
    if !valid_state_json(payload) {
        return Err("Nova state payload was not valid JSON.".to_string());
    }

    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create Nova app-data directory: {error}"))?;
    }

    // V5-style atomic flow, strengthened: complete + fsync the temp snapshot
    // before touching current state. Only a VALID current snapshot may replace
    // the last-known-good backup.
    write_and_sync(temp_path, payload.as_bytes())?;

    let current_is_valid = read_text(state_path)?
        .as_deref()
        .map(valid_state_json)
        .unwrap_or(false);

    if current_is_valid {
        fs::copy(state_path, backup_path)
            .map_err(|error| format!("Could not create Nova state backup: {error}"))?;
        sync_existing_file(backup_path)?;
    }

    if state_path.exists() {
        fs::remove_file(state_path)
            .map_err(|error| format!("Could not replace previous Nova state: {error}"))?;
    }

    if let Err(error) = fs::rename(temp_path, state_path) {
        let _ = fs::remove_file(temp_path);
        // If the commit failed after current was removed, restore the verified
        // previous snapshot when one exists. Never leave a failed save looking
        // like a successful blank reset.
        if backup_path.exists() && !state_path.exists() {
            if fs::copy(backup_path, state_path).is_ok() {
                let _ = sync_existing_file(state_path);
            }
        }
        return Err(format!("Could not commit Nova state atomically: {error}"));
    }

    sync_existing_file(state_path)?;
    let committed = read_text(state_path)?;
    let verified = committed
        .as_deref()
        .map(|raw| raw == payload && valid_state_json(raw))
        .unwrap_or(false);
    if !verified {
        if backup_path.exists() {
            let _ = fs::remove_file(state_path);
            if fs::copy(backup_path, state_path).is_ok() {
                let _ = sync_existing_file(state_path);
            }
        }
        return Err("Nova state commit verification failed; the last-known-good backup was restored when available.".to_string());
    }
    Ok(())
}

fn clear_state_paths(state_path: &Path, backup_path: &Path, temp_path: &Path) -> Result<(), String> {
    for path in [state_path, backup_path, temp_path] {
        if path.exists() {
            fs::remove_file(path)
                .map_err(|error| format!("Could not remove {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

fn file_bytes(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn modified_unix_ms(path: &Path) -> Option<u128> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis())
}

#[derive(Clone)]
struct StateCandidate {
    raw: String,
    path: PathBuf,
    saved_at: u64,
    modified_at: u128,
    primary_priority: bool,
}

fn candidate_from_file(path: &Path, primary_priority: bool) -> Result<Option<StateCandidate>, String> {
    let Some(raw) = read_text(path)? else {
        return Ok(None);
    };
    if raw.trim().is_empty() || !valid_state_json(&raw) {
        return Ok(None);
    }
    Ok(Some(StateCandidate {
        saved_at: state_saved_at(&raw),
        modified_at: modified_unix_ms(path).unwrap_or(0),
        raw,
        path: path.to_path_buf(),
        primary_priority,
    }))
}

fn newest_candidate(mut candidates: Vec<StateCandidate>) -> Option<StateCandidate> {
    candidates.sort_by(|a, b| {
        a.saved_at
            .cmp(&b.saved_at)
            .then(a.modified_at.cmp(&b.modified_at))
            .then(a.primary_priority.cmp(&b.primary_priority))
    });
    candidates.pop()
}

fn migration_candidates(app: &AppHandle) -> Result<Vec<StateCandidate>, String> {
    let primary_dir = persistent_state_dir(app)?;
    let (primary_state, primary_backup, _) = state_paths_for_dir(&primary_dir);
    let mut candidates = Vec::new();
    if let Some(candidate) = candidate_from_file(&primary_state, true)? {
        candidates.push(candidate);
    }
    if let Some(candidate) = candidate_from_file(&primary_backup, true)? {
        candidates.push(candidate);
    }
    for dir in legacy_state_dirs(app) {
        let (state, backup, _) = state_paths_for_dir(&dir);
        // Retired locations are recovery candidates, not current authority. An
        // unreadable stale directory must not block a valid current app-data mind.
        if let Ok(Some(candidate)) = candidate_from_file(&state, false) {
            candidates.push(candidate);
        }
        if let Ok(Some(candidate)) = candidate_from_file(&backup, false) {
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NovaStateDiagnostics {
    directory: String,
    state_file: String,
    backup_file: String,
    state_exists: bool,
    backup_exists: bool,
    state_bytes: u64,
    backup_bytes: u64,
    state_modified_at: Option<u128>,
    backup_modified_at: Option<u128>,
    migration_complete: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NovaSaveReceipt {
    directory: String,
    state_file: String,
    bytes: u64,
    committed_at: u128,
}

fn diagnostics(app: &AppHandle) -> Result<NovaStateDiagnostics, String> {
    let dir = persistent_state_dir(app)?;
    let (state, backup, _) = state_paths_for_dir(&dir);
    Ok(NovaStateDiagnostics {
        directory: dir.to_string_lossy().into_owned(),
        state_file: state.to_string_lossy().into_owned(),
        backup_file: backup.to_string_lossy().into_owned(),
        state_exists: state.exists(),
        backup_exists: backup.exists(),
        state_bytes: file_bytes(&state),
        backup_bytes: file_bytes(&backup),
        state_modified_at: modified_unix_ms(&state),
        backup_modified_at: modified_unix_ms(&backup),
        migration_complete: migration_marker_path(&dir).exists(),
    })
}

#[tauri::command]
pub fn nova_state_diagnostics(app: AppHandle) -> Result<NovaStateDiagnostics, String> {
    diagnostics(&app)
}

#[tauri::command]
pub fn nova_state_load(app: AppHandle) -> Result<Option<String>, String> {
    let target_dir = persistent_state_dir(&app)?;
    let (state_path, backup_path, temp_path) = state_paths_for_dir(&target_dir);

    if migration_marker_path(&target_dir).exists() {
        return load_state_from_paths(&state_path, &backup_path);
    }

    // First alpha 6.1 startup only: compare every known valid native snapshot.
    // This protects a newer V7 custom-folder mind from being overwritten by an
    // older but still valid V5 app-data snapshot (and vice versa).
    let chosen = newest_candidate(migration_candidates(&app)?);
    if let Some(candidate) = chosen {
        let primary_current = state_path == candidate.path;
        if !primary_current || read_text(&state_path)?.as_deref() != Some(candidate.raw.as_str()) {
            save_state_to_paths(&state_path, &backup_path, &temp_path, &candidate.raw)?;
        }
        mark_migration_complete(&target_dir)?;
        return Ok(Some(candidate.raw));
    }

    mark_migration_complete(&target_dir)?;
    Ok(None)
}

#[tauri::command]
pub fn nova_state_save(app: AppHandle, payload: String) -> Result<NovaSaveReceipt, String> {
    let dir = persistent_state_dir(&app)?;
    let (state_path, backup_path, temp_path) = state_paths_for_dir(&dir);
    save_state_to_paths(&state_path, &backup_path, &temp_path, &payload)?;
    mark_migration_complete(&dir)?;

    let committed = load_state_from_paths(&state_path, &backup_path)?
        .ok_or_else(|| "Nova state save completed but no valid committed snapshot could be read back.".to_string())?;
    if committed != payload {
        return Err("Nova state save verification did not match the submitted snapshot.".to_string());
    }

    let committed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    Ok(NovaSaveReceipt {
        directory: dir.to_string_lossy().into_owned(),
        state_file: state_path.to_string_lossy().into_owned(),
        bytes: file_bytes(&state_path),
        committed_at,
    })
}

#[tauri::command]
pub fn nova_state_clear(app: AppHandle) -> Result<(), String> {
    // Reset clears the ONE current app-data authority. Keep the alpha 6.1
    // migration marker so an explicit reset cannot resurrect historical V5/V7
    // snapshots from retired locations on the next launch.
    let (state_path, backup_path, temp_path) = persistent_state_paths(&app)?;
    clear_state_paths(&state_path, &backup_path, &temp_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_dir(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("nova-persistence-{label}-{stamp}"))
    }

    fn candidate(raw: &str, path: &str, modified_at: u128, primary: bool) -> StateCandidate {
        StateCandidate {
            raw: raw.to_string(),
            path: PathBuf::from(path),
            saved_at: state_saved_at(raw),
            modified_at,
            primary_priority: primary,
        }
    }

    #[test]
    fn native_state_round_trip_and_backup_recovery() {
        let dir = temp_test_dir("roundtrip");
        let (state_path, backup_path, temp_path) = state_paths_for_dir(&dir);
        let first = r#"{"messages":[],"context":{"probe":"first"},"persistence":{"savedAt":1}}"#;
        let second = r#"{"messages":[],"context":{"probe":"second"},"persistence":{"savedAt":2}}"#;

        save_state_to_paths(&state_path, &backup_path, &temp_path, first).unwrap();
        save_state_to_paths(&state_path, &backup_path, &temp_path, second).unwrap();
        assert_eq!(fs::read_to_string(&backup_path).unwrap(), first);

        fs::write(&state_path, "not valid json").unwrap();
        assert_eq!(load_state_from_paths(&state_path, &backup_path).unwrap(), Some(first.to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_utf8_current_falls_back_to_valid_backup() {
        let dir = temp_test_dir("non-utf8");
        let (state_path, backup_path, _) = state_paths_for_dir(&dir);
        let good = r#"{"messages":[],"context":{"probe":"backup"},"persistence":{"savedAt":1}}"#;
        fs::create_dir_all(&dir).unwrap();
        fs::write(&state_path, [0xff, 0xfe, 0xfd]).unwrap();
        write_and_sync(&backup_path, good.as_bytes()).unwrap();
        assert_eq!(load_state_from_paths(&state_path, &backup_path).unwrap(), Some(good.to_string()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_current_never_overwrites_good_backup() {
        let dir = temp_test_dir("invalid-current");
        let (state_path, backup_path, temp_path) = state_paths_for_dir(&dir);
        let good = r#"{"messages":[],"context":{"probe":"good"},"persistence":{"savedAt":1}}"#;
        let next = r#"{"messages":[],"context":{"probe":"next"},"persistence":{"savedAt":2}}"#;
        fs::create_dir_all(&dir).unwrap();
        write_and_sync(&backup_path, good.as_bytes()).unwrap();
        write_and_sync(&state_path, b"corrupt").unwrap();
        save_state_to_paths(&state_path, &backup_path, &temp_path, next).unwrap();
        assert_eq!(fs::read_to_string(&backup_path).unwrap(), good);
        assert_eq!(fs::read_to_string(&state_path).unwrap(), next);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_prefers_newest_saved_at_over_location() {
        let old = r#"{"messages":[],"context":{},"persistence":{"savedAt":100}}"#;
        let newer = r#"{"messages":[],"context":{},"persistence":{"savedAt":200}}"#;
        let chosen = newest_candidate(vec![
            candidate(old, "primary", 500, true),
            candidate(newer, "legacy-v7", 100, false),
        ])
        .unwrap();
        assert_eq!(chosen.saved_at, 200);
        assert_eq!(chosen.path, PathBuf::from("legacy-v7"));
    }

    #[test]
    fn migration_uses_mtime_then_primary_as_tiebreakers() {
        let raw = r#"{"messages":[],"context":{},"persistence":{"savedAt":0}}"#;
        let chosen = newest_candidate(vec![
            candidate(raw, "legacy", 100, false),
            candidate(raw, "primary", 200, true),
        ])
        .unwrap();
        assert_eq!(chosen.path, PathBuf::from("primary"));

        let tied = newest_candidate(vec![
            candidate(raw, "legacy", 200, false),
            candidate(raw, "primary", 200, true),
        ])
        .unwrap();
        assert_eq!(tied.path, PathBuf::from("primary"));
    }

    #[test]
    fn rejects_invalid_or_oversized_native_state() {
        let dir = temp_test_dir("validation");
        let (state_path, backup_path, temp_path) = state_paths_for_dir(&dir);
        assert!(save_state_to_paths(&state_path, &backup_path, &temp_path, "").is_err());
        assert!(save_state_to_paths(&state_path, &backup_path, &temp_path, "not json").is_err());
        let too_large = format!("{{\"payload\":\"{}\"}}", "x".repeat(MAX_STATE_BYTES + 1));
        assert!(save_state_to_paths(&state_path, &backup_path, &temp_path, &too_large).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
