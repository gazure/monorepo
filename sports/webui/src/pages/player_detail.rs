use dioxus::prelude::*;

use crate::{
    app::Route,
    components::Pagination,
    dto::{BattingSeasonRow, BattingTotals, PitchingSeasonRow, PitchingTotals, format_ip},
    fmt, server,
};

const PAGE_SIZE: u32 = 25;

#[component]
pub fn PlayerDetail(id: i32) -> Element {
    let detail = use_resource(move || server::player_detail(id));

    let batting_page = use_signal(|| 0u32);
    let batting_log = use_resource(move || {
        let p = batting_page();
        async move { server::player_batting_log(id, p, PAGE_SIZE).await }
    });

    let batting_seasons = use_resource(move || server::player_batting_seasons(id));
    let pitching_seasons = use_resource(move || server::player_pitching_seasons(id));

    let pitching_page = use_signal(|| 0u32);
    let pitching_log = use_resource(move || {
        let p = pitching_page();
        async move { server::player_pitching_log(id, p, PAGE_SIZE).await }
    });

    rsx! {
        match &*detail.read() {
            Some(Ok(d)) => rsx! {
                h1 { "{d.player.name}" }
                div { class: "muted", "bbref: {d.player.bbref_id}" }
                if let Some(batting) = d.batting.clone() {
                    h2 { "Career batting (regular season)" }
                    BattingTotalsView { totals: batting }
                }
                if let Some(batting) = d.batting_postseason.clone() {
                    h2 { "Career batting (postseason)" }
                    BattingTotalsView { totals: batting }
                }
                if let Some(pitching) = d.pitching.clone() {
                    h2 { "Career pitching (regular season)" }
                    PitchingTotalsView { totals: pitching }
                }
                if let Some(pitching) = d.pitching_postseason.clone() {
                    h2 { "Career pitching (postseason)" }
                    PitchingTotalsView { totals: pitching }
                }
                if d.batting.is_none() && d.pitching.is_none() && d.batting_postseason.is_none()
                    && d.pitching_postseason.is_none()
                {
                    div { class: "muted", "No stat lines recorded for this player." }
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load player: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading player…" }
            },
        }

        match &*batting_seasons.read() {
            Some(Ok(rows)) if !rows.is_empty() => {
                let regular: Vec<BattingSeasonRow> = rows.iter().filter(|r| !r.postseason).cloned().collect();
                let post: Vec<BattingSeasonRow> = rows.iter().filter(|r| r.postseason).cloned().collect();
                rsx! {
                    if !regular.is_empty() {
                        h2 { "Batting by season" }
                        BattingSeasonsTable { rows: regular }
                    }
                    if !post.is_empty() {
                        h2 { "Postseason batting by season" }
                        BattingSeasonsTable { rows: post }
                    }
                }
            }
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load batting seasons: {e}" }
            },
            _ => rsx! {},
        }

        match &*pitching_seasons.read() {
            Some(Ok(rows)) if !rows.is_empty() => {
                let regular: Vec<PitchingSeasonRow> = rows.iter().filter(|r| !r.postseason).cloned().collect();
                let post: Vec<PitchingSeasonRow> = rows.iter().filter(|r| r.postseason).cloned().collect();
                rsx! {
                    if !regular.is_empty() {
                        h2 { "Pitching by season" }
                        PitchingSeasonsTable { rows: regular }
                    }
                    if !post.is_empty() {
                        h2 { "Postseason pitching by season" }
                        PitchingSeasonsTable { rows: post }
                    }
                }
            }
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load pitching seasons: {e}" }
            },
            _ => rsx! {},
        }

        match &*batting_log.read() {
            Some(Ok(pg)) if !pg.items.is_empty() => rsx! {
                h2 { "Batting game log" }
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Date" }
                                th { "Team" }
                                th { "Opp" }
                                th { "Pos" }
                                th { class: "num", "PA" }
                                th { class: "num", "AB" }
                                th { class: "num", "R" }
                                th { class: "num", "H" }
                                th { class: "num", "RBI" }
                                th { class: "num", "BB" }
                                th { class: "num", "SO" }
                                th { class: "num", "WPA" }
                                th { "" }
                            }
                        }
                        tbody {
                            for row in pg.items.clone() {
                                tr { key: "{row.game_id}",
                                    td { "{row.game_date}" }
                                    td { "{row.team_code}" }
                                    td { "{row.opponent_code}" }
                                    td { {row.position.clone().unwrap_or_default()} }
                                    td { class: "num", {fmt::opt(row.pa)} }
                                    td { class: "num", {fmt::opt(row.ab)} }
                                    td { class: "num", {fmt::opt(row.r)} }
                                    td { class: "num", {fmt::opt(row.h)} }
                                    td { class: "num", {fmt::opt(row.rbi)} }
                                    td { class: "num", {fmt::opt(row.bb)} }
                                    td { class: "num", {fmt::opt(row.so)} }
                                    td { class: "num", {fmt::signed2(row.wpa)} }
                                    td {
                                        Link { to: Route::GameDetail { id: row.game_id }, "box" }
                                    }
                                }
                            }
                        }
                    }
                }
                Pagination {
                    page: batting_page,
                    total_pages: pg.total_pages(),
                    total: pg.total,
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load batting log: {e}" }
            },
            _ => rsx! {},
        }

        match &*pitching_log.read() {
            Some(Ok(pg)) if !pg.items.is_empty() => rsx! {
                h2 { "Pitching game log" }
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Date" }
                                th { "Team" }
                                th { "Opp" }
                                th { "Dec" }
                                th { class: "num", "IP" }
                                th { class: "num", "H" }
                                th { class: "num", "R" }
                                th { class: "num", "ER" }
                                th { class: "num", "BB" }
                                th { class: "num", "SO" }
                                th { class: "num", "HR" }
                                th { class: "num", "Pit" }
                                th { class: "num", "GSc" }
                                th { "" }
                            }
                        }
                        tbody {
                            for row in pg.items.clone() {
                                tr { key: "{row.game_id}",
                                    td { "{row.game_date}" }
                                    td { "{row.team_code}" }
                                    td { "{row.opponent_code}" }
                                    td { {row.decision.clone().unwrap_or_default()} }
                                    td { class: "num", {fmt::ip(row.ip)} }
                                    td { class: "num", {fmt::opt(row.h)} }
                                    td { class: "num", {fmt::opt(row.r)} }
                                    td { class: "num", {fmt::opt(row.er)} }
                                    td { class: "num", {fmt::opt(row.bb)} }
                                    td { class: "num", {fmt::opt(row.so)} }
                                    td { class: "num", {fmt::opt(row.hr)} }
                                    td { class: "num", {fmt::opt(row.pitches)} }
                                    td { class: "num", {fmt::opt(row.game_score)} }
                                    td {
                                        Link { to: Route::GameDetail { id: row.game_id }, "box" }
                                    }
                                }
                            }
                        }
                    }
                }
                Pagination {
                    page: pitching_page,
                    total_pages: pg.total_pages(),
                    total: pg.total,
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load pitching log: {e}" }
            },
            _ => rsx! {},
        }

        div { class: "footnote",
            "OBP is approximated as (H+BB)/PA — HBP and sacrifice flies are not scraped. SLG is an AB-weighted average of per-game SLG. Postseason is detected as games after each year's last date with 6+ games league-wide; game-163 tiebreakers count as postseason."
        }
    }
}

