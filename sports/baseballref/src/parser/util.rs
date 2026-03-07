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

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use scraper::Html;

    use super::*;

    #[test]
    fn extract_commented_html_finds_html_comments() {
        let html = r#"<html><!-- <table id="t1"><tr><td>data</td></tr></table> --></html>"#;
        let results = extract_commented_html(html);
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("<table"));
    }

    #[test]
    fn extract_commented_html_skips_non_html_comments() {
        let html = r"<html><!-- just a plain text comment --></html>";
        let results = extract_commented_html(html);
        assert!(results.is_empty());
    }

    #[test]
    fn extract_commented_html_multiple_comments() {
        let html = r"
            <!-- <div>first</div> -->
            <p>visible</p>
            <!-- <div>second</div> -->
        ";
        let results = extract_commented_html(html);
        assert_eq!(results.len(), 2);
        assert!(results[0].contains("first"));
        assert!(results[1].contains("second"));
    }

    #[test]
    fn extract_commented_html_empty_input() {
        assert!(extract_commented_html("").is_empty());
    }

    #[test]
    fn extract_commented_html_no_comments() {
        assert!(extract_commented_html("<html><body>hi</body></html>").is_empty());
    }

    #[test]
    fn get_text_trims_whitespace() {
        let html = Html::parse_fragment("<span>  hello world  </span>");
        let selector = scraper::Selector::parse("span").unwrap();
        let element = html.select(&selector).next().unwrap();
        assert_eq!(get_text(element), "hello world");
    }

    #[test]
    fn get_text_joins_nested_text() {
        let html = Html::parse_fragment("<span>hello <em>world</em></span>");
        let selector = scraper::Selector::parse("span").unwrap();
        let element = html.select(&selector).next().unwrap();
        assert_eq!(get_text(element), "hello world");
    }

    #[test]
    fn get_attr_returns_value() {
        let html = Html::parse_fragment(r#"<table><tr><td data-stat="HR">5</td></tr></table>"#);
        let selector = scraper::Selector::parse("td").unwrap();
        let element = html.select(&selector).next().unwrap();
        assert_eq!(get_attr(element, "data-stat"), Some("HR"));
    }

    #[test]
    fn get_attr_returns_none_for_missing() {
        let html = Html::parse_fragment("<table><tr><td>5</td></tr></table>");
        let selector = scraper::Selector::parse("td").unwrap();
        let element = html.select(&selector).next().unwrap();
        assert_eq!(get_attr(element, "data-stat"), None);
    }

    #[test]
    fn parse_int_basic() {
        assert_eq!(parse_int("42"), Some(42));
        assert_eq!(parse_int("-3"), Some(-3));
        assert_eq!(parse_int("0"), Some(0));
    }

    #[test]
    fn parse_int_with_commas() {
        assert_eq!(parse_int("1,234"), Some(1234));
        assert_eq!(parse_int("42,000"), Some(42000));
    }

    #[test]
    fn parse_int_with_whitespace() {
        assert_eq!(parse_int("  42  "), Some(42));
    }

    #[test]
    fn parse_int_empty_and_invalid() {
        assert_eq!(parse_int(""), None);
        assert_eq!(parse_int("  "), None);
        assert_eq!(parse_int("abc"), None);
    }

    #[test]
    fn parse_decimal_basic() {
        assert_eq!(parse_decimal(".400"), Some(Decimal::new(400, 3)));
        assert_eq!(parse_decimal("1.80"), Some(Decimal::new(180, 2)));
        assert_eq!(parse_decimal("0"), Some(Decimal::new(0, 0)));
    }

    #[test]
    fn parse_decimal_negative() {
        assert_eq!(parse_decimal("-0.25"), Some(Decimal::new(-25, 2)));
    }

    #[test]
    fn parse_decimal_empty_and_invalid() {
        assert_eq!(parse_decimal(""), None);
        assert_eq!(parse_decimal("  "), None);
        assert_eq!(parse_decimal("abc"), None);
    }

    #[test]
    fn parse_percentage_basic() {
        assert_eq!(parse_percentage("2%"), Some(Decimal::new(2, 0)));
        assert_eq!(parse_percentage("50%"), Some(Decimal::new(50, 0)));
    }

    #[test]
    fn parse_percentage_negative() {
        assert_eq!(parse_percentage("-2%"), Some(Decimal::new(-2, 0)));
    }

    #[test]
    fn parse_percentage_with_whitespace() {
        assert_eq!(parse_percentage("  5%  "), Some(Decimal::new(5, 0)));
    }

    #[test]
    fn parse_percentage_empty_and_invalid() {
        assert_eq!(parse_percentage(""), None);
        assert_eq!(parse_percentage("%"), None);
        assert_eq!(parse_percentage("abc%"), None);
    }
}
