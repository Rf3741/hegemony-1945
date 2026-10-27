//! Fase (2): Formação de preços por oferta e demanda, fator de escassez física
//! e rotas comerciais de importação/exportação entre mercados.

use crate::catalogo::CatalogoEconomia;
use crate::entidades::{BlocoComercial, ConfigSimulacao, Mercado, Pais, RotaComercial};
use crate::tipos::FormulaPreco;

/// Calcula o preço instantâneo de equilíbrio de um bem a partir de `preco_base`, `oferta` e `demanda`.
///
/// ### Fórmulas suportadas
/// - [`FormulaPreco::Classica`]:
///   $$P = P_{\text{base}} \cdot \text{clamp}\left(1 + \frac{D - O}{\max(O, D)}, \; m_{\min}, \; m_{\max}\right)$$
///   (retornando $P_{\text{base}}$ quando $O = D = 0$, evitando `0.0 / 0.0 = NaN`).
/// - [`FormulaPreco::VictoriaSuavizada`]:
///   $$P = P_{\text{base}} \cdot \text{clamp}\left(1 + A \cdot \frac{D - O}{\max(O, D, V_{\min})}, \; m_{\min}, \; m_{\max}\right)$$
///   onde $A = \max(m_{\max} - 1, 1 - m_{\min})$ (por padrão `0.75`) mapeia bijetivamente o desequilíbrio
///   relativo $[-1, +1]$ sobre $[0.25, 1.75]$, e $V_{\min}$ evita hipersensibilidade em volumes residuais.
#[inline]
pub fn calcular_preco_alvo(
    preco_base: f64,
    oferta: f64,
    demanda: f64,
    config: &ConfigSimulacao,
) -> f64 {
    let o = oferta.max(0.0);
    let d = demanda.max(0.0);
    let min_m = config.multiplicador_preco_minimo;
    let max_m = config.multiplicador_preco_maximo;

    let mult = match config.formula_preco {
        FormulaPreco::Classica => {
            let denom = o.max(d);
            if denom <= 1e-9 {
                1.0
            } else {
                (1.0 + (d - o) / denom).clamp(min_m, max_m)
            }
        }
        FormulaPreco::VictoriaSuavizada => {
            let denom = o.max(d).max(config.volume_minimo_mercado.max(1e-6));
            if (d - o).abs() <= 1e-12 {
                1.0
            } else {
                let alcance = (max_m - 1.0).max(1.0 - min_m);
                (1.0 + alcance * (d - o) / denom).clamp(min_m, max_m)
            }
        }
    };

    preco_base * mult
}

