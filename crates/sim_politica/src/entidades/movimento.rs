//! Movimentos políticos de pressão popular e movimentos revolucionários.
//!
//! No estilo Victoria 3, quando demandas populares ou de IGs não são atendidas,
//! formam-se movimentos para promulgar uma nova lei, preservar uma lei ameaçada
//! ou buscar libertação nacional anticolonial. Se o radicalismo ultrapassar o limiar,
//! o movimento aciona o termômetro revolucionário rumo à guerra civil.

use serde::{Deserialize, Serialize};

/// Objetivo reivindicado por um movimento político.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjetivoMovimento {
    /// Exige a aprovação e sanção de uma nova lei.
    PromulgarLei { categoria: String, lei_id: String },
    /// Reage contra a tentativa do governo de alterar uma lei existente estimada pelas massas.
    PreservarLei { categoria: String, lei_id: String },
    /// Exige a restauração de uma lei antiga revogada no passado recente.
    RestaurarLei { categoria: String, lei_id: String },
    /// Luta pela expulsão da administração colonial e independência nacional soberana.
    LibertacaoAnticolonial,
}

/// Movimento político organizado na sociedade civil.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovimentoPolitico {
    pub id: String,
    pub nome: String,
    pub objetivo: ObjetivoMovimento,
    /// Grupos de Interesse que aderiram formalmente ao movimento.
    pub grupos_apoiadores: Vec<String>,
    /// Nível de apoio na sociedade (0.0 a 100.0), derivado do clout dos IGs e contagem de Pops.
    #[serde(default)]
    pub apoio: f64,
    /// Nível de radicalismo das massas engajadas no movimento (0.0 a 100.0).
    #[serde(default)]
    pub radicalismo: f64,
    /// Termômetro de revolução armada (0.0 a 100.0%). Ao atingir 100%, deflagra guerra civil.
    #[serde(default)]
    pub termometro_revolucao: f64,
    /// Se `true`, o movimento transpôs a fase de petição e iniciou insurreição armada aberta.
    #[serde(default)]
    pub em_revolucao: bool,
}

impl MovimentoPolitico {
    /// Cria um novo movimento político pacífico.
    pub fn novo(
        id: impl Into<String>,
        nome: impl Into<String>,
        objetivo: ObjetivoMovimento,
    ) -> Self {
        Self {
            id: id.into(),
            nome: nome.into(),
            objetivo,
            grupos_apoiadores: Vec::new(),
            apoio: 15.0,
            radicalismo: 10.0,
            termometro_revolucao: 0.0,
            em_revolucao: false,
        }
    }

    /// Retorna verdadeiro se o movimento ultrapassou o limiar pacífico e tornou-se revolucionário.
    #[inline]
    pub fn eh_revolucionario(&self) -> bool {
        self.apoio >= 20.0 && self.radicalismo >= 45.0
    }
}
