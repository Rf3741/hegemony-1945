//! Fase (7): Contratação, promoção/qualificação profissional, demissão e ajuste salarial
//! dinâmico dos prédios conforme lucratividade e escassez local de mão de obra.

use crate::catalogo::CatalogoEconomia;
use crate::entidades::{ConfigSimulacao, Estado, Mercado, Pais, Pop, Provincia};

/// Registro compacto (`Copy`, sem alocações `String`) de promoção de trabalhadores.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LotePromocao {
    pub novo_tipo_idx: usize,
    pub cultura_idx: u16,
    pub religiao_idx: u16,
    pub quantidade: f64,
    pub riqueza: f64,
    pub padrao_de_vida: f64,
    pub alfabetizacao: f64,
}

/// Executa o mercado de trabalho local em todas as províncias.
#[allow(clippy::too_many_arguments)]
pub(crate) fn executar_contratacao_e_demissao(
    catalogo: &CatalogoEconomia,
    nomes_culturas: &[String],
    nomes_religioes: &[String],
    config: &ConfigSimulacao,
    mercados: &[Mercado],
    estados: &[Estado],
    paises: &[Pais],
    provincias: &mut [Provincia],
    dias_passo: u32,
    desemp_por_tipo: &mut [f64],
    emp_alvo_por_tipo: &mut [f64],
    pop_total_por_tipo: &mut [f64],
    promocoes: &mut Vec<LotePromocao>,
) {
    let fator_dias = dias_passo.max(1) as f64;
    let taxa_contratacao = (config.taxa_contratacao_diaria * fator_dias).clamp(0.005, 0.35);
    let taxa_demissao = (config.taxa_demissao_diaria * fator_dias).clamp(0.005, 0.25);

    for prov in provincias.iter_mut() {
        if prov.predios.is_empty() {
            continue;
        }
        let m_idx = prov.mercado_idx;
        let c_idx = prov.pais_idx;
        let acesso = estados[prov.estado_idx].acesso_mercado;
        let precos = &mercados[m_idx].precos_idx;
        let em_calote = paises[c_idx].orcamento.em_calote;

        desemp_por_tipo.fill(0.0);
        for pop in &prov.pops {
            if pop.populacao > 0.0 {
                desemp_por_tipo[pop.tipo_idx] += pop.desempregados();
            }
        }

        promocoes.clear();
        let mut houve_mudanca_emprego = false;

        for b in &mut prov.predios {
            if b.nivel == 0 {
                for slot in b.cache.vagas_mut() {
                    if slot.empregados > 0.0 {
                        slot.empregados = 0.0;
                        houve_mudanca_emprego = true;
                    }
                }
                continue;
            }

            let margem = if b.cache.financiado_pelo_governo {
                0.0
            } else if b.cache.escala_efetiva_ultimo_tick > 0.0 {
                b.cache.margem_lucro_ultimo_tick
            } else {
                let econ_escala = 1.0 + 0.01 * (b.nivel.saturating_sub(1).min(25) as f64);
                let fator_prod = econ_escala * acesso * b.cache.fator_insumos_ultimo_tick;

                let mut receita_por_nivel = 0.0;
                for &(g_out, qtd) in b.cache.produtos() {
                    receita_por_nivel += qtd * fator_prod * precos[g_out as usize];
                }
                let mut custo_ins_por_nivel = 0.0;
                for &(g_in, qtd) in b.cache.insumos() {
                    custo_ins_por_nivel += qtd * fator_prod * precos[g_in as usize];
                }
                let mut folha_por_nivel = 0.0;
                for slot in b.cache.vagas() {
                    let mult = catalogo.tipos_pop[slot.tipo_pop_idx as usize].multiplicador_salario;
                    folha_por_nivel += (slot.vagas_por_nivel as f64) * b.salario_base * mult;
                }

                if folha_por_nivel <= 1e-9 {
                    0.0
                } else {
                    (receita_por_nivel - custo_ins_por_nivel - folha_por_nivel) / folha_por_nivel
                }
            };

            let quer_contratar =
                b.cache.financiado_pelo_governo || (b.subsidiado && !em_calote) || margem > 0.015;

            if quer_contratar {
                let nivel_f = b.nivel as f64;
                let mut precisa_contratar = false;
                for slot in b.cache.vagas() {
                    let meta_vagas = (slot.vagas_por_nivel as f64) * nivel_f;
                    if slot.empregados < meta_vagas - 0.1 || slot.empregados > meta_vagas {
                        precisa_contratar = true;
                        break;
                    }
                }
                if !precisa_contratar && margem <= 0.35 {
                    continue;
                }

                let mut faltou_trabalhador = false;
                for slot in b.cache.vagas_mut() {
                    let tp_alvo_idx = slot.tipo_pop_idx as usize;
                    let meta_vagas = (slot.vagas_por_nivel as f64) * nivel_f;
                    if slot.empregados > meta_vagas {
                        let excesso = slot.empregados - meta_vagas;
                        slot.empregados = meta_vagas;
                        desemp_por_tipo[tp_alvo_idx] += excesso;
                        houve_mudanca_emprego = true;
                        continue;
                    }
                    let aberto = meta_vagas - slot.empregados;
                    if aberto <= 0.1 {
                        continue;
                    }

                    let pedido = (meta_vagas * taxa_contratacao).max(1.0).min(aberto);

                    let disp_direto = desemp_por_tipo[tp_alvo_idx];
                    let contratados_direto = pedido.min(disp_direto);
                    if contratados_direto > 0.0 {
                        slot.empregados += contratados_direto;
                        desemp_por_tipo[tp_alvo_idx] = (disp_direto - contratados_direto).max(0.0);
                        houve_mudanca_emprego = true;
                    }

                    let mut restante = pedido - contratados_direto;

                    if restante > 0.5 {
                        let tp_alvo = &catalogo.tipos_pop[tp_alvo_idx];
                        let alfa_req = tp_alvo.alfabetizacao_minima;
                        let mult_alvo = tp_alvo.multiplicador_salario;

                        for pop_doador in &mut prov.pops {
                            if restante <= 0.5 {
                                break;
                            }
                            if pop_doador.tipo_idx == tp_alvo_idx {
                                continue;
                            }
                            let disp_tipo_doador = desemp_por_tipo[pop_doador.tipo_idx];
                            if disp_tipo_doador < 1.0 || pop_doador.alfabetizacao < alfa_req {
                                continue;
                            }
                            let tp_doador = &catalogo.tipos_pop[pop_doador.tipo_idx];
                            if !(tp_doador.subsistencia
                                || tp_doador.multiplicador_salario < mult_alvo)
                            {
                                continue;
                            }

                            let disp_pop = pop_doador.desempregados().min(disp_tipo_doador);
                            if disp_pop < 1.0 {
                                continue;
                            }

                            let promovidos = restante.min(disp_pop * 0.50);
                            if promovidos < 0.5 {
                                continue;
                            }

                            let fracao = (promovidos / pop_doador.populacao).clamp(0.0, 1.0);
                            let riq_transf = pop_doador.riqueza_reservas * fracao;
                            pop_doador.populacao =
                                (pop_doador.populacao - promovidos).max(pop_doador.empregados);
                            pop_doador.riqueza_reservas =
                                (pop_doador.riqueza_reservas - riq_transf).max(0.0);
                            desemp_por_tipo[pop_doador.tipo_idx] =
                                (disp_tipo_doador - promovidos).max(0.0);

                            slot.empregados += promovidos;
                            restante -= promovidos;
                            houve_mudanca_emprego = true;

                            registrar_lote_promocao(
                                promocoes,
                                tp_alvo_idx,
                                pop_doador,
                                promovidos,
                                riq_transf,
                            );
                        }
                    }

                    if restante > pedido * 0.5 {
                        faltou_trabalhador = true;
                    }
                }

                if !b.cache.financiado_pelo_governo {
                    if faltou_trabalhador && margem > 0.12 {
                        b.salario_base =
                            (b.salario_base * (1.0 + 0.004 * fator_dias)).clamp(1.2, 40.0);
                    } else if margem > 0.35 && b.taxa_ocupacao() > 0.95 {
                        b.salario_base =
                            (b.salario_base * (1.0 + 0.0015 * fator_dias)).clamp(1.2, 40.0);
                    }
                }
            } else if !b.cache.financiado_pelo_governo && !b.subsidiado && margem < -0.03 {
                b.salario_base = (b.salario_base * (1.0 - 0.005 * fator_dias)).clamp(1.2, 40.0);

                let colchao_minimo = (b.nivel as f64) * 300.0;
                if b.reservas_caixa < colchao_minimo && margem < -0.05 {
                    for slot in b.cache.vagas_mut() {
                        let demitidos = slot.empregados * taxa_demissao;
                        if demitidos > 0.0 {
                            slot.empregados = (slot.empregados - demitidos).max(0.0);
                            desemp_por_tipo[slot.tipo_pop_idx as usize] += demitidos;
                            houve_mudanca_emprego = true;
                        }
                    }
                }
            }
        }

        if !promocoes.is_empty() {
            for &lote in promocoes.iter() {
                aplicar_lote_promocao(catalogo, nomes_culturas, nomes_religioes, prov, lote);
            }
            houve_mudanca_emprego = true;
        }

        if houve_mudanca_emprego {
            reconciliar_empregados_provincia(prov, emp_alvo_por_tipo, pop_total_por_tipo);
        }
    }
}