/// Processa as rotas de importação/exportação entre mercados, ajustando volumes automáticos,
/// injetando ordens de compra (na origem) e venda (no destino) e arrecadando tarifas.
pub(crate) fn processar_rotas_comerciais(
    catalogo: &CatalogoEconomia,
    mercados: &[Mercado],
    paises: &mut [Pais],
    rotas: &mut [RotaComercial],
    blocos: &[BlocoComercial],
    proxima_oferta: &mut [f64],
    proxima_demanda: &mut [f64],
    importacoes_liquidas: &mut [f64],
) {
    let num_bens = catalogo.bens.len();
    if num_bens == 0 {
        return;
    }

    for rota in rotas.iter_mut() {
        let g = rota.bem_idx;
        if !catalogo.bens[g].comercializavel || rota.origem_idx == rota.destino_idx {
            rota.volume_atual = 0.0;
            rota.lucro_ultimo_tick = 0.0;
            continue;
        }

        let m_orig = rota.origem_idx;
        let m_dest = rota.destino_idx;
        let p_orig = mercados[m_orig].precos_idx[g];
        let p_dest = mercados[m_dest].precos_idx[g];
        let preco_base = catalogo.bens[g].preco_base;
        let bem_id = &catalogo.bens[g].id;

        let pais_lider_orig = mercados[m_orig].pais_lider_idx;
        let pais_lider_dest = mercados[m_dest].pais_lider_idx;
        let id_orig = &paises[pais_lider_orig].id;
        let id_dest = &paises[pais_lider_dest].id;

        // Acordo comercial bilateral entre os líderes ou operador da rota
        let tem_acordo = paises[pais_lider_orig].acordos_comerciais.iter().any(|id| id == id_dest)
            || paises[pais_lider_dest].acordos_comerciais.iter().any(|id| id == id_orig)
            || paises[rota.pais_operador_idx].acordos_comerciais.iter().any(|id| id == id_orig || id == id_dest);

        // Blocos comerciais (ex.: Bloco Soviético fechado até COMECON)
        let bloco_orig = blocos.iter().find(|b| b.membros.iter().any(|m| m == id_orig));
        let bloco_dest = blocos.iter().find(|b| b.membros.iter().any(|m| m == id_dest));

        let mesmo_bloco_comecon = match (bloco_orig, bloco_dest) {
            (Some(bo), Some(bd)) => bo.id == bd.id && bo.comecon_ativo,
            _ => false,
        };

        let orig_fechada = bloco_orig.map_or(false, |b| b.fechado && bloco_dest.map_or(true, |bd| bd.id != b.id));
        let dest_fechada = bloco_dest.map_or(false, |b| b.fechado && bloco_orig.map_or(true, |bo| bo.id != b.id));

        // Mercados fechados de um bloco recusam rotas externas se não houver acordo bilateral
        if (orig_fechada || dest_fechada) && !tem_acordo {
            rota.volume_atual = 0.0;
            rota.lucro_ultimo_tick = 0.0;
            continue;
        }

        let mut tarifa_exp = paises[pais_lider_orig]
            .politica_fiscal
            .tarifa_exportacao_bem(bem_id);
        let mut tarifa_imp = paises[pais_lider_dest]
            .politica_fiscal
            .tarifa_importacao_bem(bem_id);

        if mesmo_bloco_comecon {
            // COMECON ativo: isenção total de tarifas intra-bloco
            tarifa_exp = 0.0;
            tarifa_imp = 0.0;
        } else if tem_acordo {
            // Acordo bilateral reduz tarifas em 85%
            tarifa_exp *= 0.15;
            tarifa_imp *= 0.15;
        }

        let mult_lei_operador = paises[rota.pais_operador_idx].mult_volume_comercio_lei;

        // Custo logístico base de 4% do preço-base por unidade comercializada
        let custo_unitario = p_orig * (1.0 + tarifa_exp) + preco_base * 0.04;
        let receita_liquida_unit = p_dest / (1.0 + tarifa_imp);
        let margem_unitaria = receita_liquida_unit - custo_unitario;

        let bonus_bloco = if mesmo_bloco_comecon { 1.25 } else { 1.0 };
        let oferta_origem_disponivel = proxima_oferta[m_orig * num_bens + g];
        let teto_operacional = (rota.volume_maximo * mult_lei_operador * bonus_bloco)
            .min((oferta_origem_disponivel * 0.45).max(1.0));

        if rota.automatica {
            if margem_unitaria > preco_base * 0.01 {
                let passo = (rota.volume_maximo * 0.05).max(0.5);
                rota.volume_atual = (rota.volume_atual + passo).min(teto_operacional);
            } else if margem_unitaria < 0.0 {
                let passo = (rota.volume_maximo * 0.08).max(0.5);
                rota.volume_atual = (rota.volume_atual - passo).max(0.0);
            } else {
                rota.volume_atual = rota.volume_atual.min(teto_operacional);
            }
        } else {
            rota.volume_atual = rota.volume_atual.clamp(0.0, rota.volume_maximo);
        }

        let vol = rota.volume_atual;
        if vol <= 1e-6 {
            rota.lucro_ultimo_tick = 0.0;
            continue;
        }

        // Compra no mercado exportador, vende no mercado importador
        proxima_demanda[m_orig * num_bens + g] += vol;
        proxima_oferta[m_dest * num_bens + g] += vol;
        importacoes_liquidas[m_orig * num_bens + g] -= vol;
        importacoes_liquidas[m_dest * num_bens + g] += vol;

        // Arrecadação tarifária
        let rec_exp =
            vol * preco_base * tarifa_exp * paises[pais_lider_orig].eficiencia_arrecadacao;
        let rec_imp =
            vol * preco_base * tarifa_imp * paises[pais_lider_dest].eficiencia_arrecadacao;
        paises[pais_lider_orig].orcamento.receita_tarifas += rec_exp;
        paises[pais_lider_dest].orcamento.receita_tarifas += rec_imp;

        rota.lucro_ultimo_tick = vol * margem_unitaria;
    }
}

