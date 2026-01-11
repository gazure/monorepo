use rust_decimal::Decimal;
use scraper::{Html, Selector};

use super::{get_attr, get_text, parse_decimal, parse_int};

/// Parsed batting line
#[derive(Debug, Clone)]
pub struct ParsedBattingLine {
    pub player_bbref_id: String,
    pub player_name: String,
    pub team_code: String,
    pub batting_order: Option<i32>,
    pub position: Option<String>,
    pub ab: Option<i32>,
    pub r: Option<i32>,
    pub h: Option<i32>,
    pub rbi: Option<i32>,
    pub bb: Option<i32>,
    pub so: Option<i32>,
    pub pa: Option<i32>,
    pub batting_avg: Option<Decimal>,
    pub obp: Option<Decimal>,
    pub slg: Option<Decimal>,
    pub ops: Option<Decimal>,
    pub pitches_seen: Option<i32>,
    pub strikes_seen: Option<i32>,
    pub wpa: Option<Decimal>,
    pub ali: Option<Decimal>,
    pub wpa_pos: Option<Decimal>,
    pub wpa_neg: Option<Decimal>,
    pub cwpa: Option<Decimal>,
    pub acli: Option<Decimal>,
    pub re24: Option<Decimal>,
    pub po: Option<i32>,
    pub a: Option<i32>,
    pub details: Option<String>,
}

/// Parse batting tables for both teams
pub fn parse_batting_tables(
    doc: &Html,
    comments: &[Html],
    away_team_code: &str,
    home_team_code: &str,
) -> Result<Vec<ParsedBattingLine>, String> {
    let mut all_lines = Vec::new();

    // Find batting tables - they're in comments with IDs like "LosAngelesDodgersbatting"
    // We need to search by partial ID match
    for comment_doc in comments {
        let table_selector = Selector::parse("table.stats_table").map_err(|e| format!("{e:?}"))?;

        for table in comment_doc.select(&table_selector) {
            let table_id = get_attr(table, "id").unwrap_or("");

            // Check if this is a batting table
            if !table_id.to_lowercase().contains("batting") {
                continue;
            }

            // Determine which team this is for based on table ID
            let team_code = if table_id.to_lowercase().contains(&away_team_code.to_lowercase())
                || table_id.contains("Dodgers")
                || table_id.contains("visitor")
            {
                away_team_code
            } else {
                home_team_code
            };

            let lines = parse_batting_table(table, team_code)?;
            all_lines.extend(lines);
        }
    }

    // Also check main document
    let table_selector = Selector::parse("table.stats_table").map_err(|e| format!("{e:?}"))?;
    for table in doc.select(&table_selector) {
        let table_id = get_attr(table, "id").unwrap_or("");
        if table_id.to_lowercase().contains("batting") {
            let team_code = if table_id.to_lowercase().contains(&away_team_code.to_lowercase()) {
                away_team_code
            } else {
                home_team_code
            };
            let lines = parse_batting_table(table, team_code)?;
            all_lines.extend(lines);
        }
    }

    Ok(all_lines)
}

