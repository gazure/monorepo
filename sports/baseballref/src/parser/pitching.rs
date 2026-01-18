use rust_decimal::Decimal;
use scraper::{Html, Selector};

use super::{get_attr, get_text, parse_decimal, parse_int};

/// Parsed pitching line
#[derive(Debug, Clone)]
pub struct ParsedPitchingLine {
    pub player_bbref_id: String,
    pub player_name: String,
    pub team_code: String,
    pub pitch_order: i32,
    pub decision: Option<String>,
    pub ip: Option<Decimal>,
    pub h: Option<i32>,
    pub r: Option<i32>,
    pub er: Option<i32>,
    pub bb: Option<i32>,
    pub so: Option<i32>,
    pub hr: Option<i32>,
    pub era: Option<Decimal>,
    pub batters_faced: Option<i32>,
    pub pitches: Option<i32>,
    pub strikes: Option<i32>,
    pub strikes_contact: Option<i32>,
    pub strikes_swinging: Option<i32>,
    pub strikes_looking: Option<i32>,
    pub ground_balls: Option<i32>,
    pub fly_balls: Option<i32>,
    pub line_drives: Option<i32>,
    pub game_score: Option<i32>,
    pub inherited_runners: Option<i32>,
    pub inherited_scored: Option<i32>,
    pub wpa: Option<Decimal>,
    pub ali: Option<Decimal>,
    pub cwpa: Option<Decimal>,
    pub acli: Option<Decimal>,
    pub re24: Option<Decimal>,
}

/// Parse pitching tables for both teams
pub fn parse_pitching_tables(
    doc: &Html,
    comments: &[Html],
    away_team_code: &str,
    home_team_code: &str,
) -> Result<Vec<ParsedPitchingLine>, String> {
    let mut all_lines = Vec::new();

    // Find pitching tables in comments
    for comment_doc in comments {
        let table_selector = Selector::parse("table.stats_table").map_err(|e| format!("{e:?}"))?;

        for table in comment_doc.select(&table_selector) {
            let table_id = get_attr(table, "id").unwrap_or("");

            // Check if this is a pitching table
            if !table_id.to_lowercase().contains("pitching") {
                continue;
            }

            // Determine which team this is for
            let team_code = if table_id.to_lowercase().contains(&away_team_code.to_lowercase())
                || table_id.contains("Dodgers")
                || table_id.contains("visitor")
            {
                away_team_code
            } else {
                home_team_code
            };

            let lines = parse_pitching_table(table, team_code)?;
            all_lines.extend(lines);
        }
    }

    // Also check main document
    let table_selector = Selector::parse("table.stats_table").map_err(|e| format!("{e:?}"))?;
    for table in doc.select(&table_selector) {
        let table_id = get_attr(table, "id").unwrap_or("");
        if table_id.to_lowercase().contains("pitching") {
            let team_code = if table_id.to_lowercase().contains(&away_team_code.to_lowercase()) {
                away_team_code
            } else {
                home_team_code
            };
            let lines = parse_pitching_table(table, team_code)?;
            all_lines.extend(lines);
        }
    }

    Ok(all_lines)
}

