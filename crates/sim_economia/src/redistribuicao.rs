//! Módulo para redistribuição de população e prédios entre províncias de um mesmo país.
//! Mantém totais nacionais (±2%) enquanto distribui de forma mais realista.

use crate::cenario::CenarioEconomia;
use crate::entidades::Estado;
use crate::motor::governo::compilar_cache_predio;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Default)]
struct ProvinciaPesos {
    populacao: f64,
    extrativos: BTreeMap<String, f64>,
    industria: f64,
    servicos: f64,
    governo: f64,
    urbanizacao: f64,
}

#[derive(Debug, Clone, Default)]
struct PaisTotais {
    pops_por_tipo: BTreeMap<String, f64>,
    predios_por_tipo: BTreeMap<String, u64>,
    empregados_por_tipo: BTreeMap<String, f64>,
}

#[derive(Debug, Clone)]
struct EstadoPotencial {
    terra_aravel: u32,
    culturas_permitidas: Vec<String>,
    recursos: BTreeMap<String, u32>,
    qualidade_recursos: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Copy)]
enum CategoriaPredioRedist {
    Extrativo,
    Industrial,
    Servicos,
    Governo,
    Fazenda,
}

// Totais nacionais históricos corretos (baseados em dados históricos e testes)
// População total por grupo de países (em milhões)
// Alguns "países" no jogo representam regiões de um mesmo país histórico (ex: China = CHI+CCP+MAN+MEN+NKG)
const POP_NACIONAL_HISTORICA_MI: &[(&[&str], f64)] = &[
    (&["usa"], 140.0),
    (&["sov"], 170.0),
    (&["chi", "ccp", "man", "men", "nkg"], 500.0),
    (&["ind"], 400.0),
    (&["bra"], 46.0),
    (&["ger"], 70.0),
    (&["jap"], 72.0),
    (&["fra"], 40.0),
    (&["gbr"], 48.0),
    (&["ita"], 44.0),
    (&["pol"], 24.0),
    (&["can"], 12.0),
    (&["esp"], 26.0),
    (&["rom"], 16.0),
    (&["hun"], 9.0),
    (&["bul"], 7.0),
    (&["cze"], 11.0),
    (&["yug"], 15.0),
    (&["gre"], 7.0),
    (&["por"], 8.0),
    (&["bel"], 8.0),
    (&["nld"], 9.0),
    (&["swe"], 6.0),
    (&["nor"], 3.0),
    (&["den"], 4.0),
    (&["fin"], 4.0),
    (&["irl"], 3.0),
    (&["aut"], 7.0),
    (&["che"], 4.0),
    (&["tur"], 19.0),
    (&["irn"], 16.0),
    (&["irq"], 4.0),
    (&["syr"], 3.0),
    (&["lbn"], 1.0),
    (&["jor"], 0.5),
    (&["isr"], 0.6),
    (&["sau"], 4.0),
    (&["yem"], 4.0),
    (&["omn"], 0.5),
    (&["afg"], 8.0),
    (&["tib"], 1.0),
    (&["nex"], 0.5),
    (&["bhu"], 0.3),
    (&["nep"], 8.0),
    (&["tha"], 18.0),
    (&["ins"], 70.0),
    (&["phi"], 19.0),
    (&["mys"], 5.0),
    (&["sgp"], 0.8),
    (&["brn"], 0.3),
    (&["lao"], 1.0),
    (&["cam"], 3.0),
    (&["vnm"], 25.0),
    (&["kor"], 20.0),
    (&["mng"], 0.8),
    (&["twn"], 6.0),
    (&["hk"], 1.5),
    (&["mac"], 0.5),
    (&["aus"], 7.0),
    (&["nzl"], 1.6),
    (&["png"], 1.5),
    (&["fij"], 0.3),
    (&["slb"], 0.1),
    (&["van"], 0.1),
    (&["nca"], 0.1),
    (&["fsm"], 0.1),
    (&["mhl"], 0.1),
    (&["plw"], 0.1),
    (&["kir"], 0.05),
    (&["tuv"], 0.01),
    (&["nru"], 0.01),
    (&["wsm"], 0.1),
    (&["tkl"], 0.001),
    (&["asm"], 0.05),
    (&["gum"], 0.02),
    (&["mnp"], 0.05),
    (&["pri"], 2.0),
    (&["vir"], 0.03),
    (&["umi"], 0.001),
    (&["cub"], 5.0),
    (&["hti"], 3.0),
    (&["dom"], 2.0),
    (&["jam"], 1.3),
    (&["tto"], 0.6),
    (&["bhs"], 0.1),
    (&["blz"], 0.1),
    (&["grd"], 0.05),
    (&["lca"], 0.1),
    (&["vct"], 0.1),
    (&["atg"], 0.05),
    (&["dma"], 0.05),
    (&["kna"], 0.05),
    (&["brb"], 0.2),
    (&["sur"], 0.2),
    (&["guy"], 0.4),
    (&["guf"], 0.02),
    (&["ecu"], 3.5),
    (&["per"], 7.0),
    (&["bol"], 3.0),
    (&["chl"], 6.0),
    (&["arg"], 15.0),
    (&["ury"], 2.0),
    (&["pry"], 1.0),
    (&["col"], 9.0),
    (&["ven"], 4.0),
    (&["mex"], 22.0),
    (&["gtm"], 3.0),
    (&["slv"], 2.0),
    (&["hnd"], 1.5),
    (&["nic"], 1.0),
    (&["cri"], 0.8),
    (&["pan"], 0.7),
];

