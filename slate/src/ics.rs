//! iCalendar (RFC 5545) rendering.
//!
//! Two details carry most of the weight here. Every event keeps a stable `UID`
//! derived from the upstream game id, and every event carries a `SEQUENCE` taken
//! from the store's revision count. Together they are what makes a rescheduled
//! game update in place in a subscriber's calendar instead of being ignored or
//! duplicated.

use chrono::{DateTime, Datelike, Duration, Utc};

use crate::model::Game;

/// A game plus the revision count that backs its `SEQUENCE`.
#[derive(Debug, Clone)]
pub struct FeedGame {
    pub game: Game,
    pub sequence: i32,
}

/// Typical broadcast window per league, used to give events a sensible end time.
fn duration_minutes(league: &str) -> i64 {
    match league {
        "nba" | "ncaam" => 150,
        "nhl" | "mls" => 165,
        "mlb" => 180,
        _ => 195,
    }
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out
}

/// Folds a content line to 75 octets, per RFC 5545 section 3.1.
fn fold(line: &str) -> String {
    if line.len() <= 75 {
        return line.to_string();
    }
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;
    for ch in line.chars() {
        let ch_len = ch.len_utf8();
        // Continuation lines start with a space, which counts toward the limit.
        let limit = if parts.is_empty() { 75 } else { 74 };
        if current_len + ch_len > limit {
            parts.push(std::mem::take(&mut current));
            current_len = 0;
        }
        current.push(ch);
        current_len += ch_len;
    }
    parts.push(current);
    parts.join("\r\n ")
}

struct Writer {
    lines: Vec<String>,
}

impl Writer {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }

    fn raw(&mut self, line: &str) {
        self.lines.push(fold(line));
    }

    fn prop(&mut self, name: &str, value: &str) {
        let escaped = escape(value);
        self.raw(&format!("{name}:{escaped}"));
    }

    fn finish(self) -> String {
        let mut out = self.lines.join("\r\n");
        out.push_str("\r\n");
        out
    }
}

fn stamp(time: DateTime<Utc>) -> String {
    time.format("%Y%m%dT%H%M%SZ").to_string()
}

fn date(time: DateTime<Utc>) -> String {
    time.format("%Y%m%d").to_string()
}

/// Renders a subscribable calendar for one team.
///
/// `team` is the abbreviation the feed is centred on, which decides whether each
/// game reads as "vs." or "at". `bye_week`, when present, adds an all-day marker.
pub fn render(league: &str, team: &str, games: &[FeedGame], bye_week: Option<i32>, now: DateTime<Utc>) -> String {
    let mut w = Writer::new();
    let label = games
        .iter()
        .find_map(|f| team_display(&f.game, team))
        .unwrap_or_else(|| team.to_uppercase());

    w.raw("BEGIN:VCALENDAR");
    w.raw("VERSION:2.0");
    w.raw("PRODID:-//slate//sports schedule feeds//EN");
    w.raw("CALSCALE:GREGORIAN");
    w.raw("METHOD:PUBLISH");
    w.prop("X-WR-CALNAME", &label);
    w.prop(
        "X-WR-CALDESC",
        &format!("{label} schedule. Times follow the league and update automatically when games move."),
    );
    w.raw("REFRESH-INTERVAL;VALUE=DURATION:PT6H");
    w.raw("X-PUBLISHED-TTL:PT6H");

    for entry in games {
        write_event(&mut w, entry, team, league, now);
    }

    if let Some(bye) = bye_week
        && let Some(day) = bye_date(games, bye)
    {
        w.raw("BEGIN:VEVENT");
        w.prop("UID", &format!("{league}-{}-bye-{bye}@slate", team.to_lowercase()));
        w.prop("DTSTAMP", &stamp(now));
        w.raw(&format!("DTSTART;VALUE=DATE:{}", date(day)));
        w.raw(&format!("DTEND;VALUE=DATE:{}", date(day + Duration::days(1))));
        w.prop("SUMMARY", &format!("{label} bye (week {bye})"));
        w.prop("DESCRIPTION", "No game this week.");
        w.raw("STATUS:CONFIRMED");
        w.raw("TRANSP:TRANSPARENT");
        w.raw("END:VEVENT");
    }

    w.raw("END:VCALENDAR");
    w.finish()
}

fn team_display(game: &Game, team: &str) -> Option<String> {
    if game.home_abbr.eq_ignore_ascii_case(team) {
        Some(game.home_name.clone())
    } else if game.away_abbr.eq_ignore_ascii_case(team) {
        Some(game.away_name.clone())
    } else {
        None
    }
}

/// Places the bye on the Sunday of the idle week, inferred from the last game before it.
fn bye_date(games: &[FeedGame], bye: i32) -> Option<DateTime<Utc>> {
    let prior = games
        .iter()
        .filter(|f| f.game.week.is_some_and(|w| w < bye))
        .max_by_key(|f| f.game.kickoff)?;
    let candidate = prior.game.kickoff + Duration::days(7);
    let back = i64::from(candidate.weekday().num_days_from_sunday());
    Some(candidate - Duration::days(back))
}

