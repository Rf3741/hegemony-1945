//! Fase (6): Crescimento vegetativo das pops (transição demográfica)
//! e migração interna entre províncias atraída por padrão de vida e vagas abertas.

use crate::catalogo::CatalogoEconomia;
use crate::entidades::{Pais, Pop, Provincia};

/// Registro compacto (`Copy`, sem alocações `String`) de fluxo migratório entre províncias.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LoteMigrante {
    pub tipo_idx: usize,
    pub cultura_idx: u16,
    pub religiao_idx: u16,
    pub quantidade: f64,
    pub riqueza: f64,
    pub padrao_de_vida: f64,
    pub alfabetizacao: f64,
}

/// Taxa líquida anual de crescimento populacional conforme padrão de vida, alfabetização e época histórica.
///
/// Transição demográfica do pós-guerra e contemporânea: a mortalidade despenca com a medicina moderna
/// (antibióticos, DDT, vacinas, saneamento) antes da natalidade cair, gerando pico nos anos 50-70 em países
/// pobres e médios (~2,2% a 2,8%/ano) e menor nos países ricos (baby boom ocidental ~1,2%/ano).
/// A partir de 1975, a fecundidade cai progressivamente com a urbanização, anticoncepção e educação.
pub fn taxa_crescimento_anual_pop_epoca(padrao_de_vida: f64, alfabetizacao: f64, ano: f64) -> f64 {
    let sol = padrao_de_vida.clamp(1.0, 99.0);
    let alfa = alfabetizacao.clamp(0.0, 1.0);
    // Mortalidade severa apenas em extrema fome/miséria (< 3.0)
    if sol < 3.0 {
        return -0.012 * ((3.0 - sol) / 2.0).clamp(0.0, 1.0);
    }

    // 1. Curva por padrão de vida (estágios da transição demográfica):
    // SoL 3 a 8: transição inicial e auge de fecundidade (1,4% a 3,0%)
    // SoL 8 a 16: transição plena, urbanização e renda (3,0% cai para 1,4%)
    // SoL 16 a 26: países de renda média-alta / industrializados (1,4% cai para 0,5%)
    // SoL > 26: pós-transição / estabilização demográfica (0,5% cai para ~0,15%)
    let base_sol = if sol <= 8.0 {
        0.014 + 0.016 * ((sol - 3.0) / 5.0)
    } else if sol <= 16.0 {
        0.030 - 0.016 * ((sol - 8.0) / 8.0)
    } else if sol <= 26.0 {
        0.014 - 0.009 * ((sol - 16.0) / 10.0)
    } else {
        (0.005 - 0.00015 * (sol - 26.0)).max(0.0015)
    };

    // 2. Fator época (transição global da Guerra Fria até o século XXI):
    // 1945 a 1975: ápice do pós-guerra e revolução sanitária global (0.88 a 1.04).
    // 1975 a 2010: difusão de métodos contraceptivos e planejamento familiar (1.04 cai para 0.58).
    // 2010 a 2045: estabilização do século XXI (0.58 cai para 0.32).
    let fator_epoca = if ano <= 1975.0 {
        0.88 + 0.16 * ((ano - 1945.0) / 30.0).clamp(0.0, 1.0)
    } else if ano <= 2010.0 {
        let progresso = ((ano - 1975.0) / 35.0).clamp(0.0, 1.0);
        1.04 - 0.46 * progresso
    } else {
        let progresso = ((ano - 2010.0) / 35.0).clamp(0.0, 1.0);
        0.58 - 0.26 * progresso
    };

    // 3. Efeito da alfabetização e educação feminina (acelera a transição demográfica)
    let peso_alfa = 0.0025 + 0.0065 * ((ano - 1965.0).max(0.0) / 60.0).min(1.0);
    (base_sol * fator_epoca - peso_alfa * alfa).clamp(0.001, 0.035)
}

/// Taxa líquida anual de crescimento populacional (compatibilidade: época padrão 1950).
#[inline]
pub fn taxa_crescimento_anual_pop(padrao_de_vida: f64, alfabetizacao: f64) -> f64 {
    taxa_crescimento_anual_pop_epoca(padrao_de_vida, alfabetizacao, 1950.0)
}

