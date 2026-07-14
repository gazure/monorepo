use dioxus::prelude::*;

use crate::dto::{RosterBatter, RosterPitcher, TeamDetailDto, TeamRef, TeamRosterDto, TeamSeasonRow, TeamSummary};

#[cfg(feature = "server")]
const TEAM_SUMMARY_SELECT: &str = r"
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
    LEFT JOIN games g ON g.home_team_id = t.id OR g.away_team_id = t.id
";

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct TeamSummaryRow {
    id: i32,
    code: String,
    name: String,
    games: i64,
    wins: i64,
    losses: i64,
    runs_for: i64,
    runs_against: i64,
}

#[cfg(feature = "server")]
impl TeamSummaryRow {
    fn into_dto(self) -> TeamSummary {
        TeamSummary {
            team: TeamRef {
                id: self.id,
                code: self.code,
                name: self.name,
            },
            games: self.games,
            wins: self.wins,
            losses: self.losses,
            runs_for: self.runs_for,
            runs_against: self.runs_against,
        }
    }
}

/// Lightweight id/code/name list for filter dropdowns.
#[server]
pub async fn team_options() -> Result<Vec<TeamRef>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: i32,
        code: String,
        name: String,
    }

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as("SELECT id, code, name FROM teams ORDER BY code")
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;
    Ok(db_rows
        .into_iter()
        .map(|r| TeamRef {
            id: r.id,
            code: r.code,
            name: r.name,
        })
        .collect())
}

#[server]
pub async fn list_teams() -> Result<Vec<TeamSummary>, ServerFnError> {
    let pool = crate::pool().await?;
    let sql = format!("{TEAM_SUMMARY_SELECT} GROUP BY t.id, t.code, t.name ORDER BY t.code");
    let db_rows: Vec<TeamSummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;
    Ok(db_rows.into_iter().map(TeamSummaryRow::into_dto).collect())
}

#[server]
pub async fn team_detail(team_id: i32) -> Result<TeamDetailDto, ServerFnError> {
    use super::games::rows;

    let pool = crate::pool().await?;

    let sql = format!("{TEAM_SUMMARY_SELECT} WHERE t.id = $1 GROUP BY t.id, t.code, t.name");
    let summary_row: Option<TeamSummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(team_id)
        .fetch_optional(pool)
        .await
        .map_err(super::db_err)?;
    let summary = summary_row
        .map(TeamSummaryRow::into_dto)
        .ok_or_else(|| ServerFnError::new(format!("team {team_id} not found")))?;

    let sql = format!(
        "{select}
         WHERE g.home_team_id = $1 OR g.away_team_id = $1
         ORDER BY g.game_date DESC, g.id DESC
         LIMIT 25",
        select = rows::GAME_SUMMARY_SELECT
    );
    let recent: Vec<rows::GameSummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(team_id)
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;

    Ok(TeamDetailDto {
        summary,
        recent_games: recent.into_iter().map(rows::GameSummaryRow::into_dto).collect(),
    })
}

/// Season-by-season franchise record (regular season only)
#[server]
pub async fn team_seasons(team_id: i32) -> Result<Vec<TeamSeasonRow>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        season: i32,
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
        SELECT EXTRACT(YEAR FROM g.game_date)::int4 AS season,
               COUNT(*) AS games,
               COUNT(*) FILTER (
                   WHERE (g.home_team_id = $1 AND g.home_score > g.away_score)
                      OR (g.away_team_id = $1 AND g.away_score > g.home_score)
               ) AS wins,
               COUNT(*) FILTER (
                   WHERE (g.home_team_id = $1 AND g.home_score < g.away_score)
                      OR (g.away_team_id = $1 AND g.away_score < g.home_score)
               ) AS losses,
               COALESCE(SUM(CASE WHEN g.home_team_id = $1 THEN g.home_score ELSE g.away_score END), 0)::bigint AS runs_for,
               COALESCE(SUM(CASE WHEN g.home_team_id = $1 THEN g.away_score ELSE g.home_score END), 0)::bigint AS runs_against
        FROM games g
        JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
        WHERE (g.home_team_id = $1 OR g.away_team_id = $1)
          AND g.game_date <= re.end_date
        GROUP BY 1
        ORDER BY 1 DESC
        ",
        regular_end = super::REGULAR_SEASON_END
    )))
    .bind(team_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(db_rows
        .into_iter()
        .map(|r| TeamSeasonRow {
            season: r.season,
            games: r.games,
            wins: r.wins,
            losses: r.losses,
            runs_for: r.runs_for,
            runs_against: r.runs_against,
        })
        .collect())
}

