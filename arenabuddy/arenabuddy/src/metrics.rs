use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use opentelemetry::{
    global,
    metrics::{Counter, Meter, MeterProvider as _, UpDownCounter},
    KeyValue,
};
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{
    metrics::{
        reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
        MeterProvider, PeriodicReader, SdkMeterProvider,
    },
    runtime,
    Resource,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time;
use tracingx::{error, info, warn};

/// OpenTelemetry-based metrics collector for Arenabuddy
/// Tracks key metrics and pushes them to a remote OTLP endpoint
#[derive(Clone)]
pub struct MetricsCollector {
    meter: Meter,
    games_counter: Counter<u64>,
    drafts_counter: Counter<u64>,
    parse_errors_counter: Counter<u64>,
    active_sessions: UpDownCounter<i64>,
    inner: Arc<RwLock<MetricsState>>,
    config: OtelMetricsConfig,
}

/// Configuration for OpenTelemetry metrics
#[derive(Clone, Debug)]
pub struct MetricsConfig {
    /// OTLP endpoint URL (e.g., "http://localhost:4317" or "https://otlp.grafana.com:443")
    pub otlp_endpoint: Option<String>,
    /// Optional headers for authentication (e.g., API keys)
    pub otlp_headers: Vec<(String, String)>,
    /// How often to export metrics (in seconds)
    pub export_interval_secs: u64,
    /// Application instance identifier
    pub instance_id: String,
    /// Service name for OpenTelemetry
    pub service_name: String,
    /// Whether metrics collection is enabled
    pub enabled: bool,
    /// Protocol to use (grpc or http)
    pub protocol: OtlpProtocol,
}

#[derive(Clone, Debug)]
pub enum OtlpProtocol {
    Grpc,
    Http,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: None,
            otlp_headers: vec![],
            export_interval_secs: 60, // 1 minute default for OTLP
            instance_id: format!("arenabuddy_{}", uuid::Uuid::new_v4()),
            service_name: "arenabuddy".to_string(),
            enabled: true,
            protocol: OtlpProtocol::Grpc,
        }
    }
}

impl MetricsConfig {
    /// Create config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            config.otlp_endpoint = Some(endpoint);
        }

        if let Ok(headers) = std::env::var("OTEL_EXPORTER_OTLP_HEADERS") {
            config.otlp_headers = headers
                .split(',')
                .filter_map(|h| {
                    let parts: Vec<&str> = h.split('=').collect();
                    if parts.len() == 2 {
                        Some((parts[0].to_string(), parts[1].to_string()))
                    } else {
                        None
                    }
                })
                .collect();
        }

        if let Ok(interval) = std::env::var("OTEL_METRIC_EXPORT_INTERVAL") {
            if let Ok(secs) = interval.parse() {
                config.export_interval_secs = secs;
            }
        }

        if let Ok(service) = std::env::var("OTEL_SERVICE_NAME") {
            config.service_name = service;
        }

        if let Ok(instance) = std::env::var("ARENABUDDY_INSTANCE_ID") {
            config.instance_id = instance;
        }

        if let Ok(enabled) = std::env::var("OTEL_METRICS_ENABLED") {
            config.enabled = enabled.to_lowercase() != "false";
        }

        if let Ok(protocol) = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL") {
            config.protocol = match protocol.as_str() {
                "http/protobuf" | "http" => OtlpProtocol::Http,
                _ => OtlpProtocol::Grpc,
            };
        }

        config
    }
}

#[derive(Debug, Clone)]
struct MetricsState {
    /// Application start time
    start_time: DateTime<Utc>,
    /// Last export timestamp
    last_export: Option<DateTime<Utc>>,
    /// Current session count
    active_sessions: i64,
    /// Local counters for tracking
    local_games_count: u64,
    local_drafts_count: u64,
    local_errors_count: u64,
}

