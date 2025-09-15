# Local Development Monitoring Stack

A comprehensive observability stack for local development with OpenTelemetry, Prometheus, Grafana, and more.

## 🚀 Quick Start

```bash
# Start all services
docker-compose up -d

# Stop all services
docker-compose down

# Stop and remove all data
docker-compose down -v
```

## 📊 Services

| Service | Port | URL | Purpose |
|---------|------|-----|---------|
| **Grafana** | 30300 | http://localhost:30300 | Visualization & Dashboards (admin/admin) |
| **Prometheus** | 30090 | http://localhost:30090 | Metrics Storage & Querying |
| **PostgreSQL** | 30432 | `postgresql://localhost:30432` | Database (postgres/postgres) |
| **OpenTelemetry Collector** | 30317 (gRPC), 30318 (HTTP) | - | Telemetry Collection |
| **Jaeger** | 36686 | http://localhost:36686 | Distributed Tracing UI |
| **Loki** | 33100 | http://localhost:33100 | Log Aggregation |
| **Node Exporter** | 30100 | http://localhost:30100/metrics | Host Metrics |
| **Postgres Exporter** | 30187 | http://localhost:30187/metrics | PostgreSQL Metrics |
| **cAdvisor** | 30880 | http://localhost:30880 | Container Metrics |

### Additional Endpoints

- **OTEL Collector Metrics**: http://localhost:30889/metrics
- **OTEL Collector Health**: http://localhost:31313/health
- **OTEL Collector ZPages**: http://localhost:35567/debug/tracez

## 🔧 Configuration

### Sending Metrics to OpenTelemetry Collector

The OpenTelemetry Collector accepts metrics, traces, and logs via OTLP protocol:

#### From Your Application (e.g., Arenabuddy)

```bash
# Environment variables for OTLP
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:30317  # gRPC
# or
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:30318  # HTTP

export OTEL_SERVICE_NAME=my-app
export OTEL_RESOURCE_ATTRIBUTES=deployment.environment=local
```

#### Using OpenTelemetry SDK (Python Example)

```python
from opentelemetry import trace, metrics
from opentelemetry.exporter.otlp.proto.grpc.metric_exporter import OTLPMetricExporter
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.trace import TracerProvider

# Configure metrics
metric_exporter = OTLPMetricExporter(endpoint="localhost:30317", insecure=True)
metrics.set_meter_provider(MeterProvider())

# Configure traces
trace_exporter = OTLPSpanExporter(endpoint="localhost:30317", insecure=True)
trace.set_tracer_provider(TracerProvider())
```

#### Using OpenTelemetry SDK (Node.js Example)

```javascript
const { NodeSDK } = require('@opentelemetry/sdk-node');
const { OTLPMetricExporter } = require('@opentelemetry/exporter-metrics-otlp-grpc');
const { OTLPTraceExporter } = require('@opentelemetry/exporter-trace-otlp-grpc');

const sdk = new NodeSDK({
  traceExporter: new OTLPTraceExporter({
    url: 'http://localhost:30317',
  }),
  metricExporter: new OTLPMetricExporter({
    url: 'http://localhost:30317',
  }),
  serviceName: 'my-app',
});

sdk.start();
```

#### Using cURL (Testing)

```bash
# Send a test metric via HTTP
curl -X POST http://localhost:30318/v1/metrics \
  -H "Content-Type: application/json" \
  -d '{
    "resourceMetrics": [{
      "resource": {
        "attributes": [{
          "key": "service.name",
          "value": {"stringValue": "test-service"}
        }]
      },
      "scopeMetrics": [{
        "metrics": [{
          "name": "test.counter",
          "unit": "1",
          "sum": {
            "dataPoints": [{
              "asInt": "1",
              "timeUnixNano": "'$(date +%s%N)'"
            }]
          }
        }]
      }]
    }]
  }'
```

### Sending Logs to Loki

#### Via OpenTelemetry Collector

```python
from opentelemetry.exporter.otlp.proto.grpc._log_exporter import OTLPLogExporter
from opentelemetry.sdk._logs import LoggerProvider, LoggingHandler

# Configure logging
log_exporter = OTLPLogExporter(endpoint="localhost:30317", insecure=True)
logger_provider = LoggerProvider()
logger_provider.add_log_record_processor(BatchLogRecordProcessor(log_exporter))

# Use with Python logging
import logging
handler = LoggingHandler(level=logging.INFO, logger_provider=logger_provider)
logging.getLogger().addHandler(handler)
```

#### Direct to Loki

```bash
# Send logs directly to Loki
curl -X POST http://localhost:33100/loki/api/v1/push \
  -H "Content-Type: application/json" \
  -d '{
    "streams": [{
      "stream": {
        "job": "test",
        "level": "info"
      },
      "values": [
        ["'$(date +%s%N)'", "Test log message"]
      ]
    }]
  }'
```

### Sending Traces to Jaeger

Traces sent to the OpenTelemetry Collector are automatically forwarded to Jaeger:

```python
from opentelemetry import trace
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

# Setup
trace_exporter = OTLPSpanExporter(endpoint="localhost:30317", insecure=True)
provider = TracerProvider()
processor = BatchSpanProcessor(trace_exporter)
provider.add_span_processor(processor)
trace.set_tracer_provider(provider)

# Use
tracer = trace.get_tracer(__name__)
with tracer.start_as_current_span("my-operation"):
    # Your code here
    pass
```

