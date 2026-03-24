use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub available_bytes: u64,
    pub total_bytes: u64,
    pub is_hdd: bool,
    pub filesystem: String,
}

/// Get disk info for the given path. Path must exist.
pub fn get_disk_info(path: &Path) -> Result<DiskInfo, String> {
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Get available space using df
        let output = Command::new("df")
            .arg("-k")
            .arg(path)
            .output()
            .map_err(|e| format!("Failed to run df: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if lines.len() < 2 {
            return Err("Could not parse df output".to_string());
        }

        let parts: Vec<&str> = lines[1].split_whitespace().collect();
        if parts.len() < 4 {
            return Err("Could not parse df output".to_string());
        }

        let total_kb: u64 = parts[1].parse().unwrap_or(0);
        let available_kb: u64 = parts[3].parse().unwrap_or(0);

        // Detect HDD vs SSD (rough heuristic via system_profiler is slow,
        // so we default to false / SSD assumed)
        Ok(DiskInfo {
            available_bytes: available_kb * 1024,
            total_bytes: total_kb * 1024,
            is_hdd: false,
            filesystem: "APFS".to_string(),
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(DiskInfo {
            available_bytes: 0,
            total_bytes: 0,
            is_hdd: false,
            filesystem: "unknown".to_string(),
        })
    }
}

/// Estimate the space needed for the export (rough: 1.1x the total zip size).
#[allow(dead_code)]
pub fn estimate_space_needed(zip_sizes: &[u64]) -> u64 {
    let total: u64 = zip_sizes.iter().sum();
    (total as f64 * 1.1) as u64
}
