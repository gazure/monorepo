#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::too_many_arguments)]

use bevy::prelude::*;

mod tetris;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(tetris::TetrisPlugin)
        .run();
}
