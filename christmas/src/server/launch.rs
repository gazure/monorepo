use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use dioxus::prelude::DioxusRouterExt;
use postgresql_embedded::{PostgreSQL, Settings};
use start::AppMeta;
use tokio::net::TcpListener;

use crate::database;

pub async fn launch(app: fn() -> dioxus::prelude::Element, use_embedded: bool) {
    tracingx::init_logging();
    let app_meta = AppMeta::from_env();
    let root_span = tracingx::info_span!("app", app = %app_meta.app, region = %app_meta.region, host = %app_meta.host);
    let _span = root_span.enter();

    let mut db = None::<PostgreSQL>;
    let mut url = "postgresql://postgres:postgres@localhost:30432/christmas".to_owned();
    if use_embedded {
        let mut embedded_db = PostgreSQL::new(Settings {
            port: 35432,
            password: "password".to_string(),
            data_dir: "./data/christmas".into(),
            ..Default::default()
        });
        embedded_db.setup().await.expect("db setup failed");
        embedded_db.start().await.expect("db start failed");
        embedded_db
            .create_database("christmas")
            .await
            .expect("db create failed");
        url = embedded_db.settings().url("christmas");
        db = Some(embedded_db);
    }

    tracingx::info!("using database: {url}");
    let db_conn = sqlx::PgPool::connect(&url).await.expect("db connection failed");

    database::initialize(&db_conn).await.expect("db initialize failed");

    let ip = dioxus::cli_config::server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = dioxus::cli_config::server_port().unwrap_or(8080);
    let address = SocketAddr::new(ip, port);
    let listener = TcpListener::bind(address).await.unwrap();

    // Create the serve configuration
    // The database pool can be added as context if needed
    let serve_config = dioxus::server::ServeConfig::new().unwrap();

    // Create the axum router with dioxus application
    // The serve_dioxus_application method adds routes to server side render the application,
    // serve static assets, and register server functions
    let router = axum::Router::new()
        .serve_dioxus_application(serve_config, app)
        .with_state(db_conn)
        .into_make_service();

    tracingx::info!(port = port, address = ip.to_string(), "Server started successfully");
    axum::serve(listener, router).await.unwrap();

    if let Some(db) = db {
        db.stop().await.expect("db stop failed");
    }
}
