use std::path::{Path, PathBuf};

/// Generate a collision-free filename.
/// If `base/filename.ext` exists, try `base/filename_2.ext`, `filename_3.ext`, etc.
pub fn resolve_collision(dir: &Path, filename: &str) -> PathBuf {
    let target = dir.join(filename);
    if !target.exists() {
        return target;
    }

    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let mut counter = 2u32;
    loop {
        let new_name = if ext.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, ext)
        };
        let candidate = dir.join(&new_name);
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
        if counter > 10000 {
            // Safety valve
            return dir.join(format!("{}_{}.{}", stem, uuid_simple(), ext));
        }
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{}{}", d.as_secs(), d.subsec_nanos())
}
