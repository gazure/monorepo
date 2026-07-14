use dioxus::prelude::*;

use crate::{
    app::Route,
    components::chart::{HoverInfo, LineChart, Pt, Tick},
    dto::{TeamRosterDto, TeamSeasonRow},
    fmt,
    pages::games::GamesTable,
    server,
};

#[component]
pub fn TeamDetail(id: i32) -> Element {
    let detail = use_resource(move || server::team_detail(id));
    let seasons = use_resource(move || server::team_seasons(id));

    rsx! {
        match &*detail.read() {
            Some(Ok(d)) => {
                let (last10, streak) = recent_form(d.summary.team.id, &d.recent_games);
                rsx! {
                    h1 { "{d.summary.team.name} ({d.summary.team.code})" }
                    div { class: "stat-grid",
                        StatCard { label: "Games", value: d.summary.games.to_string() }
                        StatCard { label: "Record", value: format!("{}–{}", d.summary.wins, d.summary.losses) }
                        StatCard { label: "Win %", value: win_pct(d.summary.wins, d.summary.losses) }
                        if let Some(last10) = last10 {
                            StatCard { label: "Last 10", value: last10 }
                        }
                        if let Some(streak) = streak {
                            StatCard { label: "Streak", value: streak }
                        }
                        StatCard {
                            label: "Run diff",
                            value: format!("{:+}", d.summary.runs_for - d.summary.runs_against),
                        }
                    }
                }
            }
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load team: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading team…" }
            },
        }

        match &*seasons.read() {
            Some(Ok(rows)) if !rows.is_empty() => rsx! {
                FranchiseHistory { rows: rows.clone() }
                RosterSection { team_id: id, seasons: rows.iter().map(|r| r.season).collect::<Vec<_>>() }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load seasons: {e}" }
            },
            _ => rsx! {},
        }

        match &*detail.read() {
            Some(Ok(d)) => rsx! {
                h2 { "Recent games" }
                GamesTable { games: d.recent_games.clone() }
            },
            _ => rsx! {},
        }
    }
}

/// Last-10 record and current streak from the recent games list (newest
/// first); ties and unplayed games are skipped
fn recent_form(team_id: i32, recent: &[crate::dto::GameSummary]) -> (Option<String>, Option<String>) {
    let results: Vec<bool> = recent
        .iter()
        .filter_map(|g| {
            let (hs, aws) = (g.home_score?, g.away_score?);
            if hs == aws {
                return None;
            }
            let team_is_home = g.home.id == team_id;
            Some(if team_is_home { hs > aws } else { aws > hs })
        })
        .collect();

    if results.is_empty() {
        return (None, None);
    }

    let wins = results.iter().take(10).filter(|w| **w).count();
    let sample = results.len().min(10);
    let last10 = format!("{wins}–{}", sample - wins);

    let current = results[0];
    let run = results.iter().take_while(|w| **w == current).count();
    let streak = format!("{}{run}", if current { "W" } else { "L" });

    (Some(last10), Some(streak))
}

fn win_pct(wins: i64, losses: i64) -> String {
    let decided = wins + losses;
    if decided == 0 {
        return String::new();
    }
    #[expect(clippy::cast_precision_loss, reason = "game counts are far below 2^52")]
    let pct = wins as f64 / decided as f64;
    format!("{pct:.3}")
}

