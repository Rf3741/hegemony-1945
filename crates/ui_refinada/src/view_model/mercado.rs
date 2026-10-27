//! View-model para a Tela de Mercado e Comércio (F3).
//! Tabela de cotações de bens com desvio, oferta/demanda e série temporal para egui_plot.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatusComercio {
    Neutro,
    Exportando,
    Importando,
    Subsidiado,
    Tarifado,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CotacaoBemViewModel {
    pub id: String,
    pub nome: String,
    pub categoria: String,
    pub preco_atual: f64,
    pub preco_base: f64,
    pub desvio_pct: f32,
    pub oferta_nacional: f64,
    pub demanda_nacional: f64,
    pub saldo_liquido: f64,
    pub status: StatusComercio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PontoHistorico {
    pub tempo: f64, // Ano ou Semana (ex: 1940.0 ..= 1945.0)
    pub preco: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct MercadoViewModel {
    pub bens: Vec<CotacaoBemViewModel>,
    pub bem_selecionado_id: String,
    pub historico_precos: Vec<PontoHistorico>,
    pub maiores_produtores: Vec<(String, f64)>,
    pub maiores_consumidores: Vec<(String, f64)>,
}

impl Default for MercadoViewModel {
    fn default() -> Self {
        Self {
            bens: vec![
                CotacaoBemViewModel {
                    id: "aco".to_string(),
                    nome: "Aço Estrutural e Bélico".to_string(),
                    categoria: "Industrial".to_string(),
                    preco_atual: 44.20,
                    preco_base: 50.00,
                    desvio_pct: -11.6,
                    oferta_nacional: 8200.0,
                    demanda_nacional: 7100.0,
                    saldo_liquido: 1100.0,
                    status: StatusComercio::Exportando,
                },
                CotacaoBemViewModel {
                    id: "petroleo".to_string(),
                    nome: "Petróleo Cru & Combustível".to_string(),
                    categoria: "Energia".to_string(),
                    preco_atual: 58.50,
                    preco_base: 50.00,
                    desvio_pct: 17.0,
                    oferta_nacional: 6400.0,
                    demanda_nacional: 7200.0,
                    saldo_liquido: -800.0,
                    status: StatusComercio::Importando,
                },
                CotacaoBemViewModel {
                    id: "carvao".to_string(),
                    nome: "Carvão Mineral".to_string(),
                    categoria: "Energia".to_string(),
                    preco_atual: 29.80,
                    preco_base: 30.00,
                    desvio_pct: -0.7,
                    oferta_nacional: 9800.0,
                    demanda_nacional: 9750.0,
                    saldo_liquido: 50.0,
                    status: StatusComercio::Neutro,
                },
                CotacaoBemViewModel {
                    id: "ferro".to_string(),
                    nome: "Minério de Ferro".to_string(),
                    categoria: "MateriaPrima".to_string(),
                    preco_atual: 36.40,
                    preco_base: 40.00,
                    desvio_pct: -9.0,
                    oferta_nacional: 7400.0,
                    demanda_nacional: 6900.0,
                    saldo_liquido: 500.0,
                    status: StatusComercio::Neutro,
                },
                CotacaoBemViewModel {
                    id: "graos".to_string(),
                    nome: "Grãos e Cereais".to_string(),
                    categoria: "AlimentoBasico".to_string(),
                    preco_atual: 18.20,
                    preco_base: 20.00,
                    desvio_pct: -9.0,
                    oferta_nacional: 12500.0,
                    demanda_nacional: 11200.0,
                    saldo_liquido: 1300.0,
                    status: StatusComercio::Exportando,
                },
                CotacaoBemViewModel {
                    id: "ferramentas".to_string(),
                    nome: "Máquinas e Ferramentas".to_string(),
                    categoria: "Industrial".to_string(),
                    preco_atual: 62.00,
                    preco_base: 55.00,
                    desvio_pct: 12.7,
                    oferta_nacional: 4100.0,
                    demanda_nacional: 4600.0,
                    saldo_liquido: -500.0,
                    status: StatusComercio::Subsidiado,
                },
                CotacaoBemViewModel {
                    id: "roupas".to_string(),
                    nome: "Vestuário e Têxteis".to_string(),
                    categoria: "Consumo".to_string(),
                    preco_atual: 33.50,
                    preco_base: 35.00,
                    desvio_pct: -4.3,
                    oferta_nacional: 8900.0,
                    demanda_nacional: 8600.0,
                    saldo_liquido: 300.0,
                    status: StatusComercio::Neutro,
                },
                CotacaoBemViewModel {
                    id: "armas_municoes".to_string(),
                    nome: "Armamentos e Munições".to_string(),
                    categoria: "Militar".to_string(),
                    preco_atual: 74.00,
                    preco_base: 65.00,
                    desvio_pct: 13.8,
                    oferta_nacional: 5200.0,
                    demanda_nacional: 5900.0,
                    saldo_liquido: -700.0,
                    status: StatusComercio::Tarifado,
                },
            ],
            bem_selecionado_id: "aco".to_string(),
            historico_precos: vec![
                PontoHistorico { tempo: 1940.0, preco: 38.0 },
                PontoHistorico { tempo: 1941.0, preco: 42.5 },
                PontoHistorico { tempo: 1942.0, preco: 48.0 },
                PontoHistorico { tempo: 1943.0, preco: 52.0 },
                PontoHistorico { tempo: 1944.0, preco: 47.5 },
                PontoHistorico { tempo: 1944.5, preco: 45.0 },
                PontoHistorico { tempo: 1945.0, preco: 44.2 },
            ],
            maiores_produtores: vec![
                ("Pensilvânia (Pittsburgh)".to_string(), 3200.0),
                ("Ohio (Cleveland)".to_string(), 1800.0),
                ("Indiana (Gary)".to_string(), 1400.0),
                ("Alabama (Birmingham)".to_string(), 900.0),
                ("Maryland (Sparrows Point)".to_string(), 900.0),
            ],
            maiores_consumidores: vec![
                ("Indústria Bélica e Arsenais".to_string(), 2800.0),
                ("Construção Civil e Obras Públicas".to_string(), 2100.0),
                ("Estaleiros Navais do Atlântico e Pacífico".to_string(), 1200.0),
                ("Fábricas de Ferramentas e Máquinas".to_string(), 1000.0),
            ],
        }
    }
}
