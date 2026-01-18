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

/// Get a player by `bbref_id`
pub async fn get_player_by_bbref_id(pool: &PgPool, bbref_id: &str) -> Result<Option<Player>, sqlx::Error> {
    sqlx::query_as::<_, Player>(
        r"
        SELECT id, bbref_id, name, created_at, updated_at
        FROM players
        WHERE bbref_id = $1
        ",
    )
    .bind(bbref_id)
    .fetch_optional(pool)
    .await
}

/// Batch upsert players, returning all with their IDs
pub async fn upsert_players(pool: &PgPool, players: &[NewPlayer]) -> Result<Vec<Player>, sqlx::Error> {
    let mut results = Vec::with_capacity(players.len());

    for player in players {
        let result = upsert_player(pool, player).await?;
        results.push(result);
    }

    Ok(results)
}
