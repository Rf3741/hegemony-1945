//! Fase (4) e parte da Fase (8): Renda disponível das pops, impostos diretos (renda e terra),
//! perfis de consumo por estrato (Lei de Engel), substituição elástica de bens e imposto de consumo.

use crate::catalogo::{CandidatoSubstituicao, CatalogoEconomia};
use crate::entidades::{ConfigSimulacao, Estado, Mercado, Pais, Provincia};
use crate::tipos::{CategoriaNecessidade, Estrato, NUM_NECESSIDADES};
use rayon::prelude::*;

/// Trabalho de consumo/tributos de um país (partes disjuntas do estado, seguras para threads).
struct TarefaPaisConsumo<'a> {
    pais: &'a mut Pais,
    demanda_nec: &'a mut [f64; NUM_NECESSIDADES],
    cestas: &'a [CestaNecessidadeCompilada; NUM_NECESSIDADES],
    c_dividendos_cap_pais: f64,
    c_dividendos_ari_pais: f64,
    c_pop_cap_pais: f64,
    c_pop_ari_pais: f64,
    provs: Vec<(usize, &'a mut Provincia)>,
}

/// Cesta pré-calculada por `(Pais, CategoriaNecessidade)` para um tick.
#[derive(Debug, Clone, Default)]
pub(crate) struct CestaNecessidadeCompilada {
    pub ativa: bool,
    /// Pares `(bem_idx, fatia_despesa)` cuja soma das fatias é `1.0`.
    pub fatias_despesa: Vec<(usize, f64)>,
    /// Valor real (a preços-base) adquirido por 1 unidade monetária gasta nesta necessidade.
    pub valor_real_por_moeda: f64,
    /// Imposto de consumo gerado por 1 unidade monetária gasta nesta necessidade.
    pub imposto_por_moeda: f64,
}

/// Calcula as fatias orçamentárias de substituição entre bens candidatos de uma necessidade.
///
/// Considera:
/// 1. `peso` intrínseco do bem na necessidade;
/// 2. disponibilidade relativa de oferta no mercado;
/// 3. razão de preço efetivo ao consumidor `(preco_mercado * (1 + imposto)) / preco_base`
///    elevada a `-elasticidade_substituicao`;
/// 4. tetos e pisos de participação (`fracao_minima..=fracao_maxima`).
pub fn calcular_fatias_substituicao(
    catalogo: &CatalogoEconomia,
    necessidade: CategoriaNecessidade,
    precos_mercado: &[f64],
    ofertas_mercado: &[f64],
    aliquotas_consumo_bem: &[f64],
    elasticidade: f64,
) -> Vec<(usize, f64)> {
    let candidatos = &catalogo.candidatos_por_necessidade[necessidade.indice()];
    if candidatos.is_empty() {
        return Vec::new();
    }
    let mut saida = Vec::with_capacity(candidatos.len());
    preencher_fatias_substituicao(
        catalogo,
        candidatos,
        precos_mercado,
        ofertas_mercado,
        aliquotas_consumo_bem,
        None,
        elasticidade,
        &mut saida,
    );
    saida
}

