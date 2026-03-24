use crate::db::{queries, Database};
use crate::fs::disk_check;
use crate::pipeline::orchestrator;
use crate::progress::tracker::ProgressTracker;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    pub db: Mutex<Option<Database>>,
    pub tracker: Mutex<Option<Arc<ProgressTracker>>>,
    pub source_path: Mutex<Option<PathBuf>>,
    pub output_path: Mutex<Option<PathBuf>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub exists: bool,
    pub zips_done: usize,
    pub zips_total: usize,
    pub files_organized: i64,
}

#[tauri::command]
pub async fn start_processing(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    source_path: String,
    output_path: String,
) -> Result<(), String> {
    let source = PathBuf::from(&source_path);
    let output = PathBuf::from(&output_path);

    if !source.exists() {
        return Err("Source directory does not exist".to_string());
    }

    // Open/create database
    let db = Database::open(&output).map_err(|e| format!("Database error: {}", e))?;

    // Create tracker
    let tracker = Arc::new(ProgressTracker::new(app));

    // Store state
    *state.db.lock().unwrap() = Some(Database::open(&output).map_err(|e| format!("DB error: {}", e))?);
    *state.tracker.lock().unwrap() = Some(tracker.clone());
    *state.source_path.lock().unwrap() = Some(source.clone());
    *state.output_path.lock().unwrap() = Some(output.clone());

    // Run pipeline in background thread
    std::thread::spawn(move || {
        if let Err(e) = orchestrator::run_pipeline(&source, &output, &db, &tracker) {
            tracker.emit_error(&e);
            tracker.emit_log("error", &format!("Pipeline failed: {}", e));
        }
    });

    Ok(())
}

#[tauri::command]
pub fn pause_processing(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(ref tracker) = *state.tracker.lock().unwrap() {
        tracker.paused.store(true, std::sync::atomic::Ordering::Relaxed);
        tracker.emit_log("info", "Processing paused");
    }
    Ok(())
}

#[tauri::command]
pub fn resume_processing(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(ref tracker) = *state.tracker.lock().unwrap() {
        tracker.paused.store(false, std::sync::atomic::Ordering::Relaxed);
        tracker.emit_log("info", "Processing resumed");
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_processing(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(ref tracker) = *state.tracker.lock().unwrap() {
        tracker.cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
        tracker.paused.store(false, std::sync::atomic::Ordering::Relaxed);
        tracker.emit_log("warn", "Processing cancelled");
    }
    Ok(())
}

#[tauri::command]
pub fn get_zip_statuses(state: State<'_, AppState>) -> Result<Vec<queries::ZipStatusRow>, String> {
    let db_lock = state.db.lock().unwrap();
    if let Some(ref db) = *db_lock {
        let conn = db.conn.lock().unwrap();
        queries::get_zip_statuses(&conn).map_err(|e| format!("DB error: {}", e))
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
pub fn get_disk_info(path: String) -> Result<disk_check::DiskInfo, String> {
    disk_check::get_disk_info(Path::new(&path))
}

#[tauri::command]
pub fn check_existing_session(output_path: String) -> Result<SessionInfo, String> {
    let db_path = Path::new(&output_path).join("catalog.db");
    if !db_path.exists() {
        return Ok(SessionInfo {
            exists: false,
            zips_done: 0,
            zips_total: 0,
            files_organized: 0,
        });
    }

    let db = Database::open(Path::new(&output_path))
        .map_err(|e| format!("DB error: {}", e))?;
    let conn = db.conn.lock().unwrap();

    let statuses = queries::get_zip_statuses(&conn)
        .map_err(|e| format!("DB error: {}", e))?;

    let done = statuses.iter().filter(|s| s.status == "done").count();
    let total = statuses.len();
    let files: i64 = statuses.iter().map(|s| s.files_organized).sum();

    Ok(SessionInfo {
        exists: true,
        zips_done: done,
        zips_total: total,
        files_organized: files,
    })
}

#[tauri::command]
pub fn get_summary_stats(state: State<'_, AppState>) -> Result<queries::SummaryStats, String> {
    let db_lock = state.db.lock().unwrap();
    if let Some(ref db) = *db_lock {
        let conn = db.conn.lock().unwrap();
        queries::get_summary_stats(&conn).map_err(|e| format!("DB error: {}", e))
    } else {
        Err("No database loaded".to_string())
    }
}

#[tauri::command]
pub fn open_in_finder(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let target = PathBuf::from(&path);

    // Validate the path is within the configured output directory
    let output_lock = state.output_path.lock().unwrap();
    if let Some(ref output_path) = *output_lock {
        let canonical_target = target.canonicalize()
            .map_err(|e| format!("Invalid path: {}", e))?;
        let canonical_output = output_path.canonicalize()
            .map_err(|e| format!("Invalid output path: {}", e))?;
        if !canonical_target.starts_with(&canonical_output) {
            return Err("Path is outside the output directory".to_string());
        }
    } else {
        return Err("No output directory configured".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open Finder: {}", e))?;
    }
    Ok(())
}
