use crate::db::queries;
use log::{info, warn};
use rusqlite::Connection;
use std::path::Path;

/// Parse a "Shared Library Details.csv" (or numbered variant) and load into DB.
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

    let idx = |name: &str| -> Option<usize> {
        headers.iter().position(|h| h.trim().eq_ignore_ascii_case(name))
    };

    let img_name_idx = idx("imgName").ok_or("Missing imgName column")?;
    let contributed_idx = idx("contributedByMe");

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

        let contributed = contributed_idx
            .and_then(|i| record.get(i))
            .map(|s| s.trim().eq_ignore_ascii_case("yes"))
            .unwrap_or(false);

        queries::insert_shared_library(conn, source_zip, img_name, contributed)
            .map_err(|e| format!("DB insert error: {}", e))?;

        count += 1;
    }

    info!("Loaded {} shared library rows from {}", count, csv_path.display());
    Ok(count)
}
