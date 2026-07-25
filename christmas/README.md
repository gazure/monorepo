# Christmas Gift Exchange

A Dioxus fullstack app (WASM client + Axum server) on PostgreSQL that runs the
family gift exchange: who gives to whom, and the letter of the year that gifts
are built around.

Everything is public — the site is a reveal, not a secret.

## How a draw works

Each **pool** (Island Life, Grabergishimazureson, Pets) draws independently, and
each draw is recorded with the exact settings and RNG seed it ran under, so any
result can be explained or replayed later.

The permutation is shaped by three configurable rules:

| Setting | Default | Effect |
| --- | --- | --- |
| Cycle mode | One grand ring | Everyone in a single chain. The alternative is several independent rings, each at least 3 long. |
| Keep spouses apart | On | Skips anyone linked as `spouse` or `household`. `manual` exclusions always apply. |
| No repeat receivers | On (1 year) | Nobody gives to someone they gave to in the last N years. |

A minimum ring length of 3 is what rules out reciprocal A→B/B→A pairs — a ring of
two is just swapping gifts. "One grand ring" is the special case where the only
permissible ring contains everybody, so both modes run through the same search
(`src/matching.rs`).

Re-running a draw **never overwrites** the previous one; it records a new
revision. Only the live revision is visible to the family — superseded ones are
filtered out server-side rather than merely hidden in the UI, so they are never
sent to the browser, and a viewer cannot reach one by guessing its id. Managers
see the full history.

Each draw also stores the RNG seed it ran under, so a result can be replayed and
audited. It isn't shown in the UI; read it from the `exchange` table when you
need it.

Draws are deterministic: the same seed and inputs always produce the same result.
The search is bounded by a step budget rather than a wall clock, precisely so
that stays true regardless of machine load.

## Backfilling a past year

"No repeat receivers" can only avoid what it can see, so a pool whose earlier
years happened on paper has nothing to avoid. **Manage → Backfill a past year**
takes who gave to whom in an earlier year and records it as an ordinary
exchange, which is what makes it count: the same queries read it, so the next
draw avoids those pairings and skips the letter that year used.

A backfill may be incomplete — leave anyone you cannot remember blank — but it
cannot contradict itself: nobody gives to themselves, gives twice, or receives
from two people. `config` and `seed` are left empty rather than invented, since
nobody knows what rules an old draw ran under, and the pool page's "How it was
drawn" line is omitted for those years instead of claiming something untrue.

Backfilling uses the same revision counter as a draw, so it never overwrites an
existing year — recording 2019 twice leaves two revisions, the newer one live.

## Running it

Needs a PostgreSQL. The workspace's `docker-local-monitoring/docker-compose.yml`
exposes one on `127.0.0.1:30432`, which is the default.

```bash
# create the database once
docker exec local-postgres psql -U postgres -c "CREATE DATABASE christmas;"

# load the family: 3 pools, memberships, spouse links
cargo run -p christmas --features server -- --seed christmas/seed/family.json

# serve
dx serve --package christmas
```

Migrations run automatically on first database use.

`CHRISTMAS_DATABASE_URL` overrides the connection string. `CHRISTMAS_LOG_FORMAT=pretty`
switches logging from JSON to human-readable.

There is also a zero-setup mode that boots a bundled PostgreSQL, deliberately
kept out of the default feature set so production images don't carry it:

```bash
cargo run -p christmas --features embedded -- --embedded
```

## Seed data

`seed/family.json` holds the pools, their members, and the five spouse links,
recovered from the pre-2026 YAML config. Seeding is idempotent — re-running adds
what's missing and leaves everything else alone.

Grabergishimazureson keeps its historical letter set (`ACDIJLMNORSTUXYZ`); the
other pools draw from the whole alphabet.

## Tests

```bash
cargo test -p christmas
```

The unit tests cover the draw engine's properties over many seeds: always a
permutation, never a fixed point, never a reciprocal pair, every ring at least
`min_len`, every exclusion respected, and identical output for identical seeds.

Database-backed tests are ignored by default:

```bash
docker exec local-postgres psql -U postgres -c "CREATE DATABASE christmas_test;"
CHRISTMAS_DATABASE_URL=postgresql://postgres:postgres@localhost:30432/christmas_test \
  cargo test -p christmas --features server --test draw_integration -- --ignored --test-threads=1
```

They share one database and one runtime, hence `--test-threads=1`.

## The reveal

A full-screen ceremony: the roster, then the letter, then each giver → receiver
read out one beat at a time while the ring fills in, then the finished draw.
Space plays and pauses, arrow keys step.

It reads from the stored exchange rather than drawing live, so it can be paused,
rewound and replayed without ever changing the result, and any past draw can be
re-watched. It sits behind the viewer password like everything else, so the link
can be shared with the family rather than only screen-shared.

**Opening a pool plays it automatically the first time.** Until then that pool's
ring, pairings, and letter stay behind a *Not opened yet* card — the ceremony is
the point, so arriving at a wall of names would spoil it.

Whether someone has had their turn is remembered per browser in local storage,
keyed by exchange id. Sitting through the ceremony and skipping out both count:
either way the results come out and it stops playing uninvited. Once revealed,
the ring heading carries a **Replay the reveal** control.

Re-drawing mints a new exchange id, so a fresh draw is a fresh reveal for
everyone — including a corrective re-draw, which will re-gate the page for the
whole family.

Local storage is unavailable during server rendering, which is why the first
paint reveals nothing and the check runs on mount: otherwise a returning visitor
would flash the gate, and a new one would have the draw spoiled.

## Passwords

Two shared passwords, set as environment variables — there are no accounts:

| Variable | Who | Can |
| --- | --- | --- |
| `CHRISTMAS_VIEW_PASSWORD` | the family | read everything |
| `CHRISTMAS_ADMIN_PASSWORD` | whoever runs the draw | also reach Manage and change data |

With neither set, the site is wide open and boot logs a warning — which is what
makes `dx serve` frictionless locally.

Enforcement is a single axum middleware wrapping the whole router
(`src/auth.rs`), not a check inside each server function. That matters because
it **fails closed**: any `/api` path that isn't a known read-only endpoint
(`list_*`, `pool_detail`) requires the manager role, so a server function added
later is protected by default rather than by remembering to guard it.

Signing in is a plain HTML form posting to `/auth/login`, so no JavaScript is
involved and the session cookie is `HttpOnly` — page scripts cannot read it.
`Secure` is added automatically unless the host is localhost, so http still works
in development. Sessions are held in memory and 256 bits of OS randomness, and a
restart signs everyone out.

## Styling

One hand-written stylesheet, `assets/main.css`, loaded through `asset!`. There is
deliberately no Tailwind: the Docker build never runs a CSS toolchain, so any
utility class added to the markup would silently not exist in production.

## Deploying

See [DEPLOY.md](DEPLOY.md).
