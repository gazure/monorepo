//! Exercises the served HTTP surface against a real database, so the routes,
//! headers, and 404 behaviour are verified rather than assumed.

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use postgresql_embedded::{PostgreSQL, Settings};
use slate::{
    config::Config,
    model::{Game, GameStatus},
    server::{AppState, router},
    store::Store,
};

const CONFIG: &str = r#"
database_url = "unused"
listen = "127.0.0.1:0"

[[feeds]]
league = "nfl"
team = "sea"
"#;

fn game() -> Game {
    Game {
        league: "nfl".into(),
        espn_id: "401872999".into(),
        season: 2026,
        week: Some(1),
        name: "New England Patriots at Seattle Seahawks".into(),
        short_name: "NE @ SEA".into(),
        home_abbr: "SEA".into(),
        home_name: "Seattle Seahawks".into(),
        away_abbr: "NE".into(),
        away_name: "New England Patriots".into(),
        kickoff: Utc.with_ymd_and_hms(2026, 9, 10, 0, 20, 0).single().expect("valid"),
        time_tbd: false,
        venue: Some("Lumen Field".into()),
        city: Some("Seattle, WA".into()),
        status: GameStatus::Scheduled,
        broadcast: Some("NBC".into()),
    }
}

#[tokio::test]
async fn serves_a_subscribable_calendar() {
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
    store.sync(&[game()]).await.expect("sync");

    let config = Arc::new(Config::from_toml(CONFIG).expect("config"));
    let state = AppState { store, config };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.expect("serve");
    });

    let base = format!("http://{addr}");
    let http = reqwest::Client::new();

    // The .ics suffix is what calendar clients ask for.
    let response = http.get(format!("{base}/nfl/sea.ics")).send().await.expect("request");
    assert_eq!(response.status(), 200);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert_eq!(content_type, "text/calendar; charset=utf-8");

    let body = response.text().await.expect("body");
    assert!(body.starts_with("BEGIN:VCALENDAR"));
    assert!(body.contains("UID:nfl-401872999@slate"));
    assert!(
        body.replace("\r\n ", "")
            .contains("SUMMARY:Seattle Seahawks vs. New England Patriots")
    );

    // Case and the bare path should both resolve.
    for path in ["/nfl/SEA.ics", "/nfl/sea"] {
        let response = http.get(format!("{base}{path}")).send().await.expect("request");
        assert_eq!(response.status(), 200, "{path} should resolve");
    }

    // An unknown team is a 404, not an empty calendar a client would silently accept.
    let response = http.get(format!("{base}/nfl/gb.ics")).send().await.expect("request");
    assert_eq!(response.status(), 404);

    let response = http.get(format!("{base}/healthz")).send().await.expect("request");
    assert_eq!(response.status(), 200);
}
