# Deploying christmas

Target: `https://christmas.grantazure.com`, running as a service inside the
existing arenabuddy compose stack on the DigitalOcean droplet, reusing its Caddy,
TLS certificates, and PostgreSQL.

The app is **fullstack** — a Rust server process plus a database — unlike
arenabuddy's `web`, which is static files behind Caddy. So it needs its own
service entry, not a copy of that pattern.

## What this repo does

`.github/workflows/deploy-christmas.yml` builds the image from the root
`Dockerfile` (`APP_NAME=christmas`) and pushes it to ECR as `christmas:latest`
and `christmas:<sha>` on every push to `main` that touches the crate.

It does **not** deploy on push. The compose file and Caddyfile live in the
arenabuddy repo, so the one-time setup below has to land there first; after that,
run the workflow manually with **Also restart the service on the droplet**
checked, or just `docker compose up -d christmas` on the box.

Required repo secrets: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`,
and — for the optional deploy job — `DEPLOY_HOST`, `DEPLOY_SSH_KEY`.

## One-time setup

### 1. ECR repository

ECR does not create repositories on push.

```bash
aws ecr create-repository --repository-name christmas --region "$AWS_REGION"
```

### 2. Database

Reuse the existing PostgreSQL 17 container rather than running a second one:

```bash
docker compose exec postgres psql -U arenabuddy -d postgres \
  -c "CREATE USER christmas WITH PASSWORD '<pick-one>';" \
  -c "CREATE DATABASE christmas OWNER christmas;"
```

Migrations run automatically the first time the app touches the database.

### 3. Droplet environment

Add to `/root/.env`:

```sh
CHRISTMAS_DB_PASSWORD=<the password above>
# Must be defined here, not only at deploy time — see the gotcha below.
CHRISTMAS_IMAGE=<account>.dkr.ecr.<region>.amazonaws.com/christmas:latest

# Shared passwords. If BOTH are unset the site is completely open, and the
# container logs a warning saying so at boot.
CHRISTMAS_VIEW_PASSWORD=<what the family gets told>
CHRISTMAS_ADMIN_PASSWORD=<only you>
```

### 4. DNS

An `A` record for `christmas.grantazure.com` pointing at the droplet. Note this
is a **different zone** from `arenabuddy.io`, so it is a new zone to configure,
not just a new record. Caddy issues the certificate automatically on first
request, into the `caddy_data` volume that already exists.

### 5. Changes in the arenabuddy repo

`server/docker-compose.yml` — add alongside `web`:

```yaml
  christmas:
    image: ${CHRISTMAS_IMAGE:-christmas:latest}
    restart: unless-stopped
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      CHRISTMAS_DATABASE_URL: postgres://christmas:${CHRISTMAS_DB_PASSWORD}@postgres/christmas
      CHRISTMAS_VIEW_PASSWORD: ${CHRISTMAS_VIEW_PASSWORD}
      CHRISTMAS_ADMIN_PASSWORD: ${CHRISTMAS_ADMIN_PASSWORD}
      IP: 0.0.0.0
      PORT: "8080"
    expose:
      - "8080"
```

Add `christmas` to the `caddy` service's `depends_on`.

`server/Caddyfile` — add a site block:

```
christmas.grantazure.com {
	reverse_proxy christmas:8080
}
```

## Passwords

Two shared passwords, no accounts:

- `CHRISTMAS_VIEW_PASSWORD` — the family. Reads everything.
- `CHRISTMAS_ADMIN_PASSWORD` — whoever runs the draw. Also reaches Manage and can
  change data.

Sessions live in the server's memory, so **a redeploy signs everyone out**. That
is deliberate: it keeps the whole thing to one env var per role with nothing to
migrate, and re-entering a password once a deploy is not a burden here.

Rotating a password is a compose restart with the new value; every existing
session dies with the old process anyway.

## Three things that will bite otherwise

1. **The Caddyfile is a bind mount.** Editing it does not reload the running
   Caddy — `docker compose up -d` won't restart a container whose compose
   definition is unchanged. After editing:
   ```bash
   docker compose exec caddy caddy reload --config /etc/caddy/Caddyfile
   ```

2. **arenabuddy's own deploy runs `docker compose up -d --remove-orphans`.** If
   `CHRISTMAS_IMAGE` isn't set in that shell, christmas gets recreated from the
   compose default and silently reverts. Defining it in `/root/.env` (step 3) is
   what prevents this — not just exporting it at deploy time.

3. **The server binary is named `server`, not `christmas`.** `dx bundle` emits
   `target/dx/<app>/release/web/{server,public}`. The root `Dockerfile` accounts
   for this; don't "fix" its `CMD` back to `${APP_NAME}`.

## Verifying a deploy

```bash
# 303 to /login when signed out — a 200 here means auth is not configured
curl -sI https://christmas.grantazure.com | head -1
docker compose logs christmas | tail -20   # "database ready", "password protection enabled"
```

If the logs say *"the site is completely open"*, the passwords did not reach the
container — check that `/root/.env` is populated and that the compose service
passes both variables through.

Then load the site and check that the **first** request after a cold boot is
fast. A multi-second first response means the connection pool is being built on
a runtime that has since died — the bug `src/lib.rs` documents and avoids by
connecting lazily.

## Seeding production

Once, to load the family:

```bash
docker compose run --rm christmas /usr/local/app/server --seed /usr/local/app/seed/family.json
```

Seeding is idempotent, so re-running is safe. Alternatively add the pools and
people through the Manage page.
