//! Gerenciamento de modificadores temporários com duração em dias.

use serde::{Deserialize, Serialize};

/// Modificador temporário ativo com contagem regressiva de dias.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModificadorAtivo {
    pub pais: String,
    pub id: String,
    pub valor: f32,
    pub dias_restantes: u32,
    pub duracao_total: u32,
}

impl ModificadorAtivo {
    pub fn novo(pais: impl Into<String>, id: impl Into<String>, valor: f32, duracao_dias: u32) -> Self {
        Self {
            pais: pais.into(),
            id: id.into(),
            valor,
            dias_restantes: duracao_dias,
            duracao_total: duracao_dias,
        }
    }

    /// Retorna `true` se o modificador expirou.
    pub fn expirado(&self) -> bool {
        self.dias_restantes == 0
    }
}

/// Agregação de modificadores ativos por categoria conceitual.
pub fn pertence_a_categoria(mod_id: &str, categoria: &str) -> bool {
    let m = mod_id.to_lowercase();
    match categoria {
        "economia" => {
            m.contains("pib")
                || m.contains("industrial")
                || m.contains("investimento")
                || m.contains("tributaria")
                || m.contains("producao")
                || m.contains("construcao")
                || m.contains("renda")
                || m.contains("economia")
                || m.contains("alimentos")
                || m.contains("comercial")
        }
        "estabilidade" => {
            m.contains("estabilidade")
                || m.contains("legitimidade")
                || m.contains("autoridade")
                || m.contains("coesao")
                || m.contains("ordem")
                || m.contains("paz_social")
        }
        "radicalismo" => {
            m.contains("radicalismo")
                || m.contains("agitacao")
                || m.contains("insurreicao")
                || m.contains("revolta")
                || m.contains("golpe")
                || m.contains("revolucao")
        }
        outro => m == outro,
    }
}