#[component]
fn BattingSeasonsTable(rows: Vec<BattingSeasonRow>) -> Element {
    rsx! {
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { "Season" }
                        th { class: "num", "G" }
                        th { class: "num", "PA" }
                        th { class: "num", "AB" }
                        th { class: "num", "R" }
                        th { class: "num", "H" }
                        th { class: "num", "RBI" }
                        th { class: "num", "BB" }
                        th { class: "num", "SO" }
                        th { class: "num", "AVG" }
                        th { class: "num", "OBP*" }
                        th { class: "num", "SLG*" }
                        th { class: "num", "OPS*" }
                        th { class: "num", "WPA" }
                    }
                }
                tbody {
                    for row in rows {
                        tr { key: "{row.season}",
                            td { "{row.season}" }
                            td { class: "num", "{row.games}" }
                            td { class: "num", "{row.pa}" }
                            td { class: "num", "{row.ab}" }
                            td { class: "num", "{row.r}" }
                            td { class: "num", "{row.h}" }
                            td { class: "num", "{row.rbi}" }
                            td { class: "num", "{row.bb}" }
                            td { class: "num", "{row.so}" }
                            td { class: "num", {fmt::rate3(row.avg)} }
                            td { class: "num", {fmt::rate3(row.obp)} }
                            td { class: "num", {fmt::rate3(row.slg)} }
                            td { class: "num", {fmt::rate3(row.ops)} }
                            td { class: "num", {fmt::signed2(row.wpa)} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PitchingSeasonsTable(rows: Vec<PitchingSeasonRow>) -> Element {
    rsx! {
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { "Season" }
                        th { class: "num", "G" }
                        th { class: "num", "W–L" }
                        th { class: "num", "SV" }
                        th { class: "num", "IP" }
                        th { class: "num", "H" }
                        th { class: "num", "R" }
                        th { class: "num", "ER" }
                        th { class: "num", "BB" }
                        th { class: "num", "SO" }
                        th { class: "num", "HR" }
                        th { class: "num", "ERA" }
                        th { class: "num", "WHIP" }
                        th { class: "num", "WPA" }
                    }
                }
                tbody {
                    for row in rows {
                        tr { key: "{row.season}",
                            td { "{row.season}" }
                            td { class: "num", "{row.games}" }
                            td { class: "num", "{row.wins}–{row.losses}" }
                            td { class: "num", "{row.saves}" }
                            td { class: "num", {format_ip(row.outs)} }
                            td { class: "num", "{row.h}" }
                            td { class: "num", "{row.r}" }
                            td { class: "num", "{row.er}" }
                            td { class: "num", "{row.bb}" }
                            td { class: "num", "{row.so}" }
                            td { class: "num", "{row.hr}" }
                            td { class: "num", {fmt::num2(row.era)} }
                            td { class: "num", {fmt::num2(row.whip)} }
                            td { class: "num", {fmt::signed2(row.wpa)} }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BattingTotalsView(totals: BattingTotals) -> Element {
    rsx! {
        div { class: "stat-grid",
            StatCard { label: "G", value: totals.games.to_string() }
            StatCard { label: "PA", value: totals.pa.to_string() }
            StatCard { label: "H", value: totals.h.to_string() }
            StatCard { label: "R", value: totals.r.to_string() }
            StatCard { label: "RBI", value: totals.rbi.to_string() }
            StatCard { label: "AVG", value: fmt::rate3(totals.avg) }
            StatCard { label: "OBP*", value: fmt::rate3(totals.obp) }
            StatCard { label: "SLG*", value: fmt::rate3(totals.slg) }
            StatCard { label: "OPS*", value: fmt::rate3(totals.ops) }
        }
    }
}

#[component]
fn PitchingTotalsView(totals: PitchingTotals) -> Element {
    rsx! {
        div { class: "stat-grid",
            StatCard { label: "G", value: totals.games.to_string() }
            StatCard { label: "IP", value: format_ip(totals.outs) }
            StatCard { label: "SO", value: totals.so.to_string() }
            StatCard { label: "BB", value: totals.bb.to_string() }
            StatCard { label: "HR", value: totals.hr.to_string() }
            StatCard { label: "ERA", value: fmt::num2(totals.era) }
            StatCard { label: "WHIP", value: fmt::num2(totals.whip) }
        }
    }
}

#[component]
fn StatCard(label: String, value: String) -> Element {
    rsx! {
        div { class: "stat-card",
            div { class: "stat-value", "{value}" }
            div { class: "stat-label", "{label}" }
        }
    }
}
