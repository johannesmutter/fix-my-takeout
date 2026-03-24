use log::info;
use rusqlite::{params, Connection};

/// Detect and link Live Photo pairs (same stem, .HEIC + .MOV),
/// RAW+JPEG pairs (same stem, .DNG/.CR2 + .JPG), and
/// .aae sidecars (same stem + .aae).
pub fn detect_pairs(conn: &Connection, source_zip: &str) -> Result<(), String> {
    detect_live_photo_pairs(conn, source_zip)?;
    detect_raw_jpeg_pairs(conn, source_zip)?;
    detect_aae_sidecars(conn, source_zip)?;
    Ok(())
}

fn detect_live_photo_pairs(conn: &Connection, source_zip: &str) -> Result<(), String> {
    // Find image files that have a matching video with the same stem
    // Query all files, group by stem in Rust
    let mut stmt = conn.prepare(
        "SELECT id, original_path, media_type, file_extension
         FROM files WHERE source_zip = ?1 AND (media_type IN ('image', 'video') OR media_type = 'live_photo_image')
         ORDER BY original_path"
    ).map_err(|e| format!("Query error: {}", e))?;

    let files: Vec<(i64, String, String, Option<String>)> = stmt
        .query_map(params![source_zip], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let mut paired = 0;
    let image_exts = ["heic", "heif", "jpg", "jpeg"];
    let video_exts = ["mov", "mp4"];

    for i in 0..files.len() {
        let (id_a, path_a, _, ext_a) = &files[i];
        let stem_a = get_stem(path_a);
        let ext_a_lower = ext_a.as_deref().unwrap_or("").to_lowercase();

        if !image_exts.contains(&ext_a_lower.as_str()) {
            continue;
        }

        for j in 0..files.len() {
            if i == j { continue; }
            let (id_b, path_b, _, ext_b) = &files[j];
            let stem_b = get_stem(path_b);
            let ext_b_lower = ext_b.as_deref().unwrap_or("").to_lowercase();

            if stem_a == stem_b && video_exts.contains(&ext_b_lower.as_str()) {
                // Mark as Live Photo pair
                conn.execute(
                    "UPDATE files SET media_type = 'live_photo_image', live_photo_pair = ?1 WHERE id = ?2",
                    params![id_b, id_a],
                ).map_err(|e| format!("DB error: {}", e))?;
                conn.execute(
                    "UPDATE files SET media_type = 'live_photo_video', live_photo_pair = ?1 WHERE id = ?2",
                    params![id_a, id_b],
                ).map_err(|e| format!("DB error: {}", e))?;
                paired += 1;
                break;
            }
        }
    }

    info!("Found {} Live Photo pairs", paired);
    Ok(())
}

fn detect_raw_jpeg_pairs(conn: &Connection, source_zip: &str) -> Result<(), String> {
    let mut stmt = conn.prepare(
        "SELECT id, original_path, media_type, file_extension
         FROM files WHERE source_zip = ?1 AND media_type IN ('image', 'raw_image')
         ORDER BY original_path"
    ).map_err(|e| format!("Query error: {}", e))?;

    let files: Vec<(i64, String, String, Option<String>)> = stmt
        .query_map(params![source_zip], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let raw_exts = ["dng", "cr2", "cr3", "nef", "arw", "orf", "rw2", "raf"];
    let jpeg_exts = ["jpg", "jpeg"];
    let mut paired = 0;

    for i in 0..files.len() {
        let (id_a, path_a, _, ext_a) = &files[i];
        let ext_a_lower = ext_a.as_deref().unwrap_or("").to_lowercase();
        if !raw_exts.contains(&ext_a_lower.as_str()) {
            continue;
        }
        let stem_a = get_stem(path_a);

        for j in 0..files.len() {
            if i == j { continue; }
            let (id_b, path_b, _, ext_b) = &files[j];
            let ext_b_lower = ext_b.as_deref().unwrap_or("").to_lowercase();
            if stem_a == get_stem(path_b) && jpeg_exts.contains(&ext_b_lower.as_str()) {
                conn.execute(
                    "UPDATE files SET raw_jpeg_pair = ?1 WHERE id = ?2",
                    params![id_b, id_a],
                ).map_err(|e| format!("DB error: {}", e))?;
                conn.execute(
                    "UPDATE files SET raw_jpeg_pair = ?1 WHERE id = ?2",
                    params![id_a, id_b],
                ).map_err(|e| format!("DB error: {}", e))?;
                paired += 1;
                break;
            }
        }
    }

    info!("Found {} RAW+JPEG pairs", paired);
    Ok(())
}

fn detect_aae_sidecars(conn: &Connection, source_zip: &str) -> Result<(), String> {
    let mut stmt = conn.prepare(
        "SELECT id, original_path, file_extension
         FROM files WHERE source_zip = ?1
         ORDER BY original_path"
    ).map_err(|e| format!("Query error: {}", e))?;

    let files: Vec<(i64, String, Option<String>)> = stmt
        .query_map(params![source_zip], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| format!("Query error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let mut paired = 0;

    for i in 0..files.len() {
        let (id_aae, path_aae, ext_aae) = &files[i];
        if ext_aae.as_deref().unwrap_or("") != "aae" {
            continue;
        }
        let stem = get_stem(path_aae);

        // Find the source image
        for j in 0..files.len() {
            if i == j { continue; }
            let (id_src, path_src, ext_src) = &files[j];
            let ext_src_lower = ext_src.as_deref().unwrap_or("").to_lowercase();
            if get_stem(path_src) == stem && ext_src_lower != "aae" {
                conn.execute(
                    "UPDATE files SET aae_source = ?1 WHERE id = ?2",
                    params![id_src, id_aae],
                ).map_err(|e| format!("DB error: {}", e))?;
                paired += 1;
                break;
            }
        }
    }

    info!("Found {} .aae sidecar pairs", paired);
    Ok(())
}

fn get_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
}
