use crate::db::queries;
use crate::fs::{collision, safe_move, sanitize};
use log::{info, warn};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Move all pending files for a zip to their final year/month locations.
pub fn organize_files(
    conn: &Connection,
    source_zip: &str,
    output_dir: &Path,
) -> Result<usize, String> {
    let pending = queries::get_pending_files(conn, source_zip)
        .map_err(|e| format!("DB error: {}", e))?;

    let mut organized = 0;

    for (file_id, original_path, media_type, year, month, content_category, source_type) in &pending {
        let source = Path::new(original_path);
        if !source.exists() {
            warn!("Source file missing: {}", original_path);
            continue;
        }

        let filename = source
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown");

        let dest_dir = compute_dest_dir(output_dir, content_category, media_type, *year, *month, source_type);
        let dest = collision::resolve_collision(&dest_dir, filename);

        match safe_move::safe_move_file(conn, *file_id, source, &dest) {
            Ok(()) => {
                organized += 1;

                // Also move paired files to the same directory
                move_paired_files(conn, *file_id, &dest_dir, output_dir)?;
            }
            Err(e) => {
                warn!("Failed to move {}: {}", original_path, e);
            }
        }
    }

    // Update zip file counts
    let _ = queries::update_zip_file_counts(
        conn,
        source_zip,
        organized as i64,
        organized as i64,
        pending.len() as i64,
    );

    info!("Organized {} files for zip '{}'", organized, source_zip);
    Ok(organized)
}

fn compute_dest_dir(
    output_dir: &Path,
    content_category: &str,
    _media_type: &str,
    year: Option<i32>,
    month: Option<i32>,
    source_type: &str,
) -> PathBuf {
    let photos_folder = match source_type {
        "google" => "Google-Photos",
        _ => "iCloud-Photos",
    };
    let drive_folder = match source_type {
        "google" => "Google-Drive",
        _ => "iCloud-Drive",
    };

    match content_category {
        "photo" => {
            match (year, month) {
                (Some(y), Some(m)) => {
                    let month_name = sanitize::month_folder_name(m as u32);
                    output_dir.join(photos_folder).join(y.to_string()).join(month_name)
                }
                _ => output_dir.join(photos_folder).join("unknown-date"),
            }
        }
        "drive" => output_dir.join(drive_folder),
        "contact" => output_dir.join("Contacts"),
        "calendar" => output_dir.join("Calendars"),
        "note" => output_dir.join("Notes"),
        "mail" => output_dir.join("Mail"),
        "message" => output_dir.join("Messages"),
        "reminder" => output_dir.join("Reminders"),
        "bookmark" => output_dir.join("Bookmarks"),
        _ => output_dir.join("_unsorted"),
    }
}

fn move_paired_files(
    conn: &Connection,
    file_id: i64,
    dest_dir: &Path,
    _output_dir: &Path,
) -> Result<(), String> {
    // Find paired files that should be moved to the same directory
    let mut stmt = conn.prepare(
        "SELECT id, original_path, file_extension FROM files
         WHERE (live_photo_pair = ?1 OR raw_jpeg_pair = ?1 OR aae_source = ?1)
         AND move_status = 'pending'"
    ).map_err(|e| format!("DB error: {}", e))?;

    let paired: Vec<(i64, String, Option<String>)> = stmt
        .query_map(params![file_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    for (pair_id, pair_path, _) in paired {
        let source = Path::new(&pair_path);
        if !source.exists() {
            continue;
        }
        let filename = source.file_name().and_then(|f| f.to_str()).unwrap_or("unknown");
        let dest = collision::resolve_collision(dest_dir, filename);
        let _ = safe_move::safe_move_file(conn, pair_id, source, &dest);
    }

    Ok(())
}
