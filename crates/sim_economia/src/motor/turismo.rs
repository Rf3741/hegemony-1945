//! Turismo e hotelaria: demanda internacional gerada pela renda das classes média e alta de cada
//! país, distribuída entre destinos conforme a atratividade histórica (base dos anos 50). A renda
//! é exportação de serviços (soma ao PIB) e o governo recolhe uma fração como imposto de serviços.
//! A demanda cai com guerra/instabilidade (risco externo), calote e desemprego alto.

use super::MundoEconomico;
use std::collections::BTreeMap;

/// Fração do PIB diário que vira gasto turístico internacional (emissão) num país de renda média mundial.
pub const FRACAO_PIB_EMISSAO: f64 = 0.004;
/// Fração do gasto turístico recebido que o governo arrecada (impostos sobre hotelaria e serviços).
pub const IMPOSTO_TURISMO: f64 = 0.20;
/// Atratividade para países sem entrada na tabela.
const ATRATIVIDADE_PADRAO: f64 = 0.15;
/// Atratividade de países atrás da Cortina de Ferro (turismo quase inexistente em 1945-60).
const ATRATIVIDADE_CORTINA: f64 = 0.02;

/// Atratividade histórica (0..1) por país.
pub fn atratividade_base(id: &str) -> f64 {
    match id {
        "fra" | "ita" => 1.0,
        "esp" | "che" => 0.85,
        "usa" => 0.8,
        "mex" => 0.7,
        "gre" => 0.55,
        "gbr" | "ger" => 0.5,
        "cub" | "bwi" | "dom" | "hai" | "ant" | "pan" => 0.55,
        "por" | "bel" | "nld" | "lux" | "mco" | "mlt" | "smr" | "lie" | "and" | "vat" => 0.45,
        "swe" | "nor" | "den" | "can" | "jap" | "egy" | "leb" | "isr" | "pal" => 0.3,
        "sov" | "pol" | "cze" | "hun" | "rom" | "bul" | "alb" | "yug" | "chi" | "ccp" | "mon" | "nkg" | "kor" => ATRATIVIDADE_CORTINA,
        _ => ATRATIVIDADE_PADRAO,
    }
}

/// Países do bloco oriental/fechados: atratividade mínima.
fn atratividade(id: &str) -> f64 {
    if id == "aut" { 0.75 } else { atratividade_base(id) }
}

/// Estado do turismo no mundo (valores em moeda por dia).
#[derive(Debug, Clone, Default)]
pub struct EstadoTurismo {
    /// Risco externo (0..1) fixado por outros sistemas (guerra, instabilidade): corta turistas.
    pub risco_externo: BTreeMap<String, f64>,
    /// Gasto de turistas estrangeiros recebido por país (exportação de serviços), por dia.
    pub receita_receptiva: Vec<f64>,
    /// Gasto dos cidadãos do país em viagens ao exterior, por dia.
    pub gasto_emissivo: Vec<f64>,
    /// Fator de segurança 0..1 (1 = sem crise) usado no último recálculo.
    pub fator_seguranca: Vec<f64>,
    ultimo_calculo: Option<u64>,
}

impl MundoEconomico {
    /// Define o risco turístico (0 = nenhum, 1 = guerra total) de um país, vindo de guerra/eventos.
    pub fn definir_risco_turismo(&mut self, pais_id: &str, risco: f64) {
        self.turismo.risco_externo.insert(pais_id.to_string(), risco.clamp(0.0, 1.0));
        self.turismo.ultimo_calculo = None;
    }

    /// Receita diária de turismo receptivo (gasto de estrangeiros) do país.
    pub fn turismo_receita_pais(&self, pais_id: &str) -> f64 {
        self.indice_paises.get(pais_id).and_then(|&i| self.turismo.receita_receptiva.get(i)).copied().unwrap_or(0.0)
    }

    /// Gasto diário dos cidadãos do país em turismo no exterior.
    pub fn turismo_gasto_exterior_pais(&self, pais_id: &str) -> f64 {
        self.indice_paises.get(pais_id).and_then(|&i| self.turismo.gasto_emissivo.get(i)).copied().unwrap_or(0.0)
    }

