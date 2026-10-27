//! Controle territorial, ocupação militar, resistência civil e condições de capitulação.

use crate::entidades::{ConfigGuerra, Guerra, PaisMilitar, RegiaoMapa};
use std::collections::HashMap;

/// Estatísticas de ocupação territorial e pontos de vitória de um país.
#[derive(Debug, Clone, Copy, Default)]
pub struct StatusTerritorialPais {
    pub total_pontos_vitoria: f64,
    pub pontos_controlados: f64,
    pub capital_controlada: bool,
    pub fracao_ocupada: f64,
    pub capitulou: bool,
}

/// Atualiza o estado de ocupação, calcula pontos de vitória e avalia capitulações.
pub fn atualizar_ocupacao_e_capitulacoes(
    regioes: &mut [RegiaoMapa],
    paises: &[PaisMilitar],
    guerras: &mut [Guerra],
    config: &ConfigGuerra,
    mapa_regioes: &HashMap<String, usize>,
) -> HashMap<String, StatusTerritorialPais> {
    let mut status_map: HashMap<String, StatusTerritorialPais> =
        HashMap::with_capacity(paises.len());

    // 1. Inicializa agregadores de pontos de vitória para cada país
    for pais in paises {
        let cap_ctrl = if let Some(&cap_idx) = mapa_regioes.get(&pais.capital_adm1) {
            regioes[cap_idx].controlador_tag == pais.tag
        } else {
            false
        };

        status_map.insert(
            pais.tag.clone(),
            StatusTerritorialPais {
                total_pontos_vitoria: 0.0,
                pontos_controlados: 0.0,
                capital_controlada: cap_ctrl,
                fracao_ocupada: 0.0,
                capitulou: false,
            },
        );
    }

    // 2. Soma pontos de vitória legítimos e sob controle
    for reg in regioes.iter_mut() {
        if let Some(st) = status_map.get_mut(&reg.dono_tag) {
            st.total_pontos_vitoria += reg.pontos_vitoria;
            if reg.controlador_tag == reg.dono_tag {
                st.pontos_controlados += reg.pontos_vitoria;
                // Em território soberano, resistência decai
                reg.resistencia_ocupacao = (reg.resistencia_ocupacao - 0.02).max(0.0);
            } else {
                // Em território ocupado, resistência cresce lentamente
                reg.resistencia_ocupacao = (reg.resistencia_ocupacao + 0.005).min(1.0);
            }
        }

        // Recuperação natural lenta de destruição em tempos de paz local
        if !reg.esta_ocupada() && reg.destruicao_guerra > 0.0 {
            reg.destruicao_guerra = (reg.destruicao_guerra - 0.0008).max(0.0);
        }
    }

    // 3. Calcula frações e verifica capitulação
    for pais in paises {
        if let Some(st) = status_map.get_mut(&pais.tag) {
            if st.total_pontos_vitoria > 0.0 {
                let perdidos = st.total_pontos_vitoria - st.pontos_controlados;
                st.fracao_ocupada = (perdidos / st.total_pontos_vitoria).clamp(0.0, 1.0);
            } else {
                st.fracao_ocupada = 0.0;
            }

            // Capitula se perdeu a capital e atingiu o limiar de perda territorial
            if !st.capital_controlada && st.fracao_ocupada >= config.limiar_capitulacao_vitoria {
                st.capitulou = true;
            }
        }
    }

    // 4. Propaga capitulação para as guerras ativas
    for guerra in guerras.iter_mut().filter(|g| g.ativa) {
        let mut atacantes_capitularam = true;
        for at_tag in &guerra.atacantes {
            if let Some(st) = status_map.get(at_tag)
                && !st.capitulou
            {
                atacantes_capitularam = false;
                break;
            }
        }

        let mut defensores_capitularam = true;
        for def_tag in &guerra.defensores {
            if let Some(st) = status_map.get(def_tag)
                && !st.capitulou
            {
                defensores_capitularam = false;
                break;
            }
        }

        if atacantes_capitularam && !guerra.atacantes.is_empty() {
            guerra.capitulou_atacantes = true;
        }
        if defensores_capitularam && !guerra.defensores.is_empty() {
            guerra.capitulou_defensores = true;
        }
    }

    status_map
}
