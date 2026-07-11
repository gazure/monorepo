use dioxus::prelude::*;

use crate::{
    app::Route,
    dto::{BattingLeaderboardReq, BattingSort, PitchingLeaderboardReq, PitchingSort, format_ip},
    fmt, server,
};

const LIMIT: u32 = 50;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Batting,
    Pitching,
}

#[component]
pub fn Leaderboards(season: Option<i32>) -> Element {
    let mut tab = use_signal(|| Tab::Batting);
    let mut season_sel = use_signal(|| season);
    let seasons = use_resource(server::list_seasons);

    let season_years: Vec<i32> = match &*seasons.read() {
        Some(Ok(rows)) => rows.iter().map(|s| s.season).collect(),
        _ => Vec::new(),
    };

    rsx! {
        h1 { "Leaderboards" }
        div { class: "tabs",
            button {
                class: if tab() == Tab::Batting { "active" } else { "" },
                onclick: move |_| tab.set(Tab::Batting),
                "Batting"
            }
            button {
                class: if tab() == Tab::Pitching { "active" } else { "" },
                onclick: move |_| tab.set(Tab::Pitching),
                "Pitching"
            }
            select {
                onchange: move |e| season_sel.set(e.value().parse().ok()),
                option { value: "", selected: season_sel().is_none(), "All seasons" }
                for year in season_years {
                    option { value: "{year}", selected: season_sel() == Some(year), "{year}" }
                }
            }
        }
        if tab() == Tab::Batting {
            BattingBoard { season: season_sel }
        } else {
            PitchingBoard { season: season_sel }
        }
        div { class: "footnote",
            "Regular season only. OBP is approximated as (H+BB)/PA — HBP and sacrifice flies are not scraped. SLG is an AB-weighted average of per-game SLG. Click a column header to sort."
        }
    }
}

#[component]
fn BattingBoard(season: Signal<Option<i32>>) -> Element {
    let mut sort = use_signal(BattingSort::default);
    let mut min_pa = use_signal(|| String::from("50"));

    let rows = use_resource(move || {
        let req = BattingLeaderboardReq {
            sort: sort(),
            min_pa: min_pa().trim().parse().unwrap_or(0),
            season: season(),
            limit: LIMIT,
            offset: 0,
        };
        async move { server::batting_leaderboard(req).await }
    });

    rsx! {
        div { class: "filter-bar",
            div { class: "filter-field",
                label { "Min PA" }
                input {
                    r#type: "number",
                    min: "0",
                    value: "{min_pa}",
                    oninput: move |e| min_pa.set(e.value()),
                }
            }
        }
        match &*rows.read() {
            Some(Ok(leaders)) => rsx! {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "#" }
                                th { "Player" }
                                th { class: "num", "G" }
                                for s in BattingSort::ALL {
                                    th {
                                        class: if sort() == s { "num sortable sorted" } else { "num sortable" },
                                        onclick: move |_| sort.set(s),
                                        "{s.label()}"
                                    }
                                }
                            }
                        }
                        tbody {
                            for (i , row) in leaders.clone().into_iter().enumerate() {
                                tr { key: "{row.player_id}",
                                    td { class: "num muted", "{i + 1}" }
                                    td {
                                        Link { to: Route::PlayerDetail { id: row.player_id }, "{row.name}" }
                                    }
                                    td { class: "num", "{row.games}" }
                                    td { class: "num", {fmt::rate3(row.ops)} }
                                    td { class: "num", {fmt::rate3(row.avg)} }
                                    td { class: "num", {fmt::rate3(row.obp)} }
                                    td { class: "num", {fmt::rate3(row.slg)} }
                                    td { class: "num", "{row.h}" }
                                    td { class: "num", "{row.r}" }
                                    td { class: "num", "{row.rbi}" }
                                    td { class: "num", "{row.bb}" }
                                    td { class: "num", "{row.so}" }
                                    td { class: "num", "{row.pa}" }
                                    td { class: "num", {fmt::signed2(row.wpa)} }
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load leaderboard: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading…" }
            },
        }
    }
}

#[component]
fn PitchingBoard(season: Signal<Option<i32>>) -> Element {
    let mut sort = use_signal(PitchingSort::default);
    let mut min_ip = use_signal(|| String::from("20"));

    let rows = use_resource(move || {
        let min_innings: i64 = min_ip().trim().parse().unwrap_or(0);
        let req = PitchingLeaderboardReq {
            sort: sort(),
            min_outs: min_innings * 3,
            season: season(),
            limit: LIMIT,
            offset: 0,
        };
        async move { server::pitching_leaderboard(req).await }
    });

    rsx! {
        div { class: "filter-bar",
            div { class: "filter-field",
                label { "Min IP" }
                input {
                    r#type: "number",
                    min: "0",
                    value: "{min_ip}",
                    oninput: move |e| min_ip.set(e.value()),
                }
            }
        }
        match &*rows.read() {
            Some(Ok(leaders)) => rsx! {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "#" }
                                th { "Player" }
                                th { class: "num", "G" }
                                th { class: "num", "W–L" }
                                for s in PitchingSort::ALL {
                                    th {
                                        class: if sort() == s { "num sortable sorted" } else { "num sortable" },
                                        onclick: move |_| sort.set(s),
                                        "{s.label()}"
                                    }
                                }
                            }
                        }
                        tbody {
                            for (i , row) in leaders.clone().into_iter().enumerate() {
                                tr { key: "{row.player_id}",
                                    td { class: "num muted", "{i + 1}" }
                                    td {
                                        Link { to: Route::PlayerDetail { id: row.player_id }, "{row.name}" }
                                    }
                                    td { class: "num", "{row.games}" }
                                    td { class: "num", "{row.wins}–{row.losses}" }
                                    td { class: "num", {fmt::num2(row.era)} }
                                    td { class: "num", {fmt::num2(row.whip)} }
                                    td { class: "num", "{row.so}" }
                                    td { class: "num", {format_ip(row.outs)} }
                                    td { class: "num", "{row.wins}" }
                                    td { class: "num", "{row.saves}" }
                                    td { class: "num", "{row.bb}" }
                                    td { class: "num", "{row.hr}" }
                                    td { class: "num", {fmt::signed2(row.wpa)} }
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load leaderboard: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading…" }
            },
        }
    }
}
