use dioxus::prelude::*;

use crate::{app::Route, server};

#[component]
pub fn Players() -> Element {
    let mut query = use_signal(String::new);

    let results = use_resource(move || {
        let q = query();
        async move {
            if q.trim().len() < 2 {
                return Ok(Vec::new());
            }
            server::search_players(q, 50).await
        }
    });

    rsx! {
        h1 { "Players" }
        div { class: "filter-bar",
            div { class: "filter-field",
                label { "Search by name" }
                input {
                    r#type: "search",
                    placeholder: "at least 2 characters…",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
            }
        }
        match &*results.read() {
            Some(Ok(players)) if players.is_empty() && query.read().trim().len() >= 2 => rsx! {
                div { class: "muted", "No players match." }
            },
            Some(Ok(players)) => rsx! {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Name" }
                                th { "BBRef ID" }
                            }
                        }
                        tbody {
                            for p in players.clone() {
                                tr { key: "{p.id}",
                                    td {
                                        Link { to: Route::PlayerDetail { id: p.id }, "{p.name}" }
                                    }
                                    td { class: "muted", "{p.bbref_id}" }
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Search failed: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Searching…" }
            },
        }
    }
}
