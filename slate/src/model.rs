//! Normalized schedule types, independent of any upstream provider.

use chrono::{DateTime, Utc};

/// Where a game sits in its lifecycle, normalized across providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Scheduled,
    InProgress,
    Final,
    Postponed,
    Canceled,
}

impl GameStatus {
    /// Maps an ESPN `status.type.name` value onto our own vocabulary.
    ///
    /// Unknown values fall back to [`GameStatus::Scheduled`] so an upstream
    /// addition never drops a game out of a feed.
    pub fn from_espn(name: &str) -> Self {
        match name {
            "STATUS_IN_PROGRESS" | "STATUS_HALFTIME" | "STATUS_END_PERIOD" | "STATUS_DELAYED" => Self::InProgress,
            "STATUS_FINAL" | "STATUS_FULL_TIME" => Self::Final,
            "STATUS_POSTPONED" | "STATUS_SUSPENDED" | "STATUS_RAIN_DELAY" => Self::Postponed,
            "STATUS_CANCELED" | "STATUS_FORFEIT" => Self::Canceled,
            _ => Self::Scheduled,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::InProgress => "in_progress",
            Self::Final => "final",
            Self::Postponed => "postponed",
            Self::Canceled => "canceled",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "in_progress" => Self::InProgress,
            "final" => Self::Final,
            "postponed" => Self::Postponed,
            "canceled" => Self::Canceled,
            _ => Self::Scheduled,
        }
    }

    /// The iCalendar `STATUS` property for this state.
    pub fn ics_status(self) -> &'static str {
        match self {
            Self::Canceled => "CANCELLED",
            Self::Postponed => "TENTATIVE",
            _ => "CONFIRMED",
        }
    }
}

/// One game, normalized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub league: String,
    pub espn_id: String,
    pub season: i32,
    pub week: Option<i32>,
    pub name: String,
    pub short_name: String,
    pub home_abbr: String,
    pub home_name: String,
    pub away_abbr: String,
    pub away_name: String,
    pub kickoff: DateTime<Utc>,
    /// Upstream has not fixed a start time yet; the `kickoff` value is a
    /// placeholder and the game renders as all-day.
    pub time_tbd: bool,
    pub venue: Option<String>,
    pub city: Option<String>,
    pub status: GameStatus,
    pub broadcast: Option<String>,
}

/// A single observed field change between a stored game and a freshly fetched one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldChange {
    pub field: &'static str,
    pub old: Option<String>,
    pub new: Option<String>,
    /// Whether this change should bump the iCalendar `SEQUENCE`, causing
    /// subscribed clients to treat the event as revised.
    pub significant: bool,
}

fn opt(value: Option<&String>) -> Option<String> {
    value.cloned()
}

impl Game {
    /// Returns the changes needed to turn `self` (the stored game) into `fresh`.
    ///
    /// Only time, place, and status count as significant: a broadcast swap is
    /// worth recording but should not re-notify every subscriber.
    pub fn diff(&self, fresh: &Self) -> Vec<FieldChange> {
        let mut changes = Vec::new();

        if self.kickoff != fresh.kickoff {
            changes.push(FieldChange {
                field: "kickoff",
                old: Some(self.kickoff.to_rfc3339()),
                new: Some(fresh.kickoff.to_rfc3339()),
                significant: true,
            });
        }
        if self.time_tbd != fresh.time_tbd {
            changes.push(FieldChange {
                field: "time_tbd",
                old: Some(self.time_tbd.to_string()),
                new: Some(fresh.time_tbd.to_string()),
                significant: true,
            });
        }
        if self.venue != fresh.venue {
            changes.push(FieldChange {
                field: "venue",
                old: opt(self.venue.as_ref()),
                new: opt(fresh.venue.as_ref()),
                significant: true,
            });
        }
        if self.status != fresh.status {
            changes.push(FieldChange {
                field: "status",
                old: Some(self.status.as_str().to_string()),
                new: Some(fresh.status.as_str().to_string()),
                significant: true,
            });
        }
        if self.broadcast != fresh.broadcast {
            changes.push(FieldChange {
                field: "broadcast",
                old: opt(self.broadcast.as_ref()),
                new: opt(fresh.broadcast.as_ref()),
                significant: false,
            });
        }
        changes
    }

    /// Human-readable matchup from the perspective of `team`, e.g. `Seahawks at Rams`.
    pub fn title_for(&self, team: &str) -> String {
        let team = team.to_uppercase();
        if self.home_abbr.eq_ignore_ascii_case(&team) {
            format!("{} vs. {}", self.home_name, self.away_name)
        } else if self.away_abbr.eq_ignore_ascii_case(&team) {
            format!("{} at {}", self.away_name, self.home_name)
        } else {
            self.name.clone()
        }
    }

    pub fn location(&self) -> Option<String> {
        match (self.venue.as_ref(), self.city.as_ref()) {
            (Some(v), Some(c)) => Some(format!("{v}, {c}")),
            (Some(v), None) => Some(v.clone()),
            (None, Some(c)) => Some(c.clone()),
            (None, None) => None,
        }
    }
}
