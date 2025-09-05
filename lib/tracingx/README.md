### Disclaimer

Claude Opus 4.1 wrote all this. Hopefully most of it is correct.

# tracingx

A convenience wrapper around the `tracing` and `tracing-subscriber` crates that provides:

- Re-exported tracing macros for simpler dependency management
- Multiple pre-configured initialization patterns
- Platform-aware logging (different behavior for WASM vs native)
- Flexible configuration API for custom setups

## Why tracingx?

### Single Dependency Point

Instead of adding both `tracing` and `tracing-subscriber` to your `Cargo.toml`, you only need `tracingx`:

```toml
[dependencies]
tracingx = { path = "../../lib/tracingx" }
# No need for direct tracing dependency!
```

### Consistent Versions

All your projects use the same version of tracing that tracingx uses, avoiding version conflicts.

### Re-exported Macros

All tracing macros are re-exported, so you can import everything from one place:

```rust
use tracingx::{info, debug, warn, error, trace, instrument};
// Or use the prelude:
use tracingx::prelude::*;
```

## Quick Start

### Basic Usage

```rust
use tracingx::{info, debug, warn, error};

fn main() {
    // Initialize with default settings (JSON format, info level)
    tracingx::init_logging();

    info!("Application started");
    debug!("Debug information");
    warn!("Warning message");
    error!("Error occurred");
}
```

### Development Mode

```rust
fn main() {
    // Pretty printing with debug level - great for development
    tracingx::init_dev();

    info!("This will be beautifully formatted");
}
```

### Production Mode

```rust
fn main() {
    // JSON format with info level - perfect for production logs
    tracingx::init_prod();

    info!("Structured JSON logs for your log aggregation system");
}
```

## Initialization Options

### Pre-configured Functions

```rust
// JSON format (default)
tracingx::init_logging();

// Pretty format for human reading
tracingx::init_pretty();

// Compact single-line format
tracingx::init_compact();

// Development preset (pretty + debug level)
tracingx::init_dev();

// Production preset (JSON + info level)
tracingx::init_prod();

// With custom filter
tracingx::init_with_env_filter(Some("debug,hyper=warn"));

// Initialize only if not already initialized
tracingx::init_once();
```

### Custom Configuration

```rust
use tracingx::{LoggingConfig, LogFormat};

fn main() {
    LoggingConfig::new()
        .with_filter("debug")
        .with_format(LogFormat::Pretty)
        .show_file(true)
        .show_line_number(true)
        .show_thread_names(true)
        .ansi_colors(true)
        .init();
}
```

## Structured Logging

```rust
use tracingx::{info, debug};

#[derive(Debug)]
struct User {
    id: u64,
    name: String,
}

fn process_user(user: &User) {
    // Log with structured fields
    info!(
        user.id = user.id,
        user.name = %user.name,
        "Processing user"
    );

    // Use debug formatting
    debug!(?user, "User details");
}
```

## Spans and Context

```rust
use tracingx::{info_span, debug_span, instrument};

fn main() {
    tracingx::init_pretty();

    let span = info_span!("main_operation", request_id = 123);
    let _enter = span.enter();

    info!("This log includes the span context");
    nested_operation();
}

#[instrument]
fn nested_operation() {
    debug!("This function is automatically instrumented");
}
```

## Platform Support

### Native (Non-WASM)

- Full featured logging with timestamps
- JSON, Pretty, Compact, and Full formats
- Thread names, file locations, line numbers
- ANSI color support

### WebAssembly

- Simplified format without timestamps
- Automatic ANSI color disabling
- Optimized for browser console output

## Environment Variables

Control log levels using the `RUST_LOG` environment variable:

```bash
# Set global level
RUST_LOG=debug cargo run

# Set per-module levels
RUST_LOG=debug,hyper=warn,tower=error cargo run

# Filter by target
RUST_LOG=my_app=debug cargo run
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use tracingx::{info, debug};

    #[test]
    fn test_with_logging() {
        // Initialize test logging (captured unless test fails)
        tracingx::init_test();

        info!("Test started");
        debug!("Debug info in test");

        // Your test logic here
        assert_eq!(2 + 2, 4);
    }
}
```

## Examples

Run the examples to see different configurations:

```bash
# Basic example with re-exported macros
cargo run --example basic

# Advanced configurations
cargo run --example advanced -- json
cargo run --example advanced -- pretty
cargo run --example advanced -- dev
cargo run --example advanced -- prod
```

## Features

Default features: `["fmt", "env-filter"]`

Available features:

- `fmt` - Formatting layer support
- `json` - JSON formatting
- `env-filter` - Environment variable filtering
- `registry` - Registry layer support
- `ansi` - ANSI color support
- `time` - Timestamp support
- `local-time` - Local timestamp support

## Best Practices

1. **Initialize once at program start** - Call initialization in `main()` before any logging
2. **Use structured logging** - Add context with fields rather than formatting in strings
3. **Use spans for context** - Wrap operations in spans to maintain context
4. **Configure per environment** - Use `init_dev()` for development, `init_prod()` for production
5. **Use the prelude** - Import `tracingx::prelude::*` for convenient access to common items

## License

Same as parent workspace