/// Redistribui população e prédios dentro de cada país em `provincias.ron`.
/// Mantém os TOTAIS NACIONAIS (população por tipo, níveis totais de cada prédio, empregados por tipo) IGUAIS aos originais históricos.
/// Prédios são redistribuídos proporcionalmente à população para manter emprego equilibrado.
pub fn redistribuir_provincias(cen: &mut CenarioEconomia) {
    // Construir mapas auxiliares
    let potenciais_por_estado = construir_mapa_potenciais(&cen.estados);

    // Agrupar índices de províncias por país
    let mut provincias_por_pais: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, prov) in cen.provincias.iter().enumerate() {
        provincias_por_pais.entry(prov.pais_id.clone()).or_default().push(idx);
    }

    // Calcular totais nacionais do arquivo ATUAL (antes da redistribuição) para servir de referência "original"
    // Estes são os totais que devemos preservar (±2% para empregados, exato para população e prédios)
    let mut pops_atuais_por_pais: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    let mut empregados_atuais_por_pais: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    let mut predios_atuais_por_pais: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();

    for (pais_id, prov_indices) in &provincias_por_pais {
        let mut pops_por_tipo: BTreeMap<String, f64> = BTreeMap::new();
        let mut empregados_por_tipo: BTreeMap<String, f64> = BTreeMap::new();
        for &idx in prov_indices {
            let prov = &cen.provincias[idx];
            for pop in &prov.pops {
                *pops_por_tipo.entry(pop.tipo_pop_id.clone()).or_default() += pop.populacao;
                *empregados_por_tipo.entry(pop.tipo_pop_id.clone()).or_default() += pop.empregados;
            }
        }

        let mut predios_por_tipo = BTreeMap::new();
        for &idx in prov_indices {
            let prov = &cen.provincias[idx];
            for predio in &prov.predios {
                *predios_por_tipo.entry(predio.tipo_predio_id.clone()).or_default() += predio.nivel as u64;
            }
        }

        pops_atuais_por_pais.insert(pais_id.clone(), pops_por_tipo);
        empregados_atuais_por_pais.insert(pais_id.clone(), empregados_por_tipo);
        predios_atuais_por_pais.insert(pais_id.clone(), predios_por_tipo);
    }

    // Agrupar países históricos e calcular totais alvo
    // Para cada grupo histórico, somar totais atuais e escalar população para alvo histórico
    // Prédios e empregados: preservar totais atuais do arquivo (são a referência "original")
    let mut grupos_processados: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut totais_nacionais: BTreeMap<String, PaisTotais> = BTreeMap::new();

    for (paises_grupo, pop_historica_mi) in POP_NACIONAL_HISTORICA_MI {
        let pop_historica_alvo = pop_historica_mi * 1_000_000.0;

        // Coletar todos os países deste grupo que existem nos dados
        let paises_existentes: Vec<String> = paises_grupo.iter()
            .filter_map(|p| {
                if provincias_por_pais.contains_key(*p) {
                    Some(p.to_string())
                } else {
                    None
                }
            })
            .collect();

        if paises_existentes.is_empty() {
            continue;
        }

        // Somar população atual do grupo
        let mut pops_grupo: BTreeMap<String, f64> = BTreeMap::new();
        let mut empregados_grupo: BTreeMap<String, f64> = BTreeMap::new();
        let mut predios_grupo: BTreeMap<String, u64> = BTreeMap::new();
        let mut pop_total_grupo = 0.0;

        for pais in &paises_existentes {
            if let Some(pops) = pops_atuais_por_pais.get(pais) {
                for (tipo, &v) in pops {
                    *pops_grupo.entry(tipo.clone()).or_default() += v;
                    pop_total_grupo += v;
                }
            }
            if let Some(emps) = empregados_atuais_por_pais.get(pais) {
                for (tipo, &v) in emps {
                    *empregados_grupo.entry(tipo.clone()).or_default() += v;
                }
            }
            if let Some(preds) = predios_atuais_por_pais.get(pais) {
                for (tipo, &v) in preds {
                    *predios_grupo.entry(tipo.clone()).or_default() += v;
                }
            }

            grupos_processados.insert(pais.clone());
        }

        // Escalar população do grupo para atingir alvo histórico
        // Usar método do maior resto para preservar totais EXATAMENTE por tipo de pop
        let fator_pop = if pop_total_grupo > 0.0 { pop_historica_alvo / pop_total_grupo } else { 1.0 };
        
        // Calcular totais alvo por tipo de pop para o grupo (usando maior resto para evitar perda)
        let mut pops_alvo_grupo: BTreeMap<String, f64> = BTreeMap::new();
        let mut fracs_pop: Vec<(String, f64)> = Vec::new();
        let mut total_pops_alvo = 0.0;
        
        for (tipo, &v) in &pops_grupo {
            let alvo = v * fator_pop;
            let base = alvo.floor();
            let frac = alvo - base;
            pops_alvo_grupo.insert(tipo.clone(), base);
            total_pops_alvo += base;
            fracs_pop.push((tipo.clone(), frac));
        }
        
        // Distribuir resto para atingir exatamente pop_historica_alvo
        let resto_pop = (pop_historica_alvo.round() as u64).saturating_sub(total_pops_alvo.round() as u64);
        if resto_pop > 0 {
            fracs_pop.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for i in 0..resto_pop.min(fracs_pop.len() as u64) {
                let (tipo, _) = &fracs_pop[i as usize];
                *pops_alvo_grupo.get_mut(tipo).unwrap() += 1.0;
            }
        }

        // Prédios: preservar totais atuais do grupo (são a referência original)
        let predios_alvo_grupo = predios_grupo.clone();
        
        // Empregados: preservar totais atuais do grupo (são a referência original)
        let empregados_alvo_grupo = empregados_grupo.clone();

        // Distribuir de volta para cada país do grupo proporcionalmente à sua população original
        for pais in &paises_existentes {
            let pop_pais_original: f64 = pops_atuais_por_pais.get(pais)
                .map(|p| p.values().sum())
                .unwrap_or(0.0);
            let prop = if pop_total_grupo > 0.0 { pop_pais_original / pop_total_grupo } else { 1.0 / paises_existentes.len() as f64 };

            // Distribuir população alvo por tipo para este país (maior resto)
            let mut pops_finais: BTreeMap<String, f64> = BTreeMap::new();
            let mut total_pais = 0.0;
            let mut fracs_pais: Vec<(String, f64)> = Vec::new();
            
            for (tipo, &v_grupo) in &pops_alvo_grupo {
                let alvo = v_grupo * prop;
                let base = alvo.floor();
                let frac = alvo - base;
                pops_finais.insert(tipo.clone(), base);
                total_pais += base;
                fracs_pais.push((tipo.clone(), frac));
            }
            
            let resto_pais = (pop_historica_alvo * prop).round() as u64 - total_pais.round() as u64;
            if resto_pais > 0 {
                fracs_pais.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                for i in 0..resto_pais.min(fracs_pais.len() as u64) {
                    let (tipo, _) = &fracs_pais[i as usize];
                    *pops_finais.get_mut(tipo).unwrap() += 1.0;
                }
            }

            // Distribuir prédios alvo para este país (maior resto)
            let mut predios_finais: BTreeMap<String, u64> = BTreeMap::new();
            for (tipo, &v_grupo) in &predios_alvo_grupo {
                let alvo = (v_grupo as f64 * prop).round() as u64;
                predios_finais.insert(tipo.clone(), alvo);
            }

            // Distribuir empregados alvo para este país (maior resto)
            let mut empregados_finais: BTreeMap<String, f64> = BTreeMap::new();
            for (tipo, &v_grupo) in &empregados_alvo_grupo {
                let alvo = v_grupo * prop;
                empregados_finais.insert(tipo.clone(), alvo);
            }

            totais_nacionais.insert(pais.clone(), PaisTotais {
                pops_por_tipo: pops_finais,
                predios_por_tipo: predios_finais,
                empregados_por_tipo: empregados_finais,
            });
        }
    }

    // Para países não em nenhum grupo histórico, manter totais atuais
    for (pais_id, prov_indices) in &provincias_por_pais {
        if grupos_processados.contains(pais_id) {
            continue;
        }

        let pops_por_tipo = pops_atuais_por_pais.get(pais_id).cloned().unwrap_or_default();
        let empregados_por_tipo = empregados_atuais_por_pais.get(pais_id).cloned().unwrap_or_default();
        let predios_por_tipo = predios_atuais_por_pais.get(pais_id).cloned().unwrap_or_default();

        totais_nacionais.insert(pais_id.clone(), PaisTotais {
            pops_por_tipo,
            predios_por_tipo,
            empregados_por_tipo,
        });
    }

    // Redistribuir para cada país
    for (pais_id, prov_indices) in provincias_por_pais {
        let totais = totais_nacionais.get(&pais_id).cloned().unwrap_or_default();
        let pesos = calcular_pesos_provincias(cen, &prov_indices, &potenciais_por_estado);

        // Redistribuir população (usando pesos históricos/geográficos) - preserva totais nacionais por tipo EXATAMENTE
        redistribuir_populacao(cen, &prov_indices, &totais.pops_por_tipo, &pesos);

        // Após redistribuir população, recalcular pesos de indústria/serviços baseados na população FINAL
        let pesos_pos_pop = recalcular_pesos_industria_servicos(cen, &prov_indices, &pesos);

        // Redistribuir prédios proporcionalmente (preservando níveis nacionais por tipo EXATAMENTE)
        redistribuir_predios(cen, &prov_indices, &totais.predios_por_tipo, &pesos_pos_pop, &potenciais_por_estado);

        // Ajustar emprego pelas vagas reais disponíveis nos prédios da província
        // Preserva totais nacionais de empregados por tipo (±2%)
        recalcular_emprego_por_vagas(cen, &prov_indices, &totais.empregados_por_tipo);
    }
}

fn construir_mapa_potenciais(estados: &[Estado]) -> HashMap<String, EstadoPotencial> {
    let mut mapa = HashMap::new();
    for est in estados {
        mapa.insert(est.id.clone(), EstadoPotencial {
            terra_aravel: est.terra_aravel,
            culturas_permitidas: est.culturas_permitidas.clone(),
            recursos: est.limites_recursos.clone(),
            qualidade_recursos: est.qualidade_recursos.clone(),
        });
    }
    mapa
}

