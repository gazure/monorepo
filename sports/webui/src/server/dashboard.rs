use dioxus::prelude::*;

use crate::dto::{DashboardStats, GameSummary};

#[server]
pub async fn dashboard_stats() -> Result<DashboardStats, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        teams: i64,
        players: i64,
        games: i64,
        batting_lines: i64,
        pitching_lines: i64,
        plays: i64,
        first_game: Option<chrono::NaiveDate>,
        last_game: Option<chrono::NaiveDate>,
    }

    let pool = crate::pool().await?;
    let row: Row = sqlx::query_as(
        r"
        SELECT
            (SELECT COUNT(*) FROM teams) AS teams,
            (SELECT COUNT(*) FROM players) AS players,
            (SELECT COUNT(*) FROM games) AS games,
            (SELECT COUNT(*) FROM batting_lines) AS batting_lines,
            (SELECT COUNT(*) FROM pitching_lines) AS pitching_lines,
            (SELECT COUNT(*) FROM play_by_play) AS plays,
            (SELECT MIN(game_date) FROM games) AS first_game,
            (SELECT MAX(game_date) FROM games) AS last_game
        ",
    )
    .fetch_one(pool)
    .await
    .map_err(super::db_err)?;

    Ok(DashboardStats {
        teams: row.teams,
        players: row.players,
        games: row.games,
        batting_lines: row.batting_lines,
        pitching_lines: row.pitching_lines,
        plays: row.plays,
        first_game: row.first_game,
        last_game: row.last_game,
    })
}

#[server]
pub async fn recent_games(limit: u32) -> Result<Vec<GameSummary>, ServerFnError> {
    use super::games::rows;

    let limit = limit.clamp(1, 100);
    let pool = crate::pool().await?;
    let sql = format!(
        "{select} ORDER BY g.game_date DESC, g.id DESC LIMIT $1",
        select = rows::GAME_SUMMARY_SELECT
    );
    let db_rows: Vec<rows::GameSummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;
    Ok(db_rows.into_iter().map(rows::GameSummaryRow::into_dto).collect())
}
