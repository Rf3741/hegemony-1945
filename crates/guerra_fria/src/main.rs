//! Cliente gráfico do jogo Guerra Fria (1945): Bevy + bevy_egui.
mod estado;
mod plugins;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use plugins::{
    atmosfera::AtmosferaPlugin,
    audio::AudioPlugin,
    comercio_ui::ComercioUiPlugin,
    construcao_ui::ConstrucaoUiPlugin,
    diplomacia_ui::DiplomaciaUiPlugin,
    economia_sim::EconomiaSimPlugin,
    economy::{EconomyPlugin, RecursoMundo},
    enciclopedia::EnciclopediaPlugin,
    estatisticas::EstatisticasPlugin,
    eventos_ui::EventosPlugin,
    guerra_ia::GuerraIaPlugin,
    guerra_sim::GuerraSimPlugin,
    guerra_ui::GuerraUiPlugin,
    jogador::JogadorPlugin,
    map::MapPlugin,
    mercado_ui::MercadoUiPlugin,
    personagens_sim::PersonagensPlugin,
    politics::PoliticsPlugin,
    rotas_ponte::RotasPontePlugin,
    rotulos_pais::RotulosPaisPlugin,
    tecnologia_ui::TecnologiaUiPlugin,
    time::TimePlugin,
    ui::UiPlugin,
    visual_construcoes::VisualConstrucoesPlugin,
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

fn main() -> AppExit {
    let opcoes = plugins::smoke::analisar_args(std::env::args()).unwrap_or_else(|erro| {
        eprintln!("argumentos inválidos: {erro}");
        std::process::exit(2);
    });
    // Caminho absoluto: o AssetServer resolve caminhos relativos a partir do crate quando roda via cargo.
    let raiz = raiz_projeto()
        .canonicalize()
        .unwrap_or_else(|_| raiz_projeto());
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

    let mut config = plugins::config::ConfigJogo::carregar();
    if let Some(qualidade) = opcoes.qualidade {
        config.graficos.qualidade = qualidade;
    }

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: raiz.join("assets").to_string_lossy().into_owned(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(config.janela()),
                ..default()
            }),
    )
    .add_plugins(EguiPlugin::default())
    .insert_resource(RecursoMundo(mundo))
    .insert_resource(plugins::map::RaizProjeto(raiz.clone()))
    .insert_resource(plugins::map::RecursoCatalogo(catalogo))
    .insert_resource(visual_mapa::VisualMapaConfig {
        raiz_dados: raiz.join("crates/visual_mapa/assets/visual"),
        ..default()
    })
    .add_plugins((
        estado::EstadoPlugin,
        TimePlugin,
        EconomyPlugin,
        EconomiaSimPlugin,
        MercadoUiPlugin,
        ComercioUiPlugin,
        ConstrucaoUiPlugin,
        PoliticsPlugin,
        JogadorPlugin,
        MapPlugin,
        UiPlugin,
        AudioPlugin,
        RotasPontePlugin,
        VisualConstrucoesPlugin,
        RotulosPaisPlugin,
    ))
    // Vídeo de abertura (pôster de propaganda) na tela de carregamento.
    .add_plugins(plugins::abertura::AberturaPlugin)
    .add_plugins(AtmosferaPlugin)
    .add_plugins(EventosPlugin)
    .add_plugins(plugins::formacao_sim::FormacaoSimPlugin)
    .add_plugins(plugins::bandeiras_ui::BandeirasPlugin)
    .add_plugins(DiplomaciaUiPlugin)
    .add_plugins(TecnologiaUiPlugin)
    .add_plugins(EstatisticasPlugin)
    .add_plugins(EnciclopediaPlugin)
    .add_plugins(PersonagensPlugin)
    .add_plugins((GuerraSimPlugin, GuerraUiPlugin, GuerraIaPlugin))
    .add_plugins(plugins::config::ConfigPlugin)
    // Sobrepõe o arquivo lido pelo ConfigPlugin (qualidade do smoke não é gravada).
    .insert_resource(config)
    .add_plugins(plugins::menu_pausa::MenuPausaPlugin)
    .add_plugins(plugins::menu_ui::MenuUiPlugin)
    .add_plugins(plugins::saveload::SaveLoadPlugin)
    .add_plugins(plugins::menu_saves::MenuSavesPlugin)
    .add_plugins(plugins::hud_mapas::HudMapasPlugin)
    .add_plugins(plugins::tutorial_guiado::TutorialGuiadoPlugin)
    .add_plugins(plugins::polimento_visual::PolimentoVisualPlugin);

    if opcoes.smoke_ativo() {
        app.add_plugins(plugins::smoke::SmokePlugin(opcoes));
    }

    app.run()
}
