//! Everything you can see: the ballpark, the players, and the ball.
//!
//! The field is drawn from the real dimensions in [`field`], so the bases, the
//! foul lines, the dirt and the wall are all derived from the same numbers the
//! simulation uses. The wedge shapes — fair territory, the warning track, the
//! wall — are procedural meshes, because a ballpark is mostly circular arcs and
//! rotated rectangles do not fake them convincingly.

use baseball_game_rules::Base;
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use super::{Diamond, Fielder, GameScoped, Phase, ball::LiveBall, field, pitch, theme, view};

// Depth ordering for the field view.
const Z_FOUL_GROUND: f32 = -6.0;
const Z_GRASS: f32 = -5.0;
const Z_STRIPE: f32 = -4.8;
const Z_WARNING: f32 = -4.6;
const Z_WALL: f32 = -4.4;
const Z_DIRT: f32 = -4.0;
const Z_INFIELD_GRASS: f32 = -3.8;
const Z_LINE: f32 = -3.0;
const Z_BASE: f32 = -2.5;
const Z_SHADOW: f32 = 1.0;
const Z_FIELDER: f32 = 2.0;
const Z_RUNNER: f32 = 2.5;
const Z_BALL: f32 = 3.0;

/// Root of the overhead field scene.
#[derive(Debug, Component)]
pub struct FieldScene;

/// Root of the behind-the-plate scene.
#[derive(Debug, Component)]
pub struct AtBatScene;

#[derive(Debug, Component)]
pub struct FieldBall;

#[derive(Debug, Component)]
pub struct BallShadow;

#[derive(Debug, Component)]
pub struct AtBatBall;

#[derive(Debug, Component)]
pub struct RunnerPip(pub Base);

#[derive(Debug, Component)]
pub struct PitchTarget;

#[derive(Debug, Component)]
pub struct ZoneFill;

// ---------------------------------------------------------------- mesh helpers

/// A polygon triangulated as a fan from `center`.
fn fan(center: Vec2, boundary: &[Vec2]) -> Mesh {
    let mut positions = Vec::with_capacity(boundary.len() + 1);
    positions.push([center.x, center.y, 0.0]);
    positions.extend(boundary.iter().map(|p| [p.x, p.y, 0.0]));

    let mut indices = Vec::new();
    for i in 1..boundary.len() {
        indices.extend([0u32, i as u32, i as u32 + 1]);
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U32(indices))
}

/// A band between two radii, both of which may vary with the angle. Used for the
/// warning track, the wall, and the mown stripes in the outfield.
fn band(
    from_angle: f32,
    to_angle: f32,
    inner: impl Fn(f32) -> f32,
    outer: impl Fn(f32) -> f32,
    segments: usize,
) -> Mesh {
    let mut positions = Vec::with_capacity((segments + 1) * 2);
    for step in 0..=segments {
        let angle = from_angle + (to_angle - from_angle) * step as f32 / segments as f32;
        let near = field::point_at(angle, inner(angle));
        let far = field::point_at(angle, outer(angle));
        positions.push([near.x, near.y, 0.0]);
        positions.push([far.x, far.y, 0.0]);
    }

    let mut indices = Vec::new();
    for step in 0..segments as u32 {
        let base = step * 2;
        indices.extend([base, base + 1, base + 3, base, base + 3, base + 2]);
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U32(indices))
}

/// The arc of fair territory, out to the wall.
fn fair_territory_mesh(segments: usize) -> Mesh {
    let boundary: Vec<Vec2> = (0..=segments)
        .map(|step| {
            let angle = -field::FOUL_LINE_ANGLE + 2.0 * field::FOUL_LINE_ANGLE * step as f32 / segments as f32;
            field::point_at(angle, field::fence_distance(angle))
        })
        .collect();
    fan(field::HOME, &boundary)
}

/// A thin line between two points, as a rotated rectangle.
fn line_between(from: Vec2, to: Vec2, width: f32, color: Color, z: f32, layer: usize) -> impl Bundle {
    let delta = to - from;
    let midpoint = from + delta / 2.0;
    (
        Sprite::from_color(color, Vec2::new(delta.length(), width)),
        Transform::from_xyz(midpoint.x, midpoint.y, z).with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x))),
        RenderLayers::layer(layer),
    )
}

// ---------------------------------------------------------------- field scene

