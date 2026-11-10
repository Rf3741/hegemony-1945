//! Configuração do `VisualMapaPlugin`.

use crate::dados::raiz_dados_padrao;
use crate::projecao::ProjecaoMapa;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Nível de detalhe dos objetos do mapa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QualidadeObjetos {
    Baixo,
    #[default]
    Medio,
    Alto,
    Ultra,
}

impl QualidadeObjetos {
    pub fn tem_faces(self) -> bool {
        !matches!(self, Self::Baixo)
    }
    pub fn tem_sombras(self) -> bool {
        matches!(self, Self::Alto | Self::Ultra)
    }
    pub fn tem_animacao(self) -> bool {
        matches!(self, Self::Ultra)
    }
}

/// Parâmetros visuais compartilhados pelos objetos pseudo-3D.
#[derive(Resource, Debug, Clone)]
pub struct ConfiguracaoObjetos {
    pub qualidade: QualidadeObjetos,
    /// Extrusão aparente em unidades de mundo; não altera o limite das cidades.
    pub extrusao: f32,
    pub intensidade_sombra: f32,
    pub sombras_ativas: bool,
    pub fumaca_ativa: bool,
    /// Gancho para o ciclo dia/noite, no intervalo 0..=1.
    pub intensidade_noite: f32,
}

impl Default for ConfiguracaoObjetos {
    fn default() -> Self {
        Self {
            qualidade: QualidadeObjetos::Medio,
            extrusao: 0.22,
            intensidade_sombra: 0.28,
            sombras_ativas: true,
            fumaca_ativa: true,
            intensidade_noite: 0.0,
        }
    }
}

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

/// Controla se os pontos e rótulos das cidades estão visíveis.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CidadesVisiveis(pub bool);

impl Default for CidadesVisiveis {
    fn default() -> Self {
        Self(true)
    }
}
