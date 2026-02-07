pub(crate) mod auth;
pub(crate) mod grpc_writer;
pub(crate) mod ingest;
mod launch;
mod service;
pub(crate) mod update;

pub use auth::{SharedAuthState, new_shared_auth_state};
pub use launch::launch;
pub use update::{SharedUpdateState, new_shared_update_state};
pub type Service = service::AppService<arenabuddy_data::MatchDB>;
