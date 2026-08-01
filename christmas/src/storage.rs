//! Remembers, per browser, which draws this person has already watched.
//!
//! Local storage rather than anything server-side: with one shared password
//! there is no identity to hang it on, and "have I seen the 2026 Pets reveal"
//! is a property of the device, not the account.
//!
//! Consequently this holds back the *presentation* of a draw, not the data. The
//! pairings are already in the page the server sent, so a determined viewer can
//! always read them early. That is the right trade for a family exchange —
//! nobody is attacking it, they just want the reveal to land — but callers
//! should not mistake these helpers for access control. Real enforcement lives
//! in `auth`, which gates whole endpoints by role.

/// Storage key for one recorded draw.
#[cfg_attr(
    not(any(target_arch = "wasm32", test)),
    expect(dead_code, reason = "only the wasm build reads storage; its test covers this")
)]
fn key(exchange_id: i32) -> String {
    format!("christmas.watched.{exchange_id}")
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    // Both calls can legitimately fail — private browsing disables storage.
    web_sys::window()?.local_storage().ok().flatten()
}

/// Whether this browser has watched the given draw.
///
/// Always false off-wasm, which includes server-side rendering: the first paint
/// must therefore not reveal anything, or a returning visitor would flash the
/// results before the check runs.
#[cfg(target_arch = "wasm32")]
pub fn has_watched(exchange_id: i32) -> bool {
    storage().is_some_and(|s| s.get_item(&key(exchange_id)).ok().flatten().is_some())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn has_watched(_exchange_id: i32) -> bool {
    false
}

#[cfg(target_arch = "wasm32")]
pub fn mark_watched(exchange_id: i32) {
    if let Some(s) = storage() {
        // Nothing to do if it fails; the worst case is being offered the
        // ceremony again.
        let _ = s.set_item(&key(exchange_id), "1");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn mark_watched(_exchange_id: i32) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_scoped_per_draw() {
        assert_eq!(key(7), "christmas.watched.7");
        assert_ne!(key(7), key(8));
    }

    /// Server-side rendering must never claim a draw has been watched, or the
    /// results would appear before the client can check.
    #[test]
    fn nothing_is_watched_off_wasm() {
        assert!(!has_watched(1));
    }
}
