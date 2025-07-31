use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use dioxus::{
    fullstack::ServeConfigBuilder,
    prelude::{DioxusRouterExt, Element},
};
use postgresql_embedded::{PostgreSQL, Settings};
use start::AppMeta;
use tokio::net::TcpListener;

use crate::database;

pub async fn launch(app: fn() -> Element, use_embedded: bool) {
    tracingx::init_logging();
    let app_meta = AppMeta::from_env();
    let root_span = tracing::info_span!("app", app = %app_meta.app, region = %app_meta.region, host = %app_meta.host);
    let _span = root_span.enter();

    let mut db = None::<PostgreSQL>;
    let mut url = "postgresql://postgres:password@localhost:35432/postgres".to_owned();
    if use_embedded {
        let mut embedded_db = PostgreSQL::new(Settings {
            port: 35432,
            password: "password".to_string(),
            data_dir: "./data/christmas".into(),
            ..Default::default()
        });
        embedded_db.setup().await.expect("db setup failed");
        embedded_db.start().await.expect("db start failed");
        embedded_db.create_database("christmas").await.expect("db create failed");
        url = embedded_db.settings().url("chirstmas");
        db = Some(embedded_db);
    }

    tracing::info!("using database: {url}");
    let db_conn = sqlx::PgPool::connect(&url)
        .await
        .expect("db connection failed");

    database::initialize(&db_conn).await.expect("db initialize failed");

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

    if let Some(db) = db {
        db.stop().await.expect("db stop failed");
    }
}