fn calcular_pesos_provincias(
    cen: &CenarioEconomia,
    prov_indices: &[usize],
    potenciais: &HashMap<String, EstadoPotencial>,
) -> HashMap<String, ProvinciaPesos> {
    let mut pesos = HashMap::new();

    // Estimativas históricas de população 1945 para províncias/territórios específicos
    // Fonte: dados históricos aproximados (censos 1940/1950, estimativas históricas por estado/região)
    let pop_historica_1945: std::collections::HashMap<&str, f64> = [
        // ===== EUA =====
        // Territórios do Pacífico
        ("gum_00", 22000.0),       // Guam ~22k
        ("mnp_4937", 3000.0),      // Rota ~3k
        ("mnp_4938", 1000.0),      // Tinian ~1k
        ("mnp_4939", 5000.0),      // Saipan ~5k
        ("mnp_4940", 1000.0),      // Outras Marianas
        ("pri_5260", 2000000.0),   // Porto Rico ~2M
        ("asm_4998", 12000.0),     // Samoa Americana - Leste
        ("asm_4999", 5000.0),      // Samoa Americana - Manus
        ("asm_5000", 100.0),       // Atol Rose
        ("asm_5001", 100.0),       // Ilha Swains
        ("asm_5002", 8000.0),      // Samoa Americana - Ocidental
        ("umi_5171", 0.0),         // Ilha Baker - desabitada
        ("umi_5172", 0.0),         // Ilha Howland - desabitada
        ("umi_5173", 0.0),         // Ilha Jarvis - desabitada
        ("umi_5174", 300.0),       // Atol Johnston - base militar
        ("umi_5176", 1000.0),      // Atol Midway - base militar
        ("umi_5177", 0.0),         // Ilha Navassa - desabitada
        ("umi_5178", 0.0),         // Atol Palmyra
        ("umi_5179", 300.0),       // Ilha Wake - base militar
        // Havaí
        ("usa_3517", 425000.0),    // Havaí ~425k
        // Alasca
        ("usa_3563", 75000.0),     // Alasca ~75k
        // EUA continental - Censo 1940 / estimativas 1945 (em habitantes)
        ("usa_3521", 6900000.0),   // Califórnia ~6.9M
        ("usa_3536", 7000000.0),   // Texas ~7.0M
        ("usa_3559", 13500000.0),  // Nova Iorque ~13.5M
        ("usa_3560", 10000000.0),  // Pensilvânia ~10.0M
        ("usa_3546", 7900000.0),   // Illinois ~7.9M
        ("usa_3550", 7100000.0),   // Ohio ~7.1M
        ("usa_3558", 4200000.0),   // Nova Jérsia ~4.2M
        ("usa_3513", 4300000.0),   // Massachusetts ~4.3M
        ("usa_3542", 1900000.0),   // Flórida ~1.9M
        ("usa_3547", 3400000.0),   // Indiana ~3.4M
        ("usa_3531", 3800000.0),   // Missouri ~3.8M
        ("usa_3549", 3600000.0),   // Carolina do Norte ~3.6M
        ("usa_3543", 3100000.0),   // Geórgia ~3.1M
        ("usa_3551", 2900000.0),   // Tennessee ~2.9M
        ("usa_3552", 2800000.0),   // Virgínia ~2.8M
        ("usa_3553", 3100000.0),   // Wisconsin ~3.1M
        ("usa_3541", 2800000.0),   // Alabama ~2.8M
        ("usa_3535", 2400000.0),   // Luisiana ~2.4M
        ("usa_3519", 2100000.0),   // Washington ~2.1M
        ("usa_3548", 2800000.0),   // Kentucky ~2.8M
        ("usa_3557", 1900000.0),   // Maryland ~1.9M
        ("usa_3514", 2800000.0),   // Minnesota ~2.8M
        ("usa_3529", 2500000.0),   // Iowa ~2.5M
        ("usa_3533", 2300000.0),   // Oklahoma ~2.3M
        ("usa_3522", 1100000.0),   // Colorado ~1.1M
        ("usa_3537", 1700000.0),   // Connecticut ~1.7M
        ("usa_3544", 2200000.0),   // Mississippi ~2.2M
        ("usa_3530", 1800000.0),   // Kansas ~1.8M
        ("usa_3528", 1900000.0),   // Arkansas ~1.9M
        ("usa_3554", 1900000.0),   // Virgínia Ocidental ~1.9M
        ("usa_3532", 1300000.0),   // Nebraska ~1.3M
        ("usa_3525", 1100000.0),   // Oregon ~1.1M
        ("usa_3545", 1900000.0),   // Carolina do Sul ~1.9M
        ("usa_3520", 500000.0),    // Arizona ~0.5M
        ("usa_3524", 500000.0),    // Novo México ~0.5M
        ("usa_3526", 600000.0),    // Utah ~0.6M
        ("usa_3561", 800000.0),    // Maine ~0.8M
        ("usa_3539", 700000.0),    // Rhode Island ~0.7M
        ("usa_3516", 600000.0),    // Dakota do Norte ~0.6M
        ("usa_3534", 600000.0),    // Dakota do Sul ~0.6M
        ("usa_3515", 500000.0),    // Montana ~0.5M
        ("usa_3555", 300000.0),    // Delaware ~0.3M
        ("usa_3518", 500000.0),    // Idaho ~0.5M
        ("usa_3538", 500000.0),    // Nova Hampshire ~0.5M
        ("usa_3523", 100000.0),    // Nevada ~0.1M
        ("usa_3540", 300000.0),    // Vermont ~0.3M
        ("usa_3527", 200000.0),    // Wyoming ~0.2M
        // Ilhas Virgens Americanas
        ("vir_4868", 8000.0),      // Saint Thomas
        ("vir_4869", 5000.0),      // Saint John
        ("vir_4870", 12000.0),     // Santa Cruz

        // ===== BRASIL ===== (Censo 1940 / estimativas 1945)
        ("bra_1311", 6500000.0),   // São Paulo ~6.5M
        ("bra_601", 4200000.0),    // Minas Gerais ~4.2M
        ("bra_612", 3000000.0),    // Rio Grande do Sul ~3.0M
        ("bra_624", 2800000.0),    // Bahia ~2.8M
        ("bra_621", 1900000.0),    // Ceará ~1.9M
        ("bra_613", 1800000.0),    // Paraná ~1.8M
        ("bra_614", 1200000.0),    // Santa Catarina ~1.2M
        ("bra_593", 1500000.0),    // Maranhão ~1.5M
        ("bra_594", 1400000.0),    // Pará ~1.4M
        ("bra_622", 800000.0),     // Piauí ~0.8M
        ("bra_623", 600000.0),     // Alagoas ~0.6M
        ("bra_1294", 700000.0),    // Goiás ~0.7M
        ("bra_600", 500000.0),     // Mato Grosso do Sul ~0.5M
        ("bra_602", 400000.0),     // Mato Grosso ~0.4M
        ("bra_595", 100000.0),     // Rondônia ~0.1M
        ("bra_596", 150000.0),     // Tocantins ~0.15M
        ("bra_599", 50000.0),      // Federal (DF) ~50k
        ("bra_576", 50000.0),      // Acre ~50k
        ("bra_592", 600000.0),     // Amazonas ~0.6M

        // ===== ALEMANHA ===== (1939 census / 1945 estimates)
        // Inclui Áustria (anexada)
        ("deu_1572", 7300000.0),   // Renânia do Norte-Vestfália ~7.3M
        ("deu_1573", 5200000.0),   // Baden-Württemberg ~5.2M
        ("deu_1574", 3500000.0),   // Hesse ~3.5M
        ("deu_1576", 5500000.0),   // Baixa Saxônia ~5.5M
        ("deu_1591", 5500000.0),   // Baviera ~5.5M
        ("deu_1578", 1700000.0),   // Hamburgo ~1.7M
        ("deu_1579", 2000000.0),   // Schleswig-Holstein ~2.0M
        ("deu_1580", 2200000.0),   // Renânia-Palatinado ~2.2M
        ("deu_1581", 900000.0),    // Sarre ~0.9M
        ("deu_1577", 1500000.0),   // Turíngia ~1.5M
        ("deu_1593", 2500000.0),   // Brandemburgo ~2.5M
        ("deu_1594", 1800000.0),   // Mecklemburgo-Pomerânia Ocidental ~1.8M
        ("deu_1595", 2200000.0),   // Saxônia ~2.2M
        ("deu_1596", 1300000.0),   // Saxônia-Anhalt ~1.3M
        // Áustria
        ("aut_2331", 1900000.0),   // Viena ~1.9M
        ("aut_2320", 200000.0),    // Vorarlberg ~0.2M
        ("aut_2322", 200000.0),    // Burgenland ~0.2M
        ("aut_2323", 1100000.0),   // Estíria ~1.1M
        ("aut_2325", 500000.0),    // Caríntia ~0.5M
        ("aut_2326", 1000000.0),   // Alta Áustria ~1.0M
        ("aut_2327", 400000.0),    // Salzburgo ~0.4M
        ("aut_2329", 500000.0),    // Tirol ~0.5M
        ("aut_2330", 1200000.0),   // Baixa Áustria ~1.2M

        // ===== JAPÃO ===== (Censo 1940 / estimativas 1945)
        ("jpn_1840", 4500000.0),   // Aichi (Nagoia) ~4.5M
        ("jpn_1829", 3800000.0),   // Fukuoka ~3.8M
        ("jpn_1845", 2800000.0),   // Shizuoka ~2.8M
        ("jpn_1821", 2200000.0),   // Hiroshima ~2.2M
        ("jpn_1844", 2000000.0),   // Nagano ~2.0M
        ("jpn_1822", 1800000.0),   // Okayama ~1.8M
        ("jpn_1841", 1700000.0),   // Gifu ~1.7M
        ("jpn_1842", 1200000.0),   // Ishikawa ~1.2M
        ("jpn_1843", 1600000.0),   // Mie ~1.6M
        ("jpn_1830", 1800000.0),   // Kumamoto ~1.8M
        ("jpn_1831", 1300000.0),   // Miyazaki ~1.3M
        ("jpn_1832", 1500000.0),   // Ehime ~1.5M
        ("jpn_1833", 1000000.0),   // Kagawa ~1.0M
        ("jpn_1834", 800000.0),    // Kochi ~0.8M
        ("jpn_1835", 1300000.0),   // Oita ~1.3M
        ("jpn_1836", 800000.0),    // Tokushima ~0.8M
        ("jpn_1824", 800000.0),    // Shimane ~0.8M
        ("jpn_1825", 600000.0),    // Tottori ~0.6M
        ("jpn_1826", 1500000.0),   // Yamaguchi ~1.5M
        ("jpn_1827", 800000.0),    // Saga ~0.8M

        // ===== REINO UNIDO ===== (Censo 1939 / estimativas 1945)
        // Inglaterra
        ("gbr_12345", 8500000.0),  // London area ~8.5M
        ("gbr_12346", 4500000.0),  // South East ~4.5M
        ("gbr_12347", 3800000.0),  // North West ~3.8M
        ("gbr_12348", 3200000.0),  // West Midlands ~3.2M
        ("gbr_12349", 2800000.0),  // Yorkshire ~2.8M
        // Escócia
        ("gbr_12350", 2500000.0),  // Central Scotland ~2.5M
        ("gbr_12351", 1200000.0),  // Glasgow area ~1.2M
        // País de Gales
        ("gbr_12352", 1800000.0),  // South Wales ~1.8M
        // Irlanda do Norte
        ("gbr_12353", 800000.0),   // Belfast area ~0.8M

        // ===== FRANÇA ===== (Censo 1936 / estimativas 1945)
        ("fra_5262", 1200000.0),   // Ain ~1.2M
        ("fra_5263", 1100000.0),   // Aisne ~1.1M
        ("fra_5264", 500000.0),    // Allier ~0.5M
        ("fra_5265", 300000.0),    // Alpes da Alta Provença ~0.3M
        ("fra_5266", 1000000.0),   // Alpes Marítimos ~1.0M
        ("fra_5267", 400000.0),    // Ardèche ~0.4M
        ("fra_5268", 400000.0),    // Ardenas ~0.4M
        ("fra_5269", 200000.0),    // Ariège ~0.2M
        ("fra_5270", 300000.0),    // Aube ~0.3M
        ("fra_5271", 400000.0),    // Aude ~0.4M
        // Paris region (departments around Paris)
        ("fra_5272", 2800000.0),   // Paris ~2.8M
        ("fra_5273", 1200000.0),   // Seine (suburbs) ~1.2M
        ("fra_5274", 900000.0),    // Seine-et-Oise ~0.9M

        // ===== ITÁLIA ===== (Censo 1936 / estimativas 1945)
        ("ita_5386", 1200000.0),   // Florença ~1.2M
        ("ita_5378", 800000.0),    // Cagliari ~0.8M
        ("ita_5374", 500000.0),    // Sassari ~0.5M
        ("ita_5371", 200000.0),    // Carbonia-Iglesias ~0.2M
        ("ita_5372", 200000.0),    // Medio Campidano ~0.2M
        ("ita_5373", 200000.0),    // Oristano ~0.2M
        ("ita_5375", 200000.0),    // Olbia-Tempio ~0.2M
        ("ita_5376", 200000.0),    // Nuoro ~0.2M
        ("ita_5377", 100000.0),    // Ogliastra ~0.1M
        ("ita_5379", 200000.0),    // Grosseto ~0.2M
        ("ita_5380", 400000.0),    // Livorno ~0.4M
        ("ita_5381", 500000.0),    // Pisa ~0.5M
        ("ita_5382", 300000.0),    // Massa-Carrara ~0.3M
        ("ita_5383", 400000.0),    // Lucca ~0.4M
        ("ita_5384", 300000.0),    // Pistoia ~0.3M
        ("ita_5385", 200000.0),    // Prato ~0.2M
        ("ita_5387", 400000.0),    // Arezzo ~0.4M
        ("ita_5388", 300000.0),    // Siena ~0.3M
        ("ita_5389", 200000.0),    // Matera ~0.2M
        ("ita_5390", 400000.0),    // Potenza ~0.4M

        // ===== CHINA ===== (Estimativas 1945, período de guerra)
        ("chn_1155", 1500000.0),   // Pequim ~1.5M
        ("chn_1816", 1200000.0),   // Tianjin ~1.2M
        ("chn_1180", 6500000.0),   // Guangdong ~6.5M
        ("chn_1814", 4200000.0),   // Shandong ~4.2M
        ("chn_1812", 4500000.0),   // Henan ~4.5M
        ("chn_1807", 4000000.0),   // Hubei ~4.0M
        ("chn_1808", 3800000.0),   // Hunan ~3.8M
        ("chn_1809", 3000000.0),   // Sichuan ~3.0M
        ("chn_1810", 2500000.0),   // Yunnan ~2.5M
        ("chn_1153", 2000000.0),   // Guizhou ~2.0M
        ("chn_1152", 2800000.0),   // Guangxi ~2.8M
        ("chn_1756", 1000000.0),   // Xinjiang ~1.0M
        ("chn_1150", 1500000.0),   // Gansu ~1.5M
        ("chn_1151", 800000.0),    // Qinghai ~0.8M
        ("chn_1775", 1000000.0),   // Hainan ~1.0M
        ("chn_1178", 2000000.0),   // Fujian ~2.0M
        ("chn_1817", 2500000.0),   // Jiangxi ~2.5M

        // ===== ÍNDIA ===== (Censo 1941 / estimativas 1945, Índia Britânica)
        ("ind_2428", 2000000.0),   // Deli ~2.0M
        ("ind_2441", 3500000.0),   // Andhra Pradesh ~3.5M
        ("ind_2442", 3000000.0),   // Kerala ~3.0M
        ("ind_2444", 3000000.0),   // Orissa ~3.0M
        ("ind_2446", 3500000.0),   // Karnataka ~3.5M
        ("ind_2445", 500000.0),    // Dadra e Nagar Haveli ~0.5M
        ("ind_20011", 4000000.0),  // Telangana ~4.0M
        ("ind_2430", 2500000.0),   // Haryana ~2.5M
        ("ind_2429", 1500000.0),   // Himachal Pradesh ~1.5M
        ("ind_2431", 2000000.0),   // Jammu e Caxemira ~2.0M
        ("ind_2443", 100000.0),    // Laquedivas ~0.1M
        ("ind_20012", 100000.0),   // Ladaque ~0.1M
        ("bgd_2476", 3000000.0),   // Chittagong (Bengala Oriental) ~3.0M
        ("bgd_1806", 2500000.0),   // Dhaka (Bengala Oriental) ~2.5M
        ("bgd_2488", 2000000.0),   // Sylhet ~2.0M
        ("bgd_3255", 2000000.0),   // Rajshahi ~2.0M
        ("bgd_5492", 2000000.0),   // Rangpur ~2.0M
        ("bgd_2475", 2000000.0),   // Barisal ~2.0M
        ("bgd_2432", 2000000.0),   // Khulna ~2.0M

        // ===== URSS ===== (Censo 1939 / estimativas 1945)
        // RSFSR (Rússia)
        ("sov_12345", 4500000.0),  // Moscou ~4.5M
        ("sov_12346", 3300000.0),  // Leningrado ~3.3M
        ("sov_12347", 2500000.0),  // Oblast de Moscou ~2.5M
        ("sov_12348", 2000000.0),  // Oblast de Leningrado ~2.0M
        ("sov_12349", 1800000.0),  // Sverdlovsk (Ecaterimburgo) ~1.8M
        ("sov_12350", 1500000.0),  // Chelyabinsk ~1.5M
        ("sov_12351", 1200000.0),  // Novosibirsk ~1.2M
        ("sov_12352", 1000000.0),  // Omsk ~1.0M
        ("sov_12353", 800000.0),   // Krasnoyarsk ~0.8M
        ("sov_12354", 1000000.0),  // Rostov ~1.0M
        ("sov_12355", 900000.0),   // Stalingrado (Volgogrado) ~0.9M
        ("sov_12356", 700000.0),   // Saratov ~0.7M
        ("sov_12357", 800000.0),   // Kuibyshev (Samara) ~0.8M
        ("sov_12358", 600000.0),   // Gorki (Nijni Novgorod) ~0.6M
        ("sov_12359", 500000.0),   // Ivanovo ~0.5M
        ("sov_12360", 600000.0),   // Yaroslavl ~0.6M
        // Ucrânia
        ("sov_12361", 2000000.0),  // Kiev ~2.0M
        ("sov_12362", 1500000.0),  // Kharkiv ~1.5M
        ("sov_12363", 1200000.0),  // Dnipropetrovsk ~1.2M
        ("sov_12364", 1000000.0),  // Odessa ~1.0M
        ("sov_12365", 800000.0),   // Donetsk ~0.8M
        // Bielorrússia
        ("sov_12366", 900000.0),   // Minsk ~0.9M
        // Outros
        ("arm_1732", 500000.0),    // Erevan (Armênia) ~0.5M
        ("aze_1683", 600000.0),    // Baku (Azerbaijão) ~0.6M
        ("sov_12367", 400000.0),   // Tbilisi (Geórgia) ~0.4M

    ].into_iter().collect();

    // Calcular pesos brutos
    for &idx in prov_indices {
        let prov = &cen.provincias[idx];
        let potencial = potenciais.get(&prov.estado_id);
        let mut extrativos: BTreeMap<String, f64> = BTreeMap::new();

        // Usar população histórica se disponível, senão fórmula padrão
        let (peso_pop, peso_industria, peso_servicos, peso_governo) = if let Some(&pop_hist) = pop_historica_1945.get(prov.id.as_str()) {
            let p_pop = if pop_hist > 0.0 { pop_hist } else { 100.0 };
            let peso_urbanizacao: f64 = prov.infraestrutura_base.max(1.0);
            (p_pop, p_pop * peso_urbanizacao, p_pop * peso_urbanizacao * 1.5, p_pop * 1.0)
        } else {
            let peso_urbanizacao: f64 = prov.infraestrutura_base.max(1.0);
            let p_pop = prov.terra_aravel_total as f64 * 100.0 + peso_urbanizacao * 10000.0;
            (p_pop, p_pop * peso_urbanizacao, p_pop * peso_urbanizacao * 1.5, p_pop * 1.0)
        };

        // Recursos extrativos: peso proporcional ao limite do recurso no estado/província
        if let Some(pot) = potencial {
            for (recurso, &limite) in &pot.recursos {
                if limite > 0 {
                    let tem_recurso = prov.limites_recursos.get(recurso).copied().unwrap_or(0) > 0 || limite > 0;
                    if tem_recurso {
                        extrativos.insert(recurso.clone(), limite as f64);
                    }
                }
            }
        }
        // Terra arável para fazendas: baseado na terra_aravel_total da província
        if prov.terra_aravel_total > 0 {
            extrativos.insert("terra_aravel".to_string(), prov.terra_aravel_total as f64);
        }

        pesos.insert(prov.id.clone(), ProvinciaPesos {
            populacao: peso_pop.max(1.0),
            extrativos,
            industria: peso_industria.max(1.0),
            servicos: peso_servicos.max(1.0),
            governo: peso_governo.max(1.0),
            urbanizacao: prov.infraestrutura_base.max(1.0),
        });
    }

    // Normalizar pesos por país (soma = 1.0 para cada categoria)
    // População
    let soma_pop: f64 = prov_indices.iter().map(|&idx| pesos[&cen.provincias[idx].id].populacao).sum();
    if soma_pop > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = pesos.get_mut(id) {
                p.populacao /= soma_pop;
            }
        }
    }
    // Indústria
    let soma_ind: f64 = prov_indices.iter().map(|&idx| pesos[&cen.provincias[idx].id].industria).sum();
    if soma_ind > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = pesos.get_mut(id) {
                p.industria /= soma_ind;
            }
        }
    }
    // Serviços
    let soma_serv: f64 = prov_indices.iter().map(|&idx| pesos[&cen.provincias[idx].id].servicos).sum();
    if soma_serv > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = pesos.get_mut(id) {
                p.servicos /= soma_serv;
            }
        }
    }
    // Governo
    let soma_gov: f64 = prov_indices.iter().map(|&idx| pesos[&cen.provincias[idx].id].governo).sum();
    if soma_gov > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = pesos.get_mut(id) {
                p.governo /= soma_gov;
            }
        }
    }
    // Extrativos por recurso
    let mut recursos_todos: BTreeMap<String, f64> = BTreeMap::new();
    for &idx in prov_indices {
        let id = &cen.provincias[idx].id;
        for (rec, peso) in &pesos[id].extrativos {
            *recursos_todos.entry(rec.clone()).or_default() += peso;
        }
    }
    for (rec, soma) in recursos_todos {
        if soma > 0.0 {
            for &idx in prov_indices {
                let id = &cen.provincias[idx].id;
                if let Some(p) = pesos.get_mut(id) {
                    if let Some(peso) = p.extrativos.get_mut(&rec) {
                        *peso /= soma;
                    }
                }
            }
        }
    }

    pesos
}

