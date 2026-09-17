//! Client for ESPN's public site API.
//!
//! One endpoint shape serves every league, so a league is just a path prefix:
//! `site.api.espn.com/apis/site/v2/sports/{path}/teams/{team}/schedule`.

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use serde::Deserialize;

use crate::model::{Game, GameStatus};

const BASE: &str = "https://site.api.espn.com/apis/site/v2/sports";

/// ESPN's edge rejects both browser-imitating and bare tool user agents with a
/// 403. It does accept the conventional bot form carrying a contact URL, which
/// is what we want to send anyway. Do not "simplify" this string.
const USER_AGENT: &str = "slate/0.1 (+https://github.com/gazure/monorepo)";

/// How a league labels a season that crosses a new year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeasonLabel {
    /// Labelled by the year it starts, e.g. the NFL's 2026 season ends in Jan 2027.
    StartYear,
    /// Labelled by the year it ends, e.g. the NBA's 2026-27 season is "2027".
    EndYear,
    /// Begins and ends inside one calendar year.
    Calendar,
}

/// A supported league and the path segment ESPN files it under.
#[derive(Debug, Clone, Copy)]
pub struct League {
    pub key: &'static str,
    pub path: &'static str,
    pub display: &'static str,
    pub season_label: SeasonLabel,
}

impl League {
    /// The season identifier upstream expects for a given moment.
    pub fn current_season(&self, now: DateTime<Utc>) -> i32 {
        let year = now.year();
        match self.season_label {
            SeasonLabel::StartYear if now.month() <= 2 => year - 1,
            SeasonLabel::EndYear if now.month() >= 8 => year + 1,
            _ => year,
        }
    }
}

pub const LEAGUES: &[League] = &[
    League {
        key: "nfl",
        path: "football/nfl",
        display: "NFL",
        season_label: SeasonLabel::StartYear,
    },
    League {
        key: "ncaaf",
        path: "football/college-football",
        display: "NCAA Football",
        season_label: SeasonLabel::StartYear,
    },
    League {
        key: "nba",
        path: "basketball/nba",
        display: "NBA",
        season_label: SeasonLabel::EndYear,
    },
    League {
        key: "nhl",
        path: "hockey/nhl",
        display: "NHL",
        season_label: SeasonLabel::EndYear,
    },
    League {
        key: "ncaam",
        path: "basketball/mens-college-basketball",
        display: "NCAA Men's Basketball",
        season_label: SeasonLabel::EndYear,
    },
    League {
        key: "mlb",
        path: "baseball/mlb",
        display: "MLB",
        season_label: SeasonLabel::Calendar,
    },
    League {
        key: "mls",
        path: "soccer/usa.1",
        display: "MLS",
        season_label: SeasonLabel::Calendar,
    },
    League {
        key: "wnba",
        path: "basketball/wnba",
        display: "WNBA",
        season_label: SeasonLabel::Calendar,
    },
];

pub fn league(key: &str) -> Option<&'static League> {
    LEAGUES.iter().find(|l| l.key.eq_ignore_ascii_case(key))
}

/// A team's full season as fetched upstream.
#[derive(Debug, Clone)]
pub struct TeamSchedule {
    pub games: Vec<Game>,
    /// Week number the team is idle, when the league publishes one.
    pub bye_week: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_default();
        Self { http }
    }

    /// Fetches one team's schedule for a season.
    pub async fn team_schedule(&self, league: &League, team: &str, season: i32) -> Result<TeamSchedule> {
        let url = format!("{BASE}/{}/teams/{team}/schedule?season={season}", league.path);
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("requesting {url}"))?;

        if !response.status().is_success() {
            bail!("{url} returned {}", response.status());
        }

        let body: ScheduleResponse = response.json().await.with_context(|| format!("decoding {url}"))?;

        let mut games = Vec::with_capacity(body.events.len());
        for event in &body.events {
            match convert(event, league.key, season) {
                Ok(game) => games.push(game),
                Err(error) => tracing::warn!(event = %event.id, %error, "skipping unparseable event"),
            }
        }
        games.sort_by_key(|g| g.kickoff);

        Ok(TeamSchedule {
            games,
            bye_week: body.bye_week,
        })
    }
}

/// ESPN emits `2026-09-10T00:20Z`, which omits seconds and so is not valid RFC 3339.
fn parse_time(raw: &str) -> Result<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
        return Ok(parsed.with_timezone(&Utc));
    }
    let naive = NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%MZ")
        .with_context(|| format!("unrecognized timestamp {raw:?}"))?;
    Ok(DateTime::from_naive_utc_and_offset(naive, Utc))
}

