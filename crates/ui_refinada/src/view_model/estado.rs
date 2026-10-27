//! View-model para o Painel do Estado / Província (F2).
//! Inclui dados demográficos para gráficos de pizza (profissão, cultura, religião),
//! prédios industriais, infraestrutura e recursos naturais.

use bevy::prelude::Resource;
use bevy_egui::egui::Color32;
use serde::{Deserialize, Serialize};

/// Fatia de dados para exibição em gráfico de pizza (Pops por profissão, cultura ou religião).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopFatiaViewModel {
    pub rotulo: String,
    pub quantidade: u64,
    pub percentual: f32,
    pub cor: (u8, u8, u8),
    pub detalhe_estrato_ou_origem: String,
}

impl PopFatiaViewModel {
    pub fn cor32(&self) -> Color32 {
        Color32::from_rgb(self.cor.0, self.cor.1, self.cor.2)
    }
}

/// Prédio produtivo presente no estado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredioEstadoViewModel {
    pub id: String,
    pub nome: String,
    pub nivel: u32,
    pub nivel_maximo: u32,
    pub trabalhadores_empregados: u64,
    pub trabalhadores_capacidade: u64,
    pub lucro_semanal_mil: f64,
    pub metodo_producao: String,
    pub consumo_insumos: Vec<(String, f32)>,
    pub producao_saidas: Vec<(String, f32)>,
}

/// Jazida ou capacidade de recurso natural na província.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursoLocalViewModel {
    pub nome: String,
    pub explorado_niveis: u32,
    pub limite_maximo: u32,
    pub tipo_bem_associado: String,
}

/// View-model completo do Estado / Província.
#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct EstadoViewModel {
    pub adm1_code: String,
    pub nome: String,
    pub capital_regional: String,
    pub pais_tag: String,
    pub populacao_total: u64,
    pub mao_de_obra_total: u64,
    pub padrao_vida_sol: f32,
    pub agitacao_social_pct: f32,
    pub destruicao_guerra_pct: f32,

    // Infraestrutura e Acesso ao Mercado
    pub infraestrutura_fornecida: u32,
    pub infraestrutura_usada: u32,
    pub acesso_ao_mercado_pct: f32,

    // Pops Estratificadas (Alimentam os Gráficos de Pizza)
    pub pops_profissoes: Vec<PopFatiaViewModel>,
    pub pops_culturas: Vec<PopFatiaViewModel>,
    pub pops_religioes: Vec<PopFatiaViewModel>,

    // Prédios e Instalações
    pub predios: Vec<PredioEstadoViewModel>,

    // Recursos Naturais
    pub recursos: Vec<RecursoLocalViewModel>,
}

