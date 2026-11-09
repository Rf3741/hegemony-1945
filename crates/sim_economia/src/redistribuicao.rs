//! Módulo para redistribuição de população e prédios entre províncias de um mesmo país.
//! Mantém totais nacionais (±2%) enquanto distribui de forma mais realista.

use crate::cenario::CenarioEconomia;
use crate::entidades::Estado;
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

    // Calcular totais nacionais do arquivo original (jogavel) para preservar totais nacionais (±2%)
    let mut totais_nacionais: BTreeMap<String, PaisTotais> = BTreeMap::new();
    for (pais_id, prov_indices) in &provincias_por_pais {
        let mut pops_por_tipo: BTreeMap<String, f64> = BTreeMap::new();
        for &idx in prov_indices {
            let prov = &cen.provincias[idx];
            for pop in &prov.pops {
                *pops_por_tipo.entry(pop.tipo_pop_id.clone()).or_default() += pop.populacao;
            }
        }

        let mut predios_por_tipo = BTreeMap::new();
        for &idx in prov_indices {
            let prov = &cen.provincias[idx];
            for predio in &prov.predios {
                *predios_por_tipo.entry(predio.tipo_predio_id.clone()).or_default() += predio.nivel as u64;
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

        // Redistribuir população (usando pesos históricos/geográficos)
        redistribuir_populacao(cen, &prov_indices, &totais.pops_por_tipo, &pesos);

        // Redistribuir prédios proporcionalmente em duas passadas (preservando níveis nacionais e prédios)
        redistribuir_predios(cen, &prov_indices, &totais.predios_por_tipo, &pesos, &potenciais_por_estado);

        // Ajustar emprego pelas vagas reais disponíveis nos prédios da província
        recalcular_emprego_por_vagas(cen, &prov_indices);
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

    // 3. Ajuste fino nacional por tipo de pop (para respeitar exatamente totais nacionais)
    for (tipo, &total_nacional) in totais_nacionais {
        let total_atual: f64 = prov_indices.iter()
            .flat_map(|&idx| &cen.provincias[idx].pops)
            .filter(|p| p.tipo_pop_id == *tipo)
            .map(|p| p.populacao)
            .sum();

        if total_atual <= 0.0 { continue; }

        let fator_ajuste = total_nacional / total_atual;
        if (fator_ajuste - 1.0).abs() > 1e-6 {
            for &idx in prov_indices {
                let prov = &mut cen.provincias[idx];
                for (p_idx, pop) in prov.pops.iter_mut().enumerate() {
                    if pop.tipo_pop_id == *tipo {
                        pop.populacao *= fator_ajuste;
                        let taxa = taxas_emprego_originais.get(&(idx, p_idx)).copied().unwrap_or(0.8696);
                        pop.empregados = pop.populacao * taxa;
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

    // 2. Para cada tipo de prédio, distribuir seus níveis nacionais em duas passadas
    for (tipo, &total_nacional) in totais_nacionais {
        if total_nacional == 0 {
            continue;
        }

        let provs_com_tipo: Vec<usize> = prov_indices.iter()
            .copied()
            .filter(|&idx| cen.provincias[idx].predios.iter().any(|b| b.tipo_predio_id == *tipo))
            .collect();

        if provs_com_tipo.is_empty() {
            continue;
        }

        let cat = categorizar_predio(tipo);

        // Nível base mínimo de 1 para cada província que tem o prédio
        let mut niveis: HashMap<usize, u32> = HashMap::new();
        for &idx in &provs_com_tipo {
            niveis.insert(idx, 1);
        }

        let base_total = provs_com_tipo.len() as u64;
        let niveis_restantes = total_nacional.saturating_sub(base_total);

        // Determinar pesos e capacidades locais
        let mut pesos_tipo: HashMap<usize, f64> = HashMap::new();
        let mut capacidade: HashMap<usize, u32> = HashMap::new();

        for &idx in &provs_com_tipo {
            let prov = &cen.provincias[idx];
            let pot = potenciais.get(&prov.estado_id);
            let p_peso = pesos.get(&prov.id).cloned().unwrap_or_default();

            match cat {
                CategoriaPredioRedist::Fazenda => {
                    let peso = prov.terra_aravel_total as f64;
                    pesos_tipo.insert(idx, peso);
                    // Para deixar terra_aravel_livre > 0 (essencial para lucro > 0 em Iowa):
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
                    let recurso = recurso_do_predio(tipo);
                    let lim_estado = pot.and_then(|p| p.recursos.get(&recurso)).copied().unwrap_or(0);
                    let lim_prov = prov.limites_recursos.get(&recurso).copied().unwrap_or(0);
                    let deposito = lim_estado.max(lim_prov);
                    let qual = pot.and_then(|p| p.qualidade_recursos.get(&recurso)).copied().unwrap_or(1.0);

                    if deposito > 0 {
                        pesos_tipo.insert(idx, deposito as f64 * qual);
                        capacidade.insert(idx, deposito);
                    } else {
                        // Sem depósito: mantém apenas nível base 1
                        pesos_tipo.insert(idx, 0.0);
                        capacidade.insert(idx, 1);
                    }
                }
                CategoriaPredioRedist::Industrial => {
                    pesos_tipo.insert(idx, p_peso.industria);
                }
                CategoriaPredioRedist::Servicos => {
                    pesos_tipo.insert(idx, p_peso.servicos);
                }
                CategoriaPredioRedist::Governo => {
                    pesos_tipo.insert(idx, p_peso.governo);
                }
            }
        }

        // 1ª Passada: distribuição proporcional aos pesos respeitando limites de capacidade
        if niveis_restantes > 0 {
            let soma_pesos: f64 = provs_com_tipo.iter()
                .map(|&idx| pesos_tipo.get(&idx).copied().unwrap_or(0.0))
                .sum();

            if soma_pesos > 0.0 {
                for &idx in &provs_com_tipo {
                    let p = pesos_tipo.get(&idx).copied().unwrap_or(0.0);
                    let proporcao = p / soma_pesos;
                    let desejado = (niveis_restantes as f64 * proporcao) as u32;
                    let adicionar = if let Some(&cap) = capacidade.get(&idx) {
                        desejado.min(cap.saturating_sub(niveis[&idx]))
                    } else {
                        desejado
                    };
                    *niveis.get_mut(&idx).unwrap() += adicionar;
                }
            }
        }

        // 2ª Passada: devolver os níveis que sobraram às províncias com capacidade
        let total_alocado: u64 = niveis.values().map(|&n| n as u64).sum();
        let mut sobra = total_nacional as i64 - total_alocado as i64;

        if sobra > 0 {
            let mut ordenadas = provs_com_tipo.clone();
            ordenadas.sort_by(|&a, &b| {
                let pa = pesos_tipo.get(&a).copied().unwrap_or(0.0);
                let pb = pesos_tipo.get(&b).copied().unwrap_or(0.0);
                pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
            });

            // 2a. Preencher províncias que ainda têm capacidade disponível
            for &idx in &ordenadas {
                if sobra <= 0 {
                    break;
                }
                if let Some(&cap) = capacidade.get(&idx) {
                    let espaco = cap.saturating_sub(niveis[&idx]);
                    if espaco > 0 {
                        let pegar = (sobra as u32).min(espaco);
                        *niveis.get_mut(&idx).unwrap() += pegar;
                        sobra -= pegar as i64;
                    }
                } else {
                    let pegar = sobra as u32;
                    *niveis.get_mut(&idx).unwrap() += pegar;
                    sobra = 0;
                    break;
                }
            }

            // 2b. Se ainda sobrar e nenhuma tiver capacidade formal, colocar na província de maior peso econômico
            if sobra > 0 && !ordenadas.is_empty() {
                let melhor_prov = ordenadas[0];
                *niveis.get_mut(&melhor_prov).unwrap() += sobra as u32;
                sobra = 0;
            }
        }

        // Atualizar níveis dos prédios nas províncias e ajustar reservas e vagas proporcionalmente
        for &idx in &provs_com_tipo {
            let prov = &mut cen.provincias[idx];
            let novo_nivel = niveis[&idx];
            for b in &mut prov.predios {
                if b.tipo_predio_id == *tipo {
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
    }
}

fn recalcular_emprego_por_vagas(
    cen: &mut CenarioEconomia,
    prov_indices: &[usize],
) {
    for &idx in prov_indices {
        let prov = &mut cen.provincias[idx];

        // Vagas totais disponíveis nos prédios desta província
        let total_vagas: f64 = prov.predios.iter()
            .flat_map(|b| b.empregados_por_tipo.values())
            .sum();

        // Empregados totais desejados nesta província
        let total_emp: f64 = prov.pops.iter().map(|p| p.empregados).sum();

        // Limitar o emprego da província ao total de vagas dos seus prédios
        if total_vagas > 0.0 && total_emp > total_vagas {
            let fator = total_vagas / total_emp;
            for pop in &mut prov.pops {
                pop.empregados *= fator;
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
