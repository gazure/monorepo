use dioxus::prelude::*;

use crate::{
    components::chart::{HoverInfo, LineChart, Pt, Tick, index_f64},
    dto::PlayDto,
    fmt,
};

/// Home-team win expectancy over the course of the game, synced to the
/// replay position.
///
/// Stored `win_expectancy_after` is bbref's wWE% — the eventual winner's
/// perspective — so it is normalized to home-team WE via `home_won`. Events
/// are plotted at sequential indices (stored sequences can have holes), and a
/// synthetic terminal point covers games whose final events are missing.
#[component]
#[allow(clippy::too_many_lines)]
pub(super) fn WinProbChart(
    plays: Vec<PlayDto>,
    home_code: String,
    away_code: String,
    home_won: bool,
    current: Signal<usize>,
    playing: Signal<bool>,
) -> Element {
    let mut points = vec![Pt { x: 0.0, y: 0.5 }];
    let mut hover = vec![HoverInfo {
        title: "Game start".to_string(),
        rows: vec![("WE (home)".to_string(), "50%".to_string())],
    }];
    let mut x_ticks: Vec<Tick> = Vec::new();
    let mut ticked_inning = 0;

    // Chart points ≠ plays: synthetic start, holes skipped, synthetic
    // terminal. Track both directions for cursor/seek mapping.
    let mut point_play: Vec<Option<usize>> = vec![None];
    let mut play_point: Vec<usize> = Vec::with_capacity(plays.len());
    let mut last_point = 0usize;

    for (pi, p) in plays.iter().enumerate() {
        let Some(we) = p.win_expectancy_after else {
            play_point.push(last_point);
            continue;
        };
        let x = index_f64(points.len());
        let y = if home_won { we } else { 1.0 - we };

        if p.inning > ticked_inning {
            x_ticks.push(Tick {
                at: x,
                label: p.inning.to_string(),
            });
            ticked_inning = p.inning;
        }

        let batting = if p.is_bottom { &home_code } else { &away_code };
        let half = if p.is_bottom { "b" } else { "t" };
        let mut rows = vec![(
            "Score".to_string(),
            format!(
                "{}–{}",
                fmt::score(p.score_batting_team),
                fmt::score(p.score_fielding_team)
            ),
        )];
        rows.push(("Batter".to_string(), p.batter.clone()));
        if let Some(desc) = &p.description {
            let short: String = if desc.chars().count() > 90 {
                let mut s: String = desc.chars().take(90).collect();
                s.push('…');
                s
            } else {
                desc.clone()
            };
            rows.push(("Play".to_string(), short));
        }
        rows.push(("WE (home)".to_string(), format!("{:.0}%", y * 100.0)));
        if let Some(wpa) = p.wpa {
            rows.push(("WPA".to_string(), format!("{wpa:+.1}%")));
        }

        last_point = points.len();
        point_play.push(Some(pi));
        play_point.push(last_point);
        points.push(Pt { x, y });
        hover.push(HoverInfo {
            title: format!("{half}{} · {batting} batting", p.inning),
            rows,
        });
    }

    // Terminal point at the true outcome (covers missing walk-off events)
    let final_we = if home_won { 1.0 } else { 0.0 };
    points.push(Pt {
        x: index_f64(points.len()),
        y: final_we,
    });
    hover.push(HoverInfo {
        title: "Final".to_string(),
        rows: vec![("WE (home)".to_string(), format!("{:.0}%", final_we * 100.0))],
    });
    point_play.push(None);

    if x_ticks.len() > 12 {
        let mut i = 0;
        x_ticks.retain(|_| {
            i += 1;
            i % 2 == 1
        });
    }

    let y_ticks: Vec<Tick> = [0.0, 0.25, 0.5, 0.75, 1.0]
        .into_iter()
        .map(|v| Tick {
            at: v,
            label: format!("{:.0}%", v * 100.0),
        })
        .collect();

    let cursor = play_point.get(current()).copied();
    let last_play = plays.len().saturating_sub(1);

    rsx! {
        div { class: "chart-frame",
            div { class: "chart-title", "Home win expectancy — {home_code}" }
            LineChart {
                points,
                hover,
                height: 260.0,
                y_domain: Some((0.0, 1.0)),
                y_ticks: Some(y_ticks),
                x_ticks: Some(x_ticks),
                ref_line: Some(0.5),
                cursor,
                on_point_click: move |i: usize| {
                    let mut playing = playing;
                    let mut current = current;
                    playing.set(false);
                    let target = point_play
                        .get(i)
                        .copied()
                        .flatten()
                        .unwrap_or(if i == 0 { 0 } else { last_play });
                    current.set(target);
                },
            }
            div { class: "footnote",
                "Click the chart to jump the replay. From stored play-by-play events; sequences may have gaps."
            }
        }
    }
}
