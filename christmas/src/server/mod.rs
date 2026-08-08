//! Server functions, grouped by domain.

mod draw;
mod letters;
mod participants;
mod pools;
mod relationships;
mod session;

pub use draw::{apply_swap, exchange_detail, list_exchanges, list_year, preview_swap, record_past_draw, run_draw};
pub use letters::{list_all_excluded_letters, list_excluded_letters, set_excluded_letters};
pub use participants::{add_participant, list_participants, remove_participant};
pub use pools::{add_member, create_pool, delete_pool, list_memberships, list_pools, pool_detail, remove_member};
pub use relationships::{add_relationship, list_relationships, remove_relationship};
pub use session::my_role;

#[cfg(feature = "server")]
#[expect(clippy::needless_pass_by_value, reason = "used as a map_err callback")]
pub(crate) fn db_err(e: sqlx::Error) -> dioxus::prelude::ServerFnError {
    tracingx::error!(error = %e, "database query failed");
    dioxus::prelude::ServerFnError::new(e.to_string())
}
