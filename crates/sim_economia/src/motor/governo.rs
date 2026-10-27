//! Fase (8) e Fase (9): Fechamento do orçamento público, dívida soberana, juros,
//! transferência do fundo privado e progressão da fila de construção civil.

use crate::catalogo::CatalogoEconomia;
use crate::entidades::{
    ConfigSimulacao, ItemConstrucao, MAX_FLUXOS_PREDIO, MAX_VAGAS_PREDIO, Mercado, Pais, Predio,
    PredioCompilado, Provincia, SlotEmpregoPredio,
};
use crate::tipos::{CategoriaPredio, RegimePropriedade};
use std::collections::BTreeMap;

/// Compila os métodos de produção ativos de um `Predio` em seu cache numérico rápido (`PredioCompilado`).
pub(crate) fn compilar_cache_predio(
    catalogo: &CatalogoEconomia,
    predio: &mut Predio,
) -> Option<()> {
    let tipo_idx = catalogo
        .indice_tipos_predio
        .get(&predio.tipo_predio_id)
        .copied()?;
    let def_tipo = &catalogo.tipos_predio[tipo_idx];

    let mut insumos_map: BTreeMap<usize, f64> = BTreeMap::new();
    let mut produtos_map: BTreeMap<usize, f64> = BTreeMap::new();
    let mut vagas_map: BTreeMap<usize, i32> = BTreeMap::new();
    let mut regime = match def_tipo.categoria {
        CategoriaPredio::RuralAgricola => RegimePropriedade::Aristocratico,
        CategoriaPredio::Governamental | CategoriaPredio::Militar | CategoriaPredio::Construcao => {
            RegimePropriedade::Estatal
        }
        _ => RegimePropriedade::PrivadoCapitalista,
    };
    let mut pts_const = 0.0;
    let mut buroc = 0.0;
    let mut militar = 0.0;
    let mut infra = 0.0;

    for m_id in &predio.metodos_ativos {
        let m_idx = *catalogo.indice_metodos.get(m_id)?;
        let mc = &catalogo.metodos_compilados[m_idx];
        for &(g, qtd) in &mc.insumos {
            *insumos_map.entry(g).or_insert(0.0) += qtd;
        }
        for &(g, qtd) in &mc.produtos {
            *produtos_map.entry(g).or_insert(0.0) += qtd;
        }
        for &(tp_idx, delta) in &mc.empregos {
            *vagas_map.entry(tp_idx).or_insert(0) += delta;
        }
        if let Some(r) = mc.regime_propriedade {
            regime = r;
        }
        pts_const += mc.pontos_construcao_gerados;
        buroc += mc.burocracia_gerada;
        militar += mc.forca_militar_gerada;
        infra += mc.infraestrutura_gerada;
    }

    let mut insumos_arr = [(0u16, 0.0); MAX_FLUXOS_PREDIO];
    let mut num_insumos = 0usize;
    for (g, q) in insumos_map {
        if q > 0.0 {
            if num_insumos >= MAX_FLUXOS_PREDIO {
                return None;
            }
            insumos_arr[num_insumos] = (g as u16, q);
            num_insumos += 1;
        }
    }

    let mut produtos_arr = [(0u16, 0.0); MAX_FLUXOS_PREDIO];
    let mut num_produtos = 0usize;
    for (g, q) in produtos_map {
        if q > 0.0 {
            if num_produtos >= MAX_FLUXOS_PREDIO {
                return None;
            }
            produtos_arr[num_produtos] = (g as u16, q);
            num_produtos += 1;
        }
    }

    // Preserva empregados atuais ao recompilar (por cache anterior ou pelo mapa RON)
    let mut vagas_arr = [SlotEmpregoPredio::default(); MAX_VAGAS_PREDIO];
    let mut num_vagas = 0usize;
    for (tp_idx, total_vagas_i32) in vagas_map {
        let vagas_por_nivel = total_vagas_i32.max(0) as u32;
        if vagas_por_nivel == 0 {
            continue;
        }
        if num_vagas >= MAX_VAGAS_PREDIO {
            return None;
        }
        let emp_anterior = if predio.cache.num_vagas > 0 {
            predio
                .cache
                .vagas()
                .iter()
                .find(|v| v.tipo_pop_idx as usize == tp_idx)
                .map(|v| v.empregados)
        } else {
            None
        }
        .or_else(|| {
            let id_str = &catalogo.tipos_pop[tp_idx].id;
            predio.empregados_por_tipo.get(id_str).copied()
        })
        .unwrap_or(0.0);
        let teto = (vagas_por_nivel as f64) * (predio.nivel as f64);
        vagas_arr[num_vagas] = SlotEmpregoPredio {
            tipo_pop_idx: tp_idx as u16,
            vagas_por_nivel,
            empregados: emp_anterior.clamp(0.0, teto),
        };
        num_vagas += 1;
    }

    predio.cache = PredioCompilado {
        tipo_idx,
        regime_propriedade: regime,
        financiado_pelo_governo: def_tipo.financiado_pelo_governo,
        num_insumos: num_insumos as u8,
        num_produtos: num_produtos as u8,
        num_vagas: num_vagas as u8,
        insumos_arr,
        produtos_arr,
        vagas_arr,
        pontos_construcao_por_nivel: pts_const,
        burocracia_por_nivel: buroc,
        forca_militar_por_nivel: militar,
        infraestrutura_por_nivel: infra,
        escala_efetiva_ultimo_tick: 0.0,
        fator_insumos_ultimo_tick: 1.0,
        margem_lucro_ultimo_tick: 0.0,
    };

    Some(())
}

