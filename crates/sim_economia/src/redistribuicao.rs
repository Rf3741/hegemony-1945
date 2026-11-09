//! Módulo para redistribuição de população e prédios entre províncias de um mesmo país.
//! Mantém totais nacionais (±2%) enquanto distribui de forma mais realista.

use crate::catalogo::CatalogoEconomia;
use crate::cenario::CenarioEconomia;
use crate::entidades::{Estado, Pop, Predio, Provincia};
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

/// Redistribui população e prédios dentro de cada país em `provincias.ron`.
/// Mantém os TOTAIS NACIONAIS (população por tipo, níveis totais de cada prédio) em ±2%.
/// Prédios são redistribuídos proporcionalmente à população para manter emprego equilibrado.
pub fn redistribuir_provincias(cen: &mut CenarioEconomia) {
    // Construir mapas auxiliares
    let potenciais_por_estado = construir_mapa_potenciais(&cen.estados);

    // Agrupar índices de províncias por país
    let mut provincias_por_pais: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, prov) in cen.provincias.iter().enumerate() {
        provincias_por_pais.entry(prov.pais_id.clone()).or_default().push(idx);
    }

    // Calcular totais nacionais do arquivo jogavel (antes da redistribuição) para preservar totais (±2%)
    let mut totais_nacionais: BTreeMap<String, PaisTotais> = BTreeMap::new();
    for (pais_id, prov_indices) in &provincias_por_pais {
        // População: calcular dos províncias atuais (arquivo jogavel original)
        let mut pops_por_tipo: BTreeMap<String, f64> = BTreeMap::new();
        for &idx in prov_indices {
            let prov = &cen.provincias[idx];
            for pop in &prov.pops {
                *pops_por_tipo.entry(pop.tipo_pop_id.clone()).or_default() += pop.populacao;
            }
        }

        // Prédios: usar totais do arquivo jogavel para preservar totais nacionais
        let mut predios_por_tipo = BTreeMap::new();
        for &idx in prov_indices {
            let prov = &cen.provincias[idx];
            for predio in &prov.predios {
                *predios_por_tipo.entry(predio.tipo_predio_id.clone()).or_default() += predio.nivel as u64;
            }
        }

        // Sobrescrever com totais históricos corrigidos para países onde o jogavel tinha inflação
        if let Some(historicos) = totais_nacionais_historicos(pais_id) {
            for (tipo, nivel) in historicos.predios_por_tipo {
                predios_por_tipo.insert(tipo, nivel);
            }
        }

        totais_nacionais.insert(pais_id.clone(), PaisTotais {
            pops_por_tipo,
            predios_por_tipo,
        });
    }

    // Redistribuir para cada país
    for (pais_id, prov_indices) in provincias_por_pais {
        let totais = totais_nacionais.get(&pais_id).cloned().unwrap_or_default();
        let pesos = calcular_pesos_provincias(cen, &prov_indices, &potenciais_por_estado);

        // Salvar razões de emprego alvo (92% = 8% desemprego histórico)
        let razoes_emprego_originais = salvar_razoes_emprego_originais(cen, &prov_indices);

        // Redistribuir população (usando pesos históricos/geográficos)
        redistribuir_populacao(cen, &prov_indices, &totais.pops_por_tipo, &pesos);

        // Redistribuir prédios proporcionalmente à população (mantendo totais nacionais)
        redistribuir_predios(cen, &prov_indices, &totais.predios_por_tipo, &pesos, &potenciais_por_estado);

        // Após redistribuir prédios, recalcular emprego baseado em vagas disponíveis
        recalcular_emprego_por_vagas(cen, &prov_indices, &razoes_emprego_originais);
    }
}

