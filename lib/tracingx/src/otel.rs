use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Builds an OTLP trace exporter and returns the provider (needed for shutdown).
///
/// The endpoint defaults to `http://localhost:4317` and can be overridden via
/// the standard `OTEL_EXPORTER_OTLP_ENDPOINT` environment variable.
fn build_provider(service_name: &str) -> SdkTracerProvider {
    use opentelemetry::KeyValue;
    use opentelemetry_sdk::Resource;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("failed to create OTLP span exporter");

    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_attributes([KeyValue::new("service.name", service_name.to_owned())])
                .build(),
        )
        .build()
}

fn build_env_filter(filter: Option<&str>) -> EnvFilter {
    match filter {
        Some(f) => EnvFilter::new(f),
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    }
}

/// A handle returned by the otel init functions. Call [`OtelGuard::shutdown`]
/// before the process exits to flush any pending spans.
pub struct OtelGuard {
    provider: SdkTracerProvider,
}

impl OtelGuard {
    /// Flush pending spans and shut down the exporter.
    pub fn shutdown(self) {
        if let Err(e) = self.provider.shutdown() {
            eprintln!("OpenTelemetry shutdown error: {e}");
        }
    }
}

/// Initialise compact console logging **and** an OpenTelemetry trace layer.
///
/// Returns an [`OtelGuard`] whose [`shutdown`](OtelGuard::shutdown) method
/// must be called before the process exits.
pub fn init_compact_with_otel(service_name: &str) -> OtelGuard {
    let provider = build_provider(service_name);
    let tracer = provider.tracer(service_name.to_owned());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default()
        .with(build_env_filter(None))
        .with(
            fmt::layer()
                .compact()
                .with_target(true)
                .with_level(true)
                .with_file(false)
                .with_line_number(false)
                .with_thread_names(false),
        )
        .with(otel_layer)
        .init();

    OtelGuard { provider }
}

/// Initialise pretty console logging **and** an OpenTelemetry trace layer.
pub fn init_pretty_with_otel(service_name: &str) -> OtelGuard {
    let provider = build_provider(service_name);
    let tracer = provider.tracer(service_name.to_owned());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default()
        .with(build_env_filter(None))
        .with(
            fmt::layer()
                .pretty()
                .with_target(true)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_names(false),
        )
        .with(otel_layer)
        .init();

    OtelGuard { provider }
}

/// Initialise JSON console logging **and** an OpenTelemetry trace layer.
pub fn init_json_with_otel(service_name: &str) -> OtelGuard {
    let provider = build_provider(service_name);
    let tracer = provider.tracer(service_name.to_owned());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default()
        .with(build_env_filter(None))
        .with(
            fmt::layer()
                .json()
                .with_current_span(false)
                .with_span_list(true)
                .with_target(true)
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_names(true),
        )
        .with(otel_layer)
        .init();

    OtelGuard { provider }
}
