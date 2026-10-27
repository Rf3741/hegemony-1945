//! View-model para o Painel do País (Dossiê Nacional - F1).

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct PaisViewModel {
    pub tag: String,
    pub nome_oficial: String,
    pub capital: String,
    pub forma_governo: String,
    pub ideologia_dominante: String,
    pub lider_nome: String,
    pub lider_cargo: String,
    pub vice_lider_nome: String,
    pub ano_fundacao_ou_constituicao: u16,

    // Balanço Fiscal Semanal
    pub receita_semanal_milhoes: f64,
    pub despesa_semanal_milhoes: f64,
    pub saldo_semanal_milhoes: f64,
    pub divida_publica_bilhoes: f64,
    pub teto_endividamento_bilhoes: f64,
    pub taxa_juros_divida_pct: f32,
    pub reservas_ouro_bilhoes: f64,

    // Capacidade Industrial (IC)
    pub ic_fabricas_civis: u32,
    pub ic_fabricas_militares: u32,
    pub ic_estaleiros_navais: u32,

    // Relações Internacionais
    pub num_aliados_esfera: u32,
    pub status_bretton_woods: String,
    pub status_lend_lease: String,

    // Decomposição de Receitas e Despesas para Tooltips
    pub fontes_receita: Vec<(String, f64)>,
    pub fontes_despesa: Vec<(String, f64)>,
}

impl Default for PaisViewModel {
    fn default() -> Self {
        Self {
            tag: "USA".to_string(),
            nome_oficial: "Estados Unidos da América".to_string(),
            capital: "Washington, D.C.".to_string(),
            forma_governo: "República Presidencialista Constitucional".to_string(),
            ideologia_dominante: "Democracia Liberal / Capitalismo".to_string(),
            lider_nome: "Franklin D. Roosevelt".to_string(),
            lider_cargo: "Presidente dos Estados Unidos".to_string(),
            vice_lider_nome: "Harry S. Truman".to_string(),
            ano_fundacao_ou_constituicao: 1787,
            receita_semanal_milhoes: 184.5,
            despesa_semanal_milhoes: 170.3,
            saldo_semanal_milhoes: 14.2,
            divida_publica_bilhoes: 258.7,
            teto_endividamento_bilhoes: 300.0,
            taxa_juros_divida_pct: 2.1,
            reservas_ouro_bilhoes: 20.2,
            ic_fabricas_civis: 82,
            ic_fabricas_militares: 104,
            ic_estaleiros_navais: 24,
            num_aliados_esfera: 24,
            status_bretton_woods: "Padrão-Ouro Bretton Woods (US$ 35/oz) — Hegemônico".to_string(),
            status_lend_lease: "Programa Lend-Lease Ativo (Suprimentos para URSS, GBR e FRA)".to_string(),
            fontes_receita: vec![
                ("Imposto de Renda Corporativo".to_string(), 85.2),
                ("Imposto de Renda Pessoal".to_string(), 42.4),
                ("Impostos sobre Consumo / Vendas".to_string(), 32.1),
                ("Tarifas Alfandegárias de Importação".to_string(), 14.3),
                ("Rendimentos de Reservas em Ouro".to_string(), 10.5),
            ],
            fontes_despesa: vec![
                ("Manutenção de Divisões do Exército".to_string(), 78.4),
                ("Logística Naval e Frota do Pacífico".to_string(), 46.2),
                ("Pesquisa Estratégica (Projeto Manhattan)".to_string(), 21.5),
                ("Salários da Burocracia Federal".to_string(), 12.8),
                ("Serviço da Dívida Pública de Guerra".to_string(), 11.4),
            ],
        }
    }
}
