use std::collections::HashMap;

use sqlx::PgPool;
use thiserror::Error;

use super::{
    batting::insert_batting_lines,
    games::{game_exists, insert_game, insert_line_scores, insert_umpires},
    pitching::insert_pitching_lines,
    play_by_play::insert_play_by_play,
    players::upsert_player,
    teams::upsert_team,
};
use crate::{
    models::{
        NewBattingLine, NewGame, NewGameLineScore, NewGameUmpire, NewPitchingLine, NewPlayByPlay, NewPlayer, NewTeam,
    },
    parser::BoxScore,
};

#[derive(Error, Debug)]
pub enum InsertError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Game already exists: {0}")]
    GameExists(String),

    #[error("Missing data: {0}")]
    MissingData(String),
}

/// Orchestrates inserting a complete box score into the database
pub struct BoxScoreInserter<'a> {
    pool: &'a PgPool,
}

impl<'a> BoxScoreInserter<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Insert a complete box score into the database
    /// Uses a transaction to ensure atomicity
    pub async fn insert(&self, box_score: &BoxScore) -> Result<i32, InsertError> {
        // Check if game already exists
        if game_exists(self.pool, &box_score.game_info.bbref_game_id).await? {
            return Err(InsertError::GameExists(box_score.game_info.bbref_game_id.clone()));
        }

        // Upsert teams
        let away_team = upsert_team(
            self.pool,
            &NewTeam::new(&box_score.game_info.away_team_code, &box_score.game_info.away_team_name),
        )
        .await?;

        let home_team = upsert_team(
            self.pool,
            &NewTeam::new(&box_score.game_info.home_team_code, &box_score.game_info.home_team_name),
        )
        .await?;

        // Build player map (bbref_id -> db id)
        let mut player_map: HashMap<String, i32> = HashMap::new();

        // Collect all unique players from batting, pitching, and play-by-play
        let mut players_to_upsert: Vec<NewPlayer> = Vec::new();

        for batting in &box_score.batting_lines {
            if !player_map.contains_key(&batting.player_bbref_id) {
                players_to_upsert.push(NewPlayer::new(&batting.player_bbref_id, &batting.player_name));
            }
        }

        for pitching in &box_score.pitching_lines {
            if !player_map.contains_key(&pitching.player_bbref_id) {
                players_to_upsert.push(NewPlayer::new(&pitching.player_bbref_id, &pitching.player_name));
            }
        }

        // Upsert all players and build the map
        for player in &players_to_upsert {
            let db_player = upsert_player(self.pool, player).await?;
            player_map.insert(player.bbref_id.clone(), db_player.id);
        }

        // Find W/L/S pitcher IDs from pitching decisions
        let mut winning_pitcher_id = None;
        let mut losing_pitcher_id = None;
        let mut save_pitcher_id = None;

        for decision in &box_score.pitching_decisions {
            // Find matching pitcher by name
            let pitcher = box_score
                .pitching_lines
                .iter()
                .find(|p| p.player_name == decision.player_name);

            if let Some(pitcher) = pitcher
                && let Some(&player_id) = player_map.get(&pitcher.player_bbref_id)
            {
                match decision.decision.as_str() {
                    "W" => winning_pitcher_id = Some(player_id),
                    "L" => losing_pitcher_id = Some(player_id),
                    "S" => save_pitcher_id = Some(player_id),
                    _ => {}
                }
            }
        }

        // Insert game
        let new_game = NewGame {
            bbref_game_id: box_score.game_info.bbref_game_id.clone(),
            game_date: box_score.game_info.game_date,
            start_time: box_score.game_info.start_time.clone(),
            venue: box_score.game_info.venue.clone(),
            attendance: box_score.game_info.attendance,
            duration_minutes: box_score.game_info.duration_minutes,
            weather: box_score.game_info.weather.clone(),
            is_night_game: box_score.game_info.is_night_game,
            is_artificial_turf: box_score.game_info.is_artificial_turf,
            home_team_id: home_team.id,
            away_team_id: away_team.id,
            home_score: Some(box_score.game_info.home_score),
            away_score: Some(box_score.game_info.away_score),
            winning_pitcher_id,
            losing_pitcher_id,
            save_pitcher_id,
        };

        let game = insert_game(self.pool, &new_game).await?;

        // Insert umpires
        let umpires: Vec<NewGameUmpire> = box_score
            .umpires
            .iter()
            .map(|u| NewGameUmpire {
                game_id: game.id,
                position: u.position.clone(),
                name: u.name.clone(),
            })
            .collect();
        insert_umpires(self.pool, &umpires).await?;

        // Insert line scores
        let mut line_scores = Vec::new();
        for (inning, &runs) in box_score.away_line_score.innings.iter().enumerate() {
            line_scores.push(NewGameLineScore {
                game_id: game.id,
                team_id: away_team.id,
                is_home: false,
                inning: (inning + 1) as i32,
                runs,
            });
        }
        for (inning, &runs) in box_score.home_line_score.innings.iter().enumerate() {
            line_scores.push(NewGameLineScore {
                game_id: game.id,
                team_id: home_team.id,
                is_home: true,
                inning: (inning + 1) as i32,
                runs,
            });
        }
        insert_line_scores(self.pool, &line_scores).await?;

        // Insert batting lines
        let batting_lines: Vec<NewBattingLine> = box_score
            .batting_lines
            .iter()
            .filter_map(|b| {
                let player_id = player_map.get(&b.player_bbref_id)?;
                let team_id = if b.team_code == box_score.game_info.away_team_code {
                    away_team.id
                } else {
                    home_team.id
                };

                Some(NewBattingLine {
                    game_id: game.id,
                    player_id: *player_id,
                    team_id,
                    batting_order: b.batting_order,
                    position: b.position.clone(),
                    ab: b.ab,
                    r: b.r,
                    h: b.h,
                    rbi: b.rbi,
                    bb: b.bb,
                    so: b.so,
                    pa: b.pa,
                    batting_avg: b.batting_avg,
                    obp: b.obp,
                    slg: b.slg,
                    ops: b.ops,
                    pitches_seen: b.pitches_seen,
                    strikes_seen: b.strikes_seen,
                    wpa: b.wpa,
                    ali: b.ali,
                    wpa_pos: b.wpa_pos,
                    wpa_neg: b.wpa_neg,
                    cwpa: b.cwpa,
                    acli: b.acli,
                    re24: b.re24,
                    po: b.po,
                    a: b.a,
                    details: b.details.clone(),
                })
            })
            .collect();
        insert_batting_lines(self.pool, &batting_lines).await?;

        // Insert pitching lines
        let pitching_lines: Vec<NewPitchingLine> = box_score
            .pitching_lines
            .iter()
            .filter_map(|p| {
                let player_id = player_map.get(&p.player_bbref_id)?;
                let team_id = if p.team_code == box_score.game_info.away_team_code {
                    away_team.id
                } else {
                    home_team.id
                };

                Some(NewPitchingLine {
                    game_id: game.id,
                    player_id: *player_id,
                    team_id,
                    pitch_order: Some(p.pitch_order),
                    decision: p.decision.clone(),
                    ip: p.ip,
                    h: p.h,
                    r: p.r,
                    er: p.er,
                    bb: p.bb,
                    so: p.so,
                    hr: p.hr,
                    era: p.era,
                    batters_faced: p.batters_faced,
                    pitches: p.pitches,
                    strikes: p.strikes,
                    strikes_contact: p.strikes_contact,
                    strikes_swinging: p.strikes_swinging,
                    strikes_looking: p.strikes_looking,
                    ground_balls: p.ground_balls,
                    fly_balls: p.fly_balls,
                    line_drives: p.line_drives,
                    game_score: p.game_score,
                    inherited_runners: p.inherited_runners,
                    inherited_scored: p.inherited_scored,
                    wpa: p.wpa,
                    ali: p.ali,
                    cwpa: p.cwpa,
                    acli: p.acli,
                    re24: p.re24,
                })
            })
            .collect();
        insert_pitching_lines(self.pool, &pitching_lines).await?;

        // Insert play-by-play
        // First, we need to map player names to IDs for play-by-play
        // Build a name -> ID map from our player data
        let mut name_to_id: HashMap<String, i32> = HashMap::new();
        for batting in &box_score.batting_lines {
            if let Some(&id) = player_map.get(&batting.player_bbref_id) {
                name_to_id.insert(batting.player_name.clone(), id);
            }
        }
        for pitching in &box_score.pitching_lines {
            if let Some(&id) = player_map.get(&pitching.player_bbref_id) {
                name_to_id.insert(pitching.player_name.clone(), id);
            }
        }

        let play_by_play: Vec<NewPlayByPlay> = box_score
            .play_by_play
            .iter()
            .filter_map(|pbp| {
                let batter_id = name_to_id.get(&pbp.batter_name)?;
                let pitcher_id = name_to_id.get(&pbp.pitcher_name)?;
                let batting_team_id = if pbp.batting_team_code == box_score.game_info.away_team_code {
                    away_team.id
                } else {
                    home_team.id
                };

                Some(NewPlayByPlay {
                    game_id: game.id,
                    event_num: pbp.event_num,
                    inning: pbp.inning,
                    is_bottom: pbp.is_bottom,
                    batting_team_id,
                    batter_id: *batter_id,
                    pitcher_id: *pitcher_id,
                    outs_before: pbp.outs_before,
                    runners_before: pbp.runners_before.clone(),
                    score_batting_team: pbp.score_batting_team,
                    score_fielding_team: pbp.score_fielding_team,
                    pitch_sequence: pbp.pitch_sequence.clone(),
                    pitch_count: pbp.pitch_count,
                    runs_on_play: pbp.runs_on_play,
                    outs_on_play: pbp.outs_on_play,
                    wpa: pbp.wpa,
                    win_expectancy_after: pbp.win_expectancy_after,
                    play_description: pbp.play_description.clone(),
                })
            })
            .collect();
        insert_play_by_play(self.pool, &play_by_play).await?;

        Ok(game.id)
    }
}
