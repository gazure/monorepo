use dioxus::prelude::*;

use crate::{
    app::Route,
    components::chart::{Bar, BarChart, HoverInfo, Sparkline},
    dto::{DashboardStats, DramaticGame, SeasonGamesCount},
    fmt,
    pages::games::GamesTable,
    server,
};

#[component]
pub fn Home() -> Element {
    let stats = use_resource(server::dashboard_stats);
    let recent = use_resource(|| server::recent_games(10));
    let coverage = use_resource(server::games_per_season);
    let classics = use_resource(|| server::dramatic_games(6));

    rsx! {
        h1 { "Sports Database Explorer" }
        match &*stats.read() {
            Some(Ok(s)) => rsx! {
                StatCards { stats: s.clone() }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load stats: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading stats…" }
            },
        }
        match &*classics.read() {
            Some(Ok(games)) if !games.is_empty() => rsx! {
                h2 { "Instant classics" }
                div { class: "muted", "The wildest of the last 300 games, by total win-probability swing." }
                div { class: "classic-grid",
                    for c in games.clone() {
                        ClassicCard { classic: c }
                    }
                }
            },
            _ => rsx! {},
        }
        match &*coverage.read() {
            Some(Ok(rows)) if rows.len() > 1 => rsx! {
                CoverageChart { rows: rows.clone() }
            },
            _ => rsx! {},
        }
        h2 { "Recent games" }
        match &*recent.read() {
            Some(Ok(games)) => rsx! {
                GamesTable { games: games.clone() }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load recent games: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading…" }
            },
        }
        div { class: "footnote",
            "Explore the data via "
            Link { to: Route::Games {}, "Games" }
            ", "
            Link { to: Route::Leaderboards { season: None }, "Leaderboards" }
            " or the "
            Link { to: Route::SqlConsole {}, "SQL console" }
            "."
        }
    }
}

#[component]
fn StatCards(stats: DashboardStats) -> Element {
    let coverage = match (stats.first_game, stats.last_game) {
        (Some(first), Some(last)) => format!("{first} → {last}"),
        _ => "no games yet".to_string(),
    };
    rsx! {
        div { class: "stat-grid",
            StatCard { label: "Games", value: stats.games }
            StatCard { label: "Players", value: stats.players }
            StatCard { label: "Teams", value: stats.teams }
            StatCard { label: "Batting lines", value: stats.batting_lines }
            StatCard { label: "Pitching lines", value: stats.pitching_lines }
            StatCard { label: "Plays", value: stats.plays }
        }
        div { class: "muted", "Coverage: {coverage}" }
    }
}

#[component]
fn StatCard(label: String, value: i64) -> Element {
    rsx! {
        div { class: "stat-card",
            div { class: "stat-value", "{value}" }
            div { class: "stat-label", "{label}" }
        }
    }
}

#[component]
fn ClassicCard(classic: DramaticGame) -> Element {
    let g = &classic.game;
    let swing = classic.swing / 100.0;
    rsx! {
        Link { to: Route::GameDetail { id: g.id }, class: "classic-card",
            div { class: "classic-head",
                span { class: "classic-matchup",
                    "{g.away.code} {fmt::score(g.away_score)} @ {g.home.code} {fmt::score(g.home_score)}"
                }
                span { class: "classic-date", "{g.game_date}" }
            }
            Sparkline { values: classic.we_home.clone(), ref_value: Some(0.5) }
            div { class: "classic-swing", title: "Sum of every play's win-probability change",
                "±{swing:.1} total swing"
            }
        }
    }
}

#[component]
fn CoverageChart(rows: Vec<SeasonGamesCount>) -> Element {
    let (Some(min), Some(max)) = (rows.iter().map(|r| r.season).min(), rows.iter().map(|r| r.season).max()) else {
        return rsx! {};
    };

    // Fill missing seasons with zero-height bars so unscraped years show as holes
    let bars: Vec<Bar> = (min..=max)
        .map(|year| {
            let games = rows.iter().find(|r| r.season == year).map_or(0, |r| r.games);
            Bar {
                label: year.to_string(),
                value: crate::components::chart::index_f64(usize::try_from(games).unwrap_or(0)),
                info: HoverInfo {
                    title: year.to_string(),
                    rows: vec![("Games".to_string(), games.to_string())],
                },
            }
        })
        .collect();

    rsx! {
        div { class: "chart-frame",
            div { class: "chart-title", "Games per season" }
            BarChart { bars, height: 180.0 }
            div { class: "footnote", "Gaps are seasons not yet scraped." }
        }
    }
}
