//! O plugin sobre o mapa real do `guerra_fria` (`assets/map/provinces.png`, Miller 5632x2048).
//! O mapa é um sprite centrado na origem, igual ao cliente; as cidades usam a mesma projeção.
//! `cargo run -p visual_mapa --example mapa_real --features demo --release`
//! Controles: WASD/setas, Q/E ou roda (zoom), espaço pausa, N noite, "Acelerar demo" no HUD.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use visual_mapa::VisualMapaPlugin;
use visual_mapa::demo::DemoPlugin;
use visual_mapa::demo::camera::CameraMapa;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Guerra Fria - visual_mapa sobre o mapa real".into(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.11, 0.23, 0.38)))
        .add_plugins((VisualMapaPlugin, DemoPlugin::default().com_fundo(false).com_camera(false)))
        .add_systems(Startup, (criar_camera, criar_mapa))
        .add_systems(Update, visual_mapa::demo::camera::controlar_camera_sistema)
        .run();
}

fn criar_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection { scale: 4.0, ..OrthographicProjection::default_2d() }),
        CameraMapa { velocidade_pan: 450.0, velocidade_zoom: 1.5, limite: Vec2::new(2900.0, 1100.0) },
    ));
}

/// Sprite do mapa: oceano (preto no `provinces.png`) azul; terra em tons de cinza por província.
fn criar_mapa(mut commands: Commands, mut imagens: ResMut<Assets<Image>>) {
    let caminho = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/map/provinces.png");
    let Ok(img) = image::open(caminho) else {
        error!("provinces.png nao encontrado em {caminho}");
        return;
    };
    let img = img.to_rgba8();
    let (w, h) = img.dimensions();
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for p in img.pixels() {
        let [r, g, b, _] = p.0;
        if r == 0 && g == 0 && b == 0 {
            px.extend_from_slice(&[28, 58, 96, 255]);
        } else {
            let t = 90 + ((r as u32 * 31 + g as u32 * 17 + b as u32 * 7) % 60) as u8;
            px.extend_from_slice(&[t, t.saturating_add(8), t.saturating_sub(10), 255]);
        }
    }
    let imagem = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        px,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    commands.spawn((Sprite::from_image(imagens.add(imagem)), Transform::from_xyz(0.0, 0.0, 0.0)));
}
