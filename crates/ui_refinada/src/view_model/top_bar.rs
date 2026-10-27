//! View-model para o HUD e Barra Superior (Top Bar).

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VelocidadeJogo {
    Pausado,
    Velocidade1,
    Velocidade2,
    Velocidade3,
    Velocidade4,
    Velocidade5,
}

impl VelocidadeJogo {
    pub fn nivel(self) -> u8 {
        match self {
            Self::Pausado => 0,
            Self::Velocidade1 => 1,
            Self::Velocidade2 => 2,
            Self::Velocidade3 => 3,
            Self::Velocidade4 => 4,
            Self::Velocidade5 => 5,
        }
    }
}

/// Dados apresentados na barra superior fixa do HUD.
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct TopBarViewModel {
    pub tag_pais: String,
    pub nome_pais: String,
    pub classificacao_ranking: String,
    pub nome_lider: String,
    pub titulo_lider: String,

    // Métricas Financeiras
    pub tesouro_milhoes: f64,
    pub balanco_diario: f64,
    pub balanco_semanal_milhoes: f64,
    pub reservas_ouro_bilhoes: f64,
    pub inflacao_anual_pct: f32,

    // Macroeconomia e Demografia
    pub pib_bilhoes: f64,
    pub crescimento_pib_pct: f32,
    pub populacao_milhoes: f64,
    pub mao_de_obra_milhoes: f64,
    pub ic_alocada: u32,
    pub ic_total: u32,

    // Política e Estabilidade
    pub legitimidade_pct: f32,
    pub estabilidade_pct: f32,
    pub prestigio_pontos: u32,

    // Tensão Geopolítica (Doomsday Clock)
    pub nivel_defcon: u8,
    pub tensao_global_pct: f32,

    // Calendário e Controle de Tempo
    pub data_formatada: String,
    pub velocidade: VelocidadeJogo,
    pub esta_pausado: bool,

    // Notificações e Alertas
    pub alertas_pendentes: usize,
}

impl Default for TopBarViewModel {
    fn default() -> Self {
        Self {
            tag_pais: "USA".to_string(),
            nome_pais: "Estados Unidos da América".to_string(),
            classificacao_ranking: "#1 Superpotência".to_string(),
            nome_lider: "Franklin D. Roosevelt".to_string(),
            titulo_lider: "Presidente dos EUA".to_string(),
            tesouro_milhoes: 18450.0,
            balanco_diario: 42850.0,
            balanco_semanal_milhoes: 14.2,
            reservas_ouro_bilhoes: 20.2,
            inflacao_anual_pct: 1.8,
            pib_bilhoes: 228.0,
            crescimento_pib_pct: 4.2,
            populacao_milhoes: 139.9,
            mao_de_obra_milhoes: 48.5,
            ic_alocada: 184,
            ic_total: 210,
            legitimidade_pct: 86.0,
            estabilidade_pct: 82.0,
            prestigio_pontos: 1250,
            nivel_defcon: 4,
            tensao_global_pct: 65.0,
            data_formatada: "01 JAN 1945".to_string(),
            velocidade: VelocidadeJogo::Velocidade1,
            esta_pausado: false,
            alertas_pendentes: 3,
        }
    }
}