fn parse_pitching_table(table: scraper::ElementRef<'_>, team_code: &str) -> Result<Vec<ParsedPitchingLine>, String> {
    let mut lines = Vec::new();

    let row_selector = Selector::parse("tbody tr").map_err(|e| format!("{e:?}"))?;
    let thead_selector = Selector::parse("th").map_err(|e| format!("{e:?}"))?;
    let tdata_selector = Selector::parse("td").map_err(|e| format!("{e:?}"))?;

    let mut pitch_order = 1;

    for row in table.select(&row_selector) {
        // Skip spacer rows
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
            None => continue,
        };

        // Parse player name and decision from th text
        // Format: "Yoshinobu Yamamoto, W (1-0)" or "Anthony Banda, H (1)"
        let (player_name, decision) = parse_pitcher_name_decision(&th_text);

        // Parse stats from td elements
        let cells: Vec<_> = row.select(&tdata_selector).collect();
        let mut line = ParsedPitchingLine {
            player_bbref_id,
            player_name,
            team_code: team_code.to_string(),
            pitch_order,
            decision,
            ip: None,
            h: None,
            r: None,
            er: None,
            bb: None,
            so: None,
            hr: None,
            era: None,
            batters_faced: None,
            pitches: None,
            strikes: None,
            strikes_contact: None,
            strikes_swinging: None,
            strikes_looking: None,
            ground_balls: None,
            fly_balls: None,
            line_drives: None,
            game_score: None,
            inherited_runners: None,
            inherited_scored: None,
            wpa: None,
            ali: None,
            cwpa: None,
            acli: None,
            re24: None,
        };

        pitch_order += 1;

        for cell in cells {
            let stat_name = get_attr(cell, "data-stat").unwrap_or("");
            let value = get_text(cell);

            match stat_name {
                "IP" => line.ip = parse_ip(&value),
                "H" => line.h = parse_int(&value),
                "R" => line.r = parse_int(&value),
                "ER" => line.er = parse_int(&value),
                "BB" => line.bb = parse_int(&value),
                "SO" => line.so = parse_int(&value),
                "HR" => line.hr = parse_int(&value),
                "earned_run_avg" => line.era = parse_decimal(&value),
                "batters_faced" => line.batters_faced = parse_int(&value),
                "pitches" => line.pitches = parse_int(&value),
                "strikes_total" => line.strikes = parse_int(&value),
                "strikes_contact" => line.strikes_contact = parse_int(&value),
                "strikes_swinging" => line.strikes_swinging = parse_int(&value),
                "strikes_looking" => line.strikes_looking = parse_int(&value),
                "inplay_gb_total" => line.ground_balls = parse_int(&value),
                "inplay_fb_total" => line.fly_balls = parse_int(&value),
                "inplay_ld" => line.line_drives = parse_int(&value),
                "game_score" => line.game_score = parse_int(&value),
                "inherited_runners" => line.inherited_runners = parse_int(&value),
                "inherited_score" => line.inherited_scored = parse_int(&value),
                "wpa_def" => line.wpa = parse_decimal(&value),
                "leverage_index_avg" => line.ali = parse_decimal(&value),
                "cwpa_def" => line.cwpa = parse_cwpa(&value),
                "cli_avg" => line.acli = parse_decimal(&value),
                "re24_def" => line.re24 = parse_decimal(&value),
                _ => {}
            }
        }

        lines.push(line);
    }

    Ok(lines)
}

fn parse_pitcher_name_decision(text: &str) -> (String, Option<String>) {
    let text = text.trim();

    // Format: "Yoshinobu Yamamoto, W (1-0)" or just "Shota Imanaga"
    if let Some(comma_idx) = text.find(',') {
        let name = text[..comma_idx].trim().to_string();
        let rest = text[comma_idx + 1..].trim();

        // Extract decision letter (W, L, S, H)
        let decision = if rest.starts_with("W ") || rest.starts_with("W(") {
            Some("W".to_string())
        } else if rest.starts_with("L ") || rest.starts_with("L(") {
            Some("L".to_string())
        } else if rest.starts_with("S ") || rest.starts_with("S(") {
            Some("S".to_string())
        } else if rest.starts_with("H ") || rest.starts_with("H(") {
            Some("H".to_string())
        } else {
            None
        };

        (name, decision)
    } else {
        (text.to_string(), None)
    }
}

fn parse_ip(value: &str) -> Option<Decimal> {
    // IP can be " 5 " or " 2.2" (2 and 2/3 innings)
    // Baseball uses .1 = 1/3, .2 = 2/3
    let cleaned = value.trim();
    if cleaned.is_empty() {
        return None;
    }

    // Handle fractional innings
    if let Some(dot_idx) = cleaned.find('.') {
        let whole: Decimal = cleaned[..dot_idx].trim().parse().ok()?;
        let frac_str = &cleaned[dot_idx + 1..];
        let frac: Decimal = match frac_str.trim() {
            "1" => Decimal::new(1, 1), // 0.1 representing 1/3
            "2" => Decimal::new(2, 1), // 0.2 representing 2/3
            _ => return None,
        };
        Some(whole + frac)
    } else {
        cleaned.parse().ok()
    }
}

fn parse_cwpa(value: &str) -> Option<Decimal> {
    let cleaned = value.trim().trim_end_matches('%').trim();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse().ok()
}