fn build_field(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<ColorMaterial>) {
    let layer = view::LAYER_FIELD;
    let root = commands
        .spawn((FieldScene, GameScoped, Transform::default(), Visibility::default()))
        .id();

    let mut spawn = |bundle: (Mesh2d, MeshMaterial2d<ColorMaterial>, Transform)| {
        commands.spawn((bundle.0, bundle.1, bundle.2, RenderLayers::layer(layer), ChildOf(root)));
    };

    // Foul ground, covering the whole view behind everything else.
    spawn((
        Mesh2d(meshes.add(Rectangle::new(field::VIEW_WIDTH * 1.4, field::VIEW_HEIGHT * 1.6))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::scale(theme::GRASS, 0.72)))),
        Transform::from_xyz(0.0, field::VIEW_CENTER_Y, Z_FOUL_GROUND),
    ));

    // Fair territory.
    spawn((
        Mesh2d(meshes.add(fair_territory_mesh(64))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::GRASS))),
        Transform::from_xyz(0.0, 0.0, Z_GRASS),
    ));

    // Mown stripes: alternating bands of lighter grass, following the arc of the
    // outfield rather than running straight, which is what a real cut looks like.
    let stripe_material = materials.add(ColorMaterial::from(theme::GRASS_LIGHT));
    for index in 0..7 {
        let inner = 130.0 + index as f32 * 36.0;
        if index % 2 == 1 {
            continue;
        }
        let outer = inner + 36.0;
        spawn((
            Mesh2d(meshes.add(band(
                -field::FOUL_LINE_ANGLE,
                field::FOUL_LINE_ANGLE,
                move |_| inner,
                move |angle| outer.min(field::fence_distance(angle) - 14.0),
                48,
            ))),
            MeshMaterial2d(stripe_material.clone()),
            Transform::from_xyz(0.0, 0.0, Z_STRIPE),
        ));
    }

    // Warning track, then the wall itself just outside it.
    spawn((
        Mesh2d(meshes.add(band(
            -field::FOUL_LINE_ANGLE,
            field::FOUL_LINE_ANGLE,
            |angle| field::fence_distance(angle) - 14.0,
            field::fence_distance,
            64,
        ))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::WARNING_TRACK))),
        Transform::from_xyz(0.0, 0.0, Z_WARNING),
    ));
    spawn((
        Mesh2d(meshes.add(band(
            -field::FOUL_LINE_ANGLE,
            field::FOUL_LINE_ANGLE,
            field::fence_distance,
            |angle| field::fence_distance(angle) + 7.0,
            64,
        ))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::WALL))),
        Transform::from_xyz(0.0, 0.0, Z_WALL),
    ));
    spawn((
        Mesh2d(meshes.add(band(
            -field::FOUL_LINE_ANGLE,
            field::FOUL_LINE_ANGLE,
            |angle| field::fence_distance(angle) + 7.0,
            |angle| field::fence_distance(angle) + 9.5,
            64,
        ))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::WALL_CAP))),
        Transform::from_xyz(0.0, 0.0, Z_WALL + 0.1),
    ));

    // Infield dirt, bounded by the foul lines, with the grass diamond inside it.
    spawn((
        Mesh2d(
            meshes.add(fan(
                field::HOME,
                &(0..=48)
                    .map(|step| {
                        let angle = -field::FOUL_LINE_ANGLE + 2.0 * field::FOUL_LINE_ANGLE * step as f32 / 48.0;
                        field::point_at(angle, field::INFIELD_DIRT_RADIUS + 32.0)
                    })
                    .collect::<Vec<_>>(),
            )),
        ),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::INFIELD_DIRT))),
        Transform::from_xyz(0.0, 0.0, Z_DIRT),
    ));

    // The grass inside the base paths: the diamond, pulled in from each bag.
    let inset = 13.0;
    let centre = Vec2::new(0.0, field::SECOND.y / 2.0);
    let diamond: Vec<Vec2> = [field::HOME, field::FIRST, field::SECOND, field::THIRD, field::HOME]
        .iter()
        .map(|&corner| corner + (centre - corner).normalize_or_zero() * inset)
        .collect();
    spawn((
        Mesh2d(meshes.add(fan(centre, &diamond))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::GRASS))),
        Transform::from_xyz(0.0, 0.0, Z_INFIELD_GRASS),
    ));

    // Pitcher's mound.
    spawn((
        Mesh2d(meshes.add(Circle::new(9.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::MOUND_DIRT))),
        Transform::from_xyz(field::MOUND.x, field::MOUND.y, Z_DIRT + 0.1),
    ));

    // Foul lines, drawn all the way to the wall.
    for side in [-1.0, 1.0] {
        let angle = side * field::FOUL_LINE_ANGLE;
        let end = field::point_at(angle, field::fence_distance(angle));
        commands.spawn((
            line_between(field::HOME, end, 1.6, theme::CHALK, Z_LINE, layer),
            ChildOf(root),
        ));
    }

    // Bases, and the plate.
    for base in [Base::First, Base::Second, Base::Third] {
        let spot = field::base_position(base);
        commands.spawn((
            Sprite::from_color(theme::CHALK, Vec2::splat(5.0)),
            Transform::from_xyz(spot.x, spot.y, Z_BASE)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            RenderLayers::layer(layer),
            ChildOf(root),
        ));
    }
    commands.spawn((
        Sprite::from_color(theme::CHALK, Vec2::splat(4.2)),
        Transform::from_xyz(field::HOME.x, field::HOME.y, Z_BASE)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
        RenderLayers::layer(layer),
        ChildOf(root),
    ));

    // The nine defenders.
    let fielder_mesh = meshes.add(Circle::new(4.0));
    let fielder_material = materials.add(ColorMaterial::from(theme::AWAY_UNIFORM));
    for (position, home) in field::FIELDER_HOMES {
        commands.spawn((
            Mesh2d(fielder_mesh.clone()),
            MeshMaterial2d(fielder_material.clone()),
            Transform::from_xyz(home.x, home.y, Z_FIELDER),
            RenderLayers::layer(layer),
            Fielder {
                position,
                home,
                target: None,
            },
            ChildOf(root),
        ));
    }

    // Baserunner pips, hidden until somebody is standing there.
    let runner_mesh = meshes.add(Circle::new(4.6));
    let runner_material = materials.add(ColorMaterial::from(theme::BASE_OCCUPIED));
    for base in [Base::First, Base::Second, Base::Third] {
        let spot = field::base_position(base);
        commands.spawn((
            Mesh2d(runner_mesh.clone()),
            MeshMaterial2d(runner_material.clone()),
            Transform::from_xyz(spot.x, spot.y, Z_RUNNER),
            Visibility::Hidden,
            RenderLayers::layer(layer),
            RunnerPip(base),
            ChildOf(root),
        ));
    }

    // The ball, and the shadow that tells you how high it is.
    commands.spawn((
        Mesh2d(meshes.add(Ellipse::new(3.4, 2.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::BALL_SHADOW))),
        Transform::from_xyz(0.0, 0.0, Z_SHADOW),
        Visibility::Hidden,
        RenderLayers::layer(layer),
        BallShadow,
        ChildOf(root),
    ));
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(2.6))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::BALL))),
        Transform::from_xyz(0.0, 0.0, Z_BALL),
        Visibility::Hidden,
        RenderLayers::layer(layer),
        FieldBall,
        ChildOf(root),
    ));
}

