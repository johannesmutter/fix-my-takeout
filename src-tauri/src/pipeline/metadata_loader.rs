use crate::metadata::{album_info_json, google_takeout_json, photo_details_csv, shared_library_csv, subscribed_albums_json};
use crate::metadata::album_info_json::AlbumPhotoInfo;
use log::info;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// Detected source type of a cloud export zip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    ICloud,
    GoogleTakeout,
    Unknown,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::ICloud => "icloud",
            SourceType::GoogleTakeout => "google",
            SourceType::Unknown => "unknown",
        }
    }
}

/// Result of loading all metadata from a temp directory.
#[allow(dead_code)]
pub struct MetadataLoadResult {
    pub source_type: SourceType,
    pub csv_rows: usize,
    pub shared_lib_rows: usize,
    pub album_photo_map: HashMap<String, (i64, AlbumPhotoInfo)>, // filename -> (album_id, info)
}

/// Detect source type by examining extracted content.
pub fn detect_source_type(temp_dir: &Path) -> SourceType {
    for entry in WalkDir::new(temp_dir).max_depth(5).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        let lower = filename.to_lowercase();

        // iCloud indicators
        if lower == "photo details.csv" || lower == "albuminfo.json" {
            return SourceType::ICloud;
        }

        // Google Takeout indicators: per-photo JSON sidecars (e.g. "IMG_001.jpg.json")
        if lower.ends_with(".json") && lower != "metadata.json" {
            // Check if it's a sidecar by seeing if stripping .json yields a media extension
            let without_json = &lower[..lower.len() - 5];
            if has_media_extension(without_json) {
                return SourceType::GoogleTakeout;
            }
        }
    }

    // Also check for Google Photos folder names
    for entry in WalkDir::new(temp_dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
        if entry.path().is_dir() {
            let name = entry.path().file_name().and_then(|f| f.to_str()).unwrap_or("");
            let lower = name.to_lowercase();
            if lower == "google photos" || lower == "google fotos" || lower == "google foto's" {
                return SourceType::GoogleTakeout;
            }
        }
    }

    SourceType::Unknown
}

fn has_media_extension(filename: &str) -> bool {
    let media_exts = [
        ".jpg", ".jpeg", ".png", ".heic", ".heif", ".gif", ".bmp", ".tiff", ".tif", ".webp",
        ".mov", ".mp4", ".m4v", ".avi", ".mkv", ".3gp", ".3g2",
        ".dng", ".cr2", ".cr3", ".nef", ".arw",
    ];
    media_exts.iter().any(|ext| filename.ends_with(ext))
}

/// Walk the temp directory and load all metadata files into SQLite.
/// Auto-detects source type and dispatches to the appropriate loader.
pub fn load_all_metadata(
    temp_dir: &Path,
    source_zip: &str,
    conn: &Connection,
) -> Result<MetadataLoadResult, String> {
    let source_type = detect_source_type(temp_dir);

    info!("Detected source type: {:?} for {}", source_type, source_zip);

    match source_type {
        SourceType::ICloud | SourceType::Unknown => load_icloud_metadata(temp_dir, source_zip, conn, source_type),
        SourceType::GoogleTakeout => load_google_metadata(temp_dir, source_zip, conn),
    }
}

/// Load iCloud-specific metadata (original behavior).
fn load_icloud_metadata(
    temp_dir: &Path,
    source_zip: &str,
    conn: &Connection,
    source_type: SourceType,
) -> Result<MetadataLoadResult, String> {
    let mut csv_rows = 0;
    let mut shared_lib_rows = 0;
    let mut album_photo_map: HashMap<String, (i64, AlbumPhotoInfo)> = HashMap::new();

    for entry in WalkDir::new(temp_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");
        let lower = filename.to_lowercase();

        // Photo Details.csv
        if lower == "photo details.csv" {
            match photo_details_csv::parse_and_load(path, source_zip, conn) {
                Ok(count) => csv_rows += count,
                Err(e) => log::warn!("Error loading {}: {}", path.display(), e),
            }
        }
        // Shared Library Details*.csv
        else if lower.starts_with("shared library details") && lower.ends_with(".csv") {
            match shared_library_csv::parse_and_load(path, source_zip, conn) {
                Ok(count) => shared_lib_rows += count,
                Err(e) => log::warn!("Error loading {}: {}", path.display(), e),
            }
        }
        // AlbumInfo.json
        else if lower == "albuminfo.json" {
            match album_info_json::parse_and_load(path, source_zip, conn) {
                Ok((album_id, photo_infos)) => {
                    for info in photo_infos {
                        album_photo_map.insert(info.filename.clone(), (album_id, info));
                    }
                }
                Err(e) => log::warn!("Error loading {}: {}", path.display(), e),
            }
        }
        // Subscribed Albums.json
        else if lower == "subscribed albums.json" {
            if let Err(e) = subscribed_albums_json::parse_and_load(path, source_zip, conn) {
                log::warn!("Error loading {}: {}", path.display(), e);
            }
        }
    }

    info!(
        "iCloud metadata loaded: {} CSV rows, {} shared lib rows, {} album photos",
        csv_rows,
        shared_lib_rows,
        album_photo_map.len()
    );

    Ok(MetadataLoadResult {
        source_type,
        csv_rows,
        shared_lib_rows,
        album_photo_map,
    })
}

/// Load Google Takeout metadata from JSON sidecars.
fn load_google_metadata(
    temp_dir: &Path,
    source_zip: &str,
    conn: &Connection,
) -> Result<MetadataLoadResult, String> {
    let (meta_count, album_photo_map) =
        google_takeout_json::load_all_metadata(temp_dir, source_zip, conn)?;

    info!("Google Takeout metadata loaded: {} sidecar entries", meta_count);

    Ok(MetadataLoadResult {
        source_type: SourceType::GoogleTakeout,
        csv_rows: meta_count,
        shared_lib_rows: 0,
        album_photo_map,
    })
}
