//! The two cameras, and switching between them.
//!
//! Each view owns a render layer and only ever sees its own entities, so the two
//! scenes can both sit at whatever coordinates suit them without colliding. Only
//! one of the two is active at a time; whichever it is clears the screen. A third
//! camera exists solely to give the HUD something stable to draw onto, since with
//! several cameras in play `bevy_ui` needs to be told which one it belongs to.
//!
//! Both projections use [`ScalingMode::AutoMin`], which guarantees a minimum
//! region of the world stays visible however the window is shaped. The previous
//! camera was a bare `Camera2d`, so the layout only lined up at one resolution.

use bevy::{
    camera::{Hdr, ScalingMode, visibility::RenderLayers},
    post_process::bloom::Bloom,
    prelude::*,
    ui::IsDefaultUiCamera,
};

use super::{field, theme};

/// Render layer for the overhead field scene.
pub const LAYER_FIELD: usize = 1;
/// Render layer for the behind-the-plate scene.
pub const LAYER_AT_BAT: usize = 2;

/// The at-bat view's world region. Arbitrary units — see [`AT_BAT_SCALE`].
pub const AT_BAT_WIDTH: f32 = 440.0;
pub const AT_BAT_HEIGHT: f32 = 330.0;

/// World units per foot in the at-bat view, chosen so the strike zone is a
/// comfortable size on screen.
pub const AT_BAT_SCALE: f32 = 48.0;

/// Where the middle of the strike zone sits in at-bat coordinates. Low on the
/// screen, because the plate is the near end of the view: the pitcher and the
/// stands are above it, and the batter and catcher below.
pub const ZONE_CENTER: Vec2 = Vec2::new(0.0, -30.0);

/// Where the ball appears to leave the pitcher's hand, in at-bat coordinates.
///
/// The release point cannot be derived through [`at_bat_point`]: that scale is
/// calibrated for the plate, which is a few feet from the camera, and applying it
/// to something sixty feet away puts the ball up in the crowd. The pitch is drawn
/// travelling from here instead.
pub const PITCHER_HAND: Vec2 = Vec2::new(11.0, 27.0);

/// Converts a plate coordinate (feet across, feet up) into at-bat world space.
pub fn at_bat_point(plate: Vec2) -> Vec2 {
    ZONE_CENTER + Vec2::new(plate.x, plate.y - super::pitch::ZONE_MID) * AT_BAT_SCALE
}

#[derive(Debug, Component)]
pub struct FieldCamera;

#[derive(Debug, Component)]
pub struct AtBatCamera;

#[derive(Debug, Component)]
pub struct HudCamera;

fn glow() -> Bloom {
    Bloom {
        intensity: 0.18,
        low_frequency_boost: 0.55,
        ..Bloom::NATURAL
    }
}

pub fn spawn_cameras(mut commands: Commands) {
    // Overhead field view. One world unit is one foot, so everything in `field`
    // can be used directly as a coordinate.
    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            is_active: false,
            clear_color: ClearColorConfig::Custom(theme::GRASS),
            ..default()
        },
        // The palette pushes a few highlights above 1.0 in linear space; without
        // an HDR target those would clip and bloom would have nothing to catch.
        Hdr,
        glow(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: field::VIEW_WIDTH,
                min_height: field::VIEW_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(0.0, field::VIEW_CENTER_Y, 0.0),
        RenderLayers::layer(LAYER_FIELD),
        FieldCamera,
    ));

    // Behind the catcher.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            is_active: true,
            clear_color: ClearColorConfig::Custom(theme::SKY),
            ..default()
        },
        Hdr,
        glow(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: AT_BAT_WIDTH,
                min_height: AT_BAT_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
        RenderLayers::layer(LAYER_AT_BAT),
        AtBatCamera,
    ));

    // HUD host. Always active, never clears, sits on top of both scenes.
    //
    // Deliberately left on the *default* render layer rather than given one of
    // its own: `bevy_ui` nodes carry no `RenderLayers`, so they live on layer 0,
    // and a camera restricted to any other layer culls the entire interface. It
    // still sees only the UI, because everything in the two scenes is explicitly
    // pinned to layer 1 or 2.
    //
    // It also carries `Hdr` to match the scene cameras. All three draw to the same
    // window, and a camera that disagrees about the target format leaves the
    // surface in a state the others cannot composite onto.
    //
    // `IsDefaultUiCamera` is what lets every overlay be spawned without knowing
    // this entity exists. That matters more than it sounds: `bevy_state` runs the
    // first state transition before `Startup`, so `OnEnter(Title)` builds the title
    // screen before this system has run at all, and anything that had to look the
    // camera up would find nothing and silently draw an empty screen.
    commands.spawn((
        Camera2d,
        Camera {
            order: 10,
            is_active: true,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Hdr,
        IsDefaultUiCamera,
        HudCamera,
    ));
}

