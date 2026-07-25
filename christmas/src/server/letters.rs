use dioxus::prelude::*;

#[server]
pub async fn list_excluded_letters(pool_id: i32) -> Result<Vec<char>, ServerFnError> {
    let db = crate::pool().await?;
    let rows: Vec<(String,)> = sqlx::query_as("SELECT letter FROM excluded_letter WHERE pool_id = $1 ORDER BY letter")
        .bind(pool_id)
        .fetch_all(db)
        .await
        .map_err(super::db_err)?;

    Ok(rows.into_iter().filter_map(|r| r.0.chars().next()).collect())
}

/// Excluded letters for every pool at once.
///
/// The admin page fetches these up front rather than per-pool: a resource whose
/// only input is a component prop never re-runs when that prop changes, so it
/// would go stale as soon as the pool list loaded.
#[server]
pub async fn list_all_excluded_letters() -> Result<Vec<(i32, String)>, ServerFnError> {
    let db = crate::pool().await?;
    let rows: Vec<(i32, String)> =
        sqlx::query_as("SELECT pool_id, letter FROM excluded_letter ORDER BY pool_id, letter")
            .fetch_all(db)
            .await
            .map_err(super::db_err)?;
    Ok(rows)
}

#[server]
pub async fn set_excluded_letters(pool_id: i32, letters: Vec<char>) -> Result<(), ServerFnError> {
    let db = crate::pool().await?;
    let mut tx = db.begin().await.map_err(super::db_err)?;

    sqlx::query("DELETE FROM excluded_letter WHERE pool_id = $1")
        .bind(pool_id)
        .execute(&mut *tx)
        .await
        .map_err(super::db_err)?;

    for letter in letters.iter().filter(|c| c.is_ascii_uppercase()) {
        sqlx::query("INSERT INTO excluded_letter (pool_id, letter) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(pool_id)
            .bind(letter.to_string())
            .execute(&mut *tx)
            .await
            .map_err(super::db_err)?;
    }

    tx.commit().await.map_err(super::db_err)?;
    Ok(())
}
