//! Demonstração do plugin com dados falsos e tempo acelerado (fundo: graticula de teste).
//! `cargo run -p visual_mapa --example demo --features demo --release`

use bevy::prelude::*;
use visual_mapa::VisualMapaPlugin;
use visual_mapa::demo::DemoPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Guerra Fria - Visual do Mapa Vivo (demo)".into(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.075, 0.11, 0.13)))
        .add_plugins((VisualMapaPlugin, DemoPlugin::default()))
        .run();
}
