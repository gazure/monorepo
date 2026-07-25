use dioxus::prelude::*;

use crate::model::Participant;

#[server]
pub async fn list_participants() -> Result<Vec<Participant>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: i32,
        name: String,
    }

    let pool = crate::pool().await?;
    let rows: Vec<Row> = sqlx::query_as("SELECT id, name FROM participant ORDER BY name")
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;

    Ok(rows
        .into_iter()
        .map(|r| Participant { id: r.id, name: r.name })
        .collect())
}

#[server]
pub async fn add_participant(name: String, pool_ids: Vec<i32>) -> Result<Participant, ServerFnError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Name cannot be empty"));
    }

    let db = crate::pool().await?;
    let mut tx = db.begin().await.map_err(super::db_err)?;

    let row: (i32,) = sqlx::query_as("INSERT INTO participant (name) VALUES ($1) RETURNING id")
        .bind(&name)
        .fetch_one(&mut *tx)
        .await
        .map_err(super::db_err)?;

    for pool_id in pool_ids {
        sqlx::query("INSERT INTO pool_membership (pool_id, participant_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(pool_id)
            .bind(row.0)
            .execute(&mut *tx)
            .await
            .map_err(super::db_err)?;
    }

    tx.commit().await.map_err(super::db_err)?;

    Ok(Participant { id: row.0, name })
}

#[server]
pub async fn remove_participant(id: i32) -> Result<(), ServerFnError> {
    let pool = crate::pool().await?;
    // Past exchanges keep name snapshots in JSONB, so history survives this.
    sqlx::query("DELETE FROM participant WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(super::db_err)?;
    Ok(())
}
