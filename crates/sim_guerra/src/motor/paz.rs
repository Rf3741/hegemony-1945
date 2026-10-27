//! API de tratados de paz, validação de cláusulas, avaliação de aceitação e desfecho de guerras.

use crate::entidades::{ConfigGuerra, Frente, Guerra, PaisMilitar, RegiaoMapa};
use crate::erro::{ErroGuerra, ResultadoGuerra};
use crate::tipos::{ClausulaPaz, DataJogo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Proposta diplomática de tratado de paz enviada por um dos beligerantes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropostaPaz {
    pub guerra_id: String,
    pub proponente_tag: String,
    pub clausulas: Vec<ClausulaPaz>,
}

/// Tratado de paz formalmente assinado e arquivado no histórico do jogo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TratadoPazAssinado {
    pub guerra_id: String,
    pub data_assinatura: DataJogo,
    pub vencedores: Vec<String>,
    pub derrotados: Vec<String>,
    pub clausulas_executadas: Vec<ClausulaPaz>,
}

/// Parecer diplomático da avaliação de uma proposta de paz.
#[derive(Debug, Clone, PartialEq)]
pub struct AvaliacaoPaz {
    pub aceita: bool,
    pub pontuacao_necessaria: f64,
    pub pontuacao_disponivel: f64,
    pub motivo_pt: String,
}

/// Calcula o custo em pontos de guerra de uma cláusula individual.
pub fn custo_pontuacao_clausula(
    clausula: &ClausulaPaz,
    mapa_regioes: &HashMap<String, usize>,
    regioes: &[RegiaoMapa],
) -> f64 {
    match clausula {
        ClausulaPaz::PazBranca => 0.0,
        ClausulaPaz::AnexarRegioes { regioes_adm1, .. } => {
            let mut custo = 5.0; // Custo base de quebra territorial
            for adm1 in regioes_adm1 {
                if let Some(&r_idx) = mapa_regioes.get(adm1) {
                    custo += (regioes[r_idx].pontos_vitoria * 2.5).max(3.0);
                }
            }
            custo
        }
        ClausulaPaz::MudancaRegime { .. } => 45.0,
        ClausulaPaz::EstadoSatelite { .. } => 65.0,
        ClausulaPaz::ZonaOcupacao { regioes_adm1, .. } => 10.0 + (regioes_adm1.len() as f64 * 3.0),
        ClausulaPaz::ReparacoesGuerra { fracao_receita, .. } => {
            (fracao_receita * 100.0).clamp(5.0, 30.0)
        }
        ClausulaPaz::Desmilitarizacao { .. } => 20.0,
    }
}

/// Avalia se os adversários aceitam uma proposta de paz com base na pontuação e cansaço.
pub fn avaliar_aceitacao_paz(
    guerra: &Guerra,
    paises: &[PaisMilitar],
    proposta: &PropostaPaz,
    regioes: &[RegiaoMapa],
    mapa_regioes: &HashMap<String, usize>,
) -> AvaliacaoPaz {
    let proponente_eh_atacante = guerra.eh_atacante(&proposta.proponente_tag);

    // Pontuação efetiva disponível para o proponente
    let pontuacao_disponivel = if proponente_eh_atacante {
        guerra.pontuacao_guerra
    } else {
        -guerra.pontuacao_guerra
    };

    // Capitulação total do oponente força aceitação incondicional
    let oponente_capitulou = if proponente_eh_atacante {
        guerra.capitulou_defensores
    } else {
        guerra.capitulou_atacantes
    };

    if oponente_capitulou {
        return AvaliacaoPaz {
            aceita: true,
            pontuacao_necessaria: 0.0,
            pontuacao_disponivel,
            motivo_pt: "Inimigo capitulou incondicionalmente".to_string(),
        };
    }

    let mut custo_total = 0.0;
    for c in &proposta.clausulas {
        custo_total += custo_pontuacao_clausula(c, mapa_regioes, regioes);
    }

    // Calcula cansaço médio dos adversários
    let tags_adversarias = if proponente_eh_atacante {
        &guerra.defensores
    } else {
        &guerra.atacantes
    };

    let mut soma_cansaco = 0.0;
    let mut n_adv = 0;
    for tag in tags_adversarias {
        if let Some(p) = paises.iter().find(|p| &p.tag == tag) {
            soma_cansaco += p.cansaco_guerra;
            n_adv += 1;
        }
    }
    let media_cansaco = if n_adv > 0 {
        soma_cansaco / n_adv as f64
    } else {
        0.0
    };

    // Cansaço elevado dos adversários reduz a pontuação mínima que exigem para ceder
    let desconto_cansaco = (media_cansaco * 0.40).clamp(0.0, 35.0);
    let pontuacao_exigida = (custo_total - desconto_cansaco).max(0.0);

    let aceita = pontuacao_disponivel >= pontuacao_exigida;
    let motivo_pt = if aceita {
        format!(
            "Proposta aceita (pontuação disponível {pontuacao_disponivel:.1} >= exigida {pontuacao_exigida:.1})"
        )
    } else {
        format!(
            "Proposta rejeitada (pontuação disponível {pontuacao_disponivel:.1} < exigida {pontuacao_exigida:.1})"
        )
    };

    AvaliacaoPaz {
        aceita,
        pontuacao_necessaria: pontuacao_exigida,
        pontuacao_disponivel,
        motivo_pt,
    }
}

