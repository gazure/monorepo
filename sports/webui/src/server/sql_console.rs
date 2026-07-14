use dioxus::prelude::*;

use crate::dto::SqlResult;

#[cfg(feature = "server")]
const MAX_ROWS: usize = 1000;

/// Run an ad-hoc read-only query. Guarded in layers: a first-token allowlist,
/// single-statement execution via the extended query protocol, a READ ONLY
/// transaction (always rolled back) and a 5s statement timeout.
#[server]
pub async fn run_sql(sql: String) -> Result<SqlResult, ServerFnError> {
    use futures_util::TryStreamExt;
    use sqlx::{Column, Row};

    let stripped = strip_leading_comments(&sql);
    let first_token = stripped
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(
        first_token.as_str(),
        "select" | "with" | "explain" | "show" | "table" | "values"
    ) {
        return Err(ServerFnError::new(
            "Only read-only queries are allowed (SELECT, WITH, EXPLAIN, SHOW, TABLE, VALUES)",
        ));
    }

    let pool = crate::pool().await?;
    let mut tx = pool.begin().await.map_err(super::db_err)?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(super::db_err)?;
    sqlx::query("SET LOCAL statement_timeout = 5000")
        .execute(&mut *tx)
        .await
        .map_err(super::db_err)?;

    let started = std::time::Instant::now();
    let mut columns: Vec<String> = Vec::new();
    let mut out_rows: Vec<Vec<Option<String>>> = Vec::new();
    let mut truncated = false;
    {
        // Deliberately user-supplied SQL; the READ ONLY transaction, timeout
        // and single-statement protocol above are the guards.
        let mut stream = sqlx::query(sqlx::AssertSqlSafe(stripped.to_string())).fetch(&mut *tx);
        while let Some(row) = stream.try_next().await.map_err(super::db_err)? {
            if columns.is_empty() {
                columns = row.columns().iter().map(|c| c.name().to_string()).collect();
            }
            if out_rows.len() >= MAX_ROWS {
                truncated = true;
                break;
            }
            out_rows.push(render_row(&row));
        }
    }
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    tx.rollback().await.map_err(super::db_err)?;

    Ok(SqlResult {
        row_count: out_rows.len(),
        columns,
        rows: out_rows,
        truncated,
        elapsed_ms,
    })
}

/// Skip leading whitespace, `-- line` and `/* block */` comments so the
/// allowlist sees the first real token.
#[cfg(feature = "server")]
fn strip_leading_comments(sql: &str) -> &str {
    let mut rest = sql.trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("--") {
            rest = after.split_once('\n').map_or("", |(_, tail)| tail).trim_start();
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = after.split_once("*/").map_or("", |(_, tail)| tail).trim_start();
        } else {
            return rest;
        }
    }
}

#[cfg(feature = "server")]
fn render_row(row: &sqlx::postgres::PgRow) -> Vec<Option<String>> {
    use sqlx::Row;

    (0..row.len()).map(|idx| render_value(row, idx)).collect()
}

/// Decode one cell to a display string based on its Postgres type name.
/// Unknown types render as `<TYPENAME>` rather than erroring the whole query.
#[cfg(feature = "server")]
fn render_value(row: &sqlx::postgres::PgRow, idx: usize) -> Option<String> {
    use sqlx::{Row, TypeInfo, ValueRef};

    let type_name = {
        let Ok(raw) = row.try_get_raw(idx) else {
            return Some("<error>".to_string());
        };
        if raw.is_null() {
            return None;
        }
        raw.type_info().name().to_string()
    };

    let rendered = match type_name.as_str() {
        "INT2" => row.try_get::<i16, _>(idx).map(|v| v.to_string()).ok(),
        "INT4" => row.try_get::<i32, _>(idx).map(|v| v.to_string()).ok(),
        "INT8" => row.try_get::<i64, _>(idx).map(|v| v.to_string()).ok(),
        "FLOAT4" => row.try_get::<f32, _>(idx).map(|v| v.to_string()).ok(),
        "FLOAT8" => row.try_get::<f64, _>(idx).map(|v| v.to_string()).ok(),
        "NUMERIC" => row.try_get::<sqlx::types::Decimal, _>(idx).map(|v| v.to_string()).ok(),
        "BOOL" => row.try_get::<bool, _>(idx).map(|v| v.to_string()).ok(),
        "DATE" => row.try_get::<chrono::NaiveDate, _>(idx).map(|v| v.to_string()).ok(),
        "TIME" => row.try_get::<chrono::NaiveTime, _>(idx).map(|v| v.to_string()).ok(),
        "TIMESTAMP" => row.try_get::<chrono::NaiveDateTime, _>(idx).map(|v| v.to_string()).ok(),
        "TIMESTAMPTZ" => row
            .try_get::<chrono::DateTime<chrono::Utc>, _>(idx)
            .map(|v| v.to_string())
            .ok(),
        "UUID" => row.try_get::<sqlx::types::Uuid, _>(idx).map(|v| v.to_string()).ok(),
        _ => row.try_get::<String, _>(idx).ok(),
    };

    Some(rendered.unwrap_or_else(|| format!("<{type_name}>")))
}
