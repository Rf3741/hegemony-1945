//! Executável binário autônomo para demonstração da UI Refinada de Guerra Fria 1945.

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use ui_refinada::UiPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Guerra Fria: 1945 — Gabinete de Guerra & UI Refinada".to_string(),
                resolution: (1600_u32, 900_u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(UiPlugin)
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
