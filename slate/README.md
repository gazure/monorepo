# slate

Self-hosted calendar feeds for sports schedules that stay correct when the
leagues move games around.

A static `.ics` file is right until the first flex-scheduling change, then it is
quietly wrong. slate serves *subscribable* feeds instead: your calendar app
re-polls the URL, and a rescheduled game updates in place.

## Quick start

```bash
cargo run -p slate -- sync           # fetch and store the configured feeds
cargo run -p slate -- serve          # http://127.0.0.1:8477
```

The database is created on first run if it does not exist, and migrations run
on startup.

Then subscribe to `http://127.0.0.1:8477/nfl/sea.ics` from Apple Calendar,
Google Calendar, or Outlook. Migrations run automatically on startup.

To produce a one-off file instead of subscribing:

```bash
cargo run -p slate -- export --league nfl --team sea --out seahawks.ics
```

## Configuration

`slate.toml`:

```toml
database_url = "postgresql://postgres:postgres@localhost:30432/slate"
listen = "127.0.0.1:8477"
poll_interval_minutes = 360

[[feeds]]
league = "nfl"
team = "sea"
```

`SLATE_DATABASE_URL`, `SLATE_LISTEN`, and `SLATE_POLL_INTERVAL_MINUTES`
override the file, which is how the container is configured.

Supported leagues: `nfl`, `ncaaf`, `nba`, `nhl`, `ncaam`, `mlb`, `mls`, `wnba`.
Teams use the abbreviation ESPN uses (`sea`, `lar`, `gb`). Set `season` on a feed
to pin one; otherwise the current season is derived from the league's calendar,
which differs by sport — the NFL labels a season by the year it starts, the NBA
by the year it ends.

## How updates propagate

Two properties do the work, and both are easy to get wrong:

- **`UID`** is derived from the upstream game id and never changes. Without a
  stable UID, a moved game arrives as a second event rather than a revision.
- **`SEQUENCE`** is the count of calendar-significant changes observed for that
  game. Most clients ignore a revision whose SEQUENCE has not advanced.

Significance is deliberately narrow. A change to kickoff time, TBD status,
venue, or game status advances SEQUENCE. A broadcast change is recorded in the
revision log but does *not*, because re-notifying every subscriber over a TV
listing is noise.

Games with no confirmed kickoff — the `isTBDFlex` flag upstream — render as
all-day entries titled `(time TBD)` rather than asserting a placeholder hour.
When the league sets the time, the entry becomes a timed event and SEQUENCE
advances.

## Inspecting changes

```bash
cargo run -p slate -- changes --limit 20
```

```
* 2026-12-02 09:00  nfl  SEA @ SF       kickoff   2026-11-29T21:25:00Z -> 2026-11-30T01:20:00Z
  2026-12-02 09:00  nfl  SEA @ SF       broadcast FOX -> NBC
```

A leading `*` marks a change that advanced SEQUENCE.

## Running in the local compose stack

`docker-local-monitoring/` runs slate alongside its Postgres:

```bash
cd docker-local-monitoring
docker-compose up -d slate           # feeds on http://localhost:30847
```

Teams are configured in `docker-local-monitoring/slate/slate.toml`. See that
directory's README for details.

## Data source

ESPN's public site API. One endpoint shape covers every league, so adding a
league is a row in `LEAGUES`.

Its edge rejects both browser-imitating and bare tool user agents with a 403; it
accepts the conventional bot form carrying a contact URL, which is what
`espn::USER_AGENT` sends. That string is load-bearing.

## Tests

```bash
cargo test -p slate                                        # unit + integration
cargo test -p slate --test live -- --ignored --nocapture   # hits ESPN for real
```

The integration tests start a bundled PostgreSQL, so they need no local
database. The live test is ignored by default because it depends on the network
and on schedule data the leagues can legitimately change.