/// Executa o crescimento demográfico natural e a migração interprovincial.
#[allow(clippy::too_many_arguments)]
pub(crate) fn executar_crescimento_e_migracao(
    catalogo: &CatalogoEconomia,
    nomes_culturas: &[String],
    nomes_religioes: &[String],
    paises: &[Pais],
    provincias: &mut [Provincia],
    dias_passo: u32,
    dias_decorridos: u64,
    pop_emp_por_tipo_scratch: &mut [f64],
    lotes_migrantes: &mut Vec<LoteMigrante>,
) {
    let fator_dias = dias_passo.max(1) as f64;
    let inv_ano = fator_dias / 365.0;
    let ano = 1945.0 + (dias_decorridos as f64) / 365.25;

    // 1. Crescimento natural e cálculo da atração migratória em passagem única por província
    for prov in provincias.iter_mut() {
        let mut precisou_ajuste_mortalidade = false;
        let mut pop_total = 0.0;
        let mut soma_sol = 0.0;
        let mut desemp_nao_subs = 0.0;

        for pop in &mut prov.pops {
            if pop.populacao <= 0.0 {
                pop.populacao = 0.0;
                pop.empregados = 0.0;
                continue;
            }
            let r_anual = taxa_crescimento_anual_pop_epoca(pop.padrao_de_vida, pop.alfabetizacao, ano);
            let r_passo = (r_anual * inv_ano).clamp(-0.05, 0.05);
            pop.populacao = (pop.populacao * (1.0 + r_passo)).max(0.0);
            if pop.empregados > pop.populacao {
                pop.empregados = pop.populacao;
                precisou_ajuste_mortalidade = true;
            }
            pop_total += pop.populacao;
            soma_sol += pop.populacao * pop.padrao_de_vida;
            if !catalogo.tipos_pop[pop.tipo_idx].subsistencia {
                desemp_nao_subs += pop.desempregados();
            }
        }

        if precisou_ajuste_mortalidade {
            pop_emp_por_tipo_scratch.fill(0.0);
            for pop in &prov.pops {
                pop_emp_por_tipo_scratch[pop.tipo_idx] += pop.empregados;
            }
            ajustar_teto_emprego_predios(prov, pop_emp_por_tipo_scratch);
        }

        let sol_medio = if pop_total > 0.0 {
            soma_sol / pop_total
        } else {
            10.0
        };
        let vagas_abertas = prov.vagas_abertas_cache;
        let taxa_desemp = if pop_total > 1.0 {
            (desemp_nao_subs / pop_total).clamp(0.0, 1.0)
        } else {
            0.0
        };

        prov.atracao_migratoria = sol_medio
            + 8.0 * (vagas_abertas / pop_total.max(1_000.0)).min(1.0)
            + 0.12 * (prov.terra_aravel_livre().min(40) as f64)
            - 10.0 * taxa_desemp;
    }

    // 2. Migração interna dentro de cada país
    for pais in paises {
        if pais.provincias_indices.len() < 2 {
            continue;
        }

        let mut melhor_p_idx = pais.provincias_indices[0];
        let mut melhor_atracao = provincias[melhor_p_idx].atracao_migratoria;
        for &p_idx in &pais.provincias_indices[1..] {
            let atr = provincias[p_idx].atracao_migratoria;
            if atr > melhor_atracao {
                melhor_atracao = atr;
                melhor_p_idx = p_idx;
            }
        }

        lotes_migrantes.clear();
        for &p_orig in &pais.provincias_indices {
            if p_orig == melhor_p_idx {
                continue;
            }
            let dif = melhor_atracao - provincias[p_orig].atracao_migratoria;
            if dif <= 1.5 {
                continue;
            }

            let taxa_mig = (0.00035 * dif.min(12.0) * fator_dias).min(0.02);
            let prov_orig = &mut provincias[p_orig];
            for pop in &mut prov_orig.pops {
                let disponivel = pop.desempregados();
                if disponivel < 20.0 || pop.populacao < 200.0 {
                    continue;
                }
                let qtd_mig = (disponivel * taxa_mig).min(disponivel * 0.25);
                if qtd_mig < 1.0 {
                    continue;
                }

                let fracao_pop = (qtd_mig / pop.populacao).clamp(0.0, 1.0);
                let riq_mig = pop.riqueza_reservas * fracao_pop;
                pop.populacao = (pop.populacao - qtd_mig).max(pop.empregados);
                pop.riqueza_reservas = (pop.riqueza_reservas - riq_mig).max(0.0);

                if let Some(existente) = lotes_migrantes.iter_mut().find(|l| {
                    l.tipo_idx == pop.tipo_idx
                        && l.cultura_idx == pop.cultura_idx
                        && l.religiao_idx == pop.religiao_idx
                }) {
                    let nova_soma = existente.quantidade + qtd_mig;
                    existente.padrao_de_vida = (existente.padrao_de_vida * existente.quantidade
                        + pop.padrao_de_vida * qtd_mig)
                        / nova_soma;
                    existente.alfabetizacao = (existente.alfabetizacao * existente.quantidade
                        + pop.alfabetizacao * qtd_mig)
                        / nova_soma;
                    existente.quantidade = nova_soma;
                    existente.riqueza += riq_mig;
                } else {
                    lotes_migrantes.push(LoteMigrante {
                        tipo_idx: pop.tipo_idx,
                        cultura_idx: pop.cultura_idx,
                        religiao_idx: pop.religiao_idx,
                        quantidade: qtd_mig,
                        riqueza: riq_mig,
                        padrao_de_vida: pop.padrao_de_vida,
                        alfabetizacao: pop.alfabetizacao,
                    });
                }
            }
        }

        if !lotes_migrantes.is_empty() {
            let prov_dest = &mut provincias[melhor_p_idx];
            for &lote in lotes_migrantes.iter() {
                absorver_migrantes_na_provincia(
                    catalogo,
                    nomes_culturas,
                    nomes_religioes,
                    prov_dest,
                    lote,
                );
            }
        }
    }
}

