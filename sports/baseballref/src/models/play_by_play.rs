use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Play-by-play event (one row per plate appearance)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlayByPlay {
    pub id: i32,
    pub game_id: i32,
    pub event_num: i32,
    pub inning: i32,
    pub is_bottom: bool,
    pub batting_team_id: i32,
    pub batter_id: i32,
    pub pitcher_id: i32,
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

/// Play-by-play data for insertion
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewPlayByPlay {
    pub game_id: i32,
    pub event_num: i32,
    pub inning: i32,
    pub is_bottom: bool,
    pub batting_team_id: i32,
    pub batter_id: i32,
    pub pitcher_id: i32,
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
