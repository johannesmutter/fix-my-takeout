use crate::db::{queries, recovery, Database};
use crate::pipeline::{cataloger, dedup, extractor, metadata_loader, organizer, pairer, reporter, symlinker};
use crate::progress::tracker::ProgressTracker;
use log::{error, info};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Stage weight ranges for cumulative progress (per-zip stages sum to ~85%, post-zip stages ~15%).
/// Extracting: 0-40%, Metadata: 40-50%, Cataloging: 50-65%, Organizing: 65-85%.
/// Post-zip: Dedup: 85-90%, Symlinks: 90-95%, Report: 95-100%.
const EXTRACT_START: f64 = 0.0;
const EXTRACT_END: f64 = 0.40;
const METADATA_START: f64 = 0.40;
const METADATA_END: f64 = 0.50;
const CATALOG_START: f64 = 0.50;
const CATALOG_END: f64 = 0.65;
const ORGANIZE_START: f64 = 0.65;
const ORGANIZE_END: f64 = 0.85;
const DEDUP_PCT: f64 = 0.88;
const SYMLINKS_PCT: f64 = 0.93;
const REPORT_PCT: f64 = 0.97;

/// Emit progress as a fraction of the overall job.
/// `zip_frac` is the sub-progress within the current stage (0.0 to 1.0).
/// `stage_start`/`stage_end` define the weight band of this stage.
fn emit_cumulative(
    tracker: &ProgressTracker,
    stage: &str,
    zip_name: &str,
    zip_index: usize,
    zip_total: usize,
    stage_start: f64,
    stage_end: f64,
    zip_frac: f64,
    bytes_processed: u64,
    bytes_total: u64,
    message: &str,
    force: bool,
) {
    // Within a single zip, progress goes from stage_start to stage_end.
    // Across zips, each zip owns an equal slice of the total.
    let per_zip = 1.0 / zip_total as f64;
    let within_stage = stage_start + (stage_end - stage_start) * zip_frac.clamp(0.0, 1.0);
    let overall = (zip_index as f64 + within_stage) * per_zip;

    // Convert to files_processed/files_total representation for the frontend
    let total = 10000u64;
    let processed = (overall * total as f64).round() as u64;

    tracker.emit_progress(
        stage,
        zip_name,
        zip_index,
        zip_total,
        processed,
        total,
        bytes_processed,
        bytes_total,
        message,
        force,
    );
}

