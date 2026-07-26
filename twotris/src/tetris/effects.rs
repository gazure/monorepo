//! Juice: line-clear confetti, lock flashes, floating score popups, screen
//! shake, and the drifting tetrominoes behind the title screen.

use bevy::{prelude::*, sprite::Anchor};

use super::{
    Fonts, LinesCleared, PieceLocked, RandomSource,
    board::COLS,
    piece::KINDS,
    render::{BOARD_W, Z_EFFECT, cell_offset},
    theme::{self, CELL},
};

/// Accumulated shake, in the range 0..=1. Decays on its own.
#[derive(Debug, Default, Resource)]
pub struct ScreenShake {
    trauma: f32,
}

impl ScreenShake {
    pub fn add(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).clamp(0.0, 1.0);
    }
}

#[derive(Debug, Component)]
pub struct Particle {
    velocity: Vec2,
    life: f32,
    max_life: f32,
    size: f32,
}

#[derive(Debug, Component)]
pub struct Popup {
    life: f32,
    max_life: f32,
}

/// A sprite that fades out, optionally growing as it goes.
#[derive(Debug, Component)]
pub struct Flash {
    life: f32,
    max_life: f32,
    growth: Vec2,
    size: Vec2,
}

#[derive(Debug, Component)]
pub struct TitlePiece {
    speed: f32,
    spin: f32,
}

/// Vertical extent the title decorations wrap around.
const TITLE_WRAP: f32 = 480.0;

pub fn spawn_clear_effects(
    mut commands: Commands,
    mut messages: MessageReader<LinesCleared>,
    mut random: ResMut<RandomSource>,
    fonts: Res<Fonts>,
) {
    for event in messages.read() {
        let Some(&top_row) = event.rows.iter().min() else {
            continue;
        };

        commands.entity(event.board).with_children(|board| {
            for (row, swatch) in event.rows.iter().zip(&event.swatches) {
                let y = cell_offset(0, *row).y;

                // A bright bar across the row that widens as it fades.
                board.spawn((
                    Flash {
                        life: 0.34,
                        max_life: 0.34,
                        growth: Vec2::new(46.0, 10.0),
                        size: Vec2::new(BOARD_W, CELL),
                    },
                    Sprite::from_color(Color::WHITE, Vec2::new(BOARD_W, CELL)),
                    Transform::from_xyz(0.0, y, Z_EFFECT),
                ));

                // Confetti in the colours of the blocks that were standing here.
                for (x, kind) in swatch.iter().enumerate() {
                    let Some(kind) = kind else { continue };
                    for _ in 0..2 {
                        let size = random.range(3.5, 7.0);
                        board.spawn((
                            Particle {
                                velocity: Vec2::new(random.range(-140.0, 140.0), random.range(40.0, 250.0)),
                                life: random.range(0.45, 0.95),
                                max_life: 0.95,
                                size,
                            },
                            Sprite::from_color(theme::piece_shine(*kind), Vec2::splat(size)),
                            Transform::from_xyz(
                                cell_offset(x, *row).x + random.range(-CELL / 3.0, CELL / 3.0),
                                y + random.range(-CELL / 3.0, CELL / 3.0),
                                Z_EFFECT,
                            ),
                        ));
                    }
                }
            }

            if let Some(label) = event.label {
                let color = if event.rows.len() >= 4 {
                    theme::ACCENT_WARM
                } else {
                    theme::ACCENT
                };
                board.spawn((
                    Popup {
                        life: 1.15,
                        max_life: 1.15,
                    },
                    Text2d::new(format!("{label}\n+{}", event.points)),
                    TextFont::from_font_size(26.0).with_font(fonts.bold.clone()),
                    TextColor(color),
                    TextLayout::justify(Justify::Center),
                    Anchor::CENTER,
                    Transform::from_xyz(0.0, cell_offset(0, top_row).y + CELL, Z_EFFECT + 1.0),
                ));
            }
        });
    }
}

pub fn spawn_lock_flash(mut commands: Commands, mut messages: MessageReader<PieceLocked>) {
    for event in messages.read() {
        commands.entity(event.board).with_children(|board| {
            for (x, y) in event.piece.cells() {
                if x < 0 || y < 0 || x >= COLS as i32 {
                    continue;
                }
                let offset = cell_offset(x as usize, y as usize);
                board.spawn((
                    Flash {
                        life: 0.16,
                        max_life: 0.16,
                        growth: Vec2::splat(14.0),
                        size: Vec2::splat(CELL),
                    },
                    Sprite::from_color(Color::WHITE, Vec2::splat(CELL)),
                    Transform::from_xyz(offset.x, offset.y, Z_EFFECT),
                ));
            }
        });
    }
}

