use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use dioxus::{
    fullstack::ServeConfigBuilder,
    prelude::{DioxusRouterExt, Element},
};
use postgresql_embedded::PostgreSQL;
use start::AppMeta;
use tokio::net::TcpListener;

const MIGRATIONS: &str = include_str!("../../migrations/initial.sql");

pub async fn launch(app: fn() -> Element) {
    tracingx::init_logging();
    let app_meta = AppMeta::from_env();
    let root_span = tracing::info_span!("app", app = %app_meta.app, region = %app_meta.region, host = %app_meta.host);
    let _span = root_span.enter();

    let mut db = PostgreSQL::default();
    db.setup().await.expect("db setup failed");
    db.start().await.expect("db start failed");
    db.create_database("christmas").await.expect("db create failed");

    let db_conn = sqlx::PgPool::connect(&db.settings().url("christmas"))
        .await
        .expect("db connection failed");
    tracing::info!("running migrations...");

    // Execute each SQL statement separately
    for statement in MIGRATIONS.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        tracing::info!("running: {statement}");
        sqlx::query(statement)
            .execute(&db_conn)
            .await
            .map_err(|e| {
                tracing::error!("Failed to execute migration statement: {}", statement);
                e
            })
            .expect("db migration failed");
    }
    sqlx::raw_sql(MIGRATIONS)
        .execute(&db_conn)
        .await
        .expect("db migration failed");

    let ip = dioxus::cli_config::server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = dioxus::cli_config::server_port().unwrap_or(8080);
    let address = SocketAddr::new(ip, port);
    let listener = TcpListener::bind(address).await.unwrap();
    let router = axum::Router::new()
        // serve_dioxus_application adds routes to server side render the application, serve static assets, and register server functions
        .serve_dioxus_application(ServeConfigBuilder::default().context(db_conn).build().unwrap(), app)
        .into_make_service();

    tracing::info!(port = port, address = ip.to_string(), "Server started successfully");
    axum::serve(listener, router).await.unwrap();
}
