use crate::db::queries;
use log::warn;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

/// Move a file with write-ahead logging in SQLite.
/// 1. Mark as 'moving' in DB
/// 2. Perform the move
/// 3. Mark as 'done' in DB
pub fn safe_move_file(
    conn: &Connection,
    file_id: i64,
    source: &Path,
    dest: &Path,
) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create dir {}: {}", parent.display(), e))?;
    }

    // Step 1: Mark as moving
    queries::update_file_move(conn, file_id, &dest.to_string_lossy(), "moving")
        .map_err(|e| format!("DB update error: {}", e))?;

    // Step 2: Try rename first (fast, same filesystem)
    let move_result = if source.exists() {
        match fs::rename(source, dest) {
            Ok(()) => Ok(()),
            Err(_) => {
                // Cross-filesystem: copy + delete
                fs::copy(source, dest)
                    .map_err(|e| format!("Copy failed: {}", e))?;
                fs::remove_file(source)
                    .map_err(|e| {
                        warn!("Could not remove source after copy: {}", e);
                        format!("Remove source failed: {}", e)
                    })
                    .ok();
                Ok(())
            }
        }
    } else if dest.exists() {
        // Already moved (crash recovery case)
        Ok(())
    } else {
        Err(format!("Source file not found: {}", source.display()))
    };

    // Step 3: Mark as done or error
    match move_result {
        Ok(()) => {
            queries::update_file_move(conn, file_id, &dest.to_string_lossy(), "done")
                .map_err(|e| format!("DB update error: {}", e))?;
            Ok(())
        }
        Err(e) => {
            queries::update_file_move(conn, file_id, &dest.to_string_lossy(), "error")
                .map_err(|ee| format!("DB update error: {}", ee))?;
            Err(e)
        }
    }
}
