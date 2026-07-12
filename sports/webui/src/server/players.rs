use dioxus::prelude::*;

use crate::dto::{
    BattingGameLogRow, Page, PitchingGameLogRow, PlayerBrowseRow, PlayerBrowseSort, PlayerDetailDto, PlayerHit,
    PlayerSplitsDto, SplitRow,
};

/// Home/road and vs-opponent batting splits (career, all games)
#[server]
pub async fn player_batting_splits(player_id: i32) -> Result<PlayerSplitsDto, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        label: String,
        games: i64,
        pa: i64,
        h: i64,
        home_runs: i64,
        avg: Option<f64>,
        obp: Option<f64>,
        slg: Option<f64>,
    }

    fn into_split(r: Row) -> SplitRow {
        SplitRow {
            label: r.label,
            games: r.games,
            pa: r.pa,
            h: r.h,
            home_runs: r.home_runs,
            avg: r.avg,
            obp: r.obp,
            slg: r.slg,
            ops: r.obp.zip(r.slg).map(|(o, s)| o + s),
        }
    }

    let pool = crate::pool().await?;

    let home_away: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        SELECT CASE WHEN bl.team_id = g.home_team_id THEN 'Home' ELSE 'Road' END AS label,
               COUNT(*) AS games,
               COALESCE(SUM(bl.pa), 0)::bigint AS pa,
               COALESCE(SUM(bl.h), 0)::bigint AS h,
               COALESCE(SUM(bl.home_runs), 0)::bigint AS home_runs,
               {rates}
        FROM batting_lines bl
        JOIN games g ON g.id = bl.game_id
        WHERE bl.player_id = $1
        GROUP BY 1
        ORDER BY 1
        ",
        rates = super::BATTING_RATE_SQL
    )))
    .bind(player_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let vs_team: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        SELECT t.code AS label,
               COUNT(*) AS games,
               COALESCE(SUM(bl.pa), 0)::bigint AS pa,
               COALESCE(SUM(bl.h), 0)::bigint AS h,
               COALESCE(SUM(bl.home_runs), 0)::bigint AS home_runs,
               {rates}
        FROM batting_lines bl
        JOIN games g ON g.id = bl.game_id
        JOIN teams t ON t.id = CASE WHEN bl.team_id = g.home_team_id THEN g.away_team_id ELSE g.home_team_id END
        WHERE bl.player_id = $1
        GROUP BY t.code
        HAVING COALESCE(SUM(bl.pa), 0) >= 10
        ORDER BY pa DESC
        ",
        rates = super::BATTING_RATE_SQL
    )))
    .bind(player_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(PlayerSplitsDto {
        home_away: home_away.into_iter().map(into_split).collect(),
        vs_team: vs_team.into_iter().map(into_split).collect(),
    })
}

/// Paginated all-time career batting list (players with at least one PA)
#[server]
pub async fn browse_players(
    sort: PlayerBrowseSort,
    page: u32,
    page_size: u32,
) -> Result<Page<PlayerBrowseRow>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
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
        ops: Option<f64>,
        total: i64,
    }

    let order = match sort {
        PlayerBrowseSort::Pa => "pa DESC",
        PlayerBrowseSort::Hits => "h DESC",
        PlayerBrowseSort::HomeRuns => "home_runs DESC",
        PlayerBrowseSort::StolenBases => "stolen_bases DESC",
        PlayerBrowseSort::Ops => "ops DESC NULLS LAST",
    };
    let page_size = page_size.clamp(1, 100);

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        SELECT b.*, p.name, b.obp + b.slg AS ops, COUNT(*) OVER () AS total
        FROM (
            SELECT bl.player_id,
                   COUNT(*) AS games,
                   COALESCE(SUM(bl.pa), 0)::bigint AS pa,
                   COALESCE(SUM(bl.h), 0)::bigint AS h,
                   COALESCE(SUM(bl.home_runs), 0)::bigint AS home_runs,
                   COALESCE(SUM(bl.stolen_bases), 0)::bigint AS stolen_bases,
                   {rates}
            FROM batting_lines bl
            GROUP BY bl.player_id
            HAVING COALESCE(SUM(bl.pa), 0) > 0
        ) b
        JOIN players p ON p.id = b.player_id
        ORDER BY {order}, p.name
        LIMIT $1 OFFSET $2
        ",
        rates = super::BATTING_RATE_SQL
    )))
    .bind(i64::from(page_size))
    .bind(i64::from(page * page_size))
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let total = db_rows.first().map_or(0, |r| r.total);
    Ok(Page {
        items: db_rows
            .into_iter()
            .map(|r| PlayerBrowseRow {
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
                ops: r.ops,
            })
            .collect(),
        total,
        page,
        page_size,
    })
}