## 📈 Grafana Dashboards

### Pre-configured Data Sources

1. **Prometheus** - Metrics from all exporters
2. **PostgreSQL** - Direct database queries
3. **Loki** - Log aggregation and search
4. **Jaeger** - Distributed tracing
5. **OTel-Collector-Metrics** - Metrics exported by OTEL

### Creating Dashboards

1. Open Grafana at http://localhost:30300
2. Login with `admin/admin`
3. Navigate to Dashboards → New Dashboard
4. Add panels using any of the configured data sources

### Example Queries

#### Prometheus Queries
```promql
# Container memory usage
container_memory_usage_bytes{name=~"local-.*"}

# PostgreSQL connections
pg_stat_database_numbackends{datname="localdev"}

# Request rate from OTEL
rate(http_server_duration_count[5m])
```

#### Loki Queries
```logql
# All error logs
{job="my-app"} |= "error"

# Logs from specific service
{service="api"} | json | level="error"
```

#### PostgreSQL Queries
```sql
-- Active connections
SELECT count(*) FROM pg_stat_activity;

-- Database sizes
SELECT pg_database_size('localdev');
```

## 🔍 OpenTelemetry Collector Details

The collector is configured to:
- Accept OTLP data via gRPC (port 30317) and HTTP (port 30318)
- Export metrics to Prometheus
- Export traces to Jaeger
- Export logs to Loki
- Add resource detection for environment metadata
- Apply memory limits to prevent OOM
- Batch data for efficient processing

### Collector Pipelines

1. **Metrics Pipeline**: OTLP → Memory Limiter → Resource Detection → Batch → Prometheus
2. **Traces Pipeline**: OTLP → Memory Limiter → Span Processing → Batch → Jaeger
3. **Logs Pipeline**: OTLP → Memory Limiter → Resource Detection → Batch → Loki
4. **SpanMetrics**: Generates metrics from traces automatically

## 🐛 Debugging

### Check Service Health

```bash
# Check all services are running
docker-compose ps

# Check OTEL Collector health
curl http://localhost:31313/health

# Check Prometheus targets
open http://localhost:30090/targets

# View OTEL Collector logs
docker-compose logs otel-collector

# View all logs
docker-compose logs -f
```

### Common Issues

#### Port Conflicts
If you get port binding errors, check for conflicting services:
```bash
# Check what's using a port (e.g., 30300)
lsof -i :30300
```

#### Container Can't Connect to Host Services
Use `host.docker.internal` instead of `localhost` when connecting from containers to host services.

#### Metrics Not Appearing
1. Check OTEL Collector logs for errors
2. Verify Prometheus is scraping: http://localhost:30090/targets
3. Check your application is sending to the correct endpoint

## 🎯 Use Cases

### Application Performance Monitoring
1. Send traces from your application to OTEL Collector
2. View distributed traces in Jaeger
3. Analyze automatically generated metrics in Grafana

### Database Monitoring
1. PostgreSQL metrics are automatically collected
2. Create dashboards for query performance, connections, etc.
3. Set up alerts for slow queries or connection issues

### Container Monitoring
1. cAdvisor collects container metrics automatically
2. Monitor CPU, memory, network, and disk usage
3. Track resource usage per container

### Log Analysis
1. Send application logs to Loki via OTEL
2. Correlate logs with traces using trace IDs
3. Create log-based metrics and alerts

## 📦 Data Persistence

Data is persisted in named Docker volumes:
- `local_postgres_data` - PostgreSQL data
- `local_prometheus_data` - Prometheus metrics
- `local_grafana_data` - Grafana dashboards and settings
- `local_loki_data` - Loki logs

To reset all data:
```bash
docker-compose down -v
```

## 🔧 Customization

### Modify OpenTelemetry Collector Config
Edit `otel-collector/otel-collector-config.yaml` and restart:
```bash
docker-compose restart otel-collector
```

### Add Prometheus Scrape Targets
Edit `prometheus/prometheus.yml` and reload:
```bash
docker-compose restart prometheus
# Or use the reload endpoint
curl -X POST http://localhost:30090/-/reload
```

### Add Grafana Dashboards
Place dashboard JSON files in `grafana/provisioning/dashboards/`

### Adjust Resource Limits
Modify the `deploy` section in `docker-compose.yml` for any service.

## 🚢 Production Considerations

This stack is designed for **local development**. For production:

1. **Security**: Add authentication to all services
2. **Storage**: Use persistent storage backends (S3 for Loki, remote storage for Prometheus)
3. **High Availability**: Run multiple replicas with proper load balancing
4. **Resource Limits**: Set appropriate memory/CPU limits
5. **Retention**: Configure data retention policies
6. **Backup**: Implement backup strategies for critical data

## 📚 Additional Resources

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Prometheus Documentation](https://prometheus.io/docs/)
- [Grafana Documentation](https://grafana.com/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [Loki Documentation](https://grafana.com/docs/loki/)

## 🤝 Contributing

To add new services or exporters:
1. Add the service to `docker-compose.yml`
2. Configure Prometheus to scrape it (if applicable)
3. Add Grafana datasource configuration
4. Update this README with the new service details