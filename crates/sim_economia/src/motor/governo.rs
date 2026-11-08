//! Fase (8) e Fase (9): Fechamento do orçamento público, dívida soberana, juros,
//! transferência do fundo privado e progressão da fila de construção civil.

use crate::catalogo::CatalogoEconomia;
use crate::entidades::{
    ConfigSimulacao, Estado, ItemConstrucao, MAX_FLUXOS_PREDIO, MAX_VAGAS_PREDIO, Mercado, Pais, Predio,
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
        qualidade_local: f64::NAN,
    };

    Some(())
}

/// Verifica se uma `Provincia` comporta a construção de `tipo_predio_idx`.
/// Construções são liberadas em qualquer província (recurso local não bloqueia, só muda o quanto rende).
pub fn provincia_comporta_novo_nivel(
    _catalogo: &CatalogoEconomia,
    _prov: &Provincia,
    _tipo_predio_idx: usize,
) -> bool {
    true
}

/// Verifica se dois identificadores de recursos naturais são compatíveis/equivalentes
/// (ex.: "ferro" e "minerio_ferro", "ouro" e "ouro_prata", "pesca" e "peixe").
#[inline]
pub fn recursos_compativeis(a: &str, b: &str) -> bool {
    a == b
        || (a == "minerio_ferro" && b == "ferro")
        || (a == "ferro" && b == "minerio_ferro")
        || (a == "ouro" && b == "ouro_prata")
        || (a == "ouro_prata" && b == "ouro")
        || (a == "peixe" && b == "pesca")
        || (a == "pesca" && b == "peixe")
        || (a == "borracha" && b == "borracha_natural")
        || (a == "borracha_natural" && b == "borracha")
        || (a == "gas" && b == "gas_natural")
        || (a == "gas_natural" && b == "gas")
}

/// Obtém o limite de um recurso na província considerando sinônimos.
#[inline]
pub fn limite_recurso_provincia(prov: &Provincia, recurso: &str) -> Option<u32> {
    if let Some(&l) = prov.limites_recursos.get(recurso) {
        return Some(l);
    }
    match recurso {
        "ferro" => prov.limites_recursos.get("minerio_ferro").copied(),
        "minerio_ferro" => prov.limites_recursos.get("ferro").copied(),
        "ouro" => prov.limites_recursos.get("ouro_prata").copied(),
        "ouro_prata" => prov.limites_recursos.get("ouro").copied(),
        "pesca" => prov.limites_recursos.get("peixe").copied(),
        "peixe" => prov.limites_recursos.get("pesca").copied(),
        "borracha" => prov.limites_recursos.get("borracha_natural").copied(),
        "borracha_natural" => prov.limites_recursos.get("borracha").copied(),
        "gas" => prov.limites_recursos.get("gas_natural").copied(),
        "gas_natural" => prov.limites_recursos.get("gas").copied(),
        _ => None,
    }
}

/// Conta quantos níveis de um determinado recurso estão ocupados em todas as províncias do estado.
pub fn estado_niveis_recurso(
    catalogo: &CatalogoEconomia,
    estado: &Estado,
    provincias: &[Provincia],
    recurso: &str,
) -> u32 {
    let mut total = 0u32;
    for &p_idx in &estado.provincias_indices {
        if let Some(prov) = provincias.get(p_idx) {
            for b in &prov.predios {
                let Some(t_def) = catalogo.tipos_predio.get(b.cache.tipo_idx) else { continue };
                if let Some(rec) = &t_def.recurso_natural_requerido {
                    if recursos_compativeis(rec, recurso) {
                        total = total.saturating_add(b.nivel);
                    }
                }
            }
        }
    }
    total
}

