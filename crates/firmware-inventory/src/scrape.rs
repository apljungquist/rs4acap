use regex::Regex;

/// Extract directory names from an Apache-style HTML directory listing.
pub fn parse_directory_listing(html: &str) -> Vec<String> {
    let re = Regex::new(r#"href="([^"]+)/""#).unwrap();
    re.captures_iter(html)
        .map(|c| c[1].to_string())
        .filter(|s| s != ".." && s != ".")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directory_listing() {
        let html = r#"
<html><body>
<a href="../">Parent Directory</a>
<a href="M1075-L/">M1075-L/</a>
<a href="P1375/">P1375/</a>
</body></html>
"#;
        let dirs = parse_directory_listing(html);
        assert_eq!(dirs, vec!["M1075-L", "P1375"]);
    }
}
