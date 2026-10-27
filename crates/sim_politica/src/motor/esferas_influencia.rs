//! Execução das operações de influência externa, financiamento de GIs e propaganda da Guerra Fria.

use crate::entidades::influencia::OperacaoInfluencia;
use crate::entidades::pais::PaisPolitica;
use crate::tipos::PotenciaBloco;

/// Executa os efeitos diários das operações de inteligência e financiamento estrangeiro.
pub fn processar_esferas_e_influencia_externa(pais: &mut PaisPolitica) {
    // 1. Processa operações americanas
    let ops_eua = pais.influencia_externa.operacoes_eua.clone();
    for op in ops_eua {
        match op {
            OperacaoInfluencia::FinanciarGrupoInteresse {
                gi_id,
                montante_diario,
            } => {
                if let Some(gi) = pais.grupos_interesse.get_mut(&gi_id) {
                    gi.bonus_financiamento_externo =
                        (gi.bonus_financiamento_externo + montante_diario * 0.05).min(50.0);
                }
                pais.influencia_externa
                    .aplicar_cabo_de_guerra(PotenciaBloco::Eua, montante_diario * 0.02);
            }
            OperacaoInfluencia::FinanciarPartido {
                partido_id,
                montante,
            } => {
                if let Some(part) = pais.partidos.get_mut(&partido_id) {
                    part.bonus_financiamento_externo =
                        (part.bonus_financiamento_externo + montante * 0.05).min(40.0);
                }
                pais.influencia_externa
                    .aplicar_cabo_de_guerra(PotenciaBloco::Eua, montante * 0.01);
            }
            OperacaoInfluencia::PropagandaIdeologica {
                ideologia_alvo: _,
                intensidade,
            } => {
                pais.influencia_externa
                    .aplicar_cabo_de_guerra(PotenciaBloco::Eua, intensidade * 0.03);
            }
            OperacaoInfluencia::AjudaEconomicaCondicionada {
                programa: _,
                montante_diario,
            } => {
                for pop in &mut pais.pops {
                    pop.padrao_de_vida = (pop.padrao_de_vida + montante_diario * 0.001).min(99.0);
                }
                pais.influencia_externa
                    .aplicar_cabo_de_guerra(PotenciaBloco::Eua, montante_diario * 0.03);
            }
        }
    }

    // 2. Processa operações soviéticas
    let ops_urss = pais.influencia_externa.operacoes_urss.clone();
    for op in ops_urss {
        match op {
            OperacaoInfluencia::FinanciarGrupoInteresse {
                gi_id,
                montante_diario,
            } => {
                if let Some(gi) = pais.grupos_interesse.get_mut(&gi_id) {
                    gi.bonus_financiamento_externo =
                        (gi.bonus_financiamento_externo + montante_diario * 0.05).min(50.0);
                }
                pais.influencia_externa
                    .aplicar_cabo_de_guerra(PotenciaBloco::Urss, montante_diario * 0.02);
            }
            OperacaoInfluencia::FinanciarPartido {
                partido_id,
                montante,
            } => {
                if let Some(part) = pais.partidos.get_mut(&partido_id) {
                    part.bonus_financiamento_externo =
                        (part.bonus_financiamento_externo + montante * 0.05).min(40.0);
                }
                pais.influencia_externa
                    .aplicar_cabo_de_guerra(PotenciaBloco::Urss, montante * 0.01);
            }
            OperacaoInfluencia::PropagandaIdeologica {
                ideologia_alvo: _,
                intensidade,
            } => {
                pais.influencia_externa
                    .aplicar_cabo_de_guerra(PotenciaBloco::Urss, intensidade * 0.03);
            }
            OperacaoInfluencia::AjudaEconomicaCondicionada {
                programa: _,
                montante_diario,
            } => {
                for pop in &mut pais.pops {
                    pop.padrao_de_vida = (pop.padrao_de_vida + montante_diario * 0.001).min(99.0);
                }
                pais.influencia_externa
                    .aplicar_cabo_de_guerra(PotenciaBloco::Urss, montante_diario * 0.03);
            }
        }
    }

    // Sincroniza os patamares de esfera formal
    pais.influencia_externa.sincronizar_niveis();
}