    /// Fator de segurança turística (0..1) do país no último cálculo.
    pub fn turismo_fator_seguranca_pais(&self, pais_id: &str) -> f64 {
        self.indice_paises.get(pais_id).and_then(|&i| self.turismo.fator_seguranca.get(i)).copied().unwrap_or(1.0)
    }

    fn recalcular_turismo(&mut self) {
        let n = self.paises.len();
        let ids: Vec<String> = self.paises.iter().map(|p| p.id.clone()).collect();
        let pops: Vec<f64> = ids.iter().map(|id| self.populacao_total_pais(id).unwrap_or(0.0)).collect();
        let pib_mundial: f64 = self.paises.iter().map(|p| p.pib_diario).sum();
        let pop_mundial: f64 = pops.iter().sum();
        let pib_pc_mundial = if pop_mundial > 0.0 { pib_mundial / pop_mundial } else { 0.0 };

        let mut seguranca = vec![1.0; n];
        let mut emissao = vec![0.0; n];
        let mut peso = vec![0.0; n];
        for i in 0..n {
            let id = ids[i].as_str();
            let mut s = 1.0 - self.turismo.risco_externo.get(id).copied().unwrap_or(0.0);
            if self.paises[i].orcamento.em_calote {
                s *= 0.6;
            }
            let desemp = self.taxa_desemprego_media_pais(id).unwrap_or(0.0);
            if desemp > 0.15 {
                s *= (1.0 - (desemp - 0.15) * 2.0).clamp(0.5, 1.0);
            }
            seguranca[i] = s.clamp(0.0, 1.0);
            // renda das classes média/alta: proporcional ao PIB per capita relativo (limitado)
            let rel = if pib_pc_mundial > 0.0 && pops[i] > 0.0 { (self.paises[i].pib_diario / pops[i]) / pib_pc_mundial } else { 0.0 };
            emissao[i] = FRACAO_PIB_EMISSAO * self.paises[i].pib_diario * rel.clamp(0.1, 3.0) / 1.0 * seguranca_emissao(s);
            peso[i] = atratividade(id) * seguranca[i] * pops[i].max(1.0).powf(0.15);
        }
        let total_emissao: f64 = emissao.iter().sum();
        let total_peso: f64 = peso.iter().sum();
        let mut receptiva = vec![0.0; n];
        if total_peso > 0.0 {
            for i in 0..n {
                receptiva[i] = total_emissao * peso[i] / total_peso;
            }
        }
        self.turismo.receita_receptiva = receptiva;
        self.turismo.gasto_emissivo = emissao;
        self.turismo.fator_seguranca = seguranca;
        self.turismo.ultimo_calculo = Some(self.dias_decorridos);
    }

    /// Aplica o turismo do dia: exportação de serviços no PIB e imposto na conta de receitas.
    /// Recalcula mensalmente. O imposto NÃO credita o tesouro aqui: quem consolida receita no
    /// tesouro é o fechamento fiscal (`receita_total` já inclui `receita_turismo`); creditar nas
    /// duas contas dobrava a arrecadação do turismo todos os dias.
    pub(super) fn aplicar_turismo_diario(&mut self) {
        let precisa = match self.turismo.ultimo_calculo {
            None => true,
            Some(d) => self.dias_decorridos.saturating_sub(d) >= 30,
        };
        if precisa {
            self.recalcular_turismo();
        }
        for i in 0..self.paises.len() {
            let gasto = self.turismo.receita_receptiva[i];
            let p = &mut self.paises[i];
            p.pib_diario += gasto;
            p.orcamento.receita_turismo = gasto * IMPOSTO_TURISMO;
        }
    }
}

/// Turistas de um país em crise viajam menos (a emissão acompanha a segurança interna de forma suave).
fn seguranca_emissao(s: f64) -> f64 {
    0.5 + 0.5 * s.clamp(0.0, 1.0)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn atratividade_segue_a_historia() {
        assert!(atratividade("fra") > atratividade("gbr"));
        assert!(atratividade("ita") > atratividade("jap"));
        assert!(atratividade("sov") < 0.05);
        assert!(atratividade("pol") < atratividade("gre"));
        assert!(atratividade("aut") > 0.5 && atratividade("mex") > 0.5);
    }
}
