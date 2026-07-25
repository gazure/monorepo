use dioxus::prelude::*;

// Only the types in the public signatures are imported here; the rest are
// pulled in inside the server-gated bodies so the wasm build stays clean.
use crate::model::{DrawConfig, Exchange};

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct ExchangeRow {
    id: i32,
    pool_id: i32,
    pool_name: String,
    pool_slug: String,
    year: i32,
    revision: i32,
    letter: Option<String>,
    participants: sqlx::types::Json<Vec<String>>,
    exclusions: sqlx::types::Json<Vec<crate::model::ExclusionSnapshot>>,
    pairings: sqlx::types::Json<Vec<crate::model::Pairing>>,
    config: sqlx::types::Json<serde_json::Value>,
    seed: Option<i64>,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct RelRow {
    a: String,
    b: String,
    kind: String,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct PriorRow {
    year: i32,
    pairings: sqlx::types::Json<Vec<crate::model::Pairing>>,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct PoolNameRow {
    name: String,
    slug: String,
}

/// Loads exchanges, newest first, optionally filtered to one pool.
#[cfg(feature = "server")]
pub(crate) async fn load_exchanges(db: &sqlx::PgPool, pool_id: Option<i32>) -> Result<Vec<Exchange>, ServerFnError> {
    let sql = r"
        SELECT e.id, e.pool_id, p.name AS pool_name, p.slug AS pool_slug,
               e.year, e.revision, e.letter,
               e.participants, e.exclusions, e.pairings, e.config, e.seed
        FROM exchange e
        JOIN pool p ON p.id = e.pool_id
        WHERE ($1::int IS NULL OR e.pool_id = $1)
        ORDER BY e.year DESC, e.revision DESC
    ";

    let rows: Vec<ExchangeRow> = sqlx::query_as(sql)
        .bind(pool_id)
        .fetch_all(db)
        .await
        .map_err(super::db_err)?;

    Ok(rows
        .into_iter()
        .map(|r| Exchange {
            id: r.id,
            pool_id: r.pool_id,
            pool_name: r.pool_name,
            pool_slug: r.pool_slug,
            year: r.year,
            revision: r.revision,
            letter: r.letter.and_then(|s| s.chars().next()),
            participants: r.participants.0,
            exclusions: r.exclusions.0,
            pairings: r.pairings.0,
            // Rows written before migration 003 carry `{}`; tolerate that.
            config: serde_json::from_value(r.config.0).ok(),
            seed: r.seed,
        })
        .collect())
}

/// Keeps only the live draw for each pool and year.
///
/// Rows arrive ordered by year then revision descending, so the first sighting
/// of a (pool, year) is the current one.
#[cfg(feature = "server")]
pub(crate) fn only_current_revisions(exchanges: Vec<Exchange>) -> Vec<Exchange> {
    let mut seen = std::collections::HashSet::new();
    exchanges
        .into_iter()
        .filter(|e| seen.insert((e.pool_id, e.year)))
        .collect()
}

/// Superseded revisions are dropped for anyone but a manager — filtered here
/// rather than in the UI, so they are never sent to the browser at all.
#[cfg(feature = "server")]
async fn visible_exchanges(db: &sqlx::PgPool, pool_id: Option<i32>) -> Result<Vec<Exchange>, ServerFnError> {
    let all = load_exchanges(db, pool_id).await?;
    if crate::auth::caller_role().await == crate::auth::Role::Manager {
        Ok(all)
    } else {
        Ok(only_current_revisions(all))
    }
}

#[server]
pub async fn list_exchanges(pool_id: Option<i32>) -> Result<Vec<Exchange>, ServerFnError> {
    let db = crate::pool().await?;
    visible_exchanges(db, pool_id).await
}

/// A single recorded draw.
///
/// Named so the auth middleware's viewer allowlist can opt it in explicitly —
/// see `auth::required_access`.
#[server]
pub async fn exchange_detail(id: i32) -> Result<Exchange, ServerFnError> {
    let db = crate::pool().await?;
    // Goes through the same filter, so a viewer cannot reach a superseded draw
    // by guessing its id.
    visible_exchanges(db, None)
        .await?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| ServerFnError::new(format!("No draw with id {id}")))
}

/// The current (highest-revision) draw for each pool in a given year.
#[server]
pub async fn list_year(year: i32) -> Result<Vec<Exchange>, ServerFnError> {
    let db = crate::pool().await?;
    let all = load_exchanges(db, None).await?;

    let mut seen = std::collections::HashSet::new();
    Ok(all
        .into_iter()
        .filter(|e| e.year == year)
        // Already ordered by revision DESC, so the first per pool is current.
        .filter(|e| seen.insert(e.pool_id))
        .collect())
}

/// Names of everyone in the pool.
#[cfg(feature = "server")]
async fn pool_members(db: &sqlx::PgPool, pool_id: i32) -> Result<Vec<String>, ServerFnError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r"SELECT p.name
          FROM participant p
          JOIN pool_membership m ON m.participant_id = p.id
          WHERE m.pool_id = $1
          ORDER BY p.name",
    )
    .bind(pool_id)
    .fetch_all(db)
    .await
    .map_err(super::db_err)?;

    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Turns stored relationships into forbidden edges, honouring the spouse toggle.
///
/// Returns the edges plus the snapshot rows archived alongside the draw.
#[cfg(feature = "server")]
async fn relationship_edges(
    db: &sqlx::PgPool,
    exclude_spouses: bool,
) -> Result<(Vec<crate::matching::BlockedEdge>, Vec<crate::model::ExclusionSnapshot>), ServerFnError> {
    use crate::{
        matching::{BlockReason, BlockedEdge},
        model::{ExclusionSnapshot, RelationshipKind},
    };

    let rels: Vec<RelRow> = sqlx::query_as(
        r"SELECT pa.name AS a, pb.name AS b, e.kind::text AS kind
          FROM exclusion e
          JOIN participant pa ON e.participant_a_id = pa.id
          JOIN participant pb ON e.participant_b_id = pb.id",
    )
    .fetch_all(db)
    .await
    .map_err(super::db_err)?;

    let mut edges = Vec::new();
    let mut snapshots = Vec::new();

    for rel in rels {
        let kind = RelationshipKind::from_db(&rel.kind);
        // Partnerships are only enforced when the toggle is on; manual
        // exclusions always apply.
        if kind.is_partnership() && !exclude_spouses {
            continue;
        }
        let reason = match kind {
            RelationshipKind::Spouse => BlockReason::Spouse,
            RelationshipKind::Household => BlockReason::Household,
            RelationshipKind::Manual => BlockReason::Manual,
        };
        // Symmetric: block both directions.
        edges.push(BlockedEdge {
            giver: rel.a.clone(),
            receiver: rel.b.clone(),
            reason,
        });
        edges.push(BlockedEdge {
            giver: rel.b.clone(),
            receiver: rel.a.clone(),
            reason,
        });
        snapshots.push(ExclusionSnapshot {
            a: rel.a,
            b: rel.b,
            reason: Some(kind.label().to_string()),
        });
    }

    Ok((edges, snapshots))
}

/// Edges banned because the same pairing happened in a recent year.
#[cfg(feature = "server")]
async fn repeat_edges(
    db: &sqlx::PgPool,
    pool_id: i32,
    year: i32,
    lookback: u32,
) -> Result<Vec<crate::matching::BlockedEdge>, ServerFnError> {
    use crate::matching::{BlockReason, BlockedEdge};

    let earliest = year - i32::try_from(lookback).unwrap_or(1);
    let priors: Vec<PriorRow> = sqlx::query_as(
        r"SELECT DISTINCT ON (year) year, pairings
          FROM exchange
          WHERE pool_id = $1 AND year < $2 AND year >= $3
          ORDER BY year DESC, revision DESC",
    )
    .bind(pool_id)
    .bind(year)
    .bind(earliest)
    .fetch_all(db)
    .await
    .map_err(super::db_err)?;

    Ok(priors
        .into_iter()
        .flat_map(|prior| {
            let year = prior.year;
            prior.pairings.0.into_iter().map(move |p| BlockedEdge {
                giver: p.giver,
                receiver: p.receiver,
                reason: BlockReason::RepeatOf { year },
            })
        })
        .collect())
}

/// Picks the letter, skipping the pool's excluded set and anything it has used.
#[cfg(feature = "server")]
async fn pick_letter(db: &sqlx::PgPool, pool_id: i32, seed: u64) -> Result<Option<char>, ServerFnError> {
    let excluded: Vec<(String,)> = sqlx::query_as("SELECT letter FROM excluded_letter WHERE pool_id = $1")
        .bind(pool_id)
        .fetch_all(db)
        .await
        .map_err(super::db_err)?;
    let used: Vec<(String,)> =
        sqlx::query_as("SELECT DISTINCT letter FROM exchange WHERE pool_id = $1 AND letter IS NOT NULL")
            .bind(pool_id)
            .fetch_all(db)
            .await
            .map_err(super::db_err)?;

    let excluded: Vec<char> = excluded.into_iter().filter_map(|r| r.0.chars().next()).collect();
    let used: Vec<char> = used.into_iter().filter_map(|r| r.0.chars().next()).collect();

    Ok(crate::matching::select_letter(&excluded, &used, seed))
}

/// Runs a draw for one pool and records it as a new revision.
///
/// Never overwrites a previous draw — re-running produces revision N+1, so
/// history is preserved and auditable.
#[server]
pub async fn run_draw(
    pool_id: i32,
    year: i32,
    config: DrawConfig,
    include_letter: bool,
) -> Result<Exchange, ServerFnError> {
    use sqlx::types::Json;

    use crate::{matching::build_draw, model::Pairing};

    let db = crate::pool().await?;

    // A zero seed means "pick one for me"; a real seed replays an earlier draw.
    let mut config = config;
    if config.seed == 0 {
        config.seed = fastrand::u64(1..u64::MAX);
    }

    let participants = pool_members(db, pool_id).await?;
    if participants.len() < 3 {
        return Err(ServerFnError::new("A pool needs at least 3 members before it can draw"));
    }

    let (mut blocked, exclusion_snapshots) = relationship_edges(db, config.exclude_spouses).await?;
    if let Some(lookback) = config.avoid_repeat_years {
        blocked.extend(repeat_edges(db, pool_id, year, lookback).await?);
    }

    let draw = build_draw(&participants, &blocked, &config).map_err(|e| ServerFnError::new(e.to_string()))?;

    let letter = if include_letter {
        pick_letter(db, pool_id, config.seed).await?
    } else {
        None
    };

    let pairings: Vec<Pairing> = draw
        .pairings
        .into_iter()
        .map(|(giver, receiver)| Pairing { giver, receiver })
        .collect();

    // Postgres has no unsigned 64-bit type; round-trip the bits so the seed
    // survives storage exactly.
    let seed_i64 = i64::from_ne_bytes(config.seed.to_ne_bytes());

    let row: (i32, i32) = sqlx::query_as(
        r"INSERT INTO exchange (pool_id, year, revision, letter, participants, exclusions, pairings, config, seed)
          VALUES ($1, $2,
                  (SELECT COALESCE(MAX(revision), 0) + 1 FROM exchange WHERE pool_id = $1 AND year = $2),
                  $3, $4, $5, $6, $7, $8)
          RETURNING id, revision",
    )
    .bind(pool_id)
    .bind(year)
    .bind(letter.map(|c| c.to_string()))
    .bind(Json(&participants))
    .bind(Json(&exclusion_snapshots))
    .bind(Json(&pairings))
    .bind(Json(&config))
    .bind(seed_i64)
    .fetch_one(db)
    .await
    .map_err(super::db_err)?;

    let pool_row: PoolNameRow = sqlx::query_as("SELECT name, slug FROM pool WHERE id = $1")
        .bind(pool_id)
        .fetch_one(db)
        .await
        .map_err(super::db_err)?;

    tracingx::info!(
        pool = %pool_row.name,
        year,
        revision = row.1,
        cycles = draw.cycles.len(),
        "draw recorded"
    );

    Ok(Exchange {
        id: row.0,
        pool_id,
        pool_name: pool_row.name,
        pool_slug: pool_row.slug,
        year,
        revision: row.1,
        letter,
        participants,
        exclusions: exclusion_snapshots,
        pairings,
        config: Some(config),
        seed: Some(seed_i64),
    })
}

/// Years a hand-entered draw is allowed to claim. Wide enough for any real
/// family history, narrow enough to catch a mistyped year.
const PLAUSIBLE_YEARS: std::ops::RangeInclusive<i32> = 1900..=2200;

/// Checks that a hand-entered draw is shaped like a draw.
///
/// A recorded draw is a partial permutation: each person gives once, receives
/// once, and never to themselves. Leaving someone out is allowed — old paper
/// records are often incomplete — but contradicting yourself is not.
///
/// Kept out of the server function so the rules can be tested without a
/// database, and returns the offending name so the message is actionable.
pub(crate) fn validate_backfill(pairings: &[crate::model::Pairing]) -> Result<(), String> {
    use std::collections::HashSet;

    if pairings.is_empty() {
        return Err("Fill in at least one giver before saving".to_string());
    }

    let mut givers = HashSet::new();
    let mut receivers = HashSet::new();

    for pair in pairings {
        let (giver, receiver) = (pair.giver.trim(), pair.receiver.trim());

        if giver.is_empty() || receiver.is_empty() {
            return Err("Every entry needs both a giver and a receiver".to_string());
        }
        if giver == receiver {
            return Err(format!("{giver} cannot give to themselves"));
        }
        if !givers.insert(giver) {
            return Err(format!("{giver} appears twice as a giver"));
        }
        if !receivers.insert(receiver) {
            return Err(format!("Two people are down as giving to {receiver}"));
        }
    }

    Ok(())
}

/// Everyone named in a draw, deduplicated and ordered, for the row's snapshot.
///
/// Taken from the pairings rather than the pool's current membership: whoever
/// took part in 2019 is a fact about 2019, not about who is in the pool now.
pub(crate) fn backfill_participants(pairings: &[crate::model::Pairing]) -> Vec<String> {
    let mut names: Vec<String> = pairings
        .iter()
        .flat_map(|p| [p.giver.trim().to_string(), p.receiver.trim().to_string()])
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Records a draw that happened outside the app — an earlier year still on paper.
///
/// Goes in through the same revision counter as [`run_draw`], so a backfill
/// never overwrites anything, and comes back out of the same queries. That last
/// part is the point: once a past year is on the record, "no repeat receivers"
/// can see it, and its letter is off the table for future draws.
///
/// `config` and `seed` are deliberately left at their defaults. We do not know
/// what rules an old draw ran under, and inventing them would make the pool
/// page's "How it was drawn" line say something untrue.
#[server]
pub async fn record_past_draw(
    pool_id: i32,
    year: i32,
    pairings: Vec<crate::model::Pairing>,
    letter: Option<char>,
) -> Result<Exchange, ServerFnError> {
    use sqlx::types::Json;

    use crate::model::Pairing;

    if !PLAUSIBLE_YEARS.contains(&year) {
        return Err(ServerFnError::new(format!(
            "{year} doesn't look like a year — expected {} to {}",
            PLAUSIBLE_YEARS.start(),
            PLAUSIBLE_YEARS.end()
        )));
    }

    validate_backfill(&pairings).map_err(ServerFnError::new)?;

    // Uppercased here rather than trusted: the column's CHECK only accepts
    // `[A-Z]`, and a constraint violation is a worse message than this one.
    let letter = match letter {
        Some(c) if c.is_ascii_alphabetic() => Some(c.to_ascii_uppercase()),
        Some(c) => return Err(ServerFnError::new(format!("'{c}' is not a letter"))),
        None => None,
    };

    let participants = backfill_participants(&pairings);
    let pairings: Vec<Pairing> = pairings
        .into_iter()
        .map(|p| Pairing {
            giver: p.giver.trim().to_string(),
            receiver: p.receiver.trim().to_string(),
        })
        .collect();

    let db = crate::pool().await?;

    let row: (i32, i32) = sqlx::query_as(
        r"INSERT INTO exchange (pool_id, year, revision, letter, participants, exclusions, pairings)
          VALUES ($1, $2,
                  (SELECT COALESCE(MAX(revision), 0) + 1 FROM exchange WHERE pool_id = $1 AND year = $2),
                  $3, $4, '[]'::jsonb, $5)
          RETURNING id, revision",
    )
    .bind(pool_id)
    .bind(year)
    .bind(letter.map(|c| c.to_string()))
    .bind(Json(&participants))
    .bind(Json(&pairings))
    .fetch_one(db)
    .await
    .map_err(super::db_err)?;

    let pool_row: PoolNameRow = sqlx::query_as("SELECT name, slug FROM pool WHERE id = $1")
        .bind(pool_id)
        .fetch_one(db)
        .await
        .map_err(super::db_err)?;

    tracingx::info!(
        pool = %pool_row.name,
        year,
        revision = row.1,
        pairings = pairings.len(),
        "past draw backfilled"
    );

    Ok(Exchange {
        id: row.0,
        pool_id,
        pool_name: pool_row.name,
        pool_slug: pool_row.slug,
        year,
        revision: row.1,
        letter,
        participants,
        exclusions: vec![],
        pairings,
        config: None,
        seed: None,
    })
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;
    use crate::model::Exchange;

    fn exchange(id: i32, pool_id: i32, year: i32, revision: i32) -> Exchange {
        Exchange {
            id,
            pool_id,
            pool_name: "Pool".into(),
            pool_slug: "pool".into(),
            year,
            revision,
            letter: None,
            participants: vec![],
            exclusions: vec![],
            pairings: vec![],
            config: None,
            seed: None,
        }
    }

    #[test]
    fn keeps_the_live_draw_for_each_pool_and_year() {
        // As the query returns them: year then revision, both descending.
        let rows = vec![
            exchange(9, 1, 2026, 3),
            exchange(5, 1, 2026, 2),
            exchange(2, 1, 2026, 1),
            exchange(8, 2, 2026, 1),
            exchange(1, 1, 2025, 1),
        ];

        let kept = only_current_revisions(rows);
        let ids: Vec<i32> = kept.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![9, 8, 1], "one row per pool and year, highest revision");
    }

    #[test]
    fn a_single_revision_survives_untouched() {
        let rows = vec![exchange(1, 1, 2026, 1)];
        assert_eq!(only_current_revisions(rows).len(), 1);
    }

    #[test]
    fn pools_do_not_shadow_each_other() {
        let rows = vec![
            exchange(3, 1, 2026, 1),
            exchange(4, 2, 2026, 1),
            exchange(5, 3, 2026, 1),
        ];
        assert_eq!(only_current_revisions(rows).len(), 3);
    }

    fn pairs(entries: &[(&str, &str)]) -> Vec<crate::model::Pairing> {
        entries
            .iter()
            .map(|(giver, receiver)| crate::model::Pairing {
                giver: (*giver).to_string(),
                receiver: (*receiver).to_string(),
            })
            .collect()
    }

    #[test]
    fn a_well_formed_backfill_is_accepted() {
        let ring = pairs(&[("Anne", "Claire"), ("Claire", "Noel"), ("Noel", "Anne")]);
        assert!(validate_backfill(&ring).is_ok());
    }

    #[test]
    fn a_backfill_may_be_incomplete() {
        // Old paper records often are: two entries that go nowhere in particular.
        let partial = pairs(&[("Anne", "Claire"), ("Noel", "Grant")]);
        assert!(
            validate_backfill(&partial).is_ok(),
            "a partial record is still worth having"
        );
    }

    #[test]
    fn nobody_gives_to_themselves() {
        let err = validate_backfill(&pairs(&[("Anne", "Anne")])).unwrap_err();
        assert!(err.contains("Anne"), "the message should name the offender: {err}");
    }

    #[test]
    fn a_giver_cannot_appear_twice() {
        let err = validate_backfill(&pairs(&[("Anne", "Claire"), ("Anne", "Noel")])).unwrap_err();
        assert!(err.contains("Anne"));
    }

    #[test]
    fn two_people_cannot_give_to_the_same_person() {
        let err = validate_backfill(&pairs(&[("Anne", "Grant"), ("Noel", "Grant")])).unwrap_err();
        assert!(err.contains("Grant"));
    }

    #[test]
    fn an_empty_backfill_is_rejected() {
        assert!(validate_backfill(&[]).is_err());
    }

    #[test]
    fn blank_names_are_rejected() {
        assert!(validate_backfill(&pairs(&[("Anne", "   ")])).is_err());
        assert!(validate_backfill(&pairs(&[("", "Anne")])).is_err());
    }

    #[test]
    fn participants_are_everyone_named_once_each() {
        let ring = pairs(&[("Noel", "Anne"), ("Anne", "Claire"), ("Claire", "Noel")]);
        assert_eq!(backfill_participants(&ring), vec!["Anne", "Claire", "Noel"]);
    }

    #[test]
    fn participants_include_receivers_who_never_gave() {
        // A one-sided record still tells us Grant was there.
        let partial = pairs(&[("Anne", "Grant")]);
        assert_eq!(backfill_participants(&partial), vec!["Anne", "Grant"]);
    }
}
