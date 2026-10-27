//! Liderança política de Grupos de Interesse, chefes de Estado e chefes de governo.
//!
//! A ideologia pessoal do líder é uma mecânica central no estilo Victoria 3:
//! ela modula ou sobrepõe a postura do grupo diante de projetos de lei e direciona
//! sua afinidade com partidos políticos.

use serde::{Deserialize, Serialize};

/// Líder de um Grupo de Interesse ou dirigente nacional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiderPolitico {
    /// Identificador único em snake_case ASCII.
    pub id: String,
    /// Nome histórico ou gerado (ex.: `"Harry S. Truman"`, `"Iosif Stalin"`).
    pub nome: String,
    /// Ideologia pessoal do líder (chave no catálogo de ideologias).
    pub ideologia_id: String,
    /// Popularidade pessoal perante a opinião pública (-100.0 a +100.0).
    #[serde(default)]
    pub popularidade: f64,
    /// Multiplicador de magnetismo retórico e capacidade de persuasão (0.5 a 2.0).
    #[serde(default = "padrao_carisma")]
    pub carisma: f64,
    /// Ano de nascimento do personagem.
    #[serde(default = "padrao_ano_nasc")]
    pub ano_nascimento: u16,
    /// Traços de personalidade e competência governamental.
    #[serde(default)]
    pub tracos: Vec<String>,
}

const fn padrao_carisma() -> f64 {
    1.0
}

const fn padrao_ano_nasc() -> u16 {
    1900
}

impl LiderPolitico {
    /// Cria um novo líder político.
    pub fn novo(
        id: impl Into<String>,
        nome: impl Into<String>,
        ideologia_id: impl Into<String>,
        popularidade: f64,
        carisma: f64,
        ano_nascimento: u16,
    ) -> Self {
        Self {
            id: id.into(),
            nome: nome.into(),
            ideologia_id: ideologia_id.into(),
            popularidade: popularidade.clamp(-100.0, 100.0),
            carisma: carisma.clamp(0.2, 3.0),
            ano_nascimento,
            tracos: Vec::new(),
        }
    }

    /// Retorna o bônus de momento eleitoral gerado pelo líder durante campanhas.
    #[inline]
    pub fn modificador_momento_eleitoral(&self) -> f64 {
        let fator_pop = (self.popularidade / 100.0) * 0.25;
        let fator_carisma = (self.carisma - 1.0) * 0.15;
        fator_pop + fator_carisma
    }
}
