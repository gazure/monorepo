use dioxus::prelude::*;

use super::{
    HoverInfo, MARGIN_B, MARGIN_L, MARGIN_R, MARGIN_T, Tick, VIEW_W, format_tick,
    hover::{ChartTooltip, to_pixel_x, to_viewbox_x, use_hover},
    scale::{Linear, f, nice_ticks},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub label: String,
    pub value: f64,
    pub info: HoverInfo,
}

/// Single-series bar chart with band layout and per-mark hover
#[component]
pub fn BarChart(
    bars: Vec<Bar>,
    #[props(default = 220.0)] height: f64,
    /// Draw every Nth x label; 0 = auto (~10 labels)
    #[props(default)]
    label_every: usize,
    #[props(default)] y_ticks: Option<Vec<Tick>>,
) -> Element {
    let (mut hovered, mut width_px) = use_hover();

    if bars.is_empty() {
        return rsx! {};
    }

    let n = bars.len();
    let max_v = bars.iter().map(|b| b.value).fold(0.0f64, f64::max).max(1.0);
    let y_tick_list: Vec<Tick> = y_ticks.unwrap_or_else(|| {
        nice_ticks(0.0, max_v, 4)
            .into_iter()
            .map(|v| Tick {
                at: v,
                label: format_tick(v),
            })
            .collect()
    });
    let y_top = y_tick_list.iter().map(|t| t.at).fold(max_v, f64::max);

    let plot_bottom = height - MARGIN_B;
    let plot_w = VIEW_W - MARGIN_L - MARGIN_R;
    let band_w = plot_w / f(n);
    let bar_w = (band_w - 2.0).clamp(1.0, 24.0);
    let sy = Linear {
        d0: 0.0,
        d1: y_top,
        r0: plot_bottom,
        r1: MARGIN_T,
    };

    let step = if label_every == 0 {
        n.div_ceil(10).max(1)
    } else {
        label_every
    };

    let hovered_idx = hovered();

    rsx! {
        div {
            class: "chart-body",
            onmounted: move |evt| {
                spawn(async move {
                    if let Ok(rect) = evt.data().get_client_rect().await {
                        width_px.set(rect.size.width);
                    }
                });
            },
            onresize: move |evt| {
                if let Ok(size) = evt.data().get_content_box_size() {
                    width_px.set(size.width);
                }
            },
            onmousemove: move |evt| {
                let vb_x = to_viewbox_x(evt.data().element_coordinates().x, width_px());
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clamped band-index math"
                )]
                let idx = (((vb_x - MARGIN_L) / band_w).floor().max(0.0) as usize).min(n - 1);
                hovered
                    .set(
                        if (MARGIN_L..=VIEW_W - MARGIN_R).contains(&vb_x) { Some(idx) } else { None },
                    );
            },
            onmouseleave: move |_| hovered.set(None),

            svg { view_box: "0 0 {VIEW_W} {height}", preserve_aspect_ratio: "xMidYMid meet",
                for t in &y_tick_list {
                    line {
                        class: "chart-grid-line",
                        x1: MARGIN_L,
                        x2: VIEW_W - MARGIN_R,
                        y1: sy.map(t.at),
                        y2: sy.map(t.at),
                    }
                    text {
                        class: "chart-tick-label",
                        x: MARGIN_L - 6.0,
                        y: sy.map(t.at) + 4.0,
                        text_anchor: "end",
                        "{t.label}"
                    }
                }
                line {
                    class: "chart-axis-line",
                    x1: MARGIN_L,
                    x2: VIEW_W - MARGIN_R,
                    y1: plot_bottom,
                    y2: plot_bottom,
                }
                for (i , b) in bars.iter().enumerate() {
                    rect {
                        key: "{i}",
                        class: if hovered_idx == Some(i) { "chart-bar hot" } else { "chart-bar" },
                        x: MARGIN_L + f(i) * band_w + (band_w - bar_w) / 2.0,
                        y: sy.map(b.value),
                        width: bar_w,
                        height: (plot_bottom - sy.map(b.value)).max(0.0),
                        rx: 2.0,
                    }
                    if i % step == 0 {
                        text {
                            class: "chart-tick-label",
                            x: MARGIN_L + f(i) * band_w + band_w / 2.0,
                            y: plot_bottom + 16.0,
                            text_anchor: "middle",
                            "{b.label}"
                        }
                    }
                }
            }

            if let Some(i) = hovered_idx {
                if let Some(b) = bars.get(i) {
                    ChartTooltip {
                        info: b.info.clone(),
                        px_x: to_pixel_x(MARGIN_L + f(i) * band_w + band_w / 2.0, width_px()),
                        flip: MARGIN_L + f(i) * band_w > VIEW_W * 0.55,
                    }
                }
            }
        }
    }
}
