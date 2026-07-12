use dioxus::prelude::*;

use crate::{app::Route, server};

#[component]
pub fn Teams() -> Element {
    let teams = use_resource(server::list_teams);

    rsx! {
        h1 { "Teams" }
        match &*teams.read() {
            Some(Ok(ts)) => rsx! {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Code" }
                                th { "Name" }
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
                            for t in ts.clone() {
                                tr { key: "{t.team.id}",
                                    td {
                                        Link { to: Route::TeamDetail { id: t.team.id }, "{t.team.code}" }
                                    }
                                    td { "{t.team.name}" }
                                    td { class: "num", "{t.games}" }
                                    td { class: "num", "{t.wins}" }
                                    td { class: "num", "{t.losses}" }
                                    td { class: "num", {win_pct(t.wins, t.losses)} }
                                    td { class: "num", "{t.runs_for}" }
                                    td { class: "num", "{t.runs_against}" }
                                    td { class: "num", "{t.runs_for - t.runs_against}" }
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load teams: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading teams…" }
            },
        }
        div { class: "footnote",
            "All-time totals over scraped seasons; open a team for its season-by-season history."
        }
    }
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
