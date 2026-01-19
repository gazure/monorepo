use sqlx::PgPool;

use crate::models::{NewPlayer, Player};

/// Upsert a player by `bbref_id`, returning the player with its ID
pub async fn upsert_player(pool: &PgPool, player: &NewPlayer) -> Result<Player, sqlx::Error> {
    sqlx::query_as::<_, Player>(
        r"
        INSERT INTO players (bbref_id, name)
        VALUES ($1, $2)
        ON CONFLICT (bbref_id) DO UPDATE SET
            name = EXCLUDED.name,
            updated_at = NOW()
        RETURNING id, bbref_id, name, created_at, updated_at
        ",
    )
    .bind(&player.bbref_id)
    .bind(&player.name)
    .fetch_one(pool)
    .await
}
