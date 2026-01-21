mod batting;
mod box_score;
mod failed_scrapes;
mod games;
mod pitching;
mod play_by_play;
mod players;
mod teams;

use std::time::Duration;

pub use box_score::{BoxScoreInserter, InsertError};
pub use failed_scrapes::{FailedScrape, FailedScrapesDb};
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Create a database connection pool
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
