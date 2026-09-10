//! HTTP surface: one subscribable `.ics` per configured feed.

use std::{fmt::Write as _, sync::Arc};

use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use chrono::Utc;

use crate::{
    config::Config,
    ics::{self, FeedGame},
    store::Store,
};

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub config: Arc<Config>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/healthz", get(|| async { "ok" }))
        .route("/{league}/{team}", get(feed))
        .with_state(state)
}

async fn index(State(state): State<AppState>) -> Html<String> {
    let mut body = String::from(
        "<!doctype html><meta charset=utf-8><title>slate</title>\
         <style>body{font:15px/1.6 system-ui;margin:3rem auto;max-width:34rem;padding:0 1rem}\
         code{background:#f4f4f5;padding:.15em .4em;border-radius:3px}</style>\
         <h1>slate</h1><p>Subscribe to these URLs from your calendar app:</p><ul>",
    );
    for feed in &state.config.feeds {
        let slug = feed.slug();
        let _ = write!(body, "<li><code>/{slug}.ics</code></li>");
    }
    body.push_str("</ul><p>Feeds re-check upstream automatically; moved games update in place.</p>");
    Html(body)
}

/// Infers the idle week from a gap in the week numbering.
///
/// Leagues that do not number weeks yield `None`, which is the desired result.
fn infer_bye(games: &[FeedGame]) -> Option<i32> {
    let weeks: Vec<i32> = games.iter().filter_map(|f| f.game.week).collect();
    let first = *weeks.iter().min()?;
    let last = *weeks.iter().max()?;
    (first..=last).find(|w| !weeks.contains(w))
}

async fn feed(Path((league, team)): Path<(String, String)>, State(state): State<AppState>) -> Response {
    let league = league.to_lowercase();
    // Calendar clients want the .ics suffix; the store does not care about it.
    let team = team.strip_suffix(".ics").unwrap_or(&team).to_lowercase();

    let games = match state.store.feed(&league, &team).await {
        Ok(games) => games,
        Err(error) => {
            tracing::error!(%league, %team, %error, "feed query failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response();
        }
    };

    if games.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            format!("no games stored for {league}/{team}; has it been synced?"),
        )
            .into_response();
    }

    let body = ics::render(&league, &team, &games, infer_bye(&games), Utc::now());

    (
        [
            (header::CONTENT_TYPE, "text/calendar; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{league}-{team}.ics\""),
            ),
            (header::CACHE_CONTROL, "public, max-age=1800".to_string()),
        ],
        body,
    )
        .into_response()
}
