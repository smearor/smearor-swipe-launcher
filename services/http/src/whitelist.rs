/// Checks whether a URL is allowed by at least one of the configured whitelist patterns.
pub fn is_url_allowed(url: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }

    patterns.iter().any(|pattern| url_matches_pattern(url, pattern))
}

/// Converts a whitelist pattern with `*` wildcards into a regex and tests the URL.
fn url_matches_pattern(url: &str, pattern: &str) -> bool {
    let regex_pattern = pattern.replace(".", r"\.").replace("*", ".*");
    let Ok(regex) = regex::Regex::new(&regex_pattern) else {
        return false;
    };
    regex.is_match(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_url_matches() {
        assert!(is_url_allowed("http://localhost:8080/status", &["http://localhost:8080/status".to_string()]));
    }

    #[test]
    fn wildcard_matches_ip_range() {
        let patterns = vec!["192.168.178.*".to_string()];
        assert!(is_url_allowed("http://192.168.178.71/relay/0?turn=on", &patterns));
        assert!(!is_url_allowed("http://10.0.0.1/api/status", &patterns));
    }

    #[test]
    fn wildcard_matches_host_and_path() {
        let patterns = vec!["https://api.weather.*".to_string()];
        assert!(is_url_allowed("https://api.weather.example.com/v1/current", &patterns));
        assert!(!is_url_allowed("https://evil.example.com/", &patterns));
    }

    #[test]
    fn empty_patterns_rejects_everything() {
        assert!(!is_url_allowed("http://localhost:8080/", &[]));
    }

    #[test]
    fn multiple_patterns_match_any() {
        let patterns = vec!["192.168.178.*".to_string(), "https://api.open-meteo.com/*".to_string()];
        assert!(is_url_allowed("https://api.open-meteo.com/v1/forecast", &patterns));
    }
}