/// Verifica se uma `Provincia` possui capacidade física (terra arável ou recurso natural)
/// para receber `+1` nível de `tipo_predio_idx`.
pub fn provincia_comporta_novo_nivel(
    catalogo: &CatalogoEconomia,
    prov: &Provincia,
    tipo_predio_idx: usize,
) -> bool {
    let def = &catalogo.tipos_predio[tipo_predio_idx];
    if def.usa_terra_aravel && prov.terra_aravel_livre() == 0 {
        return false;
    }
    if let Some(recurso) = &def.recurso_natural_requerido {
        let limite = prov.limites_recursos.get(recurso).copied().unwrap_or(0);
        let ocupado: u32 = prov
            .predios
            .iter()
            .filter(|b| {
                catalogo.tipos_predio[b.cache.tipo_idx]
                    .recurso_natural_requerido
                    .as_deref()
                    == Some(recurso.as_str())
            })
            .map(|b| b.nivel)
            .sum();
        if ocupado >= limite {
            return false;
        }
    }
    true
}

/// Recalcula `terra_aravel_ocupada` de uma província a partir dos prédios agrícolas presentes.
pub(crate) fn atualizar_terra_aravel_ocupada(catalogo: &CatalogoEconomia, prov: &mut Provincia) {
    let ocupada: u32 = prov
        .predios
        .iter()
        .filter(|b| catalogo.tipos_predio[b.cache.tipo_idx].usa_terra_aravel)
        .map(|b| b.nivel)
        .sum();
    prov.terra_aravel_ocupada = ocupada;
}

