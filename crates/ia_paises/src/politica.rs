//! IA politica: propoe leis conforme a ideologia dos grupos de interesse (IA.md 3.2).

use sim_politica::MundoPolitico;
use sim_politica::tipos::PapelGrupoInteresse;
use std::collections::BTreeMap;

/// Intervalo (dias) entre avaliacoes de um mesmo pais.
const PERIODO: u64 = 30;
/// Espera minima entre duas propostas do mesmo pais.
const COOLDOWN_PROPOSTA: u64 = 180;
const LEGITIMIDADE_MINIMA: f64 = 50.0;

#[derive(Debug, Clone, Default)]
pub struct IaPolitica {
    ultima_proposta: BTreeMap<String, u64>,
    /// Propostas feitas (total, para estatisticas/testes).
    pub propostas: u64,
}

impl IaPolitica {
    pub fn novo() -> Self {
        Self::default()
    }

    /// Chamar uma vez por dia. `ignorar` = ids de paises controlados por jogadores.
    pub fn passo_diario(&mut self, mundo: &mut MundoPolitico, ignorar: &[String]) {
        let dia = mundo.dias_decorridos();
        let ids: Vec<String> = mundo.paises().iter().map(|p| p.id.clone()).collect();
        for (i, id) in ids.iter().enumerate() {
            if (dia + i as u64) % PERIODO != 0 || ignorar.iter().any(|x| x == id) {
                continue;
            }
            if self.ultima_proposta.get(id).is_some_and(|&u| dia < u + COOLDOWN_PROPOSTA) {
                continue;
            }
            if let Some((cat, lei)) = escolher_lei(mundo, id)
                && let Some(p) = mundo.pais_mut(id)
                && p.iniciar_tramitacao(&cat, &lei).is_ok()
            {
                self.ultima_proposta.insert(id.clone(), dia);
                self.propostas += 1;
            }
        }
    }
}

/// Escolhe a melhor reforma `(categoria, lei)` para o pais, ou `None`.
pub fn escolher_lei(mundo: &MundoPolitico, id: &str) -> Option<(String, String)> {
    let pais = mundo.pais(id)?;
    if pais.lei_em_tramitacao.is_some() || pais.legitimidade < LEGITIMIDADE_MINIMA {
        return None;
    }
    let cat = mundo.catalogo();
    let ativas: Vec<&String> = pais.leis_ativas.values().collect();
    let mut melhor: Option<(f64, String, String)> = None;
    for (cid, c) in &cat.categorias_leis {
        let atual = pais.leis_ativas.get(cid).cloned().unwrap_or_else(|| "nenhuma".to_string());
        for l in &c.leis {
            if l.id == atual {
                continue;
            }
            let pr = &l.pre_requisitos;
            if pr.leis_obrigatorias.iter().any(|o| !ativas.contains(&o)) || pr.leis_proibidas.iter().any(|o| ativas.contains(&o)) {
                continue;
            }
            let mut apoio = 0.0;
            let mut oposicao = 0.0;
            let mut risco = false;
            for (gid, gi) in &pais.grupos_interesse {
                let v = gi.avaliar_postura_mudanca_lei(cat, &l.id, &atual).valor() as f64;
                // Quem esta no governo pesa mais na decisao do gabinete.
                let peso = if gi.papel == PapelGrupoInteresse::NoGoverno { 1.5 } else { 1.0 };
                if v > 0.0 {
                    apoio += gi.clout * peso * v;
                } else if v < 0.0 {
                    oposicao += gi.clout * peso * -v;
                    // Trava anti-golpe: grupo poderoso ou militares ja descontentes nao sao provocados.
                    if (gi.clout >= 0.30 || gid.contains("militar")) && gi.aprovacao - 2.0 * -v <= -10.0 {
                        risco = true;
                    }
                }
            }
            if risco {
                continue;
            }
            let score = apoio - 1.2 * oposicao;
            if score > 0.10 && melhor.as_ref().is_none_or(|(s, ..)| score > *s) {
                melhor = Some((score, cid.clone(), l.id.clone()));
            }
        }
    }
    melhor.map(|(_, c, l)| (c, l))
}
