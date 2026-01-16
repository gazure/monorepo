use scraper::{Html, Selector};

use super::{get_text, parse_int};

/// Parsed line score entry
#[derive(Debug, Clone)]
pub struct ParsedLineScore {
    pub team_code: String,
    pub is_home: bool,
    pub innings: Vec<i32>,
    pub total_runs: i32,
    pub total_hits: i32,
    pub total_errors: i32,
}

/// Parsed pitching decision (W/L/S)
#[derive(Debug, Clone)]
pub struct ParsedPitchingDecision {
    pub player_name: String,
    pub decision: String, // "W", "L", "S"
    pub record: String,   // e.g., "(1-0)", "(1)"
}

/// Parse the line score table
pub fn parse_line_score(doc: &Html) -> Result<(ParsedLineScore, ParsedLineScore, Vec<ParsedPitchingDecision>), String> {
    let table_selector = Selector::parse("table.linescore").map_err(|e| format!("Invalid selector: {e:?}"))?;
    let table = doc
        .select(&table_selector)
        .next()
        .ok_or("Could not find linescore table")?;

    let tbody_selector = Selector::parse("tbody tr").map_err(|e| format!("{e:?}"))?;
    let rows: Vec<_> = table.select(&tbody_selector).collect();

    if rows.len() < 2 {
        return Err("Expected at least 2 rows in linescore".to_string());
    }

    let away_line = parse_line_score_row(rows[0], false)?;
    let home_line = parse_line_score_row(rows[1], true)?;

    // Parse footer for W/L/S decisions
    let tfoot_selector = Selector::parse("tfoot td").map_err(|e| format!("{e:?}"))?;
    let decisions = if let Some(tfoot_td) = table.select(&tfoot_selector).next() {
        parse_pitching_decisions(&get_text(tfoot_td))
    } else {
        Vec::new()
    };

    Ok((away_line, home_line, decisions))
}

fn parse_line_score_row(row: scraper::ElementRef<'_>, is_home: bool) -> Result<ParsedLineScore, String> {
    let td_selector = Selector::parse("td").map_err(|e| format!("{e:?}"))?;
    let cells: Vec<_> = row.select(&td_selector).collect();

    // First cell is logo, second is team name/link
    if cells.len() < 4 {
        return Err("Not enough cells in linescore row".to_string());
    }

    // Get team code from link
    let link_selector = Selector::parse("a").map_err(|e| format!("{e:?}"))?;
    let team_code = cells[1]
        .select(&link_selector)
        .next()
        .and_then(|a| a.value().attr("href"))
        .and_then(|href| href.split("/teams/").nth(1))
        .and_then(|s| s.split('/').next())
        .unwrap_or("")
        .to_string();

    // Innings are cells 2 through n-3 (last 3 are R, H, E)
    let num_cells = cells.len();
    let mut innings = Vec::new();

    for cell in cells.iter().skip(2).take(num_cells - 5) {
        let runs = parse_int(&get_text(*cell)).unwrap_or(0);
        innings.push(runs);
    }

    // Last 3 cells are R, H, E
    let total_runs = parse_int(&get_text(cells[num_cells - 3])).unwrap_or(0);
    let total_hits = parse_int(&get_text(cells[num_cells - 2])).unwrap_or(0);
    let total_errors = parse_int(&get_text(cells[num_cells - 1])).unwrap_or(0);

    Ok(ParsedLineScore {
        team_code,
        is_home,
        innings,
        total_runs,
        total_hits,
        total_errors,
    })
}

fn parse_pitching_decisions(text: &str) -> Vec<ParsedPitchingDecision> {
    // Format: "WP: Yoshinobu Yamamoto (1-0) • LP: Ben Brown (0-1) • SV: Tanner Scott (1)"
    let mut decisions = Vec::new();

    for part in text.split('•') {
        let part = part.trim();

        let (decision, rest) = if part.starts_with("WP:") {
            ("W", part.strip_prefix("WP:").unwrap_or("").trim())
        } else if part.starts_with("LP:") {
            ("L", part.strip_prefix("LP:").unwrap_or("").trim())
        } else if part.starts_with("SV:") {
            ("S", part.strip_prefix("SV:").unwrap_or("").trim())
        } else {
            continue;
        };

        // Parse "Yoshinobu Yamamoto (1-0)" -> name and record
        if let Some(paren_idx) = rest.find('(') {
            let player_name = rest[..paren_idx].trim().replace('\u{a0}', " ");
            let record = rest[paren_idx..].trim().to_string();
            decisions.push(ParsedPitchingDecision {
                player_name,
                decision: decision.to_string(),
                record,
            });
        }
    }

    decisions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pitching_decisions() {
        let text = "WP: Yoshinobu Yamamoto (1-0) • LP: Ben Brown (0-1) • SV: Tanner Scott (1)".to_string();
        let decisions = parse_pitching_decisions(&text);

        assert_eq!(decisions.len(), 3);
        assert_eq!(decisions[0].player_name, "Yoshinobu Yamamoto");
        assert_eq!(decisions[0].decision, "W");
        assert_eq!(decisions[1].player_name, "Ben Brown");
        assert_eq!(decisions[1].decision, "L");
        assert_eq!(decisions[2].player_name, "Tanner Scott");
        assert_eq!(decisions[2].decision, "S");
    }
}