/// Executa o fechamento fiscal diário (juros, dívida, limite de crédito e calote)
/// e avança a fila nacional de construção civil, retornando o total de prédios concluídos no dia.
#[allow(clippy::too_many_arguments)]
pub(crate) fn executar_fechamento_fiscal_e_construcao(
    catalogo: &CatalogoEconomia,
    config: &ConfigSimulacao,
    mercados: &[Mercado],
    paises: &mut [Pais],
    provincias: &mut [Provincia],
    dias_decorridos: u64,
    pontos_construcao_setores_pais: &[f64],
    reservas_predios_pais: &[f64],
) -> usize {
    let mut obras_concluidas_total = 0usize;

    for (c_idx, pais) in paises.iter_mut().enumerate() {
        // 1. Limite de crédito lastreado no PIB anualizado + reservas privadas dos prédios do país
        let reservas_predios = reservas_predios_pais[c_idx];
        let limite_credito = (50_000.0 + pais.pib_diario * 180.0 + reservas_predios).max(10_000.0);
        pais.orcamento.limite_credito = limite_credito;

        // 2. Juros progressivos sobre a dívida soberana
        let alavancagem = (pais.orcamento.divida / limite_credito).clamp(0.0, 1.5);
        let juros_anual = (pais.orcamento.taxa_juros_anual_base + 0.08 * alavancagem * alavancagem)
            .clamp(0.005, 0.22);
        pais.orcamento.taxa_juros_efetiva_anual = juros_anual;
        let gasto_juros = pais.orcamento.divida * (juros_anual / 365.0);
        pais.orcamento.gasto_juros = gasto_juros;

        // 2b. Avanço da fila de obras: pontos de construção, consumo de materiais e custo pago pelo orçamento
        avancar_fila_construcao(
            catalogo,
            config,
            &mercados[pais.mercado_idx],
            pais,
            pontos_construcao_setores_pais[c_idx],
        );

        // 3. Reembolso parcial do fundo de investimento privado quando há obras privadas ativas
        let obras_privadas_ativas = pais.fila_construcao.iter().any(|i| i.privado);
        if obras_privadas_ativas && pais.orcamento.gasto_construcao > 0.0 {
            let reembolso = pais
                .orcamento
                .gasto_construcao
                .min(pais.fundo_investimento_privado * 0.50);
            pais.fundo_investimento_privado =
                (pais.fundo_investimento_privado - reembolso).max(0.0);
            // Abate o custo estatal líquido da construção financiada pelo setor privado
            pais.orcamento.gasto_construcao =
                (pais.orcamento.gasto_construcao - reembolso).max(0.0);
        }

        // 4. Consolidação do saldo diário no tesouro / dívida
        let saldo = pais.orcamento.receita_total() - pais.orcamento.despesa_total();
        pais.orcamento.saldo_diario = saldo;

        if saldo >= 0.0 {
            if pais.orcamento.divida > 0.0 {
                let amortizacao = saldo.min(pais.orcamento.divida);
                pais.orcamento.divida -= amortizacao;
                pais.orcamento.tesouro += saldo - amortizacao;
            } else {
                pais.orcamento.tesouro += saldo;
            }
        } else {
            let deficit = -saldo;
            let uso_tesouro = deficit.min(pais.orcamento.tesouro);
            pais.orcamento.tesouro -= uso_tesouro;
            pais.orcamento.divida += deficit - uso_tesouro;
        }

        // 5. Verificação de calote soberano / reestruturação no teto de crédito
        if pais.orcamento.divida >= limite_credito {
            pais.orcamento.divida = limite_credito;
            pais.orcamento.em_calote = true;
        } else if pais.orcamento.em_calote && pais.orcamento.divida < limite_credito * 0.80 {
            pais.orcamento.em_calote = false;
        }

        // 6. Investimento privado autônomo (capitalistas/aristocratas aplicando o fundo acumulado)
        if config.habilitar_investimento_privado_automatico
            && (dias_decorridos + c_idx as u64).is_multiple_of(7)
            && !pais.exige_propriedade_estatal_lei
            && pais.fundo_investimento_privado >= 3_500.0
            && pais.fila_construcao.iter().filter(|i| i.privado).count() < 2
        {
            tentar_enfileirar_obra_privada(catalogo, mercados, pais, provincias);
        }

        // 7. Fase (9): conclusão das obras que atingiram 100% dos pontos (o avanço ocorre antes do fechamento fiscal)
        if pais.fila_construcao.is_empty() {
            continue;
        }

        // Concluir obras que atingiram 100% dos pontos
        let mut idx = 0;
        while idx < pais.fila_construcao.len() {
            if pais.fila_construcao[idx].progresso_pontos + 1e-6
                >= pais.fila_construcao[idx].custo_total_pontos
            {
                let concluido = pais.fila_construcao.remove(idx);
                if materializar_nivel_predio(
                    catalogo,
                    &mut provincias[concluido.provincia_idx],
                    concluido.tipo_predio_idx,
                ) {
                    obras_concluidas_total += 1;
                }
            } else {
                idx += 1;
            }
        }
    }

    obras_concluidas_total
}