/// Aplica formalmente um tratado de paz, transferindo territórios e encerrando a guerra.
pub fn aplicar_tratado_paz(
    guerra_idx: usize,
    guerras: &mut [Guerra],
    paises: &mut [PaisMilitar],
    regioes: &mut [RegiaoMapa],
    frentes: &mut [Frente],
    proposta: PropostaPaz,
    data_atual: DataJogo,
    mapa_regioes: &HashMap<String, usize>,
) -> ResultadoGuerra<TratadoPazAssinado> {
    if guerra_idx >= guerras.len() {
        return Err(ErroGuerra::ReferenciaInexistente {
            origem_entidade: "tratado_paz",
            origem_id: "proposta".to_string(),
            alvo_entidade: "guerra",
            alvo_id: proposta.guerra_id.clone(),
        });
    }

    let guerra = &mut guerras[guerra_idx];
    if !guerra.ativa {
        return Err(ErroGuerra::RegraDominioViolada {
            contexto: "aplicar_tratado_paz".to_string(),
            detalhe: "não é possível assinar tratado de paz em uma guerra já encerrada".to_string(),
        });
    }

    let proponente_eh_atk = guerra.eh_atacante(&proposta.proponente_tag);
    let vencedores = if proponente_eh_atk {
        guerra.atacantes.clone()
    } else {
        guerra.defensores.clone()
    };
    let derrotados = if proponente_eh_atk {
        guerra.defensores.clone()
    } else {
        guerra.atacantes.clone()
    };

    // Executa cada cláusula do tratado
    for clausula in &proposta.clausulas {
        match clausula {
            ClausulaPaz::PazBranca => {
                // Todas as regiões ocupadas pelos beligerantes voltam para o dono original
            }
            ClausulaPaz::AnexarRegioes {
                vencedor_tag,
                regioes_adm1,
                ..
            } => {
                for adm1 in regioes_adm1 {
                    if let Some(&r_idx) = mapa_regioes.get(adm1) {
                        regioes[r_idx].dono_tag = vencedor_tag.clone();
                        regioes[r_idx].controlador_tag = vencedor_tag.clone();
                        regioes[r_idx].resistencia_ocupacao = 0.0;
                    }
                }
            }
            ClausulaPaz::MudancaRegime {
                alvo_tag,
                nova_ideologia,
                ..
            } => {
                if let Some(pais) = paises.iter_mut().find(|p| &p.tag == alvo_tag) {
                    pais.doutrina_id = nova_ideologia.clone();
                }
            }
            ClausulaPaz::EstadoSatelite { .. } => {
                // Registra subordinação
            }
            ClausulaPaz::ZonaOcupacao {
                ocupante_tag,
                regioes_adm1,
                ..
            } => {
                for adm1 in regioes_adm1 {
                    if let Some(&r_idx) = mapa_regioes.get(adm1) {
                        regioes[r_idx].controlador_tag = ocupante_tag.clone();
                    }
                }
            }
            ClausulaPaz::ReparacoesGuerra { .. } => {}
            ClausulaPaz::Desmilitarizacao { .. } => {}
        }
    }

    // Devolve o controle das regiões ocupadas remanescentes que não foram anexadas
    for reg in regioes.iter_mut() {
        if (guerra.participa(&reg.dono_tag) || guerra.participa(&reg.controlador_tag))
            && reg.esta_ocupada()
        {
            reg.controlador_tag = reg.dono_tag.clone();
            reg.resistencia_ocupacao = 0.0;
        }
    }

    // Encerra a guerra e desativa frentes
    guerra.ativa = false;
    for frente in frentes.iter_mut().filter(|f| f.guerra_id == guerra.id) {
        frente.ativa = false;
    }

    Ok(TratadoPazAssinado {
        guerra_id: guerra.id.clone(),
        data_assinatura: data_atual,
        vencedores,
        derrotados,
        clausulas_executadas: proposta.clausulas,
    })
}

