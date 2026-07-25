//! One-shot seeding from a JSON file (`christmas --seed <path>`).
//!
//! Idempotent: re-running adds anything missing and leaves existing rows alone,
//! so it is safe to run against a live database.

use serde::Deserialize;
use sqlx::PgPool;

use crate::model::{RelationshipKind, slugify};

#[derive(Debug, Deserialize)]
pub struct SeedFile {
    #[serde(default)]
    pub pools: Vec<SeedPool>,
    #[serde(default)]
    pub relationships: Vec<SeedRelationship>,
}

#[derive(Debug, Deserialize)]
pub struct SeedPool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub members: Vec<String>,
    /// The letters this pool draws from. Anything outside it is stored as
    /// excluded. Omit to permit the whole alphabet.
    #[serde(default)]
    pub letters_allowed: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SeedRelationship {
    pub a: String,
    pub b: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub note: Option<String>,
}

fn default_kind() -> String {
    "spouse".to_string()
}

pub async fn seed_from_path(db: &PgPool, path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let raw = std::fs::read_to_string(path)?;
    let parsed: SeedFile = serde_json::from_str(&raw)?;
    let summary = apply(db, &parsed).await?;
    Ok(summary)
}

pub async fn apply(db: &PgPool, seed: &SeedFile) -> Result<String, Box<dyn std::error::Error>> {
    let mut tx = db.begin().await?;
    let (mut pools_added, mut people_added, mut links_added, mut rels_added) = (0u32, 0u32, 0u32, 0u32);

    for (order, pool) in seed.pools.iter().enumerate() {
        let slug = slugify(&pool.name);

        let pool_id: i32 = sqlx::query_scalar(
            r"INSERT INTO pool (name, slug, description, sort_order)
              VALUES ($1, $2, $3, $4)
              ON CONFLICT (slug) DO UPDATE SET description = COALESCE(EXCLUDED.description, pool.description)
              RETURNING id",
        )
        .bind(&pool.name)
        .bind(&slug)
        .bind(&pool.description)
        .bind(i32::try_from(order).unwrap_or(0))
        .fetch_one(&mut *tx)
        .await?;
        pools_added += 1;

        for member in &pool.members {
            let participant_id: i32 = sqlx::query_scalar(
                r"INSERT INTO participant (name) VALUES ($1)
                  ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
                  RETURNING id",
            )
            .bind(member)
            .fetch_one(&mut *tx)
            .await?;
            people_added += 1;

            let linked = sqlx::query(
                "INSERT INTO pool_membership (pool_id, participant_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(pool_id)
            .bind(participant_id)
            .execute(&mut *tx)
            .await?;
            links_added += u32::try_from(linked.rows_affected()).unwrap_or(0);
        }

        if let Some(allowed) = &pool.letters_allowed {
            let allowed: Vec<char> = allowed.chars().filter(char::is_ascii_uppercase).collect();
            for letter in ('A'..='Z').filter(|c| !allowed.contains(c)) {
                sqlx::query("INSERT INTO excluded_letter (pool_id, letter) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                    .bind(pool_id)
                    .bind(letter.to_string())
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }

    for rel in &seed.relationships {
        let a_id: Option<i32> = sqlx::query_scalar("SELECT id FROM participant WHERE name = $1")
            .bind(&rel.a)
            .fetch_optional(&mut *tx)
            .await?;
        let b_id: Option<i32> = sqlx::query_scalar("SELECT id FROM participant WHERE name = $1")
            .bind(&rel.b)
            .fetch_optional(&mut *tx)
            .await?;

        let (Some(a_id), Some(b_id)) = (a_id, b_id) else {
            return Err(format!("relationship references unknown participant: {} / {}", rel.a, rel.b).into());
        };

        let (lo, hi) = if a_id < b_id { (a_id, b_id) } else { (b_id, a_id) };
        let kind = RelationshipKind::from_db(&rel.kind);

        let inserted = sqlx::query(
            r"INSERT INTO exclusion (participant_a_id, participant_b_id, kind, reason)
              VALUES ($1, $2, $3::relationship_kind, $4)
              ON CONFLICT (participant_a_id, participant_b_id) DO NOTHING",
        )
        .bind(lo)
        .bind(hi)
        .bind(kind.as_db())
        .bind(&rel.note)
        .execute(&mut *tx)
        .await?;
        rels_added += u32::try_from(inserted.rows_affected()).unwrap_or(0);
    }

    tx.commit().await?;

    Ok(format!(
        "seeded {pools_added} pools, {people_added} participant entries, \
         {links_added} new memberships, {rels_added} new relationships"
    ))
}
