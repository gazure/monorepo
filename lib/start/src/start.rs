use crate::metadata::AppMeta;

pub fn start(app: fn() -> dioxus::prelude::Element) {
    // Set up panic handler for WebAssembly
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
    }

    tracingx::init_logging();
    let app_meta = AppMeta::from_env();
    let root_span = tracingx::info_span!("app", app = %app_meta.app, region = %app_meta.region, host = %app_meta.host);
    root_span.in_scope(|| {
        tracingx::info!(port = 8080, address = "0.0.0.0", "Server started successfully");
        dioxus::launch(app);
    });
}
