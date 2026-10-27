//! Módulo de View-Models desacoplados dos dados lógicos da simulação.

pub mod construcao;
pub mod diplomacia;
pub mod estado;
pub mod eventos;
pub mod mercado;
pub mod notificacoes;
pub mod orcamento;
pub mod pais;
pub mod politica;
pub mod ranking;
pub mod top_bar;

pub use construcao::*;
pub use diplomacia::*;
pub use estado::*;
pub use eventos::*;
pub use mercado::*;
pub use notificacoes::*;
pub use orcamento::*;
pub use pais::*;
pub use politica::*;
pub use ranking::*;
pub use top_bar::*;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

/// Enumeração das telas principais ativas na interface (conforme TELAS.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TelaAtiva {
    #[default]
    MapaGeralHud,     // ESC: Visão do Mapa com HUD Superior e Notificações
    PainelPais,       // F1: Dossiê Nacional do País
    PainelEstado,     // F2: Gestão do Estado/Província e Pops
    Mercado,          // F3: Cotações e Gráfico de Preços (egui_plot)
    Construcao,       // F4: Fila de Construção e Catálogo
    Orcamento,        // F5: Balanço Fiscal e Sliders Tributários
    Politica,         // F6: Leis Ativas, Tramitação e Grupos de Interesse
    Diplomacia,       // F7: Relações Bipolares EUA x URSS
    RankingPotencias, // F8: Ranking das Grandes Potências Mundiais
}

/// Estado global de navegação e seleção da interface.
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct EstadoUiGlobal {
    pub tela_ativa: TelaAtiva,
    pub estado_selecionado_adm1: String,
    pub bem_selecionado_id: String,
    pub mostrar_evento_modal: bool,
}

impl Default for EstadoUiGlobal {
    fn default() -> Self {
        Self {
            tela_ativa: TelaAtiva::MapaGeralHud,
            estado_selecionado_adm1: "USA-3522".to_string(), // Pensilvânia
            bem_selecionado_id: "aco".to_string(),
            mostrar_evento_modal: false,
        }
    }
}
