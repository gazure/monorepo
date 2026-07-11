use chrono::NaiveDate;
use dioxus::prelude::*;

use crate::{
    app::Route,
    components::Pagination,
    dto::{GameSummary, GamesFilter},
    fmt, server,
};

const PAGE_SIZE: u32 = 50;

#[component]
pub fn Games() -> Element {
    let mut date_from = use_signal(String::new);
    let mut date_to = use_signal(String::new);
    let mut team_id = use_signal(String::new);
    let mut min_runs = use_signal(String::new);
    let mut night_only = use_signal(|| false);

    let mut applied = use_signal(GamesFilter::default);
    let mut page = use_signal(|| 0u32);

    let teams = use_resource(server::team_options);

    let games = use_resource(move || {
        let filter = applied();
        let p = page();
        async move { server::list_games(filter, p, PAGE_SIZE).await }
    });

    let apply = move |_| {
        applied.set(GamesFilter {
            date_from: NaiveDate::parse_from_str(&date_from(), "%Y-%m-%d").ok(),
            date_to: NaiveDate::parse_from_str(&date_to(), "%Y-%m-%d").ok(),
            team_id: team_id().trim().parse().ok(),
            min_total_runs: min_runs().trim().parse().ok(),
            night_games: night_only().then_some(true),
        });
        page.set(0);
    };

    let team_list = match &*teams.read() {
        Some(Ok(ts)) => ts.clone(),
        _ => Vec::new(),
    };

    rsx! {
        h1 { "Games" }
        div { class: "filter-bar",
            div { class: "filter-field",
                label { "From" }
                input {
                    r#type: "date",
                    value: "{date_from}",
                    oninput: move |e| date_from.set(e.value()),
                }
            }
            div { class: "filter-field",
                label { "To" }
                input {
                    r#type: "date",
                    value: "{date_to}",
                    oninput: move |e| date_to.set(e.value()),
                }
            }
            div { class: "filter-field",
                label { "Team" }
                select { onchange: move |e| team_id.set(e.value()),
                    option { value: "", "All teams" }
                    for t in team_list {
                        option { value: "{t.id}", "{t.code} — {t.name}" }
                    }
                }
            }
            div { class: "filter-field",
                label { "Min total runs" }
                input {
                    r#type: "number",
                    min: "0",
                    value: "{min_runs}",
                    oninput: move |e| min_runs.set(e.value()),
                }
            }
            div { class: "filter-field",
                label { "Night games only" }
                input {
                    r#type: "checkbox",
                    checked: night_only(),
                    onchange: move |e| night_only.set(e.checked()),
                }
            }
            button { onclick: apply, "Apply" }
        }
        match &*games.read() {
            Some(Ok(pg)) => rsx! {
                GamesTable { games: pg.items.clone() }
                Pagination { page, total_pages: pg.total_pages(), total: pg.total }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load games: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading games…" }
            },
        }
    }
}

#[component]
pub fn GamesTable(games: Vec<GameSummary>) -> Element {
    rsx! {
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { "Date" }
                        th { "Matchup" }
                        th { class: "num", "Score" }
                        th { "Venue" }
                        th { class: "num", "Att." }
                        th { "" }
                    }
                }
                tbody {
                    for g in games {
                        tr { key: "{g.id}",
                            td { "{g.game_date}" }
                            td {
                                Link { to: Route::TeamDetail { id: g.away.id }, "{g.away.code}" }
                                " @ "
                                Link { to: Route::TeamDetail { id: g.home.id }, "{g.home.code}" }
                            }
                            td { class: "num", "{fmt::score(g.away_score)}–{fmt::score(g.home_score)}" }
                            td { {g.venue.clone().unwrap_or_default()} }
                            td { class: "num", {fmt::opt(g.attendance)} }
                            td {
                                Link { to: Route::GameDetail { id: g.id }, "box score" }
                            }
                        }
                    }
                }
            }
        }
    }
}
