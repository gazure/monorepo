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

### Baseball webui
- Port: 30800
- Web UI: http://localhost:30800 (or https://baseball.myhome.com through Caddy)
- Source: the `webui` crate in `sports/webui`, built from the workspace
  `Dockerfile` at the repo root with `APP_NAME=webui`
- Reads the `sports` database via `SPORTS_DATABASE_URL`, the same database the
  `Sports PostgreSQL` Grafana datasource and the baseball dashboard use

Because it is a `build:` service rather than a pulled image, it is not rebuilt
automatically when the Rust source changes:

    docker-compose build baseball_webui && docker-compose up -d baseball_webui

The first build compiles the whole workspace plus the wasm bundle, so expect it
to take a while; later builds reuse the cargo-chef dependency layer.

### slate (sports calendar feeds)
- Port: 30847
- Feeds: http://localhost:30847/nfl/sea.ics (or https://slate.myhome.com through Caddy)
- Source: the `slate` crate, built from the workspace `Dockerfile.bin` at the
  repo root with `APP_NAME=slate`

Polls ESPN on an interval, records every schedule change, and serves
subscribable `.ics` feeds. Subscribe to the URL from a calendar app once and
rescheduled games update in place.

Which teams get a feed is set in `slate/slate.toml` next to the compose file.
Connection settings come from the `SLATE_*` environment variables in
`docker-compose.yml` and override that file. Edit the feed list and restart:

    docker-compose restart slate

The container creates its own `slate` database on first start and runs its
migrations, so no `createdb` step is needed. It also syncs immediately on boot
rather than waiting out the first poll interval.

Useful commands:

    docker-compose logs -f slate
    docker-compose exec slate /usr/local/bin/app changes --limit 20
    curl -s localhost:30847/nfl/sea.ics | head

Like `baseball_webui`, it is a `build:` service, so Rust changes need an
explicit rebuild:

    docker-compose build slate && docker-compose up -d slate

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