/// Receita de materiais consumidos por ponto de construção, conforme a categoria do prédio:
/// pares `(id do bem, quantidade por ponto)`.
pub fn materiais_por_ponto(categoria: CategoriaPredio) -> &'static [(&'static str, f64)] {
    match categoria {
        CategoriaPredio::RuralAgricola => &[
            ("material_construcao", 0.15),
            ("ferramentas", 0.08),
            ("madeira", 0.10),
        ],
        CategoriaPredio::Extrativo => &[
            ("material_construcao", 0.20),
            ("aco", 0.12),
            ("maquinas", 0.06),
        ],
        CategoriaPredio::Industrial | CategoriaPredio::Infraestrutura => &[
            ("material_construcao", 0.30),
            ("aco", 0.10),
            ("maquinas", 0.04),
            ("ferramentas", 0.03),
        ],
        _ => &[("material_construcao", 0.35), ("aco", 0.10)],
    }
}

/// Aplica os pontos de construção do dia à fila do país, consome materiais do mercado
/// (a demanda entra no mercado no tick seguinte), cobra o custo do orçamento (obras públicas)
/// ou do fundo privado (obras privadas) e desacelera a obra quando falta material de construção.
fn avancar_fila_construcao(
    catalogo: &CatalogoEconomia,
    config: &ConfigSimulacao,
    mercado: &Mercado,
    pais: &mut Pais,
    pontos_setores: f64,
) {
    pais.demanda_obras.clear();
    pais.orcamento.gasto_obras_materiais = 0.0;
    pais.pontos_aplicados_ultimo_tick = 0.0;
    if pais.fila_construcao.is_empty() {
        return;
    }
    let num_bens = catalogo.bens.len();
    pais.demanda_obras.resize(num_bens, 0.0);

    let mut pontos_disponiveis =
        (pais.pontos_construcao_base + pontos_setores) * pais.mult_eficiencia_construcao_lei;
    let max_por_obra = config.pontos_maximos_por_obra_dia.max(1.0);
    let idx_material = catalogo.indice_bens.get("material_construcao").copied();
    let mut custo_publico = 0.0;
    let mut custo_privado = 0.0;

    for item in &mut pais.fila_construcao {
        if pontos_disponiveis <= 1e-6 {
            break;
        }
        if !item.privado && pais.orcamento.em_calote {
            continue;
        }
        let falta = (item.custo_total_pontos - item.progresso_pontos).max(0.0);
        // Escassez de material de construção desacelera a obra (piso de 15%)
        let fator = idx_material
            .map(|g| mercado.fator_suprimento_idx[g])
            .unwrap_or(1.0)
            .clamp(0.15, 1.0);
        let aplicado = (pontos_disponiveis.min(max_por_obra) * fator).min(falta);
        pontos_disponiveis -= aplicado / fator;
        item.progresso_pontos += aplicado;
        pais.pontos_aplicados_ultimo_tick += aplicado;

        let categoria = catalogo.tipos_predio[item.tipo_predio_idx].categoria;
        let mut custo = 0.0;
        for &(bem_id, qtd) in materiais_por_ponto(categoria) {
            if let Some(&g) = catalogo.indice_bens.get(bem_id) {
                pais.demanda_obras[g] += qtd * aplicado;
                custo += qtd * aplicado * mercado.precos_idx[g];
            }
        }
        if item.privado {
            custo_privado += custo;
        } else {
            custo_publico += custo;
        }
    }
    pais.orcamento.gasto_obras_materiais = custo_publico;
    pais.fundo_investimento_privado = (pais.fundo_investimento_privado - custo_privado).max(0.0);
}

/// Estima o custo em materiais (preços do mercado) de um prédio inteiro de `custo_pontos` pontos.
pub fn custo_materiais_estimado(
    catalogo: &CatalogoEconomia,
    mercado: &Mercado,
    categoria: CategoriaPredio,
    custo_pontos: f64,
) -> f64 {
    let mut total = 0.0;
    for &(bem_id, qtd) in materiais_por_ponto(categoria) {
        if let Some(&g) = catalogo.indice_bens.get(bem_id) {
            total += qtd * custo_pontos * mercado.precos_idx[g];
        }
    }
    total
}

