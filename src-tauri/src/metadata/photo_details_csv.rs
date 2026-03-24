use crate::db::queries;
use crate::metadata::apple_date;
use log::{info, warn};
use rusqlite::Connection;
use std::path::Path;

/// Parse a "Photo Details.csv" file and insert rows into photo_metadata table.
/// Returns the number of rows loaded.
pub fn parse_and_load(csv_path: &Path, source_zip: &str, conn: &Connection) -> Result<usize, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(csv_path)
        .map_err(|e| format!("Failed to open CSV {}: {}", csv_path.display(), e))?;

    let headers = reader
        .headers()
        .map_err(|e| format!("Failed to read CSV headers: {}", e))?
        .clone();

    // Find column indices by name (handles column order variations)
    let idx = |name: &str| -> Option<usize> {
        headers.iter().position(|h| h.trim().eq_ignore_ascii_case(name))
    };

    let img_name_idx = idx("imgName").ok_or("Missing imgName column")?;
    let checksum_idx = idx("fileChecksum");
    let favorite_idx = idx("favorite");
    let hidden_idx = idx("hidden");
    let deleted_idx = idx("deleted");
    let creation_date_idx = idx("originalCreationDate");
    let import_date_idx = idx("importDate");

    let mut count = 0;
    for result in reader.records() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                warn!("Skipping malformed CSV row: {}", e);
                continue;
            }
        };

        let img_name = record.get(img_name_idx).unwrap_or("").trim();
        if img_name.is_empty() {
            continue;
        }

        let checksum = checksum_idx.and_then(|i| record.get(i)).map(|s| s.trim()).filter(|s| !s.is_empty());
        let favorite = favorite_idx
            .and_then(|i| record.get(i))
            .map(|s| s.trim().eq_ignore_ascii_case("yes"))
            .unwrap_or(false);
        let hidden = hidden_idx
            .and_then(|i| record.get(i))
            .map(|s| s.trim().eq_ignore_ascii_case("yes"))
            .unwrap_or(false);
        let deleted = deleted_idx
            .and_then(|i| record.get(i))
            .map(|s| s.trim().eq_ignore_ascii_case("yes"))
            .unwrap_or(false);

        let creation_date_raw = creation_date_idx
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string());
        let parsed_date = creation_date_raw
            .as_deref()
            .and_then(apple_date::apple_date_to_iso);

        let import_date_raw = import_date_idx
            .and_then(|i| record.get(i))
            .map(|s| s.trim().to_string());
        let import_date_parsed = import_date_raw
            .as_deref()
            .and_then(apple_date::apple_date_to_iso);

        queries::insert_photo_metadata(
            conn,
            source_zip,
            img_name,
            checksum,
            favorite,
            hidden,
            deleted,
            creation_date_raw.as_deref(),
            parsed_date.as_deref(),
            import_date_raw.as_deref(),
            import_date_parsed.as_deref(),
        )
        .map_err(|e| format!("DB insert error: {}", e))?;

        count += 1;
    }

    info!("Loaded {} rows from {}", count, csv_path.display());
    Ok(count)
}
