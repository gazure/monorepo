use rust_decimal::Decimal;
use scraper::{Html, Selector};

use super::{get_attr, get_text, parse_int, parse_percentage};

/// Parsed play-by-play event
#[derive(Debug, Clone)]
pub struct ParsedPlayByPlay {
    pub event_num: i32,
    pub inning: i32,
    pub is_bottom: bool,
    pub batting_team_code: String,
    pub batter_name: String,
    pub batter_bbref_id: Option<String>,
    pub pitcher_name: String,
    pub pitcher_bbref_id: Option<String>,
    pub outs_before: Option<i32>,
    pub runners_before: Option<String>,
    pub score_batting_team: Option<i32>,
    pub score_fielding_team: Option<i32>,
    pub pitch_sequence: Option<String>,
    pub pitch_count: Option<i32>,
    pub runs_on_play: Option<i32>,
    pub outs_on_play: Option<i32>,
    pub wpa: Option<Decimal>,
    pub win_expectancy_after: Option<Decimal>,
    pub play_description: Option<String>,
}

/// Parse play-by-play table
pub fn parse_play_by_play(
    _doc: &Html,
    comments: &[Html],
    away_team_code: &str,
    home_team_code: &str,
) -> Result<Vec<ParsedPlayByPlay>, String> {
    let mut events = Vec::new();

    // Find play_by_play table in comments
    for comment_doc in comments {
        let table_selector = Selector::parse("#play_by_play").map_err(|e| format!("{e:?}"))?;

        if let Some(table) = comment_doc.select(&table_selector).next() {
            events = parse_pbp_table(table, away_team_code, home_team_code)?;
            break;
        }
    }

    Ok(events)
}

fn parse_pbp_table(
    table: scraper::ElementRef<'_>,
    away_team_code: &str,
    home_team_code: &str,
) -> Result<Vec<ParsedPlayByPlay>, String> {
    let mut events = Vec::new();

    let row_selector = Selector::parse("tbody tr").map_err(|e| format!("{e:?}"))?;
    let thead_selector = Selector::parse("th").map_err(|e| format!("{e:?}"))?;
    let tdata_selector = Selector::parse("td").map_err(|e| format!("{e:?}"))?;

    let mut event_num = 0;

    for row in table.select(&row_selector) {
        let class = get_attr(row, "class").unwrap_or("");

        // Skip summary rows
        if class.contains("pbp_summary") || class.contains("ingame_substitution") {
            continue;
        }

        // Get inning from first th
        let Some(th) = row.select(&thead_selector).next() else {
            continue;
        };

        let inning_text = get_text(th);
        if inning_text.is_empty() {
            continue;
        }

        // Parse inning: "t1" = top 1st, "b3" = bottom 3rd
        let (inning, is_bottom) = parse_inning(&inning_text)?;

        // Parse cells
        let cells: Vec<_> = row.select(&tdata_selector).collect();
        if cells.len() < 10 {
            continue;
        }

        event_num += 1;

        let mut event = ParsedPlayByPlay {
            event_num,
            inning,
            is_bottom,
            batting_team_code: String::new(),
            batter_name: String::new(),
            batter_bbref_id: None,
            pitcher_name: String::new(),
            pitcher_bbref_id: None,
            outs_before: None,
            runners_before: None,
            score_batting_team: None,
            score_fielding_team: None,
            pitch_sequence: None,
            pitch_count: None,
            runs_on_play: None,
            outs_on_play: None,
            wpa: None,
            win_expectancy_after: None,
            play_description: None,
        };

        for cell in cells {
            let stat_name = get_attr(cell, "data-stat").unwrap_or("");
            let value = get_text(cell);

            match stat_name {
                "score_batting_team" => {
                    // Format: "0-0" or "1-0"
                    let parts: Vec<&str> = value.split('-').collect();
                    if parts.len() == 2 {
                        event.score_batting_team = parse_int(parts[0]);
                        event.score_fielding_team = parse_int(parts[1]);
                    }
                }
                "outs" => {
                    event.outs_before = parse_int(&value);
                }
                "runners_on_bases_pbp" => {
                    if !value.is_empty() && value != "---" {
                        event.runners_before = Some(value);
                    }
                }
                "pitches_pbp" => {
                    // Format: "3,(1-1) CBX"
                    let (count, sequence) = parse_pitch_info(&value);
                    event.pitch_count = count;
                    event.pitch_sequence = sequence;
                }
                "runs_outs_result" => {
                    // Format: "R", "O", "RO", etc.
                    event.runs_on_play = Some(value.matches('R').count() as i32);
                    event.outs_on_play = Some(value.matches('O').count() as i32);
                }
                "batting_team_id" => {
                    // Determine team code
                    let team = if value.contains("LAD") || is_away_batting(&value, away_team_code) {
                        away_team_code
                    } else {
                        home_team_code
                    };
                    event.batting_team_code = team.to_string();
                }
                "batter" => {
                    event.batter_name = value.replace('\u{a0}', " ");
                }
                "pitcher" => {
                    event.pitcher_name = value.replace('\u{a0}', " ");
                }
                "win_probability_added" => {
                    // Format: "-2%" or "4%"
                    event.wpa = parse_percentage(&value);
                }
                "win_expectancy_post" => {
                    // Format: "48%"
                    if let Some(pct) = parse_percentage(&value) {
                        // Convert to decimal (48% -> 0.48)
                        event.win_expectancy_after = Some(pct / Decimal::from(100));
                    }
                }
                "play_desc" => {
                    if !value.is_empty() {
                        event.play_description = Some(value);
                    }
                }
                _ => {}
            }
        }

        // Only add events with valid data
        if !event.batter_name.is_empty() {
            events.push(event);
        }
    }

    Ok(events)
}

fn parse_inning(text: &str) -> Result<(i32, bool), String> {
    let text = text.trim().to_lowercase();

    let is_bottom = text.starts_with('b');
    let inning_str = text.trim_start_matches(['t', 'b']);

    let inning: i32 = inning_str.parse().map_err(|_| format!("Invalid inning: {text}"))?;

    Ok((inning, is_bottom))
}

fn parse_pitch_info(text: &str) -> (Option<i32>, Option<String>) {
    // Format: "3,(1-1) CBX" or "5,(3-2) BSBFBFB"
    let parts: Vec<&str> = text.splitn(2, ')').collect();

    let count = if text.is_empty() {
        None
    } else {
        text.chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse()
            .ok()
    };

    let sequence = if parts.len() == 2 {
        let seq = parts[1].trim();
        if seq.is_empty() { None } else { Some(seq.to_string()) }
    } else {
        None
    };

    (count, sequence)
}

fn is_away_batting(team_id: &str, away_code: &str) -> bool {
    team_id.to_uppercase().contains(&away_code.to_uppercase())
}