fn materializar_nivel_predio(
    catalogo: &CatalogoEconomia,
    prov: &mut Provincia,
    tipo_predio_idx: usize,
) -> bool {
    if !provincia_comporta_novo_nivel(catalogo, prov, tipo_predio_idx) {
        return false;
    }

    if let Some(existente) = prov
        .predios
        .iter_mut()
        .find(|b| b.cache.tipo_idx == tipo_predio_idx)
    {
        existente.nivel += 1;
    } else {
        let def = &catalogo.tipos_predio[tipo_predio_idx];
        let mut novo = Predio {
            tipo_predio_id: def.id.clone(),
            nivel: 1,
            metodos_ativos: def.metodos_padrao.clone(),
            empregados_por_tipo: BTreeMap::new(),
            salario_base: 4.0,
            reservas_caixa: 500.0,
            subsidiado: false,
            receita_ultimo_tick: 0.0,
            custo_insumos_ultimo_tick: 0.0,
            folha_salarial_ultimo_tick: 0.0,
            lucro_ultimo_tick: 0.0,
            dividendos_ultimo_tick: 0.0,
            cache: PredioCompilado::default(),
        };
        let _ = compilar_cache_predio(catalogo, &mut novo);
        prov.predios.push(novo);
    }

    atualizar_terra_aravel_ocupada(catalogo, prov);
    true
}

fn tentar_enfileirar_obra_privada(
    catalogo: &CatalogoEconomia,
    mercados: &[Mercado],
    pais: &mut Pais,
    provincias: &[Provincia],
) {
    let precos = &mercados[pais.mercado_idx].precos_idx;
    let mut melhor_escolha: Option<(usize, usize, f64, f64)> = None; // (prov_idx, tipo_idx, score, custo_pts)

    for (tp_idx, padrao) in catalogo.tipos_predio_padrao.iter().enumerate() {
        if padrao.financiado_pelo_governo {
            continue;
        }

        // Estima lucratividade diária de 1 nível usando os métodos padrão pré-compilados
        let mut rec = 0.0;
        for &(g, q) in &padrao.produtos_padrao {
            rec += q * precos[g];
        }
        let mut ins = 0.0;
        for &(g, q) in &padrao.insumos_padrao {
            ins += q * precos[g];
        }
        let lucro_est = rec - ins - padrao.folha_base_padrao;
        if lucro_est <= 0.0 {
            continue;
        }
        let retorno_por_ponto = lucro_est / padrao.custo_pontos_construcao.max(1.0);

        for &p_idx in &pais.provincias_indices {
            let prov = &provincias[p_idx];
            if !provincia_comporta_novo_nivel(catalogo, prov, tp_idx) {
                continue;
            }
            if pais
                .fila_construcao
                .iter()
                .any(|item| item.provincia_idx == p_idx && item.tipo_predio_idx == tp_idx)
            {
                continue;
            }
            let mao_de_obra_disp: f64 = prov.pops.iter().map(|p| p.desempregados()).sum();
            let vagas_abertas: f64 = prov
                .predios
                .iter()
                .map(|b| (b.total_vagas() - b.total_empregados()).max(0.0))
                .sum();
            let mao_de_obra_liquida = mao_de_obra_disp - vagas_abertas;
            if mao_de_obra_liquida < 300.0 {
                continue;
            }
            let score = retorno_por_ponto * (1.0 + (mao_de_obra_liquida / 50_000.0).min(0.5));
            if melhor_escolha.is_none_or(|(_, _, melhor_s, _)| score > melhor_s) {
                melhor_escolha = Some((p_idx, tp_idx, score, padrao.custo_pontos_construcao));
            }
        }
    }

    if let Some((p_idx, tp_idx, _, custo_pts)) = melhor_escolha {
        let reserva_necessaria = (custo_pts * 20.0).max(2_000.0);
        if pais.fundo_investimento_privado >= reserva_necessaria {
            // Deduz sinal inicial de capitalização do projeto
            pais.fundo_investimento_privado -= reserva_necessaria * 0.35;
            pais.fila_construcao.push(ItemConstrucao {
                provincia_id: provincias[p_idx].id.clone(),
                tipo_predio_id: catalogo.tipos_predio[tp_idx].id.clone(),
                progresso_pontos: 0.0,
                custo_total_pontos: custo_pts,
                privado: true,
                provincia_idx: p_idx,
                tipo_predio_idx: tp_idx,
            });
        }
    }
}