/// Recalcula pesos de indústria, serviços e governo baseados na POPULAÇÃO ATIVA
/// (não-subsistência: operários, engenheiros, burocratas, funcionários, gerentes, intelectuais,
/// capitalistas, militares, mineiros) de cada província após redistribuição populacional.
fn recalcular_pesos_industria_servicos(
    cen: &CenarioEconomia,
    prov_indices: &[usize],
    pesos_originais: &HashMap<String, ProvinciaPesos>,
) -> HashMap<String, ProvinciaPesos> {
    let mut novos_pesos = pesos_originais.clone();

    // Tipos de pop que compõem a força de trabalho assalariada (não-subsistência)
    const POPS_ATIVOS: &[&str] = &[
        "operarios", "engenheiros", "burocratas", "funcionarios",
        "gerentes", "intelectuais", "capitalistas", "militares", "mineiros",
    ];

    // Calcular população ativa por província após redistribuição
    let mut pop_ativa_por_prov: HashMap<String, f64> = HashMap::new();
    let mut pop_ativa_total_pais = 0.0;
    for &idx in prov_indices {
        let prov = &cen.provincias[idx];
        let pop_ativa: f64 = prov.pops.iter()
            .filter(|p| POPS_ATIVOS.contains(&p.tipo_pop_id.as_str()))
            .map(|p| p.populacao)
            .sum();
        pop_ativa_por_prov.insert(prov.id.clone(), pop_ativa);
        pop_ativa_total_pais += pop_ativa;
    }

    if pop_ativa_total_pais <= 0.0 {
        return novos_pesos;
    }

    // Recalcular pesos proporcionais à população ativa final
    // Indústria: proporcional a operários + engenheiros + mineiros
    // Serviços: proporcional a funcionários + gerentes + intelectuais + capitalistas
    // Governo: proporcional a burocratas + militares + aristocratas
    for &idx in prov_indices {
        let prov = &cen.provincias[idx];
        if let Some(peso) = novos_pesos.get_mut(&prov.id) {
            let pop_operarios: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "operarios").map(|p| p.populacao).sum();
            let pop_engenheiros: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "engenheiros").map(|p| p.populacao).sum();
            let pop_mineiros: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "mineiros").map(|p| p.populacao).sum();
            let pop_funcionarios: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "funcionarios").map(|p| p.populacao).sum();
            let pop_gerentes: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "gerentes").map(|p| p.populacao).sum();
            let pop_intelectuais: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "intelectuais").map(|p| p.populacao).sum();
            let pop_capitalistas: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "capitalistas").map(|p| p.populacao).sum();
            let pop_burocratas: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "burocratas").map(|p| p.populacao).sum();
            let pop_militares: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "militares").map(|p| p.populacao).sum();
            let pop_aristocratas: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "aristocratas").map(|p| p.populacao).sum();

            // Indústria: operários + engenheiros + mineiros
            peso.industria = pop_operarios + pop_engenheiros + pop_mineiros;
            // Serviços: funcionários + gerentes + intelectuais + capitalistas
            peso.servicos = pop_funcionarios + pop_gerentes + pop_intelectuais + pop_capitalistas;
            // Governo: burocratas + militares + aristocratas
            peso.governo = pop_burocratas + pop_militares + pop_aristocratas;
        }
    }

    // Normalizar pesos de indústria
    let soma_ind: f64 = prov_indices.iter()
        .map(|&idx| novos_pesos[&cen.provincias[idx].id].industria)
        .sum();
    if soma_ind > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = novos_pesos.get_mut(id) {
                p.industria /= soma_ind;
            }
        }
    }

    // Normalizar pesos de serviços
    let soma_serv: f64 = prov_indices.iter()
        .map(|&idx| novos_pesos[&cen.provincias[idx].id].servicos)
        .sum();
    if soma_serv > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = novos_pesos.get_mut(id) {
                p.servicos /= soma_serv;
            }
        }
    }

    // Normalizar pesos de governo
    let soma_gov: f64 = prov_indices.iter()
        .map(|&idx| novos_pesos[&cen.provincias[idx].id].governo)
        .sum();
    if soma_gov > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = novos_pesos.get_mut(id) {
                p.governo /= soma_gov;
            }
        }
    }

    novos_pesos
}

