mod tracingx;

// Re-export all convenience functions from tracingx module
// Re-export common tracing types
pub use tracing::{
    Dispatch, Event, Instrument, Level, Metadata, Span, Subscriber, dispatcher, enabled, field, level_filters,
    subscriber,
};
// Re-export tracing macros
pub use tracing::{
    debug, debug_span, error, error_span, event, info, info_span, instrument, span, trace, trace_span, warn, warn_span,
};
// Re-export tracing_subscriber utilities that users might need
pub use tracing_subscriber::{
    EnvFilter, Registry, fmt,
    layer::{Layer, SubscriberExt},
    registry,
    util::SubscriberInitExt,
};
#[cfg(not(target_arch = "wasm32"))]
pub use tracingx::init_test;
pub use tracingx::{
    LogFormat, LoggingConfig, init_compact, init_compact_with_filter, init_dev, init_logging, init_pretty,
    init_pretty_with_filter, init_prod, init_with_env_filter, is_initialized,
};

// Prelude module for convenient imports
pub mod prelude {
    pub use super::{
        Event, Instrument, Level, Span, debug, debug_span, error, error_span, info, info_span, init_logging,
        instrument, span, trace, trace_span, warn, warn_span,
    };
}
