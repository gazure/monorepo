//! End-to-end tests for the draw path against a real `PostgreSQL`.
//!
//! Ignored by default because they need a database. Run them with:
//!
//! ```sh
//! CHRISTMAS_DATABASE_URL=postgresql://postgres:postgres@localhost:30432/christmas_test \
//!   cargo test -p christmas --features server --test draw_integration -- --ignored --test-threads=1
//! ```
//!
//! These share one runtime for the whole binary. `#[tokio::test]` builds a fresh
//! runtime per test, and the connection pool is a process-global `OnceCell` — so
//! once the first test's runtime is dropped, every pooled connection's I/O driver
//! dies and later acquires time out. They also share one database, hence
//! `--test-threads=1`.
#![cfg(feature = "server")]

use christmas::{
    model::{CycleMode, DrawConfig, RelationshipKind},
    server,
};

/// One runtime for the whole binary — see the module docs.
fn rt() -> &'static tokio::runtime::Runtime {
    static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build test runtime")
    })
}

/// Wipes and reseeds, so each test starts from the same known state.
async fn reset() {
    let pool = christmas::pool().await.expect("database should be reachable");
    sqlx::query(
        "TRUNCATE exchange, pool_membership, excluded_letter, exclusion, pool, participant RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await
    .expect("truncate");

    let seed: christmas::seed::SeedFile =
        serde_json::from_str(include_str!("../seed/family.json")).expect("seed file should parse");
    christmas::seed::apply(pool, &seed).await.expect("seed");
}

async fn pool_id_for(slug: &str) -> i32 {
    server::list_pools()
        .await
        .expect("list pools")
        .into_iter()
        .find(|p| p.slug == slug)
        .unwrap_or_else(|| panic!("no pool {slug}"))
        .id
}

fn config(mode: CycleMode) -> DrawConfig {
    DrawConfig {
        cycle_mode: mode,
        exclude_spouses: true,
        avoid_repeat_years: Some(1),
        seed: 0,
    }
}

#[test]
#[ignore = "needs a database"]
fn seed_file_produces_the_expected_shape() {
    rt().block_on(async {
        reset().await;

        let pools = server::list_pools().await.expect("list pools");
        assert_eq!(pools.len(), 3);

        let by_slug = |slug: &str| pools.iter().find(|p| p.slug == slug).unwrap().member_count;
        assert_eq!(by_slug("island-life"), 10);
        assert_eq!(by_slug("grabergishimazureson"), 14);
        assert_eq!(by_slug("pets"), 10);

        let rels = server::list_relationships().await.expect("list relationships");
        assert_eq!(rels.len(), 5);
        assert!(rels.iter().all(|r| r.kind == RelationshipKind::Spouse));

        // Grabergishimazureson draws from "ACDIJLMNORSTUXYZ" — 16 allowed, 10 excluded.
        let graber = pool_id_for("grabergishimazureson").await;
        let letters = server::list_excluded_letters(graber).await.expect("letters");
        assert_eq!(letters.len(), 10);

        // The admin page reads them all in one go; it must agree per pool.
        let all = server::list_all_excluded_letters().await.expect("all letters");
        let for_graber: Vec<&(i32, String)> = all.iter().filter(|(id, _)| *id == graber).collect();
        assert_eq!(for_graber.len(), 10);
        assert_eq!(
            all.len(),
            10,
            "only Grabergishimazureson restricts its letters in the seed"
        );
    });
}

#[test]
#[ignore = "needs a database"]
fn grand_draw_keeps_spouses_apart() {
    rt().block_on(async {
        reset().await;
        let pool_id = pool_id_for("grabergishimazureson").await;

        let exchange = server::run_draw(pool_id, 2026, config(CycleMode::Grand), true)
            .await
            .expect("draw should succeed");

        assert_eq!(exchange.pairings.len(), 14);
        assert_eq!(exchange.revision, 1);
        assert_eq!(exchange.cycles().len(), 1, "grand mode is a single ring");

        let couples = [
            ("Claire", "Duncan"),
            ("Anne", "Eric"),
            ("Noel", "K-Lee"),
            ("Steve", "Linda"),
            ("Jim", "Kari"),
        ];
        for (a, b) in couples {
            for p in &exchange.pairings {
                assert!(
                    !(p.giver == a && p.receiver == b || p.giver == b && p.receiver == a),
                    "{a} and {b} are married but were paired"
                );
            }
        }

        // The letter must come from the pool's permitted set.
        let allowed: Vec<char> = "ACDIJLMNORSTUXYZ".chars().collect();
        assert!(allowed.contains(&exchange.letter.expect("a letter was requested")));
    });
}

#[test]
#[ignore = "needs a database"]
fn redrawing_adds_a_revision_instead_of_overwriting() {
    rt().block_on(async {
        reset().await;
        let pool_id = pool_id_for("pets").await;

        let first = server::run_draw(pool_id, 2026, config(CycleMode::Grand), true)
            .await
            .expect("first draw");
        let second = server::run_draw(pool_id, 2026, config(CycleMode::Grand), true)
            .await
            .expect("second draw");

        assert_eq!(first.revision, 1);
        assert_eq!(second.revision, 2);
        assert_ne!(first.id, second.id, "the original draw must survive");

        let all = server::list_exchanges(Some(pool_id)).await.expect("list");
        assert_eq!(all.len(), 2, "both revisions are retained");

        // The year view shows only the current revision.
        let current = server::list_year(2026).await.expect("year");
        let pets: Vec<_> = current.iter().filter(|e| e.pool_id == pool_id).collect();
        assert_eq!(pets.len(), 1);
        assert_eq!(pets[0].revision, 2);
    });
}

#[test]
#[ignore = "needs a database"]
fn no_repeat_receivers_is_enforced_against_last_year() {
    rt().block_on(async {
        reset().await;
        let pool_id = pool_id_for("pets").await;

        let last_year = server::run_draw(pool_id, 2025, config(CycleMode::Grand), true)
            .await
            .expect("2025 draw");
        let this_year = server::run_draw(pool_id, 2026, config(CycleMode::Grand), true)
            .await
            .expect("2026 draw");

        for prev in &last_year.pairings {
            for now in &this_year.pairings {
                assert!(
                    !(prev.giver == now.giver && prev.receiver == now.receiver),
                    "{} gave to {} in both years",
                    prev.giver,
                    prev.receiver
                );
            }
        }

        // Letters should differ too, since previously used ones are skipped.
        assert_ne!(last_year.letter, this_year.letter);
    });
}

#[test]
#[ignore = "needs a database"]
fn multiple_mode_produces_rings_of_at_least_three() {
    rt().block_on(async {
        reset().await;
        let pool_id = pool_id_for("grabergishimazureson").await;

        let exchange = server::run_draw(pool_id, 2026, config(CycleMode::Multiple { min_len: 3 }), true)
            .await
            .expect("draw");

        let cycles = exchange.cycles();
        assert!(cycles.iter().all(|c| c.len() >= 3), "no ring shorter than three");
        let total: usize = cycles.iter().map(Vec::len).sum();
        assert_eq!(total, 14, "everyone appears exactly once");

        // No reciprocal pairs.
        for a in &exchange.pairings {
            for b in &exchange.pairings {
                assert!(
                    !(a.giver == b.receiver && a.receiver == b.giver),
                    "{} and {} were paired reciprocally",
                    a.giver,
                    a.receiver
                );
            }
        }
    });
}

#[test]
#[ignore = "needs a database"]
fn draw_config_and_seed_are_recorded() {
    rt().block_on(async {
        reset().await;
        let pool_id = pool_id_for("island-life").await;

        let exchange = server::run_draw(pool_id, 2026, config(CycleMode::Grand), true)
            .await
            .expect("draw");

        let stored = server::list_exchanges(Some(pool_id)).await.expect("list");
        let stored = stored.first().expect("one exchange");

        assert!(stored.seed.is_some(), "seed must be stored for replay");
        assert_eq!(stored.seed, exchange.seed);

        let config = stored.config.as_ref().expect("config snapshot");
        assert_eq!(config.cycle_mode, CycleMode::Grand);
        assert!(config.exclude_spouses);
        assert_eq!(config.avoid_repeat_years, Some(1));
    });
}

#[test]
#[ignore = "needs a database"]
fn a_pool_that_cannot_draw_reports_why() {
    rt().block_on(async {
        reset().await;

        // Three people where two are married leaves nobody a legal grand cycle.
        let pool = server::create_pool("Tiny".to_string(), None).await.expect("pool");
        let mut ids = Vec::new();
        for name in ["Ada", "Bo", "Cy"] {
            let p = server::add_participant(name.to_string(), vec![pool.id])
                .await
                .expect("participant");
            ids.push(p.id);
        }
        server::add_relationship(ids[0], ids[1], RelationshipKind::Spouse, None)
            .await
            .expect("relationship");

        let err = server::run_draw(pool.id, 2026, config(CycleMode::Grand), true)
            .await
            .expect_err("should fail");

        let message = err.to_string();
        assert!(
            message.contains("Ada") || message.contains("Bo") || message.contains("no valid arrangement"),
            "failure should explain itself, got: {message}"
        );
    });
}

#[test]
#[ignore = "needs a database"]
fn a_swap_is_recorded_as_a_new_revision() {
    rt().block_on(async {
        reset().await;
        let pool_id = pool_id_for("grabergishimazureson").await;

        let original = server::run_draw(pool_id, 2026, config(CycleMode::Grand), true)
            .await
            .expect("draw");

        // Two people who are not each other's giver or receiver, so the swap is
        // the four-edge case rather than the adjacent three-edge one.
        let ring = original.cycles().into_iter().next().expect("one grand ring");
        let (a, b) = (ring[0].clone(), ring[5].clone());

        let preview = server::preview_swap(original.id, a.clone(), b.clone())
            .await
            .expect("preview");
        assert_eq!(preview.changes.len(), 4, "a non-adjacent swap moves four pairings");

        let adjusted = server::apply_swap(original.id, a.clone(), b.clone(), false)
            .await
            .expect("swap should apply");

        assert_eq!(adjusted.revision, original.revision + 1);
        assert_ne!(adjusted.id, original.id, "the original draw must survive");
        assert_eq!(adjusted.adjusted_from, Some(original.id));
        assert_eq!(
            adjusted.adjustment_note.as_deref(),
            Some(format!("Swapped {a} and {b}").as_str())
        );
        assert!(adjusted.was_adjusted());

        // A hand-edited draw cannot be replayed, so it carries no seed.
        assert!(adjusted.seed.is_none(), "an adjusted draw must not claim a seed");
        assert!(adjusted.config.is_some(), "the rules it ran under still apply");

        // Everyone still gives and receives exactly once, in one ring.
        assert_eq!(adjusted.pairings.len(), 14);
        assert_eq!(adjusted.cycles().len(), 1);
        assert_eq!(adjusted.letter, original.letter);
        assert_eq!(adjusted.participants, original.participants);

        // The two named people really did trade places.
        let receiver_of = |draw: &christmas::model::Exchange, giver: &str| {
            draw.pairings
                .iter()
                .find(|p| p.giver == giver)
                .map(|p| p.receiver.clone())
                .expect("every giver has a receiver")
        };
        let giver_to = |draw: &christmas::model::Exchange, receiver: &str| {
            draw.pairings
                .iter()
                .find(|p| p.receiver == receiver)
                .map(|p| p.giver.clone())
                .expect("everyone receives")
        };
        assert_eq!(receiver_of(&adjusted, &a), receiver_of(&original, &b));
        assert_eq!(receiver_of(&adjusted, &b), receiver_of(&original, &a));
        assert_eq!(giver_to(&adjusted, &a), giver_to(&original, &b));
        assert_eq!(giver_to(&adjusted, &b), giver_to(&original, &a));

        // The live draw for the year is now the adjusted one.
        let current = server::list_year(2026).await.expect("year");
        let live = current.iter().find(|e| e.pool_id == pool_id).expect("a live draw");
        assert_eq!(live.id, adjusted.id);
    });
}

#[test]
#[ignore = "needs a database"]
fn a_swap_that_pairs_spouses_needs_confirming() {
    rt().block_on(async {
        reset().await;
        let pool_id = pool_id_for("grabergishimazureson").await;

        let draw = server::run_draw(pool_id, 2026, config(CycleMode::Grand), true)
            .await
            .expect("draw");

        // Claire and Duncan are married. Putting Duncan where Claire's receiver
        // is makes Claire give to him, which the draw forbade.
        let claire_gives_to = draw
            .pairings
            .iter()
            .find(|p| p.giver == "Claire")
            .expect("Claire is in the pool")
            .receiver
            .clone();

        let preview = server::preview_swap(draw.id, claire_gives_to.clone(), "Duncan".to_string())
            .await
            .expect("preview");
        assert!(
            preview
                .violations
                .iter()
                .any(|v| v.giver == "Claire" && v.receiver == "Duncan"),
            "the spouse rule should be flagged: {:?}",
            preview.violations
        );

        let refused = server::apply_swap(draw.id, claire_gives_to.clone(), "Duncan".to_string(), false).await;
        assert!(refused.is_err(), "an unconfirmed rule-breaking swap must be refused");

        let forced = server::apply_swap(draw.id, claire_gives_to, "Duncan".to_string(), true)
            .await
            .expect("confirming should let it through");
        assert_eq!(forced.revision, 2);
    });
}

#[test]
#[ignore = "needs a database"]
fn only_the_live_revision_can_be_adjusted() {
    rt().block_on(async {
        reset().await;
        let pool_id = pool_id_for("pets").await;

        let first = server::run_draw(pool_id, 2026, config(CycleMode::Grand), true)
            .await
            .expect("first draw");
        let second = server::run_draw(pool_id, 2026, config(CycleMode::Grand), true)
            .await
            .expect("second draw");

        let ring = second.cycles().into_iter().next().expect("one ring");
        let (a, b) = (ring[0].clone(), ring[2].clone());

        // Adjusting the superseded revision would fork the year's history.
        let stale = server::apply_swap(first.id, a.clone(), b.clone(), false).await;
        assert!(stale.is_err(), "a superseded revision must not be adjustable");

        server::apply_swap(second.id, a, b, false)
            .await
            .expect("the live revision adjusts fine");
    });
}
