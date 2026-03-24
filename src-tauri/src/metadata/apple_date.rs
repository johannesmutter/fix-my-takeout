use chrono::NaiveDateTime;

/// Parse Apple's date format: "Sunday March 31,2019 7:45 PM GMT"
/// Returns ISO 8601 string on success.
pub fn parse_apple_date(raw: &str) -> Option<NaiveDateTime> {
    let s = raw.trim().trim_matches('"');
    if s.is_empty() {
        return None;
    }

    // Strip timezone suffix (always GMT in Apple exports)
    let s = s.trim_end_matches(" GMT").trim_end_matches(" UTC");

    // Remove the weekday prefix: "Sunday March 31,2019 7:45 PM" -> "March 31,2019 7:45 PM"
    let s = if let Some(pos) = s.find(' ') {
        &s[pos + 1..]
    } else {
        return None;
    };

    // Try parsing with various formats
    let patterns = [
        "%B %e,%Y %l:%M %p",  // "March 31,2019 7:45 PM"
        "%B %d,%Y %I:%M %p",  // "March 31,2019 07:45 PM"
        "%B %e, %Y %l:%M %p", // "March 31, 2019 7:45 PM" (space after comma)
        "%B %d, %Y %I:%M %p", // "March 31, 2019 07:45 PM"
    ];

    for pattern in &patterns {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, pattern) {
            return Some(dt);
        }
    }

    // Try with seconds
    let patterns_with_sec = [
        "%B %e,%Y %l:%M:%S %p",
        "%B %d,%Y %I:%M:%S %p",
    ];

    for pattern in &patterns_with_sec {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, pattern) {
            return Some(dt);
        }
    }

    None
}

pub fn apple_date_to_iso(raw: &str) -> Option<String> {
    parse_apple_date(raw).map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_typical() {
        let input = "Sunday October 13,2019 3:23 PM GMT";
        let dt = parse_apple_date(input).unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2019-10-13 15:23");
    }

    #[test]
    fn test_parse_quoted() {
        let input = "\"Sunday October 13,2019 3:23 PM GMT\"";
        let dt = parse_apple_date(input).unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2019-10-13");
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_apple_date("").is_none());
    }
}
