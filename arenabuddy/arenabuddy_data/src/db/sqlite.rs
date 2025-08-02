// UNUSED
use std::sync::LazyLock;

use arenabuddy_core::{
    cards::CardsDatabase,
    models::{Deck, MTGAMatch, MTGAMatchBuilder, MatchResult, MatchResultBuilder, Mulligan},
    replay::MatchReplay,
};
use chrono::{DateTime, Utc};
use include_dir::{Dir, include_dir};
use indoc::indoc;
use rusqlite::{Connection, Params as RusqliteParams, Result as RusqliteResult, Transaction};
use rusqlite_migration::Migrations;
use tracing::{debug, error, info};

use crate::{MatchDBError, Result, Storage};

static MIGRATIONS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/migrations/sqlite");
static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::from_directory(&MIGRATIONS_DIR).unwrap_or(Migrations::new(Vec::new()))
});

#[derive(Debug)]
pub struct MatchDB {
    pub conn: Connection,
    pub cards_database: CardsDatabase,
}

impl MatchDB {
    pub fn new(conn: Connection, cards_database: CardsDatabase) -> Self {
        Self {
            conn,
            cards_database,
        }
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    /// or if the migrations cannot be applied
    pub fn init(&mut self) -> Result<()> {
        MIGRATIONS.to_latest(&mut self.conn)?;
        Ok(())
    }

    /// # Errors
    ///
    /// passes along errors from Rusqlite
    pub fn execute(&mut self, query: &str, params: impl RusqliteParams) -> RusqliteResult<usize> {
        self.conn.execute(query, params)
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    fn insert_match(mtga_match: &MTGAMatch, tx: &Transaction) -> Result<()> {
        let params = (
            mtga_match.id(),
            mtga_match.controller_seat_id(),
            mtga_match.controller_player_name(),
            mtga_match.opponent_player_name(),
            mtga_match.created_at(),
        );

        let sql = indoc! {r"INSERT INTO matches
        (id, controller_seat_id, controller_player_name, opponent_player_name, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(id) DO NOTHING"};
        tx.execute(sql, params)?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    fn insert_deck(match_id: &str, deck: &Deck, tx: &Transaction) -> Result<()> {
        let deck_string = serde_json::to_string(deck.mainboard())?;
        let sideboard_string = serde_json::to_string(deck.sideboard())?;

        tx.execute(indoc! {r"INSERT INTO decks
                (match_id, game_number, deck_cards, sideboard_cards)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT (match_id, game_number)
                DO UPDATE SET deck_cards = excluded.deck_cards, sideboard_cards = excluded.sideboard_cards"},
            (match_id, deck.game_number(), deck_string, sideboard_string)
        )?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    fn insert_mulligan_info(mulligan_info: &Mulligan, tx: &Transaction) -> Result<()> {
        tx.execute(
            indoc!{r"INSERT INTO mulligans (match_id, game_number, number_to_keep, hand, play_draw, opponent_identity, decision)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (match_id, game_number, number_to_keep)
             DO UPDATE SET hand = excluded.hand, play_draw = excluded.play_draw, opponent_identity = excluded.opponent_identity, decision = excluded.decision"},
            (
                mulligan_info.match_id(),
                mulligan_info.game_number(),
                mulligan_info.number_to_keep(),
                mulligan_info.hand(),
                mulligan_info.play_draw(),
                mulligan_info.opponent_identity(),
                mulligan_info.decision(),
            ),
        )?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    fn insert_match_result(match_result: &MatchResult, tx: &Transaction) -> Result<()> {
        let params = (
            match_result.match_id(),
            match_result.game_number(),
            match_result.winning_team_id(),
            match_result.result_scope(),
        );

        let sql = indoc! {r"INSERT INTO match_results (match_id, game_number, winning_team_id, result_scope)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (match_id, game_number)
             DO UPDATE SET winning_team_id = excluded.winning_team_id, result_scope = excluded.result_scope"};
        tx.execute(sql, params)?;
        Ok(())
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    pub fn get_match_results(&mut self, match_id: &str) -> Result<Vec<MatchResult>> {
        let mut stmt = self
            .conn
            .prepare("SELECT game_number, winning_team_id, result_scope FROM match_results WHERE match_id = ?1 AND game_number > 0")?;
        let results = stmt
            .query_map([match_id], |row| {
                let game_number: i32 = row.get(0)?;
                let winning_team_id: i32 = row.get(1)?;
                let result_scope: String = row.get(2)?;

                Ok(MatchResult::new(
                    match_id.to_string(),
                    game_number,
                    winning_team_id,
                    result_scope,
                ))
            })?
            .collect::<rusqlite::Result<Vec<MatchResult>>>()?;
        Ok(results)
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    pub fn get_decklists(&mut self, match_id: &str) -> Result<Vec<Deck>> {
        let mut stmt = self.conn.prepare(
            "SELECT game_number, deck_cards, sideboard_cards FROM decks WHERE match_id = ?1",
        )?;
        let deck = stmt
            .query_map([match_id], |row| {
                let game_number: i32 = row.get(0)?;
                let deck_cards: String = row.get(1)?;
                let sideboard_cards: String = row.get(2)?;

                Ok(Deck::from_raw(
                    "Found Deck".to_string(),
                    game_number,
                    &deck_cards,
                    &sideboard_cards,
                ))
            })?
            .collect::<rusqlite::Result<Vec<Deck>>>()?;
        Ok(deck)
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    pub fn get_mulligans(&mut self, match_id: &str) -> Result<Vec<Mulligan>> {
        let mut stmt = self
            .conn
            .prepare("SELECT game_number, number_to_keep, hand, play_draw, opponent_identity, decision FROM mulligans WHERE match_id = ?1")?;
        let mulligans = stmt
            .query_map([match_id], |row| {
                let game_number: i32 = row.get(0)?;
                let number_to_keep: i32 = row.get(1)?;
                let hand: String = row.get(2)?;
                let play_draw: String = row.get(3)?;
                let opponent_identity: String = row.get(4)?;
                let decision: String = row.get(5)?;

                Ok(Mulligan::new(
                    match_id.to_string(),
                    game_number,
                    number_to_keep,
                    hand,
                    play_draw,
                    opponent_identity,
                    decision,
                ))
            })?
            .collect::<rusqlite::Result<Vec<Mulligan>>>()?;
        Ok(mulligans)
    }

    /// # Errors
    ///
    /// will return an error if the database cannot be contacted for some reason
    pub fn get_matches(&mut self) -> Result<Vec<MTGAMatch>> {
        let mut statement = self.conn.prepare("SELECT id, controller_seat_id, controller_player_name, opponent_player_name, created_at FROM matches")?;
        let matches = statement
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let controller_seat_id: i32 = row.get(1)?;
                let controller_player_name: String = row.get(2)?;
                let opponent_player_name: String = row.get(3)?;
                let created_at: Option<DateTime<Utc>> = row.get(4)?;
                Ok(MTGAMatch::new_with_timestamp(
                    id,
                    controller_seat_id,
                    controller_player_name,
                    opponent_player_name,
                    created_at.unwrap_or_default(),
                ))
            })?
            .collect::<RusqliteResult<Vec<MTGAMatch>>>()?;

        Ok(matches)
    }

    /// # Errors
    ///
    /// Errors if underlying data is malformed
    pub fn get_match(&mut self, match_id: &str) -> Result<(MTGAMatch, Option<MatchResult>)> {
        let mut statement = self.conn.prepare(indoc! {r#"
            SELECT
                m.id, m.controller_player_name, m.opponent_player_name, mr.winning_team_id, m.controller_seat_id, m.created_at
            FROM matches m JOIN match_results mr ON m.id = mr.match_id
            WHERE m.id = ?1 AND mr.result_scope = "MatchScope_Match" LIMIT 1
            "#}
        )?;

        info!("Getting match details for match_id: {}", match_id);
        Ok(statement
            .query_row([&match_id], |row| {
                let id: String = row.get(0)?;
                let controller_player_name: String = row.get(1)?;
                let opponent_player_name: String = row.get(2)?;
                let winning_team_id: i32 = row.get(3)?;
                let controller_seat_id: i32 = row.get(4)?;
                let created_at: DateTime<Utc> = row.get(5)?;
                Ok((
                    MTGAMatch::new_with_timestamp(
                        id.clone(),
                        controller_seat_id,
                        controller_player_name,
                        opponent_player_name,
                        created_at,
                    ),
                    Some(MatchResult::new_match_result(id, winning_team_id)),
                ))
            })
            .unwrap_or_else(|e| {
                error!("Error getting match details: {:?}", e);
                (MTGAMatch::default(), None)
            }))
    }
}

impl Storage for MatchDB {
    /// # Errors
    ///
    /// will return an error if if the match replay cannot be written to the database due to missing data
    /// or connection error
    fn write(&mut self, match_replay: &MatchReplay) -> crate::Result<()> {
        info!("Writing match replay to database");
        let controller_seat_id = match_replay.get_controller_seat_id();
        let match_id = &match_replay.match_id;
        let (controller_name, opponent_name) = match_replay.get_player_names(controller_seat_id)?;
        let event_start = match_replay.match_start_time().unwrap_or(Utc::now());

        let mtga_match = MTGAMatchBuilder::default()
            .id(match_id.to_string())
            .controller_seat_id(controller_seat_id)
            .controller_player_name(controller_name)
            .opponent_player_name(opponent_name)
            .created_at(event_start)
            .build()?;

        let tx = self.conn.transaction().map_err(MatchDBError::from)?;

        Self::insert_match(&mtga_match, &tx)?;

        match_replay
            .get_decklists()?
            .iter()
            .try_for_each(|deck| Self::insert_deck(&match_replay.match_id, deck, &tx))?;

        let mulligan_infos = match_replay.get_mulligan_infos(&self.cards_database)?;
        mulligan_infos
            .iter()
            .try_for_each(|mulligan_info| Self::insert_mulligan_info(mulligan_info, &tx))?;

        // not too keen on this data model
        let match_results = match_replay.get_match_results()?;
        debug!("{:?}", match_results);
        for (i, result) in match_results.result_list.iter().enumerate() {
            let game_number = if result.scope == "MatchScope_Game" {
                i32::try_from(i + 1).unwrap_or(0)
            } else {
                0
            };

            let match_result = MatchResultBuilder::default()
                .match_id(match_id.to_string())
                .game_number(game_number)
                .winning_team_id(result.winning_team_id)
                .result_scope(result.scope.clone())
                .build()?;

            Self::insert_match_result(&match_result, &tx)?;
        }

        tx.commit()?;
        Ok(())
    }
}