fn convert(event: &Event, league_key: &str, season: i32) -> Result<Game> {
    let competition = event.competitions.first().context("event has no competition")?;

    let home = competition
        .competitors
        .iter()
        .find(|c| c.home_away == "home")
        .context("no home competitor")?;
    let away = competition
        .competitors
        .iter()
        .find(|c| c.home_away == "away")
        .context("no away competitor")?;

    let status_name = competition
        .status
        .as_ref()
        .and_then(|s| s.kind.as_ref())
        .map_or("STATUS_SCHEDULED", |k| k.name.as_str());

    // Upstream signals an unset kickoff two ways; either one means the stored
    // timestamp is a placeholder rather than a real start time.
    let flex = competition.status.as_ref().and_then(|s| s.is_tbd_flex).unwrap_or(false);
    let time_tbd = flex || !event.time_valid.unwrap_or(true);

    let venue = competition.venue.as_ref().and_then(|v| v.full_name.clone());
    let city = competition
        .venue
        .as_ref()
        .and_then(|v| v.address.as_ref())
        .map(Address::render);

    let broadcast = competition.broadcasts.as_ref().and_then(|list| {
        let names: Vec<&str> = list
            .iter()
            .filter_map(|b| b.media.as_ref())
            .filter_map(|m| m.short_name.as_deref())
            .collect();
        if names.is_empty() { None } else { Some(names.join("/")) }
    });

    Ok(Game {
        league: league_key.to_string(),
        espn_id: event.id.clone(),
        season: event.season.as_ref().map_or(season, |s| s.year),
        week: event.week.as_ref().map(|w| w.number),
        name: event.name.clone(),
        short_name: event.short_name.clone(),
        home_abbr: home.team.abbreviation.clone().unwrap_or_default(),
        home_name: home.team.display_name.clone().unwrap_or_default(),
        away_abbr: away.team.abbreviation.clone().unwrap_or_default(),
        away_name: away.team.display_name.clone().unwrap_or_default(),
        kickoff: parse_time(&event.date)?,
        time_tbd,
        venue,
        city,
        status: GameStatus::from_espn(status_name),
        broadcast,
    })
}

#[derive(Debug, Deserialize)]
struct ScheduleResponse {
    #[serde(default)]
    events: Vec<Event>,
    #[serde(rename = "byeWeek")]
    bye_week: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct Event {
    id: String,
    date: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "shortName", default)]
    short_name: String,
    #[serde(rename = "timeValid")]
    time_valid: Option<bool>,
    week: Option<Week>,
    season: Option<Season>,
    #[serde(default)]
    competitions: Vec<Competition>,
}

#[derive(Debug, Deserialize)]
struct Season {
    year: i32,
}

#[derive(Debug, Deserialize)]
struct Week {
    number: i32,
}

#[derive(Debug, Deserialize)]
struct Competition {
    venue: Option<Venue>,
    status: Option<Status>,
    broadcasts: Option<Vec<Broadcast>>,
    #[serde(default)]
    competitors: Vec<Competitor>,
}

#[derive(Debug, Deserialize)]
struct Venue {
    #[serde(rename = "fullName")]
    full_name: Option<String>,
    address: Option<Address>,
}

#[derive(Debug, Deserialize)]
struct Address {
    city: Option<String>,
    state: Option<String>,
    country: Option<String>,
}

impl Address {
    fn render(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if let Some(city) = self.city.as_deref() {
            parts.push(city);
        }
        if let Some(state) = self.state.as_deref() {
            parts.push(state);
        } else if let Some(country) = self.country.as_deref() {
            parts.push(country);
        }
        parts.join(", ")
    }
}

#[derive(Debug, Deserialize)]
struct Status {
    #[serde(rename = "type")]
    kind: Option<StatusType>,
    #[serde(rename = "isTBDFlex")]
    is_tbd_flex: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct StatusType {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Broadcast {
    media: Option<Media>,
}

#[derive(Debug, Deserialize)]
struct Media {
    #[serde(rename = "shortName")]
    short_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Competitor {
    #[serde(rename = "homeAway")]
    home_away: String,
    team: Team,
}

#[derive(Debug, Deserialize)]
struct Team {
    abbreviation: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}
