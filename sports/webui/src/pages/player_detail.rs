use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{
        Pagination,
        chart::{HoverInfo, LineChart, Pt, Tick},
    },
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

    let splits = use_resource(move || server::player_batting_splits(id));

    rsx! {
        match &*detail.read() {
            Some(Ok(d)) => rsx! {
                h1 { "{d.player.name}" }
                div { class: "muted",
                    a {
                        href: crate::bbref::player_url(&d.player.bbref_id),
                        target: "_blank",
                        rel: "noopener",
                        "{d.player.bbref_id} ↗"
                    }
                }
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
                        if regular.len() >= 3 {
                            BattingTrendChart { rows: regular.clone() }
                        }
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
                        if regular.len() >= 3 {
                            PitchingTrendCharts { rows: regular.clone() }
                        }
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

        match &*splits.read() {
            Some(Ok(s)) if !s.home_away.is_empty() => rsx! {
                h2 { "Batting splits" }
                div { class: "chart-row",
                    SplitTable { title: "Home / road".to_string(), rows: s.home_away.clone() }
                    SplitTable { title: "By opponent (min 10 PA)".to_string(), rows: s.vs_team.clone() }
                }
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
            "Postseason is detected as games after each year's last date with 6+ games league-wide; game-163 tiebreakers count as postseason."
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
                        th { class: "num", "2B" }
                        th { class: "num", "3B" }
                        th { class: "num", "HR" }
                        th { class: "num", "RBI" }
                        th { class: "num", "SB" }
                        th { class: "num", "BB" }
                        th { class: "num", "SO" }
                        th { class: "num", "AVG" }
                        th { class: "num", "OBP" }
                        th { class: "num", "SLG" }
                        th { class: "num", "OPS" }
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
                            td { class: "num", "{row.doubles}" }
                            td { class: "num", "{row.triples}" }
                            td { class: "num", "{row.home_runs}" }
                            td { class: "num", "{row.rbi}" }
                            td { class: "num", "{row.stolen_bases}" }
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
            StatCard { label: "HR", value: totals.home_runs.to_string() }
            StatCard { label: "R", value: totals.r.to_string() }
            StatCard { label: "RBI", value: totals.rbi.to_string() }
            StatCard { label: "SB", value: totals.stolen_bases.to_string() }
            StatCard { label: "AVG", value: fmt::rate3(totals.avg) }
            StatCard { label: "OBP", value: fmt::rate3(totals.obp) }
            StatCard { label: "SLG", value: fmt::rate3(totals.slg) }
            StatCard { label: "OPS", value: fmt::rate3(totals.ops) }
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
fn SplitTable(title: String, rows: Vec<crate::dto::SplitRow>) -> Element {
    if rows.is_empty() {
        return rsx! {};
    }
    rsx! {
        div {
            h2 { class: "muted", "{title}" }
            div { class: "table-scroll",
                table { class: "data-table",
                    thead {
                        tr {
                            th { "" }
                            th { class: "num", "G" }
                            th { class: "num", "PA" }
                            th { class: "num", "H" }
                            th { class: "num", "HR" }
                            th { class: "num", "AVG" }
                            th { class: "num", "OBP" }
                            th { class: "num", "SLG" }
                            th { class: "num", "OPS" }
                        }
                    }
                    tbody {
                        for row in rows {
                            tr { key: "{row.label}",
                                td { "{row.label}" }
                                td { class: "num", "{row.games}" }
                                td { class: "num", "{row.pa}" }
                                td { class: "num", "{row.h}" }
                                td { class: "num", "{row.home_runs}" }
                                td { class: "num", {fmt::rate3(row.avg)} }
                                td { class: "num", {fmt::rate3(row.obp)} }
                                td { class: "num", {fmt::rate3(row.slg)} }
                                td { class: "num", {fmt::rate3(row.ops)} }
                            }
                        }
                    }
                }
            }
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

/// x ticks for a span of seasons: every year for short careers, else round years
fn season_ticks(seasons: &[i32]) -> Vec<Tick> {
    let (Some(&min), Some(&max)) = (seasons.iter().min(), seasons.iter().max()) else {
        return Vec::new();
    };
    let step = match max - min {
        0..=8 => 1,
        9..=25 => 5,
        _ => 10,
    };
    (min..=max)
        .filter(|y| y == &min || y == &max || y % step == 0)
        .map(|y| Tick {
            at: f64::from(y),
            label: y.to_string(),
        })
        .collect()
}

#[component]
fn BattingTrendChart(rows: Vec<BattingSeasonRow>) -> Element {
    let mut rows = rows;
    rows.sort_by_key(|r| r.season);

    let mut points = Vec::new();
    let mut hover = Vec::new();
    for r in &rows {
        let Some(ops) = r.ops else { continue };
        points.push(Pt {
            x: f64::from(r.season),
            y: ops,
        });
        hover.push(HoverInfo {
            title: r.season.to_string(),
            rows: vec![
                ("OPS".to_string(), fmt::rate3(r.ops)),
                ("AVG".to_string(), fmt::rate3(r.avg)),
                ("PA".to_string(), r.pa.to_string()),
                ("WPA".to_string(), fmt::signed2(r.wpa)),
            ],
        });
    }
    let seasons: Vec<i32> = rows.iter().map(|r| r.season).collect();

    rsx! {
        div { class: "chart-frame",
            div { class: "chart-title", "OPS by season" }
            LineChart {
                points,
                hover,
                markers: true,
                gap_break: Some(1.5),
                x_ticks: Some(season_ticks(&seasons)),
            }
        }
    }
}

#[component]
fn PitchingTrendCharts(rows: Vec<PitchingSeasonRow>) -> Element {
    let mut rows = rows;
    rows.sort_by_key(|r| r.season);
    let seasons: Vec<i32> = rows.iter().map(|r| r.season).collect();

    let mut era_points = Vec::new();
    let mut era_hover = Vec::new();
    let mut whip_points = Vec::new();
    let mut whip_hover = Vec::new();
    for r in &rows {
        let x = f64::from(r.season);
        let ip = format_ip(r.outs);
        if let Some(era) = r.era {
            era_points.push(Pt { x, y: era });
            era_hover.push(HoverInfo {
                title: r.season.to_string(),
                rows: vec![
                    ("ERA".to_string(), fmt::num2(r.era)),
                    ("IP".to_string(), ip.clone()),
                    ("SO".to_string(), r.so.to_string()),
                ],
            });
        }
        if let Some(whip) = r.whip {
            whip_points.push(Pt { x, y: whip });
            whip_hover.push(HoverInfo {
                title: r.season.to_string(),
                rows: vec![
                    ("WHIP".to_string(), fmt::num2(r.whip)),
                    ("IP".to_string(), ip),
                    ("BB".to_string(), r.bb.to_string()),
                ],
            });
        }
    }

    rsx! {
        div { class: "chart-row",
            div { class: "chart-frame",
                div { class: "chart-title", "ERA by season" }
                LineChart {
                    points: era_points,
                    hover: era_hover,
                    markers: true,
                    gap_break: Some(1.5),
                    x_ticks: Some(season_ticks(&seasons)),
                }
            }
            div { class: "chart-frame",
                div { class: "chart-title", "WHIP by season" }
                LineChart {
                    points: whip_points,
                    hover: whip_hover,
                    markers: true,
                    gap_break: Some(1.5),
                    x_ticks: Some(season_ticks(&seasons)),
                }
            }
        }
    }
}