#[server]
pub async fn search_players(q: String, limit: u32) -> Result<Vec<PlayerHit>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: i32,
        bbref_id: String,
        name: String,
    }

    let q = q.trim().to_string();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let limit = limit.clamp(1, 200);
    // Escape LIKE wildcards so the search term is matched literally.
    let pattern = format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> =
        sqlx::query_as("SELECT id, bbref_id, name FROM players WHERE name ILIKE $1 ORDER BY name LIMIT $2")
            .bind(&pattern)
            .bind(i64::from(limit))
            .fetch_all(pool)
            .await
            .map_err(super::db_err)?;

    Ok(db_rows
        .into_iter()
        .map(|r| PlayerHit {
            id: r.id,
            bbref_id: r.bbref_id,
            name: r.name,
        })
        .collect())
}

#[allow(clippy::too_many_lines)]
#[server]
pub async fn player_detail(player_id: i32) -> Result<PlayerDetailDto, ServerFnError> {
    use crate::dto::{BattingTotals, PitchingTotals};

    #[derive(sqlx::FromRow)]
    struct PlayerRow {
        id: i32,
        bbref_id: String,
        name: String,
    }

    #[derive(sqlx::FromRow)]
    struct BattingRow {
        postseason: bool,
        games: i64,
        pa: i64,
        ab: i64,
        h: i64,
        r: i64,
        rbi: i64,
        bb: i64,
        so: i64,
        doubles: i64,
        triples: i64,
        home_runs: i64,
        stolen_bases: i64,
        avg: Option<f64>,
        obp: Option<f64>,
        slg: Option<f64>,
    }

    #[derive(sqlx::FromRow)]
    struct PitchingRow {
        postseason: bool,
        games: i64,
        outs: i64,
        h: i64,
        r: i64,
        er: i64,
        bb: i64,
        so: i64,
        hr: i64,
        era: Option<f64>,
        whip: Option<f64>,
    }

    let pool = crate::pool().await?;

    let player: Option<PlayerRow> = sqlx::query_as("SELECT id, bbref_id, name FROM players WHERE id = $1")
        .bind(player_id)
        .fetch_optional(pool)
        .await
        .map_err(super::db_err)?;
    let player = player.ok_or_else(|| ServerFnError::new(format!("player {player_id} not found")))?;

    // One row per postseason flag (0-2 rows).
    let batting_rows: Vec<BattingRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        WITH regular_end AS ({regular_end})
        SELECT (g.game_date > re.end_date) AS postseason,
               COUNT(*) AS games,
               COALESCE(SUM(bl.pa), 0)::bigint AS pa,
               COALESCE(SUM(bl.ab), 0)::bigint AS ab,
               COALESCE(SUM(bl.h), 0)::bigint AS h,
               COALESCE(SUM(bl.r), 0)::bigint AS r,
               COALESCE(SUM(bl.rbi), 0)::bigint AS rbi,
               COALESCE(SUM(bl.bb), 0)::bigint AS bb,
               COALESCE(SUM(bl.so), 0)::bigint AS so,
               {counts},
               {rates}
        FROM batting_lines bl
        JOIN games g ON g.id = bl.game_id
        JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
        WHERE bl.player_id = $1
        GROUP BY postseason
        ",
        regular_end = super::REGULAR_SEASON_END,
        counts = super::BATTING_COUNT_SQL,
        rates = super::BATTING_RATE_SQL
    )))
    .bind(player_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    // ip is stored in baseball notation (6.2 = 6 innings + 2 outs), so
    // convert to outs before aggregating.
    let pitching_rows: Vec<PitchingRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        WITH regular_end AS ({regular_end})
        SELECT postseason, games, outs, h, r, er, bb, so, hr,
               CASE WHEN outs > 0 THEN er::float8 * 27.0 / outs::float8 END AS era,
               CASE WHEN outs > 0 THEN (bb + h)::float8 * 3.0 / outs::float8 END AS whip
        FROM (
            SELECT (g.game_date > re.end_date) AS postseason,
                   COUNT(*) AS games,
                   COALESCE(SUM(FLOOR(pl.ip) * 3 + ROUND((pl.ip - FLOOR(pl.ip)) * 10)), 0)::bigint AS outs,
                   COALESCE(SUM(pl.h), 0)::bigint AS h,
                   COALESCE(SUM(pl.r), 0)::bigint AS r,
                   COALESCE(SUM(pl.er), 0)::bigint AS er,
                   COALESCE(SUM(pl.bb), 0)::bigint AS bb,
                   COALESCE(SUM(pl.so), 0)::bigint AS so,
                   COALESCE(SUM(pl.hr), 0)::bigint AS hr
            FROM pitching_lines pl
            JOIN games g ON g.id = pl.game_id
            JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
            WHERE pl.player_id = $1
            GROUP BY postseason
        ) totals
        ",
        regular_end = super::REGULAR_SEASON_END
    )))
    .bind(player_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let batting_totals = |r: &BattingRow| BattingTotals {
        games: r.games,
        pa: r.pa,
        ab: r.ab,
        h: r.h,
        r: r.r,
        rbi: r.rbi,
        bb: r.bb,
        so: r.so,
        doubles: r.doubles,
        triples: r.triples,
        home_runs: r.home_runs,
        stolen_bases: r.stolen_bases,
        avg: r.avg,
        obp: r.obp,
        slg: r.slg,
        ops: match (r.obp, r.slg) {
            (Some(obp), Some(slg)) => Some(obp + slg),
            _ => None,
        },
    };
    let pitching_totals = |r: &PitchingRow| PitchingTotals {
        games: r.games,
        outs: r.outs,
        h: r.h,
        r: r.r,
        er: r.er,
        bb: r.bb,
        so: r.so,
        hr: r.hr,
        era: r.era,
        whip: r.whip,
    };

    Ok(PlayerDetailDto {
        player: PlayerHit {
            id: player.id,
            bbref_id: player.bbref_id,
            name: player.name,
        },
        batting: batting_rows.iter().find(|r| !r.postseason).map(batting_totals),
        batting_postseason: batting_rows.iter().find(|r| r.postseason).map(batting_totals),
        pitching: pitching_rows.iter().find(|r| !r.postseason).map(pitching_totals),
        pitching_postseason: pitching_rows.iter().find(|r| r.postseason).map(pitching_totals),
    })
}