/// Avalia periodicamente se alguma guerra atingiu condição automática de término
/// (capitulação completa de um lado ou exaustão mútua extrema).
pub fn verificar_fim_guerras_automatico(
    guerras: &mut [Guerra],
    paises: &mut [PaisMilitar],
    regioes: &mut [RegiaoMapa],
    frentes: &mut [Frente],
    config: &ConfigGuerra,
    data_atual: DataJogo,
    mapa_regioes: &HashMap<String, usize>,
) -> Vec<TratadoPazAssinado> {
    if !config.resolver_paz_automatica {
        return Vec::new();
    }

    let mut tratados = Vec::new();

    for g_idx in 0..guerras.len() {
        if !guerras[g_idx].ativa {
            continue;
        }

        let mut proposta_auto = None;

        // Caso 1: Defensores capitularam
        if guerras[g_idx].capitulou_defensores {
            let vencedor = guerras[g_idx].atacantes[0].clone();
            let perdedor = guerras[g_idx].defensores[0].clone();

            // Identifica regiões do perdedor atualmente ocupadas pelo vencedor
            let regioes_anexar: Vec<String> = regioes
                .iter()
                .filter(|r| r.dono_tag == perdedor && r.controlador_tag == vencedor)
                .map(|r| r.adm1_code.clone())
                .collect();

            let clausulas = if regioes_anexar.is_empty() {
                vec![ClausulaPaz::PazBranca]
            } else {
                vec![ClausulaPaz::AnexarRegioes {
                    vencedor_tag: vencedor.clone(),
                    perdedor_tag: perdedor,
                    regioes_adm1: regioes_anexar,
                }]
            };

            proposta_auto = Some(PropostaPaz {
                guerra_id: guerras[g_idx].id.clone(),
                proponente_tag: vencedor,
                clausulas,
            });
        }
        // Caso 2: Atacantes capitularam
        else if guerras[g_idx].capitulou_atacantes {
            let vencedor = guerras[g_idx].defensores[0].clone();
            let perdedor = guerras[g_idx].atacantes[0].clone();

            let regioes_anexar: Vec<String> = regioes
                .iter()
                .filter(|r| r.dono_tag == perdedor && r.controlador_tag == vencedor)
                .map(|r| r.adm1_code.clone())
                .collect();

            let clausulas = if regioes_anexar.is_empty() {
                vec![ClausulaPaz::PazBranca]
            } else {
                vec![ClausulaPaz::AnexarRegioes {
                    vencedor_tag: vencedor.clone(),
                    perdedor_tag: perdedor,
                    regioes_adm1: regioes_anexar,
                }]
            };

            proposta_auto = Some(PropostaPaz {
                guerra_id: guerras[g_idx].id.clone(),
                proponente_tag: vencedor,
                clausulas,
            });
        }
        // Caso 3: Cansaço mútuo extremo (armistício estilo Guerra da Coreia)
        else {
            let tag_atk = &guerras[g_idx].atacantes[0];
            let tag_def = &guerras[g_idx].defensores[0];

            let cansaco_atk = paises
                .iter()
                .find(|p| &p.tag == tag_atk)
                .map(|p| p.cansaco_guerra)
                .unwrap_or(0.0);
            let cansaco_def = paises
                .iter()
                .find(|p| &p.tag == tag_def)
                .map(|p| p.cansaco_guerra)
                .unwrap_or(0.0);

            let lim = config.limiar_cansaco_armisticio;
            let (maior, menor) = if cansaco_atk >= cansaco_def {
                (cansaco_atk, cansaco_def)
            } else {
                (cansaco_def, cansaco_atk)
            };
            // Armistício: um lado exausto e o outro já bastante desgastado (guerra estagnada)
            if maior >= lim && menor >= lim * 0.5 {
                proposta_auto = Some(PropostaPaz {
                    guerra_id: guerras[g_idx].id.clone(),
                    proponente_tag: tag_atk.clone(),
                    clausulas: vec![ClausulaPaz::PazBranca],
                });
            }
        }

        if let Some(prop) = proposta_auto
            && let Ok(tratado) = aplicar_tratado_paz(
                g_idx,
                guerras,
                paises,
                regioes,
                frentes,
                prop,
                data_atual,
                mapa_regioes,
            )
        {
            tratados.push(tratado);
        }
    }

    tratados
}