fn parse_batting_table(table: scraper::ElementRef<'_>, team_code: &str) -> Result<Vec<ParsedBattingLine>, String> {
    let mut lines = Vec::new();

    let row_selector = Selector::parse("tbody tr").map_err(|e| format!("{e:?}"))?;
    let thead_selector = Selector::parse("th").map_err(|e| format!("{e:?}"))?;
    let tdata_selector = Selector::parse("td").map_err(|e| format!("{e:?}"))?;

    let mut batting_order = 1;

    for row in table.select(&row_selector) {
        // Skip spacer rows and total rows
        let class = get_attr(row, "class").unwrap_or("");
        if class.contains("spacer") || class.contains("thead") {
            continue;
        }

        // Get player info from th
        let Some(th) = row.select(&thead_selector).next() else {
            continue;
        };

        // Check if this is a totals row
        let th_text = get_text(th);
        if th_text.contains("Team Totals") || th_text.is_empty() {
            continue;
        }

        // Get player bbref_id from data-append-csv attribute
        let player_bbref_id = match get_attr(th, "data-append-csv") {
            Some(id) => id.to_string(),
            None => continue, // Skip rows without player ID
        };

        // Parse player name and position from th text
        // Format: "Shohei Ohtani DH" or "   Anthony Banda P" (indented for relievers)
        let (player_name, position) = parse_player_name_position(&th_text);

        // Skip pitcher rows in batting table (they have position "P" and no batting stats)
        // This happens when a player both pitches and bats (like Shohei Ohtani)
        if position.as_deref() == Some("P") {
            continue;
        }

        // Assign batting order for position players
        let current_order = Some(batting_order);
        batting_order += 1;

        // Parse stats from td elements
        let cells: Vec<_> = row.select(&tdata_selector).collect();
        let mut line = ParsedBattingLine {
            player_bbref_id,
            player_name,
            team_code: team_code.to_string(),
            batting_order: current_order,
            position,
            ab: None,
            r: None,
            h: None,
            rbi: None,
            bb: None,
            so: None,
            pa: None,
            batting_avg: None,
            obp: None,
            slg: None,
            ops: None,
            pitches_seen: None,
            strikes_seen: None,
            wpa: None,
            ali: None,
            wpa_pos: None,
            wpa_neg: None,
            cwpa: None,
            acli: None,
            re24: None,
            po: None,
            a: None,
            details: None,
        };

        for cell in cells {
            let stat_name = get_attr(cell, "data-stat").unwrap_or("");
            let value = get_text(cell);

            match stat_name {
                "AB" => line.ab = parse_int(&value),
                "R" => line.r = parse_int(&value),
                "H" => line.h = parse_int(&value),
                "RBI" => line.rbi = parse_int(&value),
                "BB" => line.bb = parse_int(&value),
                "SO" => line.so = parse_int(&value),
                "PA" => line.pa = parse_int(&value),
                "batting_avg" => line.batting_avg = parse_decimal(&value),
                "onbase_perc" => line.obp = parse_decimal(&value),
                "slugging_perc" => line.slg = parse_decimal(&value),
                "onbase_plus_slugging" => line.ops = parse_decimal(&value),
                "pitches" => line.pitches_seen = parse_int(&value),
                "strikes_total" => line.strikes_seen = parse_int(&value),
                "wpa_bat" => line.wpa = parse_decimal(&value),
                "leverage_index_avg" => line.ali = parse_decimal(&value),
                "wpa_bat_pos" => line.wpa_pos = parse_decimal(&value),
                "wpa_bat_neg" => line.wpa_neg = parse_decimal(&value),
                "cwpa_bat" => line.cwpa = parse_cwpa(&value),
                "cli_avg" => line.acli = parse_decimal(&value),
                "re24_bat" => line.re24 = parse_decimal(&value),
                "PO" => line.po = parse_int(&value),
                "A" => line.a = parse_int(&value),
                "details" => {
                    if !value.is_empty() {
                        line.details = Some(value);
                    }
                }
                _ => {}
            }
        }

        lines.push(line);
    }

    Ok(lines)
}

fn parse_player_name_position(text: &str) -> (String, Option<String>) {
    let text = text.trim();

    // Common positions
    let positions = ["DH", "C", "1B", "2B", "3B", "SS", "LF", "CF", "RF", "P", "PH", "PR"];

    for pos in positions {
        if text.ends_with(&format!(" {pos}")) {
            let name = text[..text.len() - pos.len() - 1].trim().to_string();
            return (name, Some(pos.to_string()));
        }
    }

    // No position found
    (text.to_string(), None)
}

fn parse_cwpa(value: &str) -> Option<Decimal> {
    // cWPA is formatted like " 0.02%" or "-0.08%"
    let cleaned = value.trim().trim_end_matches('%').trim();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse().ok()
}
