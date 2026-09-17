# Deploy Slate

Slate serves public calendar feeds at `https://slate.grantazure.com`. Its own
Compose project joins ArenaBuddy's `server_default` network and uses the existing
PostgreSQL and Caddy containers, following the Christmas deployment setup.
The separate project keeps ArenaBuddy's `--remove-orphans` deploys from removing
Slate.

## Build and deployment

`.github/workflows/deploy-slate.yml` builds `Dockerfile.bin` with `APP_NAME=slate`,
creates the `slate` ECR repository if needed, and pushes `latest` and commit SHA
tags. It then updates `/root/code/monorepo` on the droplet and starts Slate with
the image built by that run. Pushes to `main` that change Slate, the workspace
manifest or lockfile, the Dockerfile, or the workflow trigger a deployment.
You can also run the workflow manually.

The workflow uses the monorepo's existing GitHub secrets: `AWS_ACCESS_KEY_ID`,
`AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `DEPLOY_HOST`, and `DEPLOY_SSH_KEY`.
The AWS user needs permission to create the ECR repository on the first run.

## One-time setup

Complete these steps before running the deployment workflow.

### Create the database

On the droplet, open PostgreSQL from the ArenaBuddy stack:

```sh
cd /root/code/arenabuddy/server
docker compose exec postgres psql -U arenabuddy -d postgres
```

Create a dedicated role and database. Use the password prompt to set a randomly
generated hexadecimal password so it can also appear in the connection URL:

```sql
CREATE USER slate;
\password slate
CREATE DATABASE slate OWNER slate;
\q
```

Slate runs migrations on startup. The database must already exist because this
role does not have permission to create databases. If PostgreSQL is unavailable
at startup, the process exits and Compose restarts it.

### Configure the environment

Add these values to `/root/.env`, using the password you set for the database:

```sh
SLATE_DB_PASSWORD=<hexadecimal-password>
SLATE_IMAGE=<account>.dkr.ecr.<region>.amazonaws.com/slate:latest
```

The workflow exports the commit SHA image reference, which takes precedence over
`SLATE_IMAGE` in this file. Keep the existing AWS variables in `/root/.env`.

Optional variables are `SLATE_DB_HOST` (default `postgres`), `SLATE_NETWORK`
(default `server_default`), and `SLATE_POLL_INTERVAL_MINUTES` (default `360`).
Confirm that the shared network contains PostgreSQL and Caddy:

```sh
docker network inspect server_default --format '{{range .Containers}}{{.Name}} {{end}}'
```

`slate/deploy/feeds.toml` selects the published teams. It starts with the local
stack's Seattle NFL, MLB, NHL, and MLS feeds and Washington college football.
Edit this file and deploy to change the feeds.

### Configure DNS and Caddy

Create an `A` record for `slate.grantazure.com` pointing to the droplet.
The ArenaBuddy repository's `server/Caddyfile` includes this route:

```caddyfile
slate.grantazure.com {
    reverse_proxy slate:8477
}
```

After the ArenaBuddy change reaches the droplet, validate and reload Caddy:

```sh
cd /root/code/arenabuddy
git pull --ff-only origin main
cd server
docker compose exec caddy caddy validate --config /etc/caddy/Caddyfile
docker compose exec caddy caddy reload --config /etc/caddy/Caddyfile
```

Caddy obtains the TLS certificate automatically. Updating the bind-mounted
Caddyfile or running `docker compose up -d` does not reload the Caddy process.

## Verify the deployment

Run the Slate workflow after setup, then check the process and public endpoints:

```sh
cd /root/code/monorepo/slate
docker compose --env-file /root/.env ps
docker compose --env-file /root/.env logs --tail 50 slate
curl --fail https://slate.grantazure.com/healthz
curl --fail https://slate.grantazure.com/nfl/sea.ics
```

The health endpoint returns `ok`. The calendar endpoint returns calendar data
after the first background sync completes. Check the logs for upstream sync
errors if the feed is unavailable. Compose's `--wait` checks that the container
is running; the HTTP checks verify that Slate is reachable and serving feeds.

To deploy manually, authenticate Docker to ECR with the droplet's AWS credentials,
then run:

```sh
cd /root/code/monorepo
git pull --ff-only origin main
cd slate
docker compose --env-file /root/.env pull
docker compose --env-file /root/.env up -d --wait --wait-timeout 120
```

To roll back the application image, set `SLATE_IMAGE` to a previous ECR commit SHA
tag before the manual pull and start commands. Database migrations are not
reversed by an image rollback.