pub fn run_pipeline(
    source_dir: &Path,
    output_dir: &Path,
    db: &Database,
    tracker: &Arc<ProgressTracker>,
) -> Result<(), String> {
    tracker.start();

    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    let temp_base = output_dir.join(".temp");
    std::fs::create_dir_all(&temp_base)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let conn = db.conn.lock().unwrap();

    recovery::recover_state(&conn, &temp_base)
        .map_err(|e| format!("Recovery error: {}", e))?;

    let zip_files = discover_zips(source_dir)?;
    if zip_files.is_empty() {
        return Err("No .zip files found in source directory".to_string());
    }

    info!("Found {} zip files to process", zip_files.len());
    tracker.emit_log("info", &format!("Found {} zip files", zip_files.len()));

    for zip_path in &zip_files {
        let zip_name = zip_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown.zip");
        let size = zip_path.metadata().map(|m| m.len() as i64).ok();
        queries::upsert_zip_status(&conn, zip_name, &zip_path.to_string_lossy(), size)
            .map_err(|e| format!("DB error: {}", e))?;
    }
    tracker.emit_zip_status_changed();

    let zip_total = zip_files.len();

    for (zip_index, zip_path) in zip_files.iter().enumerate() {
        if tracker.is_cancelled() {
            tracker.emit_log("warn", "Processing cancelled");
            return Ok(());
        }
        tracker.wait_if_paused();

        let zip_name = zip_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown.zip")
            .to_string();

        let statuses = queries::get_zip_statuses(&conn)
            .map_err(|e| format!("DB error: {}", e))?;
        if let Some(s) = statuses.iter().find(|s| s.zip_name == zip_name) {
            if s.status == "done" {
                tracker.emit_log("info", &format!("Skipping completed: {}", zip_name));
                continue;
            }
        }

        tracker.emit_log("info", &format!("Processing zip {}/{}: {}", zip_index + 1, zip_total, zip_name));

        // ── Stage 1: Extract ──────────────────────────────────
        tracker.emit_stage("extracting", &zip_name);
        queries::update_zip_status(&conn, &zip_name, "extracting")
            .map_err(|e| format!("DB error: {}", e))?;
        tracker.emit_zip_status_changed();

        let temp_dir = temp_base.join(&zip_name);

        // Extract with per-file progress reporting
        let t = tracker.clone();
        let zn = zip_name.clone();
        let zi = zip_index;
        let zt = zip_total;
        let extract_result = extractor::extract_zip_with_progress(
            zip_path,
            &temp_dir,
            &move |files_done, files_total, bytes_done, bytes_total, current_file| {
                let frac = if files_total > 0 { files_done as f64 / files_total as f64 } else { 0.0 };
                emit_cumulative(
                    &t, "extracting", &zn, zi, zt,
                    EXTRACT_START, EXTRACT_END, frac,
                    bytes_done, bytes_total, current_file, false,
                );
            },
        );

        match extract_result {
            Ok(_) => {
                emit_cumulative(tracker, "extracting", &zip_name, zip_index, zip_total, EXTRACT_START, EXTRACT_END, 1.0, 0, 0, "Extraction complete", true);
            }
            Err(e) => {
                error!("Failed to extract {}: {}", zip_name, e);
                queries::mark_zip_error(&conn, &zip_name, &e)
                    .map_err(|e| format!("DB error: {}", e))?;
                tracker.emit_zip_status_changed();
                tracker.emit_log("error", &format!("Failed: {}: {}", zip_name, e));
                continue;
            }
        }

        // Extract nested zips (shared albums)
        emit_cumulative(tracker, "extracting", &zip_name, zip_index, zip_total, EXTRACT_END, METADATA_START, 0.0, 0, 0, "Extracting shared albums...", true);
        if let Err(e) = extractor::extract_nested_zips(&temp_dir) {
            tracker.emit_log("warn", &format!("Nested zip warning: {}", e));
        }

        if tracker.is_cancelled() { return Ok(()); }
        tracker.wait_if_paused();

        // ── Stage 1b: Load metadata ──────────────────────────
        tracker.emit_stage("cataloging", &zip_name);
        emit_cumulative(tracker, "cataloging", &zip_name, zip_index, zip_total, METADATA_START, METADATA_END, 0.0, 0, 0, "Reading metadata...", true);

        let metadata = match metadata_loader::load_all_metadata(&temp_dir, &zip_name, &conn) {
            Ok(m) => m,
            Err(e) => {
                error!("Metadata load error: {}", e);
                queries::mark_zip_error(&conn, &zip_name, &e)
                    .map_err(|ee| format!("DB error: {}", ee))?;
                tracker.emit_zip_status_changed();
                continue;
            }
        };

        let source_type = metadata.source_type;
        queries::update_zip_source_type(&conn, &zip_name, source_type.as_str())
            .map_err(|e| format!("DB error: {}", e))?;
        tracker.emit_log("info", &format!("Detected source: {}", match source_type { metadata_loader::SourceType::ICloud => "iCloud", metadata_loader::SourceType::GoogleTakeout => "Google Takeout", metadata_loader::SourceType::Unknown => "Unknown" }));

        emit_cumulative(tracker, "cataloging", &zip_name, zip_index, zip_total, METADATA_START, METADATA_END, 1.0, 0, 0, "Metadata loaded", true);

        // ── Stage 2: Catalog ─────────────────────────────────
        queries::update_zip_status(&conn, &zip_name, "cataloging")
            .map_err(|e| format!("DB error: {}", e))?;
        tracker.emit_zip_status_changed();
        emit_cumulative(tracker, "cataloging", &zip_name, zip_index, zip_total, CATALOG_START, CATALOG_END, 0.0, 0, 0, "Cataloging files...", true);

        let _file_count = cataloger::catalog_files(&temp_dir, &zip_name, source_type, &metadata.album_photo_map, &conn)
            .unwrap_or_else(|e| {
                tracker.emit_log("error", &format!("Catalog error: {}", e));
                0
            });

        emit_cumulative(tracker, "cataloging", &zip_name, zip_index, zip_total, CATALOG_START, CATALOG_END, 0.8, 0, 0, "Detecting pairs...", true);

        if let Err(e) = pairer::detect_pairs(&conn, &zip_name) {
            tracker.emit_log("warn", &format!("Pairing warning: {}", e));
        }

        emit_cumulative(tracker, "cataloging", &zip_name, zip_index, zip_total, CATALOG_START, CATALOG_END, 1.0, 0, 0, "Cataloging complete", true);

        if tracker.is_cancelled() { return Ok(()); }
        tracker.wait_if_paused();

        // ── Stage 3: Organize ────────────────────────────────
        tracker.emit_stage("organizing", &zip_name);
        queries::update_zip_status(&conn, &zip_name, "organizing")
            .map_err(|e| format!("DB error: {}", e))?;
        tracker.emit_zip_status_changed();
        emit_cumulative(tracker, "organizing", &zip_name, zip_index, zip_total, ORGANIZE_START, ORGANIZE_END, 0.0, 0, 0, "Organizing files...", true);

        let organized = organizer::organize_files(&conn, &zip_name, output_dir)
            .unwrap_or_else(|e| {
                tracker.emit_log("error", &format!("Organize error: {}", e));
                0
            });

        emit_cumulative(tracker, "organizing", &zip_name, zip_index, zip_total, ORGANIZE_START, ORGANIZE_END, 1.0, 0, 0, &format!("Organized {} files", organized), true);

        // ── Stage 4: Cleanup ─────────────────────────────────
        tracker.emit_log("info", &format!("Cleaning up temp for {}", zip_name));
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }

        queries::mark_zip_done(&conn, &zip_name)
            .map_err(|e| format!("DB error: {}", e))?;
        tracker.emit_zip_status_changed();
    }

    if tracker.is_cancelled() { return Ok(()); }

    // ── Post-zip stages use absolute percentages ──────────────

    // ── Stage 5: Dedup ───────────────────────────────────────
    tracker.emit_stage("dedup", "");
    tracker.emit_log("info", "Deduplicating...");
    tracker.emit_progress("dedup", "", 0, 1, (DEDUP_PCT * 10000.0) as u64, 10000, 0, 0, "Finding duplicates...", true);
    let dup_count = dedup::deduplicate(&conn, output_dir).unwrap_or_else(|e| {
        tracker.emit_log("error", &format!("Dedup error: {}", e));
        0
    });
    tracker.emit_log("info", &format!("Found {} duplicates", dup_count));

    // ── Stage 6: Symlinks ────────────────────────────────────
    tracker.emit_stage("symlinks", "");
    tracker.emit_log("info", "Creating views...");
    tracker.emit_progress("symlinks", "", 0, 1, (SYMLINKS_PCT * 10000.0) as u64, 10000, 0, 0, "Building album and filter views...", true);
    if let Err(e) = symlinker::create_symlinks(&conn, output_dir) {
        tracker.emit_log("error", &format!("Symlink error: {}", e));
    }

    // ── Stage 7: Report ──────────────────────────────────────
    tracker.emit_stage("report", "");
    tracker.emit_log("info", "Generating report...");
    tracker.emit_progress("report", "", 0, 1, (REPORT_PCT * 10000.0) as u64, 10000, 0, 0, "Generating summary...", true);
    if let Err(e) = reporter::generate_report(&conn, output_dir) {
        tracker.emit_log("error", &format!("Report error: {}", e));
    }

    if temp_base.exists() {
        let _ = std::fs::remove_dir_all(&temp_base);
    }

    tracker.emit_progress("report", "", 0, 1, 10000, 10000, 0, 0, "Complete", true);
    tracker.emit_log("info", "Processing complete!");
    tracker.emit_complete();

    Ok(())
}

