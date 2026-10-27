//! Geração e manutenção automática de frentes operacionais na fronteira entre beligerantes.
//!
//! Agrupa pares adjacentes de regiões inimigas em componentes conexos contíguos (frentes),
//! mantendo estabilidade de IDs através dos ticks e distribuindo divisões ao longo da linha.

use crate::entidades::{Divisao, Exercito, Frente, Guerra, RegiaoMapa};
use crate::grafo::GrafoCompilado;
use std::collections::{HashMap, HashSet};

/// Par de índices de regiões fronteiriças entre beligerantes inimigos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParBordaIdx {
    pub regiao_a: usize,
    pub regiao_b: usize,
}

/// Atualiza todas as frentes de combate ativas para as guerras em andamento.
pub fn atualizar_frentes_mundo(
    regioes: &[RegiaoMapa],
    grafo: &GrafoCompilado,
    guerras: &[Guerra],
    frentes: &mut Vec<Frente>,
    exercitos: &mut [Exercito],
    divisoes: &mut [Divisao],
    _mapa_regioes: &HashMap<String, usize>,
    proximo_frente_id: &mut u32,
) {
    // Interna as tags de controlador em ids pequenos (uma passada) para varreduras baratas
    let mut ids_tag: HashMap<&str, usize> = HashMap::new();
    let ctrl_id: Vec<usize> = regioes
        .iter()
        .map(|r| {
            let n = ids_tag.len();
            *ids_tag.entry(r.controlador_tag.as_str()).or_insert(n)
        })
        .collect();

    // Coleta guerras ativas
    for guerra in guerras.iter().filter(|g| g.ativa) {
        let mut pares_borda: Vec<ParBordaIdx> = Vec::new();
        let mut visitados_pares: HashSet<(usize, usize)> = HashSet::new();

        // 1. Encontra todas as arestas de fronteira entre os dois lados da guerra
        // Papel por tag: bit 1 = atacante, bit 2 = defensor
        let mut papel = vec![0u8; ids_tag.len()];
        for (tag, &id) in &ids_tag {
            if guerra.eh_atacante(tag) {
                papel[id] |= 1;
            }
            if guerra.eh_defensor(tag) {
                papel[id] |= 2;
            }
        }
        for u in 0..regioes.len() {
            let pu = papel[ctrl_id[u]];
            if pu == 0 {
                continue;
            }

            for viz in grafo.vizinhos(u) {
                let v = viz.destino_idx as usize;
                let pv = papel[ctrl_id[v]];

                if (pu & 1 != 0 && pv & 2 != 0) || (pu & 2 != 0 && pv & 1 != 0) {
                    let min_idx = u.min(v);
                    let max_idx = u.max(v);
                    if visitados_pares.insert((min_idx, max_idx)) {
                        let (reg_a, reg_b) = if pu & 1 != 0 { (u, v) } else { (v, u) };
                        pares_borda.push(ParBordaIdx {
                            regiao_a: reg_a,
                            regiao_b: reg_b,
                        });
                    }
                }
            }
        }

        // 2. Agrupa os pares de fronteira em componentes contíguos (frentes)
        let mut frentes_novas_grupos: Vec<Vec<ParBordaIdx>> = Vec::new();
        let mut processados: Vec<bool> = vec![false; pares_borda.len()];

        for i in 0..pares_borda.len() {
            if processados[i] {
                continue;
            }
            processados[i] = true;

            let mut grupo = vec![pares_borda[i]];
            let mut fila = vec![pares_borda[i]];

            while let Some(par_atual) = fila.pop() {
                for j in 0..pares_borda.len() {
                    if processados[j] {
                        continue;
                    }
                    let cand = pares_borda[j];

                    // São contíguos se compartilham região ou se são vizinhos no lado A ou lado B
                    let conecta_a = par_atual.regiao_a == cand.regiao_a
                        || grafo.sao_adjacentes(par_atual.regiao_a, cand.regiao_a);
                    let conecta_b = par_atual.regiao_b == cand.regiao_b
                        || grafo.sao_adjacentes(par_atual.regiao_b, cand.regiao_b);

                    if conecta_a || conecta_b {
                        processados[j] = true;
                        grupo.push(cand);
                        fila.push(cand);
                    }
                }
            }
            frentes_novas_grupos.push(grupo);
        }

        // 3. Emparelha grupos novos com frentes existentes da mesma guerra (preservando IDs)
        let mut frentes_existentes_guerra: Vec<(usize, u32, HashSet<String>)> = frentes
            .iter()
            .enumerate()
            .filter(|(_, f)| f.guerra_id == guerra.id && f.ativa)
            .map(|(idx, f)| {
                let regs: HashSet<String> = f
                    .regioes_lado_a
                    .iter()
                    .chain(f.regioes_lado_b.iter())
                    .cloned()
                    .collect();
                (idx, f.id, regs)
            })
            .collect();

        let mut novas_frentes: Vec<Frente> = Vec::new();

        for grupo in frentes_novas_grupos {
            let mut lado_a_set: HashSet<String> = HashSet::new();
            let mut lado_b_set: HashSet<String> = HashSet::new();
            let mut pares_str: Vec<(String, String)> = Vec::with_capacity(grupo.len());

            for p in &grupo {
                let code_a = regioes[p.regiao_a].adm1_code.clone();
                let code_b = regioes[p.regiao_b].adm1_code.clone();
                lado_a_set.insert(code_a.clone());
                lado_b_set.insert(code_b.clone());
                pares_str.push((code_a, code_b));
            }

            let mut regioes_lado_a: Vec<String> = lado_a_set.into_iter().collect();
            let mut regioes_lado_b: Vec<String> = lado_b_set.into_iter().collect();
            regioes_lado_a.sort();
            regioes_lado_b.sort();

            let conjunto_novo: HashSet<String> = regioes_lado_a
                .iter()
                .chain(regioes_lado_b.iter())
                .cloned()
                .collect();

            // Busca sobreposição máxima com frente existente
            let mut melhor_sobreposicao = 0usize;
            let mut melhor_existente_pos = None;

            for (pos, (_, _, regs_antigas)) in frentes_existentes_guerra.iter().enumerate() {
                let inter = conjunto_novo.intersection(regs_antigas).count();
                if inter > melhor_sobreposicao {
                    melhor_sobreposicao = inter;
                    melhor_existente_pos = Some(pos);
                }
            }

            let frente_id = if let Some(pos) = melhor_existente_pos {
                let (_, id, _) = frentes_existentes_guerra.remove(pos);
                id
            } else {
                let id = *proximo_frente_id;
                *proximo_frente_id += 1;
                id
            };

            let lado_a_lider = guerra.atacantes.first().cloned().unwrap_or_default();
            let lado_b_lider = guerra.defensores.first().cloned().unwrap_or_default();
            let nome_pt =
                format!("Frente Operacional {frente_id} ({lado_a_lider} x {lado_b_lider})");

            novas_frentes.push(Frente {
                id: frente_id,
                nome_pt,
                guerra_id: guerra.id.clone(),
                lado_a_tag: lado_a_lider,
                lado_b_tag: lado_b_lider,
                regioes_lado_a,
                regioes_lado_b,
                pares_fronteira: pares_str,
                ativa: true,
            });
        }

        // Desativa frentes antigas que deixaram de existir
        for f in frentes.iter_mut().filter(|f| f.guerra_id == guerra.id) {
            f.ativa = false;
        }

        // Atualiza ou insere as novas frentes
        for nova in novas_frentes {
            if let Some(existente) = frentes.iter_mut().find(|f| f.id == nova.id) {
                *existente = nova;
            } else {
                frentes.push(nova);
            }
        }
    }

    // 4. Atribui exércitos órfãos de países em guerra às frentes ativas
    let precisa_orfaos = exercitos.iter().any(|e| {
        e.frente_id
            .is_none_or(|id| !frentes.iter().any(|f| f.ativa && f.id == id))
    });
    let frentes_ativas: Vec<(u32, String, Vec<String>, Vec<String>)> = if !precisa_orfaos {
        Vec::new()
    } else {
        frentes
            .iter()
            .filter(|f| f.ativa)
            .map(|f| {
                (
                    f.id,
                    f.guerra_id.clone(),
                    f.regioes_lado_a.clone(),
                    f.regioes_lado_b.clone(),
                )
            })
            .collect()
    };

    for exercito in exercitos.iter_mut() {
        let tag = &exercito.pais_tag;
        // Se o exército já tem frente ativa válida, mantém
        if let Some(f_id) = exercito.frente_id
            && frentes_ativas.iter().any(|(id, _, _, _)| *id == f_id)
        {
            continue;
        }

        // Procura uma frente ativa em que o país seja beligerante
        for (f_id, g_id, lado_a, lado_b) in &frentes_ativas {
            if let Some(guerra) = guerras.iter().find(|g| &g.id == g_id && g.ativa)
                && guerra.participa(tag)
            {
                exercito.frente_id = Some(*f_id);
                let regs_lado = if guerra.eh_atacante(tag) {
                    lado_a
                } else {
                    lado_b
                };
                if exercito.regiao_objetivo_adm1.is_none() && !regs_lado.is_empty() {
                    exercito.regiao_objetivo_adm1 = regs_lado.first().cloned();
                }
                break;
            }
        }
    }

    // 5. Posiciona divisões dos exércitos designados ao longo da linha de frente
    let indice_divs: HashMap<String, usize> = divisoes
        .iter()
        .enumerate()
        .map(|(i, d)| (d.id.clone(), i))
        .collect();
    for exercito in exercitos.iter() {
        if let Some(f_id) = exercito.frente_id
            && let Some(frente) = frentes.iter().find(|f| f.id == f_id && f.ativa)
            && let Some(guerra) = guerras.iter().find(|g| g.id == frente.guerra_id && g.ativa)
        {
            let regs_lado = if guerra.eh_atacante(&exercito.pais_tag) {
                &frente.regioes_lado_a
            } else {
                &frente.regioes_lado_b
            };
            if !regs_lado.is_empty() {
                for (d_num, d_id) in exercito.divisoes_ids.iter().enumerate() {
                    if let Some(&di) = indice_divs.get(d_id)
                        && let Some(div) = divisoes.get_mut(di)
                        && !regs_lado.contains(&div.regiao_adm1)
                    {
                        div.regiao_adm1 = regs_lado[d_num % regs_lado.len()].clone();
                    }
                }
            }
        }
    }
}
