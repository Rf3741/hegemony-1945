//! Cálculo de suprimento logístico, malha de transporte e fontes (capitais e portos).
//!
//! Utiliza algoritmo de caminho mínimo com amortecimento sobre a infraestrutura e terreno,
//! determinando o nível de abastecimento efetivo de cada divisão e identificando bolsões cercados.

use crate::entidades::{Divisao, PaisMilitar, RegiaoMapa};
use crate::grafo::GrafoCompilado;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Nó da fila de prioridade para propagação do suprimento.
#[derive(Copy, Clone, PartialEq, Debug)]
struct NoDijkstra {
    custo: f32,
    regiao_idx: u32,
}

impl Eq for NoDijkstra {}

impl Ord for NoDijkstra {
    fn cmp(&self, other: &Self) -> Ordering {
        // Inverte a ordem para criar min-heap
        other
            .custo
            .partial_cmp(&self.custo)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for NoDijkstra {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Buffers reutilizáveis para cálculo de suprimento sem alocações no heap.
#[derive(Debug, Default)]
pub struct BuffersSuprimento {
    fila: BinaryHeap<NoDijkstra>,
    distancias: Vec<f32>,
    fontes: Vec<f32>,
    visitados_stamp: Vec<u32>,
    stamp_atual: u32,
    demanda_local: Vec<f32>,
}

impl BuffersSuprimento {
    /// Garante capacidade dos buffers para o número de regiões do mapa.
    pub fn redimensionar(&mut self, n_regioes: usize) {
        if self.distancias.len() < n_regioes {
            self.distancias.resize(n_regioes, f32::MAX);
            self.fontes.resize(n_regioes, 0.0);
            self.visitados_stamp.resize(n_regioes, 0);
            self.demanda_local.resize(n_regioes, 0.0);
        }
    }
}

/// Atualiza o nível de suprimento de todas as divisões do mundo.
pub fn calcular_suprimentos_mundo(
    regioes: &[RegiaoMapa],
    grafo: &GrafoCompilado,
    paises: &[PaisMilitar],
    divisoes: &mut [Divisao],
    mapa_regioes: &std::collections::HashMap<String, usize>,
    mapa_paises: &std::collections::HashMap<String, usize>,
    custos_terreno: &[f64],
    buffers: &mut BuffersSuprimento,
) {
    let n_regioes = regioes.len();
    buffers.redimensionar(n_regioes);

    // Limpa demanda local
    for d in buffers.demanda_local[..n_regioes].iter_mut() {
        *d = 0.0;
    }

    // Pré-indexa divisões por região e por país (uma única passada), e portos por país
    let mut regiao_da_div: Vec<Option<usize>> = Vec::with_capacity(divisoes.len());
    let mut divs_por_pais: Vec<Vec<usize>> = vec![Vec::new(); paises.len()];
    for (d_idx, div) in divisoes.iter().enumerate() {
        let r = mapa_regioes
            .get(&div.regiao_adm1)
            .copied()
            .filter(|&r| r < n_regioes);
        if let Some(r_idx) = r {
            buffers.demanda_local[r_idx] += 1.0;
        }
        regiao_da_div.push(r);
        if let Some(&p) = mapa_paises.get(&div.pais_tag) {
            divs_por_pais[p].push(d_idx);
        }
    }
    let mut portos_por_pais: Vec<Vec<usize>> = vec![Vec::new(); paises.len()];
    for (r_idx, reg) in regioes.iter().enumerate() {
        if reg.nivel_porto > 0
            && let Some(&p) = mapa_paises.get(&reg.controlador_tag)
        {
            portos_por_pais[p].push(r_idx);
        }
    }

    // Para cada país, propaga suprimentos de suas capitais e portos controlados
    for (p_pos, pais) in paises.iter().enumerate() {
        let p_tag = &pais.tag;
        let mut tem_regioes_controladas = false;

        buffers.stamp_atual = buffers.stamp_atual.wrapping_add(1);
        if buffers.stamp_atual == 0 {
            buffers.visitados_stamp.fill(0);
            buffers.stamp_atual = 1;
        }
        let stamp = buffers.stamp_atual;

        buffers.fila.clear();

        // 1. Identifica fontes: Capital (se controlada pelo país)
        if let Some(&cap_idx) = mapa_regioes.get(&pais.capital_adm1)
            && cap_idx < n_regioes
            && &regioes[cap_idx].controlador_tag == p_tag
        {
            buffers.distancias[cap_idx] = 0.0;
            buffers.fontes[cap_idx] = 1.0;
            buffers.visitados_stamp[cap_idx] = stamp;
            buffers.fila.push(NoDijkstra {
                custo: 0.0,
                regiao_idx: cap_idx as u32,
            });
            tem_regioes_controladas = true;
        }

        // 2. Identifica fontes: Portos marítimos controlados pelo país
        for &r_idx in &portos_por_pais[p_pos] {
            let reg = &regioes[r_idx];
            {
                let forca_porto = ((reg.nivel_porto as f32 / 30.0).clamp(0.25, 1.0)
                    * pais.eficiencia_naval as f32)
                    .clamp(0.0, 1.0);

                if buffers.visitados_stamp[r_idx] != stamp || forca_porto > buffers.fontes[r_idx] {
                    buffers.distancias[r_idx] = 0.0;
                    buffers.fontes[r_idx] = forca_porto;
                    buffers.visitados_stamp[r_idx] = stamp;
                    buffers.fila.push(NoDijkstra {
                        custo: 0.0,
                        regiao_idx: r_idx as u32,
                    });
                    tem_regioes_controladas = true;
                }
            }
        }

        if !tem_regioes_controladas {
            continue;
        }

        // Dijkstra sobre território controlado por este país
        while let Some(NoDijkstra { custo, regiao_idx }) = buffers.fila.pop() {
            let u = regiao_idx as usize;
            if custo > buffers.distancias[u] {
                continue;
            }

            for viz in grafo.vizinhos(u) {
                let v = viz.destino_idx as usize;
                // Suprimento terrestre só flui por território controlado pelo mesmo país
                if &regioes[v].controlador_tag != p_tag {
                    continue;
                }

                let reg_v = &regioes[v];
                let infra_efetiva = (reg_v.infraestrutura as f32)
                    * (1.0 - 0.5 * reg_v.destruicao_guerra as f32)
                    * (if reg_v.esta_ocupada() { 0.70 } else { 1.0 });
                let infra_clamped = infra_efetiva.clamp(5.0, 100.0);

                let custo_t = custos_terreno.get(v).copied().unwrap_or(1.0) as f32;
                // Custo de passagem baseado na infraestrutura e terreno
                let custo_passo = custo_t * (120.0 / (20.0 + infra_clamped));
                let novo_custo = custo + custo_passo;

                if buffers.visitados_stamp[v] != stamp || novo_custo < buffers.distancias[v] {
                    buffers.distancias[v] = novo_custo;
                    buffers.fontes[v] = buffers.fontes[u];
                    buffers.visitados_stamp[v] = stamp;
                    buffers.fila.push(NoDijkstra {
                        custo: novo_custo,
                        regiao_idx: v as u32,
                    });
                }
            }
        }

        // Atualiza as divisões do país com base nas distâncias calculadas
        let p_idx_opt = mapa_paises.get(p_tag).copied();
        let fator_nacional = p_idx_opt
            .map(|idx| paises[idx].estoque_suprimento)
            .unwrap_or(1.0);

        for &d_idx in &divs_por_pais[p_pos] {
            let div = &mut divisoes[d_idx];

            let mut suprimento_alvo = 0.05; // Mínimo de sobrevivência se isolado

            if let Some(r_idx) = regiao_da_div[d_idx]
                && buffers.visitados_stamp[r_idx] == stamp
            {
                let dist = buffers.distancias[r_idx];
                let forca_hub = buffers.fontes[r_idx] as f64;

                // Alcance pleno até ~4 saltos eficazes; além disso decai suavemente
                let alcance_pleno = 4.0;
                let acesso_rede = if dist <= alcance_pleno as f32 {
                    forca_hub
                } else {
                    let excesso = (dist - alcance_pleno as f32) as f64;
                    (forca_hub / (1.0 + (excesso / 12.0).powi(2))).clamp(0.05, 1.0)
                };

                // Capacidade local de suporte da região
                let reg = &regioes[r_idx];
                let infra_val = reg.infraestrutura as f64;
                let porto_val = reg.nivel_porto as f64;
                let capacidade_local =
                    (3.0 + 0.10 * infra_val + 0.15 * porto_val) * acesso_rede * fator_nacional;

                let demanda = buffers.demanda_local[r_idx] as f64;
                let fator_congestao = if demanda <= capacidade_local {
                    1.0
                } else {
                    (capacidade_local / demanda).clamp(0.20, 1.0)
                };

                suprimento_alvo = (acesso_rede * fator_congestao * fator_nacional).clamp(0.05, 1.0);
            }

            // Convergência diária de suprimento
            if div.suprimento_atual < suprimento_alvo {
                div.suprimento_atual = (div.suprimento_atual + 0.35).min(suprimento_alvo);
            } else {
                div.suprimento_atual = (div.suprimento_atual - 0.20).max(suprimento_alvo);
            }

            // Atrito severo para unidades totalmente cortadas de suprimento
            if div.suprimento_atual < 0.25 {
                let perda_homens = (div.efetivos_atuais as f64 * 0.02) as u32;
                div.efetivos_atuais = div.efetivos_atuais.saturating_sub(perda_homens);
                div.organizacao = (div.organizacao - 1.5).max(0.0);
            }
        }
    }
}
