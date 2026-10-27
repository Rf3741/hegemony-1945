//! Partidos políticos formados por coalizões dinâmicas de Grupos de Interesse.

use serde::{Deserialize, Serialize};

/// Partido político atuante no cenário eleitoral e parlamentar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanciaPartido {
    /// Identificador único (geralmente coincidente com o arquétipo gerador).
    pub id: String,
    /// Nome de exibição em pt-BR (ex.: `"Partido Social-Democrata"`, `"Democracia Cristã"`).
    pub nome: String,
    /// Arquétipo doutrinário que orienta o partido.
    pub arquetipo_id: String,
    /// Lista de Grupos de Interesse atualmente filiados ao partido.
    pub grupos_membros: Vec<String>,
    /// Momento eleitoral acumulado durante o período de campanha (-50.0 a +50.0).
    #[serde(default)]
    pub momento_campanha: f64,
    /// Financiamento externo direto de superpotência estrangeira para a campanha do partido.
    #[serde(default)]
    pub bonus_financiamento_externo: f64,
    /// Total de votos populares recebidos na última eleição.
    #[serde(default)]
    pub votos_ultima_eleicao: f64,
    /// Porcentagem de votos válidos obtida na última eleição (0.0 a 1.0).
    #[serde(default)]
    pub percentual_votos_ultima_eleicao: f64,
    /// Quantidade de assentos ocupados no parlamento (total de 100 cadeiras).
    #[serde(default)]
    pub cadeiras_parlamento: u32,
}

impl InstanciaPartido {
    /// Cria um novo partido associado a um arquétipo.
    pub fn novo(
        id: impl Into<String>,
        nome: impl Into<String>,
        arquetipo_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            nome: nome.into(),
            arquetipo_id: arquetipo_id.into(),
            grupos_membros: Vec::new(),
            momento_campanha: 0.0,
            bonus_financiamento_externo: 0.0,
            votos_ultima_eleicao: 0.0,
            percentual_votos_ultima_eleicao: 0.0,
            cadeiras_parlamento: 0,
        }
    }
}
