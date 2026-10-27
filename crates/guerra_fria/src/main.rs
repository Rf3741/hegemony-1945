//! Cliente gráfico do jogo Guerra Fria (1945): Bevy + bevy_egui.
mod estado;
mod plugins;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_egui::EguiPlugin;
use plugins::{
    audio::AudioPlugin,
    economia_sim::EconomiaSimPlugin,
    economy::{EconomyPlugin, RecursoMundo},
    mercado_ui::MercadoUiPlugin,
    jogador::JogadorPlugin,
    map::MapPlugin,
    politics::PoliticsPlugin,
    time::TimePlugin,
    ui::UiPlugin,
};
use simulacao::mundo::MundoJogo;
use std::path::PathBuf;

/// Localiza a raiz do projeto (onde ficam `assets/` e `data/`).
fn raiz_projeto() -> PathBuf {
    let candidatos = [
        PathBuf::from("."),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
    ];
    for c in candidatos {
        if c.join("assets/map/provinces.png").exists() {
            return c;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn main() {
    // Caminho absoluto: o AssetServer resolve caminhos relativos a partir do crate quando roda via cargo.
    let raiz = raiz_projeto().canonicalize().unwrap_or_else(|_| raiz_projeto());
    let (_catalogo_legado, mundo) = MundoJogo::carregar_de_arquivos(
        raiz.join("assets/map/provinces.ron"),
        raiz.join("data/scenario_1945/countries_1945.ron"),
        raiz.join("data/scenario_1945/province_owners_1945.ron"),
    )
    .expect("falha ao carregar o cenário de 1945");
    // Catálogo do mapa vem de gf_mapa (o de `simulacao` sai na T23).
    let catalogo: gf_mapa::CatalogoProvincias = ron::from_str(
        &std::fs::read_to_string(raiz.join("assets/map/provinces.ron")).expect("provinces.ron"),
    )
    .expect("provinces.ron inválido");

    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin { file_path: raiz.join("assets").to_string_lossy().into_owned(), ..default() }).set(WindowPlugin {
            primary_window: Some(Window {
                title: "Guerra Fria - 1945".into(),
                resolution: WindowResolution::new(1600, 900),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .insert_resource(RecursoMundo(mundo))
        .insert_resource(plugins::map::RaizProjeto(raiz))
        .insert_resource(plugins::map::RecursoCatalogo(catalogo))
        .add_plugins((estado::EstadoPlugin, TimePlugin, EconomyPlugin, EconomiaSimPlugin, MercadoUiPlugin, PoliticsPlugin, JogadorPlugin, MapPlugin, UiPlugin, AudioPlugin))
        .run();
}
