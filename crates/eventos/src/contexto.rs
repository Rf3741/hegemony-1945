//! Contexto unificado de simulação para aplicação de efeitos cruzados de eventos.

use sim_diplomacia::Diplomacia;
use sim_economia::MundoEconomico;
use sim_politica::MundoPolitico;
use simulacao::mundo::MundoJogo;

/// Estrutura que disponibiliza as referências aos motores de simulação disponíveis no momento da decisão.
pub struct ContextoEfeitos<'a> {
    pub mundo: &'a mut MundoJogo,
    pub politica: Option<&'a mut MundoPolitico>,
    pub economia: Option<&'a mut MundoEconomico>,
    pub diplomacia: Option<&'a mut Diplomacia>,
}

impl<'a> ContextoEfeitos<'a> {
    /// Contexto clássico operando exclusivamente sobre `MundoJogo`.
    pub fn apenas_mundo(mundo: &'a mut MundoJogo) -> Self {
        Self {
            mundo,
            politica: None,
            economia: None,
            diplomacia: None,
        }
    }

    /// Contexto enriquecido com o motor político Victoria 3 (`sim_politica`).
    pub fn com_politica(
        mundo: &'a mut MundoJogo,
        politica: &'a mut MundoPolitico,
    ) -> Self {
        Self {
            mundo,
            politica: Some(politica),
            economia: None,
            diplomacia: None,
        }
    }

    /// Contexto enriquecido com o motor macroeconômico (`sim_economia`).
    pub fn com_economia(
        mundo: &'a mut MundoJogo,
        economia: &'a mut MundoEconomico,
    ) -> Self {
        Self {
            mundo,
            politica: None,
            economia: Some(economia),
            diplomacia: None,
        }
    }

    /// Contexto completo contendo todos os subsistemas interconectados.
    pub fn com_tudo(
        mundo: &'a mut MundoJogo,
        politica: &'a mut MundoPolitico,
        economia: &'a mut MundoEconomico,
        diplomacia: &'a mut Diplomacia,
    ) -> Self {
        Self {
            mundo,
            politica: Some(politica),
            economia: Some(economia),
            diplomacia: Some(diplomacia),
        }
    }
}
