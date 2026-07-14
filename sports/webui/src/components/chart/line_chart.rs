use std::fmt::Write as _;

use dioxus::prelude::*;

use super::{
    HoverInfo, MARGIN_B, MARGIN_L, MARGIN_R, MARGIN_T, Pt, Tick, VIEW_W, format_tick,
    hover::{ChartTooltip, nearest_index, to_pixel_x, to_viewbox_x, use_hover},
    scale::{Linear, nice_ticks},
};

/// Single-series line chart. `points` must be sorted by x; `hover` supplies
/// tooltip content per point (same length as `points`).
#[component]
#[allow(clippy::too_many_lines)]
pub fn LineChart(
    points: Vec<Pt>,
    hover: Vec<HoverInfo>,
    #[props(default = 220.0)] height: f64,
    /// None → padded nice domain from the data
    #[props(default)]
    y_domain: Option<(f64, f64)>,
    #[props(default)] x_ticks: Option<Vec<Tick>>,
    #[props(default)] y_ticks: Option<Vec<Tick>>,
    /// Horizontal reference line (e.g. 50% win expectancy)
    #[props(default)]
    ref_line: Option<f64>,
    /// Draw a dot at every point (use for sparse series like seasons)
    #[props(default)]
    markers: bool,
    /// Start a new line segment when the x gap between neighbors exceeds this
    #[props(default)]
    gap_break: Option<f64>,
    #[props(default = true)] end_dot: bool,
    /// Persistently highlighted point (e.g. a replay cursor)
    #[props(default)]
    cursor: Option<usize>,
    /// Click on the plot snaps to the nearest point and reports its index
    #[props(default)]
    on_point_click: Option<EventHandler<usize>>,
) -> Element {
    let (mut hovered, mut width_px) = use_hover();

    if points.len() < 2 {
        return rsx! {};
    }

    let x0 = points.first().map_or(0.0, |p| p.x);
    let x1 = points.last().map_or(1.0, |p| p.x);
    let (y0, y1) = y_domain.unwrap_or_else(|| {
        let lo = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let hi = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
        let ticks = nice_ticks(lo, hi, 4);
        let step = if ticks.len() > 1 { ticks[1] - ticks[0] } else { 1.0 };
        (
            lo.min(ticks[0]) - step * 0.25,
            hi.max(*ticks.last().expect("nonempty")) + step * 0.25,
        )
    });

    let plot_bottom = height - MARGIN_B;
    let sx = Linear {
        d0: x0,
        d1: x1,
        r0: MARGIN_L,
        r1: VIEW_W - MARGIN_R,
    };
    let sy = Linear {
        d0: y0,
        d1: y1,
        r0: plot_bottom,
        r1: MARGIN_T,
    };

    let y_tick_list: Vec<Tick> = y_ticks.unwrap_or_else(|| {
        nice_ticks(y0, y1, 4)
            .into_iter()
            .map(|v| Tick {
                at: v,
                label: format_tick(v),
            })
            .collect()
    });
    let x_tick_list: Vec<Tick> = x_ticks.unwrap_or_else(|| {
        nice_ticks(x0, x1, 6)
            .into_iter()
            .map(|v| Tick {
                at: v,
                label: format_tick(v),
            })
            .collect()
    });

    // viewBox x per point, for hover snapping
    let px: Vec<f64> = points.iter().map(|p| sx.map(p.x)).collect();

    // Split into polyline segments at gaps
    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut prev_x: Option<f64> = None;
    for p in &points {
        if let (Some(prev), Some(gap)) = (prev_x, gap_break)
            && p.x - prev > gap
            && !current.is_empty()
        {
            segments.push(std::mem::take(&mut current));
        }
        let _ = write!(current, "{:.1},{:.1} ", sx.map(p.x), sy.map(p.y));
        prev_x = Some(p.x);
    }
    if !current.is_empty() {
        segments.push(current);
    }

    let hovered_idx = hovered();
    let snap_xs = px.clone();
    let click_xs = px.clone();

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
                hovered.set(nearest_index(&snap_xs, vb_x));
            },
            onmouseleave: move |_| hovered.set(None),
            onclick: move |evt| {
                if let Some(handler) = on_point_click {
                    let vb_x = to_viewbox_x(evt.data().element_coordinates().x, width_px());
                    if let Some(i) = nearest_index(&click_xs, vb_x) {
                        handler.call(i);
                    }
                }
            },

            svg { view_box: "0 0 {VIEW_W} {height}", preserve_aspect_ratio: "xMidYMid meet",
                // Gridlines + y tick labels
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
                // Baseline + x tick labels
                line {
                    class: "chart-axis-line",
                    x1: MARGIN_L,
                    x2: VIEW_W - MARGIN_R,
                    y1: plot_bottom,
                    y2: plot_bottom,
                }
                for t in &x_tick_list {
                    text {
                        class: "chart-tick-label",
                        x: sx.map(t.at),
                        y: plot_bottom + 16.0,
                        text_anchor: "middle",
                        "{t.label}"
                    }
                }
                if let Some(r) = ref_line {
                    line {
                        class: "chart-ref-line",
                        stroke_dasharray: "4 3",
                        x1: MARGIN_L,
                        x2: VIEW_W - MARGIN_R,
                        y1: sy.map(r),
                        y2: sy.map(r),
                    }
                }
                for (i , seg) in segments.iter().enumerate() {
                    polyline { key: "{i}", class: "chart-line", points: "{seg}" }
                }
                if markers {
                    for (i , p) in points.iter().enumerate() {
                        circle {
                            key: "{i}",
                            class: "chart-dot",
                            cx: sx.map(p.x),
                            cy: sy.map(p.y),
                            r: 4.0,
                        }
                    }
                }
                if end_dot && !markers {
                    if let Some(p) = points.last() {
                        circle {
                            class: "chart-dot",
                            cx: sx.map(p.x),
                            cy: sy.map(p.y),
                            r: 4.0,
                        }
                    }
                }
                if let Some(i) = cursor {
                    if i < px.len() {
                        // Translated group: transform is CSS-transitionable, raw x1/x2 aren't
                        g {
                            class: "chart-cursor-g",
                            style: "transform: translateX({px[i]}px);",
                            line {
                                class: "chart-cursor",
                                x1: 0.0,
                                x2: 0.0,
                                y1: MARGIN_T,
                                y2: plot_bottom,
                            }
                        }
                        circle {
                            class: "chart-cursor-dot",
                            cx: px[i],
                            cy: sy.map(points[i].y),
                            r: 5.0,
                        }
                    }
                }
                if let Some(i) = hovered_idx {
                    line {
                        class: "chart-crosshair",
                        x1: px[i],
                        x2: px[i],
                        y1: MARGIN_T,
                        y2: plot_bottom,
                    }
                    circle {
                        class: "chart-dot",
                        cx: px[i],
                        cy: sy.map(points[i].y),
                        r: 5.0,
                    }
                }
            }

            if let Some(i) = hovered_idx {
                if let Some(info) = hover.get(i) {
                    ChartTooltip {
                        info: info.clone(),
                        px_x: to_pixel_x(px[i], width_px()),
                        flip: px[i] > VIEW_W * 0.55,
                    }
                }
            }
        }
    }
}
