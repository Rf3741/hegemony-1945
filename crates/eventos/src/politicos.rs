//! Carga e interpretação de `eventos_politicos.ron` (módulo evolucao_ideologica).

use crate::condicoes::Condicao;
use crate::{Evento, Modificador, OpcaoEvento};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CondicoesEvento {
    #[serde(default)]
    pub leis_obrigatorias: Vec<String>,
    #[serde(default)]
    pub leis_proibidas: Vec<String>,
    #[serde(default = "padrao_legitimidade_maxima")]
    pub legitimidade_maxima: f32,
}

fn padrao_legitimidade_maxima() -> f32 {
    100.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpcaoEventoPolitico {
    pub id: String,
    pub texto: String,
    #[serde(default)]
    pub descricao_resultado: String,
    #[serde(default)]
    pub efeitos: Vec<Modificador>,
    #[serde(default)]
    pub ia_peso: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventoPolitico {
    pub id: String,
    pub titulo: String,
    #[serde(default)]
    pub categoria: String,
    pub descricao: String,
    pub condicoes: CondicoesEvento,
    pub opcoes: Vec<OpcaoEventoPolitico>,
}

impl EventoPolitico {
    /// Converte o formato do `eventos_politicos.ron` no modelo interno `Evento`.
    pub fn em_evento(self) -> Evento {
        let mut condicoes = Vec::new();
        for l in self.condicoes.leis_obrigatorias {
            condicoes.push(Condicao::TemLei(l));
        }
        for l in self.condicoes.leis_proibidas {
            condicoes.push(Condicao::NaoTemLei(l));
        }
        if self.condicoes.legitimidade_maxima < 100.0 {
            condicoes.push(Condicao::LegitimidadeMax(self.condicoes.legitimidade_maxima));
        }
        let opcoes = self
            .opcoes
            .into_iter()
            .map(|o| OpcaoEvento {
                id: o.id,
                texto: o.texto,
                descricao_resultado: o.descricao_resultado,
                efeitos: o.efeitos,
                ia_peso: o.ia_peso,
            })
            .collect();

        Evento {
            id: self.id,
            titulo: self.titulo,
            descricao: self.descricao,
            data: (1945, 1, 1),
            paises: Vec::new(), // Genérico: pode disparar em qualquer país que atenda às condições
            condicoes,
            opcoes,
            chance: 1.0,
            generico: true,
        }
    }
}