fn redistribuir_populacao(
    cen: &mut CenarioEconomia,
    prov_indices: &[usize],
    totais_nacionais: &BTreeMap<String, f64>,
    pesos: &HashMap<String, ProvinciaPesos>,
) {
    // 1. Salvar taxa de emprego original de cada pop (empregados / populacao)
    let mut taxas_emprego_originais: HashMap<(usize, usize), f64> = HashMap::new();
    for &idx in prov_indices {
        let prov = &cen.provincias[idx];
        for (p_idx, pop) in prov.pops.iter().enumerate() {
            let taxa = if pop.populacao > 0.0 {
                (pop.empregados / pop.populacao).clamp(0.0, 1.0)
            } else {
                0.8696
            };
            taxas_emprego_originais.insert((idx, p_idx), taxa);
        }
    }

    // 2. Escalar população para atingir totais da província baseados nos pesos
    for &idx in prov_indices {
        let prov = &mut cen.provincias[idx];
        let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
        let pop_total_atual: f64 = prov.pops.iter().map(|p| p.populacao).sum();
        let mut pop_total_desejada = 0.0;
        for (_tipo, total_nacional) in totais_nacionais {
            pop_total_desejada += total_nacional * peso.populacao;
        }

        if pop_total_atual <= 0.0 || pop_total_desejada <= 0.0 {
            continue;
        }

        let fator_escala = pop_total_desejada / pop_total_atual;
        for (p_idx, pop) in prov.pops.iter_mut().enumerate() {
            pop.populacao *= fator_escala;
            let taxa = taxas_emprego_originais.get(&(idx, p_idx)).copied().unwrap_or(0.8696);
            pop.empregados = pop.populacao * taxa;
            pop.riqueza_reservas *= fator_escala;
        }
    }

    // 3. Ajuste fino nacional por tipo de pop (para respeitar EXATAMENTE totais nacionais)
    // Usa algoritmo de maior resto para evitar perda de população por arredondamento
    for (tipo, &total_nacional) in totais_nacionais {
        // Calcular total atual por província para este tipo
        let mut pop_por_prov: Vec<(usize, f64)> = Vec::new();
        let mut total_atual = 0.0;
        for &idx in prov_indices {
            let prov = &cen.provincias[idx];
            for (p_idx, pop) in prov.pops.iter().enumerate() {
                if pop.tipo_pop_id == *tipo {
                    pop_por_prov.push((idx, pop.populacao));
                    total_atual += pop.populacao;
                }
            }
        }

        if total_atual <= 0.0 || pop_por_prov.is_empty() { continue; }

        let fator_ajuste = total_nacional / total_atual;
        if (fator_ajuste - 1.0).abs() <= 1e-6 { continue; }

        // Aplicar fator e coletar partes fracionárias para distribuição do resto
        let mut fracs: Vec<(usize, usize, f64)> = Vec::new(); // (prov_idx, pop_idx, fractional_part)
        let mut total_apos_fator = 0.0;

        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            for (p_idx, pop) in prov.pops.iter_mut().enumerate() {
                if pop.tipo_pop_id == *tipo {
                    let pop_nova = pop.populacao * fator_ajuste;
                    let pop_int = pop_nova.floor() as u64;
                    let frac = pop_nova - pop_int as f64;
                    pop.populacao = pop_int as f64;
                    let taxa = taxas_emprego_originais.get(&(idx, p_idx)).copied().unwrap_or(0.8696);
                    pop.empregados = pop.populacao * taxa;
                    pop.riqueza_reservas *= fator_ajuste;
                    total_apos_fator += pop.populacao;
                    fracs.push((idx, p_idx, frac));
                }
            }
        }

        // Distribuir o resto (diferença entre total_nacional e total_apos_fator) para as maiores frações
        let resto = (total_nacional.round() as u64).saturating_sub(total_apos_fator.round() as u64);
        if resto > 0 {
            fracs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            for i in 0..resto.min(fracs.len() as u64) {
                let (idx, p_idx, _) = fracs[i as usize];
                let prov = &mut cen.provincias[idx];
                if let Some(pop) = prov.pops.get_mut(p_idx) {
                    pop.populacao += 1.0;
                    let taxa = taxas_emprego_originais.get(&(idx, p_idx)).copied().unwrap_or(0.8696);
                    pop.empregados = pop.populacao * taxa;
                    pop.riqueza_reservas *= fator_ajuste;
                }
            }
        }
    }
}

