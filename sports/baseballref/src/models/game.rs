use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Game metadata
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Game {
    pub id: i32,
    pub bbref_game_id: String,
    pub game_date: NaiveDate,
    pub start_time: Option<String>,
    pub venue: Option<String>,
    pub attendance: Option<i32>,
    pub duration_minutes: Option<i32>,
    pub weather: Option<String>,
    pub is_night_game: Option<bool>,
    pub is_artificial_turf: Option<bool>,
    pub home_team_id: i32,
    pub away_team_id: i32,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
    pub winning_pitcher_id: Option<i32>,
    pub losing_pitcher_id: Option<i32>,
    pub save_pitcher_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Game data for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGame {
    pub bbref_game_id: String,
    pub game_date: NaiveDate,
    pub start_time: Option<String>,
    pub venue: Option<String>,
    pub attendance: Option<i32>,
    pub duration_minutes: Option<i32>,
    pub weather: Option<String>,
    pub is_night_game: Option<bool>,
    pub is_artificial_turf: Option<bool>,
    pub home_team_id: i32,
    pub away_team_id: i32,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
    pub winning_pitcher_id: Option<i32>,
    pub losing_pitcher_id: Option<i32>,
    pub save_pitcher_id: Option<i32>,
}

/// Umpire assignment for a game
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameUmpire {
    pub id: i32,
    pub game_id: i32,
    pub position: String,
    pub name: String,
}

/// Umpire data for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGameUmpire {
    pub game_id: i32,
    pub position: String,
    pub name: String,
}

/// Line score entry (runs per inning per team)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GameLineScore {
    pub id: i32,
    pub game_id: i32,
    pub team_id: i32,
    pub is_home: bool,
    pub inning: i32,
    pub runs: i32,
}

/// Line score data for insertion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewGameLineScore {
    pub game_id: i32,
    pub team_id: i32,
    pub is_home: bool,
    pub inning: i32,
    pub runs: i32,
}