// ---------------------------------------------------------------- at-bat scene

/// Vertical bands of the behind-the-plate view.
const HORIZON: f32 = 36.0;
const WALL_TOP: f32 = 48.0;

/// Spawns a flat coloured rectangle into the at-bat scene. A function rather than
/// a closure so it does not hold a borrow of `Commands` across the whole builder.
fn at_bat_flat(commands: &mut Commands, root: Entity, sprite: Sprite, transform: Transform) {
    commands.spawn((
        sprite,
        transform,
        RenderLayers::layer(view::LAYER_AT_BAT),
        ChildOf(root),
    ));
}

fn build_at_bat(commands: &mut Commands, meshes: &mut Assets<Mesh>, materials: &mut Assets<ColorMaterial>) {
    let layer = view::LAYER_AT_BAT;
    let root = commands
        .spawn((AtBatScene, GameScoped, Transform::default(), Visibility::default()))
        .id();

    let wide = view::AT_BAT_WIDTH * 1.4;

    // --- the far distance: sky, stands, crowd, outfield wall ---
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(theme::SKY, Vec2::new(wide, 260.0)),
        Transform::from_xyz(0.0, 150.0, -10.0),
    );
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(theme::STANDS, Vec2::new(wide, 74.0)),
        Transform::from_xyz(0.0, 93.0, -9.0),
    );
    for index in 0..108 {
        let column = index % 27;
        let row = index / 27;
        let x = -212.0 + column as f32 * 16.2 + (row as f32 * 5.5);
        let y = WALL_TOP + 12.0 + row as f32 * 17.0;
        let shade = if index % 3 == 0 {
            theme::CROWD_DARK
        } else {
            theme::scale(theme::CROWD_DARK, 1.55)
        };
        at_bat_flat(
            commands,
            root,
            Sprite::from_color(shade, Vec2::splat(4.0)),
            Transform::from_xyz(x, y, -8.5),
        );
    }
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(theme::WALL, Vec2::new(wide, WALL_TOP - HORIZON)),
        Transform::from_xyz(0.0, f32::midpoint(WALL_TOP, HORIZON), -8.0),
    );
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(theme::WALL_CAP, Vec2::new(wide, 2.0)),
        Transform::from_xyz(0.0, WALL_TOP, -7.9),
    );

    // --- the middle distance: outfield grass, then the infield dirt arc ---
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(theme::GRASS, Vec2::new(wide, 300.0)),
        Transform::from_xyz(0.0, HORIZON - 150.0, -7.5),
    );
    // Flattened and dropped clear of the horizon: at full height its top edge met
    // the wall exactly, leaving no outfield grass on screen at all.
    commands.spawn((
        Mesh2d(meshes.add(Ellipse::new(340.0, 18.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::INFIELD_DIRT))),
        Transform::from_xyz(0.0, -6.0, -7.0),
        RenderLayers::layer(layer),
        ChildOf(root),
    ));

    // The mound, and the pitcher on top of it. He is sixty feet away, so he is
    // small — but not so small that you cannot see him come set.
    commands.spawn((
        Mesh2d(meshes.add(Ellipse::new(42.0, 11.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::MOUND_DIRT))),
        Transform::from_xyz(0.0, -4.0, -6.6),
        RenderLayers::layer(layer),
        ChildOf(root),
    ));
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(theme::AWAY_UNIFORM, Vec2::new(11.0, 26.0)),
        Transform::from_xyz(0.0, 9.0, -6.0),
    );
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(theme::AWAY_TRIM, Vec2::new(11.0, 5.0)),
        Transform::from_xyz(0.0, 21.0, -5.9),
    );

    // --- the near ground: the dirt circle around home plate ---
    // Without this the bottom third of the screen is flat green, which reads as
    // the batter standing in the outfield.
    commands.spawn((
        Mesh2d(meshes.add(Ellipse::new(235.0, 72.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::INFIELD_DIRT))),
        Transform::from_xyz(0.0, -150.0, -5.5),
        RenderLayers::layer(layer),
        ChildOf(root),
    ));
    // Chalk arc of the batter's box, just enough to place the plate in space.
    commands.spawn((
        Mesh2d(meshes.add(Ellipse::new(31.0, 8.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::CHALK))),
        Transform::from_xyz(0.0, -100.0, -5.0),
        RenderLayers::layer(layer),
        ChildOf(root),
    ));

    // --- the strike zone ---
    let zone_size = Vec2::new(
        pitch::ZONE_HALF_WIDTH * 2.0 * view::AT_BAT_SCALE,
        (pitch::ZONE_TOP - pitch::ZONE_BOTTOM) * view::AT_BAT_SCALE,
    );
    commands.spawn((
        Sprite::from_color(theme::ZONE, zone_size),
        Transform::from_xyz(view::ZONE_CENTER.x, view::ZONE_CENTER.y, -4.0),
        RenderLayers::layer(layer),
        ZoneFill,
        ChildOf(root),
    ));
    for (offset, size) in [
        (Vec2::new(0.0, zone_size.y / 2.0), Vec2::new(zone_size.x, 1.4)),
        (Vec2::new(0.0, -zone_size.y / 2.0), Vec2::new(zone_size.x, 1.4)),
        (Vec2::new(-zone_size.x / 2.0, 0.0), Vec2::new(1.4, zone_size.y)),
        (Vec2::new(zone_size.x / 2.0, 0.0), Vec2::new(1.4, zone_size.y)),
    ] {
        at_bat_flat(
            commands,
            root,
            Sprite::from_color(theme::ZONE_EDGE, size),
            Transform::from_xyz(view::ZONE_CENTER.x + offset.x, view::ZONE_CENTER.y + offset.y, -3.9),
        );
    }

    // --- the batter, on the third-base side, and the catcher crouched below ---
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(theme::HOME_UNIFORM, Vec2::new(17.0, 56.0)),
        Transform::from_xyz(-64.0, -74.0, -2.0),
    );
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(theme::HOME_TRIM, Vec2::new(17.0, 7.0)),
        Transform::from_xyz(-64.0, -48.0, -1.9),
    );
    at_bat_flat(
        commands,
        root,
        Sprite::from_color(Color::srgb(0.62, 0.44, 0.24), Vec2::new(4.0, 48.0)),
        Transform::from_xyz(-47.0, -44.0, -1.8).with_rotation(Quat::from_rotation_z(-0.5)),
    );

    // The catcher: a crouched body with a mitt held up in the zone, so the pitch
    // has somewhere to finish rather than vanishing off the bottom of the screen.
    commands.spawn((
        Mesh2d(meshes.add(Ellipse::new(28.0, 14.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::scale(theme::AWAY_UNIFORM, 0.62)))),
        Transform::from_xyz(0.0, -136.0, -2.0),
        RenderLayers::layer(layer),
        ChildOf(root),
    ));
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(7.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.45, 0.29, 0.18)))),
        Transform::from_xyz(0.0, -116.0, -1.5),
        RenderLayers::layer(layer),
        ChildOf(root),
    ));

    // --- the pitcher's target, shown while the human is choosing ---
    for size in [Vec2::new(17.0, 2.2), Vec2::new(2.2, 17.0)] {
        commands.spawn((
            Sprite::from_color(theme::TARGET, size),
            Transform::from_xyz(0.0, view::ZONE_CENTER.y, 4.0),
            Visibility::Hidden,
            RenderLayers::layer(layer),
            PitchTarget,
            ChildOf(root),
        ));
    }

    // --- the ball on its way in ---
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(5.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(theme::BALL))),
        Transform::from_xyz(0.0, 0.0, 5.0),
        Visibility::Hidden,
        RenderLayers::layer(layer),
        AtBatBall,
        ChildOf(root),
    ));
}

