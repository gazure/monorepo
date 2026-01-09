use bevy::prelude::*;

use crate::GameState;

#[derive(Component)]
pub struct MenuUI;

pub fn setup_menu(mut commands: Commands) {
    info!("Setting up menu");
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
            MenuUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Conway's Game of Life"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(Justify::Center),
            ));

            parent.spawn((
                Text::new("Press SPACE to Start"),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                TextLayout::new_with_justify(Justify::Center),
            ));

            parent.spawn((
                Text::new("Press ESC to return to menu during gameplay"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                TextLayout::new_with_justify(Justify::Center),
            ));

            parent.spawn((
                Text::new("Press 1-3 to change color patterns during gameplay"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                TextLayout::new_with_justify(Justify::Center),
            ));
        });
}

pub fn menu_action(keyboard: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::Playing);
        info!("Starting game");
    }
}

pub fn cleanup_menu(mut commands: Commands, menu_query: Query<Entity, With<MenuUI>>) {
    info!("Cleaning up menu");
    for entity in &menu_query {
        commands.entity(entity).despawn();
    }
    // Camera persists and is reused by the game
}
