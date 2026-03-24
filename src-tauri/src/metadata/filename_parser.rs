use regex::Regex;
use std::sync::LazyLock;

static DATE_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // IMG_20190513_152300.jpg
        (Regex::new(r"(?:IMG|VID|PANO|MVIMG|PXL)_(\d{4})(\d{2})(\d{2})_(\d{2})(\d{2})(\d{2})").unwrap(), "full"),
        // 2019-05-13 15.23.00.jpg or 2019-05-13_15-23-00.jpg
        (Regex::new(r"(\d{4})[-_.](\d{2})[-_.](\d{2})[-_ .](\d{2})[-_.](\d{2})[-_.](\d{2})").unwrap(), "full"),
        // Photo 2019-05-13.jpg
        (Regex::new(r"(\d{4})[-_](\d{2})[-_](\d{2})").unwrap(), "date_only"),
        // 20190513_152300.jpg
        (Regex::new(r"(\d{4})(\d{2})(\d{2})_(\d{2})(\d{2})(\d{2})").unwrap(), "full"),
        // 20190513.jpg
        (Regex::new(r"^(\d{4})(\d{2})(\d{2})[_\-.]").unwrap(), "date_only_compact"),
    ]
});

/// Try to extract a date from the filename.
/// Returns ISO 8601 string if a valid date pattern is found.
pub fn parse_date_from_filename(filename: &str) -> Option<String> {
    for (regex, kind) in DATE_PATTERNS.iter() {
        if let Some(caps) = regex.captures(filename) {
            match *kind {
                "full" => {
                    let y: u32 = caps.get(1)?.as_str().parse().ok()?;
                    let m: u32 = caps.get(2)?.as_str().parse().ok()?;
                    let d: u32 = caps.get(3)?.as_str().parse().ok()?;
                    let h: u32 = caps.get(4)?.as_str().parse().ok()?;
                    let min: u32 = caps.get(5)?.as_str().parse().ok()?;
                    let sec: u32 = caps.get(6)?.as_str().parse().ok()?;

                    if y >= 1970 && y <= 2100 && m >= 1 && m <= 12 && d >= 1 && d <= 31
                        && h <= 23 && min <= 59 && sec <= 59
                    {
                        return Some(format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", y, m, d, h, min, sec));
                    }
                }
                "date_only" => {
                    let y: u32 = caps.get(1)?.as_str().parse().ok()?;
                    let m: u32 = caps.get(2)?.as_str().parse().ok()?;
                    let d: u32 = caps.get(3)?.as_str().parse().ok()?;

                    if y >= 1970 && y <= 2100 && m >= 1 && m <= 12 && d >= 1 && d <= 31 {
                        return Some(format!("{:04}-{:02}-{:02}T00:00:00", y, m, d));
                    }
                }
                "date_only_compact" => {
                    let y: u32 = caps.get(1)?.as_str().parse().ok()?;
                    let m: u32 = caps.get(2)?.as_str().parse().ok()?;
                    let d: u32 = caps.get(3)?.as_str().parse().ok()?;

                    if y >= 1970 && y <= 2100 && m >= 1 && m <= 12 && d >= 1 && d <= 31 {
                        return Some(format!("{:04}-{:02}-{:02}T00:00:00", y, m, d));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_img_pattern() {
        assert_eq!(
            parse_date_from_filename("IMG_20190513_152300.jpg"),
            Some("2019-05-13T15:23:00".to_string())
        );
    }

    #[test]
    fn test_dash_pattern() {
        assert_eq!(
            parse_date_from_filename("2019-05-13 15.23.00.jpg"),
            Some("2019-05-13T15:23:00".to_string())
        );
    }

    #[test]
    fn test_date_only() {
        assert_eq!(
            parse_date_from_filename("Photo 2019-05-13.jpg"),
            Some("2019-05-13T00:00:00".to_string())
        );
    }

    #[test]
    fn test_no_date() {
        assert_eq!(parse_date_from_filename("IMG_1234.HEIC"), None);
    }
}
