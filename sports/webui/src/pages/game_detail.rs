use dioxus::prelude::*;

use crate::{
    app::Route,
    bbref,
    components::replay::{MiniDiamond, ReplayDeck},
    dto::{BattingLineDto, GameDetailDto, PitchingLineDto, PlayDto},
    fmt, server,
};

#[component]
pub fn GameDetail(id: i32) -> Element {
    let detail = use_resource(move || server::game_detail(id));

    rsx! {
        match &*detail.read() {
            Some(Ok(d)) => rsx! {
                GameDetailView { detail: d.clone() }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load game: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading game…" }
            },
        }
    }
}

#[component]
fn GameDetailView(detail: GameDetailDto) -> Element {
    let g = &detail.game;
    let mut show_pbp = use_signal(|| false);
    let game_id = g.id;
    // Fetched eagerly: the win probability chart needs it, the table stays
    // behind the toggle.
    let pbp = use_resource(move || server::game_play_by_play(game_id));

    // None ⇒ tied/undecided: the replay renders without the WP chart
    let home_won = match (g.home_score, g.away_score) {
        (Some(hs), Some(aws)) if hs != aws => Some(hs > aws),
        _ => None,
    };

    let decisions: Vec<String> = [
        detail.winning_pitcher.as_ref().map(|p| format!("W: {p}")),
        detail.losing_pitcher.as_ref().map(|p| format!("L: {p}")),
        detail.save_pitcher.as_ref().map(|p| format!("SV: {p}")),
    ]
    .into_iter()
    .flatten()
    .collect();

    rsx! {
        ScorebugHeader { game: g.clone() }
        div { class: "game-meta",
            if let Some(venue) = &g.venue {
                span { "📍 {venue}" }
            }
            if let Some(start) = &detail.start_time {
                span { "🕐 {start}" }
            }
            if let Some(mins) = detail.duration_minutes {
                span { "⏱ {mins} min" }
            }
            if let Some(att) = g.attendance {
                span { "👥 {att}" }
            }
            if let Some(weather) = &detail.weather {
                span { "🌤 {weather}" }
            }
            if detail.is_night_game == Some(true) {
                span { "🌙 night game" }
            }
            if detail.is_artificial_turf == Some(true) {
                span { "🌱 artificial turf" }
            }
            a {
                href: bbref::box_url(&g.bbref_game_id),
                target: "_blank",
                rel: "noopener",
                "bbref ↗"
            }
        }
        if !decisions.is_empty() {
            div { class: "muted", {decisions.join(" · ")} }
        }

        StarsOfTheGame { detail: detail.clone() }

        LineScoreTable { detail: detail.clone() }

        match &*pbp.read() {
            Some(Ok(plays)) => rsx! {
                ScoringSummary {
                    plays: plays.clone(),
                    home_code: g.home.code.clone(),
                    away_code: g.away.code.clone(),
                }
            },
            _ => rsx! {},
        }

        h2 { "Batting" }
        {
            let team_plays = |code: &str| -> Vec<PlayDto> {
                match &*pbp.read() {
                    Some(Ok(plays)) => plays.iter().filter(|p| p.batting_team == code).cloned().collect(),
                    _ => Vec::new(),
                }
            };
            rsx! {
                BattingTable {
                    lines: detail.batting.iter().filter(|b| b.team_code == g.away.code).cloned().collect::<Vec<_>>(),
                    team: g.away.name.clone(),
                    plays: team_plays(&g.away.code),
                }
                BattingTable {
                    lines: detail.batting.iter().filter(|b| b.team_code == g.home.code).cloned().collect::<Vec<_>>(),
                    team: g.home.name.clone(),
                    plays: team_plays(&g.home.code),
                }
            }
        }

        h2 { "Pitching" }
        PitchingTable {
            lines: detail.pitching.iter().filter(|p| p.team_code == g.away.code).cloned().collect::<Vec<_>>(),
            team: g.away.name.clone(),
        }
        PitchingTable {
            lines: detail.pitching.iter().filter(|p| p.team_code == g.home.code).cloned().collect::<Vec<_>>(),
            team: g.home.name.clone(),
        }

        if !detail.umpires.is_empty() {
            h2 { "Umpires" }
            div { class: "muted",
                {detail.umpires.iter().map(|u| format!("{}: {}", u.position, u.name)).collect::<Vec<_>>().join(" · ")}
            }
        }

        h2 { "Game replay" }
        match &*pbp.read() {
            Some(Ok(plays)) if !plays.is_empty() => rsx! {
                ReplayDeck {
                    plays: plays.clone(),
                    home_code: g.home.code.clone(),
                    away_code: g.away.code.clone(),
                    home_won,
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load play by play: {e}" }
            },
            Some(Ok(_)) => rsx! {
                div { class: "muted", "No play-by-play recorded for this game." }
            },
            None => rsx! {
                div { class: "loading", "Loading plays…" }
            },
        }

        h2 { "Play by play" }
        button { onclick: move |_| show_pbp.toggle(), if show_pbp() { "Hide" } else { "Show" } }
        if show_pbp() {
            match &*pbp.read() {
                Some(Ok(plays)) => rsx! {
                    PlayByPlayTable { plays: plays.clone() }
                },
                Some(Err(e)) => rsx! {
                    div { class: "error-box", "Failed to load play by play: {e}" }
                },
                None => rsx! {
                    div { class: "loading", "Loading plays…" }
                },
            }
        }
    }
}

#[component]
fn LineScoreTable(detail: GameDetailDto) -> Element {
    let innings = detail.line_score.away.len().max(detail.line_score.home.len());
    if innings == 0 {
        return rsx! {};
    }
    let g = &detail.game;
    rsx! {
        div { class: "table-scroll line-score",
            table { class: "data-table",
                thead {
                    tr {
                        th { "" }
                        for i in 1..=innings {
                            th { class: "num", "{i}" }
                        }
                        th { class: "num", "R" }
                    }
                }
                tbody {
                    tr {
                        td { "{g.away.code}" }
                        for runs in detail.line_score.away.clone() {
                            td { class: "num", {runs.map_or_else(|| "x".to_string(), |r| r.to_string())} }
                        }
                        td { class: "num", b { "{fmt::score(g.away_score)}" } }
                    }
                    tr {
                        td { "{g.home.code}" }
                        for runs in detail.line_score.home.clone() {
                            td { class: "num", {runs.map_or_else(|| "x".to_string(), |r| r.to_string())} }
                        }
                        td { class: "num", b { "{fmt::score(g.home_score)}" } }
                    }
                }
            }
        }
    }
}

/// A pinch hitter/runner shares the starter's lineup slot; render the slot
/// number on the starter only and mark subs with an indented arrow
fn is_substitute(position: Option<&str>) -> bool {
    position.is_some_and(|p| p.starts_with("PH") || p.starts_with("PR"))
}

/// "PH-DH-SS" → "PH → DH → SS": the order the player moved through
fn position_sequence(position: Option<&str>) -> String {
    position.unwrap_or_default().split('-').collect::<Vec<_>>().join(" → ")
}

/// True lineup slots from the team's plate-appearance sequence: a lineup
/// cycles strictly, so the n-th plate appearance belongs to slot n mod 9 + 1
/// and anyone whose first PA comes after the first cycle is a substitute.
/// Baserunning-only rows repeat the at-bat's batter, so consecutive
/// duplicates collapse. Returns batter name → (slot, `is_sub`).
fn slots_from_plays(plays: &[PlayDto]) -> std::collections::HashMap<String, (i32, bool)> {
    let mut map = std::collections::HashMap::new();
    let mut pa_index: usize = 0;
    let mut prev_batter: Option<&str> = None;
    for p in plays {
        if prev_batter == Some(p.batter.as_str()) {
            continue;
        }
        prev_batter = Some(p.batter.as_str());
        if !map.contains_key(&p.batter) {
            let slot = i32::try_from(pa_index % 9).unwrap_or(0) + 1;
            map.insert(p.batter.clone(), (slot, pa_index >= 9));
        }
        pa_index += 1;
    }
    map
}

#[component]
fn BattingTable(lines: Vec<BattingLineDto>, team: String, plays: Vec<PlayDto>) -> Element {
    if lines.is_empty() {
        return rsx! {};
    }

    let pbp_slots = slots_from_plays(&plays);

    // Rows arrive in box-score order (grouped by lineup slot). Prefer the
    // play-by-play-derived slot; rows without a plate appearance (defensive
    // subs) inherit the previous row's slot. Fall back to the PH/PR position
    // heuristic when play-by-play isn't available.
    let mut walk_slot = 0;
    let mut prev_slot = 0;
    let rows: Vec<(BattingLineDto, bool, i32)> = lines
        .into_iter()
        .map(|line| {
            let (slot, sub) = if pbp_slots.is_empty() {
                let sub = is_substitute(line.position.as_deref());
                if !sub {
                    walk_slot += 1;
                }
                (walk_slot.max(1), sub)
            } else if let Some(&(slot, sub)) = pbp_slots.get(&line.player) {
                (slot, sub)
            } else {
                // No plate appearance: a defensive sub in the previous slot
                (prev_slot.max(1), true)
            };
            prev_slot = slot;
            (line, sub, slot)
        })
        .collect();

    rsx! {
        h2 { class: "muted", "{team}" }
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { class: "num", "#" }
                        th { "Player" }
                        th { "Pos" }
                        th { class: "num", "AB" }
                        th { class: "num", "R" }
                        th { class: "num", "H" }
                        th { class: "num", "RBI" }
                        th { class: "num", "BB" }
                        th { class: "num", "SO" }
                        th { class: "num", "PA" }
                        th { class: "num", "AVG" }
                        th { class: "num", "OPS" }
                        th { class: "num", "WPA" }
                        th { class: "num", "RE24" }
                        th { class: "num", "PO" }
                        th { class: "num", "A" }
                        th { "Details" }
                    }
                }
                tbody {
                    for (line , sub , slot) in rows {
                        tr { key: "{line.player_id}", class: if sub { "sub-row" } else { "" },
                            td { class: "num muted", if sub { "" } else { "{slot}" } }
                            td { class: if sub { "sub-player" } else { "" },
                                if sub {
                                    span { class: "sub-arrow", "↳ " }
                                }
                                Link { to: Route::PlayerDetail { id: line.player_id }, "{line.player}" }
                            }
                            td { class: "pos-seq", {position_sequence(line.position.as_deref())} }
                            td { class: "num", {fmt::opt(line.ab)} }
                            td { class: "num", {fmt::opt(line.r)} }
                            td { class: "num", {fmt::opt(line.h)} }
                            td { class: "num", {fmt::opt(line.rbi)} }
                            td { class: "num", {fmt::opt(line.bb)} }
                            td { class: "num", {fmt::opt(line.so)} }
                            td { class: "num", {fmt::opt(line.pa)} }
                            td { class: "num", {fmt::rate3(line.avg)} }
                            td { class: "num", {fmt::rate3(line.ops)} }
                            td { class: "num", {fmt::signed2(line.wpa)} }
                            td { class: "num", {fmt::signed2(line.re24)} }
                            td { class: "num", {fmt::opt(line.po)} }
                            td { class: "num", {fmt::opt(line.a)} }
                            td { {line.details.clone().unwrap_or_default()} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PitchingTable(lines: Vec<PitchingLineDto>, team: String) -> Element {
    if lines.is_empty() {
        return rsx! {};
    }
    rsx! {
        h2 { class: "muted", "{team}" }
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { "Player" }
                        th { "Dec" }
                        th { class: "num", "IP" }
                        th { class: "num", "H" }
                        th { class: "num", "R" }
                        th { class: "num", "ER" }
                        th { class: "num", "BB" }
                        th { class: "num", "SO" }
                        th { class: "num", "HR" }
                        th { class: "num", "ERA" }
                        th { class: "num", "BF" }
                        th { class: "num", "Pit" }
                        th { class: "num", "Str" }
                        th { class: "num", "GB" }
                        th { class: "num", "FB" }
                        th { class: "num", "LD" }
                        th { class: "num", "GSc" }
                        th { class: "num", "WPA" }
                    }
                }
                tbody {
                    for line in lines {
                        tr { key: "{line.player_id}-{line.pitch_order.unwrap_or_default()}",
                            td {
                                Link { to: Route::PlayerDetail { id: line.player_id }, "{line.player}" }
                            }
                            td { {line.decision.clone().unwrap_or_default()} }
                            td { class: "num", {fmt::ip(line.ip)} }
                            td { class: "num", {fmt::opt(line.h)} }
                            td { class: "num", {fmt::opt(line.r)} }
                            td { class: "num", {fmt::opt(line.er)} }
                            td { class: "num", {fmt::opt(line.bb)} }
                            td { class: "num", {fmt::opt(line.so)} }
                            td { class: "num", {fmt::opt(line.hr)} }
                            td { class: "num", {fmt::num2(line.era)} }
                            td { class: "num", {fmt::opt(line.batters_faced)} }
                            td { class: "num", {fmt::opt(line.pitches)} }
                            td { class: "num", {fmt::opt(line.strikes)} }
                            td { class: "num", {fmt::opt(line.ground_balls)} }
                            td { class: "num", {fmt::opt(line.fly_balls)} }
                            td { class: "num", {fmt::opt(line.line_drives)} }
                            td { class: "num", {fmt::opt(line.game_score)} }
                            td { class: "num", {fmt::signed2(line.wpa)} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PlayByPlayTable(plays: Vec<PlayDto>) -> Element {
    /// Ordinal inning label: 1st, 2nd, 3rd, 4th…
    fn ordinal(n: i32) -> String {
        let suffix = match (n % 10, n % 100) {
            (1, 11) | (2, 12) | (3, 13) => "th",
            (1, _) => "st",
            (2, _) => "nd",
            (3, _) => "rd",
            _ => "th",
        };
        format!("{n}{suffix}")
    }

    let mut prev_half: Option<(i32, bool)> = None;

    rsx! {
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { "Batter" }
                        th { "Pitcher" }
                        th { class: "num", "Outs" }
                        th { "Bases" }
                        th { class: "num", "Score" }
                        th { class: "num", "Pit" }
                        th { class: "num", "R" }
                        th { class: "num", "WPA" }
                        th { class: "num", "WE%" }
                        th { "Play" }
                    }
                }
                tbody {
                    for p in plays {
                        // Half-inning break row when the frame changes
                        if prev_half != Some((p.inning, p.is_bottom)) {
                            {
                                prev_half = Some((p.inning, p.is_bottom));
                                let half = if p.is_bottom { "▼ Bottom" } else { "▲ Top" };
                                rsx! {
                                    tr { class: "pbp-break", key: "break-{p.inning}-{p.is_bottom}",
                                        td { colspan: 10,
                                            "{half} of the {ordinal(p.inning)} · {p.batting_team} batting"
                                        }
                                    }
                                }
                            }
                        }
                        {
                            let scoring = p.runs_on_play.unwrap_or(0) > 0;
                            let event_only = p
                                .description
                                .as_deref()
                                .is_some_and(crate::server::is_baserunning_only);
                            let row_class = if scoring {
                                "pbp-scoring"
                            } else if event_only {
                                "pbp-event"
                            } else {
                                ""
                            };
                            rsx! {
                                tr { key: "{p.event_num}", class: "{row_class}",
                                    td { "{p.batter}" }
                                    td { "{p.pitcher}" }
                                    td { class: "num", {fmt::opt(p.outs_before)} }
                                    td {
                                        MiniDiamond { runners: p.runners_before.clone() }
                                    }
                                    td { class: "num",
                                        {format!("{}–{}", fmt::score(p.score_batting_team), fmt::score(p.score_fielding_team))}
                                    }
                                    td { class: "num", {fmt::opt(p.pitch_count)} }
                                    td { class: "num pbp-runs",
                                        if scoring {
                                            span { class: "pbp-run-badge", "+{p.runs_on_play.unwrap_or(0)}" }
                                        }
                                    }
                                    td { class: "num", {fmt::signed2(p.wpa)} }
                                    td { class: "num",
                                        {p.win_expectancy_after.map_or_else(String::new, |w| format!("{:.0}%", w * 100.0))}
                                    }
                                    td { {p.description.clone().unwrap_or_default()} }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The plays that scored runs, with the running score after each — the
/// broadcast-style "how the game unfolded" skim
#[component]
fn ScoringSummary(plays: Vec<PlayDto>, home_code: String, away_code: String) -> Element {
    struct ScoringPlay {
        key: i32,
        half_inning: String,
        batter: String,
        description: String,
        away_after: i32,
        home_after: i32,
    }

    let scoring: Vec<ScoringPlay> = plays
        .iter()
        .filter(|p| p.runs_on_play.unwrap_or(0) > 0)
        .map(|p| {
            let batting_is_home = if p.batting_team == home_code {
                true
            } else if p.batting_team == away_code {
                false
            } else {
                p.is_bottom
            };
            let bat_after = p.score_batting_team.unwrap_or(0) + p.runs_on_play.unwrap_or(0);
            let field = p.score_fielding_team.unwrap_or(0);
            let (away_after, home_after) = if batting_is_home {
                (field, bat_after)
            } else {
                (bat_after, field)
            };
            ScoringPlay {
                key: p.event_num,
                half_inning: format!("{}{}", if p.is_bottom { "▼" } else { "▲" }, p.inning),
                batter: p.batter.clone(),
                description: p.description.clone().unwrap_or_default(),
                away_after,
                home_after,
            }
        })
        .collect();

    if scoring.is_empty() {
        return rsx! {};
    }

    rsx! {
        h2 { "Scoring" }
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { "Inn" }
                        th { "Batter" }
                        th { "Play" }
                        th { class: "num", "{away_code}" }
                        th { class: "num", "{home_code}" }
                    }
                }
                tbody {
                    for s in scoring {
                        tr { key: "{s.key}",
                            td { "{s.half_inning}" }
                            td { "{s.batter}" }
                            td { "{s.description}" }
                            td { class: "num", "{s.away_after}" }
                            td { class: "num", b { "{s.home_after}" } }
                        }
                    }
                }
            }
        }
    }
}

/// Scoreboard-style game header: team rows with big scores, winner emphasized
#[component]
fn ScorebugHeader(game: crate::dto::GameSummary) -> Element {
    let away_won = game.away_score.unwrap_or(0) > game.home_score.unwrap_or(0);
    let home_won = game.home_score.unwrap_or(0) > game.away_score.unwrap_or(0);
    let decided = away_won || home_won;

    rsx! {
        div { class: "scorebug",
            div { class: "scorebug-teams",
                div { class: if away_won { "scorebug-row winner" } else { "scorebug-row" },
                    span { class: "scorebug-code",
                        Link { to: Route::TeamDetail { id: game.away.id }, "{game.away.code}" }
                    }
                    span { class: "scorebug-name", "{game.away.name}" }
                    span { class: "scorebug-score", {fmt::score(game.away_score)} }
                }
                div { class: if home_won { "scorebug-row winner" } else { "scorebug-row" },
                    span { class: "scorebug-code",
                        Link { to: Route::TeamDetail { id: game.home.id }, "{game.home.code}" }
                    }
                    span { class: "scorebug-name", "{game.home.name}" }
                    span { class: "scorebug-score", {fmt::score(game.home_score)} }
                }
            }
            div { class: "scorebug-status",
                if decided {
                    span { class: "scorebug-final", "Final" }
                }
                span { class: "muted", "{game.game_date}" }
            }
        }
    }
}

/// Top WPA performers across both box scores — who actually swung the game
#[component]
fn StarsOfTheGame(detail: GameDetailDto) -> Element {
    struct Star {
        player_id: i32,
        name: String,
        context: String,
        wpa: f64,
    }

    let mut stars: Vec<Star> = Vec::new();
    for b in &detail.batting {
        if let Some(wpa) = b.wpa {
            let mut context = format!("{}-{}", b.h.unwrap_or(0), b.ab.unwrap_or(0));
            if let Some(details) = &b.details
                && !details.is_empty()
            {
                context.push_str(", ");
                context.push_str(details);
            }
            stars.push(Star {
                player_id: b.player_id,
                name: b.player.clone(),
                context,
                wpa,
            });
        }
    }
    for p in &detail.pitching {
        if let Some(wpa) = p.wpa {
            stars.push(Star {
                player_id: p.player_id,
                name: p.player.clone(),
                context: format!("{} IP, {} ER", fmt::ip(p.ip), p.er.unwrap_or(0)),
                wpa,
            });
        }
    }
    stars.sort_by(|a, b| b.wpa.total_cmp(&a.wpa));
    stars.truncate(3);
    if stars.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "game-stars",
            span { class: "game-stars-label", "★ Stars" }
            for (i , s) in stars.into_iter().enumerate() {
                span { class: "game-star", key: "{s.player_id}",
                    if i > 0 {
                        span { class: "muted", " · " }
                    }
                    Link { to: Route::PlayerDetail { id: s.player_id }, "{s.name}" }
                    span { class: "muted", " ({s.context} · " }
                    span { class: "game-star-wpa", {fmt::signed2(Some(s.wpa))} }
                    span { class: "muted", ")" }
                }
            }
        }
    }
}