fn discover_zips(source_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut zips = Vec::new();
    let entries = std::fs::read_dir(source_dir)
        .map_err(|e| format!("Failed to read source dir: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext.to_str().map(|e| e.eq_ignore_ascii_case("zip")).unwrap_or(false) {
                    zips.push(path);
                }
            }
        }
    }

    zips.sort_by(|a, b| natural_cmp(a, b));
    Ok(zips)
}

/// Natural sort: compare strings by splitting into text and numeric segments
/// so "Teil 2" comes before "Teil 10".
fn natural_cmp(a: &Path, b: &Path) -> std::cmp::Ordering {
    let a_name = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let b_name = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let a_parts = split_natural(a_name);
    let b_parts = split_natural(b_name);
    a_parts.cmp(&b_parts)
}

fn split_natural(s: &str) -> Vec<NaturalPart> {
    let mut parts = Vec::new();
    let mut chars = s.chars().peekable();
    while chars.peek().is_some() {
        if chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            let mut num = String::new();
            while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                num.push(chars.next().unwrap());
            }
            parts.push(NaturalPart::Num(num.parse::<u64>().unwrap_or(0)));
        } else {
            let mut text = String::new();
            while chars.peek().map(|c| !c.is_ascii_digit()).unwrap_or(false) {
                text.push(chars.next().unwrap());
            }
            parts.push(NaturalPart::Text(text.to_lowercase()));
        }
    }
    parts
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum NaturalPart {
    Text(String),
    Num(u64),
}
