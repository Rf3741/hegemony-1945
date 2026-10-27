//! Representação de uma População (Pop) sob a ótica política:
//! estrato, padrão de vida, alfabetização, radicais, leais e distribuição de simpatia por GI.

use crate::erro::{ErroPolitica, ResultadoPolitica};
use crate::tipos::Estrato;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Grupo homogêneo de cidadãos com mesma profissão, cultura e religião em um país.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PopPolitica {
    /// Profissão / classe (ex.: `"operarios"`, `"camponeses"`, `"capitalistas"`).
    pub tipo_pop_id: String,
    pub cultura: String,
    pub religiao: String,
    pub estrato: Estrato,
    /// Contingente total de indivíduos adultos neste grupo (>= 0.0).
    pub populacao: f64,
    /// Cidadãos radicalizados ativos (0.0..=populacao).
    #[serde(default)]
    pub radicais: f64,
    /// Cidadãos leais e satisfeitos com a ordem vigente (0.0..=populacao).
    #[serde(default)]
    pub leais: f64,
    /// Padrão de vida material atual (escala 1.0 a 99.0).
    pub padrao_de_vida: f64,
    /// Padrão de vida mínimo esperado culturalmente para o estrato e instrução.
    pub padrao_de_vida_esperado: f64,
    /// Taxa de alfabetização (0.0 a 1.0).
    pub alfabetizacao: f64,
    /// Taxa de desemprego recente deste grupo (0.0 a 1.0).
    #[serde(default)]
    pub taxa_desemprego: f64,
    /// Poder político bruto calculado no último tick per capita.
    #[serde(default)]
    pub forca_politica_bruta: f64,
    /// Distribuição percentual de atração para cada Grupo de Interesse (`gi_id -> fracao_0_1`).
    #[serde(default)]
    pub apoio_por_gi: BTreeMap<String, f64>,
}

impl PopPolitica {
    /// Cria um novo Pop com valores padrão para radicais e leais.
    pub fn novo(
        tipo_pop_id: impl Into<String>,
        cultura: impl Into<String>,
        religiao: impl Into<String>,
        estrato: Estrato,
        populacao: f64,
        padrao_de_vida: f64,
        alfabetizacao: f64,
    ) -> Self {
        let sol = padrao_de_vida.clamp(1.0, 99.0);
        let alfa = alfabetizacao.clamp(0.0, 1.0);
        let sol_esp = match estrato {
            Estrato::Baixo => 6.0 + 8.0 * alfa,
            Estrato::Medio => 12.0 + 10.0 * alfa,
            Estrato::Alto => 22.0 + 12.0 * alfa,
        };

        Self {
            tipo_pop_id: tipo_pop_id.into(),
            cultura: cultura.into(),
            religiao: religiao.into(),
            estrato,
            populacao: populacao.max(0.0),
            radicais: 0.0,
            leais: 0.0,
            padrao_de_vida: sol,
            padrao_de_vida_esperado: sol_esp,
            alfabetizacao: alfa,
            taxa_desemprego: 0.0,
            forca_politica_bruta: 0.0,
            apoio_por_gi: BTreeMap::new(),
        }
    }

    /// Contingente de cidadãos neutros (nem radicais, nem leais).
    #[inline]
    pub fn neutros(&self) -> f64 {
        (self.populacao - self.radicais - self.leais).max(0.0)
    }

    /// Proporção de radicais na população do grupo (`0.0..=1.0`).
    #[inline]
    pub fn fracao_radicais(&self) -> f64 {
        if self.populacao <= 0.0 {
            0.0
        } else {
            (self.radicais / self.populacao).clamp(0.0, 1.0)
        }
    }

    /// Proporção de leais na população do grupo (`0.0..=1.0`).
    #[inline]
    pub fn fracao_leais(&self) -> f64 {
        if self.populacao <= 0.0 {
            0.0
        } else {
            (self.leais / self.populacao).clamp(0.0, 1.0)
        }
    }

    /// Valida invariantes estruturais do Pop.
    pub fn validar_invariantes(&self) -> ResultadoPolitica<()> {
        if !self.populacao.is_finite() || self.populacao < 0.0 {
            return Err(ErroPolitica::ValorInvalido {
                entidade: "PopPolitica",
                id: self.tipo_pop_id.clone(),
                detalhe: "população deve ser positiva e finita".into(),
            });
        }
        if self.radicais < 0.0
            || self.leais < 0.0
            || (self.radicais + self.leais) > self.populacao * 1.0001
        {
            return Err(ErroPolitica::InvarianteViolada {
                contexto: format!("PopPolitica '{}'", self.tipo_pop_id),
                detalhe: format!(
                    "soma de radicais ({}) e leais ({}) excede a população total ({})",
                    self.radicais, self.leais, self.populacao
                ),
            });
        }
        Ok(())
    }
}
