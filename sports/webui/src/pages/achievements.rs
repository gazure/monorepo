use dioxus::prelude::*;

use crate::{app::Route, dto::FeatRow, server};

/// Curated notable-game feats: no-hitters, high-strikeout starts, cycles,
/// and four-homer games across every scraped season.
#[component]
pub fn Achievements() -> Element {
    let feats = use_resource(server::achievements);

    rsx! {
        h1 { "Feats" }
        div { class: "muted",
            "One-game achievements detected from box scores across every scraped season."
        }
        match &*feats.read() {
            Some(Ok(dto)) => rsx! {
                NoHitters { rows: dto.no_hitters.clone() }
                FeatSection {
                    title: format!("High-strikeout games ({} pitchers with 18+ K)", dto.k_games.len()),
                    header: "Line",
                    rows: dto.k_games.clone(),
                }
                FeatSection {
                    title: format!("Four-homer games ({})", dto.hr_games.len()),
                    header: "Line",
                    rows: dto.hr_games.clone(),
                }
                FeatSection {
                    title: format!("Cycles ({})", dto.cycles.len()),
                    header: "Line",
                    rows: dto.cycles.clone(),
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load feats: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Scanning 76 seasons of box scores…" }
            },
        }
    }
}

#[component]
fn NoHitters(rows: Vec<crate::dto::NoHitterRow>) -> Element {
    if rows.is_empty() {
        return rsx! {};
    }
    let count = rows.len();
    let perfect = rows.iter().filter(|r| r.perfect).count();
    rsx! {
        section { class: "feat-section",
            h2 { "No-hitters ({count}, {perfect} perfect)" }
            div { class: "table-scroll feat-scroll",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "Date" }
                            th { "Pitchers" }
                            th { "Team" }
                            th { "Against" }
                            th { class: "num", "BB" }
                            th { "" }
                        }
                    }
                    tbody {
                        for r in rows {
                            tr { key: "{r.game_id}-{r.team}",
                                td {
                                    Link { to: Route::GameDetail { id: r.game_id }, "{r.game_date}" }
                                }
                                td { "{r.pitchers}" }
                                td { "{r.team}" }
                                td { "{r.opponent}" }
                                td { class: "num", "{r.walks}" }
                                td {
                                    if r.perfect {
                                        span { class: "feat-badge", "PERFECT GAME" }
                                    } else if r.pitchers.contains(',') {
                                        span { class: "feat-badge feat-badge-combined", "COMBINED" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FeatSection(title: String, header: &'static str, rows: Vec<FeatRow>) -> Element {
    if rows.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "feat-section",
            h2 { "{title}" }
            div { class: "table-scroll feat-scroll",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "Date" }
                            th { "Player" }
                            th { "Team" }
                            th { "Against" }
                            th { "{header}" }
                        }
                    }
                    tbody {
                        for r in rows {
                            tr { key: "{r.game_id}-{r.player_id}",
                                td {
                                    Link { to: Route::GameDetail { id: r.game_id }, "{r.game_date}" }
                                }
                                td {
                                    Link { to: Route::PlayerDetail { id: r.player_id }, "{r.name}" }
                                }
                                td { "{r.team}" }
                                td { "{r.opponent}" }
                                td { b { "{r.line}" } }
                            }
                        }
                    }
                }
            }
        }
    }
}
