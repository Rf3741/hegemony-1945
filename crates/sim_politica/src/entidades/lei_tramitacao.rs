//! Tramitação de projetos de lei no parlamento e cálculo de checkpoints legislativos.

use crate::tipos::FaseLegislativa;
use serde::{Deserialize, Serialize};

/// Projeto de lei atualmente em deliberação legislativa no país.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TramitacaoLei {
    /// Categoria institucional da lei (ex.: `"forma_governo"`, `"economia"`, `"trabalho"`).
    pub categoria: String,
    /// ID da lei que se pretende aprovar e sancionar.
    pub lei_alvo_id: String,
    /// ID da lei atualmente vigente na categoria que será revogada se o projeto for aprovado.
    pub lei_anterior_id: String,
    /// Fase atual do processo legislativo.
    pub fase: FaseLegislativa,
    /// Progresso acumulado na fase atual (`0.0..=100.0`).
    #[serde(default)]
    pub progresso_fase: f64,
    /// Ticks diários transcorridos desde a entrada na fase atual.
    #[serde(default)]
    pub dias_na_fase: u32,
    /// Dias restantes até a próxima avaliação por checkpoint.
    #[serde(default = "padrao_checkpoint")]
    pub dias_restantes_checkpoint: u32,
    /// Quantidade de entraves/travamentos sofridos (ao atingir 3 entraves, o projeto é cancelado).
    #[serde(default)]
    pub contagem_entraves: u32,
    /// Probabilidade de avanço calculada no último checkpoint (0.0 a 1.0).
    #[serde(default)]
    pub chance_avanco: f64,
    /// Probabilidade de debate parlamentar no último checkpoint (0.0 a 1.0).
    #[serde(default)]
    pub chance_debate: f64,
    /// Probabilidade de travamento / entrave no último checkpoint (0.0 a 1.0).
    #[serde(default)]
    pub chance_travar: f64,
    /// Modificador cumulativo de chance de sucesso obtido em negociações e debates.
    #[serde(default)]
    pub modificador_sucesso_debate: f64,
    /// Grupos de Interesse que apoiam a tramitação deste projeto.
    #[serde(default)]
    pub apoiadores_ativos: Vec<String>,
    /// Grupos de Interesse que se opõem ativamente à aprovação.
    #[serde(default)]
    pub opositores_ativos: Vec<String>,
}

const fn padrao_checkpoint() -> u32 {
    180
}

impl TramitacaoLei {
    /// Inicia uma nova tramitação legislativa.
    pub fn nova(
        categoria: impl Into<String>,
        lei_alvo_id: impl Into<String>,
        lei_anterior_id: impl Into<String>,
    ) -> Self {
        Self {
            categoria: categoria.into(),
            lei_alvo_id: lei_alvo_id.into(),
            lei_anterior_id: lei_anterior_id.into(),
            fase: FaseLegislativa::Introducao,
            progresso_fase: 0.0,
            dias_na_fase: 0,
            dias_restantes_checkpoint: 180,
            contagem_entraves: 0,
            chance_avanco: 0.35,
            chance_debate: 0.45,
            chance_travar: 0.20,
            modificador_sucesso_debate: 0.0,
            apoiadores_ativos: Vec::new(),
            opositores_ativos: Vec::new(),
        }
    }
}
