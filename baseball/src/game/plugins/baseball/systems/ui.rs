use baseball_game_rules::*;
use bevy::prelude::*;

use crate::game::plugins::baseball::{components::*, resources::*};

pub fn update_score_ui(game_data: Res<GameData>, mut query: Query<&mut Text, With<ScoreText>>) {
    if let Ok(mut text) = query.single_mut() {
        let (away, home) = match &game_data.game_result {
            GameOutcome::InProgress(game) => {
                let half_inning = game.current_half_inning();
                let pending_runs = half_inning.runs_scored();
                let (mut away, mut home) = (game.score().away(), game.score().home());

                // Add pending runs to the current batting team's score
                match half_inning.half() {
                    InningHalf::Top => away += pending_runs,
                    InningHalf::Bottom => home += pending_runs,
                }
                (away, home)
            }
            GameOutcome::Complete(summary) => (summary.final_score().away(), summary.final_score().home()),
        };
        **text = format!("Score: Away {away} - Home {home}");
    }
}

pub fn update_inning_ui(game_data: Res<GameData>, mut query: Query<&mut Text, With<InningText>>) {
    if let Ok(mut text) = query.single_mut() {
        let inning_text = match &game_data.game_result {
            GameOutcome::InProgress(game) => game.inning_description(),
            GameOutcome::Complete(_) => "Game Over".to_string(),
        };
        **text = inning_text;
    }
}

pub fn update_count_ui(game_data: Res<GameData>, mut query: Query<&mut Text, With<CountText>>) {
    if let Ok(mut text) = query.single_mut() {
        let count_text = match &game_data.game_result {
            GameOutcome::InProgress(game) => {
                let half_inning = game.current_half_inning();
                let count = half_inning.current_plate_appearance().count();
                let outs = half_inning.outs();
                format!("Count: {}-{} | Outs: {}", count.balls(), count.strikes(), outs)
            }
            GameOutcome::Complete(_) => "Final".to_string(),
        };
        **text = count_text;
    }
}

pub fn update_baserunners_ui(
    game_data: Res<GameData>,
    mut first: Query<
        &mut Visibility,
        (
            With<FirstBaseRunner>,
            Without<SecondBaseRunner>,
            Without<ThirdBaseRunner>,
        ),
    >,
    mut second: Query<
        &mut Visibility,
        (
            With<SecondBaseRunner>,
            Without<FirstBaseRunner>,
            Without<ThirdBaseRunner>,
        ),
    >,
    mut third: Query<
        &mut Visibility,
        (
            With<ThirdBaseRunner>,
            Without<FirstBaseRunner>,
            Without<SecondBaseRunner>,
        ),
    >,
) {
    let baserunners = match &game_data.game_result {
        GameOutcome::InProgress(game) => game.current_half_inning().baserunners(),
        GameOutcome::Complete(_) => BaserunnerState::empty(),
    };

    if let Ok(mut vis) = first.single_mut() {
        *vis = if baserunners.first().is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut vis) = second.single_mut() {
        *vis = if baserunners.second().is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut vis) = third.single_mut() {
        *vis = if baserunners.third().is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn update_result_display(
    time: Res<Time>,
    mut timer: ResMut<ResultDisplayTimer>,
    mut result_query: Query<&mut Visibility, With<ResultText>>,
) {
    if timer.timer.tick(time.delta()).just_finished()
        && let Ok(mut vis) = result_query.single_mut()
    {
        *vis = Visibility::Hidden;
    }
}
