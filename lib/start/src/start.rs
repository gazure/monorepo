use crate::metadata::AppMeta;

pub fn start(app: fn() -> dioxus::prelude::Element) {
    tracingx::init_logging();
    let app_meta = AppMeta::from_env();
    let root_span = tracing::info_span!("app", app = %app_meta.app, region = %app_meta.region, host = %app_meta.host);
    root_span.in_scope(|| {
        tracing::info!(
            port = 8080,
            address = "0.0.0.0",
            "Server started successfully"
        );
        dioxus::launch(app);
    });
}
