//! Motor de tramitação legislativa diária, checkpoints, chances de avanço,
//! debate parlamentar e bloqueio por entraves (3 entraves = cancelamento).

use crate::catalogo::CatalogoPolitica;
use crate::entidades::pais::PaisPolitica;
use crate::rng::RngDeterministico;
use crate::tipos::{FaseLegislativa, ResultadoCheckpointLei};

/// Avança o processo legislativo da lei em tramitação a cada tick diário.
pub fn processar_tramitacao_legislativa_diaria(
    pais: &mut PaisPolitica,
    catalogo: &CatalogoPolitica,
    rng: &mut RngDeterministico,
) {
    let Some(tram) = &mut pais.lei_em_tramitacao else {
        return;
    };

    tram.dias_na_fase += 1;

    // Se o governo for ilegítimo (< 25 pontos), ocorre paralisia legislativa total
    if !pais.faixa_legitimidade.permite_tramitacao_leis() {
        return;
    }

    // Progresso diário na barra da fase atual
    let velocidade = 0.55 * pais.faixa_legitimidade.multiplicador_velocidade_leis();
    tram.progresso_fase = (tram.progresso_fase + velocidade).min(100.0);

    if tram.dias_restantes_checkpoint > 0 {
        tram.dias_restantes_checkpoint -= 1;
    }

    // Chegada a um Checkpoint Legislativo de deliberação
    if tram.dias_restantes_checkpoint == 0 || tram.progresso_fase >= 100.0 {
        executar_checkpoint_legislativo(pais, catalogo, rng);
    }
}

/// Executa a deliberação formal de um checkpoint legislativo com suporte determinístico.
fn executar_checkpoint_legislativo(
    pais: &mut PaisPolitica,
    catalogo: &CatalogoPolitica,
    rng: &mut RngDeterministico,
) {
    let Some(tram) = &mut pais.lei_em_tramitacao else {
        return;
    };

    // 1. Calcula o clout somado de apoiadores e opositores
    let mut clout_apoio = 0.0;
    let mut clout_oposicao = 0.0;
    tram.apoiadores_ativos.clear();
    tram.opositores_ativos.clear();

    for (gi_id, gi) in &pais.grupos_interesse {
        let postura =
            gi.avaliar_postura_mudanca_lei(catalogo, &tram.lei_alvo_id, &tram.lei_anterior_id);
        if postura.eh_favoravel() {
            clout_apoio += gi.clout;
            tram.apoiadores_ativos.push(gi_id.clone());
        } else if postura.eh_contraria() {
            clout_oposicao += gi.clout;
            tram.opositores_ativos.push(gi_id.clone());
        }
    }

    // 2. Modulação pela Legitimidade Governamental
    let bonus_legitimidade = (pais.legitimidade - 50.0) / 200.0;
    let penalidade_ilegitimidade = if pais.legitimidade < 50.0 {
        (50.0 - pais.legitimidade) / 150.0
    } else {
        0.0
    };

    // 3. Probabilidades brutas
    let chance_avanco = (clout_apoio * 0.65 + bonus_legitimidade + tram.modificador_sucesso_debate)
        .clamp(0.05, 0.85);
    let chance_travar = (clout_oposicao * 0.50 + penalidade_ilegitimidade).clamp(0.05, 0.75);
    let chance_debate = (1.0 - chance_avanco - chance_travar).max(0.05);

    // Normaliza para soma = 1.0
    let soma = chance_avanco + chance_debate + chance_travar;
    let p_avanco = chance_avanco / soma;
    let p_debate = chance_debate / soma;
    let p_travar = chance_travar / soma;

    tram.chance_avanco = p_avanco;
    tram.chance_debate = p_debate;
    tram.chance_travar = p_travar;

    // 4. Sorteio determinístico
    let r = rng.proximo_f64();
    let resultado = if r < p_avanco {
        ResultadoCheckpointLei::Avanco
    } else if r < p_avanco + p_debate {
        ResultadoCheckpointLei::Debate
    } else {
        ResultadoCheckpointLei::Travar
    };

    // 5. Aplicação do resultado
    match resultado {
        ResultadoCheckpointLei::Avanco => {
            tram.progresso_fase = 0.0;
            tram.dias_na_fase = 0;

            match tram.fase {
                FaseLegislativa::Introducao => {
                    tram.fase = FaseLegislativa::Comite;
                }
                FaseLegislativa::Comite => {
                    tram.fase = FaseLegislativa::VotacaoFinal;
                }
                FaseLegislativa::VotacaoFinal => {
                    // Lei APROVADA e promulgada!
                    let cat = tram.categoria.clone();
                    let nova_lei = tram.lei_alvo_id.clone();
                    pais.promulgar_lei_instantanea(&cat, &nova_lei);
                    pais.lei_em_tramitacao = None;
                    return;
                }
            }
        }
        ResultadoCheckpointLei::Debate => {
            // Debate parlamentar: pequenas emendas aumentam a probabilidade de aprovação futura
            tram.modificador_sucesso_debate += 0.08;
            tram.progresso_fase = 40.0;
        }
        ResultadoCheckpointLei::Travar => {
            tram.contagem_entraves += 1;
            tram.modificador_sucesso_debate = (tram.modificador_sucesso_debate - 0.05).max(-0.25);
            tram.progresso_fase = 10.0;

            // Ao atingir 3 entraves, o projeto fracassa e é arquivado
            if tram.contagem_entraves >= 3 {
                pais.lei_em_tramitacao = None;
                return;
            }
        }
        ResultadoCheckpointLei::Recuo => {
            if tram.fase == FaseLegislativa::Comite {
                tram.fase = FaseLegislativa::Introducao;
            }
            tram.progresso_fase = 0.0;
        }
    }

    // Reinicia o contador de dias até o próximo checkpoint
    let base_dias = 180.0 * (1.0 - (pais.legitimidade - 50.0) / 150.0);
    if let Some(t) = &mut pais.lei_em_tramitacao {
        t.dias_restantes_checkpoint = (base_dias.round() as u32).clamp(60, 240);
    }
}