fn preencher_fatias_substituicao(
    catalogo: &CatalogoEconomia,
    candidatos: &[CandidatoSubstituicao],
    precos_mercado: &[f64],
    ofertas_mercado: &[f64],
    aliquotas_consumo_bem: &[f64],
    pesos_era_bem: Option<&[f64]>,
    elasticidade: f64,
    saida: &mut Vec<(usize, f64)>,
) {
    saida.clear();
    if candidatos.is_empty() {
        return;
    }
    if candidatos.len() == 1 {
        saida.push((candidatos[0].bem_idx, 1.0));
        return;
    }

    let mut oferta_valor_total = 0.0;
    for c in candidatos {
        let o = ofertas_mercado
            .get(c.bem_idx)
            .copied()
            .unwrap_or(0.0)
            .max(0.0);
        oferta_valor_total += o * catalogo.bens[c.bem_idx].preco_base;
    }

    let eta = elasticidade.max(0.1);
    let mut soma_util = 0.0;
    for c in candidatos {
        let g = c.bem_idx;
        let preco_base = catalogo.bens[g].preco_base;
        let preco_merc = precos_mercado
            .get(g)
            .copied()
            .unwrap_or(preco_base)
            .max(1e-6);
        let tau = aliquotas_consumo_bem
            .get(g)
            .copied()
            .unwrap_or(0.0)
            .max(0.0);
        let razao_preco = ((preco_merc * (1.0 + tau)) / preco_base).max(0.05);

        let oferta_g = ofertas_mercado.get(g).copied().unwrap_or(0.0).max(0.0) * preco_base;
        let fator_oferta = if oferta_valor_total > 1e-6 {
            0.20 + (oferta_g / oferta_valor_total)
        } else {
            1.0
        };

        let mult_era = pesos_era_bem
            .and_then(|v| v.get(g).copied())
            .unwrap_or(1.0)
            .max(0.05);
        let util = c.peso * mult_era * fator_oferta * razao_preco.powf(-eta);
        saida.push((g, util));
        soma_util += util;
    }

    if soma_util <= 1e-12 {
        let igual = 1.0 / (candidatos.len() as f64);
        for item in saida.iter_mut() {
            item.1 = igual;
        }
        return;
    }

    for item in saida.iter_mut() {
        item.1 /= soma_util;
    }

    // Duas passadas de respeito a [fracao_minima, fracao_maxima] com redistribuição proporcional
    let soma_max: f64 = candidatos.iter().map(|c| c.fracao_maxima).sum();
    if soma_max >= 1.0 - 1e-6 {
        for _ in 0..2 {
            let mut fixado = 0.0;
            let mut livre_soma = 0.0;
            for (i, c) in candidatos.iter().enumerate() {
                let mult_era_min = pesos_era_bem
                    .and_then(|v| v.get(c.bem_idx).copied())
                    .unwrap_or(1.0)
                    .clamp(0.05, 1.0);
                let f_min = c.fracao_minima * mult_era_min;
                let v = saida[i].1;
                if v > c.fracao_maxima {
                    saida[i].1 = c.fracao_maxima;
                    fixado += c.fracao_maxima;
                } else if v < f_min {
                    saida[i].1 = f_min;
                    fixado += f_min;
                } else {
                    livre_soma += v;
                }
            }
            let restante = (1.0 - fixado).max(0.0);
            if livre_soma > 1e-9 {
                let escala = restante / livre_soma;
                for (i, c) in candidatos.iter().enumerate() {
                    let mult_era_min = pesos_era_bem
                        .and_then(|v| v.get(c.bem_idx).copied())
                        .unwrap_or(1.0)
                        .clamp(0.05, 1.0);
                    let f_min = c.fracao_minima * mult_era_min;
                    let v = saida[i].1;
                    if v > f_min && v < c.fracao_maxima {
                        saida[i].1 = (v * escala).clamp(f_min, c.fracao_maxima);
                    }
                }
            }
        }
    }

    let soma_final: f64 = saida.iter().map(|x| x.1).sum();
    if soma_final > 1e-12 {
        for item in saida.iter_mut() {
            item.1 /= soma_final;
        }
    }
}

