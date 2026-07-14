use dioxus::prelude::*;

use crate::{
    app::Route,
    divisions::league_division,
    dto::{BattingLeaderboardReq, BattingSort, PitchingLeaderboardReq, PitchingSort, TeamSummary, format_ip},
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
    let bracket = use_resource(move || server::postseason_bracket(year));
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
                StandingsSection { teams: teams.clone(), year }
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

        match &*bracket.read() {
            Some(Ok(rounds)) if !rounds.is_empty() => rsx! {
                h2 { "Postseason" }
                Bracket { rounds: rounds.clone() }
            },
            _ => rsx! {},
        }

        match &*postseason.read() {
            Some(Ok(games)) if !games.is_empty() => rsx! {
                h2 { class: "muted", "Postseason games" }
                GamesTable { games: games.clone() }
            },
            Some(Err(e)) => rsx! {
                div { class: "error-box", "Failed to load postseason games: {e}" }
            },
            _ => rsx! {},
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum StandingsView {
    Division,
    League,
    Overall,
}

/// Standings grouped by division (default), league, or ungrouped — with
/// games-behind computed within each group
#[component]
fn StandingsSection(teams: Vec<TeamSummary>, year: i32) -> Element {
    let mut view = use_signal(|| StandingsView::Division);

    let mapped: Vec<(TeamSummary, Option<crate::divisions::LeagueDiv>)> = teams
        .iter()
        .map(|t| (t.clone(), league_division(&t.team.code, year)))
        .collect();
    let all_mapped = mapped.iter().all(|(_, m)| m.is_some());
    let has_divisions = mapped.iter().any(|(_, m)| m.is_some_and(|m| m.division.is_some()));

    let effective = if !all_mapped {
        StandingsView::Overall
    } else if view() == StandingsView::Division && !has_divisions {
        StandingsView::League
    } else {
        view()
    };

    // Build (title, rows) groups; teams arrive sorted by winning percentage,
    // so filtered groups stay sorted
    let division_rank = |d: Option<&str>| match d {
        Some("East") => 0,
        Some("Central") => 1,
        Some("West") => 2,
        _ => 3,
    };
    let groups: Vec<(String, Vec<TeamSummary>)> = match effective {
        StandingsView::Overall => vec![(String::new(), teams.clone())],
        StandingsView::League => ["AL", "NL"]
            .iter()
            .map(|league| {
                let rows = mapped
                    .iter()
                    .filter(|(_, m)| m.is_some_and(|m| m.league == *league))
                    .map(|(t, _)| t.clone())
                    .collect::<Vec<_>>();
                (
                    if *league == "AL" {
                        "American League".to_string()
                    } else {
                        "National League".to_string()
                    },
                    rows,
                )
            })
            .filter(|(_, rows)| !rows.is_empty())
            .collect(),
        StandingsView::Division => {
            let mut keys: Vec<(&str, Option<&str>)> = mapped
                .iter()
                .filter_map(|(_, m)| m.map(|m| (m.league, m.division)))
                .collect();
            keys.sort_by_key(|(l, d)| (*l != "AL", division_rank(*d)));
            keys.dedup();
            keys.into_iter()
                .map(|(league, division)| {
                    let rows = mapped
                        .iter()
                        .filter(|(_, m)| m.is_some_and(|m| m.league == league && m.division == division))
                        .map(|(t, _)| t.clone())
                        .collect::<Vec<_>>();
                    let title = division.map_or_else(|| league.to_string(), |d| format!("{league} {d}"));
                    (title, rows)
                })
                .collect()
        }
    };

    rsx! {
        if all_mapped {
            div { class: "tabs",
                if has_divisions {
                    button {
                        class: if effective == StandingsView::Division { "active" } else { "" },
                        onclick: move |_| view.set(StandingsView::Division),
                        "Division"
                    }
                }
                button {
                    class: if effective == StandingsView::League { "active" } else { "" },
                    onclick: move |_| view.set(StandingsView::League),
                    "League"
                }
                button {
                    class: if effective == StandingsView::Overall { "active" } else { "" },
                    onclick: move |_| view.set(StandingsView::Overall),
                    "Overall"
                }
            }
        }
        for (title , rows) in groups {
            if !title.is_empty() {
                h2 { class: "muted", "{title}" }
            }
            StandingsTable { rows }
        }
    }
}

#[component]
fn StandingsTable(rows: Vec<TeamSummary>) -> Element {
    let (leader_w, leader_l) = rows.first().map_or((0, 0), |t| (t.wins, t.losses));
    rsx! {
        div { class: "table-scroll",
            table { class: "data-table",
                thead {
                    tr {
                        th { "Team" }
                        th { class: "num", "G" }
                        th { class: "num", "W" }
                        th { class: "num", "L" }
                        th { class: "num", "PCT" }
                        th { class: "num", "GB" }
                        th { class: "num", "RF" }
                        th { class: "num", "RA" }
                        th { class: "num", "Diff" }
                    }
                }
                tbody {
                    for t in rows {
                        tr { key: "{t.team.id}",
                            td {
                                Link { to: Route::TeamDetail { id: t.team.id }, "{t.team.code}" }
                                span { class: "muted", " {t.team.name}" }
                            }
                            td { class: "num", "{t.games}" }
                            td { class: "num", "{t.wins}" }
                            td { class: "num", "{t.losses}" }
                            td { class: "num", {win_pct(t.wins, t.losses)} }
                            td { class: "num", {games_behind(leader_w, leader_l, t.wins, t.losses)} }
                            td { class: "num", "{t.runs_for}" }
                            td { class: "num", "{t.runs_against}" }
                            td { class: "num", "{t.runs_for - t.runs_against}" }
                        }
                    }
                }
            }
        }
    }
}

/// Postseason series columns, earliest round on the left, champion bolded
#[component]
fn Bracket(rounds: Vec<Vec<crate::dto::BracketSeries>>) -> Element {
    let total = rounds.len();
    let round_label = |i: usize| -> &'static str {
        match total - 1 - i {
            0 => "World Series",
            1 => "Championship",
            2 => "Division Series",
            3 => "Wild Card",
            _ => "Play-in",
        }
    };

    rsx! {
        div { class: "bracket",
            for (i , round) in rounds.into_iter().enumerate() {
                div { class: "bracket-round", key: "{i}",
                    div { class: "bracket-round-label", {round_label(i)} }
                    for (j , s) in round.into_iter().enumerate() {
                        div { class: "bracket-card", key: "{i}-{j}",
                            div { class: "bracket-team winner",
                                Link { to: Route::TeamDetail { id: s.winner.id }, "{s.winner.code}" }
                                span { class: "num", "{s.winner_wins}" }
                            }
                            div { class: "bracket-team",
                                Link { to: Route::TeamDetail { id: s.loser.id }, "{s.loser.code}" }
                                span { class: "num", "{s.loser_wins}" }
                            }
                        }
                    }
                }
            }
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

/// Games behind the group leader; the leader shows an em dash
fn games_behind(leader_w: i64, leader_l: i64, wins: i64, losses: i64) -> String {
    let halves = (leader_w - wins) + (losses - leader_l);
    if halves <= 0 {
        return "—".to_string();
    }
    #[expect(clippy::cast_precision_loss, reason = "game counts are far below 2^52")]
    let gb = halves as f64 / 2.0;
    format!("{gb:.1}")
}