#[server]
pub async fn player_batting_log(
    player_id: i32,
    page: u32,
    page_size: u32,
) -> Result<Page<BattingGameLogRow>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        game_id: i32,
        game_date: chrono::NaiveDate,
        team_code: String,
        opponent_code: String,
        position: Option<String>,
        ab: Option<i32>,
        r: Option<i32>,
        h: Option<i32>,
        rbi: Option<i32>,
        bb: Option<i32>,
        so: Option<i32>,
        pa: Option<i32>,
        wpa: Option<f64>,
        total: i64,
    }

    let page_size = page_size.clamp(1, 200);
    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(
        r"
        SELECT bl.game_id, g.game_date, t.code AS team_code,
               CASE WHEN g.home_team_id = bl.team_id THEN ta.code ELSE th.code END AS opponent_code,
               bl.position, bl.ab, bl.r, bl.h, bl.rbi, bl.bb, bl.so, bl.pa,
               bl.wpa::float8 AS wpa,
               COUNT(*) OVER () AS total
        FROM batting_lines bl
        JOIN games g ON g.id = bl.game_id
        JOIN teams t ON t.id = bl.team_id
        JOIN teams th ON th.id = g.home_team_id
        JOIN teams ta ON ta.id = g.away_team_id
        WHERE bl.player_id = $1
        ORDER BY g.game_date DESC, bl.game_id DESC
        LIMIT $2 OFFSET $3
        ",
    )
    .bind(player_id)
    .bind(i64::from(page_size))
    .bind(i64::from(page) * i64::from(page_size))
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let total = db_rows.first().map_or(0, |r| r.total);
    Ok(Page {
        items: db_rows
            .into_iter()
            .map(|r| BattingGameLogRow {
                game_id: r.game_id,
                game_date: r.game_date,
                team_code: r.team_code,
                opponent_code: r.opponent_code,
                position: r.position,
                ab: r.ab,
                r: r.r,
                h: r.h,
                rbi: r.rbi,
                bb: r.bb,
                so: r.so,
                pa: r.pa,
                wpa: r.wpa,
            })
            .collect(),
        total,
        page,
        page_size,
    })
}