/// Everyone who batted or pitched for the team in a season, with their
/// aggregate lines (includes postseason games)
#[server]
#[allow(clippy::too_many_lines)]
pub async fn team_roster(team_id: i32, season: i32) -> Result<TeamRosterDto, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct BatterRow {
        player_id: i32,
        name: String,
        games: i64,
        pa: i64,
        h: i64,
        home_runs: i64,
        stolen_bases: i64,
        avg: Option<f64>,
        obp: Option<f64>,
        slg: Option<f64>,
    }

    #[derive(sqlx::FromRow)]
    struct PitcherRow {
        player_id: i32,
        name: String,
        games: i64,
        wins: i64,
        losses: i64,
        saves: i64,
        outs: i64,
        so: i64,
        era: Option<f64>,
        whip: Option<f64>,
    }

    let pool = crate::pool().await?;

    let batters: Vec<BatterRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        SELECT bl.player_id, p.name,
               COUNT(*) AS games,
               COALESCE(SUM(bl.pa), 0)::bigint AS pa,
               COALESCE(SUM(bl.h), 0)::bigint AS h,
               {counts},
               {rates}
        FROM batting_lines bl
        JOIN players p ON p.id = bl.player_id
        JOIN games g ON g.id = bl.game_id
        WHERE bl.team_id = $1 AND EXTRACT(YEAR FROM g.game_date)::int4 = $2
        GROUP BY bl.player_id, p.name
        ORDER BY pa DESC, p.name
        ",
        counts = super::BATTING_COUNT_SQL,
        rates = super::BATTING_RATE_SQL
    )))
    .bind(team_id)
    .bind(season)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let pitchers: Vec<PitcherRow> = sqlx::query_as(sqlx::AssertSqlSafe(
        r"
        SELECT player_id, name, games, wins, losses, saves, outs, so,
               CASE WHEN outs > 0 THEN er::float8 * 27.0 / outs::float8 END AS era,
               CASE WHEN outs > 0 THEN (bb + h)::float8 * 3.0 / outs::float8 END AS whip
        FROM (
            SELECT pl.player_id, p.name,
                   COUNT(*) AS games,
                   COUNT(*) FILTER (WHERE pl.decision LIKE 'W%') AS wins,
                   COUNT(*) FILTER (WHERE pl.decision LIKE 'L%') AS losses,
                   COUNT(*) FILTER (WHERE pl.decision LIKE 'S%') AS saves,
                   COALESCE(SUM(FLOOR(pl.ip) * 3 + ROUND((pl.ip - FLOOR(pl.ip)) * 10)), 0)::bigint AS outs,
                   COALESCE(SUM(pl.so), 0)::bigint AS so,
                   COALESCE(SUM(pl.er), 0)::bigint AS er,
                   COALESCE(SUM(pl.bb), 0)::bigint AS bb,
                   COALESCE(SUM(pl.h), 0)::bigint AS h
            FROM pitching_lines pl
            JOIN players p ON p.id = pl.player_id
            JOIN games g ON g.id = pl.game_id
            WHERE pl.team_id = $1 AND EXTRACT(YEAR FROM g.game_date)::int4 = $2
            GROUP BY pl.player_id, p.name
        ) totals
        ORDER BY outs DESC, name
        "
        .to_string(),
    ))
    .bind(team_id)
    .bind(season)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(TeamRosterDto {
        batters: batters
            .into_iter()
            .map(|r| RosterBatter {
                player_id: r.player_id,
                name: r.name,
                games: r.games,
                pa: r.pa,
                h: r.h,
                home_runs: r.home_runs,
                stolen_bases: r.stolen_bases,
                avg: r.avg,
                obp: r.obp,
                slg: r.slg,
                ops: r.obp.zip(r.slg).map(|(o, s)| o + s),
            })
            .collect(),
        pitchers: pitchers
            .into_iter()
            .map(|r| RosterPitcher {
                player_id: r.player_id,
                name: r.name,
                games: r.games,
                wins: r.wins,
                losses: r.losses,
                saves: r.saves,
                outs: r.outs,
                so: r.so,
                era: r.era,
                whip: r.whip,
            })
            .collect(),
    })
}

/// Every game of a team's season in date order (for the schedule grid)
#[server]
pub async fn team_schedule(team_id: i32, season: i32) -> Result<Vec<crate::dto::GameSummary>, ServerFnError> {
    use super::games::rows;

    let pool = crate::pool().await?;
    let sql = format!(
        "{select}
         WHERE (g.home_team_id = $1 OR g.away_team_id = $1)
           AND EXTRACT(YEAR FROM g.game_date)::int4 = $2
         ORDER BY g.game_date, g.id",
        select = rows::GAME_SUMMARY_SELECT
    );
    let db_rows: Vec<rows::GameSummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(team_id)
        .bind(season)
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;
    Ok(db_rows.into_iter().map(rows::GameSummaryRow::into_dto).collect())
}

/// All-time W-L against every opponent
#[server]
pub async fn team_head_to_head(team_id: i32) -> Result<Vec<crate::dto::HeadToHeadRow>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        opponent_id: i32,
        code: String,
        name: String,
        games: i64,
        wins: i64,
        losses: i64,
    }

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(
        r"
        SELECT t.id AS opponent_id, t.code, t.name,
               COUNT(*) AS games,
               COUNT(*) FILTER (
                   WHERE (g.home_team_id = $1 AND g.home_score > g.away_score)
                      OR (g.away_team_id = $1 AND g.away_score > g.home_score)
               ) AS wins,
               COUNT(*) FILTER (
                   WHERE (g.home_team_id = $1 AND g.home_score < g.away_score)
                      OR (g.away_team_id = $1 AND g.away_score < g.home_score)
               ) AS losses
        FROM games g
        JOIN teams t ON t.id = CASE WHEN g.home_team_id = $1 THEN g.away_team_id ELSE g.home_team_id END
        WHERE g.home_team_id = $1 OR g.away_team_id = $1
        GROUP BY t.id, t.code, t.name
        ORDER BY games DESC
        ",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(db_rows
        .into_iter()
        .map(|r| crate::dto::HeadToHeadRow {
            opponent: crate::dto::TeamRef {
                id: r.opponent_id,
                code: r.code,
                name: r.name,
            },
            games: r.games,
            wins: r.wins,
            losses: r.losses,
        })
        .collect())
}
