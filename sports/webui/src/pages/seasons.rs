use dioxus::prelude::*;

use crate::{app::Route, fmt, server};

#[component]
pub fn Seasons() -> Element {
    let seasons = use_resource(server::list_seasons);

    rsx! {
        h1 { "Seasons" }
        match &*seasons.read() {
            Some(Ok(rows)) => rsx! {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Season" }
                                th { class: "num", "Teams" }
                                th { class: "num", "Games" }
                                th { class: "num", "Runs" }
                                th { class: "num", "R/G" }
                                th { class: "num", "Attendance" }
                                th { class: "num", "Avg att." }
                                th { class: "num", "PS games" }
                            }
                        }
                        tbody {
                            for s in rows.clone() {
                                tr { key: "{s.season}",
                                    td {
                                        Link { to: Route::SeasonDetail { year: s.season }, "{s.season}" }
                                    }
                                    td { class: "num", "{s.teams}" }
                                    td { class: "num", "{s.games}" }
                                    td { class: "num", "{s.runs}" }
                                    td { class: "num", {fmt::num2(s.runs_per_game)} }
                                    td { class: "num", "{s.attendance}" }
                                    td { class: "num",
                                        {s.avg_attendance.map_or_else(String::new, |a| format!("{a:.0}"))}
                                    }
                                    td { class: "num", "{s.postseason_games}" }
                                }
                            }
                        }
                    }
                }
                div { class: "footnote",
                    "Postseason is detected as games after each year's last date with 6+ games league-wide."
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load seasons: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading seasons…" }
            },
        }
    }
}
