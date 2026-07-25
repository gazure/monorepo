use dioxus::prelude::*;

use crate::model::{Pool, PoolDetail};

#[cfg(feature = "server")]
const POOL_SELECT: &str = r"
    SELECT p.id, p.name, p.slug, p.description, p.sort_order,
           COALESCE(COUNT(m.participant_id), 0) AS member_count
    FROM pool p
    LEFT JOIN pool_membership m ON m.pool_id = p.id
    GROUP BY p.id
";

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct PoolRow {
    id: i32,
    name: String,
    slug: String,
    description: Option<String>,
    sort_order: i32,
    member_count: i64,
}

#[cfg(feature = "server")]
impl From<PoolRow> for Pool {
    fn from(r: PoolRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            slug: r.slug,
            description: r.description,
            sort_order: r.sort_order,
            member_count: r.member_count,
        }
    }
}

#[server]
pub async fn list_pools() -> Result<Vec<Pool>, ServerFnError> {
    let db = crate::pool().await?;
    // Composed from a const fragment and a literal — no caller input involved.
    let rows: Vec<PoolRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{POOL_SELECT} ORDER BY p.sort_order, p.name"
    )))
    .fetch_all(db)
    .await
    .map_err(super::db_err)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

#[server]
pub async fn create_pool(name: String, description: Option<String>) -> Result<Pool, ServerFnError> {
    use crate::model::slugify;

    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Pool name cannot be empty"));
    }
    let slug = slugify(&name);

    let db = crate::pool().await?;
    let row: (i32, i32) = sqlx::query_as(
        r"INSERT INTO pool (name, slug, description, sort_order)
          VALUES ($1, $2, $3, COALESCE((SELECT MAX(sort_order) + 1 FROM pool), 0))
          RETURNING id, sort_order",
    )
    .bind(&name)
    .bind(&slug)
    .bind(&description)
    .fetch_one(db)
    .await
    .map_err(super::db_err)?;

    Ok(Pool {
        id: row.0,
        name,
        slug,
        description,
        sort_order: row.1,
        member_count: 0,
    })
}

#[server]
pub async fn delete_pool(id: i32) -> Result<(), ServerFnError> {
    let db = crate::pool().await?;
    sqlx::query("DELETE FROM pool WHERE id = $1")
        .bind(id)
        .execute(db)
        .await
        .map_err(super::db_err)?;
    Ok(())
}

/// Every (pool, participant) link, so the admin table can render checkboxes
/// without a query per row.
#[server]
pub async fn list_memberships() -> Result<Vec<(i32, i32)>, ServerFnError> {
    let db = crate::pool().await?;
    let rows: Vec<(i32, i32)> = sqlx::query_as("SELECT pool_id, participant_id FROM pool_membership")
        .fetch_all(db)
        .await
        .map_err(super::db_err)?;
    Ok(rows)
}

#[server]
pub async fn add_member(pool_id: i32, participant_id: i32) -> Result<(), ServerFnError> {
    let db = crate::pool().await?;
    sqlx::query("INSERT INTO pool_membership (pool_id, participant_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(pool_id)
        .bind(participant_id)
        .execute(db)
        .await
        .map_err(super::db_err)?;
    Ok(())
}

#[server]
pub async fn remove_member(pool_id: i32, participant_id: i32) -> Result<(), ServerFnError> {
    let db = crate::pool().await?;
    sqlx::query("DELETE FROM pool_membership WHERE pool_id = $1 AND participant_id = $2")
        .bind(pool_id)
        .bind(participant_id)
        .execute(db)
        .await
        .map_err(super::db_err)?;
    Ok(())
}

/// Everything the pool page needs in one round trip.
#[server]
pub async fn pool_detail(slug: String) -> Result<PoolDetail, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct MemberRow {
        id: i32,
        name: String,
    }

    use crate::model::Participant;

    let db = crate::pool().await?;

    // The slug is a bound parameter; only const fragments are interpolated.
    let pool_row: Option<PoolRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{POOL_SELECT} HAVING p.slug = $1")))
        .bind(&slug)
        .fetch_optional(db)
        .await
        .map_err(super::db_err)?;

    let Some(pool_row) = pool_row else {
        return Err(ServerFnError::new(format!("No pool named '{slug}'")));
    };
    let pool: Pool = pool_row.into();

    let members: Vec<MemberRow> = sqlx::query_as(
        r"SELECT p.id, p.name
          FROM participant p
          JOIN pool_membership m ON m.participant_id = p.id
          WHERE m.pool_id = $1
          ORDER BY p.name",
    )
    .bind(pool.id)
    .fetch_all(db)
    .await
    .map_err(super::db_err)?;

    let letters: Vec<(String,)> =
        sqlx::query_as("SELECT letter FROM excluded_letter WHERE pool_id = $1 ORDER BY letter")
            .bind(pool.id)
            .fetch_all(db)
            .await
            .map_err(super::db_err)?;

    let all = super::draw::load_exchanges(db, Some(pool.id)).await?;
    let exchanges = if crate::auth::caller_role().await == crate::auth::Role::Manager {
        all
    } else {
        super::draw::only_current_revisions(all)
    };

    Ok(PoolDetail {
        pool,
        members: members
            .into_iter()
            .map(|m| Participant { id: m.id, name: m.name })
            .collect(),
        excluded_letters: letters.into_iter().filter_map(|r| r.0.chars().next()).collect(),
        exchanges,
    })
}
