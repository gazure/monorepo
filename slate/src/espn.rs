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
    /// The abbreviation upstream files this team's games under, which may
    /// differ from the identifier used to fetch them.
    pub team_abbr: String,
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
        self.team_schedule_at(BASE, league, team, season).await
    }

    async fn team_schedule_at(&self, base: &str, league: &League, team: &str, season: i32) -> Result<TeamSchedule> {
        let url = format!("{base}/{}/teams/{team}/schedule?season={season}", league.path);
        let mut body = self.fetch_schedule(&url).await?;
        // Soccer separates completed results from upcoming fixtures. Keep the
        // results when an event appears in both responses during a match.
        if league.path.starts_with("soccer/") {
            let fixtures = self.fetch_schedule(&format!("{url}&fixture=true")).await?;
            let mut seen: std::collections::HashSet<String> =
                body.events.iter().map(|event| event.id.clone()).collect();
            body.events.extend(
                fixtures
                    .events
                    .into_iter()
                    .filter(|event| seen.insert(event.id.clone())),
            );
            body.team = body.team.or(fixtures.team);
        }

        let mut games = Vec::with_capacity(body.events.len());
        for event in &body.events {
            match convert(event, league.key, season) {
                Ok(game) => games.push(game),
                Err(error) => tracing::warn!(event = %event.id, %error, "skipping unparseable event"),
            }
        }
        games.sort_by_key(|g| g.kickoff);

        let team_abbr = body
            .team
            .and_then(|t| t.abbreviation)
            .unwrap_or_else(|| team.to_uppercase());

        Ok(TeamSchedule {
            team_abbr,
            games,
            bye_week: body.bye_week,
        })
    }

    async fn fetch_schedule(&self, url: &str) -> Result<ScheduleResponse> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .with_context(|| format!("requesting {url}"))?;

        if !response.status().is_success() {
            bail!("{url} returned {}", response.status());
        }

        response.json().await.with_context(|| format!("decoding {url}"))
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
    team: Option<Team>,
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::{Json, Router, extract::Query, http::StatusCode, response::IntoResponse, routing::get};
    use serde_json::{Value, json};

    use super::{Client, league};

    fn event(id: &str, date: &str) -> Value {
        json!({
            "id": id, "date": date,
            "competitions": [{"competitors": [
                {"homeAway": "home", "team": {"abbreviation": "SEA"}},
                {"homeAway": "away", "team": {"abbreviation": "LA"}}
            ]}]
        })
    }

    async fn mock_server(fail_fixtures: bool) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new().route(
            "/{sport}/{league}/teams/{team}/schedule",
            get(move |Query(query): Query<HashMap<String, String>>| async move {
                assert_eq!(query.get("season").map(String::as_str), Some("2026"));
                if query.get("fixture").is_some_and(|value| value == "true") {
                    if fail_fixtures {
                        return StatusCode::BAD_GATEWAY.into_response();
                    }
                    Json(json!({"events": [
                        event("future", "2026-09-13T02:30Z"),
                        event("past", "2026-09-07T00:30Z")
                    ]}))
                    .into_response()
                } else {
                    Json(json!({
                        "team": {"abbreviation": "SEA"}, "byeWeek": 11,
                        "events": [event("past", "2026-09-06T00:30Z")]
                    }))
                    .into_response()
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("address"));
        let task = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
        (base, task)
    }

    #[tokio::test]
    async fn soccer_combines_results_and_fixtures_without_duplicate_events() {
        let (base, task) = mock_server(false).await;
        let schedule = Client::new()
            .team_schedule_at(&base, league("mls").expect("league"), "usa.seattle", 2026)
            .await
            .expect("schedule");
        task.abort();
        assert_eq!(schedule.team_abbr, "SEA");
        assert_eq!(schedule.games.len(), 2);
        assert_eq!(schedule.games[0].espn_id, "past");
        assert_eq!(schedule.games[0].kickoff.to_rfc3339(), "2026-09-06T00:30:00+00:00");
        assert_eq!(schedule.games[1].espn_id, "future");
        assert_eq!(schedule.games[1].kickoff.to_rfc3339(), "2026-09-13T02:30:00+00:00");
    }

    #[tokio::test]
    async fn soccer_reports_fixture_failure_instead_of_returning_a_partial_schedule() {
        let (base, task) = mock_server(true).await;
        let result = Client::new()
            .team_schedule_at(&base, league("mls").expect("league"), "usa.seattle", 2026)
            .await;
        task.abort();
        assert!(result.expect_err("fixtures failed").to_string().contains("502"));
    }

    #[tokio::test]
    async fn other_sports_do_not_request_soccer_fixtures() {
        let (base, task) = mock_server(true).await;
        let schedule = Client::new()
            .team_schedule_at(&base, league("nfl").expect("league"), "sea", 2026)
            .await
            .expect("schedule");
        task.abort();
        assert_eq!(schedule.games.len(), 1);
        assert_eq!(schedule.bye_week, Some(11));
    }
}
