//! Seasonal furniture: snow over everything, a strand of lights under the nav.
//!
//! Both are decoration only — `aria-hidden`, pointer-transparent, and animated
//! entirely in CSS so nothing here costs a render on the Rust side. The motion
//! is switched off under `prefers-reduced-motion` in the stylesheet.

use dioxus::prelude::*;

/// How many flakes are in the air.
///
/// Enough to read as weather, few enough that each one can be an individual
/// element — which is what buys the side-to-side flutter. A tiled background
/// is cheaper, but every flake in a tile then sways in lockstep, and the whole
/// window looks like it is rocking rather than the snow drifting.
const FLAKES: usize = 90;

/// Deterministic pseudo-randomness from a flake's index.
///
/// Not `fastrand`: this component renders once on the server and again in the
/// browser, and any disagreement between the two is a hydration mismatch. Same
/// index and salt, same number, both times.
fn spread(i: usize, salt: u32, range: u32) -> u32 {
    let seed = u32::try_from(i)
        .unwrap_or(0)
        .wrapping_add(salt)
        .wrapping_mul(0x9E37_79B1);
    (seed >> 13) % range
}

/// Snow, falling and fluttering across the whole window.
///
/// Each flake carries its own size, speed, drift width and phase, so no two
/// take the same path down. The fall and the sway are separate CSS properties
/// (`translate` and `transform`) precisely so both animations can run on one
/// element and compose, rather than the second overwriting the first.
#[component]
pub fn Snowfall() -> Element {
    rsx! {
        div { class: "snowfall", "aria-hidden": "true",
            for i in 0..FLAKES {
                {
                    let size = 1 + spread(i, 11, 3);
                    let fall = 9 + spread(i, 23, 12);
                    let sway_secs = 3 + spread(i, 41, 6);
                    // Bigger flakes read as nearer, so they carry more weight.
                    let opacity = 35 + size * 15;
                    let style = format!(
                        "left:{left}%;width:{size}px;height:{size}px;opacity:{opacity}%;\
                         --sway:{sway}px;--rest:{rest}vh;\
                         animation-duration:{fall}s,{sway_secs}s;\
                         animation-delay:-{fall_offset}s,-{sway_offset}s",
                        left = spread(i, 3, 1000) / 10,
                        // Negative delays start the storm mid-flight instead of
                        // dropping every flake from the ceiling at once.
                        fall_offset = spread(i, 59, fall),
                        sway_offset = spread(i, 73, sway_secs),
                        sway = 7 + spread(i, 97, 26),
                        // Where a flake sits when motion is switched off.
                        rest = spread(i, 31, 100),
                    );
                    rsx! { span { key: "flake{i}", class: "flake", style: "{style}" } }
                }
            }
        }
    }
}

/// How many bulbs to string. Wide screens use them all; narrower ones clip the
/// ends, which is what a real strand does when it runs past the window.
const BULBS: usize = 28;

/// Bulbs cycle through the tints so the strand reads as a strand rather than a
/// row of identical dots.
const TINTS: [&str; 4] = ["cranberry", "holly", "brass", "ice"];

/// A strand of fairy lights, hung under the navigation.
#[component]
pub fn StringLights() -> Element {
    rsx! {
        div { class: "lights", "aria-hidden": "true",
            for i in 0..BULBS {
                span {
                    key: "bulb{i}",
                    class: "bulb {TINTS[i % TINTS.len()]}",
                    // A prime-ish stride so the twinkle ripples along the wire
                    // instead of the whole strand pulsing at once.
                    style: "animation-delay: {(i % 7) * 290}ms",
                }
            }
        }
    }
}
