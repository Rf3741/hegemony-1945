//! Pontuação de guerra (-100 a +100) e progressão do cansaço de guerra (0 a 100).

use crate::entidades::{Guerra, PaisMilitar};
use crate::motor::ocupacao::StatusTerritorialPais;
use std::collections::HashMap;

/// Atualiza a pontuação de todas as guerras ativas e o cansaço dos países.
pub fn atualizar_pontuacao_e_cansaco_mundo(
    guerras: &mut [Guerra],
    paises: &mut [PaisMilitar],
    status_territorial: &HashMap<String, StatusTerritorialPais>,
) {
    // 1. Atualiza pontuação de guerra para cada guerra ativa
    for guerra in guerras.iter_mut().filter(|g| g.ativa) {
        if guerra.capitulou_defensores {
            guerra.pontuacao_guerra = 100.0;
            continue;
        }
        if guerra.capitulou_atacantes {
            guerra.pontuacao_guerra = -100.0;
            continue;
        }

        // Componente 1: Ocupação territorial dos defensores pelos atacantes
        let mut frac_ocup_defensor = 0.0;
        let mut n_def = 0;
        for def_tag in &guerra.defensores {
            if let Some(st) = status_territorial.get(def_tag) {
                frac_ocup_defensor += st.fracao_ocupada;
                n_def += 1;
            }
        }
        let media_ocup_def = if n_def > 0 {
            frac_ocup_defensor / n_def as f64
        } else {
            0.0
        };

        // Ocupação territorial dos atacantes pelos defensores
        let mut frac_ocup_atacante = 0.0;
        let mut n_atk = 0;
        for atk_tag in &guerra.atacantes {
            if let Some(st) = status_territorial.get(atk_tag) {
                frac_ocup_atacante += st.fracao_ocupada;
                n_atk += 1;
            }
        }
        let media_ocup_atk = if n_atk > 0 {
            frac_ocup_atacante / n_atk as f64
        } else {
            0.0
        };

        let pts_ocupacao = (media_ocup_def - media_ocup_atk) * 60.0;

        // Componente 2: Saldo de vitórias em batalhas
        let tot_bat =
            (guerra.batalhas_vencidas_atacantes + guerra.batalhas_vencidas_defensores) as f64;
        let pts_batalhas = if tot_bat > 0.0 {
            let saldo = guerra.batalhas_vencidas_atacantes as f64
                - guerra.batalhas_vencidas_defensores as f64;
            (saldo / tot_bat) * 25.0
        } else {
            0.0
        };

        // Componente 3: Relação de baixas
        let tot_baixas = (guerra.baixas_atacantes + guerra.baixas_defensores) as f64;
        let pts_baixas = if tot_baixas > 0.0 {
            let saldo_baixas = guerra.baixas_defensores as f64 - guerra.baixas_atacantes as f64;
            (saldo_baixas / tot_baixas) * 15.0
        } else {
            0.0
        };

        guerra.pontuacao_guerra = (pts_ocupacao + pts_batalhas + pts_baixas).clamp(-100.0, 100.0);
    }

    // 2. Atualiza cansaço de guerra acumulado para cada país
    for pais in paises.iter_mut() {
        let tag = &pais.tag;
        let em_guerra = guerras.iter().any(|g| g.ativa && g.participa(tag));

        if em_guerra {
            let mut delta_cansaco = 0.12; // Desgaste temporal contínuo da mobilização bélica

            if let Some(st) = status_territorial.get(tag) {
                // Penalidade severa se a capital nacional está ocupada
                if !st.capital_controlada {
                    delta_cansaco += 0.25;
                }
                // Penalidade proporcional ao território ocupado
                delta_cansaco += st.fracao_ocupada * 0.20;
            }

            // Desgaste de baixas relativas à população mobilizável
            let pop_mob = pais.populacao_mobilizavel.max(10_000) as f64;
            let proporcao_baixas = (pais.baixas_totais as f64 / pop_mob).clamp(0.0, 1.0);
            delta_cansaco += proporcao_baixas * 0.15;

            pais.cansaco_guerra = (pais.cansaco_guerra + delta_cansaco).min(100.0);
        } else {
            // Em tempo de paz, o cansaço dissipa naturalmente
            pais.cansaco_guerra = (pais.cansaco_guerra - 0.20).max(0.0);
        }
    }
}
