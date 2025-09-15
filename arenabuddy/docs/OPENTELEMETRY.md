# OpenTelemetry Metrics for Arenabuddy

This document describes how to set up OpenTelemetry (OTLP) metrics for Arenabuddy as an alternative to Prometheus metrics. OpenTelemetry provides a modern, push-based approach that's ideal for desktop applications.

## Why OpenTelemetry?

- **Push-based**: Perfect for desktop apps - no need to expose ports
- **Industry Standard**: Growing ecosystem with wide support
- **Grafana Compatible**: Native integration with Grafana Cloud and Grafana Stack
- **Unified Observability**: Metrics, traces, and logs in one protocol
- **Flexible Backends**: Send to multiple destinations simultaneously

## Configuration

### Environment Variables

Configure OpenTelemetry metrics using these environment variables:

```bash
# OTLP Endpoint (required for metrics to be sent)
OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp-gateway.grafana.net:443

# Authentication headers (for Grafana Cloud, etc.)
OTEL_EXPORTER_OTLP_HEADERS=Authorization=Basic <base64_encoded_credentials>

# Service configuration
OTEL_SERVICE_NAME=arenabuddy
ARENABUDDY_INSTANCE_ID=my-desktop-01

# Export interval (seconds, default: 60)
OTEL_METRIC_EXPORT_INTERVAL=60

# Protocol (grpc or http/protobuf, default: grpc)
OTEL_EXPORTER_OTLP_PROTOCOL=grpc

# Enable/disable metrics (default: true)
OTEL_METRICS_ENABLED=true
```

## Backend Options

### 1. Grafana Cloud (Recommended)

Grafana Cloud provides a managed OTLP endpoint with built-in visualization:

```bash
# Grafana Cloud configuration
OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp-gateway-prod-us-central-0.grafana.net:443
OTEL_EXPORTER_OTLP_HEADERS=Authorization=Basic <your_grafana_cloud_token>
OTEL_SERVICE_NAME=arenabuddy
```