/// Which of the two scene cameras is live.
fn activate(field_on: bool, cameras: &mut Query<(&mut Camera, Has<FieldCamera>), Without<HudCamera>>) {
    for (mut camera, is_field) in cameras.iter_mut() {
        camera.is_active = is_field == field_on;
    }
}

pub fn show_field(mut cameras: Query<(&mut Camera, Has<FieldCamera>), Without<HudCamera>>) {
    activate(true, &mut cameras);
}

pub fn show_at_bat(mut cameras: Query<(&mut Camera, Has<FieldCamera>), Without<HudCamera>>) {
    activate(false, &mut cameras);
}

#[cfg(test)]
mod tests {
    use super::{super::pitch, *};

    #[test]
    fn the_middle_of_the_zone_maps_to_the_zone_centre() {
        let middle = at_bat_point(Vec2::new(0.0, pitch::ZONE_MID));
        assert!(middle.distance(ZONE_CENTER) < 1e-4);
    }

    #[test]
    fn a_pitch_to_the_right_of_the_plate_draws_to_the_right() {
        // The at-bat view looks out towards the pitcher, so the field's +x — the
        // first base side — is the viewer's right.
        let outside = at_bat_point(Vec2::new(0.8, pitch::ZONE_MID));
        let inside = at_bat_point(Vec2::new(-0.8, pitch::ZONE_MID));
        assert!(outside.x > 0.0);
        assert!(inside.x < 0.0);
        assert!((outside.x + inside.x).abs() < 1e-4, "symmetric about the middle");
    }

    #[test]
    fn a_high_pitch_draws_above_a_low_one() {
        let high = at_bat_point(Vec2::new(0.0, pitch::ZONE_TOP));
        let low = at_bat_point(Vec2::new(0.0, pitch::ZONE_BOTTOM));
        assert!(high.y > low.y);
    }

    #[test]
    fn the_whole_strike_zone_fits_comfortably_on_screen() {
        let half_width = AT_BAT_WIDTH / 2.0;
        let half_height = AT_BAT_HEIGHT / 2.0;

        for x in [-pitch::AIM_LIMIT_X, pitch::AIM_LIMIT_X] {
            for y in [pitch::AIM_LIMIT_LOW, pitch::AIM_LIMIT_HIGH] {
                let point = at_bat_point(Vec2::new(x, y));
                assert!(
                    point.x.abs() < half_width,
                    "a pitch aimed at {x},{y} is off the side of the at-bat view"
                );
                assert!(
                    point.y.abs() < half_height,
                    "a pitch aimed at {x},{y} is off the top or bottom of the at-bat view"
                );
            }
        }
    }

    #[test]
    fn the_two_scenes_are_on_different_layers_and_neither_is_the_default() {
        // Sharing a layer would leak one scene into the other. Neither may be
        // layer 0, because that is where `bevy_ui` puts the interface and where
        // the HUD camera therefore has to look.
        assert_ne!(LAYER_FIELD, LAYER_AT_BAT);
        assert_ne!(LAYER_FIELD, 0);
        assert_ne!(LAYER_AT_BAT, 0);
    }
}
