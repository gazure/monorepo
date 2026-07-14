use dioxus::prelude::*;

use crate::{
    app::Route,
    dto::{BattingLeaderboardReq, BattingSort, PitchingLeaderboardReq, PitchingSort, format_ip},
    fmt,
    pages::games::GamesTable,
    server,
};

const LEADER_LIMIT: u32 = 10;
const MIN_PA: i64 = 200;
const MIN_IP: i64 = 40;

#[component]
pub fn SeasonDetail(year: i32) -> Element {
    let standings = use_resource(move || server::season_standings(year));
    let postseason = use_resource(move || server::season_postseason_games(year));
    let batting = use_resource(move || {
        server::batting_leaderboard(BattingLeaderboardReq {
            sort: BattingSort::Ops,
            min_pa: MIN_PA,
            season: Some(year),
            limit: LEADER_LIMIT,
            offset: 0,
        })
    });
    let pitching = use_resource(move || {
        server::pitching_leaderboard(PitchingLeaderboardReq {
            sort: PitchingSort::Era,
            min_outs: MIN_IP * 3,
            season: Some(year),
            limit: LEADER_LIMIT,
            offset: 0,
        })
    });

    rsx! {
        h1 { "{year} season" }

        h2 { "Standings (regular season)" }
        match &*standings.read() {
            Some(Ok(teams)) if teams.is_empty() => rsx! {
                div { class: "muted", "No games recorded for {year}." }
            },
            Some(Ok(teams)) => rsx! {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "Team" }
                                th { class: "num", "G" }
                                th { class: "num", "W" }
                                th { class: "num", "L" }
                                th { class: "num", "PCT" }
                                th { class: "num", "RF" }
                                th { class: "num", "RA" }
                                th { class: "num", "Diff" }
                            }
                        }
                        tbody {
                            for t in teams.clone() {
                                tr { key: "{t.team.id}",
                                    td {
                                        Link { to: Route::TeamDetail { id: t.team.id }, "{t.team.code}" }
                                        span { class: "muted", " {t.team.name}" }
                                    }
                                    td { class: "num", "{t.games}" }
                                    td { class: "num", "{t.wins}" }
                                    td { class: "num", "{t.losses}" }
                                    td { class: "num", {win_pct(t.wins, t.losses)} }
                                    td { class: "num", "{t.runs_for}" }
                                    td { class: "num", "{t.runs_against}" }
                                    td { class: "num", "{t.runs_for - t.runs_against}" }
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load standings: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading standings…" }
            },
        }

        h2 { "Batting leaders (min {MIN_PA} PA, by OPS)" }
        match &*batting.read() {
            Some(Ok(pg)) if pg.items.is_empty() => rsx! {
                div { class: "muted", "No qualifying batters." }
            },
            Some(Ok(pg)) => rsx! {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "#" }
                                th { "Player" }
                                th { class: "num", "PA" }
                                th { class: "num", "H" }
                                th { class: "num", "HR" }
                                th { class: "num", "R" }
                                th { class: "num", "RBI" }
                                th { class: "num", "SB" }
                                th { class: "num", "AVG" }
                                th { class: "num", "OPS" }
                            }
                        }
                        tbody {
                            for (i , row) in pg.items.clone().into_iter().enumerate() {
                                tr { key: "{row.player_id}",
                                    td { class: "num muted", "{i + 1}" }
                                    td {
                                        Link { to: Route::PlayerDetail { id: row.player_id }, "{row.name}" }
                                    }
                                    td { class: "num", "{row.pa}" }
                                    td { class: "num", "{row.h}" }
                                    td { class: "num", "{row.home_runs}" }
                                    td { class: "num", "{row.r}" }
                                    td { class: "num", "{row.rbi}" }
                                    td { class: "num", "{row.stolen_bases}" }
                                    td { class: "num", {fmt::rate3(row.avg)} }
                                    td { class: "num", {fmt::rate3(row.ops)} }
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load batting leaders: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading…" }
            },
        }

        h2 { "Pitching leaders (min {MIN_IP} IP, by ERA)" }
        match &*pitching.read() {
            Some(Ok(pg)) if pg.items.is_empty() => rsx! {
                div { class: "muted", "No qualifying pitchers." }
            },
            Some(Ok(pg)) => rsx! {
                div { class: "table-scroll",
                    table { class: "data-table",
                        thead {
                            tr {
                                th { "#" }
                                th { "Player" }
                                th { class: "num", "W–L" }
                                th { class: "num", "SV" }
                                th { class: "num", "IP" }
                                th { class: "num", "SO" }
                                th { class: "num", "ERA" }
                                th { class: "num", "WHIP" }
                            }
                        }
                        tbody {
                            for (i , row) in pg.items.clone().into_iter().enumerate() {
                                tr { key: "{row.player_id}",
                                    td { class: "num muted", "{i + 1}" }
                                    td {
                                        Link { to: Route::PlayerDetail { id: row.player_id }, "{row.name}" }
                                    }
                                    td { class: "num", "{row.wins}–{row.losses}" }
                                    td { class: "num", "{row.saves}" }
                                    td { class: "num", {format_ip(row.outs)} }
                                    td { class: "num", "{row.so}" }
                                    td { class: "num", {fmt::num2(row.era)} }
                                    td { class: "num", {fmt::num2(row.whip)} }
                                }
                            }
                        }
                    }
                }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load pitching leaders: {e}" }
            },
            None => rsx! {
                div { class: "loading", "Loading…" }
            },
        }
        div { class: "footnote",
            Link { to: Route::Leaderboards { season: Some(year) }, "Full {year} leaderboards →" }
        }

        match &*postseason.read() {
            Some(Ok(games)) if !games.is_empty() => rsx! {
                h2 { "Postseason" }
                GamesTable { games: games.clone() }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load postseason games: {e}" }
            },
            _ => rsx! {},
        }
    }
}

fn win_pct(wins: i64, losses: i64) -> String {
    let total = wins + losses;
    if total == 0 {
        return String::new();
    }
    #[expect(clippy::cast_precision_loss)]
    let pct = wins as f64 / total as f64;
    format!("{pct:.3}")
}
