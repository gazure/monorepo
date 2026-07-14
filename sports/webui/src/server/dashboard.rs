use dioxus::prelude::*;

use crate::dto::{DashboardStats, DramaticGame, GameSummary, SeasonGamesCount};

/// The most dramatic recent games: of the last 300 decided games, the ones
/// with the largest total win-probability movement, each with its
/// home-perspective win-expectancy series
#[server]
pub async fn dramatic_games(limit: u32) -> Result<Vec<DramaticGame>, ServerFnError> {
    use std::collections::HashMap;

    use super::games::rows;

    #[derive(sqlx::FromRow)]
    struct DramaRow {
        game_id: i32,
        swing: Option<f64>,
    }

    #[derive(sqlx::FromRow)]
    struct WeRow {
        game_id: i32,
        win_expectancy_after: Option<f64>,
    }

    let limit = limit.clamp(1, 20);
    let pool = crate::pool().await?;

    let drama: Vec<DramaRow> = sqlx::query_as(
        r"
        WITH recent AS (
            SELECT id FROM games
            WHERE home_score IS NOT NULL AND away_score IS NOT NULL AND home_score <> away_score
            ORDER BY game_date DESC, id DESC
            LIMIT 300
        )
        SELECT p.game_id, SUM(ABS(p.wpa))::float8 AS swing
        FROM play_by_play p
        JOIN recent r ON r.id = p.game_id
        GROUP BY 1
        ORDER BY swing DESC NULLS LAST
        LIMIT $1
        ",
    )
    .bind(i64::from(limit))
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let ids: Vec<i32> = drama.iter().map(|d| d.game_id).collect();

    let sql = format!("{select} WHERE g.id = ANY($1)", select = rows::GAME_SUMMARY_SELECT);
    let summaries: Vec<rows::GameSummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(&ids)
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;
    let mut by_id: HashMap<i32, GameSummary> = summaries
        .into_iter()
        .map(|r| {
            let dto = r.into_dto();
            (dto.id, dto)
        })
        .collect();

    let we_rows: Vec<WeRow> = sqlx::query_as(
        r"
        SELECT game_id, win_expectancy_after::float8 AS win_expectancy_after
        FROM play_by_play
        WHERE game_id = ANY($1)
        ORDER BY game_id, event_num
        ",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;
    let mut series: HashMap<i32, Vec<f64>> = HashMap::new();
    for row in we_rows {
        if let Some(we) = row.win_expectancy_after {
            series.entry(row.game_id).or_default().push(we);
        }
    }

    Ok(drama
        .into_iter()
        .filter_map(|d| {
            let game = by_id.remove(&d.game_id)?;
            // Stored WE is winner-perspective; flip to home perspective
            let home_won = game.home_score.unwrap_or(0) > game.away_score.unwrap_or(0);
            let mut we_home: Vec<f64> = vec![0.5];
            we_home.extend(
                series
                    .remove(&d.game_id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|we| if home_won { we } else { 1.0 - we }),
            );
            we_home.push(if home_won { 1.0 } else { 0.0 });
            Some(DramaticGame {
                game,
                swing: d.swing.unwrap_or(0.0),
                we_home,
            })
        })
        .collect())
}

#[server]
pub async fn games_per_season() -> Result<Vec<SeasonGamesCount>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        season: i32,
        games: i64,
    }

    let pool = crate::pool().await?;
    let rows: Vec<Row> = sqlx::query_as(
        r"
        SELECT EXTRACT(YEAR FROM game_date)::int4 AS season, COUNT(*) AS games
        FROM games
        GROUP BY 1
        ORDER BY 1
        ",
    )
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(rows
        .into_iter()
        .map(|r| SeasonGamesCount {
            season: r.season,
            games: r.games,
        })
        .collect())
}

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
