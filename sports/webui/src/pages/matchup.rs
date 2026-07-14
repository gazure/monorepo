use dioxus::prelude::*;

use crate::{app::Route, fmt, server};

#[component]
pub fn Matchup(batter: Option<i32>, pitcher: Option<i32>) -> Element {
    let batter_sel = use_signal(|| batter);
    let pitcher_sel = use_signal(|| pitcher);

    let result = use_resource(move || {
        let pair = batter_sel().zip(pitcher_sel());
        async move {
            match pair {
                Some((b, p)) => Some(server::matchup(b, p).await),
                None => None,
            }
        }
    });

    rsx! {
        h1 { "Batter vs pitcher" }
        div { class: "filter-bar",
            PlayerPicker { label: "Batter", selected: batter_sel }
            PlayerPicker { label: "Pitcher", selected: pitcher_sel }
        }

        match &*result.read() {
            Some(Some(Ok(m))) => rsx! {
                h2 { "{m.batter} vs {m.pitcher}" }
                if m.tally.pa == 0 {
                    div { class: "muted", "These two never faced each other in the scraped data." }
                } else {
                    div { class: "stat-grid",
                        MatchupCard { label: "PA", value: m.tally.pa.to_string() }
                        MatchupCard { label: "H", value: m.tally.hits().to_string() }
                        MatchupCard { label: "2B", value: m.tally.doubles.to_string() }
                        MatchupCard { label: "3B", value: m.tally.triples.to_string() }
                        MatchupCard { label: "HR", value: m.tally.home_runs.to_string() }
                        MatchupCard { label: "BB", value: m.tally.walks.to_string() }
                        MatchupCard { label: "SO", value: m.tally.strikeouts.to_string() }
                    }
                    h2 { "Every plate appearance" }
                    div { class: "table-scroll",
                        table { class: "data-table",
                            thead {
                                tr {
                                    th { "Date" }
                                    th { "Inn" }
                                    th { "Result" }
                                    th { class: "num", "WPA" }
                                    th { "" }
                                }
                            }
                            tbody {
                                for (i , e) in m.events.iter().enumerate() {
                                    tr { key: "{i}",
                                        td { "{e.game_date}" }
                                        td { {format!("{}{}", if e.is_bottom { "b" } else { "t" }, e.inning)} }
                                        td { {e.description.clone().unwrap_or_default()} }
                                        td { class: "num", {fmt::signed2(e.wpa)} }
                                        td {
                                            Link { to: Route::GameDetail { id: e.game_id }, "box" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "footnote",
                        "Outcomes classified from play descriptions; walks include intentional."
                    }
                }
            },
            Some(Some(Err(e))) => rsx! {
                div { class: "error-box", "Failed to load matchup: {e}" }
            },
            Some(None) => rsx! {
                div { class: "muted", "Pick a batter and a pitcher to see their head-to-head history." }
            },
            None => rsx! {
                div { class: "loading", "Loading…" }
            },
        }
    }
}

/// Search-as-you-type player selector; stores the chosen player id
#[component]
fn PlayerPicker(label: String, selected: Signal<Option<i32>>) -> Element {
    let mut query = use_signal(String::new);
    let mut chosen_name = use_signal(String::new);

    let hits = use_resource(move || {
        let q = query();
        async move {
            if q.trim().len() < 2 {
                return Ok(Vec::new());
            }
            server::search_players(q, 8).await
        }
    });

    rsx! {
        div { class: "filter-field picker",
            label { "{label}" }
            input {
                r#type: "search",
                placeholder: "search players…",
                value: "{query}",
                oninput: move |e| query.set(e.value()),
            }
            if !chosen_name.read().is_empty() {
                div { class: "picker-chosen", "{chosen_name}" }
            }
            if query.read().trim().len() >= 2 {
                match &*hits.read() {
                    Some(Ok(players)) if !players.is_empty() => rsx! {
                        div { class: "picker-results",
                            for p in players.clone() {
                                button {
                                    key: "{p.id}",
                                    class: "picker-hit",
                                    onclick: {
                                        let mut selected = selected;
                                        let name = p.name.clone();
                                        move |_| {
                                            selected.set(Some(p.id));
                                            chosen_name.set(name.clone());
                                            query.set(String::new());
                                        }
                                    },
                                    "{p.name}"
                                }
                            }
                        }
                    },
                    _ => rsx! {},
                }
            }
        }
    }
}

#[component]
fn MatchupCard(label: String, value: String) -> Element {
    rsx! {
        div { class: "stat-card",
            div { class: "stat-value", "{value}" }
            div { class: "stat-label", "{label}" }
        }
    }
}
