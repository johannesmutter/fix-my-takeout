use crate::db::queries;
use crate::fs::collision;
use log::info;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;

/// Deduplicate files using checksums (Apple fileChecksum or computed content hash).
/// Files with the same checksum are confirmed duplicates.
/// Keeps the earliest by import date, moves others to duplicates/.
pub fn deduplicate(conn: &Connection, output_dir: &Path) -> Result<usize, String> {
    let duplicates_dir = output_dir.join("duplicates");

    // Group files by checksum
    let mut stmt = conn.prepare(
        "SELECT f.id, f.file_checksum, f.file_size, f.final_path,
                COALESCE(pm.import_date_parsed, f.date_taken, '') as sort_date
         FROM files f
         LEFT JOIN photo_metadata pm ON f.photo_meta_id = pm.id
         WHERE f.is_duplicate = 0 AND f.move_status = 'done' AND f.file_checksum IS NOT NULL
         ORDER BY f.file_checksum, sort_date ASC"
    ).map_err(|e| format!("Query error: {}", e))?;

    let files: Vec<(i64, String, i64, Option<String>, String)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    // Group by checksum
    let mut groups: HashMap<String, Vec<(i64, Option<String>)>> = HashMap::new();
    for (id, checksum, _, final_path, _) in &files {
        if !checksum.is_empty() {
            groups.entry(checksum.clone()).or_default().push((*id, final_path.clone()));
        }
    }

    let mut dup_count = 0;

    for (_, group) in &groups {
        if group.len() <= 1 {
            continue;
        }

        // Keep the first (earliest), mark rest as duplicates
        let keeper_id = group[0].0;

        for &(dup_id, ref final_path) in &group[1..] {
            queries::mark_duplicate(conn, dup_id, keeper_id)
                .map_err(|e| format!("DB error: {}", e))?;

            // Move the duplicate file to duplicates/
            if let Some(ref fp) = final_path {
                let source = Path::new(fp);
                if source.exists() {
                    let filename = source.file_name().and_then(|f| f.to_str()).unwrap_or("dup");
                    let dest = collision::resolve_collision(&duplicates_dir, filename);
                    if let Some(parent) = dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::rename(source, &dest);
                    // Update final_path
                    let _ = conn.execute(
                        "UPDATE files SET final_path = ?1 WHERE id = ?2",
                        params![dest.to_string_lossy().as_ref(), dup_id],
                    );
                }
            }

            dup_count += 1;
        }
    }

    info!("Found and moved {} duplicates", dup_count);
    Ok(dup_count)
}
