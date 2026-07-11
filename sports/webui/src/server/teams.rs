use dioxus::prelude::*;

use crate::dto::{TeamDetailDto, TeamRef, TeamSummary};

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
