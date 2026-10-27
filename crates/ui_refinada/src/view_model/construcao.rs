//! View-model para a Tela de Construção e Obras (F4).
//! Fila nacional prioritária, controles de prioridade e catálogo de edifícios.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemFilaConstrucaoViewModel {
    pub id: String,
    pub nome_projeto: String,
    pub nome_estado: String,
    pub nivel_atual: u32,
    pub nivel_alvo: u32,
    pub progresso_pct: f32,
    pub ic_alocada: u32,
    pub semanas_restantes: u32,
    pub pausado: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogoPredioViewModel {
    pub id: String,
    pub nome: String,
    pub categoria: String,
    pub custo_pontos_ic: u32,
    pub insumos_necessarios: Vec<(String, u32)>,
    pub empregos_por_nivel: u64,
    pub descricao: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct ConstrucaoViewModel {
    pub ic_total_disponivel: u32,
    pub ic_em_uso: u32,
    pub eficiencia_construcao_pct: f32,
    pub fila: Vec<ItemFilaConstrucaoViewModel>,
    pub catalogo: Vec<CatalogoPredioViewModel>,
}

impl Default for ConstrucaoViewModel {
    fn default() -> Self {
        Self {
            ic_total_disponivel: 72,
            ic_em_uso: 72,
            eficiencia_construcao_pct: 100.0,
            fila: vec![
                ItemFilaConstrucaoViewModel {
                    id: "proj_01".to_string(),
                    nome_projeto: "Siderúrgica de Alto-Forno de Pittsburgh".to_string(),
                    nome_estado: "Pensilvânia".to_string(),
                    nivel_atual: 8,
                    nivel_alvo: 9,
                    progresso_pct: 75.0,
                    ic_alocada: 25,
                    semanas_restantes: 6,
                    pausado: false,
                },
                ItemFilaConstrucaoViewModel {
                    id: "proj_02".to_string(),
                    nome_projeto: "Refinaria de Combustível de Aviação".to_string(),
                    nome_estado: "Texas".to_string(),
                    nivel_atual: 4,
                    nivel_alvo: 5,
                    progresso_pct: 45.0,
                    ic_alocada: 20,
                    semanas_restantes: 12,
                    pausado: false,
                },
                ItemFilaConstrucaoViewModel {
                    id: "proj_03".to_string(),
                    nome_projeto: "Complexo Aeronáutico Boeing B-29".to_string(),
                    nome_estado: "Washington (Seattle)".to_string(),
                    nivel_atual: 3,
                    nivel_alvo: 4,
                    progresso_pct: 20.0,
                    ic_alocada: 15,
                    semanas_restantes: 18,
                    pausado: false,
                },
                ItemFilaConstrucaoViewModel {
                    id: "proj_04".to_string(),
                    nome_projeto: "Usina Hidrelétrica TVA".to_string(),
                    nome_estado: "Tennessee".to_string(),
                    nivel_atual: 2,
                    nivel_alvo: 3,
                    progresso_pct: 10.0,
                    ic_alocada: 12,
                    semanas_restantes: 22,
                    pausado: false,
                },
            ],
            catalogo: vec![
                CatalogoPredioViewModel {
                    id: "siderurgica".to_string(),
                    nome: "Usina Siderúrgica & Metalurgia".to_string(),
                    categoria: "Industrial".to_string(),
                    custo_pontos_ic: 550,
                    insumos_necessarios: vec![("Aço".to_string(), 30), ("Cimento".to_string(), 25)],
                    empregos_por_nivel: 5000,
                    descricao: "Converte minério de ferro e carvão em aço estrutural e ligas pesadas.".to_string(),
                },
                CatalogoPredioViewModel {
                    id: "refinaria_quimica".to_string(),
                    nome: "Refinaria de Petróleo & Petroquímica".to_string(),
                    categoria: "Industrial".to_string(),
                    custo_pontos_ic: 550,
                    insumos_necessarios: vec![("Aço".to_string(), 25), ("Cimento".to_string(), 20)],
                    empregos_por_nivel: 5000,
                    descricao: "Refina petróleo em combustível para veículos, frotas e insumos sintéticos.".to_string(),
                },
                CatalogoPredioViewModel {
                    id: "fabrica_maquinas".to_string(),
                    nome: "Fábrica de Ferramentas & Motores".to_string(),
                    categoria: "Industrial".to_string(),
                    custo_pontos_ic: 500,
                    insumos_necessarios: vec![("Aço".to_string(), 20), ("Ferramentas".to_string(), 10)],
                    empregos_por_nivel: 5000,
                    descricao: "Produz maquinário pesado essencial para a manutenção de indústrias e minas.".to_string(),
                },
                CatalogoPredioViewModel {
                    id: "usina_eletrica".to_string(),
                    nome: "Usina Termelétrica / Hidrelétrica".to_string(),
                    categoria: "Infraestrutura".to_string(),
                    custo_pontos_ic: 450,
                    insumos_necessarios: vec![("Aço".to_string(), 15), ("Cimento".to_string(), 30)],
                    empregos_por_nivel: 5000,
                    descricao: "Gera eletricidade local para alimentar cidades e linhas fabris modernas.".to_string(),
                },
                CatalogoPredioViewModel {
                    id: "ferrovia_porto".to_string(),
                    nome: "Malha Ferroviária & Porto Comercial".to_string(),
                    categoria: "Infraestrutura".to_string(),
                    custo_pontos_ic: 450,
                    insumos_necessarios: vec![("Aço".to_string(), 30), ("Madeira".to_string(), 20)],
                    empregos_por_nivel: 5000,
                    descricao: "Expande a capacidade de transporte da província e garante acesso ao mercado nacional.".to_string(),
                },
                CatalogoPredioViewModel {
                    id: "industria_belica".to_string(),
                    nome: "Arsenal de Armamentos & Munições".to_string(),
                    categoria: "Militar".to_string(),
                    custo_pontos_ic: 600,
                    insumos_necessarios: vec![("Aço".to_string(), 35), ("Ferramentas".to_string(), 15)],
                    empregos_por_nivel: 5000,
                    descricao: "Produz armas portáteis, artilharia e munições para suprir divisões ativas.".to_string(),
                },
            ],
        }
    }
}
