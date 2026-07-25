use dioxus::prelude::*;

use crate::auth::Role;

/// What the caller is allowed to be.
///
/// The UI needs this to decide whether to surface manager-only detail such as
/// earlier revisions; the actual filtering still happens server-side, so a
/// tampered answer here reveals nothing.
#[server]
pub async fn my_role() -> Result<Role, ServerFnError> {
    Ok(crate::auth::caller_role().await)
}
