//! Fase (3): Contabilidade dos prédios, pagamento de salários, formação de lucros/caixas
//! e distribuição de dividendos (capitalistas, aristocratas, estatais e cooperativas).

use crate::catalogo::CatalogoEconomia;
use crate::entidades::{Mercado, Pais, Provincia};
use crate::motor::governo;
use crate::tipos::{CategoriaPredio, RegimePropriedade};

/// Capacidade máxima de reserva de caixa operacional por nível de prédio.
pub const RESERVA_CAIXA_POR_NIVEL: f64 = 5_000.0;

/// Executa o fechamento operacional diário de todos os prédios, creditando salários por profissão,
/// reservas de caixa, gastos governamentais, fundo de investimento privado e pools de dividendos.
#[allow(clippy::too_many_arguments)]
pub(crate) fn executar_distribuicao_rendas(
    catalogo: &CatalogoEconomia,
    mercados: &[Mercado],
    paises: &mut [Pais],
    provincias: &mut [Provincia],
    salarios_prov_tipo: &mut [f64],
    dividendos_cap_prov: &mut [f64],
    dividendos_ari_prov: &mut [f64],
    dividendos_cap_pais: &mut [f64],
    dividendos_ari_pais: &mut [f64],
    pop_cap_pais: &mut [f64],
    pop_ari_pais: &mut [f64],
    reservas_predios_pais: &mut [f64],
) {
    let num_tipos_pop = catalogo.tipos_pop.len();
    salarios_prov_tipo.fill(0.0);
    dividendos_cap_prov.fill(0.0);
    dividendos_ari_prov.fill(0.0);
    dividendos_cap_pais.fill(0.0);
    dividendos_ari_pais.fill(0.0);
    pop_cap_pais.fill(0.0);
    pop_ari_pais.fill(0.0);
    reservas_predios_pais.fill(0.0);

    for (p_idx, prov) in provincias.iter_mut().enumerate() {
        let c_idx = prov.pais_idx;
        let m_idx = prov.mercado_idx;
        let precos = &mercados[m_idx].precos_idx;
        let offset_sal = p_idx * num_tipos_pop;

        let mut pop_cap_local = 0.0;
        let mut pop_ari_local = 0.0;
        for pop in &prov.pops {
            let tp = &catalogo.tipos_pop[pop.tipo_idx];
            if tp.recebe_dividendos_capitalista {
                pop_cap_local += pop.populacao;
                pop_cap_pais[c_idx] += pop.populacao;
            }
            if tp.recebe_dividendos_aristocrata {
                pop_ari_local += pop.populacao;
                pop_ari_pais[c_idx] += pop.populacao;
            }
        }

        let mult_gov = paises[c_idx]
            .politica_fiscal
            .multiplicador_salario_governo
            .clamp(0.4, 2.0);
        let mult_mil = paises[c_idx]
            .politica_fiscal
            .multiplicador_salario_militar
            .clamp(0.4, 2.0);
        let exige_estatal = paises[c_idx].exige_propriedade_estatal_lei;
        let em_calote = paises[c_idx].orcamento.em_calote;
        let imp_rural_nivel = paises[c_idx]
            .politica_fiscal
            .imposto_sobre_terra_por_nivel_rural
            .max(0.0);
        let efic_arrec = paises[c_idx].eficiencia_arrecadacao;

        let mut reservas_prov = 0.0;

        for b in &mut prov.predios {
            let escala = b.cache.escala_efetiva_ultimo_tick;
            if b.nivel == 0 || escala <= 0.0 {
                b.receita_ultimo_tick = 0.0;
                b.custo_insumos_ultimo_tick = 0.0;
                b.folha_salarial_ultimo_tick = 0.0;
                b.lucro_ultimo_tick = 0.0;
                b.dividendos_ultimo_tick = 0.0;
                b.cache.margem_lucro_ultimo_tick = 0.0;
                reservas_prov += b.reservas_caixa;
                continue;
            }

            let mut custo_insumos = 0.0;
            for &(g_in, qtd) in b.cache.insumos() {
                custo_insumos += escala * qtd * precos[g_in as usize];
            }

            let mut receita = 0.0;
            for &(g_out, qtd) in b.cache.produtos() {
                receita += escala * qtd * precos[g_out as usize];
            }

            b.custo_insumos_ultimo_tick = custo_insumos;
            b.receita_ultimo_tick = receita;

            let categoria = catalogo.tipos_predio[b.cache.tipo_idx].categoria;

            if b.cache.financiado_pelo_governo {
                let mult_sal_setor = match categoria {
                    CategoriaPredio::Militar => mult_mil,
                    CategoriaPredio::Construcao if em_calote => 0.5,
                    _ => mult_gov,
                };

                let mut folha_total = 0.0;
                for slot in b.cache.vagas() {
                    let tp_idx = slot.tipo_pop_idx as usize;
                    let mult_tp = catalogo.tipos_pop[tp_idx].multiplicador_salario;
                    let sal = slot.empregados * b.salario_base * mult_tp * mult_sal_setor;
                    salarios_prov_tipo[offset_sal + tp_idx] += sal;
                    folha_total += sal;
                }

                let despesa_setor = custo_insumos + folha_total;
                match categoria {
                    CategoriaPredio::Militar => {
                        paises[c_idx].orcamento.gasto_militar += despesa_setor;
                    }
                    CategoriaPredio::Construcao => {
                        paises[c_idx].orcamento.gasto_construcao += despesa_setor;
                    }
                    _ => {
                        paises[c_idx].orcamento.gasto_burocracia += despesa_setor;
                    }
                }

                b.folha_salarial_ultimo_tick = folha_total;
                b.lucro_ultimo_tick = 0.0;
                b.dividendos_ultimo_tick = 0.0;
                b.cache.margem_lucro_ultimo_tick = 0.0;
                paises[c_idx].pib_diario += folha_total;
                reservas_prov += b.reservas_caixa;
                continue;
            }

            let valor_adicionado = receita - custo_insumos;
            paises[c_idx].pib_diario += valor_adicionado.max(0.0);

            let mut folha_alvo = 0.0;
            for slot in b.cache.vagas() {
                let mult_tp = catalogo.tipos_pop[slot.tipo_pop_idx as usize].multiplicador_salario;
                folha_alvo += slot.empregados * b.salario_base * mult_tp;
            }

            let margem = if folha_alvo > 1e-9 {
                (valor_adicionado - folha_alvo) / folha_alvo
            } else {
                0.0
            };
            b.cache.margem_lucro_ultimo_tick = margem;

            let regime_efetivo =
                if exige_estatal && b.cache.regime_propriedade != RegimePropriedade::Cooperativo {
                    RegimePropriedade::Estatal
                } else {
                    b.cache.regime_propriedade
                };

            let reserva_max = (b.nivel as f64) * RESERVA_CAIXA_POR_NIVEL;
            let mut folha_efetiva = folha_alvo;
            let mut dividendos = 0.0;

            if valor_adicionado >= folha_alvo {
                let lucro = valor_adicionado - folha_alvo;
                b.lucro_ultimo_tick = lucro;

                let espaco_caixa = (reserva_max - b.reservas_caixa).max(0.0);
                let retencao_caixa = (lucro * 0.35).min(espaco_caixa);
                b.reservas_caixa += retencao_caixa;
                dividendos = (lucro - retencao_caixa).max(0.0);
            } else {
                let deficit = folha_alvo - valor_adicionado;
                b.lucro_ultimo_tick = -deficit;

                let saque_caixa = deficit.min(b.reservas_caixa);
                b.reservas_caixa = (b.reservas_caixa - saque_caixa).max(0.0);
                let descoberto = (deficit - saque_caixa).max(0.0);

                if descoberto > 0.0 && b.subsidiado && !em_calote {
                    paises[c_idx].orcamento.gasto_subsidios += descoberto;
                    folha_efetiva = folha_alvo;
                } else if descoberto > 0.0 {
                    folha_efetiva = (valor_adicionado + saque_caixa).clamp(0.0, folha_alvo);
                }
            }

            let razao_pagamento = if folha_alvo <= 1e-9 {
                0.0
            } else {
                (folha_efetiva / folha_alvo).clamp(0.0, 1.0)
            };
            for slot in b.cache.vagas() {
                let tp_idx = slot.tipo_pop_idx as usize;
                let mult_tp = catalogo.tipos_pop[tp_idx].multiplicador_salario;
                let sal = slot.empregados * b.salario_base * mult_tp * razao_pagamento;
                salarios_prov_tipo[offset_sal + tp_idx] += sal;
            }
            b.folha_salarial_ultimo_tick = folha_efetiva;

            if categoria == CategoriaPredio::RuralAgricola
                && regime_efetivo != RegimePropriedade::Estatal
                && imp_rural_nivel > 0.0
                && b.reservas_caixa > 0.0
            {
                let trib_terra = ((b.nivel as f64) * imp_rural_nivel).min(b.reservas_caixa * 0.20);
                b.reservas_caixa = (b.reservas_caixa - trib_terra).max(0.0);
                paises[c_idx].orcamento.receita_imposto_terra += trib_terra * efic_arrec;
            }

            reservas_prov += b.reservas_caixa;
            b.dividendos_ultimo_tick = dividendos;
            if dividendos > 0.0 {
                let (taxa_cap_base, taxa_ari_base, _) = governo::parametros_sistema_economico(
                    &paises[c_idx].politica_fiscal.lei_sistema_economico_id,
                );
                // Saturação suave baseada na escala do PIB anual do país (sem teto artificial de 250k)
                let cap_escala = (paises[c_idx].pib_diario * 365.0).max(500_000.0);
                let saturacao = (paises[c_idx].fundo_investimento_privado / (cap_escala * 2.0)).clamp(0.0, 1.0);

                match regime_efetivo {
                    RegimePropriedade::PrivadoCapitalista => {
                        let taxa_inv = (taxa_cap_base * (1.0 - 0.40 * saturacao)).max(0.0);
                        let aporte_inv = dividendos * taxa_inv;
                        let div_liquido = dividendos - aporte_inv;
                        paises[c_idx].fundo_investimento_privado += aporte_inv;
                        paises[c_idx].aporte_fundo_privado_ultimo_tick += aporte_inv;
                        if pop_cap_local > 0.0 {
                            dividendos_cap_prov[p_idx] += div_liquido * 0.70;
                            dividendos_cap_pais[c_idx] += div_liquido * 0.30;
                        } else {
                            dividendos_cap_pais[c_idx] += div_liquido;
                        }
                    }
                    RegimePropriedade::Aristocratico => {
                        let taxa_inv = (taxa_ari_base * (1.0 - 0.40 * saturacao)).max(0.0);
                        let aporte_inv = dividendos * taxa_inv;
                        let div_liquido = dividendos - aporte_inv;
                        paises[c_idx].fundo_investimento_privado += aporte_inv;
                        paises[c_idx].aporte_fundo_privado_ultimo_tick += aporte_inv;
                        if pop_ari_local > 0.0 {
                            dividendos_ari_prov[p_idx] += div_liquido * 0.75;
                            dividendos_ari_pais[c_idx] += div_liquido * 0.25;
                        } else {
                            dividendos_ari_pais[c_idx] += div_liquido;
                        }
                    }
                    RegimePropriedade::Estatal => {
                        paises[c_idx].orcamento.receita_dividendos_estatais += dividendos;
                    }
                    RegimePropriedade::Cooperativo => {
                        if folha_alvo > 1e-9 {
                            for slot in b.cache.vagas() {
                                let tp_idx = slot.tipo_pop_idx as usize;
                                let mult_tp = catalogo.tipos_pop[tp_idx].multiplicador_salario;
                                let peso =
                                    (slot.empregados * b.salario_base * mult_tp) / folha_alvo;
                                salarios_prov_tipo[offset_sal + tp_idx] += dividendos * peso;
                            }
                        }
                    }
                }
            }
        }

        reservas_predios_pais[c_idx] += reservas_prov;
    }

    for (c_idx, pais) in paises.iter_mut().enumerate() {
        if pop_cap_pais[c_idx] <= 0.0 && dividendos_cap_pais[c_idx] > 0.0 {
            if pais.exige_propriedade_estatal_lei {
                pais.orcamento.receita_dividendos_estatais += dividendos_cap_pais[c_idx];
            } else {
                pais.fundo_investimento_privado += dividendos_cap_pais[c_idx];
                pais.aporte_fundo_privado_ultimo_tick += dividendos_cap_pais[c_idx];
            }
            dividendos_cap_pais[c_idx] = 0.0;
        }
        if pop_ari_pais[c_idx] <= 0.0 && dividendos_ari_pais[c_idx] > 0.0 {
            if pais.exige_propriedade_estatal_lei {
                pais.orcamento.receita_dividendos_estatais += dividendos_ari_pais[c_idx];
            } else {
                pais.fundo_investimento_privado += dividendos_ari_pais[c_idx];
                pais.aporte_fundo_privado_ultimo_tick += dividendos_ari_pais[c_idx];
            }
            dividendos_ari_pais[c_idx] = 0.0;
        }
    }
}