/// Builds both scenes if they are not already there. Safe to call on every entry
/// to the windup, which is what makes restarting a game a matter of despawning.
pub fn ensure_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    existing: Query<Entity, With<FieldScene>>,
) {
    if !existing.is_empty() {
        return;
    }
    build_field(&mut commands, &mut meshes, &mut materials);
    build_at_bat(&mut commands, &mut meshes, &mut materials);
}

// ---------------------------------------------------------------- drawing

/// Places the ball in the field view, with a shadow whose offset and size tell
/// you how high it is.
pub fn draw_ball(
    live: Res<LiveBall>,
    phase: Res<State<Phase>>,
    mut ball: Query<(&mut Transform, &mut Visibility), (With<FieldBall>, Without<BallShadow>)>,
    mut shadow: Query<(&mut Transform, &mut Visibility), (With<BallShadow>, Without<FieldBall>)>,
) {
    let visible = live.live && *phase.get() == Phase::BallInPlay;

    if let Ok((mut transform, mut visibility)) = ball.single_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        // Height lifts the ball up the screen a little and makes it bigger, which
        // is the only cue an overhead view has for altitude.
        let lift = live.height() * 0.18;
        transform.translation = Vec3::new(live.pos.x, live.pos.y + lift, Z_BALL);
        let scale = 1.0 + (live.height() / 90.0).min(1.2);
        transform.scale = Vec3::splat(scale);
    }

    if let Ok((mut transform, mut visibility)) = shadow.single_mut() {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        transform.translation = Vec3::new(live.pos.x, live.pos.y, Z_SHADOW);
        // The shadow spreads and fades as the ball climbs.
        let spread = 1.0 + (live.height() / 60.0).min(1.6);
        transform.scale = Vec3::new(spread, spread, 1.0);
    }
}

