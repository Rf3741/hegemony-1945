//! Resolução tática e operacional de combates, desgaste e avanço da frente região a região.
//!
//! Modela largura de combate, terreno, entrincheiramento, suprimento, moral, organização,
//! comandantes, tecnologias militares, perfuração vs blindagem, retirada e captura de províncias.

use crate::catalogo::CatalogoGuerra;
use crate::entidades::{
    Comandante, ConfigGuerra, Divisao, Exercito, Frente, Guerra, PaisMilitar, RegiaoMapa,
};
use crate::grafo::GrafoCompilado;
use crate::rng::RngDeterministico;
use std::collections::HashMap;

/// Registro de uma batalha resolvida em um determinado tick.
#[derive(Debug, Clone)]
pub struct RelatorioBatalha {
    pub regiao_alvo_adm1: String,
    pub atacante_tag: String,
    pub defensor_tag: String,
    pub baixas_atacante: u32,
    pub baixas_defensor: u32,
    pub dano_org_atacante: f64,
    pub dano_org_defensor: f64,
    pub vitoria_atacante: bool,
    pub regiao_conquistada: bool,
}

/// Executa a fase diária de combate em todas as frentes ativas.
pub fn resolver_combates_mundo(
    regioes: &mut [RegiaoMapa],
    grafo: &GrafoCompilado,
    catalogo: &CatalogoGuerra,
    paises: &mut [PaisMilitar],
    comandantes: &[Comandante],
    divisoes: &mut [Divisao],
    exercitos: &[Exercito],
    frentes: &[Frente],
    guerras: &mut [Guerra],
    config: &ConfigGuerra,
    rng: &mut RngDeterministico,
    mapa_regioes: &HashMap<String, usize>,
    mapa_paises: &HashMap<String, usize>,
    mapa_comandantes: &HashMap<String, usize>,
) -> Vec<RelatorioBatalha> {
    let mut relatorios = Vec::new();

    // Sem frentes ativas não há combate: evita montar os índices auxiliares (O(regiões + divisões)).
    if !frentes.iter().any(|f| f.ativa) {
        return relatorios;
    }

    // Mapeia cada divisão ao seu exército e comandante
    let mut div_para_exercito: Vec<Option<usize>> = vec![None; divisoes.len()];
    let indice_divs: HashMap<&str, usize> = divisoes
        .iter()
        .enumerate()
        .map(|(i, d)| (d.id.as_str(), i))
        .collect();
    for (e_idx, exe) in exercitos.iter().enumerate() {
        for d_id in &exe.divisoes_ids {
            if let Some(&d_idx) = indice_divs.get(d_id.as_str()) {
                div_para_exercito[d_idx] = Some(e_idx);
            }
        }
    }

    // Pré-computa mapa de divisões por região para consultas imediatas
    let n_regioes = regioes.len();
    let mut divisoes_por_regiao: Vec<Vec<usize>> = vec![Vec::new(); n_regioes];
    for (d_idx, div) in divisoes.iter().enumerate() {
        if div.efetivos_atuais > 0
            && let Some(&r_idx) = mapa_regioes.get(&div.regiao_adm1)
            && r_idx < n_regioes
        {
            divisoes_por_regiao[r_idx].push(d_idx);
        }
    }

    // Pré-calcula bônus tecnológicos por país
    let mut bonus_tec_pais: Vec<(f64, f64, f64, f64)> = vec![(0.0, 0.0, 0.0, 0.0); paises.len()];
    for (p_idx, pais) in paises.iter().enumerate() {
        let mut b_ataque = 0.0;
        let mut b_defesa = 0.0;
        let mut b_ruptura = 0.0;
        let mut b_org = 0.0;
        for tec_id in &pais.tecnologias_ativas {
            if let Some(&t_idx) = catalogo.mapa_tecnologias.get(tec_id) {
                let tec = &catalogo.tecnologias[t_idx];
                b_ataque += tec.bonus_ataque;
                b_defesa += tec.bonus_defesa;
                b_ruptura += tec.bonus_ruptura;
                b_org += tec.bonus_organizacao_max;
            }
        }
        bonus_tec_pais[p_idx] = (b_ataque, b_defesa, b_ruptura, b_org);
    }

    // Processa combates ao longo dos pares de fronteira das frentes ativas
    for frente in frentes.iter().filter(|f| f.ativa) {
        let guerra_idx = match guerras
            .iter()
            .position(|g| g.id == frente.guerra_id && g.ativa)
        {
            Some(idx) => idx,
            None => continue,
        };

        for (code_origem, code_alvo) in &frente.pares_fronteira {
            let u = match mapa_regioes.get(code_origem) {
                Some(&idx) => idx,
                None => continue,
            };
            let v = match mapa_regioes.get(code_alvo) {
                Some(&idx) => idx,
                None => continue,
            };

            let tag_atacante = regioes[u].controlador_tag.clone();
            let tag_defensor = regioes[v].controlador_tag.clone();

            if !guerras[guerra_idx].sao_inimigos(&tag_atacante, &tag_defensor) {
                continue;
            }

            // Divisões atacantes prontas na região u
            let atacantes_indices: Vec<usize> = divisoes_por_regiao[u]
                .iter()
                .copied()
                .filter(|&d_idx| {
                    let d = &divisoes[d_idx];
                    if d.pais_tag != tag_atacante || !d.pronta_para_combate() {
                        return false;
                    }
                    if let Some(e_idx) = div_para_exercito[d_idx] {
                        exercitos[e_idx].postura.autoriza_ataque()
                    } else {
                        false
                    }
                })
                .collect();

            if atacantes_indices.is_empty() {
                continue;
            }

            // Divisões defensoras na região v
            let defensores_indices: Vec<usize> = divisoes_por_regiao[v]
                .iter()
                .copied()
                .filter(|&d_idx| {
                    divisoes[d_idx].pais_tag == tag_defensor && divisoes[d_idx].efetivos_atuais > 0
                })
                .collect();

            // Parâmetros de terreno da região defensora
            let reg_v = &regioes[v];
            let t_idx = catalogo
                .mapa_terrenos
                .get(&reg_v.terreno_id)
                .copied()
                .unwrap_or(0);
            let terreno = &catalogo.terrenos[t_idx];
            let largura_maxima = terreno.largura_combate;

            let mod_terreno_atk = terreno.modificador_ataque;
            let mod_terreno_def = terreno.modificador_defesa;

            let aresta_uv = grafo.obter_aresta(u, v);
            let penalidade_rio = if aresta_uv.is_some_and(|a| a.travessia_rio) {
                0.80
            } else {
                1.00
            };
            let penalidade_estreito = if aresta_uv.is_some_and(|a| a.estreito_maritimo) {
                0.65
            } else {
                1.00
            };

            let fort_def = 1.0 + (reg_v.nivel_fortificacao as f64 * 0.08);

            // Se a região inimiga está vazia ou sem guarnição viva, avanço incruento imediato
            if defensores_indices.is_empty() {
                // Captura a região v
                let d_adv_idx = atacantes_indices[0];
                let dono_v = reg_v.dono_tag.clone();
                let eh_aliado_dono = guerras[guerra_idx].eh_atacante(&dono_v)
                    || (!guerras[guerra_idx].eh_defensor(&dono_v) && dono_v == tag_atacante);

                if eh_aliado_dono {
                    regioes[v].controlador_tag = dono_v;
                } else {
                    regioes[v].controlador_tag = tag_atacante.clone();
                }
                regioes[v].destruicao_guerra = (regioes[v].destruicao_guerra + 0.01).min(1.0);

                // Move divisão de ponta para a nova região
                divisoes[d_adv_idx].regiao_adm1 = code_alvo.clone();
                divisoes[d_adv_idx].entrincheiramento = 0.0;
                divisoes[d_adv_idx].organizacao = (divisoes[d_adv_idx].organizacao - 2.0).max(5.0);

                relatorios.push(RelatorioBatalha {
                    regiao_alvo_adm1: code_alvo.clone(),
                    atacante_tag: tag_atacante.clone(),
                    defensor_tag: tag_defensor.clone(),
                    baixas_atacante: 0,
                    baixas_defensor: 0,
                    dano_org_atacante: 2.0,
                    dano_org_defensor: 0.0,
                    vitoria_atacante: true,
                    regiao_conquistada: true,
                });
                continue;
            }

            // Seleciona divisões que cabem na largura de combate
            let mut atacantes_ativos = Vec::new();
            let mut largura_usada_atk = 0u32;
            for &d_idx in &atacantes_indices {
                let tmpl_idx = catalogo
                    .mapa_templates
                    .get(&divisoes[d_idx].template_id)
                    .copied()
                    .unwrap_or(0);
                let larg = catalogo.templates_compilados[tmpl_idx].largura_combate;
                if largura_usada_atk == 0 || largura_usada_atk + larg <= largura_maxima {
                    atacantes_ativos.push(d_idx);
                    largura_usada_atk += larg;
                }
            }

            let mut defensores_ativos = Vec::new();
            let mut largura_usada_def = 0u32;
            for &d_idx in &defensores_indices {
                let tmpl_idx = catalogo
                    .mapa_templates
                    .get(&divisoes[d_idx].template_id)
                    .copied()
                    .unwrap_or(0);
                let larg = catalogo.templates_compilados[tmpl_idx].largura_combate;
                if largura_usada_def == 0 || largura_usada_def + larg <= largura_maxima {
                    defensores_ativos.push(d_idx);
                    largura_usada_def += larg;
                }
            }

            // Bônus de comandante atacante
            let (com_atk_mod, _) = if let Some(e_idx) = div_para_exercito[atacantes_ativos[0]] {
                if let Some(ref c_id) = exercitos[e_idx].comandante_id {
                    if let Some(&c_idx) = mapa_comandantes.get(c_id) {
                        let c = &comandantes[c_idx];
                        (
                            1.0 + c.habilidade_ataque as f64 * 0.05,
                            c.habilidade_logistica,
                        )
                    } else {
                        (1.0, 1)
                    }
                } else {
                    (1.0, 1)
                }
            } else {
                (1.0, 1)
            };

            // Bônus de comandante defensor
            let (com_def_mod, _) = if let Some(e_idx) = div_para_exercito[defensores_ativos[0]] {
                if let Some(ref c_id) = exercitos[e_idx].comandante_id {
                    if let Some(&c_idx) = mapa_comandantes.get(c_id) {
                        let c = &comandantes[c_idx];
                        (
                            1.0 + c.habilidade_defesa as f64 * 0.05,
                            c.habilidade_logistica,
                        )
                    } else {
                        (1.0, 1)
                    }
                } else {
                    (1.0, 1)
                }
            } else {
                (1.0, 1)
            };

            let p_atk_idx = mapa_paises.get(&tag_atacante).copied().unwrap_or(0);
            let p_def_idx = mapa_paises.get(&tag_defensor).copied().unwrap_or(0);
            let (b_atk_tec, _, b_rup_tec, _) = bonus_tec_pais[p_atk_idx];
            let (_, b_def_tec, _, _) = bonus_tec_pais[p_def_idx];

            // Calcula poder de fogo e perfuração dos atacantes
            let mut soma_ataque_atk = 0.0;
            let mut max_perfuracao_atk: f64 = 0.0;
            let mut soma_ruptura_atk = 0.0;
            let mut _soma_dureza_atk = 0.0;

            for &d_idx in &atacantes_ativos {
                let d = &divisoes[d_idx];
                let tmpl_idx = catalogo
                    .mapa_templates
                    .get(&d.template_id)
                    .copied()
                    .unwrap_or(0);
                let st = &catalogo.templates_compilados[tmpl_idx];

                let mult_sup = (0.25 + 0.75 * d.suprimento_atual).clamp(0.25, 1.0);
                let mult_exp = 1.0 + (d.experiencia - 0.20) * 0.35;
                let mult_eq = d.equipamento_fracao.clamp(0.20, 1.0);
                let fator_geral = mult_sup * mult_exp * mult_eq;

                soma_ataque_atk += (st.ataque_leve + st.ataque_pesado * 0.5) * fator_geral;
                soma_ruptura_atk += st.ruptura * fator_geral;
                max_perfuracao_atk = max_perfuracao_atk.max(st.perfuracao);
                _soma_dureza_atk += st.dureza;
            }

            // Calcula poder de fogo e blindagem dos defensores
            let mut soma_ataque_def = 0.0;
            let mut soma_defesa_def = 0.0;
            let mut max_blindagem_def: f64 = 0.0;
            let mut max_perfuracao_def: f64 = 0.0;
            let mut max_blindagem_atk: f64 = 0.0;
            let mut media_entrincheiramento = 0.0;

            for &d_idx in &defensores_ativos {
                let d = &divisoes[d_idx];
                let tmpl_idx = catalogo
                    .mapa_templates
                    .get(&d.template_id)
                    .copied()
                    .unwrap_or(0);
                let st = &catalogo.templates_compilados[tmpl_idx];

                let mult_sup = (0.25 + 0.75 * d.suprimento_atual).clamp(0.25, 1.0);
                let mult_exp = 1.0 + (d.experiencia - 0.20) * 0.35;
                let mult_eq = d.equipamento_fracao.clamp(0.20, 1.0);
                let fator_geral = mult_sup * mult_exp * mult_eq;

                soma_ataque_def += (st.ataque_leve + st.ataque_pesado * 0.5) * fator_geral;
                soma_defesa_def += st.defesa * fator_geral;
                max_blindagem_def = max_blindagem_def.max(st.blindagem);
                max_perfuracao_def = max_perfuracao_def.max(st.perfuracao);
                media_entrincheiramento += d.entrincheiramento;
            }
            media_entrincheiramento /= defensores_ativos.len() as f64;

            for &d_idx in &atacantes_ativos {
                let tmpl_idx = catalogo
                    .mapa_templates
                    .get(&divisoes[d_idx].template_id)
                    .copied()
                    .unwrap_or(0);
                max_blindagem_atk =
                    max_blindagem_atk.max(catalogo.templates_compilados[tmpl_idx].blindagem);
            }

            // Modificador de blindagem não-perfurada
            let bonus_blindagem_def = if max_blindagem_def > max_perfuracao_atk {
                0.60
            } else {
                1.00
            };
            let bonus_blindagem_atk = if max_blindagem_atk > max_perfuracao_def {
                0.60
            } else {
                1.00
            };

            let bonus_entrinch_def = 1.0 + media_entrincheiramento * 0.025;
            let rng_mult = if config.variacao_tatica_aleatoria {
                rng.interval_f64(0.90, 1.10)
            } else {
                1.00
            };

            // Dano infligido pelo atacante ao defensor
            let poder_atk_final = soma_ataque_atk
                * (1.0 + b_atk_tec)
                * mod_terreno_atk
                * penalidade_rio
                * penalidade_estreito
                * com_atk_mod
                * rng_mult
                / (fort_def * bonus_entrinch_def);

            let defesa_def_final =
                soma_defesa_def * (1.0 + b_def_tec) * mod_terreno_def * com_def_mod;
            let dano_base_def = if poder_atk_final <= defesa_def_final {
                poder_atk_final * 0.05
            } else {
                (defesa_def_final * 0.05) + (poder_atk_final - defesa_def_final) * 0.20
            } * bonus_blindagem_def
                * config.fator_dano_combate;

            // Dano infligido pelo defensor ao atacante
            let poder_def_final = soma_ataque_def
                * (1.0 + b_def_tec)
                * mod_terreno_def
                * com_def_mod
                * fort_def
                * rng_mult;

            let ruptura_atk_final = soma_ruptura_atk * (1.0 + b_rup_tec) * com_atk_mod;
            let dano_base_atk = if poder_def_final <= ruptura_atk_final {
                poder_def_final * 0.05
            } else {
                (ruptura_atk_final * 0.05) + (poder_def_final - ruptura_atk_final) * 0.20
            } * bonus_blindagem_atk
                * config.fator_dano_combate;

            // Aplica danos e baixas nas divisões ativas
            let baixas_def = (dano_base_def * 12.0) as u32;
            let baixas_atk = (dano_base_atk * 14.0) as u32;

            let dano_org_def = (dano_base_def * 0.45).max(1.0);
            let dano_org_atk = (dano_base_atk * 0.40).max(1.0);

            let fatia_dano_def = dano_org_def / defensores_ativos.len() as f64;
            let fatia_baixas_def = baixas_def / defensores_ativos.len() as u32;

            for &d_idx in &defensores_ativos {
                let d = &mut divisoes[d_idx];
                d.efetivos_atuais = d.efetivos_atuais.saturating_sub(fatia_baixas_def);
                d.organizacao = (d.organizacao - fatia_dano_def).max(0.0);
                d.entrincheiramento = (d.entrincheiramento - 0.5).max(0.0);
                d.equipamento_fracao = (d.equipamento_fracao - 0.005).max(0.10);
            }

            let fatia_dano_atk = dano_org_atk / atacantes_ativos.len() as f64;
            let fatia_baixas_atk = baixas_atk / atacantes_ativos.len() as u32;

            for &d_idx in &atacantes_ativos {
                let d = &mut divisoes[d_idx];
                d.efetivos_atuais = d.efetivos_atuais.saturating_sub(fatia_baixas_atk);
                d.organizacao = (d.organizacao - fatia_dano_atk).max(0.0);
                d.entrincheiramento = 0.0;
                d.equipamento_fracao = (d.equipamento_fracao - 0.004).max(0.10);
            }

            // Atualiza estatísticas da guerra
            guerras[guerra_idx].baixas_atacantes += baixas_atk as u64;
            guerras[guerra_idx].baixas_defensores += baixas_def as u64;
            paises[p_atk_idx].baixas_totais += baixas_atk as u64;
            paises[p_def_idx].baixas_totais += baixas_def as u64;

            // Incrementa dano de guerra na região da batalha
            regioes[v].destruicao_guerra = (regioes[v].destruicao_guerra + 0.02).min(1.0);

            // Verifica se os defensores perderam toda a coesão (retirada)
            let todos_defensores_derrotados = defensores_indices.iter().all(|&d_idx| {
                divisoes[d_idx].organizacao < 5.0 || divisoes[d_idx].efetivos_atuais == 0
            });

            let vitoria_atacante = todos_defensores_derrotados;

            if vitoria_atacante {
                guerras[guerra_idx].batalhas_vencidas_atacantes += 1;

                // Encontra região amiga vizinha para retirada dos defensores
                let mut regiao_recuo_idx = None;
                for viz in grafo.vizinhos(v) {
                    let viz_idx = viz.destino_idx as usize;
                    if regioes[viz_idx].controlador_tag == tag_defensor {
                        regiao_recuo_idx = Some(viz_idx);
                        break;
                    }
                }

                if let Some(r_recuo) = regiao_recuo_idx {
                    let code_recuo = regioes[r_recuo].adm1_code.clone();
                    for &d_idx in &defensores_indices {
                        divisoes[d_idx].regiao_adm1 = code_recuo.clone();
                        divisoes[d_idx].organizacao = divisoes[d_idx].organizacao.max(1.0);
                        divisoes[d_idx].entrincheiramento = 0.0;
                    }
                } else {
                    // Cercados (bolsão)! Tropas são destruídas ou rendidas
                    for &d_idx in &defensores_indices {
                        guerras[guerra_idx].baixas_defensores +=
                            divisoes[d_idx].efetivos_atuais as u64;
                        divisoes[d_idx].efetivos_atuais = 0;
                        divisoes[d_idx].organizacao = 0.0;
                    }
                }

                // Captura a região v
                let dono_v = regioes[v].dono_tag.clone();
                let eh_aliado_dono = guerras[guerra_idx].eh_atacante(&dono_v)
                    || (!guerras[guerra_idx].eh_defensor(&dono_v) && dono_v == tag_atacante);

                if eh_aliado_dono {
                    regioes[v].controlador_tag = dono_v;
                } else {
                    regioes[v].controlador_tag = tag_atacante.clone();
                }

                // Move divisão de vanguarda atacante para a região conquistada
                let d_adv_idx = atacantes_ativos[0];
                divisoes[d_adv_idx].regiao_adm1 = code_alvo.clone();
                divisoes[d_adv_idx].entrincheiramento = 0.0;
            } else {
                guerras[guerra_idx].batalhas_vencidas_defensores += 1;
            }

            relatorios.push(RelatorioBatalha {
                regiao_alvo_adm1: code_alvo.clone(),
                atacante_tag: tag_atacante.clone(),
                defensor_tag: tag_defensor.clone(),
                baixas_atacante: baixas_atk,
                baixas_defensor: baixas_def,
                dano_org_atacante: dano_org_atk,
                dano_org_defensor: dano_org_def,
                vitoria_atacante,
                regiao_conquistada: vitoria_atacante,
            });
        }
    }

    relatorios
}
