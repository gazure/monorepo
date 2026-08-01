//! Juice: dust off the bat, screen shake, and the numbers that float away.

use bevy::{camera::visibility::RenderLayers, prelude::*};

use super::{GameScoped, RandomSource, ball::LiveBall, theme, view};

/// Something worth reacting to. Written by the pitch loop, read here, so the
/// gameplay code never has to know how an effect is drawn.
#[derive(Debug, Clone, Copy, Message)]
pub enum Splash {
    /// Bat on ball, with `0.0..1.0` for how flush it was.
    Contact {
        quality: f32,
    },
    /// Swung through it.
    Whiff,
    HomeRun,
}

/// Camera shake, as an amount of accumulated trauma that decays.
#[derive(Debug, Default, Resource)]
pub struct ScreenShake {
    pub trauma: f32,
}

impl ScreenShake {
    pub fn add(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
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

/// Turns splashes into particles and shake.
pub fn spawn_splashes(
    mut commands: Commands,
    mut splashes: MessageReader<Splash>,
    mut shake: ResMut<ScreenShake>,
    mut rng: ResMut<RandomSource>,
    live: Res<LiveBall>,
) {
    for splash in splashes.read() {
        match *splash {
            Splash::Contact { quality } => {
                shake.add(0.10 + quality * 0.30);
                // Dust at the plate, thrown along the ball's line.
                let count = 6 + (quality * 10.0) as usize;
                for _ in 0..count {
                    let spread = rng.range(-0.9, 0.9);
                    let speed = rng.range(14.0, 34.0) * (0.5 + quality);
                    let direction = live.vel.truncate().normalize_or_zero();
                    let angled = Vec2::new(
                        direction.x * spread.cos() - direction.y * spread.sin(),
                        direction.x * spread.sin() + direction.y * spread.cos(),
                    );
                    spawn_particle(
                        &mut commands,
                        Vec2::new(0.0, 2.0),
                        angled * speed,
                        rng.range(0.35, 0.8),
                        rng.range(1.4, 3.0),
                        theme::with_alpha(theme::INFIELD_DIRT, 0.85),
                    );
                }
            }
            Splash::Whiff => shake.add(0.05),
            Splash::HomeRun => shake.add(0.55),
        }
    }
}

fn spawn_particle(commands: &mut Commands, at: Vec2, velocity: Vec2, life: f32, size: f32, colour: Color) {
    commands.spawn((
        Sprite::from_color(colour, Vec2::splat(size)),
        Transform::from_xyz(at.x, at.y, 4.0),
        RenderLayers::layer(view::LAYER_FIELD),
        Particle {
            velocity,
            life,
            max_life: life,
            size,
        },
        GameScoped,
    ));
}

pub fn drive_particles(
    time: Res<Time>,
    mut commands: Commands,
    mut particles: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform, mut sprite) in particles.iter_mut() {
        particle.life -= dt;
        if particle.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        // Dust settles rather than flying flat.
        particle.velocity.y -= 42.0 * dt;
        particle.velocity *= 1.0 - 1.6 * dt;
        let step = particle.velocity * dt;
        transform.translation.x += step.x;
        transform.translation.y += step.y;

        let remaining = particle.life / particle.max_life;
        sprite.color = sprite.color.with_alpha(remaining);
        sprite.custom_size = Some(Vec2::splat(particle.size * remaining.max(0.25)));
    }
}

pub fn drive_popups(
    time: Res<Time>,
    mut commands: Commands,
    mut popups: Query<(Entity, &mut Popup, &mut Transform, &mut TextColor)>,
) {
    let dt = time.delta_secs();
    for (entity, mut popup, mut transform, mut colour) in popups.iter_mut() {
        popup.life -= dt;
        if popup.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation.y += 26.0 * dt;
        let remaining = popup.life / popup.max_life;
        // Fade over the last third only, so it is readable for most of its life.
        colour.0 = colour.0.with_alpha((remaining * 3.0).min(1.0));
    }
}

/// Nudges the field camera about, then settles it back. Quadratic falloff so a
/// big hit kicks hard and then calms quickly.
pub fn apply_screen_shake(
    time: Res<Time>,
    mut shake: ResMut<ScreenShake>,
    mut cameras: Query<&mut Transform, With<view::FieldCamera>>,
) {
    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };

    shake.trauma = (shake.trauma - time.delta_secs() * 1.5).max(0.0);
    let amount = shake.trauma * shake.trauma;
    let elapsed = time.elapsed_secs();
    let offset = Vec2::new((elapsed * 47.0).sin(), (elapsed * 61.0).cos()) * amount * 14.0;

    transform.translation.x = offset.x;
    transform.translation.y = super::field::VIEW_CENTER_Y + offset.y;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trauma_never_exceeds_one() {
        let mut shake = ScreenShake::default();
        for _ in 0..20 {
            shake.add(0.5);
        }
        assert!(shake.trauma <= 1.0);
    }

    #[test]
    fn flush_contact_shakes_harder_than_a_whiff() {
        let mut solid = ScreenShake::default();
        solid.add(0.10 + 1.0 * 0.30);
        let mut whiff = ScreenShake::default();
        whiff.add(0.05);
        assert!(solid.trauma > whiff.trauma);
    }
}
