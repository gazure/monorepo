//! Advanced example demonstrating different initialization options in tracingx
//!
//! Run different examples:
//! ```
//! cargo run --example advanced -- json
//! cargo run --example advanced -- pretty
//! cargo run --example advanced -- compact
//! cargo run --example advanced -- custom
//! cargo run --example advanced -- dev
//! cargo run --example advanced -- prod
//! ```

use std::{env, error::Error};

use tracingx::{LogFormat, LoggingConfig, debug, error, info, trace, warn};

#[derive(Debug)]
struct Order {
    id: u64,
    customer_id: u64,
    total: f64,
    items: Vec<String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("json");

    match mode {
        "json" => {
            // Default JSON logging
            tracingx::init_logging();
            info!("Initialized with JSON format");
        }
        "pretty" => {
            // Pretty printing for development
            tracingx::init_pretty();
            info!("Initialized with pretty format");
        }
        "compact" => {
            // Compact single-line format
            tracingx::init_compact();
            info!("Initialized with compact format");
        }
        "custom" => {
            // Custom configuration
            LoggingConfig::new()
                .with_filter("trace")
                .with_format(LogFormat::Pretty)
                .show_target(true)
                .show_level(true)
                .show_file(true)
                .show_line_number(true)
                .show_thread_names(true)
                .ansi_colors(true)
                .init();
            info!("Initialized with custom configuration");
        }
        "dev" => {
            // Development preset
            tracingx::init_dev();
            info!("Initialized with dev preset");
        }
        "prod" => {
            // Production preset
            tracingx::init_prod();
            info!("Initialized with production preset");
        }
        _ => {
            // With custom filter
            tracingx::init_with_env_filter(Some("debug"));
            info!("Initialized with custom filter");
        }
    }

    demonstrate_logging();
    demonstrate_structured_logging();
    demonstrate_error_handling();
    demonstrate_async_context().unwrap();
}

fn demonstrate_logging() {
    let span = tracingx::info_span!("demonstrate_logging");
    let _enter = span.enter();

    trace!("Trace level - most verbose");
    debug!("Debug level - debugging info");
    info!("Info level - general information");
    warn!("Warn level - warning messages");
    error!("Error level - error messages");
}

fn demonstrate_structured_logging() {
    let order = Order {
        id: 12345,
        customer_id: 67890,
        total: 299.99,
        items: vec!["Widget A".to_string(), "Widget B".to_string(), "Widget C".to_string()],
    };

    // Structured logging with multiple fields
    info!(
        order.id = order.id,
        order.customer_id = order.customer_id,
        order.total = order.total,
        order.item_count = order.items.len(),
        "Order received"
    );

    // Using debug representation
    debug!(?order, "Order details");

    // Nested span with inherited context
    let order_span = tracingx::info_span!(
        "process_order",
        order.id = order.id,
        order.customer_id = order.customer_id
    );
    let _order_guard = order_span.enter();

    info!("Validating order");
    validate_order(&order);
    info!("Processing payment");
    process_payment(&order);
    info!("Order completed");
}

fn validate_order(order: &Order) {
    debug!(items = ?order.items, "Checking inventory");

    for (i, item) in order.items.iter().enumerate() {
        trace!(item_index = i, item_name = %item, "Checking item availability");
    }
}

fn process_payment(order: &Order) {
    let payment_span = tracingx::debug_span!("payment", amount = order.total);
    let _payment_guard = payment_span.enter();

    debug!("Initiating payment");
    std::thread::sleep(std::time::Duration::from_millis(100));
    info!("Payment processed successfully");
}

fn demonstrate_error_handling() {
    let result = risky_operation();

    match result {
        Ok(value) => {
            info!(value, "Operation succeeded");
        }
        Err(err) => {
            error!(%err, "Operation failed");

            // Log with error chain if available
            let mut source = err.source();
            let mut level = 0;
            while let Some(err) = source {
                error!(cause_level = level, cause = %err, "Error cause");
                source = err.source();
                level += 1;
            }
        }
    }
}

fn risky_operation() -> Result<i32, RiskyError> {
    Err(RiskyError::new("Something went wrong"))
}

#[derive(Debug)]
struct RiskyError {
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl RiskyError {
    fn new(msg: &str) -> Self {
        Self {
            message: msg.to_string(),
            source: Some(Box::new(std::io::Error::other("Underlying IO error"))),
        }
    }
}

impl std::fmt::Display for RiskyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Risky operation error: {}", self.message)
    }
}

impl std::error::Error for RiskyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| &**e as &(dyn std::error::Error + 'static))
    }
}

fn demonstrate_async_context() -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    };

    // Simulate async context with manual future
    struct AsyncOperation {
        completed: bool,
    }

    impl Future for AsyncOperation {
        type Output = String;

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            if !self.completed {
                debug!("Async operation in progress");
                self.completed = true;
                Poll::Pending
            } else {
                info!("Async operation completed");
                Poll::Ready("Success".to_string())
            }
        }
    }

    let operation = AsyncOperation { completed: false };

    // Use Instrument to attach span to async operation
    let _instrumented =
        tracingx::Instrument::instrument(operation, tracingx::info_span!("async_operation", task_id = 42));

    info!("Async context demonstrated (without actual async runtime)");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_initialization() {
        // Use init_once to avoid double initialization in tests
        let initialized = tracingx::init_once();

        if initialized {
            info!("Test logging initialized");
        } else {
            info!("Test logging was already initialized");
        }

        debug!("Running test");
        assert!(tracingx::is_initialized());
    }
}