fn registrar_lote_promocao(
    promocoes: &mut Vec<LotePromocao>,
    novo_tipo_idx: usize,
    origem: &Pop,
    quantidade: f64,
    riqueza: f64,
) {
    if let Some(l) = promocoes.iter_mut().find(|l| {
        l.novo_tipo_idx == novo_tipo_idx
            && l.cultura_idx == origem.cultura_idx
            && l.religiao_idx == origem.religiao_idx
    }) {
        let total = l.quantidade + quantidade;
        l.padrao_de_vida =
            (l.padrao_de_vida * l.quantidade + origem.padrao_de_vida * quantidade) / total;
        l.alfabetizacao =
            (l.alfabetizacao * l.quantidade + origem.alfabetizacao * quantidade) / total;
        l.quantidade = total;
        l.riqueza += riqueza;
    } else {
        promocoes.push(LotePromocao {
            novo_tipo_idx,
            cultura_idx: origem.cultura_idx,
            religiao_idx: origem.religiao_idx,
            quantidade,
            riqueza,
            padrao_de_vida: origem.padrao_de_vida,
            alfabetizacao: origem.alfabetizacao,
        });
    }
}

fn aplicar_lote_promocao(
    catalogo: &CatalogoEconomia,
    nomes_culturas: &[String],
    nomes_religioes: &[String],
    prov: &mut Provincia,
    lote: LotePromocao,
) {
    if let Some(alvo) = prov.pops.iter_mut().find(|p| {
        p.tipo_idx == lote.novo_tipo_idx
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
        && let Some(alvo_tipo) = prov
            .pops
            .iter_mut()
            .find(|p| p.tipo_idx == lote.novo_tipo_idx)
    {
        let total = alvo_tipo.populacao + lote.quantidade;
        if total > 0.0 {
            alvo_tipo.padrao_de_vida = (alvo_tipo.padrao_de_vida * alvo_tipo.populacao
                + lote.padrao_de_vida * lote.quantidade)
                / total;
            alvo_tipo.alfabetizacao = (alvo_tipo.alfabetizacao * alvo_tipo.populacao
                + lote.alfabetizacao * lote.quantidade)
                / total;
        }
        alvo_tipo.populacao = total;
        alvo_tipo.riqueza_reservas += lote.riqueza;
        return;
    }

    prov.pops.push(Pop {
        tipo_pop_id: catalogo.tipos_pop[lote.novo_tipo_idx].id.clone(),
        cultura: nomes_culturas
            .get(lote.cultura_idx as usize)
            .cloned()
            .unwrap_or_default(),
        religiao: nomes_religioes
            .get(lote.religiao_idx as usize)
            .cloned()
            .unwrap_or_default(),
        populacao: lote.quantidade,
        empregados: lote.quantidade,
        riqueza_reservas: lote.riqueza,
        padrao_de_vida: lote.padrao_de_vida,
        alfabetizacao: lote.alfabetizacao,
        renda_ultimo_tick: 0.0,
        gasto_consumo_ultimo_tick: 0.0,
        tipo_idx: lote.novo_tipo_idx,
        cultura_idx: lote.cultura_idx,
        religiao_idx: lote.religiao_idx,
    });
}

pub(crate) fn reconciliar_empregados_provincia(
    prov: &mut Provincia,
    emp_alvo_por_tipo: &mut [f64],
    pop_total_por_tipo: &mut [f64],
) {
    emp_alvo_por_tipo.fill(0.0);
    pop_total_por_tipo.fill(0.0);

    for b in &prov.predios {
        if b.nivel > 0 {
            for slot in b.cache.vagas() {
                emp_alvo_por_tipo[slot.tipo_pop_idx as usize] += slot.empregados;
            }
        }
    }

    for pop in &prov.pops {
        if pop.populacao > 0.0 {
            pop_total_por_tipo[pop.tipo_idx] += pop.populacao;
        }
    }

    let mut precisou_corte = false;
    for (tp_idx, &alvo) in emp_alvo_por_tipo.iter().enumerate() {
        let disp = pop_total_por_tipo[tp_idx];
        if alvo > disp + 1e-6 {
            precisou_corte = true;
            let fator = if alvo > 0.0 {
                (disp / alvo).clamp(0.0, 1.0)
            } else {
                0.0
            };
            for b in &mut prov.predios {
                for slot in b.cache.vagas_mut() {
                    if slot.tipo_pop_idx as usize == tp_idx {
                        slot.empregados *= fator;
                    }
                }
            }
        }
    }

    if precisou_corte {
        emp_alvo_por_tipo.fill(0.0);
        for b in &prov.predios {
            if b.nivel > 0 {
                for slot in b.cache.vagas() {
                    emp_alvo_por_tipo[slot.tipo_pop_idx as usize] += slot.empregados;
                }
            }
        }
    }

    for pop in &mut prov.pops {
        let total_tp = pop_total_por_tipo[pop.tipo_idx];
        if pop.populacao <= 0.0 || total_tp <= 1e-9 {
            pop.empregados = 0.0;
        } else {
            let quota = emp_alvo_por_tipo[pop.tipo_idx] * (pop.populacao / total_tp);
            pop.empregados = quota.clamp(0.0, pop.populacao);
        }
    }
}
