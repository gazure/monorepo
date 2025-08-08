use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize structured JSON logging for the Dioxus server
pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    #[cfg(target_arch = "wasm32")]
    {
        // For WebAssembly, use a simpler format without timestamps
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .without_time() // Disable timestamps in WebAssembly
            .with_target(true)
            .with_level(true);

        tracing_subscriber::registry().with(env_filter).with(fmt_layer).init();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // For non-WebAssembly platforms, use full JSON logging with timestamps
        let json_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(false)
            .with_span_list(true)
            .with_target(true)
            .with_level(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true);

        tracing_subscriber::registry().with(env_filter).with(json_layer).init();
    }
}
