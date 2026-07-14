use dioxus::prelude::*;

use crate::{pages::games::GamesTable, server};

#[component]
pub fn TeamDetail(id: i32) -> Element {
    let detail = use_resource(move || server::team_detail(id));

    rsx! {
        match &*detail.read() {
            Some(Ok(d)) => rsx! {
                h1 { "{d.summary.team.name} ({d.summary.team.code})" }
                div { class: "stat-grid",
                    StatCard { label: "Games", value: d.summary.games.to_string() }
                    StatCard { label: "Record", value: format!("{}–{}", d.summary.wins, d.summary.losses) }
                    StatCard { label: "Runs for", value: d.summary.runs_for.to_string() }
                    StatCard { label: "Runs against", value: d.summary.runs_against.to_string() }
                    StatCard {
                        label: "Run diff",
                        value: format!("{:+}", d.summary.runs_for - d.summary.runs_against),
                    }
                }
                h2 { "Recent games" }
                GamesTable { games: d.recent_games.clone() }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load team: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading team…" }
            },
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
