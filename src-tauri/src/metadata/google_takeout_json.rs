use crate::db::queries;
use crate::fs::sanitize;
use crate::metadata::album_info_json::AlbumPhotoInfo;
use log::{info, warn};
use rusqlite::Connection;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct GooglePhotoMeta {
    title: Option<String>,
    description: Option<String>,
    photo_taken_time: Option<GoogleTimestamp>,
    creation_time: Option<GoogleTimestamp>,
    photo_last_modified_time: Option<GoogleTimestamp>,
    #[serde(default)]
    favorited: bool,
    #[serde(default)]
    trashed: bool,
    #[serde(default)]
    archived: bool,
    geo_data: Option<GeoData>,
}

#[derive(Debug, Deserialize)]
struct GoogleTimestamp {
    timestamp: Option<String>,
    #[allow(dead_code)]
    formatted: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeoData {
    #[allow(dead_code)]
    latitude: Option<f64>,
    #[allow(dead_code)]
    longitude: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleAlbumMeta {
    title: Option<String>,
    #[allow(dead_code)]
    description: Option<String>,
    #[allow(dead_code)]
    date: Option<GoogleTimestamp>,
}

/// Convert a Google Takeout unix timestamp string to ISO 8601.
fn google_timestamp_to_iso(ts: &GoogleTimestamp) -> Option<String> {
    let secs: i64 = ts.timestamp.as_deref()?.parse().ok()?;
    if secs <= 0 {
        return None;
    }
    let dt = chrono::DateTime::from_timestamp(secs, 0)?;
    Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string())
}

/// Determine the media filename that a JSON sidecar corresponds to.
/// Google Takeout uses `photo.jpg.json` naming. For supplemental metadata
/// like `metadata.json`, returns None.
fn media_filename_for_sidecar(json_path: &Path) -> Option<String> {
    let filename = json_path.file_name()?.to_str()?;
    let lower = filename.to_lowercase();

    // Skip album-level metadata files
    if lower == "metadata.json" || lower == "print-subscriptions.json" {
        return None;
    }

    // Strip the trailing .json to get the media filename
    // e.g. "IMG_001.jpg.json" → "IMG_001.jpg"
    if lower.ends_with(".json") {
        let media_name = &filename[..filename.len() - 5];
        // Verify it looks like it has a media extension
        if media_name.contains('.') {
            return Some(media_name.to_string());
        }
    }
    None
}

/// Detect whether a folder is a Google Photos year folder ("Photos from YYYY").
fn is_year_folder(name: &str) -> bool {
    let lower = name.to_lowercase();
    // English, German, Dutch, French, Spanish, etc.
    lower.starts_with("photos from ")
        || lower.starts_with("fotos von ")
        || lower.starts_with("foto's van ")
        || lower.starts_with("photos de ")
        || lower.starts_with("fotos de ")
        || lower.starts_with("photos à partir de ")
}

/// Load all Google Takeout JSON sidecar metadata from the extracted temp directory.
/// Returns the number of metadata rows loaded and an album photo map for the cataloger.
pub fn load_all_metadata(
    temp_dir: &Path,
    source_zip: &str,
    conn: &Connection,
) -> Result<(usize, HashMap<String, (i64, AlbumPhotoInfo)>), String> {
    let mut meta_count = 0;
    let mut album_photo_map: HashMap<String, (i64, AlbumPhotoInfo)> = HashMap::new();

    // Track which album folders we've seen (folder path → album_id)
    let mut album_ids: HashMap<String, i64> = HashMap::new();

    for entry in WalkDir::new(temp_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        let lower = filename.to_lowercase();

        if !lower.ends_with(".json") {
            continue;
        }

        // Handle album-level metadata.json
        if lower == "metadata.json" {
            if let Some(parent) = path.parent() {
                let folder_name = parent
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("Unknown");

                // Skip year folders — they're not albums
                if is_year_folder(folder_name) {
                    continue;
                }

                match parse_album_metadata(path, folder_name, source_zip, conn) {
                    Ok(album_id) => {
                        album_ids.insert(parent.to_string_lossy().to_string(), album_id);
                    }
                    Err(e) => warn!("Error loading album metadata {}: {}", path.display(), e),
                }
            }
            continue;
        }

        // Handle per-photo JSON sidecars
        if let Some(media_filename) = media_filename_for_sidecar(path) {
            match parse_sidecar(path, &media_filename, source_zip, conn) {
                Ok(()) => meta_count += 1,
                Err(e) => warn!("Error loading sidecar {}: {}", path.display(), e),
            }

            // If this file is in an album folder, link it
            if let Some(parent) = path.parent() {
                let parent_key = parent.to_string_lossy().to_string();
                if let Some(&album_id) = album_ids.get(&parent_key) {
                    album_photo_map.insert(
                        media_filename.clone(),
                        (
                            album_id,
                            AlbumPhotoInfo {
                                filename: media_filename,
                                date_created: None,
                                contributor_name: None,
                                contributor_appleid: None,
                            },
                        ),
                    );
                }
            }
        }
    }

    // Second pass: resolve album membership for files in album folders
    // that weren't yet in album_ids during first pass (metadata.json may appear after sidecars)
    // Re-scan for album folders we discovered
    for entry in WalkDir::new(temp_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        if !filename.to_lowercase().ends_with(".json") || filename.to_lowercase() == "metadata.json" {
            continue;
        }

        if let Some(media_filename) = media_filename_for_sidecar(path) {
            if album_photo_map.contains_key(&media_filename) {
                continue; // Already mapped
            }
            if let Some(parent) = path.parent() {
                let parent_key = parent.to_string_lossy().to_string();
                if let Some(&album_id) = album_ids.get(&parent_key) {
                    album_photo_map.insert(
                        media_filename.clone(),
                        (
                            album_id,
                            AlbumPhotoInfo {
                                filename: media_filename,
                                date_created: None,
                                contributor_name: None,
                                contributor_appleid: None,
                            },
                        ),
                    );
                }
            }
        }
    }

    info!(
        "Google Takeout: loaded {} photo metadata entries, {} album photos",
        meta_count,
        album_photo_map.len()
    );

    Ok((meta_count, album_photo_map))
}

/// Parse a per-photo JSON sidecar and insert into photo_metadata table.
fn parse_sidecar(
    json_path: &Path,
    media_filename: &str,
    source_zip: &str,
    conn: &Connection,
) -> Result<(), String> {
    let content = std::fs::read_to_string(json_path)
        .map_err(|e| format!("Failed to read {}: {}", json_path.display(), e))?;

    let meta: GooglePhotoMeta = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", json_path.display(), e))?;

    // Prefer photoTakenTime, fall back to creationTime
    let date_taken = meta
        .photo_taken_time
        .as_ref()
        .and_then(google_timestamp_to_iso);
    let creation_date = meta
        .creation_time
        .as_ref()
        .and_then(google_timestamp_to_iso);

    let parsed_date = date_taken.as_deref().or(creation_date.as_deref());
    let import_date = creation_date.as_deref();

    queries::insert_photo_metadata(
        conn,
        source_zip,
        media_filename,
        None, // no Apple-style checksum; computed later
        meta.favorited,
        meta.archived, // Google 'archived' maps to 'hidden'
        meta.trashed,
        meta.photo_taken_time
            .as_ref()
            .and_then(|t| t.timestamp.as_deref()),
        parsed_date,
        meta.creation_time
            .as_ref()
            .and_then(|t| t.timestamp.as_deref()),
        import_date,
    )
    .map_err(|e| format!("DB insert error: {}", e))?;

    Ok(())
}

/// Parse album-level metadata.json and create an album entry.
fn parse_album_metadata(
    json_path: &Path,
    folder_name: &str,
    source_zip: &str,
    conn: &Connection,
) -> Result<i64, String> {
    let content = std::fs::read_to_string(json_path)
        .map_err(|e| format!("Failed to read {}: {}", json_path.display(), e))?;

    let meta: GoogleAlbumMeta = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse {}: {}", json_path.display(), e))?;

    let album_name = meta.title.as_deref().unwrap_or(folder_name);
    let safe_folder = sanitize::sanitize_folder_name(album_name);

    let album_id = queries::insert_album(
        conn,
        album_name,
        "google_album",
        None,
        None,
        None,
        false,
        false,
        Some(source_zip),
        &safe_folder,
    )
    .map_err(|e| format!("DB insert error: {}", e))?;

    info!("Created Google album '{}' from {}", album_name, json_path.display());
    Ok(album_id)
}

/// Check if a file is a Google Takeout JSON metadata file.
pub fn is_google_metadata_file(path: &Path) -> bool {
    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    let lower = filename.to_lowercase();

    if !lower.ends_with(".json") {
        return false;
    }

    // Album-level metadata
    if lower == "metadata.json" || lower == "print-subscriptions.json" {
        return true;
    }

    // Per-photo sidecar: something.ext.json where ext is a media extension
    media_filename_for_sidecar(path).is_some()
}
