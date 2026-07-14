use dioxus::prelude::*;

use crate::{app::Route, dto::RecordBoard, server};

/// Best single seasons ever, board by board, with a decade filter
#[component]
pub fn Records() -> Element {
    let mut decade = use_signal(|| None::<i32>);
    let boards = use_resource(move || {
        let d = decade();
        async move { server::single_season_records(d).await }
    });

    let decades: Vec<i32> = (1950..=2020).step_by(10).collect();

    rsx! {
        h1 { "Single-season records" }
        div { class: "filter-bar",
            div { class: "filter-field",
                label { "Era" }
                select {
                    onchange: move |e| decade.set(e.value().parse().ok()),
                    option { value: "", selected: decade().is_none(), "All time" }
                    for d in decades {
                        option { value: "{d}", selected: decade() == Some(d), "{d}s" }
                    }
                }
            }
        }
        match &*boards.read() {
            Some(Ok(boards)) => rsx! {
                div { class: "records-grid",
                    for board in boards.clone() {
                        RecordTable { board }
                    }
                }
                div { class: "footnote",
                    "Regular season only. AVG and OPS boards require 400 plate appearances."
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load records: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading records…" }
            },
        }
    }
}

fn board_title(key: &str) -> &'static str {
    match key {
        "hr" => "Home runs",
        "sb" => "Stolen bases",
        "h" => "Hits",
        "rbi" => "Runs batted in",
        "avg" => "Batting average",
        "ops" => "OPS",
        _ => "Records",
    }
}

#[component]
fn RecordTable(board: RecordBoard) -> Element {
    if board.rows.is_empty() {
        return rsx! {};
    }
    rsx! {
        div {
            h2 { class: "muted", {board_title(&board.key)} }
            div { class: "table-scroll",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "#" }
                            th { "Player" }
                            th { "Season" }
                            th { class: "num", "Value" }
                        }
                    }
                    tbody {
                        for (i , r) in board.rows.into_iter().enumerate() {
                            tr { key: "{r.player_id}-{r.season}",
                                td { class: "num muted", "{i + 1}" }
                                td {
                                    Link { to: Route::PlayerDetail { id: r.player_id }, "{r.name}" }
                                }
                                td {
                                    Link { to: Route::SeasonDetail { year: r.season }, "{r.season}" }
                                }
                                td { class: "num", b { "{r.value}" } }
                            }
                        }
                    }
                }
            }
        }
    }
}
