//! View-model para o Ranking das Grandes Potências Mundiais (F8).

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntradaRankingViewModel {
    pub posicao: u32,
    pub tag: String,
    pub nome: String,
    pub categoria_potencia: String,
    pub pontos_prestigio: u32,
    pub pib_pontos: u32,
    pub forca_militar_pontos: u32,
    pub esfera_diplomatica_pontos: u32,
    pub tecnologia_pontos: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct RankingViewModel {
    pub potencias: Vec<EntradaRankingViewModel>,
}

impl Default for RankingViewModel {
    fn default() -> Self {
        Self {
            potencias: vec![
                EntradaRankingViewModel {
                    posicao: 1,
                    tag: "USA".to_string(),
                    nome: "Estados Unidos da América".to_string(),
                    categoria_potencia: "Superpotência Global".to_string(),
                    pontos_prestigio: 1250,
                    pib_pontos: 550,
                    forca_militar_pontos: 380,
                    esfera_diplomatica_pontos: 220,
                    tecnologia_pontos: 100,
                },
                EntradaRankingViewModel {
                    posicao: 2,
                    tag: "SOV".to_string(),
                    nome: "União das Repúblicas Socialistas Soviéticas".to_string(),
                    categoria_potencia: "Superpotência Global".to_string(),
                    pontos_prestigio: 1080,
                    pib_pontos: 380,
                    forca_militar_pontos: 460,
                    esfera_diplomatica_pontos: 180,
                    tecnologia_pontos: 60,
                },
                EntradaRankingViewModel {
                    posicao: 3,
                    tag: "GBR".to_string(),
                    nome: "Reino Unido & Império Britânico".to_string(),
                    categoria_potencia: "Grande Potência Imperial".to_string(),
                    pontos_prestigio: 890,
                    pib_pontos: 340,
                    forca_militar_pontos: 280,
                    esfera_diplomatica_pontos: 210,
                    tecnologia_pontos: 60,
                },
                EntradaRankingViewModel {
                    posicao: 4,
                    tag: "FRA".to_string(),
                    nome: "República Francesa (Gov. Provisório)".to_string(),
                    categoria_potencia: "Grande Potência em Reconstrução".to_string(),
                    pontos_prestigio: 460,
                    pib_pontos: 180,
                    forca_militar_pontos: 150,
                    esfera_diplomatica_pontos: 90,
                    tecnologia_pontos: 40,
                },
                EntradaRankingViewModel {
                    posicao: 5,
                    tag: "CHI".to_string(),
                    nome: "República da China (Nacionalista)".to_string(),
                    categoria_potencia: "Grande Potência em Guerra Civil".to_string(),
                    pontos_prestigio: 380,
                    pib_pontos: 120,
                    forca_militar_pontos: 180,
                    esfera_diplomatica_pontos: 60,
                    tecnologia_pontos: 20,
                },
                EntradaRankingViewModel {
                    posicao: 6,
                    tag: "CAN".to_string(),
                    nome: "Domínio do Canadá".to_string(),
                    categoria_potencia: "Potência Média Industrial".to_string(),
                    pontos_prestigio: 280,
                    pib_pontos: 120,
                    forca_militar_pontos: 90,
                    esfera_diplomatica_pontos: 45,
                    tecnologia_pontos: 25,
                },
                EntradaRankingViewModel {
                    posicao: 7,
                    tag: "AUS".to_string(),
                    nome: "Comunidade da Austrália".to_string(),
                    categoria_potencia: "Potência Regional do Pacífico".to_string(),
                    pontos_prestigio: 210,
                    pib_pontos: 90,
                    forca_militar_pontos: 75,
                    esfera_diplomatica_pontos: 30,
                    tecnologia_pontos: 15,
                },
                EntradaRankingViewModel {
                    posicao: 8,
                    tag: "BRA".to_string(),
                    nome: "Estados Unidos do Brasil".to_string(),
                    categoria_potencia: "Potência Regional (FEB na Itália)".to_string(),
                    pontos_prestigio: 190,
                    pib_pontos: 85,
                    forca_militar_pontos: 65,
                    esfera_diplomatica_pontos: 30,
                    tecnologia_pontos: 10,
                },
            ],
        }
    }
}