/// Verifica se um `Estado` e uma `Provincia` comportam a construção de `tipo_predio_idx`.
/// Construções são liberadas em qualquer província (recurso local não bloqueia, só muda o quanto rende).
/// Portos marítimos exigem estado costeiro.
pub fn estado_comporta_novo_nivel(
    catalogo: &CatalogoEconomia,
    estado: &Estado,
    _prov: &Provincia,
    tipo_predio_idx: usize,
) -> bool {
    let def = &catalogo.tipos_predio[tipo_predio_idx];

    // Portos marítimos exigem estado costeiro
    if def.id == "porto_comercial" || def.id == "porto_pesca" {
        if !estado.eh_costeiro && estado.limite_recurso("pesca") == 0 {
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

/// Recalcula `terra_aravel_ocupada` dos estados a partir das províncias vinculadas.
pub(crate) fn atualizar_terra_aravel_estados(estados: &mut [Estado], provincias: &[Provincia]) {
    for est in estados.iter_mut() {
        let mut soma = 0u32;
        for &p_idx in &est.provincias_indices {
            if let Some(p) = provincias.get(p_idx) {
                soma = soma.saturating_add(p.terra_aravel_ocupada);
            }
        }
        est.terra_aravel_ocupada = soma;
    }
}

/// Retorna os parâmetros de investimento privado e alocação de construção conforme a lei econômica:
/// `(taxa_reinvestimento_capitalista, taxa_reinvestimento_aristocrata, parcela_construcao_privada)` (estilo Victoria 3).
///
/// - Laissez-Faire: Capitalistas investem forte (35%), aristocratas 10%, 75% da capacidade de construção para o setor privado.
/// - Intervencionismo: Equilibrado (25% cap, 12% ari), 50% privado / 50% estatal.
/// - Economia Mista: (20% cap, 10% ari), 50% privado / 50% estatal.
/// - Tradicionalismo Agrário: Aristocratas investem 20% (agrícola/rural), cap 10%, 25% privado / 75% estatal.
/// - Extrativismo Colonial: 15% cap, 15% ari, 50% privado.
/// - Planificação Central (Gosplan): 0% privado, 100% estatal; lucros vão 100% para o orçamento do Estado.
pub fn parametros_sistema_economico(lei_id: &str) -> (f64, f64, f64) {
    match lei_id {
        "laissez_faire" => (0.35, 0.10, 0.75),
        "intervencionismo" => (0.25, 0.12, 0.50),
        "economia_mista" => (0.20, 0.10, 0.50),
        "tradicionalismo_agrario" => (0.10, 0.20, 0.25),
        "extrativismo_colonial" => (0.15, 0.15, 0.50),
        "planificacao_central" => (0.00, 0.00, 0.00),
        _ => (0.20, 0.10, 0.50),
    }
}

/// Executa o fechamento fiscal diário (juros, dívida, limite de crédito e calote)
/// e avança a fila nacional de construção civil, retornando o total de prédios concluídos no dia.
#[allow(clippy::too_many_arguments)]
pub(crate) fn executar_fechamento_fiscal_e_construcao(
    catalogo: &CatalogoEconomia,
    config: &ConfigSimulacao,
    mercados: &[Mercado],
    estados: &[Estado],
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
        let limite_credito = (50_000.0 + pais.pib_diario * 730.0 + reservas_predios)
            .max(pais.orcamento.limite_credito)
            .max(10_000.0);
        pais.orcamento.limite_credito = limite_credito;

        // 2. Juros progressivos sobre a dívida soberana
        let alavancagem = (pais.orcamento.divida / limite_credito).clamp(0.0, 1.5);
        let juros_anual = (pais.orcamento.taxa_juros_anual_base + 0.08 * alavancagem * alavancagem)
            .clamp(0.005, 0.22);
        pais.orcamento.taxa_juros_efetiva_anual = juros_anual;
        let gasto_juros = pais.orcamento.divida * (juros_anual / 365.0);
        pais.orcamento.gasto_juros = gasto_juros;

        // 2b. Avanço da fila de obras: pontos compartilhados por setor, consumo de materiais e custos
        avancar_fila_construcao(
            catalogo,
            config,
            &mercados[pais.mercado_idx],
            pais,
            pontos_construcao_setores_pais[c_idx],
        );

        // 3. Reembolso proporcional do setor de construção pelo uso privado de pontos (Victoria 3)
        if pais.orcamento.gasto_construcao > 0.0 && pais.pontos_aplicados_ultimo_tick > 0.0 {
            let frac_priv = (pais.pontos_privados_ultimo_tick / pais.pontos_aplicados_ultimo_tick).clamp(0.0, 1.0);
            if frac_priv > 0.0 {
                let reembolso = (pais.orcamento.gasto_construcao * frac_priv).min(pais.fundo_investimento_privado);
                pais.fundo_investimento_privado = (pais.fundo_investimento_privado - reembolso).max(0.0);
                pais.orcamento.gasto_construcao = (pais.orcamento.gasto_construcao - reembolso).max(0.0);
                pais.gasto_fundo_privado_ultimo_tick += reembolso;
            }
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
        let (_, _, cota_priv) = parametros_sistema_economico(&pais.politica_fiscal.lei_sistema_economico_id);
        let pts_totais = (pais.pontos_construcao_base + pontos_construcao_setores_pais[c_idx])
            * pais.mult_eficiencia_construcao_lei;
        // Obras simultâneas acompanham a capacidade de construção (cada obra absorve no máximo
        // `pontos_maximos_por_obra_dia` por dia); antes o teto fixo de 3 travava economias grandes.
        let max_obras_privadas = ((pts_totais * cota_priv) / config.pontos_maximos_por_obra_dia.max(1.0))
            .ceil()
            .clamp(1.0, 60.0) as usize;
        let obras_priv_ativas = pais.fila_construcao.iter().filter(|i| i.privado).count();

        if config.habilitar_investimento_privado_automatico
            && (dias_decorridos + c_idx as u64).is_multiple_of(30)
            && !pais.exige_propriedade_estatal_lei
        {
            crate::motor::gestao_predios::gerir_predios_setor_privado_pais(
                catalogo,
                &mercados[pais.mercado_idx],
                estados,
                pais,
                provincias,
                dias_decorridos,
            );
        }

        if config.habilitar_investimento_privado_automatico
            && (dias_decorridos + c_idx as u64).is_multiple_of(7)
            && !pais.exige_propriedade_estatal_lei
            && cota_priv > 0.0
            && pais.fundo_investimento_privado >= 2_000.0
            && obras_priv_ativas < max_obras_privadas
            && !pais.orcamento.em_calote
        {
            let vagas_fila = (max_obras_privadas - obras_priv_ativas).min(8);
            tentar_enfileirar_obra_privada(catalogo, mercados, estados, pais, provincias, vagas_fila);
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
                for _ in 0..concluido.niveis.max(1) {
                    if materializar_nivel_predio(
                        catalogo,
                        &mut provincias[concluido.provincia_idx],
                        concluido.tipo_predio_idx,
                    ) {
                        obras_concluidas_total += 1;
                    } else {
                        break;
                    }
                }
            } else {
                idx += 1;
            }
        }
    }

    obras_concluidas_total
}

/// Tamanho do lote de obra para o porte do país: economias grandes constroem vários níveis por obra,
/// senão a fila (e a IA que a preenche) não acompanha a capacidade de construção.
pub fn tamanho_lote_obra(provincias: &[Provincia], pais: &Pais) -> u32 {
    let niveis: f64 = pais
        .provincias_indices
        .iter()
        .map(|&p| provincias[p].predios.iter().map(|b| b.nivel as f64).sum::<f64>())
        .sum();
    ((niveis / 12_000.0).round() as u32).clamp(1, 24)
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

/// Aplica os pontos de construção do dia divididos entre setor estatal e setor privado (Victoria 3).
///
/// Regras de rateio e transbordo:
/// 1. A capacidade diária é particionada segundo a lei econômica (ex.: Laissez-Faire 75% privado / 25% estatal,
///    Intervencionismo 50/50, Tradicionalismo 25/75, Planificação Central 0/100).
/// 2. Se um setor não possui obras ativas (ou está em calote / sem fundos), seus pontos transbordam
///    livremente para o outro setor, evitando desperdício de capacidade física construída.
/// 3. Materiais de obras estatais são cobrados do tesouro (`orcamento.gasto_obras_materiais`).
/// 4. Materiais de obras privadas são debitados diretamente do `fundo_investimento_privado`.
fn avancar_fila_construcao(
    catalogo: &CatalogoEconomia,
    config: &ConfigSimulacao,
    mercado: &Mercado,
    pais: &mut Pais,
    pontos_setores: f64,
) {
    pais.demanda_obras.clear();
    pais.orcamento.gasto_obras_materiais = 0.0;
    pais.gasto_fundo_privado_ultimo_tick = 0.0;
    pais.pontos_aplicados_ultimo_tick = 0.0;
    pais.pontos_privados_ultimo_tick = 0.0;
    pais.pontos_publicos_ultimo_tick = 0.0;

    if pais.fila_construcao.is_empty() {
        return;
    }
    let num_bens = catalogo.bens.len();
    pais.demanda_obras.resize(num_bens, 0.0);
    // Materiais por ponto de cada categoria já resolvidos para índices de bens (evita buscas por texto por obra).
    let mats: [Vec<(usize, f64)>; 7] = [
        CategoriaPredio::RuralAgricola,
        CategoriaPredio::Extrativo,
        CategoriaPredio::Industrial,
        CategoriaPredio::Infraestrutura,
        CategoriaPredio::Construcao,
        CategoriaPredio::Governamental,
        CategoriaPredio::Militar,
    ]
    .map(|c| {
        materiais_por_ponto(c)
            .iter()
            .filter_map(|&(id, q)| catalogo.indice_bens.get(id).map(|&g| (g, q)))
            .collect()
    });

    let pts_totais = (pais.pontos_construcao_base + pontos_setores) * pais.mult_eficiencia_construcao_lei;
    let (_, _, cota_priv) = parametros_sistema_economico(&pais.politica_fiscal.lei_sistema_economico_id);
    let cota_est = 1.0 - cota_priv;

    let tem_estatais = pais.fila_construcao.iter().any(|i| !i.privado && !pais.orcamento.em_calote);
    let tem_privadas = pais.fila_construcao.iter().any(|i| i.privado && pais.fundo_investimento_privado > 1.0);

    let (mut pts_est_disp, mut pts_priv_disp) = match (tem_estatais, tem_privadas) {
        (true, true) => (pts_totais * cota_est, pts_totais * cota_priv),
        (true, false) => (pts_totais, 0.0),
        (false, true) => (0.0, pts_totais),
        (false, false) => (0.0, 0.0),
    };

    let max_por_obra = config.pontos_maximos_por_obra_dia.max(1.0);
    let idx_material = catalogo.indice_bens.get("material_construcao").copied();
    let fator_material = idx_material
        .map(|g| mercado.fator_suprimento_idx[g])
        .unwrap_or(1.0)
        .clamp(0.15, 1.0);

    let mut custo_publico = 0.0;
    let mut custo_privado = 0.0;

    // Passagem 1: Atende as obras de cada setor com suas cotas reservadas
    for item in &mut pais.fila_construcao {
        let falta = (item.custo_total_pontos - item.progresso_pontos).max(0.0);
        if falta <= 1e-6 {
            continue;
        }

        if item.privado {
            if pts_priv_disp <= 1e-6 || pais.fundo_investimento_privado <= 1.0 {
                continue;
            }
            let aplicar_pts = (pts_priv_disp.min(max_por_obra * item.niveis as f64) * fator_material).min(falta);
            if aplicar_pts <= 1e-6 {
                continue;
            }

            let categoria = catalogo.tipos_predio[item.tipo_predio_idx].categoria;
            let mut custo_item = 0.0;
            for &(g, qtd) in &mats[categoria as usize] {
                {
                    custo_item += qtd * aplicar_pts * mercado.precos_idx[g];
                }
            }

            let (aplicar_efetivo, custo_efetivo) = if custo_item > pais.fundo_investimento_privado && custo_item > 1e-6 {
                let frac = (pais.fundo_investimento_privado / custo_item).clamp(0.0, 1.0);
                (aplicar_pts * frac, pais.fundo_investimento_privado)
            } else {
                (aplicar_pts, custo_item)
            };

            if aplicar_efetivo > 1e-6 {
                pts_priv_disp -= aplicar_efetivo / fator_material;
                item.progresso_pontos += aplicar_efetivo;
                pais.pontos_privados_ultimo_tick += aplicar_efetivo;
                pais.fundo_investimento_privado = (pais.fundo_investimento_privado - custo_efetivo).max(0.0);
                custo_privado += custo_efetivo;

                for &(g, qtd) in &mats[categoria as usize] {
                    {
                        pais.demanda_obras[g] += qtd * aplicar_efetivo;
                    }
                }
            }
        } else {
            // Obra estatal
            if pts_est_disp <= 1e-6 || pais.orcamento.em_calote {
                continue;
            }
            let aplicar_pts = (pts_est_disp.min(max_por_obra * item.niveis as f64) * fator_material).min(falta);
            if aplicar_pts <= 1e-6 {
                continue;
            }

            let categoria = catalogo.tipos_predio[item.tipo_predio_idx].categoria;
            let mut custo_item = 0.0;
            for &(g, qtd) in &mats[categoria as usize] {
                {
                    custo_item += qtd * aplicar_pts * mercado.precos_idx[g];
                }
            }

            pts_est_disp -= aplicar_pts / fator_material;
            item.progresso_pontos += aplicar_pts;
            pais.pontos_publicos_ultimo_tick += aplicar_pts;
            custo_publico += custo_item;

            for &(g, qtd) in &mats[categoria as usize] {
                {
                    pais.demanda_obras[g] += qtd * aplicar_pts;
                }
            }
        }
    }

    // Passagem 2: Transbordo de pontos não utilizados para o outro setor
    if pts_priv_disp > 1e-6 && !pais.orcamento.em_calote {
        for item in &mut pais.fila_construcao {
            if pts_priv_disp <= 1e-6 {
                break;
            }
            if item.privado {
                continue;
            }
            let falta = (item.custo_total_pontos - item.progresso_pontos).max(0.0);
            if falta <= 1e-6 {
                continue;
            }
            let aplicar_pts = (pts_priv_disp.min(max_por_obra * item.niveis as f64) * fator_material).min(falta);
            if aplicar_pts <= 1e-6 {
                continue;
            }

            let categoria = catalogo.tipos_predio[item.tipo_predio_idx].categoria;
            let mut custo_item = 0.0;
            for &(g, qtd) in &mats[categoria as usize] {
                {
                    custo_item += qtd * aplicar_pts * mercado.precos_idx[g];
                }
            }

            pts_priv_disp -= aplicar_pts / fator_material;
            item.progresso_pontos += aplicar_pts;
            pais.pontos_publicos_ultimo_tick += aplicar_pts;
            custo_publico += custo_item;

            for &(g, qtd) in &mats[categoria as usize] {
                {
                    pais.demanda_obras[g] += qtd * aplicar_pts;
                }
            }
        }
    } else if pts_est_disp > 1e-6 && pais.fundo_investimento_privado > 1.0 {
        for item in &mut pais.fila_construcao {
            if pts_est_disp <= 1e-6 || pais.fundo_investimento_privado <= 1.0 {
                break;
            }
            if !item.privado {
                continue;
            }
            let falta = (item.custo_total_pontos - item.progresso_pontos).max(0.0);
            if falta <= 1e-6 {
                continue;
            }
            let aplicar_pts = (pts_est_disp.min(max_por_obra * item.niveis as f64) * fator_material).min(falta);
            if aplicar_pts <= 1e-6 {
                continue;
            }

            let categoria = catalogo.tipos_predio[item.tipo_predio_idx].categoria;
            let mut custo_item = 0.0;
            for &(g, qtd) in &mats[categoria as usize] {
                {
                    custo_item += qtd * aplicar_pts * mercado.precos_idx[g];
                }
            }

            let (aplicar_efetivo, custo_efetivo) = if custo_item > pais.fundo_investimento_privado && custo_item > 1e-6 {
                let frac = (pais.fundo_investimento_privado / custo_item).clamp(0.0, 1.0);
                (aplicar_pts * frac, pais.fundo_investimento_privado)
            } else {
                (aplicar_pts, custo_item)
            };

            if aplicar_efetivo > 1e-6 {
                pts_est_disp -= aplicar_efetivo / fator_material;
                item.progresso_pontos += aplicar_efetivo;
                pais.pontos_privados_ultimo_tick += aplicar_efetivo;
                pais.fundo_investimento_privado = (pais.fundo_investimento_privado - custo_efetivo).max(0.0);
                custo_privado += custo_efetivo;

                for &(g, qtd) in &mats[categoria as usize] {
                    {
                        pais.demanda_obras[g] += qtd * aplicar_efetivo;
                    }
                }
            }
        }
    }

    pais.orcamento.gasto_obras_materiais = custo_publico;
    pais.gasto_fundo_privado_ultimo_tick = custo_privado;
    pais.pontos_aplicados_ultimo_tick = pais.pontos_privados_ultimo_tick + pais.pontos_publicos_ultimo_tick;
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
            fechado: false,
            dias_retomada: 0,
            dias_prejuizo: 0,
            dias_fechado: 0,
            lucro_ema: 0.0,
            tendencia_lucro: 0.0,
            dia_ultima_mudanca_estado: 0,
            cache: PredioCompilado::default(),
        };
        let _ = compilar_cache_predio(catalogo, &mut novo);
        prov.predios.push(novo);
    }

    atualizar_terra_aravel_ocupada(catalogo, prov);
    true
}

/// Seleciona e enfileira uma nova obra autônoma financiada pelo setor privado (Victoria 3).
///
/// Critérios de decisão do investidor privado:
/// 1. Lucratividade esperada (receita - insumos - salários > 0 e margem líquida >= 5%).
/// 2. Demanda insatisfeita: prioriza bens com escassez no mercado local (preço alto / demanda > oferta).
/// 3. Viabilidade de insumos: recusa fábricas cujos insumos estejam em déficit severo (fator de suprimento < 65%).
/// 4. Potencial regional: respeita depósitos minerais, terra arável livre e culturas aptas (`estado_comporta_novo_nivel`).
/// 5. Infraestrutura: evita estados com déficit operacional e penalidade de acesso ao mercado.
/// 6. Mão de obra: exige trabalhadores desempregados ou camponeses disponíveis na província.
/// 7. Lei econômica: veta investimentos industriais sob Tradicionalismo Agrário e veta qualquer obra privada sob Planificação Central.
fn tentar_enfileirar_obra_privada(
    catalogo: &CatalogoEconomia,
    mercados: &[Mercado],
    estados: &[Estado],
    pais: &mut Pais,
    provincias: &[Provincia],
    max_novas: usize,
) {
    if pais.exige_propriedade_estatal_lei {
        return;
    }
    if pais.fundo_investimento_privado < 2_000.0 {
        return;
    }

    let lei_id = pais.politica_fiscal.lei_sistema_economico_id.as_str();
    let m = &mercados[pais.mercado_idx];
    let precos = &m.precos_idx;
    let ofertas = &m.oferta_idx;
    let demandas = &m.demanda_idx;
    let suprimentos = &m.fator_suprimento_idx;

    // (província, tipo, score, custo em pontos, lucro estimado) de todos os candidatos viáveis.
    let mut candidatos: Vec<(usize, usize, f64, f64, f64)> = Vec::new();

    for (tp_idx, padrao) in catalogo.tipos_predio_padrao.iter().enumerate() {
        let def_tipo = &catalogo.tipos_predio[tp_idx];
        if padrao.financiado_pelo_governo {
            continue;
        }

        // Restrição setorial sob Tradicionalismo Agrário (aristocratas investem no campo/minas)
        if lei_id == "tradicionalismo_agrario"
            && def_tipo.categoria != CategoriaPredio::RuralAgricola
            && def_tipo.categoria != CategoriaPredio::Extrativo
        {
            continue;
        }

        // 1. Insumos: verificar disponibilidade e custo no mercado local
        let mut fator_insumos_min = 1.0f64;
        let mut custo_insumos = 0.0f64;
        for &(g, q) in &padrao.insumos_padrao {
            fator_insumos_min = fator_insumos_min.min(suprimentos[g]);
            custo_insumos += q * precos[g];
        }
        if fator_insumos_min < 0.65 {
            continue; // Falta de matéria-prima estrangularia o prédio de imediato
        }

        // 2. Receita e Demanda Insatisfeita
        let mut receita = 0.0f64;
        let mut mult_demanda = 1.0f64;
        for &(g, q) in &padrao.produtos_padrao {
            let p = precos[g];
            receita += q * p;
            let p_base = catalogo.bens[g].preco_base;
            let ratio_p = (p / p_base.max(1e-6)).clamp(0.5, 2.0);
            let d = demandas[g];
            let o = ofertas[g].max(1.0);
            let ratio_od = (d / o).clamp(0.5, 2.0);
            mult_demanda *= (ratio_p * 0.60 + ratio_od * 0.40).clamp(0.40, 2.20);
        }

        // 3. Lucro líquido projetado por nível diário
        let lucro_est = receita - custo_insumos - padrao.folha_base_padrao;
        if lucro_est <= 0.0 {
            continue; // Sem prédios que dariam prejuízo
        }
        let margem = lucro_est / receita.max(1.0);
        if margem < 0.05 {
            continue; // Margem muito estreita para atrair capital privado
        }

        let retorno_por_ponto = lucro_est / padrao.custo_pontos_construcao.max(1.0);

        // 4. Avaliar províncias e estados nacionais aptos
        for &p_idx in &pais.provincias_indices {
            let prov = &provincias[p_idx];
            let e_idx = prov.estado_idx;
            if e_idx >= estados.len() {
                continue;
            }
            let estado = &estados[e_idx];

            // Potencial regional geográfico (recursos, terra arável, litoral)
            if !estado_comporta_novo_nivel(catalogo, estado, prov, tp_idx) {
                continue;
            }
            if !provincia_comporta_novo_nivel(catalogo, prov, tp_idx) {
                continue;
            }

            // Não expandir um prédio que está atualmente fechado nesta província (reabrir primeiro)
            if prov
                .predios
                .iter()
                .any(|b| b.cache.tipo_idx == tp_idx && b.fechado)
            {
                continue;
            }

            // Não duplicar a mesma obra na mesma província enquanto pendente
            if pais
                .fila_construcao
                .iter()
                .any(|item| item.provincia_idx == p_idx && item.tipo_predio_idx == tp_idx)
            {
                continue;
            }

            // Infraestrutura: respeitar gargalos e acesso ao mercado
            let saldo_infra = estado.saldo_infraestrutura();
            if estado.acesso_mercado < 0.70 || estado.penalidade_infraestrutura > 0.30 {
                continue;
            }
            let mod_infra = if saldo_infra >= 2.0 && estado.acesso_mercado >= 0.98 {
                1.25
            } else if saldo_infra < 0.0 {
                (1.0 + saldo_infra / 15.0).clamp(0.40, 1.0)
            } else {
                1.0
            };

            // Mão de obra disponível (desempregados + camponeses de subsistência)
            let desempregados: f64 = prov.pops.iter().map(|p| p.desempregados()).sum();
            let camponeses: f64 = prov
                .pops
                .iter()
                .filter(|p| catalogo.tipos_pop.get(p.tipo_idx).map_or(false, |tp| tp.subsistencia))
                .map(|p| p.populacao)
                .sum();
            let vagas_abertas: f64 = prov
                .predios
                .iter()
                .filter(|b| !b.fechado)
                .map(|b| (b.total_vagas() * b.fator_retomada() - b.total_empregados()).max(0.0))
                .sum();
            let mao_de_obra_liquida = desempregados + camponeses * 0.25 - vagas_abertas;
            if mao_de_obra_liquida < 250.0 {
                continue;
            }
            let mod_labor = (1.0 + (mao_de_obra_liquida / 40_000.0).min(0.50)).clamp(0.80, 1.50);

            // Bônus de qualidade de jazida mineral ou fertilidade agrícola
            let mod_qualidade = estado.fator_qualidade_predio(def_tipo).clamp(0.80, 1.50);

            let score = retorno_por_ponto * mult_demanda * mod_infra * mod_labor * mod_qualidade;

            candidatos.push((p_idx, tp_idx, score, padrao.custo_pontos_construcao, lucro_est));
        }
    }

    let lote = tamanho_lote_obra(provincias, pais);
    // Os melhores candidatos, no máximo um por província, até `max_novas`.
    candidatos.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    let mut escolhidos: Vec<(usize, usize)> = Vec::new();
    for &(p_idx, tp_idx, _, custo_pts, lucro_est) in &candidatos {
        if escolhidos.len() >= max_novas.max(1) {
            break;
        }
        if escolhidos.iter().any(|&(p, _)| p == p_idx) {
            continue;
        }
        escolhidos.push((p_idx, tp_idx));
        let just = format!("Retorno: ${lucro_est:.0}/dia");
        pais.fila_construcao.push(ItemConstrucao {
            provincia_id: provincias[p_idx].id.clone(),
            tipo_predio_id: catalogo.tipos_predio[tp_idx].id.clone(),
            progresso_pontos: 0.0,
            custo_total_pontos: custo_pts * lote as f64,
            privado: true,
            retorno_esperado: lucro_est,
            justificativa: just,
            niveis: lote,
            provincia_idx: p_idx,
            tipo_predio_idx: tp_idx,
        });
    }
}
