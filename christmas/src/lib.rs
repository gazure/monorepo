mod app;
mod components;
#[cfg(feature = "server")]
mod database;
mod matching;
pub mod server;

pub use app::App;
#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use sqlx::PgPool;

#[cfg(feature = "server")]
static POOL: std::sync::OnceLock<PgPool> = std::sync::OnceLock::new();

#[cfg(feature = "server")]
pub fn set_pool(pool: PgPool) {
    POOL.set(pool).expect("Pool already set");
}

#[cfg(feature = "server")]
pub fn pool() -> Result<&'static PgPool, ServerFnError> {
    POOL.get()
        .ok_or_else(|| ServerFnError::new("Database pool not initialized"))
}

#[cfg(feature = "server")]
pub async fn init_server() {
    use tracingx::info;

    info!(
        app = env!("CARGO_PKG_NAME"),
        region = option_env!("FLY_REGION").unwrap_or("local"),
        host = option_env!("FLY_ALLOC_ID").unwrap_or("local"),
        "initializing"
    );

    let use_embedded = std::env::args().any(|arg| arg == "--embedded");

    let db_url = if use_embedded {
        use postgresql_embedded::{PostgreSQL, Settings};

        let settings = Settings {
            password: "password".to_string(),
            data_dir: "./data/christmas".into(),
            port: 35432,
            ..Default::default()
        };

        let mut pg = PostgreSQL::new(settings);
        pg.setup().await.expect("Failed to setup PostgreSQL");
        pg.start().await.expect("Failed to start PostgreSQL");

        // Leak the PostgreSQL instance to keep it running
        Box::leak(Box::new(pg));

        "postgresql://postgres:password@localhost:35432/christmas".to_string()
    } else {
        std::env::var("CHRISTMAS_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:30432/christmas".into())
    };

    let pool = PgPool::connect(&db_url).await.expect("Failed to connect to database");

    database::initialize(&pool)
        .await
        .expect("Failed to initialize database");

    set_pool(pool);

    info!("server initialized");
}