fn absorver_migrantes_na_provincia(
    catalogo: &CatalogoEconomia,
    nomes_culturas: &[String],
    nomes_religioes: &[String],
    prov: &mut Provincia,
    lote: LoteMigrante,
) {
    if let Some(alvo) = prov.pops.iter_mut().find(|p| {
        p.tipo_idx == lote.tipo_idx
            && p.cultura_idx == lote.cultura_idx
            && p.religiao_idx == lote.religiao_idx
    }) {
        let total = alvo.populacao + lote.quantidade;
        if total > 0.0 {
            alvo.padrao_de_vida = (alvo.padrao_de_vida * alvo.populacao
                + lote.padrao_de_vida * lote.quantidade)
                / total;
            alvo.alfabetizacao = (alvo.alfabetizacao * alvo.populacao
                + lote.alfabetizacao * lote.quantidade)
                / total;
        }
        alvo.populacao = total;
        alvo.riqueza_reservas += lote.riqueza;
        return;
    }

    if prov.pops.len() >= 24
        && let Some(alvo_mesmo_tipo) = prov.pops.iter_mut().find(|p| p.tipo_idx == lote.tipo_idx)
    {
        let total = alvo_mesmo_tipo.populacao + lote.quantidade;
        if total > 0.0 {
            alvo_mesmo_tipo.padrao_de_vida = (alvo_mesmo_tipo.padrao_de_vida
                * alvo_mesmo_tipo.populacao
                + lote.padrao_de_vida * lote.quantidade)
                / total;
            alvo_mesmo_tipo.alfabetizacao = (alvo_mesmo_tipo.alfabetizacao
                * alvo_mesmo_tipo.populacao
                + lote.alfabetizacao * lote.quantidade)
                / total;
        }
        alvo_mesmo_tipo.populacao = total;
        alvo_mesmo_tipo.riqueza_reservas += lote.riqueza;
        return;
    }

    prov.pops.push(Pop {
        tipo_pop_id: catalogo.tipos_pop[lote.tipo_idx].id.clone(),
        cultura: nomes_culturas
            .get(lote.cultura_idx as usize)
            .cloned()
            .unwrap_or_default(),
        religiao: nomes_religioes
            .get(lote.religiao_idx as usize)
            .cloned()
            .unwrap_or_default(),
        populacao: lote.quantidade,
        empregados: 0.0,
        riqueza_reservas: lote.riqueza,
        padrao_de_vida: lote.padrao_de_vida,
        alfabetizacao: lote.alfabetizacao,
        renda_ultimo_tick: 0.0,
        gasto_consumo_ultimo_tick: 0.0,
        tipo_idx: lote.tipo_idx,
        cultura_idx: lote.cultura_idx,
        religiao_idx: lote.religiao_idx,
    });
}

fn ajustar_teto_emprego_predios(prov: &mut Provincia, pop_viva_emp_por_tipo: &[f64]) {
    for (tp_idx, &limite_vivo) in pop_viva_emp_por_tipo.iter().enumerate() {
        let mut soma_predios = 0.0;
        for b in &prov.predios {
            for slot in b.cache.vagas() {
                if slot.tipo_pop_idx as usize == tp_idx {
                    soma_predios += slot.empregados;
                }
            }
        }
        if soma_predios > limite_vivo + 1e-6 && soma_predios > 0.0 {
            let fator = (limite_vivo / soma_predios).clamp(0.0, 1.0);
            for b in &mut prov.predios {
                for slot in b.cache.vagas_mut() {
                    if slot.tipo_pop_idx as usize == tp_idx {
                        slot.empregados *= fator;
                    }
                }
            }
        }
    }
}
