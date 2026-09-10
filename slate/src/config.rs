//! Configuration loaded from `slate.toml`.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::espn;

fn default_database_url() -> String {
    "postgresql://postgres:postgres@localhost:30432/slate".to_string()
}

fn default_listen() -> String {
    "127.0.0.1:8477".to_string()
}

fn default_poll_minutes() -> u64 {
    360
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_database_url")]
    pub database_url: String,
    #[serde(default = "default_listen")]
    pub listen: String,
    /// How often the background poller re-checks upstream, in minutes.
    #[serde(default = "default_poll_minutes")]
    pub poll_interval_minutes: u64,
    #[serde(default)]
    pub feeds: Vec<Feed>,
}

/// One team whose schedule is tracked and published.
#[derive(Debug, Clone, Deserialize)]
pub struct Feed {
    pub league: String,
    pub team: String,
    /// Pin a season; otherwise the current one is derived from the league calendar.
    pub season: Option<i32>,
}

impl Feed {
    pub fn slug(&self) -> String {
        format!("{}/{}", self.league.to_lowercase(), self.team.to_lowercase())
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path).with_context(|| format!("reading config at {}", path.display()))?;
        Self::from_toml(&raw).with_context(|| format!("in config at {}", path.display()))
    }

    /// Parses and validates a configuration document, then applies environment
    /// overrides.
    ///
    /// `SLATE_DATABASE_URL`, `SLATE_LISTEN`, and `SLATE_POLL_INTERVAL_MINUTES`
    /// take precedence over the file, so a container can point at a different
    /// database without a bespoke config.
    pub fn from_toml(raw: &str) -> Result<Self> {
        let mut config: Self = toml::from_str(raw).context("parsing config")?;
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(url) = std::env::var("SLATE_DATABASE_URL") {
            self.database_url = url;
        }
        if let Ok(listen) = std::env::var("SLATE_LISTEN") {
            self.listen = listen;
        }
        if let Ok(raw) = std::env::var("SLATE_POLL_INTERVAL_MINUTES") {
            if let Ok(minutes) = raw.parse() {
                self.poll_interval_minutes = minutes;
            } else {
                tracing::warn!(value = %raw, "ignoring unparseable SLATE_POLL_INTERVAL_MINUTES");
            }
        }
    }

    fn validate(&self) -> Result<()> {
        for feed in &self.feeds {
            if espn::league(&feed.league).is_none() {
                let known: Vec<&str> = espn::LEAGUES.iter().map(|l| l.key).collect();
                bail!(
                    "unknown league {:?} in config; known leagues: {}",
                    feed.league,
                    known.join(", ")
                );
            }
        }
        Ok(())
    }
}
