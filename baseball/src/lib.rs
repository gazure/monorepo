#![expect(clippy::type_complexity)]

mod baseball;
mod game;

pub use baseball::*;
pub use game::start::run;
