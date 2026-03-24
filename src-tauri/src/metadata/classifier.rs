use crate::metadata::google_takeout_json;
use std::path::Path;

/// Classify a file's media type based on extension.
pub fn classify_media_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        // Images
        "heic" | "heif" | "jpg" | "jpeg" | "png" | "tiff" | "tif" | "bmp" | "gif" | "webp" => "image",
        // RAW formats
        "dng" | "cr2" | "cr3" | "nef" | "arw" | "orf" | "rw2" | "raf" | "srw" | "pef" => "raw_image",
        // Videos
        "mov" | "mp4" | "m4v" | "avi" | "mkv" | "wmv" | "3gp" | "3g2" => "video",
        // AAE sidecars
        "aae" => "aae_sidecar",
        // Other
        _ => "other",
    }
}

/// Classify a file's content category based on its path in the export.
pub fn classify_content_category(path: &Path) -> &'static str {
    let path_str = path.to_string_lossy().to_lowercase();

    // Google Takeout paths
    if path_str.contains("google photos") || path_str.contains("google fotos")
        || path_str.contains("google foto") {
        return "photo";
    }

    // iCloud / general paths
    if path_str.contains("photos") || path_str.contains("fotos") || path_str.contains("shared album") {
        return "photo";
    }
    if path_str.contains("drive") || path_str.contains("icloud drive") || path_str.contains("google drive") {
        return "drive";
    }
    if path_str.contains("contacts") || path_str.contains("kontakte") {
        return "contact";
    }
    if path_str.contains("calendars") || path_str.contains("kalender") {
        return "calendar";
    }
    if path_str.contains("notes") || path_str.contains("notizen") {
        return "note";
    }
    if path_str.contains("mail") || path_str.contains("gmail") {
        return "mail";
    }
    if path_str.contains("messages") || path_str.contains("nachrichten") {
        return "message";
    }
    if path_str.contains("reminders") || path_str.contains("erinnerungen") {
        return "reminder";
    }
    if path_str.contains("bookmarks") || path_str.contains("lesezeichen") {
        return "bookmark";
    }

    // Check by file extension for photo content
    let media_type = classify_media_type(path);
    if media_type == "image" || media_type == "raw_image" || media_type == "video" || media_type == "aae_sidecar" {
        return "photo";
    }

    "other"
}

/// Detect if a file is a screenshot based on filename patterns.
pub fn is_screenshot(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.starts_with("screenshot")
        || lower.contains("bildschirmfoto")
        || lower.contains("screen shot")
        || (lower.starts_with("img_") && lower.contains("screenshot"))
}

/// Get the file extension (lowercase, without dot).
pub fn get_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

/// Check if a file is a metadata file that should be processed but not organized.
pub fn is_metadata_file(path: &Path) -> bool {
    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    let lower = filename.to_lowercase();

    // Apple/iCloud metadata
    lower == "photo details.csv"
        || lower.starts_with("shared library details")
        || lower == "albuminfo.json"
        || lower == "subscribed albums.json"
        || (lower.ends_with(".csv") && lower.contains("detail"))
        // Google Takeout metadata
        || google_takeout_json::is_google_metadata_file(path)
}
