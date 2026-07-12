use dioxus::prelude::*;

use super::{HoverInfo, VIEW_W};

/// Hover state shared by chart bodies: hovered point index + rendered pixel
/// width (needed to map mouse px ↔ viewBox units, since the SVG scales with
/// `width: 100%`).
pub(super) fn use_hover() -> (Signal<Option<usize>>, Signal<f64>) {
    let hovered = use_signal(|| None::<usize>);
    let width_px = use_signal(|| VIEW_W);
    (hovered, width_px)
}

/// Mouse x in element pixels → viewBox units
pub(super) fn to_viewbox_x(mouse_x: f64, width_px: f64) -> f64 {
    mouse_x * VIEW_W / width_px.max(1.0)
}

/// viewBox x → element pixels (for positioning the HTML tooltip)
pub(super) fn to_pixel_x(viewbox_x: f64, width_px: f64) -> f64 {
    viewbox_x * width_px / VIEW_W
}

/// Index of the point whose viewBox x is nearest to `target` (linear scan;
/// handles non-uniform spacing like season gaps)
pub(super) fn nearest_index(xs: &[f64], target: f64) -> Option<usize> {
    xs.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - target).abs().total_cmp(&(*b - target).abs()))
        .map(|(i, _)| i)
}

#[component]
pub(super) fn ChartTooltip(info: HoverInfo, px_x: f64, flip: bool) -> Element {
    let transform = if flip {
        "translateX(-100%) translateX(-10px)"
    } else {
        "translateX(10px)"
    };
    rsx! {
        div { class: "chart-tooltip", style: "left: {px_x}px; transform: {transform};",
            div { class: "tt-title", "{info.title}" }
            for (i , (label , val)) in info.rows.iter().enumerate() {
                div { key: "{i}",
                    span { class: "tt-label", "{label}: " }
                    span { class: "tt-val", "{val}" }
                }
            }
        }
    }
}
