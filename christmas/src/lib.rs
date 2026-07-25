mod app;
pub mod auth;
mod components;
#[cfg(feature = "server")]
mod database;
pub mod matching;
pub mod model;
mod pages;
#[cfg(feature = "server")]
pub mod seed;
pub mod server;
pub mod storage;

pub use app::App;
#[cfg(feature = "server")]
use dioxus::prelude::ServerFnError;
#[cfg(feature = "server")]
use sqlx::PgPool;

#[cfg(feature = "server")]
const DEFAULT_DATABASE_URL: &str = "postgresql://postgres:postgres@localhost:30432/christmas";

#[cfg(feature = "server")]
static DB_URL: std::sync::OnceLock<String> = std::sync::OnceLock::new();

#[cfg(feature = "server")]
static POOL: tokio::sync::OnceCell<PgPool> = tokio::sync::OnceCell::const_new();

/// Resolves the database URL, preferring one set during boot (embedded mode).
#[cfg(feature = "server")]
fn database_url() -> String {
    DB_URL
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::var("CHRISTMAS_DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string()))
}

/// Lazily connect on first use so the pool is created inside the axum server's
/// runtime. Building it in `main` before `dioxus::launch` puts the initial
/// connection on a throwaway runtime whose I/O driver dies, which makes the
/// first request after boot stall until the acquire timeout.
///
/// Migrations run once, as part of this same initialization.
#[cfg(feature = "server")]
pub async fn pool() -> Result<&'static PgPool, ServerFnError> {
    POOL.get_or_try_init(|| async {
        let db_url = database_url();
        tracingx::info!(app = env!("CARGO_PKG_NAME"), "connecting to database");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&db_url)
            .await?;

        database::initialize(&pool).await?;
        tracingx::info!("database ready");
        Ok::<_, database::InitError>(pool)
    })
    .await
    .map_err(|e| ServerFnError::new(format!("database connection failed: {e}")))
}

/// Synchronous boot hook run before [`dioxus::launch`].
///
/// Handles the one-shot admin flags (`--seed`) and, when built with the
/// `embedded` feature, starts a bundled `PostgreSQL`. The database pool itself
/// is intentionally *not* built here — see [`pool`].
#[cfg(feature = "server")]
pub fn boot() {
    if std::env::var("CHRISTMAS_LOG_FORMAT").as_deref() == Ok("pretty") {
        tracingx::init_dev();
    } else {
        tracingx::init_prod();
    }

    tracingx::info!(
        app = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        "initializing"
    );

    auth::report_configuration();

    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--embedded") {
        start_embedded();
    }

    // `--seed <path>` is a one-shot admin command: seed, then exit.
    if let Some(idx) = args.iter().position(|a| a == "--seed") {
        let Some(path) = args.get(idx + 1) else {
            eprintln!("--seed requires a path to a seed file");
            std::process::exit(2);
        };
        run_seed(path);
    }
}

#[cfg(all(feature = "server", feature = "embedded"))]
fn start_embedded() {
    use postgresql_embedded::{PostgreSQL, Settings};

    let settings = Settings {
        password: "password".to_string(),
        data_dir: "./data/christmas".into(),
        port: 35432,
        ..Default::default()
    };

    let runtime = tokio::runtime::Runtime::new().expect("failed to build runtime for embedded postgres");
    runtime.block_on(async {
        let mut pg = PostgreSQL::new(settings);
        pg.setup().await.expect("failed to setup embedded PostgreSQL");
        pg.start().await.expect("failed to start embedded PostgreSQL");
        pg.create_database("christmas")
            .await
            .expect("failed to create christmas database");
        // Keep the instance alive for the life of the process; dropping it stops
        // the server.
        Box::leak(Box::new(pg));
    });
    // The embedded server is a child process supervised by this runtime, so the
    // runtime has to outlive `boot`.
    Box::leak(Box::new(runtime));

    DB_URL
        .set("postgresql://postgres:password@localhost:35432/christmas".to_string())
        .ok();
    tracingx::info!("embedded postgres started on port 35432");
}

#[cfg(all(feature = "server", not(feature = "embedded")))]
fn start_embedded() {
    eprintln!("--embedded requires building with the `embedded` feature: cargo run -p christmas --features embedded");
    std::process::exit(2);
}

#[cfg(feature = "server")]
fn run_seed(path: &str) {
    let runtime = tokio::runtime::Runtime::new().expect("failed to build runtime for seeding");
    let result = runtime.block_on(async {
        let pool = pool().await.map_err(|e| e.to_string())?;
        seed::seed_from_path(pool, path).await.map_err(|e| e.to_string())
    });

    match result {
        Ok(summary) => {
            println!("{summary}");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("seeding failed: {e}");
            std::process::exit(1);
        }
    }
}
