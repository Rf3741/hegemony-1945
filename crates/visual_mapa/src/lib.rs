//! # `visual_mapa` — mapa vivo (cidades, indústrias, transporte, eras, LOD)
//!
//! Plugin Bevy reutilizável. O cliente adiciona [`VisualMapaPlugin`] e alimenta recursos como:
//! [`AnoVisual`], [`EntradaVisual`] e [`EntradaEconomiaVisual`] (alimentado pela economia real).
//!
//! - Projeção: Miller do `gf_mapa` ([`projecao`]); posições de entrada são (lon, lat) em graus.
//!   O mapa é um sprite centrado na origem, então cidades caem sobre o pixel certo de `provinces.png`.
//! - Arte: atlas de `arte_cidades` (`assets/visual/atlas.*`), com reserva procedural sem atlas.
//! - O plugin NÃO cria câmera nem janela. O LOD usa a escala da `Camera2d` existente.
//! - Feature `demo`: [`demo::DemoPlugin`] (dados falsos, tempo acelerado, câmera, HUD egui).

pub mod atlas;
pub mod cenario;
pub mod cidade;
pub mod config;
pub mod dados;
pub mod entrada;
pub mod era;
pub mod industria;
pub mod lod;
pub mod mapa;
pub mod mapeamento;
pub mod predio;
pub mod projecao;
pub mod transporte;

#[cfg(feature = "demo")]
pub mod demo;

use bevy::prelude::*;

pub use atlas::AtlasVisual;
pub use cenario::carregar_cenario_1945;
pub use cidade::{CidadeVisual, IndiceEntidades, RotuloNomeCidade};
pub use config::{CamadaConstrucoesVisivel, VisualMapaConfig};
pub use entrada::{
    AnoVisual, BlocoVisual, CidadeDemograficaEntrada, CidadeEntrada, ConexaoEntrada,
    EntradaEconomiaVisual, EntradaVisual, FonteEconomiaVisual, InstalacaoEntrada,
    PredioEconomiaEntrada, populacao_urbana_para_nivel, populacao_urbana_para_nivel_fracionario,
};
pub use era::GerenciadorEras;
pub use lod::{EstadoLod, LimitesZoomVisual, NivelLod};
pub use mapa::{EstadoModoMapa, ModoMapa};
pub use mapeamento::{
    CatalogoMapeamentoPredios, MapeamentoPredioRegistro, carregar_mapeamento_predios,
};
pub use predio::{
    AgregadoEstadoVisual, IndiceEntidadesEconomia, PredioVisual,
    alternar_camada_construcoes_sistema, calcular_offset_espiral, tamanho_sprite_predio,
};
pub use projecao::{ProjecaoMapa, geo_para_mundo};

/// Conjuntos de sistemas do plugin (para ordenar contra os do cliente).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisualMapaSet {
    /// Cria/atualiza entidades a partir de [`EntradaVisual`] e [`EntradaEconomiaVisual`].
    Sincronizar,
    /// Animações, LOD e eras.
    Animar,
}

/// Plugin do mapa vivo.
#[derive(Default)]
pub struct VisualMapaPlugin;

impl Plugin for VisualMapaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VisualMapaConfig>()
            .init_resource::<CamadaConstrucoesVisivel>()
            .init_resource::<LimitesZoomVisual>()
            .init_resource::<AnoVisual>()
            .init_resource::<EntradaVisual>()
            .init_resource::<EntradaEconomiaVisual>()
            .init_resource::<GerenciadorEras>()
            .init_resource::<EstadoModoMapa>()
            .init_resource::<EstadoLod>()
            .init_resource::<IndiceEntidades>()
            .init_resource::<IndiceEntidadesEconomia>()
            .init_resource::<AtlasVisual>()
            .init_resource::<CatalogoMapeamentoPredios>()
            .add_systems(
                Startup,
                (
                    carregar_arte_sistema,
                    industria::criar_recursos_fumaca,
                    predio::setup_recursos_predios,
                ),
            )
            .add_systems(
                Update,
                (
                    cidade::sincronizar_cidades_sistema,
                    industria::sincronizar_industrias_sistema,
                    transporte::sincronizar_transporte_sistema,
                )
                    .chain()
                    .run_if(resource_changed::<EntradaVisual>)
                    .in_set(VisualMapaSet::Sincronizar),
            )
            .add_systems(
                Update,
                predio::sincronizar_economia_sistema
                    .run_if(resource_changed::<EntradaEconomiaVisual>)
                    .in_set(VisualMapaSet::Sincronizar),
            )
            .add_systems(
                Update,
                (
                    era::sincronizar_eras_sistema,
                    mapa::atualizar_transicao_mapa_sistema,
                    cidade::atualizar_cidades_sistema,
                    cidade::atualizar_sprites_cidade_sistema,
                    cidade::animar_predios_cidade_sistema,
                    industria::atualizar_industrias_sistema,
                    industria::emitir_fumaca_sistema,
                    industria::atualizar_particulas_fumaca_sistema,
                    predio::animar_predios_sistema,
                    predio::emitir_fumaca_predios_sistema,
                    predio::alternar_camada_construcoes_sistema,
                    lod::atualizar_lod_sistema,
                )
                    .in_set(VisualMapaSet::Animar)
                    .after(VisualMapaSet::Sincronizar),
            );
    }
}

/// Carrega eras, catálogo de mapeamento e atlas da `raiz_dados` configurada.
fn carregar_arte_sistema(
    cfg: Res<VisualMapaConfig>,
    mut eras: ResMut<GerenciadorEras>,
    mut atlas: ResMut<AtlasVisual>,
    mut catalogo: ResMut<CatalogoMapeamentoPredios>,
    mut imagens: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    eras.eras = dados::carregar_eras(&cfg.raiz_dados);
    *catalogo = mapeamento::carregar_mapeamento_predios(&cfg.raiz_dados);
    if cfg.usar_atlas {
        atlas.carregado = atlas::carregar_atlas(&cfg.raiz_dados, &mut imagens, &mut layouts);
        if atlas.carregado.is_none() {
            warn!(
                "visual_mapa: atlas indisponível em {}; usando visual procedural.",
                cfg.raiz_dados.display()
            );
        }
    }
}
