//! Postgres persistence and change detection.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, migrate::MigrateDatabase, postgres::PgPoolOptions};

use crate::{
    ics::FeedGame,
    model::{FieldChange, Game, GameStatus},
};

#[derive(Debug, Clone)]
pub struct Store {
    pool: PgPool,
}

/// What one sync pass did.
#[derive(Debug, Default)]
pub struct SyncReport {
    pub inserted: usize,
    pub unchanged: usize,
    /// Games whose stored state differed from upstream, with the fields that moved.
    pub changed: Vec<(Game, Vec<FieldChange>)>,
}

impl SyncReport {
    pub fn significant(&self) -> usize {
        self.changed
            .iter()
            .filter(|(_, changes)| changes.iter().any(|c| c.significant))
            .count()
    }
}

/// A logged change, for the `changes` command.
#[derive(Debug, FromRow)]
pub struct ChangeRow {
    pub observed_at: DateTime<Utc>,
    pub league: String,
    pub short_name: String,
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub significant: bool,
}

#[derive(Debug, FromRow)]
struct GameRow {
    id: i64,
    league: String,
    espn_id: String,
    season: i32,
    week: Option<i32>,
    name: String,
    short_name: String,
    home_abbr: String,
    home_name: String,
    away_abbr: String,
    away_name: String,
    kickoff: DateTime<Utc>,
    time_tbd: bool,
    venue: Option<String>,
    city: Option<String>,
    status: String,
    broadcast: Option<String>,
    sequence: i32,
}

impl GameRow {
    fn to_game(&self) -> Game {
        Game {
            league: self.league.clone(),
            espn_id: self.espn_id.clone(),
            season: self.season,
            week: self.week,
            name: self.name.clone(),
            short_name: self.short_name.clone(),
            home_abbr: self.home_abbr.clone(),
            home_name: self.home_name.clone(),
            away_abbr: self.away_abbr.clone(),
            away_name: self.away_name.clone(),
            kickoff: self.kickoff,
            time_tbd: self.time_tbd,
            venue: self.venue.clone(),
            city: self.city.clone(),
            status: GameStatus::parse(&self.status),
            broadcast: self.broadcast.clone(),
        }
    }
}

/// The `games` column list, shared by every read so `GameRow` always lines up.
macro_rules! game_columns {
    () => {
        "id, league, espn_id, season, week, name, short_name, \
         home_abbr, home_name, away_abbr, away_name, kickoff, time_tbd, venue, city, \
         status, broadcast, sequence"
    };
}

impl Store {
    /// Connects, creating the database first if it does not exist yet.
    ///
    /// Creating it here keeps a fresh deployment to a single step; the compose
    /// stack has no hook for running `createdb` against an existing volume.
    pub async fn connect(database_url: &str) -> Result<Self> {
        let exists = Postgres::database_exists(database_url)
            .await
            .with_context(|| "checking whether the slate database exists")?;
        if !exists {
            Postgres::create_database(database_url)
                .await
                .with_context(|| "creating the slate database")?;
            tracing::info!("created the slate database");
        }

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .with_context(|| "connecting to the slate database")?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Upserts a batch of freshly fetched games, logging every field that moved.
    pub async fn sync(&self, games: &[Game]) -> Result<SyncReport> {
        let mut report = SyncReport::default();
        let mut tx = self.pool.begin().await?;

        for fresh in games {
            let existing: Option<GameRow> = sqlx::query_as(concat!(
                "SELECT ",
                game_columns!(),
                " FROM games WHERE league = $1 AND espn_id = $2"
            ))
            .bind(&fresh.league)
            .bind(&fresh.espn_id)
            .fetch_optional(&mut *tx)
            .await?;

            match existing {
                None => {
                    insert_game(&mut tx, fresh).await?;
                    report.inserted += 1;
                }
                Some(row) => {
                    let changes = row.to_game().diff(fresh);
                    if changes.is_empty() {
                        sqlx::query("UPDATE games SET last_seen = NOW() WHERE id = $1")
                            .bind(row.id)
                            .execute(&mut *tx)
                            .await?;
                        report.unchanged += 1;
                    } else {
                        apply_changes(&mut tx, row.id, row.sequence, fresh, &changes).await?;
                        report.changed.push((fresh.clone(), changes));
                    }
                }
            }
        }

        tx.commit().await?;
        Ok(report)
    }

    /// Every game involving `team`, oldest first, with the `SEQUENCE` to publish.
    pub async fn feed(&self, league: &str, team: &str) -> Result<Vec<FeedGame>> {
        let rows: Vec<GameRow> = sqlx::query_as(concat!(
            "SELECT ",
            game_columns!(),
            " FROM games \
             WHERE league = $1 AND (home_abbr ILIKE $2 OR away_abbr ILIKE $2) \
             ORDER BY kickoff"
        ))
        .bind(league)
        .bind(team)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|row| FeedGame {
                game: row.to_game(),
                sequence: row.sequence,
            })
            .collect())
    }

    pub async fn recent_changes(&self, limit: i64) -> Result<Vec<ChangeRow>> {
        let rows = sqlx::query_as(
            "SELECT r.observed_at, g.league, g.short_name, r.field, r.old_value, \
                    r.new_value, r.significant \
             FROM game_revisions r JOIN games g ON g.id = r.game_id \
             ORDER BY r.observed_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

async fn insert_game(tx: &mut Tx<'_>, game: &Game) -> Result<()> {
    sqlx::query(
        "INSERT INTO games (league, espn_id, season, week, name, short_name, \
            home_abbr, home_name, away_abbr, away_name, kickoff, time_tbd, venue, \
            city, status, broadcast, sequence) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,0)",
    )
    .bind(&game.league)
    .bind(&game.espn_id)
    .bind(game.season)
    .bind(game.week)
    .bind(&game.name)
    .bind(&game.short_name)
    .bind(&game.home_abbr)
    .bind(&game.home_name)
    .bind(&game.away_abbr)
    .bind(&game.away_name)
    .bind(game.kickoff)
    .bind(game.time_tbd)
    .bind(&game.venue)
    .bind(&game.city)
    .bind(game.status.as_str())
    .bind(&game.broadcast)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Writes the revision log and the new game state.
///
/// `SEQUENCE` advances only when a calendar-significant field moved, so a
/// broadcast change is recorded without re-notifying every subscriber.
async fn apply_changes(
    tx: &mut Tx<'_>,
    game_id: i64,
    current_sequence: i32,
    fresh: &Game,
    changes: &[FieldChange],
) -> Result<()> {
    for change in changes {
        sqlx::query(
            "INSERT INTO game_revisions (game_id, field, old_value, new_value, significant) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(game_id)
        .bind(change.field)
        .bind(change.old.as_ref())
        .bind(change.new.as_ref())
        .bind(change.significant)
        .execute(&mut **tx)
        .await?;
    }

    let bump = i32::from(changes.iter().any(|c| c.significant));

    sqlx::query(
        "UPDATE games SET kickoff = $1, time_tbd = $2, venue = $3, city = $4, \
            status = $5, broadcast = $6, week = $7, name = $8, short_name = $9, \
            sequence = $10, last_seen = NOW() \
         WHERE id = $11",
    )
    .bind(fresh.kickoff)
    .bind(fresh.time_tbd)
    .bind(&fresh.venue)
    .bind(&fresh.city)
    .bind(fresh.status.as_str())
    .bind(&fresh.broadcast)
    .bind(fresh.week)
    .bind(&fresh.name)
    .bind(&fresh.short_name)
    .bind(current_sequence + bump)
    .bind(game_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
