/// Linear domain → range mapping
pub struct Linear {
    pub d0: f64,
    pub d1: f64,
    pub r0: f64,
    pub r1: f64,
}

impl Linear {
    pub fn map(&self, v: f64) -> f64 {
        if (self.d1 - self.d0).abs() < f64::EPSILON {
            return f64::midpoint(self.r0, self.r1);
        }
        self.r0 + (v - self.d0) / (self.d1 - self.d0) * (self.r1 - self.r0)
    }
}

/// "Nice" tick positions covering [min, max] using a 1/2/5 step ladder,
/// aiming for roughly `target` ticks
pub fn nice_ticks(min: f64, max: f64, target: usize) -> Vec<f64> {
    if !(max - min).is_finite() || max <= min || target == 0 {
        return vec![min, max];
    }
    let raw_step = (max - min) / f(target);
    let mag = 10f64.powf(raw_step.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * mag)
        .find(|s| *s >= raw_step)
        .unwrap_or(10.0 * mag);

    let first = (min / step).ceil() * step;
    let mut ticks = Vec::new();
    let mut v = first;
    while v <= max + step * 1e-6 {
        // Snap near-zero float noise (e.g. 0.30000000000000004) to the grid
        ticks.push((v / step).round() * step);
        v += step;
    }
    if ticks.is_empty() {
        return vec![min, max];
    }
    ticks
}

/// Centralized `usize` → `f64` cast for chart coordinate math
#[expect(clippy::cast_precision_loss, reason = "chart coordinates; values are far below 2^52")]
pub fn f(n: usize) -> f64 {
    n as f64
}