/// Fecha o livro de ofertas e demandas de todos os mercados e atualiza preços e fatores de suprimento.
pub(crate) fn fechar_mercados_e_atualizar_precos(
    catalogo: &CatalogoEconomia,
    config: &ConfigSimulacao,
    mercados: &mut [Mercado],
    proxima_oferta: &[f64],
    proxima_demanda: &[f64],
    importacoes_liquidas: &[f64],
) {
    let num_bens = catalogo.bens.len();
    let alfa = match config.formula_preco {
        FormulaPreco::Classica => 1.0,
        FormulaPreco::VictoriaSuavizada => config.fator_suavizacao_preco.clamp(0.01, 1.0),
    };

    for (m_idx, mercado) in mercados.iter_mut().enumerate() {
        let base_offset = m_idx * num_bens;
        for g in 0..num_bens {
            let o = proxima_oferta[base_offset + g].max(0.0);
            let d = proxima_demanda[base_offset + g].max(0.0);
            let imp = importacoes_liquidas[base_offset + g];
            let preco_base = catalogo.bens[g].preco_base;

            mercado.oferta_idx[g] = o;
            mercado.demanda_idx[g] = d;
            mercado.importacoes_liquidas_idx[g] = imp;

            let alvo = calcular_preco_alvo(preco_base, o, d, config);
            let anterior = mercado.precos_idx[g];
            let novo_preco = (anterior + alfa * (alvo - anterior)).clamp(
                preco_base * config.multiplicador_preco_minimo,
                preco_base * config.multiplicador_preco_maximo,
            );
            mercado.precos_idx[g] = novo_preco;

            // Fator de abastecimento físico: quando D > 2 * O há escassez severa de insumos.
            // Piso em 0.15 evita travamento circular (deadlock carvão <-> ferramentas).
            let suprimento_alvo = if d <= 1e-6 || o * 2.0 >= d {
                1.0
            } else {
                ((2.0 * o) / d).clamp(0.15, 1.0)
            };
            let sup_ant = mercado.fator_suprimento_idx[g];
            mercado.fator_suprimento_idx[g] =
                (sup_ant + 0.30 * (suprimento_alvo - sup_ant)).clamp(0.15, 1.0);
        }
    }
}

/// Atualiza os preços de um `Mercado` diretamente a partir de seus mapas `ordens_venda` e `ordens_compra`.
/// Útil para testes unitários isolados de formação de preço.
pub fn atualizar_precos_mercado_isolado(
    mercado: &mut Mercado,
    catalogo: &CatalogoEconomia,
    config: &ConfigSimulacao,
) {
    let alfa = match config.formula_preco {
        FormulaPreco::Classica => 1.0,
        FormulaPreco::VictoriaSuavizada => config.fator_suavizacao_preco.clamp(0.01, 1.0),
    };
    for bem in &catalogo.bens {
        let o = mercado.ordens_venda.get(&bem.id).copied().unwrap_or(0.0);
        let d = mercado.ordens_compra.get(&bem.id).copied().unwrap_or(0.0);
        let alvo = calcular_preco_alvo(bem.preco_base, o, d, config);
        let ant = mercado
            .precos_atuais
            .get(&bem.id)
            .copied()
            .unwrap_or(bem.preco_base);
        let novo = (ant + alfa * (alvo - ant)).clamp(
            bem.preco_base * config.multiplicador_preco_minimo,
            bem.preco_base * config.multiplicador_preco_maximo,
        );
        mercado.precos_atuais.insert(bem.id.clone(), novo);
    }
}
