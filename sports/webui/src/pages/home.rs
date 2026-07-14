use dioxus::prelude::*;

use crate::{app::Route, dto::DashboardStats, pages::games::GamesTable, server};

#[component]
pub fn Home() -> Element {
    let stats = use_resource(server::dashboard_stats);
    let recent = use_resource(|| server::recent_games(10));

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