#[server]
pub async fn player_pitching_log(
    player_id: i32,
    page: u32,
    page_size: u32,
) -> Result<Page<PitchingGameLogRow>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        game_id: i32,
        game_date: chrono::NaiveDate,
        team_code: String,
        opponent_code: String,
        decision: Option<String>,
        ip: Option<f64>,
        h: Option<i32>,
        r: Option<i32>,
        er: Option<i32>,
        bb: Option<i32>,
        so: Option<i32>,
        hr: Option<i32>,
        pitches: Option<i32>,
        game_score: Option<i32>,
        total: i64,
    }

    let page_size = page_size.clamp(1, 200);
    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(
        r"
        SELECT pl.game_id, g.game_date, t.code AS team_code,
               CASE WHEN g.home_team_id = pl.team_id THEN ta.code ELSE th.code END AS opponent_code,
               pl.decision, pl.ip::float8 AS ip,
               pl.h, pl.r, pl.er, pl.bb, pl.so, pl.hr, pl.pitches, pl.game_score,
               COUNT(*) OVER () AS total
        FROM pitching_lines pl
        JOIN games g ON g.id = pl.game_id
        JOIN teams t ON t.id = pl.team_id
        JOIN teams th ON th.id = g.home_team_id
        JOIN teams ta ON ta.id = g.away_team_id
        WHERE pl.player_id = $1
        ORDER BY g.game_date DESC, pl.game_id DESC
        LIMIT $2 OFFSET $3
        ",
    )
    .bind(player_id)
    .bind(i64::from(page_size))
    .bind(i64::from(page) * i64::from(page_size))
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let total = db_rows.first().map_or(0, |r| r.total);
    Ok(Page {
        items: db_rows
            .into_iter()
            .map(|r| PitchingGameLogRow {
                game_id: r.game_id,
                game_date: r.game_date,
                team_code: r.team_code,
                opponent_code: r.opponent_code,
                decision: r.decision,
                ip: r.ip,
                h: r.h,
                r: r.r,
                er: r.er,
                bb: r.bb,
                so: r.so,
                hr: r.hr,
                pitches: r.pitches,
                game_score: r.game_score,
            })
            .collect(),
        total,
        page,
        page_size,
    })
}

#[server]
pub async fn player_batting_seasons(player_id: i32) -> Result<Vec<crate::dto::BattingSeasonRow>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        season: i32,
        postseason: bool,
        games: i64,
        pa: i64,
        ab: i64,
        h: i64,
        r: i64,
        rbi: i64,
        bb: i64,
        so: i64,
        doubles: i64,
        triples: i64,
        home_runs: i64,
        stolen_bases: i64,
        avg: Option<f64>,
        obp: Option<f64>,
        slg: Option<f64>,
        wpa: Option<f64>,
    }

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        WITH regular_end AS ({regular_end})
        SELECT EXTRACT(YEAR FROM g.game_date)::int4 AS season,
               (g.game_date > re.end_date) AS postseason,
               COUNT(*) AS games,
               COALESCE(SUM(bl.pa), 0)::bigint AS pa,
               COALESCE(SUM(bl.ab), 0)::bigint AS ab,
               COALESCE(SUM(bl.h), 0)::bigint AS h,
               COALESCE(SUM(bl.r), 0)::bigint AS r,
               COALESCE(SUM(bl.rbi), 0)::bigint AS rbi,
               COALESCE(SUM(bl.bb), 0)::bigint AS bb,
               COALESCE(SUM(bl.so), 0)::bigint AS so,
               {counts},
               {rates},
               SUM(bl.wpa)::float8 AS wpa
        FROM batting_lines bl
        JOIN games g ON g.id = bl.game_id
        JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
        WHERE bl.player_id = $1
        GROUP BY 1, 2
        ORDER BY season DESC, postseason
        ",
        regular_end = super::REGULAR_SEASON_END,
        counts = super::BATTING_COUNT_SQL,
        rates = super::BATTING_RATE_SQL
    )))
    .bind(player_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(db_rows
        .into_iter()
        .map(|r| crate::dto::BattingSeasonRow {
            season: r.season,
            postseason: r.postseason,
            games: r.games,
            pa: r.pa,
            ab: r.ab,
            h: r.h,
            r: r.r,
            rbi: r.rbi,
            bb: r.bb,
            so: r.so,
            doubles: r.doubles,
            triples: r.triples,
            home_runs: r.home_runs,
            stolen_bases: r.stolen_bases,
            avg: r.avg,
            obp: r.obp,
            slg: r.slg,
            ops: r.obp.zip(r.slg).map(|(o, s)| o + s),
            wpa: r.wpa,
        })
        .collect())
}

