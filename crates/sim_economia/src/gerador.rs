//! Gerador determinístico de mundos de grande escala (ex.: >3.000 províncias e ~150 países)
//! para benchmarks de desempenho e testes de estresse macroeconômico.

use crate::catalogo::CatalogoEconomia;
use crate::cenario::CenarioEconomia;
use crate::entidades::{
    ConfigSimulacao, Estado, ItemConstrucao, Mercado, OrcamentoGoverno, Pais, PoliticaFiscal, Pop,
    Predio, PredioCompilado, Provincia, RotaComercial,
};
use crate::tipos::{CategoriaPredio, DataJogo, Estrato, GrupoLeiEconomica};
use std::collections::BTreeMap;

/// Parâmetros de escala para síntese procedural determinística de um cenário mundial.
#[derive(Debug, Clone, Copy)]
pub struct ParametrosEscalaMundial {
    pub num_paises: usize,
    pub num_provincias: usize,
    pub num_mercados: usize,
    pub num_rotas_comerciais: usize,
}

impl Default for ParametrosEscalaMundial {
    fn default() -> Self {
        Self {
            num_paises: 150,
            num_provincias: 3_200,
            num_mercados: 60,
            num_rotas_comerciais: 180,
        }
    }
}

/// Gera deterministicamente um `CenarioEconomia` povoado com cadeias produtivas completas,
/// pops estratificadas, leis variadas e comércio internacional a partir de um `CatalogoEconomia`.
pub fn gerar_cenario_escala_mundial(
    mut catalogo: CatalogoEconomia,
    params: ParametrosEscalaMundial,
) -> CenarioEconomia {
    let _ = catalogo.validar_e_indexar();

    let num_paises = params.num_paises.max(2);
    let num_mercados = params.num_mercados.clamp(1, num_paises);
    let num_provincias = params.num_provincias.max(num_paises);

    // Selecionar leis disponíveis por eixo no catálogo
    let leis_sist: Vec<String> = catalogo
        .leis
        .iter()
        .filter(|l| l.grupo == GrupoLeiEconomica::SistemaEconomico)
        .map(|l| l.id.clone())
        .collect();
    let leis_trib: Vec<String> = catalogo
        .leis
        .iter()
        .filter(|l| l.grupo == GrupoLeiEconomica::Tributacao)
        .map(|l| l.id.clone())
        .collect();
    let leis_com: Vec<String> = catalogo
        .leis
        .iter()
        .filter(|l| l.grupo == GrupoLeiEconomica::Comercio)
        .map(|l| l.id.clone())
        .collect();

    let sist_padrao = leis_sist.first().cloned().unwrap_or_default();
    let trib_padrao = leis_trib.first().cloned().unwrap_or_default();
    let com_padrao = leis_com.first().cloned().unwrap_or_default();

    // 1. Mercados
    let mut mercados = Vec::with_capacity(num_mercados);
    for m in 0..num_mercados {
        mercados.push(Mercado {
            id: format!("mercado_bloco_{}", m + 1),
            nome: format!("Mercado Regional {}", m + 1),
            pais_lider_id: format!("pais_{}", m + 1),
            precos_atuais: BTreeMap::new(),
            ordens_venda: BTreeMap::new(),
            ordens_compra: BTreeMap::new(),
            importacoes_liquidas: BTreeMap::new(),
            pais_lider_idx: 0,
            precos_idx: Vec::new(),
            oferta_idx: Vec::new(),
            demanda_idx: Vec::new(),
            fator_suprimento_idx: Vec::new(),
            importacoes_liquidas_idx: Vec::new(),
        });
    }

    // 2. Países
    let culturas = [
        "anglo_americana",
        "eslava_oriental",
        "latino_americana",
        "germano_europeia",
        "franco_europeia",
        "luso_brasileira",
        "sino_asiatica",
        "indo_asiatica",
    ];
    let religioes = ["protestante", "catolica", "ortodoxa", "secular", "islamica"];

    let mut paises = Vec::with_capacity(num_paises);
    for c in 0..num_paises {
        let m_idx = c % num_mercados;
        let lei_s = if leis_sist.is_empty() {
            sist_padrao.clone()
        } else {
            leis_sist[c % leis_sist.len()].clone()
        };
        let lei_t = if leis_trib.is_empty() {
            trib_padrao.clone()
        } else {
            leis_trib[c % leis_trib.len()].clone()
        };
        let lei_c = if leis_com.is_empty() {
            com_padrao.clone()
        } else {
            leis_com[c % leis_com.len()].clone()
        };

        paises.push(Pais {
            id: format!("pais_{}", c + 1),
            tag: Some(format!("P{:02}", (c + 1).min(99))),
            nome: format!("Nação {}", c + 1),
            mercado_id: mercados[m_idx].id.clone(),
            acordos_comerciais: Vec::new(),
            culturas_primarias: vec![culturas[c % culturas.len()].to_string()],
            religiao_oficial: religioes[c % religioes.len()].to_string(),
            politica_fiscal: PoliticaFiscal {
                lei_sistema_economico_id: lei_s,
                lei_tributaria_id: lei_t,
                lei_comercial_id: lei_c,
                aliquota_imposto_renda_baixo: 0.10,
                aliquota_imposto_renda_medio: 0.16,
                aliquota_imposto_renda_alto: 0.24,
                aliquota_imposto_dividendos: 0.18,
                imposto_sobre_terra_por_campones: 0.03,
                imposto_sobre_terra_por_nivel_rural: 8.0,
                aliquota_consumo_geral: 0.05,
                bens_tributados_consumo: BTreeMap::new(),
                tarifa_importacao: 0.08,
                tarifa_exportacao: 0.03,
                tarifas_importacao_bens: BTreeMap::new(),
                tarifas_exportacao_bens: BTreeMap::new(),
                nivel_abertura_mercado: None,
                nivel_investimento_educacao: ((c % 4) + 1) as u8,
                multiplicador_salario_governo: 1.0,
                multiplicador_salario_militar: 1.0,
            },
            orcamento: OrcamentoGoverno {
                tesouro: 40_000.0,
                divida: 0.0,
                limite_credito: 200_000.0,
                taxa_juros_anual_base: 0.04,
                taxa_juros_efetiva_anual: 0.04,
                ..Default::default()
            },
            pontos_construcao_base: 12.0,
            fila_construcao: Vec::new(),
            fundo_investimento_privado: 5_000.0,
            burocracia_produzida: 0.0,
            burocracia_necessaria: 0.0,
            eficiencia_arrecadacao: 1.0,
            forca_militar_total: 0.0,
            pib_diario: 0.0,
            mercado_idx: 0,
            provincias_indices: Vec::new(),
            estados_indices: Vec::new(),
            aliquota_consumo_por_bem: Vec::new(),
            mult_eficiencia_construcao_lei: 1.0,
            mult_volume_comercio_lei: 1.0,
            exige_propriedade_estatal_lei: false,
            utilizacao_construcao_ultimo_tick: 0.0,
            demanda_obras: Vec::new(),
            pontos_aplicados_ultimo_tick: 0.0,
        });
    }

    // 3. Estados (~4 províncias por estado, no mínimo 1 estado por país)
    let estados_por_pais = (num_provincias / (num_paises * 4)).max(1);
    let mut estados = Vec::with_capacity(num_paises * estados_por_pais);
    for (c, pais) in paises.iter().enumerate() {
        for s in 0..estados_por_pais {
            estados.push(Estado {
                id: format!("estado_{}_{}", c + 1, s + 1),
                nome: format!("Estado {} de {}", s + 1, pais.nome),
                pais_id: pais.id.clone(),
                incorporado: true,
                infraestrutura_bonus: 15.0,
                acesso_mercado: 1.0,
                terra_aravel: 100,
                culturas_permitidas: Vec::new(),
                limites_recursos: BTreeMap::new(),
                qualidade_recursos: BTreeMap::new(),
                pais_idx: 0,
                provincias_indices: Vec::new(),
                terra_aravel_ocupada: 0,
            });
        }
    }

    // Classificar tipos de pop e tipos de prédio para distribuição realista
    let tp_campones = catalogo
        .tipos_pop
        .iter()
        .find(|t| t.subsistencia)
        .unwrap_or(&catalogo.tipos_pop[0])
        .id
        .clone();
    let tps_baixo: Vec<String> = catalogo
        .tipos_pop
        .iter()
        .filter(|t| t.estrato == Estrato::Baixo && !t.subsistencia)
        .map(|t| t.id.clone())
        .collect();
    let tps_medio: Vec<String> = catalogo
        .tipos_pop
        .iter()
        .filter(|t| t.estrato == Estrato::Medio)
        .map(|t| t.id.clone())
        .collect();
    let tps_alto: Vec<String> = catalogo
        .tipos_pop
        .iter()
        .filter(|t| t.estrato == Estrato::Alto)
        .map(|t| t.id.clone())
        .collect();

    let predios_rurais: Vec<usize> = catalogo
        .tipos_predio
        .iter()
        .enumerate()
        .filter_map(|(i, p)| (p.categoria == CategoriaPredio::RuralAgricola).then_some(i))
        .collect();
    let predios_extrativos: Vec<usize> = catalogo
        .tipos_predio
        .iter()
        .enumerate()
        .filter_map(|(i, p)| (p.categoria == CategoriaPredio::Extrativo).then_some(i))
        .collect();
    let predios_ind: Vec<usize> = catalogo
        .tipos_predio
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            matches!(
                p.categoria,
                CategoriaPredio::Industrial | CategoriaPredio::Infraestrutura
            )
            .then_some(i)
        })
        .collect();
    let predios_inst: Vec<usize> = catalogo
        .tipos_predio
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            matches!(
                p.categoria,
                CategoriaPredio::Governamental
                    | CategoriaPredio::Construcao
                    | CategoriaPredio::Militar
            )
            .then_some(i)
        })
        .collect();

    // 4. Províncias (~3.200+)
    let mut provincias = Vec::with_capacity(num_provincias);
    for p in 0..num_provincias {
        let c_idx = p % num_paises;
        let s_local = (p / num_paises) % estados_por_pais;
        let e_idx = c_idx * estados_por_pais + s_local;
        let eh_capital_pais = p < num_paises;

        let cultura = culturas[c_idx % culturas.len()].to_string();
        let religiao = religioes[c_idx % religioes.len()].to_string();

        let mut limites_recursos = BTreeMap::new();
        for tp_pred in &catalogo.tipos_predio {
            if let Some(rec) = &tp_pred.recurso_natural_requerido {
                limites_recursos.insert(rec.clone(), 16);
            }
        }

        let mut pops = Vec::with_capacity(6);
        pops.push(Pop {
            tipo_pop_id: tp_campones.clone(),
            cultura: cultura.clone(),
            religiao: religiao.clone(),
            populacao: 28_000.0,
            empregados: 0.0,
            riqueza_reservas: 4_000.0,
            padrao_de_vida: 9.5,
            alfabetizacao: 0.42,
            renda_ultimo_tick: 0.0,
            gasto_consumo_ultimo_tick: 0.0,
            tipo_idx: 0,
            cultura_idx: 0,
            religiao_idx: 0,
        });
        for tp_id in &tps_baixo {
            pops.push(Pop {
                tipo_pop_id: tp_id.clone(),
                cultura: cultura.clone(),
                religiao: religiao.clone(),
                populacao: 12_000.0,
                empregados: 8_500.0,
                riqueza_reservas: 6_000.0,
                padrao_de_vida: 12.5,
                alfabetizacao: 0.55,
                renda_ultimo_tick: 0.0,
                gasto_consumo_ultimo_tick: 0.0,
                tipo_idx: 0,
                cultura_idx: 0,
                religiao_idx: 0,
            });
        }
        if !tps_medio.is_empty() {
            let m1 = &tps_medio[p % tps_medio.len()];
            let m2 = &tps_medio[(p + 1) % tps_medio.len()];
            pops.push(Pop {
                tipo_pop_id: m1.clone(),
                cultura: cultura.clone(),
                religiao: religiao.clone(),
                populacao: 4_500.0,
                empregados: 3_200.0,
                riqueza_reservas: 5_000.0,
                padrao_de_vida: 17.5,
                alfabetizacao: 0.78,
                renda_ultimo_tick: 0.0,
                gasto_consumo_ultimo_tick: 0.0,
                tipo_idx: 0,
                cultura_idx: 0,
                religiao_idx: 0,
            });
            if m2 != m1 {
                pops.push(Pop {
                    tipo_pop_id: m2.clone(),
                    cultura: cultura.clone(),
                    religiao: religiao.clone(),
                    populacao: 3_500.0,
                    empregados: 2_500.0,
                    riqueza_reservas: 4_500.0,
                    padrao_de_vida: 18.0,
                    alfabetizacao: 0.80,
                    renda_ultimo_tick: 0.0,
                    gasto_consumo_ultimo_tick: 0.0,
                    tipo_idx: 0,
                    cultura_idx: 0,
                    religiao_idx: 0,
                });
            }
        }
        for tp_id in &tps_alto {
            pops.push(Pop {
                tipo_pop_id: tp_id.clone(),
                cultura: cultura.clone(),
                religiao: religiao.clone(),
                populacao: 800.0,
                empregados: 500.0,
                riqueza_reservas: 8_000.0,
                padrao_de_vida: 26.0,
                alfabetizacao: 0.92,
                renda_ultimo_tick: 0.0,
                gasto_consumo_ultimo_tick: 0.0,
                tipo_idx: 0,
                cultura_idx: 0,
                religiao_idx: 0,
            });
        }

        // Prédios da província (4 a 6 prédios por província cobrindo toda a cadeia produtiva)
        let mut predios = Vec::with_capacity(6);
        let mut adicionar_predio = |tp_idx: usize, nivel: u32| {
            let def = &catalogo.tipos_predio[tp_idx];
            let mut emp_map = BTreeMap::new();
            for m_id in &def.metodos_padrao {
                if let Some(m) = catalogo.obter_metodo(m_id) {
                    for (pop_id, &vagas) in &m.empregos {
                        if vagas > 0 {
                            *emp_map.entry(pop_id.clone()).or_insert(0.0) +=
                                (vagas as f64) * (nivel as f64) * 0.85;
                        }
                    }
                }
            }
            predios.push(Predio {
                tipo_predio_id: def.id.clone(),
                nivel,
                metodos_ativos: def.metodos_padrao.clone(),
                empregados_por_tipo: emp_map,
                salario_base: 4.0,
                reservas_caixa: (nivel as f64) * 2_000.0,
                subsidiado: false,
                receita_ultimo_tick: 0.0,
                custo_insumos_ultimo_tick: 0.0,
                folha_salarial_ultimo_tick: 0.0,
                lucro_ultimo_tick: 0.0,
                dividendos_ultimo_tick: 0.0,
                cache: PredioCompilado::default(),
            });
        };

        if !predios_rurais.is_empty() {
            adicionar_predio(predios_rurais[p % predios_rurais.len()], 2);
        }
        if !predios_extrativos.is_empty() {
            adicionar_predio(predios_extrativos[p % predios_extrativos.len()], 2);
        }
        if !predios_ind.is_empty() {
            adicionar_predio(predios_ind[p % predios_ind.len()], 2);
            if predios_ind.len() > 1 {
                adicionar_predio(predios_ind[(p + 3) % predios_ind.len()], 1);
            }
        }
        if eh_capital_pais {
            for &inst_idx in &predios_inst {
                adicionar_predio(inst_idx, 1);
            }
        }

        let prov_id = format!("prov_{}", p + 1);
        if eh_capital_pais && !predios_ind.is_empty() {
            let alvo_obra = &catalogo.tipos_predio[predios_ind[c_idx % predios_ind.len()]];
            paises[c_idx].fila_construcao.push(ItemConstrucao {
                provincia_id: prov_id.clone(),
                tipo_predio_id: alvo_obra.id.clone(),
                progresso_pontos: 0.0,
                custo_total_pontos: alvo_obra.custo_pontos_construcao,
                privado: false,
                provincia_idx: 0,
                tipo_predio_idx: 0,
            });
        }

        provincias.push(Provincia {
            id: prov_id,
            nome: format!("Província {}", p + 1),
            codigo_externo: None,
            estado_id: estados[e_idx].id.clone(),
            pais_id: paises[c_idx].id.clone(),
            terra_aravel_total: 30,
            limites_recursos,
            infraestrutura_base: 35.0,
            pops,
            predios,
            estado_idx: 0,
            pais_idx: 0,
            mercado_idx: 0,
            atracao_migratoria: 10.0,
            vagas_abertas_cache: 0.0,
            terra_aravel_ocupada: 0,
        });
    }

    // 5. Rotas comerciais internacionais entre mercados
    let bens_comercializaveis: Vec<String> = catalogo
        .bens
        .iter()
        .filter(|b| b.comercializavel)
        .map(|b| b.id.clone())
        .collect();
    let mut rotas_comerciais = Vec::new();
    if num_mercados >= 2 && !bens_comercializaveis.is_empty() {
        for r in 0..params.num_rotas_comerciais {
            let m_orig = r % num_mercados;
            let m_dest = (r + 1 + (r / num_mercados)) % num_mercados;
            if m_orig == m_dest {
                continue;
            }
            rotas_comerciais.push(RotaComercial {
                id: format!("rota_global_{}", r + 1),
                mercado_origem_id: mercados[m_orig].id.clone(),
                mercado_destino_id: mercados[m_dest].id.clone(),
                pais_operador_id: mercados[m_dest].pais_lider_id.clone(),
                bem_id: bens_comercializaveis[r % bens_comercializaveis.len()].clone(),
                volume_atual: 8.0,
                volume_maximo: 80.0,
                automatica: true,
                lucro_ultimo_tick: 0.0,
                origem_idx: 0,
                destino_idx: 0,
                pais_operador_idx: 0,
                bem_idx: 0,
            });
        }
    }

    CenarioEconomia {
        data_inicial: DataJogo::default(),
        config: ConfigSimulacao::default(),
        catalogo,
        mercados,
        paises,
        estados,
        provincias,
        rotas_comerciais,
        blocos_comerciais: Vec::new(),
    }
}
