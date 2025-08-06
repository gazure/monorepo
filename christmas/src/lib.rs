mod app;
#[cfg(feature = "server")]
mod database;
mod error;
mod model;
pub mod server;

pub use app::app;
pub(crate) use error::Result;
