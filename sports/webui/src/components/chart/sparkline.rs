use std::fmt::Write as _;

use dioxus::prelude::*;

use super::scale::{Linear, f};

const SPARK_W: f64 = 120.0;
const SPARK_H: f64 = 36.0;

/// Tiny inline line chart: no axes, no hover — the shape is the message
#[component]
pub fn Sparkline(
    values: Vec<f64>,
    /// Dashed midline (e.g. 50% win expectancy)
    #[props(default)]
    ref_value: Option<f64>,
) -> Element {
    if values.len() < 2 {
        return rsx! {};
    }
    let sx = Linear {
        d0: 0.0,
        d1: f(values.len() - 1),
        r0: 2.0,
        r1: SPARK_W - 2.0,
    };
    let sy = Linear {
        d0: 0.0,
        d1: 1.0,
        r0: SPARK_H - 2.0,
        r1: 2.0,
    };
    let mut points = String::new();
    for (i, v) in values.iter().enumerate() {
        let _ = write!(points, "{:.1},{:.1} ", sx.map(f(i)), sy.map(v.clamp(0.0, 1.0)));
    }

    rsx! {
        svg {
            class: "sparkline",
            view_box: "0 0 {SPARK_W} {SPARK_H}",
            preserve_aspect_ratio: "none",
            if let Some(r) = ref_value {
                line {
                    class: "spark-ref",
                    stroke_dasharray: "3 3",
                    x1: 2.0,
                    x2: SPARK_W - 2.0,
                    y1: sy.map(r),
                    y2: sy.map(r),
                }
            }
            polyline { class: "spark-line", points: "{points}" }
        }
    }
}
