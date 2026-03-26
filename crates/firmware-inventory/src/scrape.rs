use chrono::NaiveDateTime;
use regex::Regex;

/// An entry from an Apache-style HTML directory listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub last_modified: Option<NaiveDateTime>,
}

/// Extract directory entries (name + last-modified timestamp) from an Apache-style HTML directory
/// listing.
pub fn parse_directory_listing(html: &str) -> Vec<DirectoryEntry> {
    let re = Regex::new(
        r#"href="([^"]+)/"[^<]*</a>\s*</td>\s*<td[^>]*>\s*(\d{4}-\d{2}-\d{2} \d{2}:\d{2})\s*"#,
    )
    .unwrap();
    re.captures_iter(html)
        .filter(|c| {
            let name = &c[1];
            name != ".." && name != "."
        })
        .map(|c| DirectoryEntry {
            name: c[1].to_string(),
            last_modified: NaiveDateTime::parse_from_str(&c[2], "%Y-%m-%d %H:%M").ok(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directory_listing() {
        // Matches the actual HTML served by Apache mod_autoindex at
        // https://www.axis.com/ftp/pub/axis/software/MPQT/
        let html = r#"
<html><body><table>
<tr><th>Name</th><th>Last modified</th><th>Size</th><th>Description</th></tr>
<tr><td valign="top"><img src="/icons/back.gif" alt="[PARENTDIR]"></td><td><a href="/ftp/pub/axis/software/">Parent Directory</a></td><td>&nbsp;</td><td align="right">  - </td><td>&nbsp;</td></tr>
<tr><td valign="top"><img src="/icons/folder.gif" alt="[DIR]"></td><td><a href="M1075-L/">M1075-L/</a></td><td align="right">2026-01-22 02:34  </td><td align="right">  - </td><td>&nbsp;</td></tr>
<tr><td valign="top"><img src="/icons/folder.gif" alt="[DIR]"></td><td><a href="P1375/">P1375/</a></td><td align="right">2026-03-10 10:54  </td><td align="right">  - </td><td>&nbsp;</td></tr>
</table></body></html>
"#;
        let entries = parse_directory_listing(html);
        assert_eq!(
            entries,
            vec![
                DirectoryEntry {
                    name: "M1075-L".to_string(),
                    last_modified: Some(
                        NaiveDateTime::parse_from_str("2026-01-22 02:34", "%Y-%m-%d %H:%M")
                            .unwrap()
                    ),
                },
                DirectoryEntry {
                    name: "P1375".to_string(),
                    last_modified: Some(
                        NaiveDateTime::parse_from_str("2026-03-10 10:54", "%Y-%m-%d %H:%M")
                            .unwrap()
                    ),
                },
            ]
        );
    }
}
