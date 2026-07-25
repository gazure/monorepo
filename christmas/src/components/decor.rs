//! Seasonal furniture: snow over everything, a strand of lights under the nav.
//!
//! Both are decoration only — `aria-hidden`, pointer-transparent, and animated
//! entirely in CSS so nothing here costs a render on the Rust side. The motion
//! is switched off under `prefers-reduced-motion` in the stylesheet.

use dioxus::prelude::*;

/// Three parallax layers of falling snow.
///
/// Each layer is a tiled radial-gradient rather than one element per flake, so
/// a whole snowstorm is three nodes.
#[component]
pub fn Snowfall() -> Element {
    rsx! {
        div { class: "snowfall", "aria-hidden": "true",
            div { class: "snow-layer snow-far" }
            div { class: "snow-layer snow-mid" }
            div { class: "snow-layer snow-near" }
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
