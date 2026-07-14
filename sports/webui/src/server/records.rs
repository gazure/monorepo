use dioxus::prelude::*;

use crate::dto::{RecordBoard, RecordRow};

/// Best single seasons ever (regular season), optionally within one decade.
/// Rate-stat boards require 400 PA.
#[server]
pub async fn single_season_records(decade: Option<i32>) -> Result<Vec<RecordBoard>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        board: String,
        player_id: i32,
        name: String,
        season: i32,
        value: Option<f64>,
    }

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        r"
        WITH regular_end AS ({regular_end}),
        ps AS (
            SELECT bl.player_id, p.name,
                   EXTRACT(YEAR FROM g.game_date)::int4 AS season,
                   COALESCE(SUM(bl.pa), 0)::bigint AS pa,
                   COALESCE(SUM(bl.h), 0)::bigint AS h,
                   COALESCE(SUM(bl.rbi), 0)::bigint AS rbi,
                   {counts},
                   {rates}
            FROM batting_lines bl
            JOIN players p ON p.id = bl.player_id
            JOIN games g ON g.id = bl.game_id
            JOIN regular_end re ON re.season = EXTRACT(YEAR FROM g.game_date)::int4
            WHERE g.game_date <= re.end_date
              AND ($1::int4 IS NULL OR EXTRACT(YEAR FROM g.game_date)::int4 BETWEEN $1 AND $1 + 9)
            GROUP BY 1, 2, 3
        )
        SELECT 'hr' AS board, player_id, name, season, home_runs::float8 AS value
        FROM (SELECT * FROM ps ORDER BY home_runs DESC, season LIMIT 10) hr_board
        UNION ALL
        SELECT 'sb', player_id, name, season, stolen_bases::float8
        FROM (SELECT * FROM ps ORDER BY stolen_bases DESC, season LIMIT 10) sb_board
        UNION ALL
        SELECT 'h', player_id, name, season, h::float8
        FROM (SELECT * FROM ps ORDER BY h DESC, season LIMIT 10) h_board
        UNION ALL
        SELECT 'rbi', player_id, name, season, rbi::float8
        FROM (SELECT * FROM ps ORDER BY rbi DESC, season LIMIT 10) rbi_board
        UNION ALL
        SELECT 'avg', player_id, name, season, avg
        FROM (SELECT * FROM ps WHERE pa >= 400 ORDER BY avg DESC NULLS LAST, season LIMIT 10) avg_board
        UNION ALL
        SELECT 'ops', player_id, name, season, obp + slg
        FROM (SELECT * FROM ps WHERE pa >= 400 ORDER BY obp + slg DESC NULLS LAST, season LIMIT 10) ops_board
        ",
        regular_end = super::REGULAR_SEASON_END,
        counts = super::BATTING_COUNT_SQL,
        rates = super::BATTING_RATE_SQL
    )))
    .bind(decade)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let mut boards: Vec<RecordBoard> = ["hr", "sb", "h", "rbi", "avg", "ops"]
        .iter()
        .map(|k| RecordBoard {
            key: (*k).to_string(),
            rows: Vec::new(),
        })
        .collect();
    for r in db_rows {
        let is_rate = r.board == "avg" || r.board == "ops";
        let value = r.value.map_or(String::new(), |v| {
            if is_rate { format!("{v:.3}") } else { format!("{v:.0}") }
        });
        if let Some(board) = boards.iter_mut().find(|b| b.key == r.board) {
            board.rows.push(RecordRow {
                player_id: r.player_id,
                name: r.name,
                season: r.season,
                value,
            });
        }
    }
    Ok(boards)
}
