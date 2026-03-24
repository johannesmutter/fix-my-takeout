use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Try to read EXIF DateTimeOriginal or CreateDate from an image file.
/// Returns ISO 8601 string if found.
pub fn read_date_from_exif(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(&file);
    let exif_data = exif::Reader::new().read_from_container(&mut reader).ok()?;

    // Try DateTimeOriginal first, then CreateDate
    let tag_attempts = [
        exif::Tag::DateTimeOriginal,
        exif::Tag::DateTimeDigitized,
        exif::Tag::DateTime,
    ];

    for tag in &tag_attempts {
        if let Some(field) = exif_data.get_field(*tag, exif::In::PRIMARY) {
            let val = field.display_value().to_string();
            if let Some(iso) = exif_datetime_to_iso(&val) {
                return Some(iso);
            }
        }
    }

    None
}

/// Read the ContentIdentifier from EXIF (used for Live Photo pairing).
/// Apple stores this in MakerNote or as ContentIdentifier.
#[allow(dead_code)]
pub fn read_content_identifier(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(&file);
    let exif_data = exif::Reader::new().read_from_container(&mut reader).ok()?;

    // ContentIdentifier is a custom Apple tag, typically in MakerNote
    // For now, we rely on filename-based pairing instead
    // This is a placeholder for future enhancement
    let _ = exif_data;
    None
}

/// Convert EXIF datetime "2019:10:13 15:23:00" to ISO 8601
fn exif_datetime_to_iso(val: &str) -> Option<String> {
    let s = val.trim().trim_matches('"');
    if s.len() < 19 {
        return None;
    }

    // EXIF format: "2019:10:13 15:23:00" or "2019-10-13 15:23:00"
    let normalized = s.replace(':', "-");
    // After replacing all colons: "2019-10-13 15-23-00"
    // We need "2019-10-13T15:23:00"
    if normalized.len() >= 19 {
        let date_part = &s[..10].replace(':', "-");
        let time_part = &s[11..19]; // keep original colons for time
        let iso = format!("{}T{}", date_part, time_part);
        // Validate it looks right
        if chrono::NaiveDateTime::parse_from_str(&iso, "%Y-%m-%dT%H:%M:%S").is_ok() {
            return Some(iso);
        }
    }

    None
}
