//! Small display helpers for optional stat values.

pub fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or_else(String::new, |x| x.to_string())
}

/// Rate stats like AVG/OBP/SLG/OPS, 3 decimals.
pub fn rate3(v: Option<f64>) -> String {
    v.map_or_else(String::new, |x| format!("{x:.3}"))
}

/// ERA/WHIP-style numbers, 2 decimals.
pub fn num2(v: Option<f64>) -> String {
    v.map_or_else(String::new, |x| format!("{x:.2}"))
}

/// Signed 2-decimal numbers (WPA).
pub fn signed2(v: Option<f64>) -> String {
    v.map_or_else(String::new, |x| format!("{x:+.2}"))
}

pub fn score(v: Option<i32>) -> String {
    v.map_or_else(|| "–".to_string(), |s| s.to_string())
}

/// A pitching line's `ip` column (already baseball notation), shown as-is.
pub fn ip(v: Option<f64>) -> String {
    v.map_or_else(String::new, |x| format!("{x:.1}"))
}
