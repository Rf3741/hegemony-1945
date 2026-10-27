//! Registro de eleições periódicas, campanhas e distribuição parlamentar de cadeiras.

use crate::tipos::DataJogo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Estado de uma campanha eleitoral em andamento antes do pleito.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampanhaEleitoral {
    /// Se a campanha está ativa nos ticks diários atuais.
    pub em_andamento: bool,
    /// Dias restantes até o dia da eleição (geralmente inicia em 90 ou 180 dias).
    pub dias_restantes: u32,
    /// Duração total original da campanha em ticks diários.
    pub duracao_total: u32,
}

impl Default for CampanhaEleitoral {
    fn default() -> Self {
        Self {
            em_andamento: false,
            dias_restantes: 0,
            duracao_total: 90,
        }
    }
}

/// Registro e estatísticas de uma eleição concluída.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultadoEleicao {
    /// Data em que as urnas foram apuradas.
    pub data: DataJogo,
    /// Soma de todos os votos populares computados.
    pub total_votos_validos: f64,
    /// Taxa de participação eleitoral (comparecimento às urnas de 0.0 a 1.0).
    pub taxa_participacao: f64,
    /// Votos obtidos por cada partido (`partido_id -> votos`).
    pub votos_por_partido: BTreeMap<String, f64>,
    /// Participação percentual de votos válidos (`partido_id -> 0.0..=1.0`).
    pub percentual_por_partido: BTreeMap<String, f64>,
    /// Cadeiras parlamentares conquistadas (`partido_id -> cadeiras`).
    pub cadeiras_por_partido: BTreeMap<String, u32>,
    /// Partido mais votado.
    pub partido_vencedor_id: Option<String>,
}

impl ResultadoEleicao {
    /// Calcula a distribuição de 100 cadeiras parlamentares pelo método D'Hondt.
    pub fn calcular_cadeiras_dhondt(
        total_cadeiras: u32,
        votos: &BTreeMap<String, f64>,
        clausula_barreira: f64,
    ) -> BTreeMap<String, u32> {
        let mut cadeiras: BTreeMap<String, u32> = BTreeMap::new();
        let total_votos: f64 = votos.values().copied().sum();
        if total_votos <= 0.0 || votos.is_empty() {
            return cadeiras;
        }

        // Filtra partidos que atingiram a cláusula de barreira (ex.: 3% ou 5%)
        let elegiveis: Vec<(&String, f64)> = votos
            .iter()
            .map(|(p, v)| (p, *v))
            .filter(|(_, v)| (*v / total_votos) >= clausula_barreira)
            .collect();

        if elegiveis.is_empty() {
            // Se nenhum atingiu a cláusula, todos participam
            for p in votos.keys() {
                cadeiras.insert(p.clone(), 0);
            }
        } else {
            for (p, _) in &elegiveis {
                cadeiras.insert((*p).clone(), 0);
            }
        }

        // Atribui cadeiras uma a uma pelo maior quociente D'Hondt: V / (s + 1)
        for _ in 0..total_cadeiras {
            let mut melhor_partido: Option<String> = None;
            let mut maior_quociente = -1.0;

            for (part_id, &voto) in votos {
                if let Some(&assentos_atuais) = cadeiras.get(part_id) {
                    let quociente = voto / (assentos_atuais as f64 + 1.0);
                    if quociente > maior_quociente {
                        maior_quociente = quociente;
                        melhor_partido = Some(part_id.clone());
                    }
                }
            }

            if let Some(vencedor) = melhor_partido {
                *cadeiras.entry(vencedor).or_insert(0) += 1;
            } else {
                break;
            }
        }

        cadeiras
    }
}
