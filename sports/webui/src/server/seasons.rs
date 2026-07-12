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

/// Postseason series grouped into rounds (earliest round first, World Series
/// last), reconstructed by chaining backward from the latest series: each
/// participant's immediately-preceding series belongs to the prior round.
#[server]
#[allow(clippy::too_many_lines)]
pub async fn postseason_bracket(year: i32) -> Result<Vec<Vec<crate::dto::BracketSeries>>, ServerFnError> {
    use std::collections::HashMap;

    use crate::dto::{BracketSeries, TeamRef};

    #[derive(sqlx::FromRow)]
    struct Row {
        game_date: chrono::NaiveDate,
        home_id: i32,
        home_code: String,
        home_name: String,
        away_id: i32,
        away_code: String,
        away_name: String,
        home_score: i32,
        away_score: i32,
    }

    struct Series {
        teams: [TeamRef; 2],
        wins: [i64; 2],
        first: chrono::NaiveDate,
        last: chrono::NaiveDate,
    }

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        WITH regular_end AS ({regular_end})
        SELECT g.game_date,
               th.id AS home_id, th.code AS home_code, th.name AS home_name,
               ta.id AS away_id, ta.code AS away_code, ta.name AS away_name,
               g.home_score, g.away_score
        FROM games g
        JOIN teams th ON th.id = g.home_team_id
        JOIN teams ta ON ta.id = g.away_team_id
        JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
        WHERE EXTRACT(YEAR FROM g.game_date)::int4 = $1
          AND g.game_date > re.end_date
          AND g.home_score IS NOT NULL AND g.away_score IS NOT NULL
          AND g.home_score <> g.away_score
        ORDER BY g.game_date, g.id
        ",
        regular_end = super::REGULAR_SEASON_END
    )))
    .bind(year)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    // Group games into series by team pair
    let mut series: HashMap<(i32, i32), Series> = HashMap::new();
    for r in db_rows {
        let key = (r.home_id.min(r.away_id), r.home_id.max(r.away_id));
        let entry = series.entry(key).or_insert_with(|| Series {
            teams: [
                TeamRef {
                    id: r.home_id,
                    code: r.home_code.clone(),
                    name: r.home_name.clone(),
                },
                TeamRef {
                    id: r.away_id,
                    code: r.away_code.clone(),
                    name: r.away_name.clone(),
                },
            ],
            wins: [0, 0],
            first: r.game_date,
            last: r.game_date,
        });
        let home_won = r.home_score > r.away_score;
        let winner_id = if home_won { r.home_id } else { r.away_id };
        let idx = usize::from(entry.teams[1].id == winner_id);
        entry.wins[idx] += 1;
        entry.first = entry.first.min(r.game_date);
        entry.last = entry.last.max(r.game_date);
    }

    let mut remaining: Vec<Series> = series.into_values().collect();
    if remaining.is_empty() {
        return Ok(Vec::new());
    }

    // Backward chain: the latest-ending series is the final round
    let mut rounds_rev: Vec<Vec<Series>> = Vec::new();
    let final_idx = remaining
        .iter()
        .enumerate()
        .max_by_key(|(_, s)| s.last)
        .map(|(i, _)| i)
        .expect("nonempty");
    let mut current = vec![remaining.swap_remove(final_idx)];

    while !current.is_empty() {
        // Predecessors: for each participant of the current round, their
        // latest earlier series
        let mut pred_indices: Vec<usize> = Vec::new();
        for s in &current {
            for team in &s.teams {
                let best = remaining
                    .iter()
                    .enumerate()
                    .filter(|(_, cand)| cand.last < s.first && cand.teams.iter().any(|t| t.id == team.id))
                    .max_by_key(|(_, cand)| cand.last)
                    .map(|(i, _)| i);
                if let Some(i) = best
                    && !pred_indices.contains(&i)
                {
                    pred_indices.push(i);
                }
            }
        }
        rounds_rev.push(std::mem::take(&mut current));
        pred_indices.sort_unstable_by(|a, b| b.cmp(a));
        for i in pred_indices {
            current.push(remaining.swap_remove(i));
        }
    }

    Ok(rounds_rev
        .into_iter()
        .rev()
        .map(|round| {
            round
                .into_iter()
                .map(|s| {
                    let [a, b] = s.teams;
                    let [wa, wb] = s.wins;
                    if wa >= wb {
                        BracketSeries {
                            winner: a,
                            winner_wins: wa,
                            loser: b,
                            loser_wins: wb,
                        }
                    } else {
                        BracketSeries {
                            winner: b,
                            winner_wins: wb,
                            loser: a,
                            loser_wins: wa,
                        }
                    }
                })
                .collect()
        })
        .collect())
}
