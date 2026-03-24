use crate::db::queries;
use crate::fs::collision;
use log::info;
use rusqlite::Connection;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

/// Create all symlink-based views.
pub fn create_symlinks(conn: &Connection, output_dir: &Path) -> Result<(), String> {
    let files = queries::get_files_for_symlinks(conn)
        .map_err(|e| format!("DB error: {}", e))?;

    let views = [
        "images",
        "videos",
        "screenshots",
        "live-photos",
        "favourites",
        "hidden",
        "recently-deleted",
    ];

    // Clean and recreate view directories
    for view in &views {
        let dir = output_dir.join(view);
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
    }

    let albums_dir = output_dir.join("albums");
    if albums_dir.exists() {
        let _ = std::fs::remove_dir_all(&albums_dir);
    }
    std::fs::create_dir_all(&albums_dir)
        .map_err(|e| format!("Failed to create albums dir: {}", e))?;

    let mut link_count = 0;

    for file in &files {
        let final_path = match &file.final_path {
            Some(p) => PathBuf::from(p),
            None => continue,
        };

        if !final_path.exists() {
            continue;
        }

        let filename = final_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown");

        // Images view
        if matches!(file.media_type.as_str(), "image" | "raw_image" | "live_photo_image" | "screenshot") {
            create_relative_symlink(output_dir, &final_path, "images", filename)?;
            link_count += 1;
        }

        // Videos view
        if matches!(file.media_type.as_str(), "video" | "live_photo_video") {
            create_relative_symlink(output_dir, &final_path, "videos", filename)?;
            link_count += 1;
        }

        // Screenshots view
        if file.media_type == "screenshot" {
            create_relative_symlink(output_dir, &final_path, "screenshots", filename)?;
            link_count += 1;
        }

        // Live Photos view (image half only)
        if file.media_type == "live_photo_image" {
            create_relative_symlink(output_dir, &final_path, "live-photos", filename)?;
            link_count += 1;
        }

        // Favourites
        if file.is_favourite {
            create_relative_symlink(output_dir, &final_path, "favourites", filename)?;
            link_count += 1;
        }

        // Hidden
        if file.is_hidden {
            create_relative_symlink(output_dir, &final_path, "hidden", filename)?;
            link_count += 1;
        }

        // Recently deleted
        if file.is_recently_deleted {
            create_relative_symlink(output_dir, &final_path, "recently-deleted", filename)?;
            link_count += 1;
        }
    }

    // Large files symlinks (>10MB, >100MB, >1GB)
    let large_dirs = [
        ("large-files/over-10MB", 10 * 1024 * 1024i64),
        ("large-files/over-100MB", 100 * 1024 * 1024i64),
        ("large-files/over-1GB", 1024 * 1024 * 1024i64),
    ];
    for (subdir, _) in &large_dirs {
        let dir = output_dir.join(subdir);
        if dir.exists() {
            let _ = std::fs::remove_dir_all(&dir);
        }
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
    }

    for file in &files {
        let final_path = match &file.final_path {
            Some(p) => PathBuf::from(p),
            None => continue,
        };
        if !final_path.exists() {
            continue;
        }
        let filename = final_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown");

        for (subdir, threshold) in &large_dirs {
            if file.file_size > *threshold {
                create_relative_symlink(output_dir, &final_path, subdir, filename)?;
                link_count += 1;
            }
        }
    }

    // Album symlinks
    let album_files = queries::get_album_files(conn)
        .map_err(|e| format!("DB error: {}", e))?;

    for (folder_name, _, final_path_str) in &album_files {
        let album_dir = albums_dir.join(folder_name);
        std::fs::create_dir_all(&album_dir)
            .map_err(|e| format!("Failed to create album dir: {}", e))?;

        let final_path = Path::new(final_path_str);
        if !final_path.exists() {
            continue;
        }

        let filename = final_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown");

        let link_path = collision::resolve_collision(&album_dir, filename);
        if let Ok(rel) = compute_relative_path(&link_path, final_path) {
            let _ = unix_fs::symlink(&rel, &link_path);
            link_count += 1;
        }
    }

    info!("Created {} symlinks", link_count);
    Ok(())
}

fn create_relative_symlink(
    output_dir: &Path,
    target: &Path,
    view_name: &str,
    filename: &str,
) -> Result<(), String> {
    let view_dir = output_dir.join(view_name);
    let link_path = collision::resolve_collision(&view_dir, filename);

    match compute_relative_path(&link_path, target) {
        Ok(rel) => {
            let _ = unix_fs::symlink(&rel, &link_path);
            Ok(())
        }
        Err(e) => {
            log::warn!("Could not compute relative path: {}", e);
            Ok(())
        }
    }
}

fn compute_relative_path(from: &Path, to: &Path) -> Result<PathBuf, String> {
    let from_dir = from.parent().ok_or("No parent dir")?;

    // Canonicalize paths for accurate relative computation
    let from_abs = if from_dir.exists() {
        from_dir.canonicalize().unwrap_or_else(|_| from_dir.to_path_buf())
    } else {
        from_dir.to_path_buf()
    };

    let to_abs = if to.exists() {
        to.canonicalize().unwrap_or_else(|_| to.to_path_buf())
    } else {
        to.to_path_buf()
    };

    // Find common prefix
    let from_parts: Vec<_> = from_abs.components().collect();
    let to_parts: Vec<_> = to_abs.components().collect();

    let common = from_parts
        .iter()
        .zip(to_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut rel = PathBuf::new();
    for _ in common..from_parts.len() {
        rel.push("..");
    }
    for part in &to_parts[common..] {
        rel.push(part);
    }

    Ok(rel)
}
