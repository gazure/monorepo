use std::path::Path;

use scraper::{Html, Selector};

/// A box score URL extracted from the schedule
#[derive(Debug, Clone)]
pub struct BoxScoreUrl {
    /// The game ID (e.g., "CHN202503180")
    pub game_id: String,
    /// The relative path (e.g., "/boxes/CHN/CHN202503180.shtml")
    pub path: String,
}

/// Extract all box score URLs from a schedule HTML file
pub fn extract_boxscore_urls(path: impl AsRef<Path>) -> Result<Vec<BoxScoreUrl>, std::io::Error> {
    let html = std::fs::read_to_string(path)?;
    Ok(extract_boxscore_urls_from_html(&html))
}

/// Extract all box score URLs from schedule HTML content
pub fn extract_boxscore_urls_from_html(html: &str) -> Vec<BoxScoreUrl> {
    let doc = Html::parse_document(html);
    let link_selector = Selector::parse("a[href]").expect("valid selector");

    let mut urls = Vec::new();

    for element in doc.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            // Match pattern like /boxes/CHN/CHN202503180.shtml
            if let Some(url) = parse_boxscore_href(href) {
                // Deduplicate (same game may appear multiple times)
                if !urls.iter().any(|u: &BoxScoreUrl| u.game_id == url.game_id) {
                    urls.push(url);
                }
            }
        }
    }

    urls
}

/// Parse a box score href and extract the game ID
fn parse_boxscore_href(href: &str) -> Option<BoxScoreUrl> {
    // Pattern: /boxes/XXX/XXX########.shtml
    // where XXX is the team code and ######## is the date + game number

    if !href.starts_with("/boxes/") {
        return None;
    }

    // Split path
    let parts: Vec<&str> = href.split('/').collect();
    if parts.len() != 4 {
        return None;
    }

    // parts[0] = "", parts[1] = "boxes", parts[2] = team code, parts[3] = filename
    let filename = parts[3];

    if Path::new(filename)
        .extension()
        .is_none_or(|ext| ext.eq_ignore_ascii_case("rs"))
    {
        return None;
    }

    let game_id = filename.trim_end_matches(".shtml");

    // Validate game ID format: 3 letter team code + 8 digits (YYYYMMDD) + game number
    if game_id.len() < 11 {
        return None;
    }

    let team_code = &game_id[..3];
    let rest = &game_id[3..];

    // Team code should be uppercase letters
    if !team_code.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }

    // Rest should be digits
    if !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    Some(BoxScoreUrl {
        game_id: game_id.to_string(),
        path: href.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_boxscore_href() {
        // Valid box score URLs
        let url = parse_boxscore_href("/boxes/CHN/CHN202503180.shtml");
        assert!(url.is_some());
        let url = url.unwrap();
        assert_eq!(url.game_id, "CHN202503180");
        assert_eq!(url.path, "/boxes/CHN/CHN202503180.shtml");

        // Double-header (game 2)
        let url = parse_boxscore_href("/boxes/NYA/NYA202507041.shtml");
        assert!(url.is_some());
        assert_eq!(url.unwrap().game_id, "NYA202507041");

        // Invalid URLs
        assert!(parse_boxscore_href("/boxes/").is_none());
        assert!(parse_boxscore_href("/boxes/?date=2025-03-18").is_none());
        assert!(parse_boxscore_href("/teams/CHC/2025.shtml").is_none());
    }

    #[test]
    fn test_extract_from_sample_html() {
        let html = r#"
            <html>
            <body>
                <a href="/boxes/CHN/CHN202503180.shtml">Box Score</a>
                <a href="/boxes/?date=2025-03-18">All games</a>
                <a href="/boxes/NYA/NYA202503270.shtml">Box Score</a>
                <a href="/boxes/CHN/CHN202503180.shtml">Duplicate</a>
            </body>
            </html>
        "#;

        let urls = extract_boxscore_urls_from_html(html);
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0].game_id, "CHN202503180");
        assert_eq!(urls[1].game_id, "NYA202503270");
    }
}
