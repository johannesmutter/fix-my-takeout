use rusqlite::{params, Connection, Result};
use std::path::Path;
use log::info;

pub fn recover_state(conn: &Connection, temp_base: &Path) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT zip_name, status FROM zip_status WHERE status NOT IN ('done', 'pending')",
    )?;
    let interrupted: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (zip_name, status) in interrupted {
        info!("Recovery: zip '{}' was in state '{}', resetting to pending", zip_name, status);

        // Delete temp folder for this zip (partial data is unsafe)
        let temp_dir = temp_base.join(&zip_name);
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
            info!("Recovery: deleted temp dir for '{}'", zip_name);
        }

        // Delete any files from this zip that weren't fully moved
        conn.execute(
            "DELETE FROM files WHERE source_zip = ?1 AND move_status != 'done'",
            params![zip_name],
        )?;

        // Reset zip status
        conn.execute(
            "UPDATE zip_status SET status = 'pending', files_extracted = 0, error_message = NULL WHERE zip_name = ?1",
            params![zip_name],
        )?;
    }

    // Fix any files stuck in 'moving' state
    let mut move_stmt = conn.prepare(
        "SELECT id, original_path, final_path FROM files WHERE move_status = 'moving'",
    )?;
    let stuck_moves: Vec<(i64, String, Option<String>)> = move_stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (id, original, final_path) in stuck_moves {
        if let Some(ref fp) = final_path {
            if Path::new(fp).exists() {
                conn.execute(
                    "UPDATE files SET move_status = 'done' WHERE id = ?1",
                    params![id],
                )?;
                info!("Recovery: file {} found at destination, marked done", id);
            } else if Path::new(&original).exists() {
                conn.execute(
                    "UPDATE files SET move_status = 'pending', final_path = NULL WHERE id = ?1",
                    params![id],
                )?;
                info!("Recovery: file {} found at source, will retry", id);
            } else {
                conn.execute(
                    "UPDATE files SET move_status = 'error' WHERE id = ?1",
                    params![id],
                )?;
                info!("Recovery: file {} not found at source or destination", id);
            }
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub fn has_existing_session(conn: &Connection) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM zip_status",
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

#[allow(dead_code)]
pub fn get_pending_zips(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT zip_name, zip_path FROM zip_status WHERE status = 'pending' ORDER BY zip_name",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}