#[server]
pub async fn player_pitching_seasons(player_id: i32) -> Result<Vec<crate::dto::PitchingSeasonRow>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        season: i32,
        postseason: bool,
        games: i64,
        outs: i64,
        h: i64,
        r: i64,
        er: i64,
        bb: i64,
        so: i64,
        hr: i64,
        wins: i64,
        losses: i64,
        saves: i64,
        era: Option<f64>,
        whip: Option<f64>,
        wpa: Option<f64>,
    }

    let pool = crate::pool().await?;
    // Same conventions as the career totals: ip aggregated as outs,
    // decisions matched on their leading letter.
    let db_rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        WITH regular_end AS ({regular_end})
        SELECT season, postseason, games, outs, h, r, er, bb, so, hr, wins, losses, saves,
               CASE WHEN outs > 0 THEN er::float8 * 27.0 / outs::float8 END AS era,
               CASE WHEN outs > 0 THEN (bb + h)::float8 * 3.0 / outs::float8 END AS whip,
               wpa
        FROM (
            SELECT EXTRACT(YEAR FROM g.game_date)::int4 AS season,
                   (g.game_date > re.end_date) AS postseason,
                   COUNT(*) AS games,
                   COALESCE(SUM(FLOOR(pl.ip) * 3 + ROUND((pl.ip - FLOOR(pl.ip)) * 10)), 0)::bigint AS outs,
                   COALESCE(SUM(pl.h), 0)::bigint AS h,
                   COALESCE(SUM(pl.r), 0)::bigint AS r,
                   COALESCE(SUM(pl.er), 0)::bigint AS er,
                   COALESCE(SUM(pl.bb), 0)::bigint AS bb,
                   COALESCE(SUM(pl.so), 0)::bigint AS so,
                   COALESCE(SUM(pl.hr), 0)::bigint AS hr,
                   COUNT(*) FILTER (WHERE pl.decision LIKE 'W%') AS wins,
                   COUNT(*) FILTER (WHERE pl.decision LIKE 'L%') AS losses,
                   COUNT(*) FILTER (WHERE pl.decision LIKE 'S%') AS saves,
                   SUM(pl.wpa)::float8 AS wpa
            FROM pitching_lines pl
            JOIN games g ON g.id = pl.game_id
            JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
            WHERE pl.player_id = $1
            GROUP BY 1, 2
        ) totals
        ORDER BY season DESC, postseason
        ",
        regular_end = super::REGULAR_SEASON_END
    )))
    .bind(player_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(db_rows
        .into_iter()
        .map(|r| crate::dto::PitchingSeasonRow {
            season: r.season,
            postseason: r.postseason,
            games: r.games,
            outs: r.outs,
            h: r.h,
            r: r.r,
            er: r.er,
            bb: r.bb,
            so: r.so,
            hr: r.hr,
            wins: r.wins,
            losses: r.losses,
            saves: r.saves,
            era: r.era,
            whip: r.whip,
            wpa: r.wpa,
        })
        .collect())
}
