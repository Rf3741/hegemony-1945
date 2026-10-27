//! Crises políticas agudas (greves, revoltas, golpes militares) e sinal de guerra civil
//! emitido para o motor de guerra (`sim_guerra`).

use crate::tipos::{DataJogo, OrigemGuerraCivil, PotenciaBloco, TipoCrisePolitica};
use serde::{Deserialize, Serialize};

/// Crise política e social em andamento no país.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriseInterna {
    pub id: String,
    pub tipo_crise: TipoCrisePolitica,
    /// Grupo de Interesse que lidera ou provocou a crise (se houver).
    pub grupo_origem_id: Option<String>,
    /// Movimento político na raiz da insurreição (se houver).
    pub movimento_origem_id: Option<String>,
    /// Gravidade e intensidade da crise (0.0 a 100.0).
    pub intensidade: f64,
    /// Ticks diários em que a crise permanece sem solução.
    #[serde(default)]
    pub dias_ativa: u32,
    /// Indicador de encerramento da crise.
    #[serde(default)]
    pub resolvida: bool,
}

impl CriseInterna {
    /// Cria uma nova crise política ativa.
    pub fn nova(
        id: impl Into<String>,
        tipo_crise: TipoCrisePolitica,
        grupo_origem_id: Option<String>,
        intensidade: f64,
    ) -> Self {
        Self {
            id: id.into(),
            tipo_crise,
            grupo_origem_id,
            movimento_origem_id: None,
            intensidade: intensidade.clamp(1.0, 100.0),
            dias_ativa: 0,
            resolvida: false,
        }
    }
}

/// Contrato estruturado de evento de Guerra Civil transmitido ao futuro crate `sim_guerra`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SinalGuerraCivil {
    /// ID do país onde eclodiu o conflito.
    pub pais_id: String,
    /// Tag de 3 letras do país (ex.: `"USA"`, `"SOV"`, `"BRA"`).
    pub pais_tag: String,
    /// Data histórica de eclosão dos combates armados.
    pub data_eclosao: DataJogo,
    /// Causa motriz da ruptura institucional.
    pub origem: OrigemGuerraCivil,
    /// Identificador único da facção rebelde beligerante.
    pub faccao_rebelde_id: String,
    /// Nome em pt-BR da facção insurrecta (ex.: `"Frente Revolucionária Proletária"`).
    pub nome_faccao_rebelde: String,
    /// Ideologia hegemônica das forças rebeldes.
    pub ideologia_rebelde_id: String,
    /// Grupos de Interesse que aderiram ao levante rebelde armado.
    pub grupos_rebeldes: Vec<String>,
    /// Grupos de Interesse que permaneceram fiéis ao governo legalista.
    pub grupos_legalistas: Vec<String>,
    /// Lei reivindicada pelas armas (`(categoria, nova_lei_id)`).
    pub lei_demandada: Option<(String, String)>,
    /// Fração do contingente militar e armas que desertou para a rebelião (0.0 a 1.0).
    pub proporcao_exercito_rebelde: f64,
    /// Superpotência da Guerra Fria que financiou ou armou os rebeldes.
    pub patrocinador_externo: Option<PotenciaBloco>,
}
