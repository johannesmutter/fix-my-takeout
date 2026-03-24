use crate::db::queries;
use crate::metadata::album_info_json::AlbumPhotoInfo;
use crate::metadata::{classifier, exif, filename_parser};
use crate::pipeline::metadata_loader::SourceType;
use log::{info, warn};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;
use xxhash_rust::xxh3::xxh3_128;

/// Walk all extracted files and catalog them into the files table.
/// Returns the number of files cataloged.
pub fn catalog_files(
    temp_dir: &Path,
    source_zip: &str,
    source_type: SourceType,
    album_photo_map: &HashMap<String, (i64, AlbumPhotoInfo)>,
    conn: &Connection,
) -> Result<usize, String> {
    let mut count = 0;
    let source_type_str = source_type.as_str();

    for entry in WalkDir::new(temp_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Skip metadata files
        if classifier::is_metadata_file(path) {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");

        if filename.is_empty() {
            continue;
        }

        let file_size = path.metadata().map(|m| m.len()).unwrap_or(0) as i64;
        if file_size == 0 {
            warn!("Skipping 0-byte file: {}", path.display());
            continue;
        }

        let media_type = classifier::classify_media_type(path);
        let content_category = classifier::classify_content_category(path);
        let extension = classifier::get_extension(path);

        // Determine if it's a screenshot
        let final_media_type = if classifier::is_screenshot(filename) && media_type == "image" {
            "screenshot"
        } else {
            media_type
        };

        // Look up metadata from CSV / Google JSON (both stored in photo_metadata)
        let csv_meta = queries::lookup_photo_metadata(conn, filename)
            .map_err(|e| format!("DB lookup error: {}", e))?;

        // Look up shared library info (iCloud-specific, harmless no-op for Google)
        let shared_lib = queries::lookup_shared_library(conn, filename)
            .map_err(|e| format!("DB lookup error: {}", e))?;

        // Look up album info
        let album_info = album_photo_map.get(filename);

        // Determine date using the fallback chain
        let (date_taken, date_source) = resolve_date(
            &csv_meta,
            album_info.map(|(_, info)| info),
            path,
            filename,
            source_type,
        );

        // Parse year/month from date
        let (year, month) = if let Some(ref dt) = date_taken {
            parse_year_month(dt)
        } else {
            (None, None)
        };

        // Extract metadata fields
        let (photo_meta_id, file_checksum, is_fav, is_hidden, is_deleted) = match &csv_meta {
            Some((id, checksum, _, fav, hidden, deleted, _)) => {
                (Some(*id), checksum.clone(), *fav, *hidden, *deleted)
            }
            None => (None, None, false, false, false),
        };

        // For Google Takeout files without an Apple checksum, compute content hash
        let final_checksum = if file_checksum.is_some() {
            file_checksum
        } else if source_type == SourceType::GoogleTakeout {
            compute_content_hash(path)
        } else {
            None
        };

        let (contributor_name, contributor_appleid) = match album_info {
            Some((_, info)) => (info.contributor_name.clone(), info.contributor_appleid.clone()),
            None => (None, None),
        };

        let file_id = queries::insert_file(
            conn,
            source_zip,
            source_type_str,
            &path.to_string_lossy(),
            file_size,
            final_media_type,
            content_category,
            extension.as_deref(),
            photo_meta_id,
            final_checksum.as_deref(),
            date_taken.as_deref(),
            date_source.as_deref(),
            year,
            month,
            is_fav,
            is_hidden,
            is_deleted,
            shared_lib,
            None, // live_photo_id set later by pairer
            contributor_name.as_deref(),
            contributor_appleid.as_deref(),
        )
        .map_err(|e| format!("DB insert error: {}", e))?;

        // If this file belongs to an album, link it
        if let Some((album_id, _)) = album_info {
            let _ = queries::insert_file_album(conn, file_id, *album_id);
        }

        count += 1;
    }

    info!("Cataloged {} files from {}", count, temp_dir.display());
    Ok(count)
}

fn resolve_date(
    csv_meta: &Option<(i64, Option<String>, Option<String>, bool, bool, bool, Option<String>)>,
    album_info: Option<&AlbumPhotoInfo>,
    path: &Path,
    filename: &str,
    source_type: SourceType,
) -> (Option<String>, Option<String>) {
    // 1. Metadata date (Apple CSV parsed_date or Google JSON photoTakenTime)
    //    Both are stored in photo_metadata.parsed_date
    if let Some((_, _, Some(parsed_date), _, _, _, _)) = csv_meta {
        if !parsed_date.is_empty() {
            let source_label = match source_type {
                SourceType::ICloud => "apple_csv",
                SourceType::GoogleTakeout => "google_json",
                _ => "metadata",
            };
            return (Some(parsed_date.clone()), Some(source_label.to_string()));
        }
    }

    // 2. Album JSON date
    if let Some(info) = album_info {
        if let Some(ref date) = info.date_created {
            return (Some(date.clone()), Some("album_json".to_string()));
        }
    }

    // 3. EXIF date
    let media_type = classifier::classify_media_type(path);
    if media_type == "image" || media_type == "raw_image" {
        if let Some(exif_date) = exif::read_date_from_exif(path) {
            return (Some(exif_date), Some("exif".to_string()));
        }
    }

    // 4. Filename date
    if let Some(fn_date) = filename_parser::parse_date_from_filename(filename) {
        return (Some(fn_date), Some("filename".to_string()));
    }

    // 5. File modification time
    if let Ok(metadata) = path.metadata() {
        if let Ok(modified) = metadata.modified() {
            let dt: chrono::DateTime<chrono::Utc> = modified.into();
            return (
                Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string()),
                Some("filesystem".to_string()),
            );
        }
    }

    // 6. No date
    (None, Some("none".to_string()))
}

fn parse_year_month(iso_date: &str) -> (Option<i32>, Option<i32>) {
    // Parse from "2019-10-13T15:23:00"
    if iso_date.len() >= 7 {
        let year: Option<i32> = iso_date[..4].parse().ok();
        let month: Option<i32> = iso_date[5..7].parse().ok();
        if let (Some(y), Some(m)) = (year, month) {
            if (1970..=2100).contains(&y) && (1..=12).contains(&m) {
                return (Some(y), Some(m));
            }
        }
    }
    (None, None)
}

/// Compute a content hash (xxh3-128) for dedup of files without Apple checksums.
fn compute_content_hash(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;
    let hash = xxh3_128(&buf);
    Some(format!("{:032x}", hash))
}
