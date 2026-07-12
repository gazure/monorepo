//! Era-aware league/division reference for the franchises in the data
//! (1950 onward). The schema stores no league/division, and the alignment
//! changed over time — no divisions before 1969, East/West 1969–1993,
//! three divisions from 1994 — so this maps (team code, season) to the
//! alignment that applied that year.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeagueDiv {
    pub league: &'static str,
    pub division: Option<&'static str>,
}

const fn ld(league: &'static str, division: Option<&'static str>) -> LeagueDiv {
    LeagueDiv { league, division }
}

/// League/division for a team code in a given season; `None` for codes or
/// seasons outside the mapping (callers fall back to ungrouped standings)
#[allow(clippy::too_many_lines, clippy::match_same_arms)]
pub fn league_division(code: &str, season: i32) -> Option<LeagueDiv> {
    let s = season;
    let al = |div| Some(ld("AL", div));
    let nl = |div| Some(ld("NL", div));

    // Pre-division era: league only
    let al_div = |from_1969: &'static str, from_1994: &'static str| {
        if s < 1969 {
            al(None)
        } else if s < 1994 {
            al(Some(from_1969))
        } else {
            al(Some(from_1994))
        }
    };
    let nl_div = |from_1969: &'static str, from_1994: &'static str| {
        if s < 1969 {
            nl(None)
        } else if s < 1994 {
            nl(Some(from_1969))
        } else {
            nl(Some(from_1994))
        }
    };

    match code {
        // ---- American League ----
        "NYY" => al_div("East", "East"),
        "BOS" => al_div("East", "East"),
        "BAL" if s >= 1954 => al_div("East", "East"),
        "SLB" if (1950..=1953).contains(&s) => al(None),
        "DET" => {
            if s < 1969 {
                al(None)
            } else if s < 1998 {
                al(Some("East"))
            } else {
                al(Some("Central"))
            }
        }
        "CLE" => al_div("East", "Central"),
        "CHW" => al_div("West", "Central"),
        "MIN" if s >= 1961 => al_div("West", "Central"),
        "WSH" if (1950..=1960).contains(&s) => al(None),
        "WSA" if (1961..=1971).contains(&s) => {
            if s < 1969 {
                al(None)
            } else {
                al(Some("East"))
            }
        }
        "TEX" if s >= 1972 => al_div("West", "West"),
        "KCR" if s >= 1969 => al_div("West", "Central"),
        "KCA" if (1955..=1967).contains(&s) => al(None),
        "PHA" if (1950..=1954).contains(&s) => al(None),
        "OAK" if (1968..=2024).contains(&s) => al_div("West", "West"),
        "ATH" if s >= 2025 => al(Some("West")),
        "LAA" if (1961..=1964).contains(&s) => al(None),
        "CAL" if (1965..=1996).contains(&s) => al_div("West", "West"),
        "ANA" if (1997..=2004).contains(&s) => al(Some("West")),
        "LAA" if s >= 2005 => al(Some("West")),
        "SEP" if s == 1969 => al(Some("West")),
        "SEA" if s >= 1977 => al_div("West", "West"),
        "TOR" if s >= 1977 => al_div("East", "East"),
        "TBD" if (1998..=2007).contains(&s) => al(Some("East")),
        "TBR" if s >= 2008 => al(Some("East")),
        // Brewers: AL West 1970-71, AL East 1972-93, AL Central 1994-97,
        // NL Central from 1998
        "MIL" if (1970..=1971).contains(&s) => al(Some("West")),
        "MIL" if (1972..=1993).contains(&s) => al(Some("East")),
        "MIL" if (1994..=1997).contains(&s) => al(Some("Central")),
        "MIL" if s >= 1998 => nl(Some("Central")),
        // Astros: NL through 2012, AL West from 2013
        "HOU" if (1962..=2012).contains(&s) => nl_div("West", "Central"),
        "HOU" if s >= 2013 => al(Some("West")),

        // ---- National League ----
        "BRO" if (1950..=1957).contains(&s) => nl(None),
        "LAD" if s >= 1958 => nl_div("West", "West"),
        "NYG" if (1950..=1957).contains(&s) => nl(None),
        "SFG" if s >= 1958 => nl_div("West", "West"),
        "CHC" => nl_div("East", "Central"),
        "STL" => nl_div("East", "Central"),
        "PIT" => nl_div("East", "Central"),
        "PHI" => nl_div("East", "East"),
        "CIN" => nl_div("West", "Central"),
        "BSN" if (1950..=1952).contains(&s) => nl(None),
        "MLN" if (1953..=1965).contains(&s) => nl(None),
        // Braves: NL West 1969-93, NL East from 1994
        "ATL" if s >= 1966 => nl_div("West", "East"),
        "NYM" if s >= 1962 => nl_div("East", "East"),
        "MON" if (1969..=2004).contains(&s) => nl(Some("East")),
        "WSN" if s >= 2005 => nl(Some("East")),
        "SDP" if s >= 1969 => nl(Some("West")),
        "COL" if s >= 1993 => nl(Some("West")),
        "FLA" if (1993..=2011).contains(&s) => nl(Some("East")),
        "MIA" if s >= 2012 => nl(Some("East")),
        "ARI" if s >= 1998 => nl(Some("West")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn astros_switch_leagues_in_2013() {
        assert_eq!(league_division("HOU", 2012), Some(ld("NL", Some("Central"))));
        assert_eq!(league_division("HOU", 2013), Some(ld("AL", Some("West"))));
        assert_eq!(league_division("HOU", 1968), Some(ld("NL", None)));
    }

    #[test]
    fn brewers_wander_the_leagues() {
        assert_eq!(league_division("MIL", 1970), Some(ld("AL", Some("West"))));
        assert_eq!(league_division("MIL", 1972), Some(ld("AL", Some("East"))));
        assert_eq!(league_division("MIL", 1995), Some(ld("AL", Some("Central"))));
        assert_eq!(league_division("MIL", 1998), Some(ld("NL", Some("Central"))));
    }

    #[test]
    fn braves_move_east_in_1994() {
        assert_eq!(league_division("ATL", 1993), Some(ld("NL", Some("West"))));
        assert_eq!(league_division("ATL", 1994), Some(ld("NL", Some("East"))));
    }

    #[test]
    fn pre_division_era_is_league_only() {
        assert_eq!(league_division("NYY", 1955), Some(ld("AL", None)));
        assert_eq!(league_division("BRO", 1955), Some(ld("NL", None)));
        assert_eq!(league_division("NYY", 1969), Some(ld("AL", Some("East"))));
    }

    #[test]
    fn codes_outside_their_era_are_unknown() {
        assert_eq!(league_division("BRO", 1958), None);
        assert_eq!(league_division("TEX", 1971), None);
        assert_eq!(league_division("XXX", 2020), None);
    }
}
