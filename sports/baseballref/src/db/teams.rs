use sqlx::PgPool;

use crate::models::{NewTeam, Team};

/// Upsert a team by code, returning the team with its ID
pub async fn upsert_team(pool: &PgPool, team: &NewTeam) -> Result<Team, sqlx::Error> {
    sqlx::query_as::<_, Team>(
        r"
        INSERT INTO teams (code, name)
        VALUES ($1, $2)
        ON CONFLICT (code) DO UPDATE SET name = EXCLUDED.name
        RETURNING id, code, name, created_at
        ",
    )
    .bind(&team.code)
    .bind(&team.name)
    .fetch_one(pool)
    .await
}
