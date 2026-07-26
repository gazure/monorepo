//! The draw ceremony — a paced, screen-shareable reveal of a recorded draw.
//!
//! Reads from a stored exchange rather than drawing live, so it can be replayed,
//! paused, and stepped back without ever changing the result.

use dioxus::prelude::*;

use crate::{app::Route, components::RevealRing, model::Exchange, server};

/// Milliseconds per beat. Slow enough that people can find their own name on a
/// video call before it moves on.
const BEAT_MS: u32 = 2400;

/// Where the ceremony has got to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Scene {
    /// Everyone gathered, nothing drawn yet.
    Gathering,
    /// The letter of the year.
    Letter,
    /// Reading out one giver → receiver pair.
    Link { ring: usize, edge: usize },
    /// Everything revealed.
    Complete,
}

/// Flattens a draw into the ordered beats of the ceremony.
fn build_scenes(cycles: &[Vec<String>], has_letter: bool) -> Vec<Scene> {
    let mut scenes = vec![Scene::Gathering];
    if has_letter {
        scenes.push(Scene::Letter);
    }
    for (ring, cycle) in cycles.iter().enumerate() {
        for edge in 0..cycle.len() {
            scenes.push(Scene::Link { ring, edge });
        }
    }
    scenes.push(Scene::Complete);
    scenes
}

#[component]
pub fn Reveal(id: i32) -> Element {
    let exchange = use_resource(move || server::exchange_detail(id));

    rsx! {
        match &*exchange.read() {
            Some(Ok(exchange)) => rsx! {
                Ceremony {
                    exchange: exchange.clone(),
                    autoplay: false,
                    on_watched: move |()| {},
                    on_close: None,
                }
            },
            Some(Err(e)) => rsx! { div { class: "error-box", "{e}" } },
            None => rsx! { div { class: "loading", "Setting up…" } },
        }
    }
}

/// The ceremony itself.
///
/// `on_watched` fires when the last beat is reached. `on_close` being `Some`
/// puts it in embedded mode, where it overlays a page and can be dismissed.
/// The two stay separate so callers can treat sitting through it and skipping
/// out differently, even though the pool page currently treats them the same.
#[component]
pub fn Ceremony(
    exchange: Exchange,
    autoplay: bool,
    on_watched: EventHandler<()>,
    on_close: Option<EventHandler<()>>,
) -> Element {
    let cycles = exchange.cycles();
    let scenes = build_scenes(&cycles, exchange.letter.is_some());
    let total = scenes.len();

    let mut position = use_signal(|| 0usize);
    let mut playing = use_signal(|| autoplay);
    // Latched so reaching the end repeatedly doesn't re-notify.
    let mut notified = use_signal(|| false);

    use_autoplay(position, playing, total);

    // Getting to the last beat counts as having seen it.
    use_effect(move || {
        if position() >= total - 1 && !notified() {
            notified.set(true);
            on_watched.call(());
        }
    });

    let at = position().min(total - 1);
    let scene = scenes[at].clone();

    // Which ring to show, and how much of it has been read out.
    let (active_ring, revealed) = match &scene {
        Scene::Gathering | Scene::Letter => (0usize, 0usize),
        Scene::Link { ring, edge } => (*ring, edge + 1),
        Scene::Complete => (0, cycles.first().map_or(0, Vec::len)),
    };
    let active_cycle = cycles.get(active_ring).cloned().unwrap_or_default();
    let ring_count = cycles.len();

    // Saturating rather than signed arithmetic, so the ends of the ceremony
    // simply stop instead of wrapping.
    let mut step = move |forward: bool| {
        let current = position();
        let next = if forward {
            (current + 1).min(total - 1)
        } else {
            current.saturating_sub(1)
        };
        position.set(next);
    };

    let embedded = on_close.is_some();
    // Dismissing is not the same as having watched: the parent decides what a
    // skip means, so it can stop re-playing without revealing the results.
    let skip = move |()| {
        if let Some(close) = on_close {
            close.call(());
        }
    };

    rsx! {
        div {
            class: if embedded { "reveal reveal-overlay" } else { "reveal" },
            tabindex: "0",
            autofocus: true,
            onkeydown: move |e| {
                match e.key() {
                    Key::ArrowRight => step(true),
                    Key::ArrowLeft => step(false),
                    Key::Character(c) if c == " " => {
                        let now = playing();
                        playing.set(!now);
                    }
                    _ => return,
                }
                e.prevent_default();
            },

            header { class: "reveal-head",
                if embedded {
                    span { class: "reveal-back", "{exchange.pool_name}" }
                } else {
                    Link {
                        to: Route::PoolPage { slug: exchange.pool_slug.clone() },
                        class: "reveal-back",
                        "← {exchange.pool_name}"
                    }
                }
                span { class: "reveal-year mono", "{exchange.year}" }
                if embedded {
                    button {
                        class: "linklike reveal-skip",
                        onclick: move |_| skip(()),
                        if position() >= total - 1 { "Close" } else { "Skip →" }
                    }
                }
            }

            div { class: "reveal-stage",
                match &scene {
                    Scene::Gathering => rsx! {
                        div { class: "reveal-title",
                            h1 { "{exchange.pool_name}" }
                            p { "{exchange.participants.len()} names in the hat." }
                        }
                    },
                    Scene::Letter => rsx! {
                        div { class: "reveal-letter",
                            p { class: "eyebrow", "every gift starts with" }
                            div { class: "letter-glyph huge", "{exchange.letter.unwrap_or('?')}" }
                        }
                    },
                    Scene::Link { ring, edge } => {
                        let cycle = &cycles[*ring];
                        let giver = cycle[*edge].clone();
                        let receiver = cycle[(edge + 1) % cycle.len()].clone();
                        rsx! {
                            div { class: "reveal-ring-wrap",
                                if ring_count > 1 {
                                    p { class: "eyebrow", "ring {ring + 1} of {ring_count}" }
                                }
                                RevealRing {
                                    cycle: active_cycle.clone(),
                                    revealed,
                                    letter: exchange.letter,
                                    show_letter: true,
                                }
                            }
                            div { class: "reveal-callout-slot",
                                // Keyed so each new pair is a fresh node and the
                                // entrance animation replays; a text swap alone
                                // would reuse the node and never restart it.
                                div { key: "{giver}-{receiver}", class: "reveal-callout",
                                    span { class: "giver", "{giver}" }
                                    span { class: "gives", "gives to" }
                                    span { class: "receiver", "{receiver}" }
                                }
                            }
                        }
                    },
                    Scene::Complete => rsx! {
                        div { class: "reveal-complete",
                            h1 { "That's the draw." }
                            if let Some(letter) = exchange.letter {
                                p { "Everything starts with " span { class: "accent", "{letter}" } "." }
                            }
                            div { class: "reveal-all-rings",
                                for (i, cycle) in cycles.iter().enumerate() {
                                    RevealRing {
                                        key: "done{i}",
                                        cycle: cycle.clone(),
                                        revealed: cycle.len(),
                                        letter: exchange.letter,
                                        show_letter: ring_count == 1,
                                    }
                                }
                            }
                        }
                    },
                }
            }

            div { class: "reveal-progress",
                for i in 0..total {
                    span {
                        key: "p{i}",
                        class: if i <= at { "pip done" } else { "pip" },
                    }
                }
            }

            div { class: "reveal-controls",
                button {
                    class: "ghost",
                    disabled: at == 0,
                    onclick: move |_| step(false),
                    "← Back"
                }
                button {
                    onclick: move |_| {
                        // Restarting from the end is the natural second action.
                        if position() >= total - 1 {
                            position.set(0);
                        }
                        let now = playing();
                        playing.set(!now);
                    },
                    if playing() { "Pause" } else if at >= total - 1 { "Play again" } else { "Play" }
                }
                button {
                    class: "ghost",
                    disabled: at >= total - 1,
                    onclick: move |_| step(true),
                    "Next →"
                }
            }
        }
    }
}

