#![deny(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::needless_pass_by_value)]
// Bevy systems take resources and queries by value, and this is a game full of
// grid-index-to-world-position arithmetic; the numeric casts are all bounded by
// the board dimensions.
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::too_many_lines)]

use bevy::{prelude::*, window::WindowResolution};

mod tetris;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Twotris".into(),
                        resolution: WindowResolution::new(1180, 820),
                        ..default()
                    }),
                    ..default()
                })
                // Crisp block edges: the art is all axis-aligned rectangles.
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(tetris::TetrisPlugin)
        .run();
}
