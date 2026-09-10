//! End-to-end proof of the reconciliation loop against a real PostgreSQL.
//!
//! The point of slate is that a moved game updates in place in an already
//! subscribed calendar. That depends on two behaviours that are easy to get
//! wrong and impossible to verify without a database: a change to a
//! calendar-significant field must advance `SEQUENCE`, and a change to anything
//! else must not.

use chrono::{DateTime, TimeZone, Utc};
use postgresql_embedded::{PostgreSQL, Settings};
use slate::{
    model::{Game, GameStatus},
    store::Store,
};

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("valid timestamp")
}

fn game() -> Game {
    Game {
        league: "nfl".into(),
        espn_id: "401872999".into(),
        season: 2026,
        week: Some(18),
        name: "Seattle Seahawks at Los Angeles Rams".into(),
        short_name: "SEA @ LAR".into(),
        home_abbr: "LAR".into(),
        home_name: "Los Angeles Rams".into(),
        away_abbr: "SEA".into(),
        away_name: "Seattle Seahawks".into(),
        kickoff: at(2027, 1, 10, 5, 0),
        time_tbd: true,
        venue: Some("SoFi Stadium".into()),
        city: Some("Inglewood, CA".into()),
        status: GameStatus::Scheduled,
        broadcast: None,
    }
}

async fn boot() -> (PostgreSQL, Store) {
    let settings = Settings {
        password: "password".to_string(),
        ..Default::default()
    };
    let mut pg = PostgreSQL::new(settings);
    pg.setup().await.expect("setup embedded postgres");
    pg.start().await.expect("start embedded postgres");
    pg.create_database("slate").await.expect("create slate database");

    let port = pg.settings().port;
    let url = format!("postgresql://postgres:password@localhost:{port}/slate");
    let store = Store::connect(&url).await.expect("connect");
    store.migrate().await.expect("migrate");
    (pg, store)
}

async fn sequence_of(store: &Store) -> i32 {
    let feed = store.feed("nfl", "sea").await.expect("feed");
    assert_eq!(feed.len(), 1, "expected exactly one stored game");
    feed[0].sequence
}

#[tokio::test]
async fn reschedules_bump_sequence_but_broadcast_changes_do_not() {
    let (_pg, store) = boot().await;

    // First sight of the game.
    let report = store.sync(&[game()]).await.expect("initial sync");
    assert_eq!(report.inserted, 1);
    assert_eq!(report.changed.len(), 0);
    assert_eq!(sequence_of(&store).await, 0);

    // Re-polling unchanged data must be a no-op, or every poll would notify.
    let report = store.sync(&[game()]).await.expect("idempotent sync");
    assert_eq!(report.inserted, 0);
    assert_eq!(report.unchanged, 1);
    assert_eq!(report.changed.len(), 0);
    assert_eq!(sequence_of(&store).await, 0);

    // The league sets the Week 18 kickoff: TBD resolves to a real time.
    let mut scheduled = game();
    scheduled.time_tbd = false;
    scheduled.kickoff = at(2027, 1, 10, 21, 25);
    scheduled.broadcast = Some("FOX".into());

    let report = store.sync(&[scheduled.clone()]).await.expect("reschedule sync");
    assert_eq!(report.changed.len(), 1);
    assert_eq!(report.significant(), 1);
    let fields: Vec<&str> = report.changed[0].1.iter().map(|c| c.field).collect();
    assert!(fields.contains(&"kickoff"), "{fields:?}");
    assert!(fields.contains(&"time_tbd"), "{fields:?}");
    assert!(fields.contains(&"broadcast"), "{fields:?}");
    assert_eq!(sequence_of(&store).await, 1, "a reschedule must advance SEQUENCE");

    // A broadcast swap alone is recorded but must not re-notify subscribers.
    let mut tv_only = scheduled.clone();
    tv_only.broadcast = Some("NBC".into());
    let report = store.sync(&[tv_only]).await.expect("broadcast sync");
    assert_eq!(report.changed.len(), 1);
    assert_eq!(report.significant(), 0);
    assert_eq!(sequence_of(&store).await, 1, "a TV change must not advance SEQUENCE");

    // A venue move is significant.
    let mut moved = scheduled.clone();
    moved.broadcast = Some("NBC".into());
    moved.venue = Some("Empower Field at Mile High".into());
    let report = store.sync(&[moved]).await.expect("venue sync");
    assert_eq!(report.significant(), 1);
    assert_eq!(sequence_of(&store).await, 2);

    // Every observed change is retained, significant or not.
    let changes = store.recent_changes(50).await.expect("changes");
    let kickoff_moves = changes.iter().filter(|c| c.field == "kickoff").count();
    let tv_moves = changes.iter().filter(|c| c.field == "broadcast").count();
    assert_eq!(kickoff_moves, 1);
    assert_eq!(tv_moves, 2);
    assert!(changes.iter().any(|c| c.field == "venue" && c.significant));
}

#[tokio::test]
async fn feed_matches_either_side_of_the_matchup() {
    let (_pg, store) = boot().await;
    store.sync(&[game()]).await.expect("sync");

    // The feed is centred on a team, which may be home or away in any given game.
    let away = store.feed("nfl", "sea").await.expect("away feed");
    let home = store.feed("nfl", "lar").await.expect("home feed");
    assert_eq!(away.len(), 1);
    assert_eq!(home.len(), 1);

    assert!(away[0].game.title_for("SEA").contains("Seahawks at Los Angeles Rams"));
    assert!(home[0].game.title_for("LAR").contains("Rams vs. Seattle Seahawks"));

    let missing = store.feed("nfl", "gb").await.expect("unrelated feed");
    assert!(missing.is_empty());
}