/// Advances the ceremony while playing. Only wasm has a timer; on the server the
/// first frame is rendered and the client takes over.
fn use_autoplay(position: Signal<usize>, playing: Signal<bool>, total: usize) {
    #[cfg(target_arch = "wasm32")]
    {
        let mut position = position;
        let mut playing = playing;
        use_future(move || async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(BEAT_MS).await;
                if playing() && total > 0 {
                    let next = position() + 1;
                    if next >= total {
                        playing.set(false);
                    } else {
                        position.set(next);
                    }
                }
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (position, playing, total, BEAT_MS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cycles() -> Vec<Vec<String>> {
        vec![
            vec!["A".into(), "B".into(), "C".into()],
            vec!["D".into(), "E".into(), "F".into(), "G".into()],
        ]
    }

    #[test]
    fn every_pairing_gets_its_own_beat() {
        let scenes = build_scenes(&cycles(), true);
        let links = scenes.iter().filter(|s| matches!(s, Scene::Link { .. })).count();
        assert_eq!(links, 7, "one beat per giver");
    }

    #[test]
    fn the_ceremony_opens_on_the_roster_and_closes_complete() {
        let scenes = build_scenes(&cycles(), true);
        assert_eq!(scenes.first(), Some(&Scene::Gathering));
        assert_eq!(scenes.last(), Some(&Scene::Complete));
    }

    #[test]
    fn the_letter_beat_is_skipped_when_there_is_no_letter() {
        let with = build_scenes(&cycles(), true);
        let without = build_scenes(&cycles(), false);
        assert!(with.contains(&Scene::Letter));
        assert!(!without.contains(&Scene::Letter));
        assert_eq!(with.len(), without.len() + 1);
    }

    #[test]
    fn rings_are_read_out_in_order() {
        let scenes = build_scenes(&cycles(), false);
        let rings: Vec<usize> = scenes
            .iter()
            .filter_map(|s| match s {
                Scene::Link { ring, .. } => Some(*ring),
                _ => None,
            })
            .collect();
        assert_eq!(rings, vec![0, 0, 0, 1, 1, 1, 1]);
    }

    #[test]
    fn a_single_ring_still_works() {
        let scenes = build_scenes(&[vec!["A".into(), "B".into(), "C".into()]], true);
        // gathering + letter + 3 links + complete
        assert_eq!(scenes.len(), 6);
    }
}