/// Processa rendas das pops, recolhe impostos de renda e sobre a terra, calcula consumo
/// por estrato com substituição de bens, lança ordens de compra nos mercados e atualiza
/// `padrao_de_vida` e `alfabetizacao` (Fase 5 acoplada em passagem única de memória).
#[allow(clippy::too_many_arguments)]
pub(crate) fn executar_consumo_tributos_e_bem_estar(
    catalogo: &CatalogoEconomia,
    config: &ConfigSimulacao,
    mercados: &[Mercado],
    estados: &[Estado],
    paises: &mut [Pais],
    provincias: &mut [Provincia],
    salarios_prov_tipo: &[f64],
    dividendos_cap_prov: &[f64],
    dividendos_ari_prov: &[f64],
    dividendos_cap_pais: &[f64],
    dividendos_ari_pais: &[f64],
    pop_cap_pais: &[f64],
    pop_ari_pais: &[f64],
    cestas_pais: &mut [[CestaNecessidadeCompilada; NUM_NECESSIDADES]],
    demanda_necessidade_pais: &mut [[f64; NUM_NECESSIDADES]],
    pesos_era_bem: &[f64],
    _empregados_tipo_scratch: &mut [f64],
    _pop_total_tipo_scratch: &mut [f64],
    proxima_demanda: &mut [f64],
) {
    let num_bens = catalogo.bens.len();
    let num_tipos_pop = catalogo.tipos_pop.len();

    // 1. Pré-calcular a cesta ótima de substituição por (País, CategoriaNecessidade) — paralelo por país
    // (cada país escreve só na sua cesta/demanda; leitura apenas dos mercados).
    let paises_ro: &[Pais] = &*paises;
    paises_ro
        .par_iter()
        .zip(cestas_pais.par_iter_mut())
        .zip(demanda_necessidade_pais.par_iter_mut())
        .for_each(|((pais, cestas), demanda_nec)| {
            *demanda_nec = [0.0; NUM_NECESSIDADES];
            let mercado = &mercados[pais.mercado_idx];
            for nec in CategoriaNecessidade::TODAS {
                let n_idx = nec.indice();
                let candidatos = &catalogo.candidatos_por_necessidade[n_idx];
                let cesta = &mut cestas[n_idx];
                if candidatos.is_empty() {
                    cesta.ativa = false;
                    cesta.fatias_despesa.clear();
                    cesta.valor_real_por_moeda = 0.0;
                    cesta.imposto_por_moeda = 0.0;
                    continue;
                }
                cesta.ativa = true;
                preencher_fatias_substituicao(
                    catalogo,
                    candidatos,
                    &mercado.precos_idx,
                    &mercado.oferta_idx,
                    &pais.aliquota_consumo_por_bem,
                    Some(pesos_era_bem),
                    config.elasticidade_substituicao,
                    &mut cesta.fatias_despesa,
                );

                let mut v_real = 0.0;
                let mut imp_moeda = 0.0;
                for &(g, s) in &cesta.fatias_despesa {
                    let preco_base = catalogo.bens[g].preco_base;
                    let preco_m = mercado.precos_idx[g].max(1e-6);
                    let tau = pais.aliquota_consumo_por_bem[g].max(0.0);
                    let preco_cons = preco_m * (1.0 + tau);
                    v_real += s * (preco_base / preco_cons);
                    imp_moeda += s * (tau / (1.0 + tau));
                }
                cesta.valor_real_por_moeda = v_real;
                cesta.imposto_por_moeda = imp_moeda;
            }
        });

    // 2. Passagem sobre as províncias, paralela por PAÍS. Cada país escreve só em si mesmo, na sua demanda por
    // necessidade e nas suas províncias; a ordem das províncias dentro do país é a global, então as somas de
    // ponto flutuante são idênticas à versão sequencial (determinístico, independente do nº de threads).
    let cestas_ro: &[[CestaNecessidadeCompilada; NUM_NECESSIDADES]] = &*cestas_pais;
    let mut tarefas: Vec<TarefaPaisConsumo> = paises
        .iter_mut()
        .zip(demanda_necessidade_pais.iter_mut())
        .enumerate()
        .map(|(c, (pais, demanda_nec))| TarefaPaisConsumo {
            pais,
            demanda_nec,
            cestas: &cestas_ro[c],
            c_dividendos_cap_pais: dividendos_cap_pais[c],
            c_dividendos_ari_pais: dividendos_ari_pais[c],
            c_pop_cap_pais: pop_cap_pais[c],
            c_pop_ari_pais: pop_ari_pais[c],
            provs: Vec::new(),
        })
        .collect();
    for (p_idx, prov) in provincias.iter_mut().enumerate() {
        let c = prov.pais_idx;
        tarefas[c].provs.push((p_idx, prov));
    }
    tarefas.into_par_iter().for_each(|t| {
        let TarefaPaisConsumo { pais, demanda_nec, cestas, c_dividendos_cap_pais, c_dividendos_ari_pais, c_pop_cap_pais, c_pop_ari_pais, provs } = t;
        let mut empregados_tipo_scratch = vec![0.0f64; num_tipos_pop];
        let mut pop_total_tipo_scratch = vec![0.0f64; num_tipos_pop];
        for (p_idx, prov) in provs {
            let e_idx = prov.estado_idx;
            let offset_sal = p_idx * num_tipos_pop;

            empregados_tipo_scratch.fill(0.0);
            pop_total_tipo_scratch.fill(0.0);
            let mut pop_cap_local = 0.0;
            let mut pop_ari_local = 0.0;
            let mut camponeses_desemp_local = 0.0;

            for pop in &prov.pops {
                if pop.populacao <= 0.0 {
                    continue;
                }
                empregados_tipo_scratch[pop.tipo_idx] += pop.empregados;
                pop_total_tipo_scratch[pop.tipo_idx] += pop.populacao;
                let tp = &catalogo.tipos_pop[pop.tipo_idx];
                if tp.recebe_dividendos_capitalista {
                    pop_cap_local += pop.populacao;
                }
                if tp.recebe_dividendos_aristocrata {
                    pop_ari_local += pop.populacao;
                }
                if tp.subsistencia {
                    camponeses_desemp_local += pop.desempregados();
                }
            }

            let cap_terra = (prov.terra_aravel_livre() as f64) * 5_000.0;
            let fator_terra = if camponeses_desemp_local <= cap_terra || camponeses_desemp_local <= 1.0
            {
                1.0
            } else {
                (cap_terra / camponeses_desemp_local).clamp(0.35, 1.0)
            };

            let estado_incorporado = estados[e_idx].incorporado;
            let fator_inc = if estado_incorporado { 1.0 } else { 0.35 };
            let efic_fiscal = pais.eficiencia_arrecadacao * fator_inc;

            let aliq_baixo = pais
                .politica_fiscal
                .aliquota_imposto_renda_baixo
                .clamp(0.0, 0.85);
            let aliq_medio = pais
                .politica_fiscal
                .aliquota_imposto_renda_medio
                .clamp(0.0, 0.85);
            let aliq_alto = pais
                .politica_fiscal
                .aliquota_imposto_renda_alto
                .clamp(0.0, 0.85);
            let aliq_div = pais
                .politica_fiscal
                .aliquota_imposto_dividendos
                .clamp(0.0, 0.85);
            let imp_terra_camp = pais
                .politica_fiscal
                .imposto_sobre_terra_por_campones
                .max(0.0);
            let nivel_educ = pais.politica_fiscal.nivel_investimento_educacao;
            let efic_pais = pais.eficiencia_arrecadacao;

            let mut rec_renda_prov = 0.0;
            let mut rec_terra_prov = 0.0;
            let mut pib_subsistencia_prov = 0.0;

            for pop in &mut prov.pops {
                if pop.populacao <= 1e-6 {
                    pop.populacao = 0.0;
                    pop.empregados = 0.0;
                    pop.renda_ultimo_tick = 0.0;
                    pop.gasto_consumo_ultimo_tick = 0.0;
                    continue;
                }

                let tp = &catalogo.tipos_pop[pop.tipo_idx];

                // 2a. Renda salarial
                let total_emp_tp = empregados_tipo_scratch[pop.tipo_idx];
                let renda_salarial = if pop.empregados > 0.0 && total_emp_tp > 1e-9 {
                    salarios_prov_tipo[offset_sal + pop.tipo_idx] * (pop.empregados / total_emp_tp)
                } else {
                    0.0
                };

                // 2b. Renda e autoconsumo de subsistência (camponeses)
                let (renda_monetaria_subs, valor_real_autoconsumo) = if tp.subsistencia {
                    let desemp = pop.desempregados();
                    let parcela_mercado = tp.fator_consumo_mercado.clamp(0.10, 1.0);
                    let monetaria = desemp * 1.45 * parcela_mercado * fator_terra;
                    let autoconsumo = desemp * 1.65 * (1.0 - parcela_mercado * 0.65) * fator_terra;
                    pib_subsistencia_prov += monetaria;
                    (monetaria, autoconsumo)
                } else {
                    (0.0, 0.0)
                };

                // 2c. Dividendos
                let mut renda_dividendos = 0.0;
                if tp.recebe_dividendos_capitalista {
                    if pop_cap_local > 1e-9 {
                        renda_dividendos +=
                            dividendos_cap_prov[p_idx] * (pop.populacao / pop_cap_local);
                    }
                    if c_pop_cap_pais > 1e-9 {
                        renda_dividendos +=
                            c_dividendos_cap_pais * (pop.populacao / c_pop_cap_pais);
                    }
                }
                if tp.recebe_dividendos_aristocrata {
                    if pop_ari_local > 1e-9 {
                        renda_dividendos +=
                            dividendos_ari_prov[p_idx] * (pop.populacao / pop_ari_local);
                    }
                    if c_pop_ari_pais > 1e-9 {
                        renda_dividendos +=
                            c_dividendos_ari_pais * (pop.populacao / c_pop_ari_pais);
                    }
                }

                // 2d. Impostos diretos (renda salarial, dividendos e imposto sobre a terra)
                let aliq_estrato = match tp.estrato {
                    Estrato::Baixo => aliq_baixo,
                    Estrato::Medio => aliq_medio,
                    Estrato::Alto => aliq_alto,
                };
                let base_trabalho = renda_salarial + renda_monetaria_subs;
                let trib_renda_bruto = base_trabalho * aliq_estrato + renda_dividendos * aliq_div;
                let trib_terra_bruto = if tp.subsistencia && imp_terra_camp > 0.0 {
                    (pop.desempregados() * imp_terra_camp).min(base_trabalho * 0.35)
                } else {
                    0.0
                };

                // Eficiência burocrática: o Estado arrecada `efic_fiscal`; metade da evasão fica com a pop
                let retido_renda = trib_renda_bruto * (0.5 + 0.5 * efic_fiscal);
                let retido_terra = trib_terra_bruto * (0.5 + 0.5 * efic_fiscal);
                rec_renda_prov += trib_renda_bruto * efic_fiscal;
                rec_terra_prov += trib_terra_bruto * efic_fiscal;

                let renda_bruta = base_trabalho + renda_dividendos;
                let renda_liquida = (renda_bruta - retido_renda - retido_terra).max(0.0);
                pop.renda_ultimo_tick = renda_liquida;

                // 2e. Orçamento doméstico de consumo com colchão suavizador de poupança (15 dias)
                let reserva_alvo = 15.0 * renda_liquida;
                // Poupança limitada a 15% da renda: sem esse piso, pops com reserva inicial baixa passavam
                // semanas poupando ~90% da renda e a demanda por bens despencava (colapso de 1945).
                let orcamento_consumo = (renda_liquida + 0.06 * (pop.riqueza_reservas - reserva_alvo))
                    .clamp(0.85 * renda_liquida, renda_liquida + 0.25 * pop.riqueza_reservas);

                pop.riqueza_reservas =
                    (pop.riqueza_reservas + renda_liquida - orcamento_consumo).max(0.0);
                pop.gasto_consumo_ultimo_tick = orcamento_consumo;

                // 2f. Alocação pelas 6 categorias de necessidade segundo estrato + Lei de Engel
                let mut valor_real_comprado = 0.0;
                if orcamento_consumo > 1e-9 {
                    let mut pesos =
                        CategoriaNecessidade::pesos_orcamento(tp.estrato, pop.padrao_de_vida);
                    let mut soma_ativa = 0.0;
                    for n in 0..NUM_NECESSIDADES {
                        if !cestas[n].ativa {
                            pesos[n] = 0.0;
                        } else {
                            soma_ativa += pesos[n];
                        }
                    }
                    if soma_ativa > 1e-12 {
                        let inv_soma = 1.0 / soma_ativa;
                        for n in 0..NUM_NECESSIDADES {
                            if pesos[n] > 0.0 {
                                let despesa_n = orcamento_consumo * pesos[n] * inv_soma;
                                demanda_nec[n] += despesa_n;
                                valor_real_comprado +=
                                    despesa_n * cestas[n].valor_real_por_moeda;
                            }
                        }
                    }
                }

                // 2g. Fase (5): Atualização contínua do Padrão de Vida e da Alfabetização
                let valor_real_per_capita =
                    (valor_real_comprado + valor_real_autoconsumo) / pop.populacao.max(1.0);
                let sol_alvo = (2.0 + 7.2 * (1.0 + valor_real_per_capita).ln()).clamp(1.0, 99.0);
                pop.padrao_de_vida =
                    (pop.padrao_de_vida + 0.03 * (sol_alvo - pop.padrao_de_vida)).clamp(1.0, 99.0);

                let acesso_escola = (nivel_educ as f64) * 0.14 * fator_inc * efic_pais;
                let acesso_sol = (pop.padrao_de_vida * 0.015).min(0.45);
                let bonus_prof = match tp.estrato {
                    Estrato::Baixo => 0.0,
                    Estrato::Medio => 0.18,
                    Estrato::Alto => 0.25,
                };
                let alfa_alvo = (0.05 + acesso_escola + acesso_sol + bonus_prof).clamp(0.02, 0.995);
                pop.alfabetizacao =
                    (pop.alfabetizacao + 0.0015 * (alfa_alvo - pop.alfabetizacao)).clamp(0.0, 1.0);
            }

            pais.orcamento.receita_imposto_renda += rec_renda_prov;
            pais.orcamento.receita_imposto_terra += rec_terra_prov;
            pais.pib_diario += pib_subsistencia_prov;
        }
    });

    // 3. Desdobrar a demanda agregada por necessidade em ordens de compra por bem e imposto de consumo
    for (c_idx, pais) in paises.iter_mut().enumerate() {
        let m_idx = pais.mercado_idx;
        let offset_m = m_idx * num_bens;
        let mercado = &mercados[m_idx];
        let mut rec_consumo = 0.0;

        for n in 0..NUM_NECESSIDADES {
            let gasto_total_n = demanda_necessidade_pais[c_idx][n];
            if gasto_total_n <= 1e-9 || !cestas_pais[c_idx][n].ativa {
                continue;
            }
            let cesta = &cestas_pais[c_idx][n];
            rec_consumo += gasto_total_n * cesta.imposto_por_moeda * pais.eficiencia_arrecadacao;

            for &(g, s) in &cesta.fatias_despesa {
                let preco_m = mercado.precos_idx[g].max(1e-6);
                let tau = pais.aliquota_consumo_por_bem[g].max(0.0);
                let qtd = (gasto_total_n * s) / (preco_m * (1.0 + tau));
                proxima_demanda[offset_m + g] += qtd;
            }
        }

        pais.orcamento.receita_imposto_consumo += rec_consumo;
    }
}
