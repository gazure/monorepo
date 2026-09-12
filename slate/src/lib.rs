//! slate builds subscribable calendar feeds for sports schedules and keeps them
//! correct as the leagues move games around.
//!
//! The upstream source is ESPN's public site API. Each poll is diffed against
//! stored state, every field change is logged, and calendar-significant changes
//! advance the event's iCalendar `SEQUENCE` so subscribers see a revision rather
//! than a stale entry.

pub mod config;
pub mod espn;
pub mod ics;
pub mod model;
pub mod server;
pub mod store;

use anyhow::Result;
use chrono::Utc;

use crate::{
    config::{Config, Feed},
    store::{Store, SyncReport},
};

/// Fetches every configured feed once and reconciles it into the store.
pub async fn sync_all(config: &Config, store: &Store, client: &espn::Client) -> Result<SyncReport> {
    let mut total = SyncReport::default();

    for feed in &config.feeds {
        match sync_feed(feed, store, client).await {
            Ok(report) => {
                tracing::info!(
                    feed = %feed.slug(),
                    inserted = report.inserted,
                    changed = report.changed.len(),
                    unchanged = report.unchanged,
                    "synced"
                );
                total.inserted += report.inserted;
                total.unchanged += report.unchanged;
                total.changed.extend(report.changed);
            }
            Err(error) => tracing::error!(feed = %feed.slug(), %error, "sync failed"),
        }
    }

    Ok(total)
}

async fn sync_feed(feed: &Feed, store: &Store, client: &espn::Client) -> Result<SyncReport> {
    let league = espn::league(&feed.league).ok_or_else(|| anyhow::anyhow!("unknown league {:?}", feed.league))?;
    let season = feed.season.unwrap_or_else(|| league.current_season(Utc::now()));
    let schedule = client.team_schedule(league, &feed.team, season).await?;
    store
        .record_feed_team(league.key, &feed.team, &schedule.team_abbr)
        .await?;
    store.sync(&schedule.games).await
}
