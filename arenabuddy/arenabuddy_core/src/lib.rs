#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

pub mod cards;
pub mod display;
pub mod errors;
pub mod events;
pub mod models;
pub mod player_log;

pub use errors::{Error, Result};
