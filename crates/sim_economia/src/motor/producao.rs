//! Fase (1): Produção diária dos prédios segundo métodos ativos, ocupação,
//! economia de escala, acesso ao mercado e disponibilidade de insumos.

use crate::catalogo::CatalogoEconomia;
use crate::entidades::{ConfigSimulacao, Estado, Mercado, Pais, Provincia};
use crate::tipos::{CategoriaBem, CategoriaPredio};

/// Executa a produção física de todos os prédios e fazendas de subsistência em passagem única,
/// atualizando também acesso ao mercado dos estados e eficiência burocrática dos países.
#[allow(clippy::too_many_arguments)]
pub(crate) fn executar_producao_diaria(
    catalogo: &CatalogoEconomia,
    config: &ConfigSimulacao,
    mercados: &[Mercado],
    estados: &mut [Estado],
    paises: &mut [Pais],
    provincias: &mut [Provincia],
    bens_alimento_basico: &[usize],
    proxima_oferta: &mut [f64],
    proxima_demanda: &mut [f64],
    pontos_construcao_setores_pais: &mut [f64],
    infra_gerada_estado: &mut [f64],
    infra_usada_estado: &mut [f64],
    pop_incorporada_pais: &mut [f64],
) {
    let num_bens = catalogo.bens.len();
    proxima_oferta.fill(0.0);
    proxima_demanda.fill(0.0);
    pontos_construcao_setores_pais.fill(0.0);
    pop_incorporada_pais.fill(0.0);

    for (e_idx, est) in estados.iter().enumerate() {
        infra_gerada_estado[e_idx] = est.infraestrutura_bonus;
        infra_usada_estado[e_idx] = 0.0;
    }

    // 1. Resetar acumuladores diários dos países e estimar utilização da fila de construção
    for pais in paises.iter_mut() {
        pais.orcamento.receita_imposto_renda = 0.0;
        pais.orcamento.receita_imposto_consumo = 0.0;
        pais.orcamento.receita_imposto_terra = 0.0;
        pais.orcamento.receita_dividendos_estatais = 0.0;
        pais.orcamento.receita_tarifas = 0.0;
        pais.orcamento.gasto_burocracia = 0.0;
        pais.orcamento.gasto_militar = 0.0;
        pais.orcamento.gasto_construcao = 0.0;
        pais.orcamento.gasto_subsidios = 0.0;
        pais.orcamento.gasto_juros = 0.0;
        pais.orcamento.saldo_diario = 0.0;

        // Demanda por materiais das obras do tick anterior (cimento, aço, máquinas...)
        let off_obras = pais.mercado_idx * num_bens;
        for (g, q) in pais.demanda_obras.iter().enumerate() {
            proxima_demanda[off_obras + g] += *q;
        }

        pais.burocracia_produzida = 15.0;
        pais.forca_militar_total = 0.0;
        pais.pib_diario = 0.0;

        let obras_ativas = pais
            .fila_construcao
            .iter()
            .filter(|item| item.privado || !pais.orcamento.em_calote)
            .count();
        pais.utilizacao_construcao_ultimo_tick = if obras_ativas == 0 {
            0.0
        } else {
            let demanda_pontos = (obras_ativas as f64) * config.pontos_maximos_por_obra_dia;
            if demanda_pontos <= pais.pontos_construcao_base {
                0.15
            } else {
                1.0
            }
        };
    }

    // 2. Passagem linear única sobre todas as províncias, pops de subsistência e prédios
    let num_alimentos = bens_alimento_basico.len();
    for prov in provincias.iter_mut() {
        let m_idx = prov.mercado_idx;
        let c_idx = prov.pais_idx;
        let e_idx = prov.estado_idx;
        let acesso = estados[e_idx].acesso_mercado;
        let est_inc = estados[e_idx].incorporado;
        let offset_mercado = m_idx * num_bens;

        infra_gerada_estado[e_idx] += prov.infraestrutura_base;

        let mut pop_prov = 0.0;
        let mut camponeses_ativos = 0.0;
        for pop in &prov.pops {
            pop_prov += pop.populacao;
            if catalogo.tipos_pop[pop.tipo_idx].subsistencia {
                camponeses_ativos += pop.desempregados();
            }
        }
        if est_inc {
            pop_incorporada_pais[c_idx] += pop_prov;
        }

        // 2a. Produção excedente das terras de subsistência
        if num_alimentos > 0 && camponeses_ativos > 0.0 && prov.terra_aravel_livre() > 0 {
            let cap_camponeses = (prov.terra_aravel_livre() as f64) * 5_000.0;
            let efetivos = camponeses_ativos.min(cap_camponeses);
            let excedente_total = (efetivos / 1_000.0) * 0.30 * acesso;
            let por_bem = excedente_total / (num_alimentos as f64);
            for &g_idx in bens_alimento_basico {
                proxima_oferta[offset_mercado + g_idx] += por_bem;
            }
        }

        // 2b. Prédios formais
        let util_construcao = paises[c_idx].utilizacao_construcao_ultimo_tick;
        let sup_mercado = &mercados[m_idx].fator_suprimento_idx;
        let mut vagas_abertas_prov = 0.0;

        for b in &mut prov.predios {
            if b.nivel == 0 {
                b.cache.escala_efetiva_ultimo_tick = 0.0;
                continue;
            }
            let vagas_totais = b.total_vagas();
            let emp_totais = b.total_empregados();
            if vagas_totais > emp_totais {
                vagas_abertas_prov += vagas_totais - emp_totais;
            }
            let r_emp = if vagas_totais <= 0.0 {
                1.0
            } else {
                (emp_totais / vagas_totais).clamp(0.0, 1.0)
            };
            if r_emp <= 1e-7 {
                b.cache.escala_efetiva_ultimo_tick = 0.0;
                continue;
            }

            let nivel_f = b.nivel as f64;
            let escala_inst = nivel_f * r_emp;
            let categoria = catalogo.tipos_predio[b.cache.tipo_idx].categoria;

            infra_gerada_estado[e_idx] += escala_inst * b.cache.infraestrutura_por_nivel;
            let peso_uso = match categoria {
                CategoriaPredio::Industrial | CategoriaPredio::Extrativo => 1.5,
                CategoriaPredio::RuralAgricola => 0.8,
                CategoriaPredio::Infraestrutura => 0.1,
                _ => 0.5,
            };
            infra_usada_estado[e_idx] += escala_inst * peso_uso;

            let mut fator_insumos = 1.0_f64;
            for &(g_in, _) in b.cache.insumos() {
                fator_insumos = fator_insumos.min(sup_mercado[g_in as usize]);
            }

            let mult_setor = if categoria == CategoriaPredio::Construcao {
                0.10 + 0.90 * util_construcao
            } else {
                1.0
            };

            let econ_escala = 1.0 + 0.01 * (b.nivel.saturating_sub(1).min(25) as f64);
            let escala = escala_inst * econ_escala * acesso * fator_insumos * mult_setor;

            b.cache.escala_efetiva_ultimo_tick = escala;
            b.cache.fator_insumos_ultimo_tick = fator_insumos;

            for &(g_in, qtd) in b.cache.insumos() {
                proxima_demanda[offset_mercado + (g_in as usize)] += escala * qtd;
            }
            for &(g_out, qtd) in b.cache.produtos() {
                proxima_oferta[offset_mercado + (g_out as usize)] += escala * qtd;
            }

            let escala_cap = escala_inst * fator_insumos;
            if b.cache.pontos_construcao_por_nivel > 0.0 {
                pontos_construcao_setores_pais[c_idx] +=
                    escala_cap * b.cache.pontos_construcao_por_nivel * mult_setor;
            }
            if b.cache.burocracia_por_nivel > 0.0 {
                paises[c_idx].burocracia_produzida += escala_cap * b.cache.burocracia_por_nivel;
            }
            if b.cache.forca_militar_por_nivel > 0.0 {
                paises[c_idx].forca_militar_total += escala_cap * b.cache.forca_militar_por_nivel;
            }
        }
        prov.vagas_abertas_cache = vagas_abertas_prov;
    }

    // 3. Consolidar acesso ao mercado por Estado (750 escalares)
    for (e_idx, est) in estados.iter_mut().enumerate() {
        let usada = infra_usada_estado[e_idx];
        est.acesso_mercado = if usada <= 1.0 {
            1.0
        } else {
            (infra_gerada_estado[e_idx] / usada).clamp(0.40, 1.0)
        };
    }

    // 4. Consolidar eficiência de arrecadação por País (150 escalares)
    for (c_idx, pais) in paises.iter_mut().enumerate() {
        let estados_inc = pais
            .estados_indices
            .iter()
            .filter(|&&e| estados[e].incorporado)
            .count();
        let mult_educ = 1.0 + 0.30 * (pais.politica_fiscal.nivel_investimento_educacao as f64);
        let nec = 10.0
            + (estados_inc as f64) * 5.0
            + (pop_incorporada_pais[c_idx] / 100_000.0) * 8.0 * mult_educ;
        pais.burocracia_necessaria = nec;
        pais.eficiencia_arrecadacao = if nec <= 0.0 {
            1.0
        } else {
            (pais.burocracia_produzida / nec).clamp(0.40, 1.0)
        };
    }
}

/// Extrai os índices dos bens marcados como `CategoriaBem::AlimentoBasico` no catálogo.
pub(crate) fn listar_bens_alimento_basico(catalogo: &CatalogoEconomia) -> Vec<usize> {
    catalogo
        .bens
        .iter()
        .enumerate()
        .filter_map(|(i, b)| (b.categoria == CategoriaBem::AlimentoBasico).then_some(i))
        .collect()
}
