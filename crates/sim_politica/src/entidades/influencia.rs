//! Esferas de influência geopolítica e operações de inteligência da Guerra Fria (EUA vs. URSS).

use crate::tipos::{AlinhamentoBloco, NivelEsfera, PotenciaBloco};
use serde::{Deserialize, Serialize};

/// Operação ativa de financiamento, propaganda ou ajuda externa promovida por superpotência.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperacaoInfluencia {
    /// Financia um Grupo de Interesse aliado para expandir seu clout e atração social.
    FinanciarGrupoInteresse { gi_id: String, montante_diario: f64 },
    /// Financia a campanha eleitoral de um partido para aumentar seus votos nas urnas.
    FinanciarPartido { partido_id: String, montante: f64 },
    /// Emite propaganda ideológica (ondas curtas, panfletos, imprensa) para converter Pops.
    PropagandaIdeologica {
        ideologia_alvo: String,
        intensidade: f64,
    },
    /// Programa governamental de ajuda externa condicionada (Plano Marshall / COMECON).
    AjudaEconomicaCondicionada {
        programa: String,
        montante_diario: f64,
    },
}

/// Registro das pressões geopolíticas e laços de dependência de um país de terceiro porte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistroInfluenciaExterna {
    /// Pontos de influência acumulados pelos Estados Unidos (0.0 a 100.0).
    pub pontos_eua: f64,
    /// Pontos de influência acumulados pela União Soviética (0.0 a 100.0).
    pub pontos_urss: f64,
    /// Nível formal de vinculação à esfera norte-americana.
    pub nivel_esfera_eua: NivelEsfera,
    /// Nível formal de vinculação à esfera soviética.
    pub nivel_esfera_urss: NivelEsfera,
    /// Alinhamento diplomático de bloco no cenário internacional.
    pub alinhamento: AlinhamentoBloco,
    /// Operações ativas conduzidas pelos EUA no país.
    #[serde(default)]
    pub operacoes_eua: Vec<OperacaoInfluencia>,
    /// Operações ativas conduzidas pela URSS no país.
    #[serde(default)]
    pub operacoes_urss: Vec<OperacaoInfluencia>,
}

impl Default for RegistroInfluenciaExterna {
    fn default() -> Self {
        Self {
            pontos_eua: 10.0,
            pontos_urss: 10.0,
            nivel_esfera_eua: NivelEsfera::ContatoDiplomatico,
            nivel_esfera_urss: NivelEsfera::ContatoDiplomatico,
            alinhamento: AlinhamentoBloco::NaoAlinhado,
            operacoes_eua: Vec::new(),
            operacoes_urss: Vec::new(),
        }
    }
}

impl RegistroInfluenciaExterna {
    /// Atualiza os níveis de esfera com base nos pontos de influência.
    pub fn sincronizar_niveis(&mut self) {
        self.nivel_esfera_eua = NivelEsfera::de_pontos(self.pontos_eua);
        self.nivel_esfera_urss = NivelEsfera::de_pontos(self.pontos_urss);

        // Se uma superpotência alcança alinhamento ou integração dominante, atualiza o bloco
        if self.pontos_eua >= 60.0 && self.pontos_eua > self.pontos_urss + 20.0 {
            self.alinhamento = AlinhamentoBloco::BlocoOcidental;
        } else if self.pontos_urss >= 60.0 && self.pontos_urss > self.pontos_eua + 20.0 {
            self.alinhamento = AlinhamentoBloco::BlocoSovietico;
        }
    }

    /// Executa a regra do cabo de guerra (tug-of-war) competitivo da Guerra Fria.
    ///
    /// Quando uma superpotência ganha influência significativa, desgasta a presença da rival.
    pub fn aplicar_cabo_de_guerra(&mut self, potencia: PotenciaBloco, delta: f64) {
        match potencia {
            PotenciaBloco::Eua => {
                self.pontos_eua = (self.pontos_eua + delta).clamp(0.0, 100.0);
                if self.pontos_eua > 50.0 {
                    let atrito = (self.pontos_eua - 50.0) * 0.02 * delta.max(0.0);
                    self.pontos_urss = (self.pontos_urss - atrito).max(0.0);
                }
            }
            PotenciaBloco::Urss => {
                self.pontos_urss = (self.pontos_urss + delta).clamp(0.0, 100.0);
                if self.pontos_urss > 50.0 {
                    let atrito = (self.pontos_urss - 50.0) * 0.02 * delta.max(0.0);
                    self.pontos_eua = (self.pontos_eua - atrito).max(0.0);
                }
            }
        }
        self.sincronizar_niveis();
    }
}
