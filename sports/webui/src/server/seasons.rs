use dioxus::prelude::*;

use crate::dto::{GameSummary, SeasonSummary, TeamSummary};

#[server]
pub async fn list_seasons() -> Result<Vec<SeasonSummary>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        season: i32,
        games: i64,
        teams: i64,
        runs: i64,
        runs_per_game: Option<f64>,
        attendance: i64,
        avg_attendance: Option<f64>,
        postseason_games: i64,
    }

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        WITH regular_end AS ({regular_end})
        SELECT EXTRACT(YEAR FROM g.game_date)::int4 AS season,
               COUNT(*) AS games,
               COUNT(DISTINCT g.home_team_id) AS teams,
               COALESCE(SUM(g.home_score + g.away_score), 0)::bigint AS runs,
               SUM(g.home_score + g.away_score)::float8
                   / NULLIF(COUNT(*) FILTER (WHERE g.home_score IS NOT NULL), 0)::float8 AS runs_per_game,
               COALESCE(SUM(g.attendance), 0)::bigint AS attendance,
               AVG(g.attendance)::float8 AS avg_attendance,
               COUNT(*) FILTER (WHERE g.game_date > re.end_date) AS postseason_games
        FROM games g
        JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
        GROUP BY 1
        ORDER BY 1 DESC
        ",
        regular_end = super::REGULAR_SEASON_END
    )))
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(db_rows
        .into_iter()
        .map(|r| SeasonSummary {
            season: r.season,
            games: r.games,
            teams: r.teams,
            runs: r.runs,
            runs_per_game: r.runs_per_game,
            attendance: r.attendance,
            avg_attendance: r.avg_attendance,
            postseason_games: r.postseason_games,
        })
        .collect())
}

/// Regular-season standings for one year, best record first.
#[server]
pub async fn season_standings(year: i32) -> Result<Vec<TeamSummary>, ServerFnError> {
    use crate::dto::TeamRef;

    #[derive(sqlx::FromRow)]
    struct Row {
        id: i32,
        code: String,
        name: String,
        games: i64,
        wins: i64,
        losses: i64,
        runs_for: i64,
        runs_against: i64,
    }

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        WITH regular_end AS ({regular_end})
        SELECT t.id, t.code, t.name,
               COUNT(g.id) AS games,
               COUNT(*) FILTER (
                   WHERE (g.home_team_id = t.id AND g.home_score > g.away_score)
                      OR (g.away_team_id = t.id AND g.away_score > g.home_score)
               ) AS wins,
               COUNT(*) FILTER (
                   WHERE (g.home_team_id = t.id AND g.home_score < g.away_score)
                      OR (g.away_team_id = t.id AND g.away_score < g.home_score)
               ) AS losses,
               COALESCE(SUM(CASE WHEN g.home_team_id = t.id THEN g.home_score ELSE g.away_score END), 0)::bigint AS runs_for,
               COALESCE(SUM(CASE WHEN g.home_team_id = t.id THEN g.away_score ELSE g.home_score END), 0)::bigint AS runs_against
        FROM teams t
        JOIN games g ON g.home_team_id = t.id OR g.away_team_id = t.id
        JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
        WHERE EXTRACT(YEAR FROM g.game_date)::int4 = $1
          AND g.game_date <= re.end_date
        GROUP BY t.id, t.code, t.name
        ORDER BY COUNT(*) FILTER (
                   WHERE (g.home_team_id = t.id AND g.home_score > g.away_score)
                      OR (g.away_team_id = t.id AND g.away_score > g.home_score)
               )::float8 / NULLIF(COUNT(g.id), 0)::float8 DESC NULLS LAST
        ",
        regular_end = super::REGULAR_SEASON_END
    )))
    .bind(year)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(db_rows
        .into_iter()
        .map(|r| TeamSummary {
            team: TeamRef {
                id: r.id,
                code: r.code,
                name: r.name,
            },
            games: r.games,
            wins: r.wins,
            losses: r.losses,
            runs_for: r.runs_for,
            runs_against: r.runs_against,
        })
        .collect())
}

/// That year's postseason games in chronological order.
#[server]
pub async fn season_postseason_games(year: i32) -> Result<Vec<GameSummary>, ServerFnError> {
    use super::games::rows;

    let pool = crate::pool().await?;
    let db_rows: Vec<rows::GameSummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        WITH regular_end AS ({regular_end})
        {select}
        JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
        WHERE EXTRACT(YEAR FROM g.game_date)::int4 = $1
          AND g.game_date > re.end_date
        ORDER BY g.game_date, g.id
        ",
        regular_end = super::REGULAR_SEASON_END,
        select = rows::GAME_SUMMARY_SELECT
    )))
    .bind(year)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(db_rows.into_iter().map(rows::GameSummaryRow::into_dto).collect())
}