To get your credentials:
1. Sign up for [Grafana Cloud](https://grafana.com/products/cloud/)
2. Navigate to your stack → Configuration → Data Sources
3. Find "OpenTelemetry" and copy the endpoint and authentication details

### 2. Self-Hosted Grafana with Tempo/Mimir

Run your own OTLP collector with Docker Compose:

```yaml
version: '3.8'

services:
  # OpenTelemetry Collector
  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    ports:
      - "4317:4317"  # OTLP gRPC
      - "4318:4318"  # OTLP HTTP
    volumes:
      - ./otel-collector-config.yaml:/etc/otel-collector-config.yaml
    command: ["--config=/etc/otel-collector-config.yaml"]

  # Grafana for visualization
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-storage:/var/lib/grafana

  # Mimir for metrics storage
  mimir:
    image: grafana/mimir:latest
    ports:
      - "9009:9009"
    volumes:
      - ./mimir-config.yaml:/etc/mimir/config.yaml
      - mimir-data:/data
    command: ["-config.file=/etc/mimir/config.yaml"]

volumes:
  grafana-storage:
  mimir-data:
```

OpenTelemetry Collector configuration (`otel-collector-config.yaml`):

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 1s

exporters:
  prometheusremotewrite:
    endpoint: http://mimir:9009/api/v1/push
    
  logging:
    loglevel: debug

service:
  pipelines:
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [prometheusremotewrite, logging]
```

Then configure Arenabuddy:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=arenabuddy
OTEL_METRICS_ENABLED=true
```

### 3. InfluxDB with Telegraf

InfluxDB is excellent for time-series data from desktop apps:

```yaml
version: '3.8'

services:
  influxdb:
    image: influxdb:2.7
    ports:
      - "8086:8086"
    environment:
      - DOCKER_INFLUXDB_INIT_MODE=setup
      - DOCKER_INFLUXDB_INIT_USERNAME=admin
      - DOCKER_INFLUXDB_INIT_PASSWORD=password123
      - DOCKER_INFLUXDB_INIT_ORG=arenabuddy
      - DOCKER_INFLUXDB_INIT_BUCKET=metrics
    volumes:
      - influxdb-data:/var/lib/influxdb2

  telegraf:
    image: telegraf:latest
    ports:
      - "4317:4317"  # OTLP gRPC
    volumes:
      - ./telegraf.conf:/etc/telegraf/telegraf.conf
    depends_on:
      - influxdb

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    depends_on:
      - influxdb

volumes:
  influxdb-data:
```

Telegraf configuration (`telegraf.conf`):

```toml
[[inputs.opentelemetry]]
  service_address = "0.0.0.0:4317"

[[outputs.influxdb_v2]]
  urls = ["http://influxdb:8086"]
  token = "your-influxdb-token"
  organization = "arenabuddy"
  bucket = "metrics"
```

### 4. VictoriaMetrics

VictoriaMetrics offers excellent compression and Prometheus compatibility:

```bash
# Run VictoriaMetrics with OTLP support
docker run -d \
  -p 8428:8428 \
  -p 4317:4317 \
  victoriametrics/victoria-metrics \
  -httpListenAddr=:8428 \
  -opentelemetry.grpc.listenAddr=:4317
```

Configure Arenabuddy:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=arenabuddy
```

## Metrics Collected

The OpenTelemetry implementation tracks these metrics:

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `arenabuddy.games.ingested` | Counter | Total games ingested | `instance`, `game_type` |
| `arenabuddy.drafts.ingested` | Counter | Total drafts ingested | `instance`, `draft_type` |
| `arenabuddy.parse.errors` | Counter | Parse errors encountered | `instance`, `error_type` |
| `arenabuddy.sessions.active` | UpDownCounter | Active sessions | `instance` |

## Grafana Dashboard

Import this dashboard JSON for instant visualization:

```json
{
  "dashboard": {
    "title": "Arenabuddy Analytics",
    "panels": [
      {
        "title": "Games Ingested",
        "targets": [
          {
            "expr": "sum(rate(arenabuddy_games_ingested[5m])) by (instance)"
          }
        ]
      },
      {
        "title": "Drafts Ingested", 
        "targets": [
          {
            "expr": "sum(rate(arenabuddy_drafts_ingested[5m])) by (instance)"
          }
        ]
      },
      {
        "title": "Active Sessions",
        "targets": [
          {
            "expr": "sum(arenabuddy_sessions_active) by (instance)"
          }
        ]
      },
      {
        "title": "Parse Errors",
        "targets": [
          {
            "expr": "sum(rate(arenabuddy_parse_errors[5m]))"
          }
        ]
      }
    ]
  }
}
```

## Migration from Prometheus

To migrate from the existing Prometheus push gateway setup:

1. **Update Dependencies**: Add OpenTelemetry crates to `Cargo.toml`
2. **Replace MetricsCollector**: Use `OtelMetricsCollector` instead
3. **Update Configuration**: Switch from `ARENABUDDY_METRICS_URL` to `OTEL_EXPORTER_OTLP_ENDPOINT`
4. **Test Locally**: Verify metrics are being sent before deploying

## Comparison with Other Solutions

| Feature | OpenTelemetry | Prometheus | PostHog | InfluxDB |
|---------|--------------|------------|---------|----------|
| Push-based | ✅ | ❌ (needs gateway) | ✅ | ✅ |
| Grafana Integration | ✅ Native | ✅ Native | ❌ Limited | ✅ Good |
| Desktop App Friendly | ✅ Excellent | ⚠️ Requires workarounds | ✅ Good | ✅ Good |
| Unified Observability | ✅ Metrics, Traces, Logs | ❌ Metrics only | ❌ Analytics focused | ❌ Metrics only |
| Industry Standard | ✅ Growing fast | ✅ Established | ❌ Proprietary | ⚠️ Less common |
| Self-hostable | ✅ | ✅ | ✅ | ✅ |
| Data Ownership | ✅ Full control | ✅ Full control | ⚠️ Depends on setup | ✅ Full control |

## Troubleshooting

### Metrics Not Appearing

1. Check endpoint connectivity:
   ```bash
   curl -v your-otlp-endpoint:4317
   ```

2. Enable debug logging:
   ```bash
   RUST_LOG=debug OTEL_LOG_LEVEL=debug arenabuddy
   ```

3. Verify authentication headers are correct

### High Memory Usage

- Increase export interval: `OTEL_METRIC_EXPORT_INTERVAL=300`
- Check for metric cardinality issues (too many unique label combinations)

### Connection Errors

- For gRPC: Ensure port 4317 is accessible
- For HTTP: Try switching protocol: `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf`
- Check firewall rules and proxy settings

## Security Best Practices

1. **Never hardcode credentials** - Use environment variables or secret management
2. **Use TLS/HTTPS** for production endpoints
3. **Rotate API keys** regularly
4. **Limit metric cardinality** to prevent resource exhaustion
5. **Set up rate limiting** on your OTLP endpoint

## Additional Resources

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Grafana OTLP Documentation](https://grafana.com/docs/grafana-cloud/send-data/otlp/)
- [OTLP Specification](https://opentelemetry.io/docs/specs/otlp/)
- [Grafana Agent (Alloy)](https://grafana.com/docs/alloy/latest/)