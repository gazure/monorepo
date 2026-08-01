// Bevy systems take their resources and queries by value, and a ball game is
// wall-to-wall arithmetic converting between physical units and screen space; the
// casts involved are all bounded by field dimensions.
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::type_complexity)]

mod game;

pub use game::start::run;
