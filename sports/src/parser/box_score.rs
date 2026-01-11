use std::path::Path;

use scraper::Html;
use thiserror::Error;

use super::{
    batting::{ParsedBattingLine, parse_batting_tables},
    extract_commented_html,
    game_info::{ParsedGameInfo, ParsedUmpire, parse_game_info},
    line_score::{ParsedLineScore, ParsedPitchingDecision, parse_line_score},
    pitching::{ParsedPitchingLine, parse_pitching_tables},
    play_by_play::{ParsedPlayByPlay, parse_play_by_play},
};

/// Errors that can occur during parsing
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Missing required data: {0}")]
    MissingData(String),
}

/// Complete parsed box score data
#[derive(Debug, Clone)]
pub struct BoxScore {
    pub game_info: ParsedGameInfo,
    pub umpires: Vec<ParsedUmpire>,
    pub away_line_score: ParsedLineScore,
    pub home_line_score: ParsedLineScore,
    pub pitching_decisions: Vec<ParsedPitchingDecision>,
    pub batting_lines: Vec<ParsedBattingLine>,
    pub pitching_lines: Vec<ParsedPitchingLine>,
    pub play_by_play: Vec<ParsedPlayByPlay>,
}

impl BoxScore {
    /// Parse a box score from an HTML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        let path = path.as_ref();

        // Extract game ID from filename (e.g., "CHN202503180.shtml" -> "CHN202503180")
        let game_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ParseError::MissingData("Could not extract game ID from filename".to_string()))?;

        let html = std::fs::read_to_string(path)?;
        Self::from_html(&html, game_id)
    }

    /// Parse a box score from HTML string
    pub fn from_html(html: &str, game_id: &str) -> Result<Self, ParseError> {
        let doc = Html::parse_document(html);

        // Extract commented HTML sections
        let comment_strings = extract_commented_html(html);
        let comments: Vec<Html> = comment_strings.iter().map(|s| Html::parse_fragment(s)).collect();

        // Parse game info
        let (game_info, umpires) = parse_game_info(&doc, &comments, game_id)
            .map_err(|e| ParseError::Parse(format!("Failed to parse game info: {e}")))?;

        // Parse line score
        let (away_line_score, home_line_score, pitching_decisions) =
            parse_line_score(&doc).map_err(|e| ParseError::Parse(format!("Failed to parse line score: {e}")))?;

        // Parse batting tables
        let batting_lines = parse_batting_tables(&doc, &comments, &game_info.away_team_code, &game_info.home_team_code)
            .map_err(|e| ParseError::Parse(format!("Failed to parse batting: {e}")))?;

        // Parse pitching tables
        let pitching_lines =
            parse_pitching_tables(&doc, &comments, &game_info.away_team_code, &game_info.home_team_code)
                .map_err(|e| ParseError::Parse(format!("Failed to parse pitching: {e}")))?;

        // Parse play-by-play
        let play_by_play = parse_play_by_play(&doc, &comments, &game_info.away_team_code, &game_info.home_team_code)
            .map_err(|e| ParseError::Parse(format!("Failed to parse play-by-play: {e}")))?;

        Ok(BoxScore {
            game_info,
            umpires,
            away_line_score,
            home_line_score,
            pitching_decisions,
            batting_lines,
            pitching_lines,
            play_by_play,
        })
    }

    /// Get a summary of the parsed data
    pub fn summary(&self) -> String {
        format!(
            "{} @ {} - {} ({}-{})\n\
             Date: {}\n\
             Venue: {}\n\
             Attendance: {}\n\
             Batting lines: {} away, {} home\n\
             Pitching lines: {} away, {} home\n\
             Play-by-play events: {}",
            self.game_info.away_team_name,
            self.game_info.home_team_name,
            self.game_info.bbref_game_id,
            self.game_info.away_score,
            self.game_info.home_score,
            self.game_info.game_date,
            self.game_info.venue.as_deref().unwrap_or("Unknown"),
            self.game_info
                .attendance
                .map_or_else(|| "Unknown".to_string(), |a| a.to_string()),
            self.batting_lines
                .iter()
                .filter(|b| b.team_code == self.game_info.away_team_code)
                .count(),
            self.batting_lines
                .iter()
                .filter(|b| b.team_code == self.game_info.home_team_code)
                .count(),
            self.pitching_lines
                .iter()
                .filter(|p| p.team_code == self.game_info.away_team_code)
                .count(),
            self.pitching_lines
                .iter()
                .filter(|p| p.team_code == self.game_info.home_team_code)
                .count(),
            self.play_by_play.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample_file() {
        let path = "data/bbref/CHN202503180.shtml";
        if !Path::new(path).exists() {
            eprintln!("Sample file not found, skipping test");
            return;
        }

        let box_score = BoxScore::from_file(path).expect("Failed to parse box score");

        // Verify basic game info
        assert_eq!(box_score.game_info.bbref_game_id, "CHN202503180");
        assert_eq!(box_score.game_info.away_team_code, "LAD");
        assert_eq!(box_score.game_info.home_team_code, "CHC");
        assert_eq!(box_score.game_info.away_score, 4);
        assert_eq!(box_score.game_info.home_score, 1);

        // Verify we got batting lines
        assert!(!box_score.batting_lines.is_empty());

        // Verify we got pitching lines
        assert!(!box_score.pitching_lines.is_empty());

        // Verify we got play-by-play
        assert!(!box_score.play_by_play.is_empty());

        println!("{}", box_score.summary());
    }
}