/// Salva a razão empregados/população alvo para cada pop.
/// Usa 92% (8% desemprego) como teto histórico realista para 1945,
/// em vez das razões do arquivo atual (que estão em 99% após redistribuição anterior).
fn salvar_razoes_emprego_originais(
    _cen: &CenarioEconomia,
    prov_indices: &[usize],
) -> HashMap<(String, String), f64> {
    let mut razoes = HashMap::new();
    const RAZAO_EMPREGO_ALVO: f64 = 0.92; // 92% empregados = 8% desemprego (faixa histórica 4-10%)
    for &idx in prov_indices {
        let prov = &_cen.provincias[idx];
        for pop in &prov.pops {
            if pop.populacao > 0.0 {
                razoes.insert((prov.id.clone(), pop.tipo_pop_id.clone()), RAZAO_EMPREGO_ALVO);
            }
        }
    }
    razoes
}

/// Recalcula emprego após redistribuição de prédios: usa razão original como teto,
/// mas limitado pelas vagas reais disponíveis nos prédios da província.
fn recalcular_emprego_por_vagas(
    cen: &mut CenarioEconomia,
    prov_indices: &[usize],
    razoes_originais: &HashMap<(String, String), f64>,
) {
    for &idx in prov_indices {
        let prov = &mut cen.provincias[idx];

        // Calcular vagas totais por tipo de pop nesta província
        let mut vagas_por_tipo_pop: BTreeMap<String, f64> = BTreeMap::new();
        for predio in &prov.predios {
            for (tipo_pop, vagas) in &predio.empregados_por_tipo {
                *vagas_por_tipo_pop.entry(tipo_pop.clone()).or_default() += *vagas;
            }
        }

        // Para cada pop, emprego = min(populacao * razao_original, vagas_disponiveis_para_este_tipo)
        // Distribuir vagas proporcionalmente se houver mais pops do mesmo tipo que vagas
        let mut pops_por_tipo: BTreeMap<String, Vec<&mut Pop>> = BTreeMap::new();
        for pop in &mut prov.pops {
            pops_por_tipo.entry(pop.tipo_pop_id.clone()).or_default().push(pop);
        }

        for (tipo_pop, pops) in pops_por_tipo {
            let vagas_totais = vagas_por_tipo_pop.get(&tipo_pop).copied().unwrap_or(0.0);
            if vagas_totais <= 0.0 {
                for pop in pops {
                    pop.empregados = 0.0;
                }
                continue;
            }

            // Soma da população * razão_original para este tipo de pop na província
            let mut demanda_total = 0.0;
            for pop in &pops {
                let chave = (prov.id.clone(), tipo_pop.clone());
                let razao_original = razoes_originais.get(&chave).copied().unwrap_or(0.9);
                demanda_total += pop.populacao * razao_original;
            }

            if demanda_total <= 0.0 {
                for pop in pops {
                    pop.empregados = 0.0;
                }
                continue;
            }

            // Se demanda <= vagas, todos empregados na razão original
            // Se demanda > vagas, escalar proporcionalmente
            let fator = if demanda_total <= vagas_totais {
                1.0
            } else {
                vagas_totais / demanda_total
            };

            for pop in pops {
                let chave = (prov.id.clone(), tipo_pop.clone());
                let razao_original = razoes_originais.get(&chave).copied().unwrap_or(0.9);
                pop.empregados = (pop.populacao * razao_original * fator).min(pop.populacao);
            }
        }
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

/// Totais nacionais históricos corretos (baseados no branch jogavel) para preservar durante redistribuição.
/// O commit 6f3f1efa inflacionou estes valores; aqui restauramos os totais corretos.
/// Apenas USA definido; demais países usam totais do arquivo jogavel.
/// Nota: USA precisa de escala 8.0x para PIB na faixa 200-350 bi.
fn totais_nacionais_historicos(pais_id: &str) -> Option<PaisTotais> {
    match pais_id {
        "usa" => {
            let mut pops_por_tipo = BTreeMap::new();
            let mut predios_por_tipo = BTreeMap::new();
            // Pop totals will be calculated from historical weights
            // Building totals from jogavel branch (USA only) - scaled 8.0x for PIB 200-350 bi
            const USA_SCALE: f64 = 8.0;
            predios_por_tipo.insert("usina_termeletrica".to_string(), (45152.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("fazenda_graos".to_string(), (35227.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("mina_bauxita".to_string(), (27268.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("serraria".to_string(), (25492.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("industria_alimenticia".to_string(), (22032.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("porto_pesca".to_string(), (9116.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("porto_comercial".to_string(), (8910.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("mina_ferro".to_string(), (8364.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("complexo_militar_quartel".to_string(), (7889.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("mina_cobre".to_string(), (7616.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("mina_uranio".to_string(), (6067.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("fabrica_ferramentas".to_string(), (5236.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("poco_petroleo".to_string(), (4692.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("setor_servicos_comercio".to_string(), (2652.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("campo_gas_natural".to_string(), (2584.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("estaleiro_naval".to_string(), (1768.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("mina_carvao".to_string(), (1564.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("fabrica_maquinas".to_string(), (1292.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("siderurgica".to_string(), (1292.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("administracao_governamental".to_string(), (1007.0 * USA_SCALE) as u64);
            predios_por_tipo.insert("fabrica_textil".to_string(), (612.0 * USA_SCALE) as u64);
            Some(PaisTotais { pops_por_tipo, predios_por_tipo })
        }
        _ => None,
    }
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
        ("usa_3562", 5500000.0),   // Michigan ~5.5M
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
        let mut peso_pop: f64 = prov.terra_aravel_total as f64 * 0.3;
        let mut peso_industria: f64 = 0.0;
        let mut peso_servicos: f64 = 0.0;
        let mut peso_governo: f64 = 0.0;
        let mut extrativos: BTreeMap<String, f64> = BTreeMap::new();

        // Usar população histórica se disponível, senão fórmula padrão
        if let Some(&pop_hist) = pop_historica_1945.get(prov.id.as_str()) {
            if pop_hist > 0.0 {
                peso_pop = pop_hist;
            } else {
                // Território desabitado ou base militar mínima
                peso_pop = 100.0;
            }
            // Para territórios históricos, pesos de indústria/serviços/governo baseados em urbanização
            let peso_urbanizacao: f64 = prov.infraestrutura_base.max(1.0);
            peso_industria = peso_urbanizacao * 1000.0;
            peso_servicos = peso_urbanizacao * 2000.0;
            peso_governo = peso_urbanizacao * 50.0;
        } else {
            // Províncias continentais: pesos base muito maiores
            let peso_urbanizacao: f64 = prov.infraestrutura_base.max(1.0);
            peso_pop = prov.terra_aravel_total as f64 * 100.0 + peso_urbanizacao * 10000.0;
            peso_industria = peso_urbanizacao * 5000.0;
            peso_servicos = peso_urbanizacao * 8000.0;
            peso_governo = peso_urbanizacao * 100.0;
        }

        // Recursos extrativos: peso proporcional ao limite do recurso no estado
        if let Some(pot) = potencial {
            for (recurso, &limite) in &pot.recursos {
                if limite > 0 {
                    let tem_recurso = prov.limites_recursos.get(recurso).copied().unwrap_or(0) > 0;
                    if tem_recurso {
                        extrativos.insert(recurso.clone(), limite as f64);
                    }
                }
            }
            // Terra arável para fazendas
            if pot.terra_aravel > 0 && prov.terra_aravel_total > 0 {
                extrativos.insert("terra_aravel".to_string(), pot.terra_aravel as f64);
            }
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

fn redistribuir_populacao(
    cen: &mut CenarioEconomia,
    prov_indices: &[usize],
    totais_nacionais: &BTreeMap<String, f64>,
    pesos: &HashMap<String, ProvinciaPesos>,
) {
    // Primeiro, calcular população total atual por tipo de pop por província
    let mut pop_atual_por_prov_tipo: HashMap<(String, String), f64> = HashMap::new();
    for &idx in prov_indices {
        let prov = &cen.provincias[idx];
        for pop in &prov.pops {
            *pop_atual_por_prov_tipo.entry((prov.id.clone(), pop.tipo_pop_id.clone())).or_default() += pop.populacao;
        }
    }

    // Calcular população total atual por tipo de pop no país
    let mut pop_atual_nacional: BTreeMap<String, f64> = BTreeMap::new();
    for ((_, tipo), pop) in &pop_atual_por_prov_tipo {
        *pop_atual_nacional.entry(tipo.clone()).or_default() += pop;
    }

    // Para cada província, escalar as pops existentes para atingir o novo total
    for &idx in prov_indices {
        let prov = &mut cen.provincias[idx];
        let peso = pesos.get(&prov.id).cloned().unwrap_or_default();

        // Calcular população total atual desta província
        let pop_total_atual: f64 = prov.pops.iter().map(|p| p.populacao).sum();

        // Calcular população total desejada para esta província
        let mut pop_total_desejada = 0.0;
        for (tipo, total_nacional) in totais_nacionais {
            pop_total_desejada += total_nacional * peso.populacao;
        }

        if pop_total_atual <= 0.0 || pop_total_desejada <= 0.0 {
            continue;
        }

        let fator_escala = pop_total_desejada / pop_total_atual;

        // Escalar cada pop existente mantendo razões de emprego e diversidade cultural
        for pop in &mut prov.pops {
            pop.populacao *= fator_escala;
            pop.empregados *= fator_escala;
            pop.riqueza_reservas *= fator_escala;
        }
    }

    // Ajuste fino: garantir que totais nacionais por tipo sejam preservados exatamente
    // Calculando discrepâncias e redistribuindo o resíduo
    for (tipo, &total_nacional) in totais_nacionais {
        let total_atual: f64 = prov_indices.iter()
            .flat_map(|&idx| &cen.provincias[idx].pops)
            .filter(|p| p.tipo_pop_id == *tipo)
            .map(|p| p.populacao)
            .sum();

        if total_atual <= 0.0 { continue; }

        let fator_ajuste = total_nacional / total_atual;
        if (fator_ajuste - 1.0).abs() > 0.01 {
            for &idx in prov_indices {
                let prov = &mut cen.provincias[idx];
                for pop in &mut prov.pops {
                    if pop.tipo_pop_id == *tipo {
                        pop.populacao *= fator_ajuste;
                        pop.empregados *= fator_ajuste;
                        pop.riqueza_reservas *= fator_ajuste;
                    }
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
    // Categorizar tipos de prédio
    let mut extrativos_tipos = Vec::new();
    let mut industriais_tipos = Vec::new();
    let mut servicos_tipos = Vec::new();
    let mut governo_tipos = Vec::new();
    let mut fazenda_tipos = Vec::new();

    for (tipo, _) in totais_nacionais {
        let cat = categorizar_predio(tipo);
        match cat {
            CategoriaPredioRedist::Extrativo => extrativos_tipos.push(tipo.clone()),
            CategoriaPredioRedist::Industrial => industriais_tipos.push(tipo.clone()),
            CategoriaPredioRedist::Servicos => servicos_tipos.push(tipo.clone()),
            CategoriaPredioRedist::Governo => governo_tipos.push(tipo.clone()),
            CategoriaPredioRedist::Fazenda => fazenda_tipos.push(tipo.clone()),
        }
    }

    // Limpar prédios existentes
    for &idx in prov_indices {
        cen.provincias[idx].predios.clear();
    }

    // 1. Extrativos: só onde há recurso
    for tipo in extrativos_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        let recurso = recurso_do_predio(&tipo);
        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let peso_recurso = peso.extrativos.get(&recurso).copied().unwrap_or(0.0);

            // Verificar limite no estado e na província
            let limite_estado = potenciais.get(&prov.estado_id)
                .and_then(|p| p.recursos.get(&recurso))
                .copied().unwrap_or(0);

            let limite_prov = prov.limites_recursos.get(&recurso).copied().unwrap_or(0);
            let limite = limite_prov.min(limite_estado);

            if limite > 0 && peso_recurso > 0.0 {
                let nivel = ((total_nacional * peso_recurso) as u32).min(limite);
                if nivel > 0 {
                    prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
                }
            }
        }
    }

    // 2. Fazendas: só onde há terra arável
    for tipo in fazenda_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let peso_terra = peso.extrativos.get("terra_aravel").copied().unwrap_or(0.0);

            let terra_livre = prov.terra_aravel_total;
            if terra_livre > 0 && peso_terra > 0.0 {
                let nivel_por_terra = ((total_nacional * peso_terra) as u32).min(terra_livre);
                
                // Usar apenas limite de terra arável; emprego será ajustado depois por vagas reais
                let nivel = nivel_por_terra;
                if nivel > 0 {
                    prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
                }
            }
        }
    }

    // 3. Industrial: proporcional a população + urbanização
    // Usa pesos já normalizados (soma = 1.0) para preservar total nacional
    for tipo in industriais_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let nivel = (total_nacional * peso.industria) as u32;
            if nivel > 0 {
                prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
            }
        }
    }

    // 4. Serviços: proporcional a população + urbanização
    for tipo in servicos_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let nivel = (total_nacional * peso.servicos) as u32;
            if nivel > 0 {
                prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
            }
        }
    }

// 5. Governo/quartéis: proporcional à população
    for tipo in governo_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let nivel = (total_nacional * peso.governo) as u32;
            if nivel > 0 {
                prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
            }
        }
    }
}

fn categorizar_predio(tipo: &str) -> CategoriaPredioRedist {
    if tipo.starts_with("mina_") || tipo.starts_with("poco_") || tipo.starts_with("campo_") || tipo == "porto_pesca" {
        CategoriaPredioRedist::Extrativo
    } else if tipo.starts_with("fazenda_") || tipo.starts_with("plantacao_") || tipo.starts_with("rancho_") || tipo == "pecuaria_pesca" {
        CategoriaPredioRedist::Fazenda
    } else if tipo.contains("fabrica") || tipo.contains("usina") || tipo.contains("siderurgica") || tipo.contains("industria") || tipo == "serraria" {
        CategoriaPredioRedist::Industrial
    } else if tipo.contains("administracao") || tipo.contains("quartel") || tipo.contains("centro_urbano") || tipo.contains("setor_construcao") || tipo.contains("porto_comercial") {
        CategoriaPredioRedist::Governo
    } else {
        CategoriaPredioRedist::Servicos
    }
}

fn recurso_do_predio(tipo: &str) -> String {
    if tipo.starts_with("mina_ferro") { "ferro".to_string() }
    else if tipo.starts_with("mina_carvao") { "carvao".to_string() }
    else if tipo.starts_with("mina_cobre") { "cobre".to_string() }
    else if tipo.starts_with("mina_ouro") || tipo.starts_with("mina_prata") { "ouro_prata".to_string() }
    else if tipo.starts_with("mina_uranio") { "uranio".to_string() }
    else if tipo.starts_with("poco_petroleo") { "petroleo".to_string() }
    else if tipo.starts_with("poco_gas") || tipo.starts_with("campo_gas") { "gas_natural".to_string() }
    else if tipo == "porto_pesca" { "pesca".to_string() }
    else { "desconhecido".to_string() }
}

fn tem_materia_prima_local(tipo: &str, prov: &Provincia, potenciais: &HashMap<String, EstadoPotencial>) -> bool {
    if let Some(pot) = potenciais.get(&prov.estado_id) {
        if tipo.contains("siderurgica") || tipo.contains("fabrica_ferramentas") || tipo.contains("fabrica_armamentos") {
            return pot.recursos.get("ferro").copied().unwrap_or(0) > 0 && pot.recursos.get("carvao").copied().unwrap_or(0) > 0;
        }
        if tipo.contains("fabrica_texteis") {
            return pot.culturas_permitidas.iter().any(|c| c == "algodao");
        }
        if tipo.contains("industria_alimenticia") || tipo.contains("fabrica_alimentos") {
            return pot.terra_aravel > 0;
        }
    }
    false
}

fn criar_predio_padrao(tipo: &str, nivel: u32, pops: &[Pop], catalogo: &CatalogoEconomia) -> Predio {
    // Buscar métodos padrão do catálogo
    let metodos = if let Some(tipo_predio) = catalogo.tipos_predio.iter().find(|tp| tp.id == tipo) {
        tipo_predio.metodos_padrao.clone()
    } else {
        // Fallback genérico
        vec!["metodo_padrao".to_string(), "propriedade_privada".to_string()]
    };

    let mut empregados_por_tipo = BTreeMap::new();
    // Vagas por nível calibradas para manter emprego total similar ao original.
    // O original tinha prédios em TODAS as províncias; agora estão concentrados onde há recursos.
    // Valores baseados no original (~400-500 por nível para siderúrgica, ~300-400 para fábricas).
    let vagas_por_nivel = match tipo {
        t if t.starts_with("mina_") => 800,
        t if t.starts_with("poco_") => 600,
        t if t.starts_with("fazenda_") => 500,
        t if t.contains("siderurgica") => 1000,
        t if t.contains("fabrica") => 800,
        t if t.contains("usina") => 600,
        t if t.contains("administracao") => 400,
        t if t.contains("quartel") => 500,
        _ => 400,
    };

    let total_vagas = vagas_por_nivel * nivel as usize;

    // Definir quais tipos de pop trabalham em cada categoria de prédio
    let pop_tipos_trabalhadores = match tipo {
        t if t.starts_with("mina_") || t.starts_with("poco_") || t == "porto_pesca" || t == "serraria" => {
            // Extrativos: operários, engenheiros, mineiros, maquinistas
            vec!["operarios", "engenheiros", "mineiros", "maquinistas", "tecnicos"]
        }
        t if t.starts_with("fazenda_") || t.starts_with("plantacao_") || t.starts_with("rancho_") || t == "pecuaria_pesca" => {
            // Agrícolas: camponeses, agricultores, operários
            vec!["camponeses", "agricultores", "operarios", "maquinistas"]
        }
        t if t.contains("siderurgica") || t.contains("fabrica_") => {
            // Industriais: operários, engenheiros, maquinistas, técnicos
            vec!["operarios", "engenheiros", "maquinistas", "tecnicos", "mecanicos"]
        }
        t if t.contains("usina_") => {
            // Energia: operários, engenheiros, técnicos
            vec!["operarios", "engenheiros", "tecnicos", "maquinistas"]
        }
        t if t.contains("administracao") => {
            // Governo: burocratas, intelectuais, funcionários
            vec!["burocratas", "intelectuais", "funcionarios", "tecnicos"]
        }
        t if t.contains("quartel") => {
            // Militar: soldados, oficiais
            vec!["soldados", "oficiais", "militares"]
        }
        t if t.contains("centro_urbano") || t.contains("porto_comercial") => {
            // Serviços urbanos: operários, burocratas, técnicos, comerciantes
            vec!["operarios", "burocratas", "tecnicos", "comerciantes", "funcionarios"]
        }
        t if t.contains("setor_construcao") || t.contains("rede_ferroviaria") || t.contains("rodovia") => {
            // Construção/infraestrutura: operários, engenheiros, maquinistas
            vec!["operarios", "engenheiros", "maquinistas", "tecnicos"]
        }
        _ => {
            // Genérico: todos os tipos de trabalhadores comuns
            vec!["operarios", "engenheiros", "maquinistas", "tecnicos", "burocratas", "funcionarios", "camponeses", "agricultores", "mineiros", "soldados"]
        }
    };

    // Filtrar pops que são tipos de trabalhadores apropriados
    let pops_trabalhadores: Vec<_> = pops.iter()
        .filter(|p| p.populacao > 0.0 && pop_tipos_trabalhadores.contains(&p.tipo_pop_id.as_str()))
        .collect();

    if !pops_trabalhadores.is_empty() {
        let total_vagas_f64 = total_vagas as f64;
        let por_pop = total_vagas_f64 / pops_trabalhadores.len() as f64;
        for pop in pops_trabalhadores {
            empregados_por_tipo.insert(pop.tipo_pop_id.clone(), por_pop);
        }
    }

    Predio {
        tipo_predio_id: tipo.to_string(),
        nivel,
        metodos_ativos: metodos,
        empregados_por_tipo,
        salario_base: 4.0,
        reservas_caixa: 5000.0 * nivel as f64,
        subsidiado: tipo.contains("administracao") || tipo.contains("quartel"),
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
    }
}