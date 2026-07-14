use dioxus::prelude::*;

use crate::{app::Route, components::Pagination, dto::PlayerBrowseSort, fmt, server};

const PAGE_SIZE: u32 = 50;

#[component]
pub fn Players() -> Element {
    let mut query = use_signal(String::new);
    let mut sort = use_signal(PlayerBrowseSort::default);
    let mut page = use_signal(|| 0u32);

    let results = use_resource(move || {
        let q = query();
        async move {
            if q.trim().len() < 2 {
                return Ok(Vec::new());
            }
            server::search_players(q, 50).await
        }
    });

    let browse = use_resource(move || {
        let (s, p) = (sort(), page());
        async move { server::browse_players(s, p, PAGE_SIZE).await }
    });

    let searching = query.read().trim().len() >= 2;

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
            if !searching {
                div { class: "filter-field",
                    label { "Sort" }
                    select {
                        onchange: move |e| {
                            if let Ok(i) = e.value().parse::<usize>()
                                && let Some(s) = PlayerBrowseSort::ALL.get(i)
                            {
                                sort.set(*s);
                                page.set(0);
                            }
                        },
                        for (i , s) in PlayerBrowseSort::ALL.iter().enumerate() {
                            option { value: "{i}", selected: sort() == *s, "{s.label()}" }
                        }
                    }
                }
            }
        }

        if searching {
            match &*results.read() {
                Some(Ok(players)) if players.is_empty() => rsx! {
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
        } else {
            match &*browse.read() {
                Some(Ok(pg)) => rsx! {
                    div { class: "table-scroll",
                        table { class: "data-table",
                            thead {
                                tr {
                                    th { "#" }
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
                                for (i , row) in pg.items.clone().into_iter().enumerate() {
                                    tr { key: "{row.player_id}",
                                        td { class: "num muted",
                                            "{i + 1 + usize::try_from(page() * PAGE_SIZE).unwrap_or(0)}"
                                        }
                                        td {
                                            Link { to: Route::PlayerDetail { id: row.player_id }, "{row.name}" }
                                        }
                                        td { class: "num", "{row.games}" }
                                        td { class: "num", "{row.pa}" }
                                        td { class: "num", "{row.h}" }
                                        td { class: "num", "{row.home_runs}" }
                                        td { class: "num", "{row.stolen_bases}" }
                                        td { class: "num", {fmt::rate3(row.avg)} }
                                        td { class: "num", {fmt::rate3(row.obp)} }
                                        td { class: "num", {fmt::rate3(row.slg)} }
                                        td { class: "num", {fmt::rate3(row.ops)} }
                                    }
                                }
                            }
                        }
                    }
                    Pagination { page, total_pages: pg.total_pages(), total: pg.total }
                    div { class: "footnote",
                        "All-time career batting totals across scraped seasons (regular season + postseason)."
                    }
                },
                Some(Err(e)) => rsx! {
                    div { class: "error-box", "Failed to load players: {e}" }
                },
                None => rsx! {
                    div { class: "loading", "Loading players…" }
                },
            }
        }
    }
}
