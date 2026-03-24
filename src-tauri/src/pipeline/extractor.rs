use log::{info, warn};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Progress callback: (files_done, files_total, bytes_done, bytes_total, current_file)
pub type ProgressCallback<'a> = &'a dyn Fn(u64, u64, u64, u64, &str);

/// Extract a zip file with progress reporting.
pub fn extract_zip_with_progress(
    zip_path: &Path,
    temp_dir: &Path,
    on_progress: ProgressCallback,
) -> Result<Vec<PathBuf>, String> {
    info!("Extracting {} to {}", zip_path.display(), temp_dir.display());

    fs::create_dir_all(temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let file = fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open zip {}: {}", zip_path.display(), e))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip {}: {}", zip_path.display(), e))?;

    let total_entries = archive.len() as u64;

    // Calculate total uncompressed size for byte-level progress
    let mut total_bytes: u64 = 0;
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            total_bytes += entry.size();
        }
    }

    let mut extracted_files = Vec::new();
    let mut bytes_done: u64 = 0;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;

        let entry_path = match entry.enclosed_name() {
            Some(p) => p.to_owned(),
            None => {
                warn!("Skipping zip entry with unsafe path");
                continue;
            }
        };

        let entry_size = entry.size();
        let entry_name = entry_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("")
            .to_string();

        let output_path = temp_dir.join(&entry_path);

        if entry.is_dir() {
            fs::create_dir_all(&output_path)
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        } else {
            if entry_size == 0 {
                continue;
            }

            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir: {}", e))?;
            }

            let mut outfile = fs::File::create(&output_path)
                .map_err(|e| format!("Failed to create file {}: {}", output_path.display(), e))?;

            io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Failed to extract {}: {}", entry_path.display(), e))?;

            extracted_files.push(output_path);
        }

        bytes_done += entry_size;
        on_progress(
            i as u64 + 1,
            total_entries,
            bytes_done,
            total_bytes,
            &entry_name,
        );
    }

    info!("Extracted {} files from {}", extracted_files.len(), zip_path.display());
    Ok(extracted_files)
}

/// Backwards-compatible wrapper without progress.
pub fn extract_zip(zip_path: &Path, temp_dir: &Path) -> Result<Vec<PathBuf>, String> {
    extract_zip_with_progress(zip_path, temp_dir, &|_, _, _, _, _| {})
}

/// Find and extract nested zip files (Shared Albums).
pub fn extract_nested_zips(temp_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut all_new_files = Vec::new();
    let nested_zips = find_nested_zips(temp_dir);

    for nested_zip in &nested_zips {
        info!("Extracting nested zip: {}", nested_zip.display());

        let stem = nested_zip
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("nested");
        let extract_to = nested_zip.parent().unwrap_or(temp_dir).join(stem);

        match extract_zip(nested_zip, &extract_to) {
            Ok(files) => {
                all_new_files.extend(files);
                if let Err(e) = fs::remove_file(nested_zip) {
                    warn!("Could not delete nested zip: {}", e);
                }
            }
            Err(e) => {
                warn!("Failed to extract nested zip {}: {}", nested_zip.display(), e);
            }
        }
    }

    if !all_new_files.is_empty() {
        let more = extract_nested_zips(temp_dir)?;
        all_new_files.extend(more);
    }

    Ok(all_new_files)
}

fn find_nested_zips(dir: &Path) -> Vec<PathBuf> {
    let mut zips = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                zips.extend(find_nested_zips(&path));
            } else if let Some(ext) = path.extension() {
                if ext.to_str().map(|e| e.eq_ignore_ascii_case("zip")).unwrap_or(false) {
                    zips.push(path);
                }
            }
        }
    }
    zips
}