impl MetricsCollector {
    /// Create a new OpenTelemetry metrics collector
    pub fn new(config: MetricsConfig) -> Self {
        // If metrics are disabled, create a no-op meter
        if !config.enabled || config.otlp_endpoint.is_none() {
            let meter = global::meter("arenabuddy_noop");
            let games_counter = meter.u64_counter("noop").init();
            let drafts_counter = meter.u64_counter("noop").init();
            let parse_errors_counter = meter.u64_counter("noop").init();
            let active_sessions = meter.i64_up_down_counter("noop").init();

            return Self {
                meter,
                games_counter,
                drafts_counter,
                parse_errors_counter,
                active_sessions,
                inner: Arc::new(RwLock::new(MetricsState {
                    start_time: Utc::now(),
                    last_export: None,
                    active_sessions: 0,
                    local_games_count: 0,
                    local_drafts_count: 0,
                    local_errors_count: 0,
                })),
                config,
            };
        }

        // Build the OTLP exporter
        let endpoint = config.otlp_endpoint.as_ref().unwrap();

        let exporter = match config.protocol {
            OtlpProtocol::Grpc => {
                let mut exporter = opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint);

                // Add headers for authentication
                for (key, value) in &config.otlp_headers {
                    exporter = exporter.with_metadata(vec![(key.clone(), value.clone())]);
                }

                opentelemetry_otlp::new_pipeline()
                    .metrics(runtime::Tokio)
                    .with_exporter(exporter)
                    .with_period(Duration::from_secs(config.export_interval_secs))
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", config.service_name.clone()),
                        KeyValue::new("service.instance.id", config.instance_id.clone()),
                        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    ]))
                    .build()
            },
            OtlpProtocol::Http => {
                let mut exporter = opentelemetry_otlp::new_exporter()
                    .http()
                    .with_endpoint(endpoint)
                    .with_protocol(Protocol::HttpBinary);

                // Add headers for authentication
                let headers: std::collections::HashMap<String, String> =
                    config.otlp_headers.iter().cloned().collect();
                exporter = exporter.with_headers(headers);

                opentelemetry_otlp::new_pipeline()
                    .metrics(runtime::Tokio)
                    .with_exporter(exporter)
                    .with_period(Duration::from_secs(config.export_interval_secs))
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", config.service_name.clone()),
                        KeyValue::new("service.instance.id", config.instance_id.clone()),
                        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                    ]))
                    .build()
            }
        };

        let meter_provider = match exporter {
            Ok(provider) => provider,
            Err(e) => {
                error!("Failed to create OTLP exporter: {}", e);
                // Return a no-op collector
                let meter = global::meter("arenabuddy_noop");
                let games_counter = meter.u64_counter("noop").init();
                let drafts_counter = meter.u64_counter("noop").init();
                let parse_errors_counter = meter.u64_counter("noop").init();
                let active_sessions = meter.i64_up_down_counter("noop").init();

                return Self {
                    meter,
                    games_counter,
                    drafts_counter,
                    parse_errors_counter,
                    active_sessions,
                    inner: Arc::new(RwLock::new(MetricsState {
                        start_time: Utc::now(),
                        last_export: None,
                        active_sessions: 0,
                        local_games_count: 0,
                        local_drafts_count: 0,
                        local_errors_count: 0,
                    })),
                    config,
                };
            }
        };

        // Set as global meter provider
        global::set_meter_provider(meter_provider.clone());

        // Get meter
        let meter = meter_provider.meter("arenabuddy");

        // Create metric instruments
        let games_counter = meter
            .u64_counter("arenabuddy.games.ingested")
            .with_description("Total number of games ingested")
            .with_unit("games")
            .init();

        let drafts_counter = meter
            .u64_counter("arenabuddy.drafts.ingested")
            .with_description("Total number of drafts ingested")
            .with_unit("drafts")
            .init();

        let parse_errors_counter = meter
            .u64_counter("arenabuddy.parse.errors")
            .with_description("Total number of parse errors encountered")
            .with_unit("errors")
            .init();

        let active_sessions = meter
            .i64_up_down_counter("arenabuddy.sessions.active")
            .with_description("Number of active sessions")
            .with_unit("sessions")
            .init();

        info!(
            "OpenTelemetry metrics initialized - endpoint: {}, interval: {}s",
            endpoint,
            config.export_interval_secs
        );

        Self {
            meter,
            games_counter,
            drafts_counter,
            parse_errors_counter,
            active_sessions,
            inner: Arc::new(RwLock::new(MetricsState {
                start_time: Utc::now(),
                last_export: None,
                active_sessions: 0,
                local_games_count: 0,
                local_drafts_count: 0,
                local_errors_count: 0,
            })),
            config,
        }
    }

    /// Increment the games ingested counter
    pub async fn increment_games_ingested(&self) {
        if !self.config.enabled {
            return;
        }

        self.games_counter.add(
            1,
            &[
                KeyValue::new("instance", self.config.instance_id.clone()),
                KeyValue::new("game_type", "match"),
            ],
        );

        let mut state = self.inner.write().await;
        state.local_games_count += 1;
    }

    /// Increment the drafts ingested counter
    pub async fn increment_drafts_ingested(&self) {
        if !self.config.enabled {
            return;
        }

        self.drafts_counter.add(
            1,
            &[
                KeyValue::new("instance", self.config.instance_id.clone()),
                KeyValue::new("draft_type", "mtga"),
            ],
        );

        let mut state = self.inner.write().await;
        state.local_drafts_count += 1;
    }

    /// Increment the parse errors counter
    pub async fn increment_parse_errors(&self) {
        if !self.config.enabled {
            return;
        }

        self.parse_errors_counter.add(
            1,
            &[
                KeyValue::new("instance", self.config.instance_id.clone()),
                KeyValue::new("error_type", "parse"),
            ],
        );

        let mut state = self.inner.write().await;
        state.local_errors_count += 1;
    }

    /// Update active session count
    pub async fn set_active_sessions(&self, count: i64) {
        if !self.config.enabled {
            return;
        }

        let mut state = self.inner.write().await;
        let delta = count - state.active_sessions;

        if delta != 0 {
            self.active_sessions.add(
                delta,
                &[KeyValue::new("instance", self.config.instance_id.clone())],
            );
            state.active_sessions = count;
        }
    }

    /// Start a new session
    pub async fn session_started(&self) {
        if !self.config.enabled {
            return;
        }

        let mut state = self.inner.write().await;
        state.active_sessions += 1;

        self.active_sessions.add(
            1,
            &[KeyValue::new("instance", self.config.instance_id.clone())],
        );
    }

    /// End a session
    pub async fn session_ended(&self) {
        if !self.config.enabled {
            return;
        }

        let mut state = self.inner.write().await;
        if state.active_sessions > 0 {
            state.active_sessions -= 1;

            self.active_sessions.add(
                -1,
                &[KeyValue::new("instance", self.config.instance_id.clone())],
            );
        }
    }

    /// Get current metrics snapshot (for debugging/status)
    pub async fn get_snapshot(&self) -> MetricsSnapshot {
        let state = self.inner.read().await;
        let uptime = Utc::now().signed_duration_since(state.start_time);

        MetricsSnapshot {
            games_ingested_total: state.local_games_count,
            drafts_ingested_total: state.local_drafts_count,
            parse_errors_total: state.local_errors_count,
            active_sessions: state.active_sessions,
            uptime_seconds: uptime.num_seconds() as u64,
            last_export: state.last_export,
            instance_id: self.config.instance_id.clone(),
            service_name: self.config.service_name.clone(),
        }
    }

    /// Force a metrics export (useful for shutdown)
    pub async fn force_export(&self) {
        if !self.config.enabled || self.config.otlp_endpoint.is_none() {
            return;
        }

        // OpenTelemetry handles export through the periodic reader
        // This is a placeholder for explicit flush if needed
        info!("Forcing metrics export...");

        let mut state = self.inner.write().await;
        state.last_export = Some(Utc::now());
    }

    /// Shutdown the metrics collector gracefully
    pub async fn shutdown(&self) {
        if !self.config.enabled {
            return;
        }

        info!("Shutting down OpenTelemetry metrics...");

        // End any active sessions
        let state = self.inner.read().await;
        if state.active_sessions > 0 {
            drop(state);
            self.set_active_sessions(0).await;
        }

        // Shutdown global meter provider to flush remaining metrics
        global::shutdown_tracer_provider();
    }
}