#[component]
fn FranchiseHistory(rows: Vec<TeamSeasonRow>) -> Element {
    let mut asc = rows.clone();
    asc.sort_by_key(|r| r.season);

    let mut points = Vec::new();
    let mut hover = Vec::new();
    for r in &asc {
        let decided = r.wins + r.losses;
        if decided == 0 {
            continue;
        }
        #[expect(clippy::cast_precision_loss, reason = "game counts are far below 2^52")]
        let pct = r.wins as f64 / decided as f64;
        points.push(Pt {
            x: f64::from(r.season),
            y: pct,
        });
        hover.push(HoverInfo {
            title: r.season.to_string(),
            rows: vec![
                ("Record".to_string(), format!("{}–{}", r.wins, r.losses)),
                ("PCT".to_string(), format!("{pct:.3}")),
                ("Run diff".to_string(), format!("{:+}", r.runs_for - r.runs_against)),
            ],
        });
    }
    let x_ticks: Vec<Tick> = asc
        .iter()
        .map(|r| r.season)
        .filter(|y| y % 10 == 0)
        .map(|y| Tick {
            at: f64::from(y),
            label: y.to_string(),
        })
        .collect();

    rsx! {
        h2 { "Franchise history" }
        if points.len() >= 3 {
            div { class: "chart-frame",
                div { class: "chart-title", "Win percentage by season (regular season)" }
                LineChart {
                    points,
                    hover,
                    gap_break: Some(1.5),
                    ref_line: Some(0.5),
                    x_ticks: Some(x_ticks),
                }
            }
        }
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { "Season" }
                        th { class: "num", "G" }
                        th { class: "num", "W" }
                        th { class: "num", "L" }
                        th { class: "num", "PCT" }
                        th { class: "num", "RF" }
                        th { class: "num", "RA" }
                        th { class: "num", "Diff" }
                    }
                }
                tbody {
                    for r in rows {
                        tr { key: "{r.season}",
                            td {
                                Link { to: Route::SeasonDetail { year: r.season }, "{r.season}" }
                            }
                            td { class: "num", "{r.games}" }
                            td { class: "num", "{r.wins}" }
                            td { class: "num", "{r.losses}" }
                            td { class: "num", {win_pct(r.wins, r.losses)} }
                            td { class: "num", "{r.runs_for}" }
                            td { class: "num", "{r.runs_against}" }
                            td { class: "num", "{r.runs_for - r.runs_against}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RosterSection(team_id: i32, seasons: Vec<i32>) -> Element {
    let latest = seasons.first().copied().unwrap_or(2024);
    let mut season_sel = use_signal(|| latest);
    let roster = use_resource(move || {
        let s = season_sel();
        async move { server::team_roster(team_id, s).await }
    });

    rsx! {
        h2 { "Roster" }
        div { class: "filter-bar",
            div { class: "filter-field",
                label { "Season" }
                select {
                    onchange: move |e| {
                        if let Ok(y) = e.value().parse::<i32>() {
                            season_sel.set(y);
                        }
                    },
                    for year in seasons {
                        option { value: "{year}", selected: season_sel() == year, "{year}" }
                    }
                }
            }
        }
        match &*roster.read() {
            Some(Ok(r)) => rsx! {
                RosterTables { roster: r.clone() }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load roster: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading roster…" }
            },
        }
    }
}

#[component]
fn RosterTables(roster: TeamRosterDto) -> Element {
    rsx! {
        if !roster.batters.is_empty() {
            h2 { class: "muted", "Batting" }
            div { class: "table-scroll",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "Player" }
                            th { class: "num", "G" }
                            th { class: "num", "PA" }
                            th { class: "num", "H" }
                            th { class: "num", "HR" }
                            th { class: "num", "SB" }
                            th { class: "num", "AVG" }
                            th { class: "num", "OBP" }
                            th { class: "num", "SLG" }
                            th { class: "num", "OPS" }
                        }
                    }
                    tbody {
                        for b in roster.batters {
                            tr { key: "{b.player_id}",
                                td {
                                    Link { to: Route::PlayerDetail { id: b.player_id }, "{b.name}" }
                                }
                                td { class: "num", "{b.games}" }
                                td { class: "num", "{b.pa}" }
                                td { class: "num", "{b.h}" }
                                td { class: "num", "{b.home_runs}" }
                                td { class: "num", "{b.stolen_bases}" }
                                td { class: "num", {fmt::rate3(b.avg)} }
                                td { class: "num", {fmt::rate3(b.obp)} }
                                td { class: "num", {fmt::rate3(b.slg)} }
                                td { class: "num", {fmt::rate3(b.ops)} }
                            }
                        }
                    }
                }
            }
        }
        if !roster.pitchers.is_empty() {
            h2 { class: "muted", "Pitching" }
            div { class: "table-scroll",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "Player" }
                            th { class: "num", "G" }
                            th { class: "num", "W–L" }
                            th { class: "num", "SV" }
                            th { class: "num", "IP" }
                            th { class: "num", "SO" }
                            th { class: "num", "ERA" }
                            th { class: "num", "WHIP" }
                        }
                    }
                    tbody {
                        for p in roster.pitchers {
                            tr { key: "{p.player_id}",
                                td {
                                    Link { to: Route::PlayerDetail { id: p.player_id }, "{p.name}" }
                                }
                                td { class: "num", "{p.games}" }
                                td { class: "num", "{p.wins}–{p.losses}" }
                                td { class: "num", "{p.saves}" }
                                td { class: "num", {crate::dto::format_ip(p.outs)} }
                                td { class: "num", "{p.so}" }
                                td { class: "num", {fmt::num2(p.era)} }
                                td { class: "num", {fmt::num2(p.whip)} }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(label: String, value: String) -> Element {
    rsx! {
        div { class: "stat-card",
            div { class: "stat-value", "{value}" }
            div { class: "stat-label", "{label}" }
        }
    }
}
