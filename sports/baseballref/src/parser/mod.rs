mod batting;
mod box_score;
mod game_info;
mod line_score;
mod pitching;
mod play_by_play;

pub use box_score::{BoxScore, ParseError};
use scraper::Html;

/// Extract HTML content from comment nodes in the document.
/// Baseball Reference embeds tables in HTML comments for lazy loading.
pub fn extract_commented_html(html: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut in_comment = false;
    let mut comment_content = String::new();
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        if in_comment {
            // Look for end of comment: -->
            if c == '-' && chars.peek() == Some(&'-') && chars.clone().nth(1) == Some('>') {
                chars.next(); // -
                chars.next(); // >
                in_comment = false;

                // Only keep comments that contain HTML (have tags)
                let trimmed = comment_content.trim();
                if trimmed.contains('<') && trimmed.contains('>') {
                    results.push(trimmed.to_string());
                }
            } else {
                comment_content.push(c);
            }
        } else {
            // Look for start of comment: <!--
            if c == '<'
                && chars.peek() == Some(&'!')
                && chars.clone().nth(1) == Some('-')
                && chars.clone().nth(2) == Some('-')
            {
                chars.next(); // !
                chars.next(); // -
                chars.next(); // -
                in_comment = true;
                comment_content.clear();
            }
        }
    }

    results
}

/// Parse HTML and find an element by its ID within the main document or extracted comments
pub fn find_table_by_id<'a>(doc: &'a Html, comments: &'a [Html], id: &str) -> Option<&'a Html> {
    let selector = scraper::Selector::parse(&format!("#{id}")).ok()?;

    // First check main document
    if doc.select(&selector).next().is_some() {
        return Some(doc);
    }

    // Then check comments
    comments
        .iter()
        .find(|&comment_doc| comment_doc.select(&selector).next().is_some())
        .map(|v| v as _)
}

/// Get text content from an element, trimmed
pub fn get_text(element: scraper::ElementRef<'_>) -> String {
    element.text().collect::<Vec<_>>().join("").trim().to_string()
}

/// Get an attribute value from an element
pub fn get_attr<'a>(element: scraper::ElementRef<'a>, attr: &str) -> Option<&'a str> {
    element.value().attr(attr)
}

/// Parse an integer from text, returning None if parsing fails or text is empty
pub fn parse_int(text: &str) -> Option<i32> {
    let cleaned = text.trim().replace(',', "");
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse().ok()
}

/// Parse a decimal from text (e.g., ".400", "1.80")
pub fn parse_decimal(text: &str) -> Option<rust_decimal::Decimal> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse().ok()
}

/// Parse a percentage like "2%" or "-2%" into a decimal
pub fn parse_percentage(text: &str) -> Option<rust_decimal::Decimal> {
    let cleaned = text.trim().trim_end_matches('%');
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse().ok()
}
