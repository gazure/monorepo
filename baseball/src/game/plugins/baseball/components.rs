use bevy::prelude::*;

#[derive(Component)]
pub struct Ball;

#[derive(Component)]
pub struct BallVelocity {
    pub v: Vec3,
}

impl BallVelocity {
    pub fn zero() -> Self {
        BallVelocity { v: Vec3::ZERO }
    }

    pub fn set(&mut self, v: Vec3) {
        self.v = v;
    }
}

#[derive(Component)]
pub struct PitcherMound;

#[derive(Component)]
pub struct HomePlate;

#[derive(Component)]
#[expect(dead_code)]
pub struct Player(pub baseball_game_rules::PlayerPosition);

#[derive(Component)]
pub struct Batter;

#[derive(Component)]
pub struct BaseMarker;

#[derive(Component)]
pub struct FirstBaseRunner;

#[derive(Component)]
pub struct SecondBaseRunner;

#[derive(Component)]
pub struct ThirdBaseRunner;

#[derive(Component)]
pub struct ScoreText;

#[derive(Component)]
pub struct InningText;

#[derive(Component)]
pub struct CountText;

#[derive(Component)]
pub struct InstructionText;

#[derive(Component)]
pub struct ResultText;
