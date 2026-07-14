use dioxus::prelude::*;

use crate::{
    app::Route,
    components::chart::{HoverInfo, LineChart, Pt, Tick},
    dto::SeasonSummary,
    fmt, server,
};

#[component]
pub fn Seasons() -> Element {
    let seasons = use_resource(server::list_seasons);

    rsx! {
        h1 { "Seasons" }
        match &*seasons.read() {
            Some(Ok(rows)) => rsx! {
                SeasonTrendCharts { rows: rows.clone() }
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

fn decade_ticks(rows: &[SeasonSummary]) -> Vec<Tick> {
    let (Some(min), Some(max)) = (rows.iter().map(|r| r.season).min(), rows.iter().map(|r| r.season).max()) else {
        return Vec::new();
    };
    (min..=max)
        .filter(|y| y % 10 == 0)
        .map(|y| Tick {
            at: f64::from(y),
            label: y.to_string(),
        })
        .collect()
}

fn compact_thousands(v: f64) -> String {
    if v >= 1000.0 {
        format!("{:.0}k", v / 1000.0)
    } else {
        format!("{v:.0}")
    }
}

#[component]
fn SeasonTrendCharts(rows: Vec<SeasonSummary>) -> Element {
    let mut rows = rows;
    rows.sort_by_key(|r| r.season);
    if rows.len() < 2 {
        return rsx! {};
    }
    let x_ticks = decade_ticks(&rows);

    let mut rpg_points = Vec::new();
    let mut rpg_hover = Vec::new();
    let mut att_points = Vec::new();
    let mut att_hover = Vec::new();
    for r in &rows {
        let x = f64::from(r.season);
        if let Some(rpg) = r.runs_per_game {
            rpg_points.push(Pt { x, y: rpg });
            rpg_hover.push(HoverInfo {
                title: r.season.to_string(),
                rows: vec![
                    ("R/G".to_string(), format!("{rpg:.2}")),
                    ("Runs".to_string(), r.runs.to_string()),
                    ("Games".to_string(), r.games.to_string()),
                ],
            });
        }
        if let Some(att) = r.avg_attendance {
            att_points.push(Pt { x, y: att });
            att_hover.push(HoverInfo {
                title: r.season.to_string(),
                rows: vec![
                    ("Avg attendance".to_string(), format!("{att:.0}")),
                    ("Total".to_string(), r.attendance.to_string()),
                    ("Games".to_string(), r.games.to_string()),
                ],
            });
        }
    }

    let att_hi = att_points.iter().map(|p| p.y).fold(1.0f64, f64::max);
    let att_ticks: Vec<Tick> = crate::components::chart::nice_ticks(0.0, att_hi, 4)
        .into_iter()
        .map(|v| Tick {
            at: v,
            label: compact_thousands(v),
        })
        .collect();
    let att_top = att_ticks.iter().map(|t| t.at).fold(att_hi, f64::max);

    rsx! {
        div { class: "chart-row",
            div { class: "chart-frame",
                div { class: "chart-title", "Runs per game by season" }
                LineChart {
                    points: rpg_points,
                    hover: rpg_hover,
                    gap_break: Some(1.5),
                    x_ticks: Some(x_ticks.clone()),
                }
            }
            div { class: "chart-frame",
                div { class: "chart-title", "Avg attendance by season" }
                LineChart {
                    points: att_points,
                    hover: att_hover,
                    gap_break: Some(1.5),
                    x_ticks: Some(x_ticks),
                    y_ticks: Some(att_ticks),
                    y_domain: Some((0.0, att_top)),
                }
            }
        }
    }
}
