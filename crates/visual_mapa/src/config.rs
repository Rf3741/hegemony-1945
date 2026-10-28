//! Configuração do `VisualMapaPlugin`.

use crate::dados::raiz_dados_padrao;
use crate::projecao::ProjecaoMapa;
use bevy::prelude::*;
use std::path::PathBuf;

/// Configuração (insira ANTES de adicionar o plugin para sobrescrever o padrão).
#[derive(Resource, Debug, Clone)]
pub struct VisualMapaConfig {
    /// Pasta com `atlas.png`, `atlas.ron` e `eras.ron`. Padrão: `$GF_DATA_DIR/visual` ou os
    /// assets do crate.
    pub raiz_dados: PathBuf,
    /// Projeção usada para posicionar tudo (deve bater com o sprite do mapa do cliente).
    pub projecao: ProjecaoMapa,
    /// Fator de tamanho de sprites e marcadores (1.0 = padrão para o mapa de 5632 px).
    pub escala_sprites: f32,
    /// Máximo de instalações desenhadas (desempenho).
    pub limite_instalacoes: usize,
    /// Máximo de prédios individuais detalhados por estado antes de agregar no ícone estadual (desempenho).
    pub limite_predios_por_estado: usize,
    /// Espaçamento entre prédios na espiral concêntrica (unidades de mundo).
    pub espacamento_predios: f32,
    /// Tenta carregar o atlas; `false` força o visual procedural.
    pub usar_atlas: bool,
}

impl Default for VisualMapaConfig {
    fn default() -> Self {
        Self {
            raiz_dados: raiz_dados_padrao(),
            projecao: ProjecaoMapa::default(),
            escala_sprites: 1.0,
            limite_instalacoes: 5_000,
            limite_predios_por_estado: 32,
            espacamento_predios: 15.0,
            usar_atlas: true,
        }
    }
}

/// Controla se a camada visual das construções e marcadores no mapa está visível (tecla V).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CamadaConstrucoesVisivel(pub bool);

impl Default for CamadaConstrucoesVisivel {
    fn default() -> Self {
        Self(true)
    }
}