fn write_event(w: &mut Writer, entry: &FeedGame, team: &str, league: &str, now: DateTime<Utc>) {
    let game = &entry.game;
    let mut summary = game.title_for(team);
    if game.time_tbd {
        summary.push_str(" (time TBD)");
    }

    w.raw("BEGIN:VEVENT");
    w.prop("UID", &format!("{league}-{}@slate", game.espn_id));
    w.prop("DTSTAMP", &stamp(now));

    if game.time_tbd {
        // No confirmed kickoff yet, so occupy the day rather than assert an hour.
        w.raw(&format!("DTSTART;VALUE=DATE:{}", date(game.kickoff)));
        w.raw(&format!("DTEND;VALUE=DATE:{}", date(game.kickoff + Duration::days(1))));
    } else {
        let end = game.kickoff + Duration::minutes(duration_minutes(league));
        w.raw(&format!("DTSTART:{}", stamp(game.kickoff)));
        w.raw(&format!("DTEND:{}", stamp(end)));
    }

    w.prop("SUMMARY", &summary);
    if let Some(location) = game.location() {
        w.prop("LOCATION", &location);
    }
    w.prop("DESCRIPTION", &description(game));
    w.raw(&format!("STATUS:{}", game.status.ics_status()));
    w.raw(&format!("SEQUENCE:{}", entry.sequence));
    w.raw("TRANSP:TRANSPARENT");
    w.raw("END:VEVENT");
}

fn description(game: &Game) -> String {
    let mut parts: Vec<String> = vec![game.name.clone()];
    if let Some(week) = game.week {
        parts.push(format!("Week {week}"));
    }
    if let Some(tv) = game.broadcast.as_ref() {
        parts.push(format!("TV: {tv}"));
    }
    if game.time_tbd {
        parts.push("Kickoff time not yet set by the league; this entry updates automatically.".to_string());
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GameStatus;

    fn sample(tbd: bool) -> FeedGame {
        FeedGame {
            game: Game {
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
                kickoff: "2027-01-10T05:00:00Z".parse().expect("valid time"),
                time_tbd: tbd,
                venue: Some("SoFi Stadium".into()),
                city: Some("Inglewood, CA".into()),
                status: GameStatus::Scheduled,
                broadcast: Some("FOX".into()),
            },
            sequence: 3,
        }
    }

    fn now() -> DateTime<Utc> {
        "2026-09-08T12:00:00Z".parse().expect("valid time")
    }

    #[test]
    fn every_line_fits_the_octet_limit() {
        let out = render("nfl", "SEA", &[sample(false)], None, now());
        for line in out.split("\r\n") {
            assert!(line.len() <= 75, "line too long: {line}");
        }
    }

    #[test]
    fn commas_in_location_are_escaped() {
        let out = render("nfl", "SEA", &[sample(false)], None, now());
        let unfolded = out.replace("\r\n ", "");
        assert!(
            unfolded.contains("LOCATION:SoFi Stadium\\, Inglewood\\, CA"),
            "{unfolded}"
        );
    }

    #[test]
    fn away_game_reads_as_at() {
        let out = render("nfl", "SEA", &[sample(false)], None, now());
        let unfolded = out.replace("\r\n ", "");
        assert!(unfolded.contains("SUMMARY:Seattle Seahawks at Los Angeles Rams"));
    }

    #[test]
    fn sequence_is_carried_through() {
        let out = render("nfl", "SEA", &[sample(false)], None, now());
        assert!(out.contains("SEQUENCE:3"));
    }

    #[test]
    fn tbd_game_renders_all_day() {
        let out = render("nfl", "SEA", &[sample(true)], None, now());
        let unfolded = out.replace("\r\n ", "");
        assert!(unfolded.contains("DTSTART;VALUE=DATE:20270110"), "{unfolded}");
        assert!(unfolded.contains("(time TBD)"));
    }

    #[test]
    fn timed_game_uses_utc_instants() {
        let out = render("nfl", "SEA", &[sample(false)], None, now());
        assert!(out.contains("DTSTART:20270110T050000Z"));
        assert!(out.contains("DTEND:20270110T081500Z"));
    }

    #[test]
    fn uid_is_stable_across_renders() {
        let a = render("nfl", "SEA", &[sample(false)], None, now());
        let b = render("nfl", "SEA", &[sample(true)], None, now());
        assert!(a.contains("UID:nfl-401872999@slate"));
        assert!(b.contains("UID:nfl-401872999@slate"));
    }

    #[test]
    fn structure_is_balanced() {
        let out = render("nfl", "SEA", &[sample(false)], None, now());
        assert_eq!(out.matches("BEGIN:VEVENT").count(), out.matches("END:VEVENT").count());
        assert!(out.starts_with("BEGIN:VCALENDAR"));
        assert!(out.ends_with("END:VCALENDAR\r\n"));
    }
}
