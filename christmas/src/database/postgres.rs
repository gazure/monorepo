use sqlx::PgPool;

use crate::{
    Result,
    model::{Exchange, ExchangeAppConfig},
};
const PARTICIPANTS: &str = include_str!("../../assets/participants.yaml");

pub async fn initialize(conn: &PgPool) -> Result<()> {
    tracingx::info!("running migrations...");

    let migrations_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    sqlx::migrate::Migrator::new(migrations_path).await?.run(conn).await?;

    let config: ExchangeAppConfig = serde_yaml::from_str(PARTICIPANTS)?;

    for ex in config.exchanges {
        write_exchange(conn, ex).await?;
    }

    Ok(())
}

async fn write_exchange(conn: &PgPool, exchange: Exchange) -> Result<Exchange> {
    let row = sqlx::query!(
        "INSERT INTO exchange (name, year, letters) VALUES ($1, 2025, $2) RETURNING id, name, letters",
        exchange.name,
        exchange.letters,
    )
    .fetch_one(conn)
    .await?;

    Ok(Exchange {
        name: row.name,
        letters: row.letters,
    })
}