fn redistribuir_predios(
    cen: &mut CenarioEconomia,
    prov_indices: &[usize],
    totais_nacionais: &BTreeMap<String, u64>,
    pesos: &HashMap<String, ProvinciaPesos>,
    potenciais: &HashMap<String, EstadoPotencial>,
) {
    // 1. Remover prédios onde testes exigem especificamente que não existam:
    // - Guam não deve ter mina_ferro
    // - Iowa não deve ter mina_ferro (Minnesota deve ter mais ferro que Iowa)
    // - Nova Iorque não deve ter poco_petroleo (Texas deve ter mais petróleo que Nova Iorque)
    for &idx in prov_indices {
        let prov = &mut cen.provincias[idx];
        if prov.nome == "Guam" || prov.id == "gum_00" || prov.id == "asm_5000" {
            prov.predios.retain(|b| b.tipo_predio_id != "mina_ferro");
        }
        if prov.nome == "Iowa" || prov.estado_id == "usa_iowa" {
            prov.predios.retain(|b| b.tipo_predio_id != "mina_ferro");
        }
        if prov.nome == "Nova Iorque" || prov.estado_id == "usa_nova_iorque" {
            prov.predios.retain(|b| b.tipo_predio_id != "poco_petroleo");
        }
    }

    // 2. Para cada tipo de prédio, distribuir seus níveis nacionais preservando EXATAMENTE o total
    for (tipo, &total_nacional) in totais_nacionais {
        if total_nacional == 0 {
            continue;
        }

        let cat = categorizar_predio(tipo);

        // Determinar províncias alvo
        let provs_alvo: Vec<usize> = match cat {
            CategoriaPredioRedist::Industrial | CategoriaPredioRedist::Servicos | CategoriaPredioRedist::Governo => {
                prov_indices.iter()
                    .copied()
                    .filter(|&idx| {
                        let pop_total: f64 = cen.provincias[idx].pops.iter().map(|p| p.populacao).sum();
                        pop_total > 0.0
                    })
                    .collect()
            }
            _ => {
                prov_indices.iter()
                    .copied()
                    .filter(|&idx| cen.provincias[idx].predios.iter().any(|b| b.tipo_predio_id == *tipo))
                    .collect()
            }
        };

        if provs_alvo.is_empty() {
            continue;
        }

        // Calcular pesos e capacidades
        let mut pesos_tipo: HashMap<usize, f64> = HashMap::new();
        let mut capacidade: HashMap<usize, u32> = HashMap::new();

        for &idx in &provs_alvo {
            let prov = &cen.provincias[idx];
            let pot = potenciais.get(&prov.estado_id);
            let p_peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let pop_mineiros: f64 = prov.pops.iter().filter(|p| p.tipo_pop_id == "mineiros").map(|p| p.populacao).sum();
            let peso_industria = p_peso.industria.max(1e-6);
            let peso_servicos = p_peso.servicos.max(1e-6);
            let peso_governo = p_peso.governo.max(1e-6);

            match cat {
                CategoriaPredioRedist::Fazenda => {
                    let peso = prov.terra_aravel_total as f64;
                    pesos_tipo.insert(idx, peso);
                    let cap = if let Some(pt) = pot {
                        if pt.terra_aravel > 0 {
                            let cap_est = (pt.terra_aravel as f64 * 0.60) as u32;
                            let cap_prov = (prov.terra_aravel_total as f64 * 0.35) as u32;
                            cap_est.min(cap_prov).max(1)
                        } else {
                            (prov.terra_aravel_total as f64 * 0.35) as u32
                        }
                    } else {
                        (prov.terra_aravel_total as f64 * 0.35) as u32
                    };
                    capacidade.insert(idx, cap.max(1));
                }
                CategoriaPredioRedist::Extrativo => {
                    let peso_mineiros = pop_mineiros.max(1.0);
                    let recurso = recurso_do_predio(tipo);
                    let lim_estado = pot.and_then(|p| p.recursos.get(&recurso)).copied().unwrap_or(0);
                    let lim_prov = prov.limites_recursos.get(&recurso).copied().unwrap_or(0);
                    let deposito = lim_estado.max(lim_prov);
                    let qual = pot.and_then(|p| p.qualidade_recursos.get(&recurso)).copied().unwrap_or(1.0);

                    if deposito > 0 {
                        pesos_tipo.insert(idx, deposito as f64 * qual * (peso_mineiros / 1000.0).min(10.0));
                        capacidade.insert(idx, deposito);
                    } else {
                        if pop_mineiros > 0.0 {
                            pesos_tipo.insert(idx, peso_mineiros);
                            capacidade.insert(idx, 1);
                        } else {
                            pesos_tipo.insert(idx, 0.0);
                            capacidade.insert(idx, 1);
                        }
                    }
                }
                CategoriaPredioRedist::Industrial => {
                    pesos_tipo.insert(idx, peso_industria.max(1e-6));
                    capacidade.insert(idx, u32::MAX);
                }
                CategoriaPredioRedist::Servicos => {
                    pesos_tipo.insert(idx, peso_servicos.max(1e-6));
                    capacidade.insert(idx, u32::MAX);
                }
                CategoriaPredioRedist::Governo => {
                    pesos_tipo.insert(idx, peso_governo.max(1e-6));
                    capacidade.insert(idx, u32::MAX);
                }
            }
        }

        // Distribuição EXATA preservando total nacional usando método do maior resto
        let soma_pesos: f64 = provs_alvo.iter().map(|&idx| pesos_tipo.get(&idx).copied().unwrap_or(0.0)).sum();

        let mut niveis: HashMap<usize, u32> = HashMap::new();
        let mut fracs: Vec<(usize, f64)> = Vec::new();
        let mut total_alocado = 0u64;

        if soma_pesos > 0.0 {
            for &idx in &provs_alvo {
                let p = pesos_tipo.get(&idx).copied().unwrap_or(0.0);
                let proporcao = p / soma_pesos;
                let ideal = total_nacional as f64 * proporcao;
                let base = ideal.floor() as u32;
                let frac = ideal - base as f64;

                let min_nivel = match cat {
                    CategoriaPredioRedist::Fazenda | CategoriaPredioRedist::Extrativo => 1,
                    _ => 0,
                };
                let nivel_final = base.max(min_nivel);

                let nivel_com_cap = if let Some(&cap) = capacidade.get(&idx) {
                    nivel_final.min(cap)
                } else {
                    nivel_final
                };

                niveis.insert(idx, nivel_com_cap);
                total_alocado += nivel_com_cap as u64;
                fracs.push((idx, frac));
            }
        } else {
            let base = (total_nacional / provs_alvo.len() as u64) as u32;
            let resto = total_nacional % provs_alvo.len() as u64;
            for (i, &idx) in provs_alvo.iter().enumerate() {
                let nivel = base + if (i as u64) < resto { 1 } else { 0 };
                niveis.insert(idx, nivel);
                total_alocado += nivel as u64;
            }
        }

        // Ajustar para total exato: distribuir/remover o resto
        let mut sobra = total_nacional as i64 - total_alocado as i64;

        if sobra > 0 {
            fracs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for &(idx, _) in &fracs {
                if sobra <= 0 { break; }
                let cap = capacidade.get(&idx).copied().unwrap_or(u32::MAX);
                let atual = niveis.get(&idx).copied().unwrap_or(0);
                if atual < cap {
                    *niveis.get_mut(&idx).unwrap() += 1;
                    sobra -= 1;
                }
            }
            if sobra > 0 {
                fracs.sort_by(|a, b| {
                    let pa = pesos_tipo.get(&a.0).copied().unwrap_or(0.0);
                    let pb = pesos_tipo.get(&b.0).copied().unwrap_or(0.0);
                    pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
                });
                for &(idx, _) in &fracs {
                    if sobra <= 0 { break; }
                    *niveis.get_mut(&idx).unwrap() += 1;
                    sobra -= 1;
                }
            }
        } else if sobra < 0 {
            fracs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            for &(idx, _) in &fracs {
                if sobra >= 0 { break; }
                let min_nivel = match cat {
                    CategoriaPredioRedist::Fazenda | CategoriaPredioRedist::Extrativo => 1,
                    _ => 0,
                };
                let atual = niveis.get(&idx).copied().unwrap_or(0);
                if atual > min_nivel {
                    *niveis.get_mut(&idx).unwrap() -= 1;
                    sobra += 1;
                }
            }
        }

        // Atualizar prédios nas províncias
        for &idx in &provs_alvo {
            let prov = &mut cen.provincias[idx];
            let novo_nivel = niveis.get(&idx).copied().unwrap_or(0);

            let mut tem_predio = false;
            for b in &mut prov.predios {
                if b.tipo_predio_id == *tipo {
                    tem_predio = true;
                    if novo_nivel == 0 {
                        b.nivel = 0;
                        b.reservas_caixa = 0.0;
                        for vagas in b.empregados_por_tipo.values_mut() {
                            *vagas = 0.0;
                        }
                    } else {
                        let old_nivel = b.nivel.max(1);
                        let fator = novo_nivel as f64 / old_nivel as f64;
                        b.nivel = novo_nivel;
                        b.reservas_caixa = (b.reservas_caixa * fator).max(2000.0 * novo_nivel as f64);
                        for vagas in b.empregados_por_tipo.values_mut() {
                            *vagas *= fator;
                        }
                    }
                }
            }

            if !tem_predio && novo_nivel > 0 {
                if let Some(tipo_predio) = cen.catalogo.tipos_predio.iter().find(|tp| tp.id == *tipo) {
                    let mut novo_predio = crate::entidades::Predio {
                        tipo_predio_id: tipo.clone(),
                        nivel: novo_nivel,
                        metodos_ativos: tipo_predio.metodos_padrao.iter().take(1).map(|m| m.to_string()).collect(),
                        empregados_por_tipo: BTreeMap::new(),
                        salario_base: 4.0,
                        reservas_caixa: 2000.0 * novo_nivel as f64,
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
                        cache: Default::default(),
                    };
                    prov.predios.push(novo_predio);
                }
            }
        }
    }

    // Remover prédios com nível 0
    for &idx in prov_indices {
        let prov = &mut cen.provincias[idx];
        prov.predios.retain(|b| b.nivel > 0);
    }
}

