//! Live smoke test against ESPN. Ignored by default: it needs network access and
//! asserts on data the league can legitimately change.
//!
//! Run with: `cargo test -p slate --test live -- --ignored --nocapture`

use chrono::Utc;
use postgresql_embedded::{PostgreSQL, Settings};
use slate::{espn, ics, store::Store};

#[tokio::test]
#[ignore = "requires network access to ESPN"]
async fn fetches_and_renders_a_real_season() {
    let settings = Settings {
        password: "password".to_string(),
        ..Default::default()
    };
    let mut pg = PostgreSQL::new(settings);
    pg.setup().await.expect("setup embedded postgres");
    pg.start().await.expect("start embedded postgres");
    pg.create_database("slate").await.expect("create slate database");
    let port = pg.settings().port;

    let store = Store::connect(&format!("postgresql://postgres:password@localhost:{port}/slate"))
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");

    let league = espn::league("nfl").expect("nfl is a known league");
    let client = espn::Client::new();
    let schedule = client
        .team_schedule(league, "sea", 2026)
        .await
        .expect("fetch seahawks 2026");

    assert_eq!(schedule.games.len(), 17, "an NFL team plays 17 games");
    assert_eq!(schedule.bye_week, Some(11));

    let report = store.sync(&schedule.games).await.expect("sync");
    assert_eq!(report.inserted, 17);

    // A second pass over identical upstream data must record nothing.
    let report = store.sync(&schedule.games).await.expect("resync");
    assert_eq!(report.unchanged, 17);
    assert_eq!(report.changed.len(), 0);

    let feed = store.feed("nfl", "sea").await.expect("feed");
    assert_eq!(feed.len(), 17);

    let calendar = ics::render("nfl", "SEA", &feed, schedule.bye_week, Utc::now());

    // Week 18 has no kickoff time yet, so it must render as an all-day entry.
    let unfolded = calendar.replace("\r\n ", "");
    assert!(unfolded.contains("(time TBD)"), "week 18 should be marked TBD");
    assert!(unfolded.contains("Seahawks bye (week 11)"), "bye should be present");
    for line in calendar.split("\r\n") {
        assert!(line.len() <= 75, "unfolded line: {line}");
    }

    std::fs::write("/tmp/slate-live.ics", &calendar).expect("write output");
    println!("wrote /tmp/slate-live.ics ({} bytes)", calendar.len());
}