pub fn drive_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform, mut sprite) in &mut particles {
        particle.life -= dt;
        if particle.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        particle.velocity.y -= 900.0 * dt;
        let velocity = particle.velocity;
        transform.translation += (velocity * dt).extend(0.0);

        let fade = (particle.life / particle.max_life).clamp(0.0, 1.0);
        sprite.color = theme::with_alpha(sprite.color, fade);
        sprite.custom_size = Some(Vec2::splat(particle.size * fade.max(0.2)));
    }
}

pub fn drive_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut popups: Query<(Entity, &mut Popup, &mut Transform, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut popup, mut transform, mut color) in &mut popups {
        popup.life -= dt;
        if popup.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = 1.0 - popup.life / popup.max_life;
        transform.translation.y += 42.0 * dt;
        // Pop in quickly, then hold, then fade out over the last third.
        let scale = 1.0 + 0.35 * (1.0 - progress).powi(3);
        transform.scale = Vec3::splat(scale);
        color.0 = theme::with_alpha(color.0, ((1.0 - progress) / 0.35).clamp(0.0, 1.0));
    }
}

pub fn drive_flashes(mut commands: Commands, time: Res<Time>, mut flashes: Query<(Entity, &mut Flash, &mut Sprite)>) {
    let dt = time.delta_secs();
    for (entity, mut flash, mut sprite) in &mut flashes {
        flash.life -= dt;
        if flash.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let progress = 1.0 - flash.life / flash.max_life;
        sprite.color = theme::with_alpha(Color::WHITE, (1.0 - progress) * 0.85);
        sprite.custom_size = Some(flash.size + flash.growth * progress);
    }
}

pub fn apply_screen_shake(
    time: Res<Time>,
    mut shake: ResMut<ScreenShake>,
    mut cameras: Query<&mut Transform, With<Camera2d>>,
) {
    shake.trauma = (shake.trauma - time.delta_secs() * 1.7).max(0.0);
    // Squaring makes small hits barely register and big ones land hard.
    let amount = shake.trauma * shake.trauma * 11.0;
    let t = time.elapsed_secs();
    for mut transform in &mut cameras {
        transform.translation.x = ((t * 47.0).sin() + (t * 31.0).sin() * 0.5) * amount;
        transform.translation.y = ((t * 41.0).cos() + (t * 23.0).cos() * 0.5) * amount;
    }
}

pub fn spawn_title_pieces(mut commands: Commands, mut random: ResMut<RandomSource>) {
    for _ in 0..18 {
        let kind = KINDS[random.next(0, KINDS.len() as u32) as usize];
        let (cells, width, height) = kind.preview_cells();
        let block = 24.0;

        commands
            .spawn((
                TitlePiece {
                    speed: random.range(20.0, 62.0),
                    spin: random.range(-0.45, 0.45),
                },
                Transform::from_xyz(
                    random.range(-640.0, 640.0),
                    random.range(-TITLE_WRAP, TITLE_WRAP),
                    -10.0,
                )
                .with_scale(Vec3::splat(random.range(0.5, 1.2)))
                .with_rotation(Quat::from_rotation_z(random.range(0.0, std::f32::consts::TAU))),
                Visibility::default(),
            ))
            .with_children(|piece| {
                // Dim enough that the wordmark and controls stay legible on top.
                let color = theme::with_alpha(theme::scale(theme::piece_color(kind), 0.30), 0.55);
                for (x, y) in cells {
                    piece.spawn((
                        Sprite::from_color(color, Vec2::splat(block - 3.0)),
                        Transform::from_xyz(
                            (x as f32 - (width as f32 - 1.0) / 2.0) * block,
                            -(y as f32 - (height as f32 - 1.0) / 2.0) * block,
                            0.0,
                        ),
                    ));
                }
            });
    }
}

pub fn drift_title_pieces(time: Res<Time>, mut pieces: Query<(&TitlePiece, &mut Transform)>) {
    let dt = time.delta_secs();
    for (piece, mut transform) in &mut pieces {
        transform.translation.y -= piece.speed * dt;
        if transform.translation.y < -TITLE_WRAP {
            transform.translation.y = TITLE_WRAP;
        }
        transform.rotate_z(piece.spin * dt);
    }
}