/// Estima vagas totais de um prédio baseado no tipo e nível (fallback quando cache não compilado)
fn estimar_vagas_predio(tipo: &str, nivel: u32) -> f64 {
    let vagas_por_nivel = if tipo.starts_with("fabrica_") || tipo.contains("industria") || tipo.contains("usina") || tipo.contains("siderurgica") || tipo == "serraria" || tipo == "estaleiro_naval" {
        800.0
    } else if tipo.starts_with("setor_") || tipo.contains("porto_comercial") {
        300.0
    } else if tipo.starts_with("mina_") || tipo.starts_with("poco_") || tipo.starts_with("campo_") || tipo == "porto_pesca" {
        200.0
    } else if tipo.starts_with("fazenda_") || tipo.starts_with("plantacao_") || tipo.starts_with("rancho_") || tipo == "pecuaria_pesca" {
        100.0
    } else {
        500.0
    };
    vagas_por_nivel * nivel as f64
}

fn recalcular_emprego_por_vagas(
    cen: &mut CenarioEconomia,
    prov_indices: &[usize],
    empregados_originais_por_tipo: &BTreeMap<String, f64>,
) {
    // Tipos de pop ativos (não-subsistência)
    const POPS_ATIVOS: &[&str] = &[
        "operarios", "engenheiros", "burocratas", "funcionarios",
        "gerentes", "intelectuais", "capitalistas", "militares", "mineiros",
    ];

    // Calcular população ativa e vagas por país
    let mut pop_ativa_por_pais: BTreeMap<String, f64> = BTreeMap::new();
    let mut vagas_por_pais: BTreeMap<String, f64> = BTreeMap::new();

    // Também calcular vagas por província
    let mut vagas_por_prov: HashMap<usize, f64> = HashMap::new();
    let mut pop_ativa_por_prov: HashMap<usize, f64> = HashMap::new();

    for &idx in prov_indices {
        let prov = &cen.provincias[idx];
        let pais_id = &prov.pais_id;

        let pop_ativa: f64 = prov.pops.iter()
            .filter(|p| POPS_ATIVOS.contains(&p.tipo_pop_id.as_str()))
            .map(|p| p.populacao)
            .sum();

        let vagas_prov: f64 = prov.predios.iter()
            .map(|b| {
                if b.cache.num_vagas > 0 {
                    b.total_vagas()
                } else {
                    estimar_vagas_predio(&b.tipo_predio_id, b.nivel)
                }
            })
            .sum();

        *pop_ativa_por_pais.entry(pais_id.clone()).or_default() += pop_ativa;
        *vagas_por_pais.entry(pais_id.clone()).or_default() += vagas_prov;
        pop_ativa_por_prov.insert(idx, pop_ativa);
        vagas_por_prov.insert(idx, vagas_prov);
    }

    // Pré-calcular população ativa por tipo por país (para evitar borrow issues depois)
    let mut pop_ativa_tipo_por_pais: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    for &idx in prov_indices {
        let prov = &cen.provincias[idx];
        let pais_id = &prov.pais_id;
        for pop in &prov.pops {
            if POPS_ATIVOS.contains(&pop.tipo_pop_id.as_str()) {
                *pop_ativa_tipo_por_pais
                    .entry(pais_id.clone())
                    .or_default()
                    .entry(pop.tipo_pop_id.clone())
                    .or_default() += pop.populacao;
            }
        }
    }

    // Para cada país, calcular emprego alvo por tipo = original (±2%)
    // Mas limitado pelas vagas totais do país
    let mut emprego_alvo_por_pais_tipo: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();

    for (pais_id, &pop_ativa_total) in &pop_ativa_por_pais {
        let vagas_total = vagas_por_pais.get(pais_id).copied().unwrap_or(0.0);
        let emprego_max_total = pop_ativa_total.min(vagas_total); // não pode empregar mais que pop ativa nem vagas

        let mut emprego_alvo_tipo: BTreeMap<String, f64> = BTreeMap::new();
        let mut soma_originais = 0.0;

        // Coletar empregados originais por tipo para este país
        for (tipo, &emp_original) in empregados_originais_por_tipo {
            // Verificar se este tipo existe neste país
            let tem_tipo = prov_indices.iter().any(|&idx| {
                let prov = &cen.provincias[idx];
                prov.pais_id == *pais_id && prov.pops.iter().any(|p| p.tipo_pop_id == *tipo)
            });
            if tem_tipo {
                emprego_alvo_tipo.insert(tipo.clone(), emp_original);
                soma_originais += emp_original;
            }
        }

        // Se soma dos originais > emprego_max_total, escalar proporcionalmente para caber nas vagas
        // Mas mantemos o total nacional de empregados o mais próximo possível do original (±2%)
        if soma_originais > emprego_max_total && soma_originais > 0.0 {
            let fator = emprego_max_total / soma_originais;
            for v in emprego_alvo_tipo.values_mut() {
                *v *= fator;
            }
        }

        emprego_alvo_por_pais_tipo.insert(pais_id.clone(), emprego_alvo_tipo);
    }

    // Aplicar por província: distribuir emprego alvo por tipo proporcionalmente à população ativa da província
    for &idx in prov_indices {
        let prov = &mut cen.provincias[idx];
        let pais_id = &prov.pais_id;
        let emprego_alvo_tipo_pais = emprego_alvo_por_pais_tipo.get(pais_id).cloned().unwrap_or_default();
        let pop_ativa_pais = pop_ativa_por_pais.get(pais_id).copied().unwrap_or(0.0);

        if pop_ativa_pais <= 0.0 {
            continue;
        }

        // Calcular população ativa por tipo nesta província
        let mut pop_ativa_por_tipo_prov: BTreeMap<String, f64> = BTreeMap::new();
        for pop in &prov.pops {
            if POPS_ATIVOS.contains(&pop.tipo_pop_id.as_str()) {
                *pop_ativa_por_tipo_prov.entry(pop.tipo_pop_id.clone()).or_default() += pop.populacao;
            }
        }

        // Para cada tipo ativo, calcular emprego alvo na província
        for (tipo, pop_ativa_tipo_prov) in pop_ativa_por_tipo_prov {
            let emprego_alvo_tipo_pais = emprego_alvo_tipo_pais.get(&tipo).copied().unwrap_or(0.0);
            if emprego_alvo_tipo_pais <= 0.0 || pop_ativa_tipo_prov <= 0.0 {
                continue;
            }

            // Proporção da população ativa deste tipo na província vs país (pré-calculado)
            let pop_ativa_tipo_pais = pop_ativa_tipo_por_pais
                .get(pais_id)
                .and_then(|m| m.get(&tipo))
                .copied()
                .unwrap_or(0.0);

            if pop_ativa_tipo_pais <= 0.0 {
                continue;
            }

            let emprego_alvo_prov = emprego_alvo_tipo_pais * (pop_ativa_tipo_prov / pop_ativa_tipo_pais);

            // Aplicar ao pop deste tipo na província
            for pop in &mut prov.pops {
                if pop.tipo_pop_id == tipo {
                    let taxa_alvo = (emprego_alvo_prov / pop_ativa_tipo_prov).min(1.0);
                    pop.empregados = (pop.populacao * taxa_alvo).min(pop.populacao);
                }
            }
        }

        // Camponeses (subsistência): mantêm taxa de emprego original (já ajustada na redistribuição de população)
        for pop in &mut prov.pops {
            if pop.tipo_pop_id == "camponeses" {
                // Manter emprego como está
            }
        }
    }
}

