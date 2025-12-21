use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize structured JSON logging with default settings
pub fn init_logging() {
    init_with_env_filter(None);
}

/// Initialize logging with a custom environment filter string
pub fn init_with_env_filter(filter: Option<&str>) {
    let env_filter = match filter {
        Some(f) => EnvFilter::new(f),
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    };

    #[cfg(target_arch = "wasm32")]
    {
        // For WebAssembly, use a simpler format without timestamps
        let fmt_layer = fmt::layer()
            .with_ansi(false)
            .without_time() // Disable timestamps in WebAssembly
            .with_target(true)
            .with_level(true);

        Registry::default().with(env_filter).with(fmt_layer).init();
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // For non-WebAssembly platforms, use full JSON logging with timestamps
        let json_layer = fmt::layer()
            .json()
            .with_current_span(false)
            .with_span_list(true)
            .with_target(true)
            .with_level(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true);

        Registry::default().with(env_filter).with(json_layer).init();
    }
}

/// Initialize human-readable logging (pretty format)
pub fn init_pretty() {
    init_pretty_with_filter(None);
}

/// Initialize human-readable logging with custom filter
pub fn init_pretty_with_filter(filter: Option<&str>) {
    let env_filter = match filter {
        Some(f) => EnvFilter::new(f),
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    };

    let fmt_layer = fmt::layer()
        .pretty()
        .with_target(true)
        .with_level(true)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true);

    Registry::default().with(env_filter).with(fmt_layer).init();
}

/// Initialize compact logging (single line per log)
pub fn init_compact() {
    init_compact_with_filter(None);
}

/// Initialize compact logging with custom filter
pub fn init_compact_with_filter(filter: Option<&str>) {
    let env_filter = match filter {
        Some(f) => EnvFilter::new(f),
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    };

    let fmt_layer = fmt::layer()
        .compact()
        .with_target(true)
        .with_level(true)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false);

    Registry::default().with(env_filter).with(fmt_layer).init();
}

/// Initialize test logging (useful for tests)
/// This captures logs but doesn't print them unless the test fails
#[cfg(not(target_arch = "wasm32"))]
pub fn init_test() {
    let _ = Registry::default()
        .with(fmt::layer().with_test_writer().with_target(true).with_level(true))
        .try_init();
}

/// Configuration builder for more complex setups
pub struct LoggingConfig {
    filter: Option<String>,
    format: LogFormat,
    show_target: bool,
    show_level: bool,
    show_file: bool,
    show_line_number: bool,
    show_thread_names: bool,
    ansi_colors: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
    Full,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            filter: None,
            format: LogFormat::Json,
            show_target: true,
            show_level: true,
            show_file: true,
            show_line_number: true,
            show_thread_names: false,
            ansi_colors: true,
        }
    }
}

impl LoggingConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_filter<S: Into<String>>(mut self, filter: S) -> Self {
        self.filter = Some(filter.into());
        self
    }

    pub fn with_format(mut self, format: LogFormat) -> Self {
        self.format = format;
        self
    }

    pub fn show_target(mut self, show: bool) -> Self {
        self.show_target = show;
        self
    }

    pub fn show_level(mut self, show: bool) -> Self {
        self.show_level = show;
        self
    }

    pub fn show_file(mut self, show: bool) -> Self {
        self.show_file = show;
        self
    }

    pub fn show_line_number(mut self, show: bool) -> Self {
        self.show_line_number = show;
        self
    }

    pub fn show_thread_names(mut self, show: bool) -> Self {
        self.show_thread_names = show;
        self
    }

    pub fn ansi_colors(mut self, enabled: bool) -> Self {
        self.ansi_colors = enabled;
        self
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn init(self) {
        let env_filter = match self.filter {
            Some(f) => EnvFilter::new(f),
            None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        };

        let base_fmt = fmt::layer()
            .with_target(self.show_target)
            .with_level(self.show_level)
            .with_file(self.show_file)
            .with_line_number(self.show_line_number)
            .with_thread_names(self.show_thread_names)
            .with_ansi(self.ansi_colors);

        match self.format {
            LogFormat::Json => {
                Registry::default()
                    .with(env_filter)
                    .with(base_fmt.json().with_current_span(false).with_span_list(true))
                    .init();
            }
            LogFormat::Pretty => {
                Registry::default().with(env_filter).with(base_fmt.pretty()).init();
            }
            LogFormat::Compact => {
                Registry::default().with(env_filter).with(base_fmt.compact()).init();
            }
            LogFormat::Full => {
                Registry::default().with(env_filter).with(base_fmt).init();
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn init_once(self) {
        if !crate::is_initialized() {
            self.init();
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn init(self) {
        let env_filter = match self.filter {
            Some(f) => EnvFilter::new(f),
            None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        };

        // For WebAssembly, always use a simpler format
        let fmt_layer = fmt::layer()
            .with_ansi(false)
            .without_time()
            .with_target(self.show_target)
            .with_level(self.show_level);

        Registry::default().with(env_filter).with(fmt_layer).init();
    }
}

/// Initialize logging for development with pretty printing
/// Uses RUST_LOG environment variable if set, otherwise defaults to info level
pub fn init_dev() {
    LoggingConfig::new()
        .with_format(LogFormat::Pretty)
        .show_file(true)
        .show_line_number(true)
        .init_once();
}

/// Initialize logging for production with JSON format and info level
pub fn init_prod() {
    LoggingConfig::new()
        .with_filter("info")
        .with_format(LogFormat::Json)
        .show_file(false)
        .show_line_number(false)
        .show_thread_names(true)
        .init_once();
}

/// Check if a tracing subscriber has already been set
/// Useful to avoid double-initialization in tests
pub fn is_initialized() -> bool {
    tracing::dispatcher::has_been_set()
}