/// Walks each fielder towards wherever they are meant to be.
pub fn draw_fielders(time: Res<Time>, mut fielders: Query<(&Fielder, &mut Transform)>) {
    for (fielder, mut transform) in fielders.iter_mut() {
        let goal = fielder.target.unwrap_or(fielder.home);
        let here = transform.translation.truncate();
        let step = field::FIELDER_SPEED * time.delta_secs();
        let next = if here.distance(goal) <= step {
            goal
        } else {
            here + (goal - here).normalize_or_zero() * step
        };
        transform.translation = Vec3::new(next.x, next.y, Z_FIELDER);
    }
}

/// Shows a pip on each occupied base.
pub fn draw_runners(diamond: Res<Diamond>, mut pips: Query<(&RunnerPip, &mut Visibility)>) {
    let runners = diamond.game().map(|game| game.current_half_inning().baserunners());
    for (pip, mut visibility) in pips.iter_mut() {
        let occupied = runners.is_some_and(|state| match pip.0 {
            Base::First => state.first().is_some(),
            Base::Second => state.second().is_some(),
            Base::Third => state.third().is_some(),
            Base::Home => false,
        });
        *visibility = if occupied {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Draws the incoming pitch and the pitcher's target in the at-bat view.
pub fn draw_at_bat(
    phase: Res<State<Phase>>,
    diamond: Res<Diamond>,
    plan: Res<pitch::PitchPlan>,
    live_pitch: Res<pitch::LivePitch>,
    mut ball: Query<(&mut Transform, &mut Visibility), (With<AtBatBall>, Without<PitchTarget>)>,
    mut target: Query<(&mut Transform, &mut Visibility), (With<PitchTarget>, Without<AtBatBall>)>,
) {
    let in_flight = *phase.get() == Phase::Pitch;

    if let Ok((mut transform, mut visibility)) = ball.single_mut() {
        *visibility = if in_flight {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if in_flight {
            // Drawn travelling from the pitcher's hand to the plate rather than
            // from the release point mapped through `at_bat_point`, which would
            // put the ball above the outfield wall before it was thrown.
            //
            // The easing is the depth cue: the ball seems to hang while it is far
            // away and then cover the last few feet all at once, which is what
            // makes the timing window feel like a real pitch. It is allowed to run
            // past 1.0 so the ball carries on into the catcher's mitt.
            let progress = live_pitch.progress();
            let eased = progress.powf(1.8);
            let plate = view::at_bat_point(live_pitch.spot_at(progress.min(1.0)));
            let spot = view::PITCHER_HAND.lerp(plate, eased);
            transform.translation = Vec3::new(spot.x, spot.y, 5.0);
            transform.scale = Vec3::splat(0.30 + eased.min(1.4) * 1.05);
        }
    }

    // The reticle only helps the player who is choosing the pitch.
    let show_target = *phase.get() == Phase::Windup && !diamond.human_is_batting();
    for (mut transform, mut visibility) in target.iter_mut() {
        *visibility = if show_target {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let spot = view::at_bat_point(plan.target);
        transform.translation = Vec3::new(spot.x, spot.y, 4.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fan_mesh_has_one_triangle_per_boundary_edge() {
        let boundary = vec![
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-1.0, 0.0),
        ];
        let mesh = fan(Vec2::ZERO, &boundary);
        let indices = mesh.indices().expect("fan should be indexed");
        // Four boundary points make three edges, so three triangles.
        assert_eq!(indices.len(), 3 * 3);
    }

    #[test]
    fn a_band_mesh_has_two_triangles_per_segment() {
        let mesh = band(-1.0, 1.0, |_| 10.0, |_| 20.0, 8);
        let indices = mesh.indices().expect("band should be indexed");
        assert_eq!(indices.len(), 8 * 6);
    }

    #[test]
    fn the_fair_territory_mesh_reaches_the_wall_in_every_direction() {
        let mesh = fair_territory_mesh(32);
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("positions")
            .as_float3()
            .expect("float3");

        // Skip the apex at home plate; every other vertex should be on the wall.
        for point in positions.iter().skip(1) {
            let spot = Vec2::new(point[0], point[1]);
            let expected = field::fence_distance(field::spray_angle(spot));
            assert!(
                (spot.length() - expected).abs() < 0.5,
                "vertex at {spot:?} is {} from home, wall is {expected}",
                spot.length()
            );
        }
    }

    #[test]
    fn the_bases_the_scene_draws_are_the_ones_the_simulation_uses() {
        // The whole point of deriving the layout from `field`: there is no second
        // set of coordinates that could drift.
        assert_eq!(field::base_position(Base::First), field::FIRST);
        assert_eq!(field::base_position(Base::Second), field::SECOND);
        assert_eq!(field::base_position(Base::Third), field::THIRD);
    }

    #[test]
    fn the_infield_dirt_stays_inside_the_outfield_grass() {
        // The dirt arc is drawn at a fixed radius; if it ever reached the wall the
        // outfield would vanish underneath it.
        let dirt = field::INFIELD_DIRT_RADIUS + 32.0;
        let nearest_wall = field::fence_distance(field::FOUL_LINE_ANGLE);
        assert!(
            dirt < nearest_wall,
            "infield dirt at {dirt} reaches the {nearest_wall} wall"
        );
    }
}