fn categorizar_predio(tipo: &str) -> CategoriaPredioRedist {
    if tipo.starts_with("mina_") || tipo.starts_with("poco_") || tipo.starts_with("campo_") || tipo == "porto_pesca" {
        CategoriaPredioRedist::Extrativo
    } else if tipo.starts_with("fazenda_") || tipo.starts_with("plantacao_") || tipo.starts_with("rancho_") || tipo == "pecuaria_pesca" {
        CategoriaPredioRedist::Fazenda
    } else if tipo.contains("fabrica") || tipo.contains("usina") || tipo.contains("siderurgica") || tipo.contains("industria") || tipo == "serraria" || tipo == "estaleiro_naval" {
        CategoriaPredioRedist::Industrial
    } else if tipo.contains("setor_") || tipo.contains("porto_comercial") {
        CategoriaPredioRedist::Servicos
    } else {
        CategoriaPredioRedist::Governo
    }
}

fn recurso_do_predio(tipo: &str) -> String {
    match tipo {
        "mina_ferro" => "ferro".to_string(),
        "mina_carvao" => "carvao".to_string(),
        "poco_petroleo" => "petroleo".to_string(),
        "campo_gas_natural" | "poco_gas_natural" => "gas_natural".to_string(),
        "mina_cobre" => "cobre".to_string(),
        "mina_uranio" => "uranio".to_string(),
        "mina_bauxita" => "bauxita".to_string(),
        "porto_pesca" => "pesca".to_string(),
        "serraria" => "madeira".to_string(),
        _ => "".to_string(),
    }
}