mod bar_chart;
mod hover;
mod line_chart;
mod scale;
mod sparkline;

pub use bar_chart::{Bar, BarChart};
pub use line_chart::LineChart;
pub use scale::{f as index_f64, nice_ticks};
pub use sparkline::Sparkline;

/// A data point in domain units
#[derive(Debug, Clone, PartialEq)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

/// An axis tick: domain position plus label
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    pub at: f64,
    pub label: String,
}

/// Tooltip content, precomputed per point at the call site
#[derive(Debug, Clone, PartialEq)]
pub struct HoverInfo {
    pub title: String,
    pub rows: Vec<(String, String)>,
}

/// Fixed viewBox width; rendered width scales via `width: 100%`
const VIEW_W: f64 = 720.0;
const MARGIN_L: f64 = 46.0;
const MARGIN_R: f64 = 14.0;
const MARGIN_T: f64 = 10.0;
const MARGIN_B: f64 = 26.0;

fn format_tick(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{v:.0}")
    } else if (v * 10.0).fract().abs() < 1e-9 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}
