# Deploying christmas

Target: `https://christmas.grantazure.com`, running from its own compose project
in this repo (`christmas/compose.yaml`), joined to the Docker network the
arenabuddy stack already runs on so it can reuse that Caddy, its TLS
certificates, and its PostgreSQL.

Being a **separate compose project** is deliberate. arenabuddy deploys with
`docker compose up -d --remove-orphans`; a service defined in *its* compose file
would be torn down or silently reverted to `:latest` by that. Owning our own
project keeps the two deploys independent.

The app is **fullstack** — a Rust server process plus a database — unlike
arenabuddy's `web`, which is static files behind Caddy.

## What this repo does

`.github/workflows/deploy-christmas.yml` builds the image from the root
`Dockerfile` (`APP_NAME=christmas`) and pushes it to ECR as `christmas:latest`
and `christmas:<sha>` on every push to `main` that touches the crate.

It then SSHes to the droplet and rolls the service over, the same way arenabuddy
does. **The one-time setup below must be in place before the first run**, or the
deploy job will fail on a missing network or database.

To deploy by hand:

```bash
cd /root/code/monorepo && git pull --ff-only origin main
cd christmas
docker compose --env-file /root/.env pull
docker compose --env-file /root/.env up -d
```

Required repo secrets: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION`,
and — for the optional deploy job — `DEPLOY_HOST`, `DEPLOY_SSH_KEY`.

## One-time setup

The ECR repository is not on this list: the workflow creates `christmas` on its
first run if it does not already exist, because ECR does not create repositories
on push and the resulting error is unhelpful.

### 1. Database

Reuse the existing PostgreSQL 17 container rather than running a second one:

```bash
docker compose exec postgres psql -U arenabuddy -d postgres \
  -c "CREATE USER christmas WITH PASSWORD '<pick-one>';" \
  -c "CREATE DATABASE christmas OWNER christmas;"
```

Migrations run automatically the first time the app touches the database.

### 2. The shared network

`compose.yaml` joins an existing external network. Compose names a project's
default network `<project>_default`, and arenabuddy's project is the directory
its compose file sits in — so it is most likely `server_default`. Confirm:

```bash
docker network ls
docker network inspect server_default --format '{{range .Containers}}{{.Name}} {{end}}'
```

`server_default` is confirmed correct. If that ever changes, set
`CHRISTMAS_NETWORK` in `/root/.env`. Getting it wrong fails loudly — *"network
... declared as external, but could not be found"* — rather than starting
something unreachable.

### 3. Droplet environment

The monorepo must be checked out at `/root/code/monorepo` — the deploy job
`git pull`s there and runs compose from `christmas/`.

Everything else is environment. `/root/.env` is read twice: `source`d by the
deploy script, and passed to compose as `--env-file`.

**Already present for arenabuddy** — nothing to do, just don't remove them:

| Variable | Used by | Why |
| --- | --- | --- |
| `AWS_REGION` | deploy script | Builds the ECR registry hostname, and `aws ecr get-login-password` |
| `AWS_ACCESS_KEY_ID` | `aws` CLI on the droplet | `aws sts get-caller-identity` discovers the account id |
| `AWS_SECRET_ACCESS_KEY` | `aws` CLI on the droplet | as above |

**New, add these:**

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `CHRISTMAS_DB_PASSWORD` | **yes** | — | Password for the `christmas` database role. Compose refuses to start without it. |
| `CHRISTMAS_ADMIN_PASSWORD` | strongly | *(empty)* | Manager password. Empty means nobody can reach Manage. |
| `CHRISTMAS_VIEW_PASSWORD` | strongly | *(empty)* | Family password. **If this and the admin one are both empty the site is wide open**, and the container logs a warning saying so. |
| `CHRISTMAS_IMAGE` | no | `christmas:latest` | The deploy job exports the exact `:<sha>` tag, which wins. Worth setting to the ECR `:latest` path so a manual `docker compose up` still finds a real image. |
| `CHRISTMAS_NETWORK` | no | `server_default` | The arenabuddy network. The default is correct today. |
| `CHRISTMAS_DB_HOST` | no | `postgres` | PostgreSQL's service name on the shared network. |

```sh
# --- christmas ---
CHRISTMAS_DB_PASSWORD=<the password from step 1>
CHRISTMAS_VIEW_PASSWORD=<what the family gets told>
CHRISTMAS_ADMIN_PASSWORD=<only you>
CHRISTMAS_IMAGE=<account>.dkr.ecr.<region>.amazonaws.com/christmas:latest
```

### 4. DNS

An `A` record for `christmas.grantazure.com` pointing at the droplet. Note this
is a **different zone** from `arenabuddy.io`, so it is a new zone to configure,
not just a new record. Caddy issues the certificate automatically on first
request, into the `caddy_data` volume that already exists.

### 5. The one change in the arenabuddy repo

Caddy lives over there and owns ports 80/443, so it needs to learn the hostname.
Add to `server/Caddyfile`:

```
christmas.grantazure.com {
	reverse_proxy christmas:8080
}
```

That is the *only* arenabuddy-side change — no compose edit, no `depends_on`.
`christmas` resolves because both projects share the network and the service
declares that name as an explicit network alias.

### GitHub repository secrets

Already configured, listed here for completeness: `AWS_ACCESS_KEY_ID`,
`AWS_SECRET_ACCESS_KEY`, `AWS_REGION`, `DEPLOY_HOST`, `DEPLOY_SSH_KEY`.

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

## Four things that will bite otherwise

1. **The Caddyfile is a bind mount.** Editing it does not reload the running
   Caddy — `docker compose up -d` won't restart a container whose compose
   definition is unchanged. After editing:
   ```bash
   docker compose exec caddy caddy reload --config /etc/caddy/Caddyfile
   ```

2. **Compose only substitutes variables it can see.** `source /root/.env` sets
   shell variables, not environment ones, so the deploy passes `--env-file
   /root/.env` explicitly. Running `docker compose up -d` in that directory
   without it will fail on the missing database password — which is the intended
   failure, not a bug.

3. **`depends_on` cannot cross compose projects**, so christmas may start before
   PostgreSQL is ready. That is survivable by design: the pool connects lazily on
   first use and only caches a *successful* connection, so once the database
   comes up the next request succeeds without a restart.

4. **The server binary is named `server`, not `christmas`.** `dx bundle` emits
   `target/dx/<app>/release/web/{server,public}`. The root `Dockerfile` accounts
   for this; don't "fix" its `CMD` back to `${APP_NAME}`.

## Verifying a deploy

```bash
# 303 to /login when signed out — a 200 here means auth is not configured
curl -sI https://christmas.grantazure.com | head -1
cd /root/code/monorepo/christmas
docker compose --env-file /root/.env logs | tail -20   # "database ready", "password protection enabled"
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
cd /root/code/monorepo/christmas
docker compose --env-file /root/.env run --rm \
  christmas /usr/local/app/server --seed /usr/local/app/seed/family.json
```

Seeding is idempotent, so re-running is safe. Alternatively add the pools and
people through the Manage page.
