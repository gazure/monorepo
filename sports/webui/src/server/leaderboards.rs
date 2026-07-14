use dioxus::prelude::*;

use crate::dto::{BattingLeaderRow, BattingLeaderboardReq, Page, PitchingLeaderRow, PitchingLeaderboardReq};

#[server]
#[allow(clippy::too_many_lines)]
pub async fn batting_leaderboard(req: BattingLeaderboardReq) -> Result<Page<BattingLeaderRow>, ServerFnError> {
    use crate::dto::BattingSort;

    #[derive(sqlx::FromRow)]
    struct Row {
        player_id: i32,
        name: String,
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
        ops: Option<f64>,
        wpa: Option<f64>,
        total: i64,
    }

    // Sort keys map to hardcoded column aliases; user input is never
    // interpolated into the SQL.
    let order = match req.sort {
        BattingSort::Ops => "ops DESC NULLS LAST",
        BattingSort::Avg => "avg DESC NULLS LAST",
        BattingSort::Obp => "obp DESC NULLS LAST",
        BattingSort::Slg => "slg DESC NULLS LAST",
        BattingSort::HomeRuns => "home_runs DESC",
        BattingSort::Doubles => "doubles DESC",
        BattingSort::Triples => "triples DESC",
        BattingSort::StolenBases => "stolen_bases DESC",
        BattingSort::Hits => "h DESC",
        BattingSort::Runs => "r DESC",
        BattingSort::Rbi => "rbi DESC",
        BattingSort::Walks => "bb DESC",
        BattingSort::Strikeouts => "so DESC",
        BattingSort::Pa => "pa DESC",
        BattingSort::Wpa => "wpa DESC NULLS LAST",
    };

    let limit = req.limit.clamp(1, 200);

    // Regular season only.
    let sql = format!(
        r"
        WITH regular_end AS ({regular_end})
        SELECT *, obp + slg AS ops, COUNT(*) OVER () AS total
        FROM (
            SELECT bl.player_id, p.name,
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
            JOIN players p ON p.id = bl.player_id
            JOIN games g ON g.id = bl.game_id
            JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
            WHERE g.game_date <= re.end_date
              AND ($4::int4 IS NULL OR EXTRACT(YEAR FROM g.game_date)::int4 = $4)
            GROUP BY bl.player_id, p.name
            HAVING COALESCE(SUM(bl.pa), 0) >= $1
        ) totals
        ORDER BY {order}
        LIMIT $2 OFFSET $3
        ",
        regular_end = super::REGULAR_SEASON_END,
        counts = super::BATTING_COUNT_SQL,
        rates = super::BATTING_RATE_SQL
    );

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(req.min_pa)
        .bind(i64::from(limit))
        .bind(i64::from(req.offset))
        .bind(req.season)
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;

    let total = db_rows.first().map_or(0, |r| r.total);
    Ok(Page {
        items: db_rows
            .into_iter()
            .map(|r| BattingLeaderRow {
                player_id: r.player_id,
                name: r.name,
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
                ops: r.ops,
                wpa: r.wpa,
            })
            .collect(),
        total,
        page: req.offset / limit,
        page_size: limit,
    })
}

#[server]
#[allow(clippy::too_many_lines)]
pub async fn pitching_leaderboard(req: PitchingLeaderboardReq) -> Result<Page<PitchingLeaderRow>, ServerFnError> {
    use crate::dto::PitchingSort;

    #[derive(sqlx::FromRow)]
    struct Row {
        player_id: i32,
        name: String,
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
        total: i64,
    }

    let order = match req.sort {
        PitchingSort::Era => "era ASC NULLS LAST",
        PitchingSort::Whip => "whip ASC NULLS LAST",
        PitchingSort::Strikeouts => "so DESC",
        PitchingSort::InningsPitched => "outs DESC",
        PitchingSort::Wins => "wins DESC",
        PitchingSort::Saves => "saves DESC",
        PitchingSort::Walks => "bb DESC",
        PitchingSort::HomeRuns => "hr DESC",
        PitchingSort::Wpa => "wpa DESC NULLS LAST",
    };

    let limit = req.limit.clamp(1, 200);

    // ip is stored in baseball notation (6.2 = 6 innings + 2 outs), so
    // aggregate as outs. Decisions are matched on their leading letter
    // ("W (1-0)", "L", "S (12)"); blown saves ("BS") don't match 'S%'.
    // Regular season only.
    let sql = format!(
        r"
        WITH regular_end AS ({regular_end})
        SELECT player_id, name, games, outs, h, r, er, bb, so, hr, wins, losses, saves,
               CASE WHEN outs > 0 THEN er::float8 * 27.0 / outs::float8 END AS era,
               CASE WHEN outs > 0 THEN (bb + h)::float8 * 3.0 / outs::float8 END AS whip,
               wpa,
               COUNT(*) OVER () AS total
        FROM (
            SELECT pl.player_id, p.name,
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
            JOIN players p ON p.id = pl.player_id
            JOIN games g ON g.id = pl.game_id
            JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
            WHERE g.game_date <= re.end_date
              AND ($4::int4 IS NULL OR EXTRACT(YEAR FROM g.game_date)::int4 = $4)
            GROUP BY pl.player_id, p.name
        ) totals
        WHERE outs >= $1
        ORDER BY {order}
        LIMIT $2 OFFSET $3
        ",
        regular_end = super::REGULAR_SEASON_END
    );

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(req.min_outs)
        .bind(i64::from(limit))
        .bind(i64::from(req.offset))
        .bind(req.season)
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;

    let total = db_rows.first().map_or(0, |r| r.total);
    Ok(Page {
        items: db_rows
            .into_iter()
            .map(|r| PitchingLeaderRow {
                player_id: r.player_id,
                name: r.name,
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
            .collect(),
        total,
        page: req.offset / limit,
        page_size: limit,
    })
}
