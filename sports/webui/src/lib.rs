// The dioxus prelude is designed for glob imports, and #[server]-generated
// code can't carry doc sections.
#![allow(clippy::wildcard_imports, clippy::missing_panics_doc)]

mod app;
mod bbref;
mod components;
mod dto;
mod fmt;
mod pages;
pub mod server;

pub use app::App;
#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use sqlx::PgPool;

#[cfg(feature = "server")]
static POOL: tokio::sync::OnceCell<PgPool> = tokio::sync::OnceCell::const_new();

/// Lazily connect on first use so the pool is created inside the axum server's
/// runtime. Building it in `main` before `dioxus::launch` puts the initial
/// connection on a throwaway runtime whose I/O driver dies, which made the
/// first request after boot stall until the acquire timeout.
#[cfg(feature = "server")]
pub async fn pool() -> Result<&'static PgPool, ServerFnError> {
    POOL.get_or_try_init(|| async {
        let db_url = std::env::var("SPORTS_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/sports".into());

        tracingx::info!(app = env!("CARGO_PKG_NAME"), "connecting to database");

        // Pages fire several server fns concurrently, so keep enough headroom
        // that a burst doesn't hit the acquire timeout.
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&db_url)
            .await
    })
    .await
    .map_err(|e| ServerFnError::new(format!("database connection failed: {e}")))
}
