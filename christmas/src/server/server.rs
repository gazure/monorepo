use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use dioxus::{
    fullstack::ServeConfigBuilder,
    prelude::{DioxusRouterExt, Element},
};
use start::AppMeta;

pub async fn launch(app: fn() -> Element) {
    tracingx::init_logging();
    let app_meta = AppMeta::from_env();
    let root_span = tracing::info_span!("app", app = %app_meta.app, region = %app_meta.region, host = %app_meta.host);
    let _span = root_span.enter();

    let ip =
        dioxus::cli_config::server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = dioxus::cli_config::server_port().unwrap_or(8080);
    let address = SocketAddr::new(ip, port);
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    let router = axum::Router::new()
        // serve_dioxus_application adds routes to server side render the application, serve static assets, and register server functions
        .serve_dioxus_application(ServeConfigBuilder::default().build().unwrap(), app)
        .into_make_service();

    tracing::info!(
        port = port,
        address = ip.to_string(),
        "Server started successfully"
    );
    axum::serve(listener, router).await.unwrap();
}
