use dioxus::prelude::*;

use crate::dto::{AchievementsDto, FeatRow, NoHitterRow};

/// Curated notable-game feats detectable from box-score sums: no-hitters
/// (with perfect games flagged), 18+ strikeout starts, cycles, and
/// four-homer games.
#[server]
#[allow(clippy::too_many_lines)]
pub async fn achievements() -> Result<AchievementsDto, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct NhRow {
        game_id: i32,
        game_date: chrono::NaiveDate,
        team: String,
        opponent: String,
        pitchers: String,
        walks: i64,
        perfect: bool,
    }

    #[derive(sqlx::FromRow)]
    struct PitchFeat {
        game_id: i32,
        game_date: chrono::NaiveDate,
        player_id: i32,
        name: String,
        team: String,
        opponent: String,
        so: i64,
        outs: i64,
    }

    #[derive(sqlx::FromRow)]
    struct BatFeat {
        game_id: i32,
        game_date: chrono::NaiveDate,
        player_id: i32,
        name: String,
        team: String,
        opponent: String,
        h: i64,
        ab: i64,
        home_runs: i64,
        rbi: i64,
    }

    const BAT_FEAT_SELECT: &str = r"
        SELECT bl.game_id, g.game_date, bl.player_id, p.name, t.code AS team,
               CASE WHEN bl.team_id = g.home_team_id THEN ta.code ELSE th.code END AS opponent,
               COALESCE(bl.h, 0)::bigint AS h, COALESCE(bl.ab, 0)::bigint AS ab,
               bl.home_runs::bigint AS home_runs, COALESCE(bl.rbi, 0)::bigint AS rbi
        FROM batting_lines bl
        JOIN games g ON g.id = bl.game_id
        JOIN players p ON p.id = bl.player_id
        JOIN teams t ON t.id = bl.team_id
        JOIN teams th ON th.id = g.home_team_id
        JOIN teams ta ON ta.id = g.away_team_id
    ";

    let pool = crate::pool().await?;

    let nh_rows: Vec<NhRow> = sqlx::query_as(sqlx::AssertSqlSafe(
        r"
        WITH nh AS (
            SELECT pl.game_id, pl.team_id,
                   string_agg(p.name, ', ' ORDER BY pl.pitch_order) AS pitchers,
                   COALESCE(SUM(pl.bb), 0)::bigint AS walks,
                   COALESCE(SUM(pl.batters_faced), 0)::bigint AS bf
            FROM pitching_lines pl
            JOIN players p ON p.id = pl.player_id
            GROUP BY pl.game_id, pl.team_id
            HAVING SUM(pl.h) = 0
               AND SUM(FLOOR(pl.ip) * 3 + ROUND((pl.ip - FLOOR(pl.ip)) * 10)) >= 27
        )
        SELECT nh.game_id, g.game_date, t.code AS team,
               CASE WHEN nh.team_id = g.home_team_id THEN ta.code ELSE th.code END AS opponent,
               nh.pitchers, nh.walks,
               (nh.walks = 0 AND nh.bf = 27 AND NOT EXISTS (
                   SELECT 1 FROM play_by_play pbp
                   WHERE pbp.game_id = nh.game_id
                     AND pbp.batting_team_id <> nh.team_id
                     AND pbp.runners_before IS NOT NULL
               )) AS perfect
        FROM nh
        JOIN games g ON g.id = nh.game_id
        JOIN teams t ON t.id = nh.team_id
        JOIN teams th ON th.id = g.home_team_id
        JOIN teams ta ON ta.id = g.away_team_id
        ORDER BY g.game_date DESC
        "
        .to_string(),
    ))
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let k_rows: Vec<PitchFeat> = sqlx::query_as(sqlx::AssertSqlSafe(
        r"
        SELECT pl.game_id, g.game_date, pl.player_id, p.name, t.code AS team,
               CASE WHEN pl.team_id = g.home_team_id THEN ta.code ELSE th.code END AS opponent,
               pl.so::bigint AS so,
               (FLOOR(pl.ip) * 3 + ROUND((pl.ip - FLOOR(pl.ip)) * 10))::bigint AS outs
        FROM pitching_lines pl
        JOIN games g ON g.id = pl.game_id
        JOIN players p ON p.id = pl.player_id
        JOIN teams t ON t.id = pl.team_id
        JOIN teams th ON th.id = g.home_team_id
        JOIN teams ta ON ta.id = g.away_team_id
        WHERE pl.so >= 18
        ORDER BY pl.so DESC, g.game_date
        "
        .to_string(),
    ))
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let cycle_rows: Vec<BatFeat> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"{BAT_FEAT_SELECT}
        WHERE bl.h - bl.doubles - bl.triples - bl.home_runs >= 1
          AND bl.doubles >= 1 AND bl.triples >= 1 AND bl.home_runs >= 1
        ORDER BY g.game_date DESC
        "
    )))
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let hr_rows: Vec<BatFeat> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"{BAT_FEAT_SELECT}
        WHERE bl.home_runs >= 4
        ORDER BY g.game_date DESC
        "
    )))
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(AchievementsDto {
        no_hitters: nh_rows
            .into_iter()
            .map(|r| NoHitterRow {
                game_id: r.game_id,
                game_date: r.game_date,
                team: r.team,
                opponent: r.opponent,
                pitchers: r.pitchers,
                walks: r.walks,
                perfect: r.perfect,
            })
            .collect(),
        k_games: k_rows
            .into_iter()
            .map(|r| FeatRow {
                game_id: r.game_id,
                game_date: r.game_date,
                player_id: r.player_id,
                name: r.name,
                team: r.team,
                opponent: r.opponent,
                line: format!("{} K in {} IP", r.so, crate::dto::format_ip(r.outs)),
            })
            .collect(),
        cycles: cycle_rows
            .into_iter()
            .map(|r| FeatRow {
                line: format!("{}-for-{}, {} RBI", r.h, r.ab, r.rbi),
                game_id: r.game_id,
                game_date: r.game_date,
                player_id: r.player_id,
                name: r.name,
                team: r.team,
                opponent: r.opponent,
            })
            .collect(),
        hr_games: hr_rows
            .into_iter()
            .map(|r| FeatRow {
                line: format!("{} HR, {} RBI ({}-for-{})", r.home_runs, r.rbi, r.h, r.ab),
                game_id: r.game_id,
                game_date: r.game_date,
                player_id: r.player_id,
                name: r.name,
                team: r.team,
                opponent: r.opponent,
            })
            .collect(),
    })
}
