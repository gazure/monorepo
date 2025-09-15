# Arenabuddy Metrics Documentation

Arenabuddy includes built-in metrics collection using OpenTelemetry (OTLP) to track game and draft ingestion, helping you monitor the application's performance and usage patterns.

## Why OpenTelemetry?

- **Push-based**: Perfect for desktop applications - no need to expose ports
- **Industry Standard**: Growing ecosystem with wide support
- **Grafana Compatible**: Native integration with Grafana Cloud and Grafana Stack
- **Flexible**: Can send to multiple backends simultaneously

## Collected Metrics

The following metrics are automatically collected:

- **Games Ingested** (`arenabuddy.games.ingested`) - Total number of games processed
- **Drafts Ingested** (`arenabuddy.drafts.ingested`) - Total number of drafts processed  
- **Parse Errors** (`arenabuddy.parse.errors`) - Total number of parsing errors encountered
- **Active Sessions** (`arenabuddy.sessions.active`) - Current number of active sessions

## Configuration

Metrics are configured via environment variables. Create a `.env` file in the arenabuddy directory:

```bash
# OTLP Endpoint (required for metrics export)
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# Service configuration
OTEL_SERVICE_NAME=arenabuddy
ARENABUDDY_INSTANCE_ID=my-desktop-01

# Export interval in seconds (default: 60)
OTEL_METRIC_EXPORT_INTERVAL=60

# Protocol: grpc or http/protobuf (default: grpc)
OTEL_EXPORTER_OTLP_PROTOCOL=grpc

# Enable/disable metrics (default: true)
OTEL_METRICS_ENABLED=true

# Authentication headers (for cloud providers)
# Format: key1=value1,key2=value2
OTEL_EXPORTER_OTLP_HEADERS=Authorization=Bearer YOUR_TOKEN
```

## Quick Start - Local Metrics Stack

### Option 1: OpenTelemetry Collector + Grafana

Create a `docker-compose.yml`:

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

  # Prometheus for metrics storage
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus

  # Grafana for visualization
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-storage:/var/lib/grafana

volumes:
  prometheus-data:
  grafana-storage:
```

Create `otel-collector-config.yaml`:

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
  prometheus:
    endpoint: "0.0.0.0:8889"
    
service:
  pipelines:
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [prometheus]
```

Create `prometheus.yml`:

```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'otel-collector'
    static_configs:
      - targets: ['otel-collector:8889']
```

Start the stack:

```bash
docker-compose up -d
```

Configure Arenabuddy:

```bash
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=arenabuddy
OTEL_METRICS_ENABLED=true
```

Access:
- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9090

### Option 2: Grafana Cloud (Recommended for Production)

