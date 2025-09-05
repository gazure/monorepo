//! Example demonstrating how to use tracingx with re-exported macros
//!
//! Run with:
//! ```
//! cargo run --example basic
//! ```
//!
//! Try different log levels:
//! ```
//! RUST_LOG=debug cargo run --example basic
//! RUST_LOG=trace cargo run --example basic
//! ```

// Import everything from tracingx instead of tracing directly
use tracingx::{debug, error, info, prelude::*, trace, warn};

#[derive(Debug)]
struct User {
    id: u64,
    name: String,
    email: String,
}

fn main() {
    // Initialize logging using tracingx's convenience function
    tracingx::init_logging();

    // Basic logging at different levels
    trace!("This is a trace message - most verbose");
    debug!("This is a debug message");
    info!("Application starting");
    warn!("This is a warning");
    error!("This is an error (but not a panic!)");

    // Structured logging with fields
    let user = User {
        id: 42,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    info!(
        user.id = user.id,
        user.name = %user.name,
        user.email = %user.email,
        "User logged in"
    );

    // Using spans for context
    let span = info_span!("process_request", request_id = 123);
    let _enter = span.enter();

    info!("Processing request");
    do_some_work();
    info!("Request completed");

    // Using the instrument attribute (if you have a function)
    instrumented_function();

    // Demonstrate nested spans
    demonstrate_nested_spans();
}

fn do_some_work() {
    debug!("Doing some work...");
    std::thread::sleep(std::time::Duration::from_millis(100));
    debug!("Work completed");
}

#[instrument]
fn instrumented_function() {
    info!("This function is automatically instrumented");
    debug!("Any logs in here will be associated with this function's span");
}

fn demonstrate_nested_spans() {
    let outer_span = info_span!("outer_operation", id = 1);
    let _outer = outer_span.enter();

    info!("Starting outer operation");

    {
        let inner_span = debug_span!("inner_operation", id = 2);
        let _inner = inner_span.enter();

        debug!("Performing inner operation");
        trace!("Very detailed trace information");
    }

    info!("Outer operation complete");
}
