/// Sanitize a string for use as a folder name.
/// Keeps emoji and Unicode, strips only filesystem-unsafe characters.
pub fn sanitize_folder_name(name: &str) -> String {
    let unsafe_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

    let sanitized: String = name
        .chars()
        .map(|c| {
            if unsafe_chars.contains(&c) {
                '_'
            } else if c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();

    // Trim leading/trailing whitespace and dots (macOS/Windows issue)
    let trimmed = sanitized.trim().trim_matches('.');

    if trimmed.is_empty() {
        "Untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Format month number to "01-January" style folder name.
pub fn month_folder_name(month: u32) -> String {
    let names = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    let name = names.get((month as usize).wrapping_sub(1)).unwrap_or(&"Unknown");
    format!("{:02}-{}", month, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keep_emoji() {
        assert_eq!(sanitize_folder_name("Yejeong ❤️ Johannes"), "Yejeong ❤️ Johannes");
    }

    #[test]
    fn test_strip_unsafe() {
        assert_eq!(sanitize_folder_name("A/B:C*D"), "A_B_C_D");
    }

    #[test]
    fn test_empty() {
        assert_eq!(sanitize_folder_name(""), "Untitled");
    }

    #[test]
    fn test_month_folder() {
        assert_eq!(month_folder_name(1), "01-January");
        assert_eq!(month_folder_name(12), "12-December");
    }
}