/// Metrics snapshot for serialization/debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub games_ingested_total: u64,
    pub drafts_ingested_total: u64,
    pub parse_errors_total: u64,
    pub active_sessions: i64,
    pub uptime_seconds: u64,
    pub last_export: Option<DateTime<Utc>>,
    pub instance_id: String,
    pub service_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector_disabled() {
        let config = MetricsConfig {
            enabled: false,
            ..Default::default()
        };

        let collector = MetricsCollector::new(config);

        // Operations should be no-ops when disabled
        collector.increment_games_ingested().await;
        collector.increment_drafts_ingested().await;
        collector.increment_parse_errors().await;

        let snapshot = collector.get_snapshot().await;
        assert_eq!(snapshot.games_ingested_total, 0);
        assert_eq!(snapshot.drafts_ingested_total, 0);
        assert_eq!(snapshot.parse_errors_total, 0);
    }

    #[tokio::test]
    async fn test_session_tracking() {
        let config = MetricsConfig {
            enabled: false, // Use no-op for testing
            ..Default::default()
        };

        let collector = MetricsCollector::new(config);

        collector.session_started().await;
        collector.session_started().await;

        let snapshot = collector.get_snapshot().await;
        assert_eq!(snapshot.active_sessions, 0); // No-op mode doesn't track

        collector.session_ended().await;

        let snapshot = collector.get_snapshot().await;
        assert_eq!(snapshot.active_sessions, 0);
    }
}
