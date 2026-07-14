use dioxus::prelude::*;

use crate::{dto::PlayDto, fmt};

#[component]
pub(super) fn ScoreBug(
    away_code: String,
    home_code: String,
    away_score: Option<i32>,
    home_score: Option<i32>,
    inning: i32,
    is_bottom: bool,
) -> Element {
    let half = if is_bottom { "▼" } else { "▲" };
    rsx! {
        div { class: "replay-scorebug",
            span { "{away_code} " }
            b { {fmt::score(away_score)} }
            span { class: "muted", " — " }
            span { "{home_code} " }
            b { {fmt::score(home_score)} }
            span { class: "replay-half", " {half}{inning}" }
        }
    }
}

#[component]
pub(super) fn OutsDots(outs: Option<i32>) -> Element {
    let filled = usize::try_from(outs.unwrap_or(0)).unwrap_or(0).min(3);
    rsx! {
        div { class: "replay-outs",
            for i in 0..3usize {
                span { key: "{i}", class: if i < filled { "dot filled" } else { "dot" } }
            }
            span { class: "muted", "out" }
        }
    }
}

#[component]
pub(super) fn Matchup(batter: String, pitcher: String) -> Element {
    rsx! {
        div { class: "replay-matchup",
            b { "{batter}" }
            span { class: "muted", " vs " }
            span { "{pitcher}" }
        }
    }
}

#[component]
pub(super) fn PlayCard(play: PlayDto, home_won: Option<bool>) -> Element {
    let we_text = match (home_won, play.win_expectancy_after) {
        (Some(hw), Some(we)) => {
            let home_we = if hw { we } else { 1.0 - we } * 100.0;
            Some(format!("WE (home) {home_we:.0}%"))
        }
        _ => None,
    };
    rsx! {
        div { class: "replay-playcard",
            div { {play.description.clone().unwrap_or_else(|| "—".to_string())} }
            div { class: "replay-wpa",
                if let Some(wpa) = play.wpa {
                    span { "WPA {wpa:+.1}% " }
                }
                if let Some(we) = &we_text {
                    span { "· {we}" }
                }
            }
        }
    }
}
