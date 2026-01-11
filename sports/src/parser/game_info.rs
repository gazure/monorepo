use chrono::NaiveDate;
use scraper::{Html, Selector};

use super::{get_text, parse_int};

/// Parsed game metadata
#[derive(Debug, Clone)]
pub struct ParsedGameInfo {
    pub bbref_game_id: String,
    pub game_date: NaiveDate,
    pub start_time: Option<String>,
    pub venue: Option<String>,
    pub attendance: Option<i32>,
    pub duration_minutes: Option<i32>,
    pub weather: Option<String>,
    pub is_night_game: Option<bool>,
    pub is_artificial_turf: Option<bool>,
    pub home_team_code: String,
    pub home_team_name: String,
    pub away_team_code: String,
    pub away_team_name: String,
    pub home_score: i32,
    pub away_score: i32,
}

/// Parsed umpire info
#[derive(Debug, Clone)]
pub struct ParsedUmpire {
    pub position: String,
    pub name: String,
}

/// Parse game info from the HTML document
pub fn parse_game_info(
    doc: &Html,
    comments: &[Html],
    bbref_game_id: &str,
) -> Result<(ParsedGameInfo, Vec<ParsedUmpire>), String> {
    // Parse the h1 title to get date
    let h1_selector = Selector::parse("h1").map_err(|e| format!("Invalid selector: {e:?}"))?;
    let h1 = doc.select(&h1_selector).next().ok_or("Could not find h1 element")?;
    let title = get_text(h1);

    // Extract year from game ID (e.g., TOR202511010 -> 2025)
    let year = extract_year_from_game_id(bbref_game_id)?;

    // Parse date from title like "Los Angeles Dodgers vs Chicago Cubs Box Score: March 18, 2025"
    let game_date = parse_date_from_title(&title, year)?;

    // Parse scorebox for team info and scores
    let scorebox_selector = Selector::parse(".scorebox").map_err(|e| format!("Invalid selector: {e:?}"))?;
    let scorebox = doc.select(&scorebox_selector).next().ok_or("Could not find scorebox")?;

    // Get team links and scores
    let (away_team_code, away_team_name, away_score) = parse_team_from_scorebox(scorebox, 0)?;
    let (home_team_code, home_team_name, home_score) = parse_team_from_scorebox(scorebox, 1)?;

    // Parse scorebox metadata
    let meta_selector = Selector::parse(".scorebox_meta div").map_err(|e| format!("Invalid selector: {e:?}"))?;
    let mut start_time = None;
    let mut venue = None;
    let mut attendance = None;
    let mut duration_minutes = None;
    let mut is_night_game = None;
    let mut is_artificial_turf = None;

    for meta_div in scorebox.select(&meta_selector) {
        let text = get_text(meta_div);

        if text.contains("Start Time:") {
            start_time = Some(text.replace("Start Time:", "").trim().to_string());
        } else if text.contains("Attendance") {
            let att_text = text.replace("Attendance:", "").trim().to_string();
            attendance = parse_int(&att_text);
        } else if text.contains("Venue") {
            venue = Some(text.replace("Venue:", "").trim().to_string());
        } else if text.contains("Game Duration:") {
            duration_minutes = parse_duration(&text);
        } else if text.contains("Night Game") || text.contains("Day Game") {
            is_night_game = Some(text.contains("Night Game"));
            is_artificial_turf = Some(text.contains("artificial turf"));
        }
    }

    // Parse umpires from comments
    let umpires = parse_umpires(comments);

    // Parse weather from comments
    let weather = parse_weather(comments);

    let info = ParsedGameInfo {
        bbref_game_id: bbref_game_id.to_string(),
        game_date,
        start_time,
        venue,
        attendance,
        duration_minutes,
        weather,
        is_night_game,
        is_artificial_turf,
        home_team_code,
        home_team_name,
        away_team_code,
        away_team_name,
        home_score,
        away_score,
    };

    Ok((info, umpires))
}

fn extract_year_from_game_id(game_id: &str) -> Result<i32, String> {
    // Game ID format: TOR202511010 -> extract 2025
    if game_id.len() < 11 {
        return Err(format!("Invalid game ID format: {game_id}"));
    }
    let year_str = &game_id[3..7];
    year_str
        .parse()
        .map_err(|_| format!("Could not parse year from game ID: {game_id}"))
}

fn parse_date_from_title(title: &str, default_year: i32) -> Result<NaiveDate, String> {
    // Title format: "... Box Score: March 18, 2025" or "... vs ...: March 18, 2025"
    // or playoff format: "... Dodgers at Blue Jays, November  1" (no year)
    // Find the last colon or comma in the title
    let date_str = if let Some(colon_pos) = title.rfind(':') {
        title[colon_pos + 1..].trim()
    } else if let Some(comma_pos) = title.rfind(',') {
        title[comma_pos + 1..].trim()
    } else {
        return Err("Could not find date in title".to_string());
    };

    parse_date_string(date_str, default_year)
}

