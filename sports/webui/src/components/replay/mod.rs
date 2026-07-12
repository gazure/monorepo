mod chips;
mod diamond;
mod panels;
mod wp_chart;

use dioxus::prelude::*;

use crate::dto::PlayDto;

/// Animated play-by-play replay: scoreboard, diamond, pitch chips, play card,
/// scrubber/autoplay controls, and (for decided games) the synced win
/// probability chart.
#[component]
pub fn ReplayDeck(
    plays: Vec<PlayDto>,
    home_code: String,
    away_code: String,
    /// `None` ⇒ tied/undecided game: deck renders, WP chart is omitted
    home_won: Option<bool>,
) -> Element {
    let mut current = use_signal(|| 0usize);
    let mut playing = use_signal(|| false);
    let len = plays.len();
    use_autoplay(current, playing, len);

    if len == 0 {
        return rsx! {};
    }
    let idx = current().min(len - 1);
    let p = &plays[idx];

    // Away/home attribution: trust the batting-team code (relocated "home"
    // games bat first); fall back to the inning half.
    let batting_is_home = if p.batting_team == home_code {
        true
    } else if p.batting_team == away_code {
        false
    } else {
        p.is_bottom
    };
    let (away_score, home_score) = if batting_is_home {
        (p.score_fielding_team, p.score_batting_team)
    } else {
        (p.score_batting_team, p.score_fielding_team)
    };

    rsx! {
        div {
            class: "replay-deck",
            tabindex: "0",
            onkeydown: move |evt| match evt.key() {
                Key::ArrowLeft => {
                    playing.set(false);
                    let c = current();
                    if c > 0 {
                        current.set(c - 1);
                    }
                }
                Key::ArrowRight => {
                    playing.set(false);
                    let c = current();
                    if c + 1 < len {
                        current.set(c + 1);
                    }
                }
                _ => {}
            },
            div { class: "replay-main",
                diamond::Diamond { runners: p.runners_before.clone() }
                div { class: "replay-state",
                    panels::ScoreBug {
                        away_code: away_code.clone(),
                        home_code: home_code.clone(),
                        away_score,
                        home_score,
                        inning: p.inning,
                        is_bottom: p.is_bottom,
                    }
                    panels::OutsDots { outs: p.outs_before }
                    panels::Matchup { batter: p.batter.clone(), pitcher: p.pitcher.clone() }
                    chips::PitchChips { sequence: p.pitch_sequence.clone() }
                    panels::PlayCard { play: p.clone(), home_won }
                }
            }
            ReplayControls { current, playing, len }
        }
        if let Some(hw) = home_won {
            wp_chart::WinProbChart {
                plays: plays.clone(),
                home_code: home_code.clone(),
                away_code: away_code.clone(),
                home_won: hw,
                current,
                playing,
            }
        }
    }
}

#[component]
fn ReplayControls(mut current: Signal<usize>, mut playing: Signal<bool>, len: usize) -> Element {
    let at_start = current() == 0;
    let at_end = current() + 1 >= len;
    rsx! {
        div { class: "replay-controls",
            button {
                disabled: at_start,
                onclick: move |_| {
                    playing.set(false);
                    let c = current();
                    if c > 0 {
                        current.set(c - 1);
                    }
                },
                "◀"
            }
            button {
                onclick: move |_| {
                    if playing() {
                        playing.set(false);
                    } else {
                        if current() + 1 >= len {
                            current.set(0);
                        }
                        playing.set(true);
                    }
                },
                if playing() { "⏸" } else { "▶ play" }
            }
            button {
                disabled: at_end,
                onclick: move |_| {
                    playing.set(false);
                    let c = current();
                    if c + 1 < len {
                        current.set(c + 1);
                    }
                },
                "▶"
            }
            input {
                r#type: "range",
                min: "0",
                max: "{len.saturating_sub(1)}",
                value: "{current()}",
                oninput: move |evt| {
                    playing.set(false);
                    if let Ok(v) = evt.value().parse::<usize>() {
                        current.set(v.min(len.saturating_sub(1)));
                    }
                },
            }
            span { class: "replay-counter", "{current() + 1}/{len}" }
        }
    }
}

/// Advance the replay while playing; auto-pause on the final play.
/// The timer only exists in the wasm client — SSR renders a static frame.
fn use_autoplay(current: Signal<usize>, playing: Signal<bool>, len: usize) {
    #[cfg(target_arch = "wasm32")]
    {
        let mut current = current;
        let mut playing = playing;
        use_future(move || async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(1200).await;
                if playing() && len > 0 {
                    let next = current() + 1;
                    if next >= len {
                        playing.set(false);
                    } else {
                        current.set(next);
                    }
                }
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (current, playing, len);
}
