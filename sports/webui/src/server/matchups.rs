use dioxus::prelude::*;

use crate::dto::{MatchupDto, MatchupEvent, MatchupTally};

/// Plate-appearance outcome classified from a `bbref` play description
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Single,
    Double,
    Triple,
    HomeRun,
    Walk,
    HitByPitch,
    Strikeout,
    Other,
}

pub(crate) fn classify_outcome(description: &str) -> Outcome {
    let d = description.trim();
    // Order matters: "Double Play" / "Triple Play" describe outs, not hits
    if d.starts_with("Double Play") || d.starts_with("Triple Play") || d.starts_with("Ground") {
        return Outcome::Other;
    }
    if d.starts_with("Single") {
        Outcome::Single
    } else if d.starts_with("Double") {
        Outcome::Double
    } else if d.starts_with("Triple") {
        Outcome::Triple
    } else if d.starts_with("Home Run") || d.starts_with("Inside-the-park Home Run") {
        Outcome::HomeRun
    } else if d.starts_with("Walk") || d.starts_with("Intentional Walk") {
        Outcome::Walk
    } else if d.starts_with("Hit By Pitch") {
        Outcome::HitByPitch
    } else if d.starts_with("Strikeout") {
        Outcome::Strikeout
    } else {
        Outcome::Other
    }
}

/// Baserunning-only rows ("E. Núñez Steals 2B") share the batter's at-bat but
/// aren't plate appearances
pub(crate) fn is_baserunning_only(d: &str) -> bool {
    const BASERUNNING: [&str; 8] = [
        "Steals",
        "Caught Stealing",
        "Picked Off",
        "Pickoff",
        "Wild Pitch",
        "Passed Ball",
        "Balk",
        "Defensive Indifference",
    ];
    const BATTING: [&str; 13] = [
        "Single",
        "Double",
        "Triple",
        "Home Run",
        "Walk",
        "Strikeout",
        "Groundout",
        "Flyball",
        "Lineout",
        "Popfly",
        "Reached",
        "Hit By Pitch",
        "Bunt",
    ];
    BASERUNNING.iter().any(|k| d.contains(k)) && !BATTING.iter().any(|k| d.contains(k))
}

pub(crate) fn tally_outcomes<'a>(descriptions: impl Iterator<Item = Option<&'a str>>) -> MatchupTally {
    let mut t = MatchupTally::default();
    for desc in descriptions {
        if desc.is_some_and(is_baserunning_only) {
            continue;
        }
        t.pa += 1;
        match desc.map_or(Outcome::Other, classify_outcome) {
            Outcome::Single => t.singles += 1,
            Outcome::Double => t.doubles += 1,
            Outcome::Triple => t.triples += 1,
            Outcome::HomeRun => t.home_runs += 1,
            Outcome::Walk => t.walks += 1,
            Outcome::HitByPitch => t.hbp += 1,
            Outcome::Strikeout => t.strikeouts += 1,
            Outcome::Other => t.other_outs += 1,
        }
    }
    t
}

/// Every head-to-head plate appearance between a batter and a pitcher
#[server]
pub async fn matchup(batter_id: i32, pitcher_id: i32) -> Result<MatchupDto, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct NameRow {
        name: String,
    }

    #[derive(sqlx::FromRow)]
    struct EventRow {
        game_id: i32,
        game_date: chrono::NaiveDate,
        inning: i32,
        is_bottom: bool,
        description: Option<String>,
        wpa: Option<f64>,
    }

    let pool = crate::pool().await?;

    let mut names = Vec::new();
    for id in [batter_id, pitcher_id] {
        let row: Option<NameRow> = sqlx::query_as("SELECT name FROM players WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(super::db_err)?;
        names.push(
            row.ok_or_else(|| ServerFnError::new(format!("player {id} not found")))?
                .name,
        );
    }
    let pitcher = names.pop().expect("two names pushed");
    let batter = names.pop().expect("two names pushed");

    let events: Vec<EventRow> = sqlx::query_as(
        r"
        SELECT p.game_id, g.game_date, p.inning, p.is_bottom,
               p.play_description AS description, p.wpa::float8 AS wpa
        FROM play_by_play p
        JOIN games g ON g.id = p.game_id
        WHERE p.batter_id = $1 AND p.pitcher_id = $2
        ORDER BY g.game_date DESC, p.event_num DESC
        ",
    )
    .bind(batter_id)
    .bind(pitcher_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    let tally = tally_outcomes(events.iter().map(|e| e.description.as_deref()));

    Ok(MatchupDto {
        batter,
        pitcher,
        tally,
        events: events
            .into_iter()
            .map(|e| MatchupEvent {
                game_id: e.game_id,
                game_date: e.game_date,
                inning: e.inning,
                is_bottom: e.is_bottom,
                description: e.description,
                wpa: e.wpa,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_hits_and_outs() {
        assert_eq!(classify_outcome("Single to CF"), Outcome::Single);
        assert_eq!(classify_outcome("Double to LF (Line Drive)"), Outcome::Double);
        assert_eq!(classify_outcome("Home Run (Fly Ball to Deep RF)"), Outcome::HomeRun);
        assert_eq!(classify_outcome("Strikeout Swinging"), Outcome::Strikeout);
        assert_eq!(classify_outcome("Walk"), Outcome::Walk);
        assert_eq!(classify_outcome("Flyball: CF"), Outcome::Other);
    }

    #[test]
    fn double_play_is_not_a_double() {
        assert_eq!(classify_outcome("Double Play: SS-2B-1B"), Outcome::Other);
        assert_eq!(classify_outcome("Ground Ball Double Play: 3B-2B-1B"), Outcome::Other);
        assert_eq!(classify_outcome("Grounded into Double Play"), Outcome::Other);
    }

    #[test]
    fn tallies_descriptions() {
        let t = tally_outcomes([Some("Single to LF"), Some("Strikeout Looking"), Some("Walk"), None].into_iter());
        assert_eq!(t.pa, 4);
        assert_eq!(t.hits(), 1);
        assert_eq!(t.strikeouts, 1);
        assert_eq!(t.walks, 1);
        assert_eq!(t.other_outs, 1);
    }

    #[test]
    fn baserunning_events_are_not_plate_appearances() {
        assert!(is_baserunning_only("E. Núñez Steals 2B"));
        assert!(is_baserunning_only("D. Solano Caught Stealing (PO) 2B (P-1B-2B-1B)"));
        assert!(!is_baserunning_only("Single to CF; E. Núñez Steals 3B"));
        assert!(!is_baserunning_only("Strikeout Swinging"));

        let t = tally_outcomes([Some("E. Núñez Steals 2B"), Some("Single to CF")].into_iter());
        assert_eq!(t.pa, 1);
        assert_eq!(t.hits(), 1);
    }
}
