use bevy::{log::LogPlugin, prelude::*, window::WindowResolution};

use crate::game::plugins::baseball::BaseballPlugin;

pub fn run() {
    tracingx::init_dev();

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Baseball".into(),
                    // Windowed rather than borderless fullscreen. Both cameras use
                    // `ScalingMode::AutoMin`, so the layout holds at any size, and a
                    // window is far easier to develop against.
                    resolution: WindowResolution::new(1280, 800),
                    ..default()
                }),
                ..default()
            })
            .disable::<LogPlugin>(),
    )
    .add_plugins(BaseballPlugin);

    // The world inspector draws straight over the score bug, so it is opt-in:
    //   cargo run -p baseball --features debug-inspector
    #[cfg(feature = "debug-inspector")]
    {
        use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
        app.add_plugins(EguiPlugin::default())
            .add_plugins(WorldInspectorPlugin::new());
    }

    app.run();
}