fn parse_date_string(date_str: &str, default_year: i32) -> Result<NaiveDate, String> {
    // Format: "March 18, 2025" or "March 18" (may have extra whitespace)
    // Normalize whitespace first
    let normalized = date_str.split_whitespace().collect::<Vec<_>>().join(" ");

    let months = [
        ("January", 1),
        ("February", 2),
        ("March", 3),
        ("April", 4),
        ("May", 5),
        ("June", 6),
        ("July", 7),
        ("August", 8),
        ("September", 9),
        ("October", 10),
        ("November", 11),
        ("December", 12),
    ];

    for (month_name, month_num) in months {
        if let Some(rest) = normalized.strip_prefix(month_name).map(str::trim) {
            let parts: Vec<&str> = rest.split(',').collect();
            // Try to parse day
            let day: u32 = parts[0].trim().parse().map_err(|_| "Invalid day".to_string())?;

            // Year is either in the string or we use the default
            let year = if parts.len() >= 2 {
                parts[1].trim().parse().map_err(|_| "Invalid year".to_string())?
            } else {
                default_year
            };

            return NaiveDate::from_ymd_opt(year, month_num, day).ok_or_else(|| "Invalid date".to_string());
        }
    }

    Err(format!("Could not parse date: {date_str}"))
}

fn parse_team_from_scorebox(scorebox: scraper::ElementRef<'_>, index: usize) -> Result<(String, String, i32), String> {
    let div_selector = Selector::parse(".scorebox > div").map_err(|e| format!("{e:?}"))?;
    let team_div = scorebox
        .select(&div_selector)
        .nth(index)
        .ok_or_else(|| format!("Could not find team div at index {index}"))?;

    // Get team link
    let link_selector = Selector::parse("strong a").map_err(|e| format!("{e:?}"))?;
    let team_link = team_div
        .select(&link_selector)
        .next()
        .ok_or("Could not find team link")?;

    let team_name = get_text(team_link);

    // Extract team code from href like "/teams/LAD/2025.shtml"
    let href = team_link.value().attr("href").unwrap_or("");
    let team_code = href
        .split("/teams/")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("")
        .to_string();

    // Get score
    let score_selector = Selector::parse(".score").map_err(|e| format!("{e:?}"))?;
    let score_elem = team_div.select(&score_selector).next().ok_or("Could not find score")?;
    let score: i32 = get_text(score_elem).parse().map_err(|_| "Invalid score".to_string())?;

    Ok((team_code, team_name, score))
}

fn parse_duration(text: &str) -> Option<i32> {
    // Format: "Game Duration: 2:38"
    let parts: Vec<&str> = text.split(':').collect();
    if parts.len() >= 3 {
        let hours: i32 = parts[1].trim().parse().ok()?;
        let minutes: i32 = parts[2].trim().parse().ok()?;
        return Some(hours * 60 + minutes);
    }
    None
}

fn parse_umpires(comments: &[Html]) -> Vec<ParsedUmpire> {
    let mut umpires = Vec::new();

    for comment_doc in comments {
        let text = comment_doc.root_element().text().collect::<String>();
        if text.contains("Umpires:") {
            // Parse format: "Umpires: HP - Bill Miller, 1B - Mike Estabrook, ..."
            for part in text.split(',') {
                let part = part.trim();
                // Look for pattern like "HP - Name" or "1B - Name"
                if let Some(dash_idx) = part.find(" - ") {
                    let position = part[..dash_idx].trim();
                    let name = part[dash_idx + 3..].trim();

                    // Clean up position (remove "Umpires:" prefix if present)
                    let position = position.replace("Umpires:", "").trim().to_string();

                    if !position.is_empty() && !name.is_empty() {
                        umpires.push(ParsedUmpire {
                            position,
                            name: name.to_string(),
                        });
                    }
                }
            }
            break;
        }
    }

    umpires
}

fn parse_weather(comments: &[Html]) -> Option<String> {
    for comment_doc in comments {
        let text = comment_doc.root_element().text().collect::<String>();
        if text.contains("Start Time Weather:") {
            // Extract the weather portion
            if let Some(idx) = text.find("Start Time Weather:") {
                let weather_part = &text[idx + "Start Time Weather:".len()..];
                // Take until next period or end
                let weather = weather_part.split('.').next().unwrap_or("").trim().to_string();
                if !weather.is_empty() {
                    return Some(weather);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_string() {
        let date = parse_date_string("March 18, 2025", 2025).unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 3, 18).unwrap());

        // Test with no year
        let date = parse_date_string("March 18", 2025).unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2025, 3, 18).unwrap());
    }
}
