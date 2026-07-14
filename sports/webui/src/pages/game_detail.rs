use dioxus::prelude::*;

use crate::{
    app::Route,
    bbref,
    components::replay::ReplayDeck,
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
    let title = format!(
        "{} {} @ {} {} — {}",
        g.away.code,
        fmt::score(g.away_score),
        g.home.code,
        fmt::score(g.home_score),
        g.game_date
    );
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
        h1 { "{title}" }
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
        BattingTable {
            lines: detail.batting.iter().filter(|b| b.team_code == g.away.code).cloned().collect::<Vec<_>>(),
            team: g.away.name.clone(),
        }
        BattingTable {
            lines: detail.batting.iter().filter(|b| b.team_code == g.home.code).cloned().collect::<Vec<_>>(),
            team: g.home.name.clone(),
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

#[component]
fn BattingTable(lines: Vec<BattingLineDto>, team: String) -> Element {
    if lines.is_empty() {
        return rsx! {};
    }
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
                    for line in lines {
                        tr { key: "{line.player_id}",
                            td { class: "num", {fmt::opt(line.batting_order)} }
                            td {
                                Link { to: Route::PlayerDetail { id: line.player_id }, "{line.player}" }
                            }
                            td { {line.position.clone().unwrap_or_default()} }
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
    rsx! {
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { "Inn" }
                        th { "Team" }
                        th { "Batter" }
                        th { "Pitcher" }
                        th { class: "num", "Outs" }
                        th { "Runners" }
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
                        tr { key: "{p.event_num}",
                            td { {format!("{}{}", if p.is_bottom { "b" } else { "t" }, p.inning)} }
                            td { "{p.batting_team}" }
                            td { "{p.batter}" }
                            td { "{p.pitcher}" }
                            td { class: "num", {fmt::opt(p.outs_before)} }
                            td { {p.runners_before.clone().unwrap_or_default()} }
                            td { class: "num",
                                {format!("{}–{}", fmt::score(p.score_batting_team), fmt::score(p.score_fielding_team))}
                            }
                            td { class: "num", {fmt::opt(p.pitch_count)} }
                            td { class: "num", {fmt::opt(p.runs_on_play)} }
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
