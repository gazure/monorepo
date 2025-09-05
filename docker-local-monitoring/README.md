# Docker Local Monitoring Stack

Local development environment with PostgreSQL, Prometheus, and Grafana.

## Services

### PostgreSQL
- Port: 30432
- Username: postgres
- Password: postgres

### Prometheus
- Port: 30090
- Web UI: http://localhost:30090

### Grafana
- Port: 30300
- Web UI: http://localhost:30300
- Username: admin
- Password: admin

## Data Persistence

All data is persisted in Docker volumes:
- local_postgres_data - PostgreSQL database files
- local_prometheus_data - Prometheus time series data
- local_grafana_data - Grafana dashboards and settings

To completely reset and remove all data:
docker-compose down -v

## Connecting to Services

### PostgreSQL Connection String
postgresql://postgres:postgres@localhost:30432/localdev

### Adding Data Sources in Grafana

1. Navigate to http://localhost:30300
2. Login with admin/admin
3. Go to Configuration > Data Sources
4. Add PostgreSQL:
   - Host: postgres:5432
   - Database: localdev
   - User: postgres
   - Password: postgres
   - SSL Mode: disable
5. Add Prometheus:
   - URL: http://prometheus:9090

## Adding Custom Metrics

Edit prometheus/prometheus.yml to add your application endpoints:

scrape_configs:
  - job_name: 'my-app'
    static_configs:
      - targets: ['host.docker.internal:8080']

Then reload Prometheus configuration:
docker-compose exec prometheus kill -HUP 1

## Network

All services are on the 'local_monitoring_network' bridge network for inter-service communication.

## Troubleshooting

Check service health:
docker-compose ps

Verify volumes:
docker volume ls | grep local_

Connect to PostgreSQL from command line:
psql -h localhost -p 30432 -U postgres -d localdev

Access Prometheus targets status:
http://localhost:30090/targets

Test Prometheus query:
http://localhost:30090/graph?g0.expr=up

## Optional Enhancements

### PostgreSQL Exporter for Prometheus

Add to docker-compose.yml to monitor PostgreSQL metrics:

  postgres_exporter:
    image: wrouesnel/postgres_exporter:latest
    container_name: local-postgres-exporter
    restart: unless-stopped
    environment:
      DATA_SOURCE_NAME: "postgresql://postgres:postgres@postgres:5432/localdev?sslmode=disable"
    ports:
      - "30187:9187"
    networks:
      - monitoring
    depends_on:
      - postgres

Then uncomment the postgres job in prometheus/prometheus.yml.
