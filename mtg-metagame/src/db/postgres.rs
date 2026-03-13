use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::info;

use crate::models::{Deck, DeckCard, Tournament};

pub async fn connect(db_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new().max_connections(5).connect(db_url).await?;
    Ok(pool)
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

pub async fn upsert_tournament(pool: &PgPool, tournament: &Tournament) -> Result<i32> {
    let row = sqlx::query_scalar::<_, i32>(
        "INSERT INTO tournament (goldfish_id, name, format, date, url)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (goldfish_id) DO UPDATE SET
             name = EXCLUDED.name,
             format = EXCLUDED.format,
             date = EXCLUDED.date,
             url = EXCLUDED.url,
             scraped_at = NOW()
         RETURNING id",
    )
    .bind(tournament.goldfish_id)
    .bind(&tournament.name)
    .bind(&tournament.format)
    .bind(tournament.date)
    .bind(&tournament.url)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn upsert_archetype(pool: &PgPool, name: &str, format: &str, url: Option<&str>) -> Result<i32> {
    let row = sqlx::query_scalar::<_, i32>(
        "INSERT INTO archetype (name, format, url)
         VALUES ($1, $2, $3)
         ON CONFLICT (name, format) DO UPDATE SET
             url = COALESCE(EXCLUDED.url, archetype.url)
         RETURNING id",
    )
    .bind(name)
    .bind(format)
    .bind(url)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn upsert_deck(
    pool: &PgPool,
    deck: &Deck,
    tournament_id: Option<i32>,
    archetype_id: Option<i32>,
    cards: &[DeckCard],
) -> Result<i32> {
    let mut tx = pool.begin().await?;

    let deck_id = sqlx::query_scalar::<_, i32>(
        "INSERT INTO deck (goldfish_id, tournament_id, archetype_id, player_name, placement, format, date, url)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (goldfish_id) DO UPDATE SET
             tournament_id = EXCLUDED.tournament_id,
             archetype_id = EXCLUDED.archetype_id,
             player_name = EXCLUDED.player_name,
             placement = EXCLUDED.placement,
             format = EXCLUDED.format,
             date = EXCLUDED.date,
             url = EXCLUDED.url,
             scraped_at = NOW()
         RETURNING id",
    )
    .bind(deck.goldfish_id)
    .bind(tournament_id)
    .bind(archetype_id)
    .bind(&deck.player_name)
    .bind(&deck.placement)
    .bind(&deck.format)
    .bind(deck.date)
    .bind(&deck.url)
    .fetch_one(&mut *tx)
    .await?;

    // Delete existing cards and re-insert (simpler than upsert for card lists)
    sqlx::query("DELETE FROM deck_card WHERE deck_id = $1")
        .bind(deck_id)
        .execute(&mut *tx)
        .await?;

    for card in cards {
        sqlx::query(
            "INSERT INTO deck_card (deck_id, card_name, quantity, is_sideboard)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(deck_id)
        .bind(&card.card_name)
        .bind(card.quantity)
        .bind(card.is_sideboard)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(deck_id)
}

pub async fn stats(pool: &PgPool, format: &str) -> Result<()> {
    let tournament_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tournament WHERE format = $1")
        .bind(format)
        .fetch_one(pool)
        .await?;

    let archetype_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM archetype WHERE format = $1")
        .bind(format)
        .fetch_one(pool)
        .await?;

    let deck_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deck WHERE format = $1")
        .bind(format)
        .fetch_one(pool)
        .await?;

    let card_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deck_card dc
             JOIN deck d ON dc.deck_id = d.id
             WHERE d.format = $1",
    )
    .bind(format)
    .fetch_one(pool)
    .await?;

    info!("=== {format} metagame stats ===");
    info!("Tournaments: {tournament_count}");
    info!("Archetypes:  {archetype_count}");
    info!("Decks:       {deck_count}");
    info!("Card entries: {card_count}");

    Ok(())
}