impl Default for EstadoViewModel {
    fn default() -> Self {
        Self {
            adm1_code: "USA-3522".to_string(),
            nome: "Pensilvânia".to_string(),
            capital_regional: "Pittsburgh / Harrisburg".to_string(),
            pais_tag: "USA".to_string(),
            populacao_total: 9_900_000,
            mao_de_obra_total: 3_400_000,
            padrao_vida_sol: 61.8, // Alto
            agitacao_social_pct: 4.2,
            destruicao_guerra_pct: 0.0,
            infraestrutura_fornecida: 140,
            infraestrutura_usada: 108,
            acesso_ao_mercado_pct: 100.0,
            pops_profissoes: vec![
                PopFatiaViewModel {
                    rotulo: "Operários Industriais".to_string(),
                    quantidade: 1_850_000,
                    percentual: 54.4,
                    cor: (45, 95, 175),
                    detalhe_estrato_ou_origem: "Estrato Baixo • Salário médio: US$ 38.50/sem".to_string(),
                },
                PopFatiaViewModel {
                    rotulo: "Produtores Rurais".to_string(),
                    quantidade: 680_000,
                    percentual: 20.0,
                    cor: (108, 127, 95),
                    detalhe_estrato_ou_origem: "Estrato Baixo • Salário médio: US$ 24.00/sem".to_string(),
                },
                PopFatiaViewModel {
                    rotulo: "Engenheiros e Técnicos".to_string(),
                    quantidade: 420_000,
                    percentual: 12.4,
                    cor: (200, 134, 39),
                    detalhe_estrato_ou_origem: "Estrato Médio • Salário médio: US$ 72.00/sem".to_string(),
                },
                PopFatiaViewModel {
                    rotulo: "Burocratas e Serviços".to_string(),
                    quantidade: 405_000,
                    percentual: 11.9,
                    cor: (56, 115, 158),
                    detalhe_estrato_ou_origem: "Estrato Médio • Salário médio: US$ 58.00/sem".to_string(),
                },
                PopFatiaViewModel {
                    rotulo: "Capitalistas e Sócios".to_string(),
                    quantidade: 45_000,
                    percentual: 1.3,
                    cor: (223, 177, 91),
                    detalhe_estrato_ou_origem: "Estrato Alto • Dividendos médios: US$ 480.00/sem".to_string(),
                },
            ],
            pops_culturas: vec![
                PopFatiaViewModel {
                    rotulo: "Anglo-Americano".to_string(),
                    quantidade: 6_435_000,
                    percentual: 65.0,
                    cor: (41, 92, 143),
                    detalhe_estrato_ou_origem: "Cultura Primária Aceita".to_string(),
                },
                PopFatiaViewModel {
                    rotulo: "Germano-Americano".to_string(),
                    quantidade: 1_782_000,
                    percentual: 18.0,
                    cor: (138, 123, 88),
                    detalhe_estrato_ou_origem: "Cultura Integrada".to_string(),
                },
                PopFatiaViewModel {
                    rotulo: "Ítalo-Americano".to_string(),
                    quantidade: 990_000,
                    percentual: 10.0,
                    cor: (170, 75, 60),
                    detalhe_estrato_ou_origem: "Comunidade Urbana Fabril".to_string(),
                },
                PopFatiaViewModel {
                    rotulo: "Afro-Americano".to_string(),
                    quantidade: 693_000,
                    percentual: 7.0,
                    cor: (104, 117, 130),
                    detalhe_estrato_ou_origem: "Grande Migração do Sul".to_string(),
                },
            ],
            pops_religioes: vec![
                PopFatiaViewModel {
                    rotulo: "Cristianismo Protestante".to_string(),
                    quantidade: 5_940_000,
                    percentual: 60.0,
                    cor: (56, 115, 158),
                    detalhe_estrato_ou_origem: "Presbiterianos, Metodistas, Batistas".to_string(),
                },
                PopFatiaViewModel {
                    rotulo: "Catolicismo Romano".to_string(),
                    quantidade: 3_465_000,
                    percentual: 35.0,
                    cor: (200, 134, 39),
                    detalhe_estrato_ou_origem: "Dioceses de Filadélfia e Pittsburgh".to_string(),
                },
                PopFatiaViewModel {
                    rotulo: "Judaísmo e Outros".to_string(),
                    quantidade: 495_000,
                    percentual: 5.0,
                    cor: (217, 167, 56),
                    detalhe_estrato_ou_origem: "Comunidades Urbanas Comerciais".to_string(),
                },
            ],
            predios: vec![
                PredioEstadoViewModel {
                    id: "siderurgica_pittsburgh".to_string(),
                    nome: "Siderúrgicas Integradas de Pittsburgh".to_string(),
                    nivel: 8,
                    nivel_maximo: 20,
                    trabalhadores_empregados: 38_400,
                    trabalhadores_capacidade: 40_000,
                    lucro_semanal_mil: 248.0,
                    metodo_producao: "Siderurgia de Alto-Forno com Coque".to_string(),
                    consumo_insumos: vec![
                        ("Minério de Ferro".to_string(), 240.0),
                        ("Carvão Mineral".to_string(), 200.0),
                        ("Eletricidade".to_string(), 80.0),
                    ],
                    producao_saidas: vec![
                        ("Aço Estrutural".to_string(), 400.0),
                        ("Alumínio e Ligas".to_string(), 144.0),
                    ],
                },
                PredioEstadoViewModel {
                    id: "mina_carvao_apalaches".to_string(),
                    nome: "Minas de Carvão Betuminoso dos Apalaches".to_string(),
                    nivel: 6,
                    nivel_maximo: 15,
                    trabalhadores_empregados: 29_100,
                    trabalhadores_capacidade: 30_000,
                    lucro_semanal_mil: 112.5,
                    metodo_producao: "Mineração Mecanizada com Ventilação".to_string(),
                    consumo_insumos: vec![
                        ("Ferramentas e Máquinas".to_string(), 60.0),
                    ],
                    producao_saidas: vec![
                        ("Carvão Mineral".to_string(), 300.0),
                    ],
                },
                PredioEstadoViewModel {
                    id: "ferrovia_pennsylvania".to_string(),
                    nome: "Pennsylvania Railroad & Terminais".to_string(),
                    nivel: 5,
                    nivel_maximo: 10,
                    trabalhadores_empregados: 24_500,
                    trabalhadores_capacidade: 25_000,
                    lucro_semanal_mil: 84.0,
                    metodo_producao: "Locomotivas a Vapor e Carvão".to_string(),
                    consumo_insumos: vec![
                        ("Aço".to_string(), 50.0),
                        ("Carvão".to_string(), 60.0),
                    ],
                    producao_saidas: vec![
                        ("Capacidade de Transporte".to_string(), 300.0),
                    ],
                },
            ],
            recursos: vec![
                RecursoLocalViewModel {
                    nome: "Bacia de Carvão Betuminoso".to_string(),
                    explorado_niveis: 6,
                    limite_maximo: 15,
                    tipo_bem_associado: "Carvão Mineral".to_string(),
                },
                RecursoLocalViewModel {
                    nome: "Depósitos de Minério de Ferro".to_string(),
                    explorado_niveis: 3,
                    limite_maximo: 8,
                    tipo_bem_associado: "Minério de Ferro".to_string(),
                },
                RecursoLocalViewModel {
                    nome: "Terras Agrícolas do Vale de Susquehanna".to_string(),
                    explorado_niveis: 12,
                    limite_maximo: 25,
                    tipo_bem_associado: "Grãos e Pecuária".to_string(),
                },
                RecursoLocalViewModel {
                    nome: "Hidrovias Industriais (Rios Ohio e Monongahela)".to_string(),
                    explorado_niveis: 5,
                    limite_maximo: 10,
                    tipo_bem_associado: "Logística Fluvial".to_string(),
                },
            ],
        }
    }
}