1. Sign up for [Grafana Cloud](https://grafana.com/products/cloud/) (free tier available)

2. Get your OTLP endpoint and credentials:
   - Navigate to your stack → Configuration → OpenTelemetry
   - Copy the endpoint and authentication token

3. Configure Arenabuddy:

```bash
# Grafana Cloud configuration
OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp-gateway-prod-us-central-0.grafana.net:443
OTEL_EXPORTER_OTLP_HEADERS=Authorization=Basic YOUR_BASE64_TOKEN
OTEL_SERVICE_NAME=arenabuddy
OTEL_EXPORTER_OTLP_PROTOCOL=grpc
```

### Option 3: InfluxDB + Telegraf

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
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin

volumes:
  influxdb-data:
```

Create `telegraf.conf`:

```toml
[[inputs.opentelemetry]]
  service_address = "0.0.0.0:4317"

[[outputs.influxdb_v2]]
  urls = ["http://influxdb:8086"]
  token = "your-token-here"
  organization = "arenabuddy"
  bucket = "metrics"
```

## Grafana Dashboard

Import this dashboard JSON for instant visualization:

```json
{
  "dashboard": {
    "title": "Arenabuddy Metrics",
    "panels": [
      {
        "title": "Games Ingested Rate",
        "gridPos": { "h": 8, "w": 12, "x": 0, "y": 0 },
        "targets": [
          {
            "expr": "rate(arenabuddy_games_ingested[5m]) * 60",
            "legendFormat": "{{instance}}"
          }
        ]
      },
      {
        "title": "Drafts Ingested Rate",
        "gridPos": { "h": 8, "w": 12, "x": 12, "y": 0 },
        "targets": [
          {
            "expr": "rate(arenabuddy_drafts_ingested[5m]) * 60",
            "legendFormat": "{{instance}}"
          }
        ]
      },
      {
        "title": "Active Sessions",
        "gridPos": { "h": 8, "w": 12, "x": 0, "y": 8 },
        "targets": [
          {
            "expr": "sum(arenabuddy_sessions_active) by (instance)",
            "legendFormat": "{{instance}}"
          }
        ]
      },
      {
        "title": "Parse Error Rate",
        "gridPos": { "h": 8, "w": 12, "x": 12, "y": 8 },
        "targets": [
          {
            "expr": "rate(arenabuddy_parse_errors[5m]) * 60",
            "legendFormat": "Errors/min"
          }
        ]
      }
    ]
  }
}
```

## PromQL Queries

Example queries for Prometheus/Grafana:

```promql
# Total games ingested
sum(arenabuddy_games_ingested)

# Games ingested per minute
rate(arenabuddy_games_ingested[5m]) * 60

# Drafts ingested today
increase(arenabuddy_drafts_ingested[1d])

# Parse error rate
rate(arenabuddy_parse_errors[5m])

# Active sessions by instance
arenabuddy_sessions_active

# Games by instance
sum by (instance) (arenabuddy_games_ingested)
```

## Migration from Prometheus Push Gateway

If you were using the old Prometheus push gateway approach:

1. **Update environment variables**:
   - Replace `ARENABUDDY_METRICS_URL` with `OTEL_EXPORTER_OTLP_ENDPOINT`
   - Replace `ARENABUDDY_METRICS_INTERVAL` with `OTEL_METRIC_EXPORT_INTERVAL`
   - Add `OTEL_SERVICE_NAME=arenabuddy`

2. **Update endpoints**:
   - Push Gateway: `http://localhost:9091` → OTLP: `http://localhost:4317`
   - Grafana Cloud: Use OTLP endpoint instead of Prometheus remote write

3. **Metric names have changed**:
   - `arenabuddy_games_ingested_total` → `arenabuddy_games_ingested`
   - `arenabuddy_drafts_ingested_total` → `arenabuddy_drafts_ingested`
   - `arenabuddy_parse_errors_total` → `arenabuddy_parse_errors`

## Troubleshooting

### Metrics not appearing

1. **Check connectivity**:
   ```bash
   telnet localhost 4317  # For gRPC
   curl http://localhost:4318  # For HTTP
   ```

2. **Enable debug logging**:
   ```bash
   RUST_LOG=debug OTEL_LOG_LEVEL=debug ./arenabuddy
   ```

3. **Verify configuration**:
   ```bash
   env | grep OTEL
   ```

### Connection errors

- **gRPC issues**: Try switching to HTTP protocol
  ```bash
  OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
  OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
  ```

- **TLS/Certificate issues**: For local development, use HTTP instead of HTTPS

- **Authentication failures**: Check your headers format
  ```bash
  OTEL_EXPORTER_OTLP_HEADERS=Authorization=Bearer token123
  ```

### High memory usage

- Increase export interval: `OTEL_METRIC_EXPORT_INTERVAL=300`
- Check for cardinality issues (too many unique label combinations)

## Security Best Practices

1. **Never hardcode credentials** - Always use environment variables
2. **Use TLS/HTTPS** for production endpoints
3. **Rotate API keys** regularly
4. **Limit metric cardinality** to prevent resource exhaustion
5. **Set up rate limiting** on your OTLP endpoint

## Performance Impact

The OpenTelemetry metrics system has minimal impact:
- **Memory**: ~2KB for metric instruments
- **CPU**: Negligible (atomic operations)
- **Network**: One gRPC/HTTP request every 60 seconds (configurable)
- **Disk**: No local persistence required

## Disabling Metrics

To completely disable metrics collection:

```bash
OTEL_METRICS_ENABLED=false
```

Or simply don't set `OTEL_EXPORTER_OTLP_ENDPOINT` - metrics will be collected locally but not exported.

## Additional Resources

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Grafana OTLP Documentation](https://grafana.com/docs/grafana-cloud/send-data/otlp/)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [Grafana Cloud Free Tier](https://grafana.com/products/cloud/pricing/)