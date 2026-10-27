use image::{ImageBuffer, Rgb};
use shapefile::dbase::FieldValue;
use simulacao::mapa::{
    AlinhamentoGeopolitico, CatalogoProvincias, DefinicaoPais1945, DonosProvincias1945,
    ProvinciaMapa, RegrasCenario1945, cor_unica_provincia, projetar_miller,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

const LARGURA: u32 = 5632;
const ALTURA: u32 = 2048;

fn extrair_str(v: Option<&FieldValue>) -> String {
    match v {
        Some(FieldValue::Character(Some(s))) => s.trim().to_string(),
        Some(FieldValue::Memo(s)) => s.trim().to_string(),
        _ => String::new(),
    }
}

fn extrair_f64(v: Option<&FieldValue>) -> f64 {
    match v {
        Some(FieldValue::Numeric(Some(n))) => *n,
        Some(FieldValue::Double(d)) => *d,
        Some(FieldValue::Float(Some(f))) => *f as f64,
        Some(FieldValue::Integer(i)) => *i as f64,
        _ => 0.0,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== GERADOR DE MAPA E CENÁRIO 1945 (GUERRA FRIA) ===");

    let workspace_root = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|_| ".".to_string()),
    )
    .join("../..")
    .canonicalize()
    .unwrap_or_else(|_| PathBuf::from("."));

    let shp_path = workspace_root.join("data/raw/ne_10m_admin_1_states_provinces/ne_10m_admin_1_states_provinces.shp");
    if !shp_path.exists() {
        eprintln!(
            "Erro: shapefile não encontrado em {:?}. Certifique-se de que foi descompactado.",
            shp_path
        );
        std::process::exit(1);
    }

    println!("Lendo shapefile: {:?}", shp_path);
    let mut reader = shapefile::Reader::from_path(&shp_path)?;

    let mut raster: Vec<u16> = vec![0; (LARGURA * ALTURA) as usize];
    let mut provincias: Vec<ProvinciaMapa> = Vec::new();

    let mut prov_id_counter: u32 = 1;

    for result in reader.iter_shapes_and_records() {
        let (shape, record) = result?;

        let adm1_code = extrair_str(record.get("adm1_code"));
        let name = extrair_str(record.get("name"));
        let name_pt = extrair_str(record.get("name_pt"));
        let admin = extrair_str(record.get("admin"));
        let adm0_a3 = extrair_str(record.get("adm0_a3"));
        let lat = extrair_f64(record.get("latitude"));
        let lon = extrair_f64(record.get("longitude"));
        let area_sqkm = extrair_f64(record.get("area_sqkm"));

        let nome_final = if !name_pt.is_empty() {
            name_pt
        } else if !name.is_empty() {
            name
        } else {
            adm1_code.clone()
        };

        let id = prov_id_counter;
        prov_id_counter += 1;

        let cor = cor_unica_provincia(id);
        let (cx_px, cy_px) = projetar_miller(lon, lat, LARGURA, ALTURA);
        let mut cx = cx_px.clamp(0.0, (LARGURA - 1) as f64) as u32;
        let mut cy = cy_px.clamp(0.0, (ALTURA - 1) as f64) as u32;

        let mut polygon_rings: Vec<Vec<(f64, f64)>> = Vec::new();

        if let shapefile::Shape::Polygon(poly) = shape {
            for ring in poly.rings() {
                let pts: Vec<(f64, f64)> = ring
                    .points()
                    .iter()
                    .map(|p| projetar_miller(p.x, p.y, LARGURA, ALTURA))
                    .collect();
                if pts.len() >= 3 {
                    polygon_rings.push(pts);
                }
            }
        }

        // Rasterização por scanline even-odd
        let mut min_y_all = ALTURA;
        let mut max_y_all = 0;

        for ring in &polygon_rings {
            for &(_, y) in ring {
                let py = (y.floor() as i64).clamp(0, (ALTURA - 1) as i64) as u32;
                min_y_all = min_y_all.min(py);
                max_y_all = max_y_all.max(py);
            }
        }

        if min_y_all <= max_y_all && !polygon_rings.is_empty() {
            let mut inters: Vec<f64> = Vec::new();
            for y in min_y_all..=max_y_all {
                let scan_y = y as f64 + 0.5;
                inters.clear();

                for ring in &polygon_rings {
                    let n = ring.len();
                    for i in 0..n {
                        let j = (i + 1) % n;
                        let (x1, y1) = ring[i];
                        let (x2, y2) = ring[j];

                        if (y1 <= scan_y && y2 > scan_y) || (y2 <= scan_y && y1 > scan_y) {
                            let t = (scan_y - y1) / (y2 - y1);
                            let inter_x = x1 + t * (x2 - x1);
                            inters.push(inter_x);
                        }
                    }
                }

                inters.sort_by(|a, b| a.partial_cmp(b).unwrap());

                let mut k = 0;
                while k + 1 < inters.len() {
                    let x_start = (inters[k].ceil() as i64).clamp(0, (LARGURA - 1) as i64) as u32;
                    let x_end = (inters[k + 1].floor() as i64).clamp(0, (LARGURA - 1) as i64) as u32;

                    for x in x_start..=x_end {
                        raster[(y * LARGURA + x) as usize] = id as u16;
                    }
                    k += 2;
                }
            }
        }

        // Garante que o centroide da província tenha pelo menos 1 pixel no mapa
        if raster[(cy * LARGURA + cx) as usize] != id as u16 {
            // Se o centroide geométrico caiu fora (ex: baía/ilha), procura o primeiro pixel pintado
            let mut achou = false;
            if min_y_all <= max_y_all {
                'busca: for y in min_y_all..=max_y_all {
                    for x in 0..LARGURA {
                        if raster[(y * LARGURA + x) as usize] == id as u16 {
                            cx = x;
                            cy = y;
                            achou = true;
                            break 'busca;
                        }
                    }
                }
            }
            if !achou {
                raster[(cy * LARGURA + cx) as usize] = id as u16;
            }
        }

        provincias.push(ProvinciaMapa {
            id,
            adm1_code,
            nome: nome_final,
            cor,
            centroide: (lon as f32, lat as f32),
            centroide_px: (cx, cy),
            area: area_sqkm.max(1.0) as f32,
            vizinhos: Vec::new(),
            pais_moderno: if !admin.is_empty() { admin } else { adm0_a3.clone() },
            adm0_moderno: adm0_a3,
        });

        if id % 500 == 0 {
            println!("  Processadas {} províncias...", id);
        }
    }

    println!("Total de províncias processadas: {}", provincias.len());

    // Cálculo de vizinhos por adjacência de pixel
    println!("Calculando vizinhos de fronteira...");
    let mut adj: HashMap<u32, HashSet<u32>> = HashMap::new();

    for y in 0..ALTURA {
        for x in 0..LARGURA {
            let p1 = raster[(y * LARGURA + x) as usize] as u32;
            if p1 == 0 {
                continue;
            }
            if x + 1 < LARGURA {
                let p2 = raster[(y * LARGURA + (x + 1)) as usize] as u32;
                if p2 != 0 && p1 != p2 {
                    adj.entry(p1).or_default().insert(p2);
                    adj.entry(p2).or_default().insert(p1);
                }
            }
            if y + 1 < ALTURA {
                let p2 = raster[((y + 1) * LARGURA + x) as usize] as u32;
                if p2 != 0 && p1 != p2 {
                    adj.entry(p1).or_default().insert(p2);
                    adj.entry(p2).or_default().insert(p1);
                }
            }
        }
    }

    for prov in &mut provincias {
        if let Some(viz) = adj.get(&prov.id) {
            let mut v: Vec<u32> = viz.iter().copied().collect();
            v.sort();
            prov.vizinhos = v;
        }
    }

    // Salvar assets/map/provinces.png
    let assets_map_dir = workspace_root.join("assets/map");
    fs::create_dir_all(&assets_map_dir)?;
    let png_path = assets_map_dir.join("provinces.png");
    println!("Salvando bitmap de províncias em {:?}...", png_path);

    let mut img_buf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(LARGURA, ALTURA);
    for y in 0..ALTURA {
        for x in 0..LARGURA {
            let id = raster[(y * LARGURA + x) as usize] as u32;
            let cor = cor_unica_provincia(id);
            img_buf.put_pixel(x, y, Rgb([cor.0, cor.1, cor.2]));
        }
    }
    img_buf.save(&png_path)?;
    println!("provinces.png salvo com sucesso!");

    // Salvar assets/map/provinces.ron
    let ron_path = assets_map_dir.join("provinces.ron");
    println!("Salvando catálogo de províncias em {:?}...", ron_path);
    let catalogo = CatalogoProvincias {
        largura: LARGURA,
        altura: ALTURA,
        projecao: "miller".to_string(),
        provincias: provincias.clone(),
    };
    let ron_str = ron::ser::to_string_pretty(&catalogo, ron::ser::PrettyConfig::default())?;
    fs::write(&ron_path, ron_str)?;
    println!("provinces.ron salvo com sucesso!");

    // Gerar data/scenario_1945/
    let scenario_dir = workspace_root.join("data/scenario_1945");
    fs::create_dir_all(&scenario_dir)?;

    println!("Gerando regras e donos do cenário histórico de 1945...");
    let (regras, paises_1945) = gerar_regras_e_paises();

    // Resolver dono de cada província
    let mut donos_por_id: BTreeMap<u32, String> = BTreeMap::new();
    let mut donos_por_adm1: BTreeMap<String, String> = BTreeMap::new();

    for prov in &provincias {
        let tag = if let Some(t) = regras.overrides_adm1.get(&prov.adm1_code) {
            t.clone()
        } else if let Some(t) = regras.adm0_para_tag_1945.get(&prov.adm0_moderno) {
            t.clone()
        } else {
            prov.adm0_moderno.clone()
        };

        donos_por_id.insert(prov.id, tag.clone());
        donos_por_adm1.insert(prov.adm1_code.clone(), tag);
    }

    let donos = DonosProvincias1945 {
        por_id: donos_por_id,
        por_adm1: donos_por_adm1,
    };

    let rules_path = scenario_dir.join("rules_1945.ron");
    fs::write(
        &rules_path,
        ron::ser::to_string_pretty(&regras, ron::ser::PrettyConfig::default())?,
    )?;

    let owners_path = scenario_dir.join("province_owners_1945.ron");
    fs::write(
        &owners_path,
        ron::ser::to_string_pretty(&donos, ron::ser::PrettyConfig::default())?,
    )?;

    let countries_path = scenario_dir.join("countries_1945.ron");
    fs::write(
        &countries_path,
        ron::ser::to_string_pretty(&paises_1945, ron::ser::PrettyConfig::default())?,
    )?;

    println!("Cenário de 1945 gerado com sucesso:");
    println!("  - {:?}", rules_path);
    println!("  - {:?}", owners_path);
    println!("  - {:?}", countries_path);
    println!("  Total de entidades de 1945: {}", paises_1945.len());

    // Verificação de cobertura: toda província deve pertencer a uma entidade definida em 1945
    let tags_definidas: HashSet<&str> = paises_1945.iter().map(|p| p.tag.as_str()).collect();
    let mut sem_definicao: BTreeMap<String, u32> = BTreeMap::new();
    for tag in donos.por_id.values() {
        if !tags_definidas.contains(tag.as_str()) {
            *sem_definicao.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    if sem_definicao.is_empty() {
        println!("  Cobertura: 100% das províncias pertencem a entidades definidas de 1945.");
    } else {
        eprintln!("  AVISO: tags sem definição em 1945: {sem_definicao:?}");
    }
    let usadas: HashSet<&str> = donos.por_id.values().map(String::as_str).collect();
    let paises_com_terra = paises_1945.iter().filter(|p| usadas.contains(p.tag.as_str())).count();
    println!("  Entidades com ao menos uma província: {paises_com_terra}");

    Ok(())
}

fn gerar_regras_e_paises() -> (RegrasCenario1945, Vec<DefinicaoPais1945>) {
    let mut adm0_map: BTreeMap<String, String> = BTreeMap::new();

    // 1. URSS (União Soviética: 15 repúblicas soviéticas reunificadas em 1945)
    for tag in [
        "RUS", "UKR", "BLR", "KAZ", "UZB", "TKM", "KGZ", "TJK", "GEO", "ARM", "AZE", "LTU",
        "LVA", "EST", "MDA", "KAB",
    ] {
        adm0_map.insert(tag.to_string(), "SOV".to_string());
    }

    // 2. Raj Britânico / Índia Britânica
    for tag in ["IND", "PAK", "BGD", "KAS"] {
        adm0_map.insert(tag.to_string(), "IND".to_string());
    }

    // 3. Potências do Eixo ocupadas
    adm0_map.insert("DEU".to_string(), "GER".to_string());
    adm0_map.insert("JPN".to_string(), "JAP".to_string());

    // 4. Coreia japonesa
    adm0_map.insert("KOR".to_string(), "KOR".to_string());
    adm0_map.insert("PRK".to_string(), "KOR".to_string());

    // 5. Indochina Francesa
    for tag in ["VNM", "LAO", "KHM"] {
        adm0_map.insert(tag.to_string(), "FIC".to_string());
    }

    // 6. Índias Orientais Holandesas (Indonésia)
    adm0_map.insert("IDN".to_string(), "INS".to_string());

    // 7. Congo Belga e Ruanda-Urundi
    adm0_map.insert("COD".to_string(), "CGO".to_string());
    adm0_map.insert("RWA".to_string(), "RUA".to_string());
    adm0_map.insert("BDI".to_string(), "RUA".to_string());

    // 8. África Ocidental Francesa (AOF)
    for tag in ["SEN", "MLI", "MRT", "GIN", "CIV", "BFA", "NER", "BEN"] {
        adm0_map.insert(tag.to_string(), "AOF".to_string());
    }

    // 9. África Equatorial Francesa (AEF)
    for tag in ["TCD", "CAF", "COG", "GAB"] {
        adm0_map.insert(tag.to_string(), "AEF".to_string());
    }

    // 10. Outras colônias francesas
    adm0_map.insert("DZA".to_string(), "DZA".to_string());
    adm0_map.insert("MOR".to_string(), "MOR".to_string());
    adm0_map.insert("TUN".to_string(), "TUN".to_string());
    adm0_map.insert("MDG".to_string(), "MDG".to_string());
    adm0_map.insert("CMR".to_string(), "CMR".to_string());
    adm0_map.insert("TGO".to_string(), "TGO".to_string());
    adm0_map.insert("DJI".to_string(), "CFS".to_string());

    // 11. Império Português
    adm0_map.insert("PRT".to_string(), "POR".to_string());
    adm0_map.insert("AGO".to_string(), "ANG".to_string());
    adm0_map.insert("MOZ".to_string(), "MOZ".to_string());
    adm0_map.insert("GNB".to_string(), "GNB".to_string());
    adm0_map.insert("CPV".to_string(), "CPV".to_string());
    adm0_map.insert("STP".to_string(), "STP".to_string());
    adm0_map.insert("TLS".to_string(), "TIM".to_string());
    adm0_map.insert("MAC".to_string(), "MAC".to_string());

    // 12. Império Britânico (África, Oriente Médio, Ásia)
    adm0_map.insert("NGA".to_string(), "NGA".to_string());
    adm0_map.insert("GHA".to_string(), "GHA".to_string());
    adm0_map.insert("SLE".to_string(), "SLE".to_string());
    adm0_map.insert("GMB".to_string(), "GMB".to_string());
    adm0_map.insert("KEN".to_string(), "KEN".to_string());
    adm0_map.insert("UGA".to_string(), "UGA".to_string());
    adm0_map.insert("TZA".to_string(), "TZA".to_string());
    adm0_map.insert("ZMB".to_string(), "NRH".to_string());
    adm0_map.insert("ZWE".to_string(), "SRH".to_string());
    adm0_map.insert("MWI".to_string(), "NYA".to_string());
    adm0_map.insert("BWA".to_string(), "BWA".to_string());
    adm0_map.insert("LSO".to_string(), "BAS".to_string());
    adm0_map.insert("SWZ".to_string(), "SWZ".to_string());
    adm0_map.insert("SOM".to_string(), "SOM".to_string());
    adm0_map.insert("SOL".to_string(), "SOM".to_string());
    adm0_map.insert("SDN".to_string(), "SUD".to_string());
    adm0_map.insert("SDS".to_string(), "SUD".to_string());
    adm0_map.insert("ERI".to_string(), "ERI".to_string());
    adm0_map.insert("LBY".to_string(), "LBY".to_string());
    adm0_map.insert("MYS".to_string(), "MAL".to_string());
    adm0_map.insert("SGP".to_string(), "MAL".to_string());
    adm0_map.insert("BRN".to_string(), "BRU".to_string());
    adm0_map.insert("MMR".to_string(), "BUR".to_string());
    adm0_map.insert("LKA".to_string(), "CEY".to_string());
    adm0_map.insert("HKG".to_string(), "HKG".to_string());
    adm0_map.insert("ISR".to_string(), "PAL".to_string());
    adm0_map.insert("PSX".to_string(), "PAL".to_string());
    adm0_map.insert("JOR".to_string(), "JOR".to_string());
    adm0_map.insert("KWT".to_string(), "KWT".to_string());
    adm0_map.insert("BHR".to_string(), "BHR".to_string());
    adm0_map.insert("QAT".to_string(), "QAT".to_string());
    adm0_map.insert("ARE".to_string(), "TRU".to_string());
    adm0_map.insert("CYP".to_string(), "CYP".to_string());
    adm0_map.insert("CYN".to_string(), "CYP".to_string());
    adm0_map.insert("MLT".to_string(), "MLT".to_string());
    adm0_map.insert("BLZ".to_string(), "BLZ".to_string());
    adm0_map.insert("GUY".to_string(), "GUY".to_string());
    for caribe in ["JAM", "TTO", "BRB", "BHS", "LCA", "GRD", "VCT", "DMA", "ATG", "KNA"] {
        adm0_map.insert(caribe.to_string(), "BWI".to_string());
    }

    // 13. Iugoslávia e Tchecoslováquia
    for yug in ["SRB", "HRV", "BIH", "SVN", "MKD", "MNE", "KOS"] {
        adm0_map.insert(yug.to_string(), "YUG".to_string());
    }
    adm0_map.insert("CZE".to_string(), "CZE".to_string());
    adm0_map.insert("SVK".to_string(), "CZE".to_string());

    // 14. Outros
    adm0_map.insert("CHN".to_string(), "CHI".to_string());
    adm0_map.insert("TWN".to_string(), "CHI".to_string());
    adm0_map.insert("PHL".to_string(), "PHI".to_string());
    adm0_map.insert("SUR".to_string(), "SUR".to_string());
    adm0_map.insert("ZAF".to_string(), "SAF".to_string());
    adm0_map.insert("NAM".to_string(), "SAF".to_string());
    adm0_map.insert("AUS".to_string(), "AST".to_string());
    adm0_map.insert("PNG".to_string(), "PNG".to_string());
    adm0_map.insert("DNK".to_string(), "DEN".to_string());
    adm0_map.insert("GRL".to_string(), "GRL".to_string());
    adm0_map.insert("ESP".to_string(), "ESP".to_string());
    adm0_map.insert("SAH".to_string(), "SPO".to_string());
    adm0_map.insert("GNQ".to_string(), "GNQ".to_string());

    // 15. Correções de códigos ADM0 do Natural Earth e microestados europeus/árabes
    for (adm0, tag) in [
        ("LBN", "LEB"),
        ("MAR", "MOR"),
        ("OMN", "OMA"),
        ("LIE", "LIE"),
        ("SMR", "SMR"),
        ("MCO", "MCO"),
        ("AND", "AND"),
        ("VAT", "VAT"),
        ("ALD", "FIN"),
        ("FRO", "DEN"),
        ("USG", "CUB"),
        ("ESB", "CYP"),
        ("WSB", "CYP"),
    ] {
        adm0_map.insert(adm0.to_string(), tag.to_string());
    }

    // 16. Territórios ultramarinos britânicos (Caribe, Atlântico, Índico, Pacífico)
    for adm0 in [
        "GIB", "BMU", "SHN", "JEY", "GGY", "IMN", "SGS", "FLK", "IOT", "PCN", "SYC", "MUS", "MDV",
        "KIR", "FJI", "TON", "SLB", "TUV", "VUT",
    ] {
        adm0_map.insert(adm0.to_string(), "GBR".to_string());
    }
    for adm0 in ["TCA", "MSR", "AIA", "VGB", "CYM"] {
        adm0_map.insert(adm0.to_string(), "BWI".to_string());
    }

    // 17. Territórios ultramarinos franceses, neerlandeses e portugueses
    for adm0 in ["MAF", "BLM", "SPM", "WLF", "NCL", "PYF", "ATF", "CLP"] {
        adm0_map.insert(adm0.to_string(), "FRA".to_string());
    }
    adm0_map.insert("COM".to_string(), "MDG".to_string());
    for adm0 in ["SXM", "CUW", "ABW"] {
        adm0_map.insert(adm0.to_string(), "NLD".to_string());
    }

    // 18. Territórios dos EUA, Austrália, Nova Zelândia e Pacífico ocupado
    for adm0 in ["UMI", "VIR", "PRI", "ASM", "GUM", "MNP", "MHL", "FSM", "PLW"] {
        let tag = if matches!(adm0, "MHL" | "FSM" | "PLW" | "MNP") { "PAC" } else { "USA" };
        adm0_map.insert(adm0.to_string(), tag.to_string());
    }
    for adm0 in ["HMD", "NFK", "IOA", "CSI", "ATC", "NRU"] {
        adm0_map.insert(adm0.to_string(), "AST".to_string());
    }
    for adm0 in ["WSM", "COK", "NIU"] {
        adm0_map.insert(adm0.to_string(), "NZL".to_string());
    }
    adm0_map.insert("PGA".to_string(), "CHI".to_string());

    let mut overrides_adm1: BTreeMap<String, String> = BTreeMap::new();
    overrides_adm1.insert("CAN-683".to_string(), "NEW".to_string()); // Terra Nova britânica até 1949
    overrides_adm1.insert("IND-3261".to_string(), "POR".to_string()); // Goa Portuguesa

    let regras = RegrasCenario1945 {
        adm0_para_tag_1945: adm0_map,
        overrides_adm1,
    };

    // Lista de entidades históricas de 1945
    let mut paises = Vec::new();

    let entidades: &[(&str, &str, (u8, u8, u8), &str, Option<&str>, AlinhamentoGeopolitico, &str, u64, u64)] = &[
        ("USA", "Estados Unidos da América", (45, 95, 175), "Washington, D.C.", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_liberal", 140_000_000, 1_474_000),
        ("SOV", "União Soviética", (175, 28, 28), "Moscou", None, AlinhamentoGeopolitico::BlocoSovietico, "comunismo", 170_500_000, 343_000),
        ("GBR", "Reino Unido", (205, 35, 45), "Londres", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_liberal", 49_200_000, 331_000),
        ("FRA", "França (Governo Provisório)", (52, 88, 168), "Paris", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_liberal", 40_500_000, 110_000),
        ("CHI", "República da China", (215, 165, 45), "Nanquim", None, AlinhamentoGeopolitico::BlocoOcidental, "nacionalismo", 520_000_000, 240_000),
        ("GER", "Alemanha Ocupada (Conselho de Controle)", (105, 110, 115), "Berlim", None, AlinhamentoGeopolitico::OcupacaoAliada, "ocupacao_militar", 65_500_000, 280_000),
        ("JAP", "Japão Ocupado (SCAP)", (210, 185, 185), "Tóquio", None, AlinhamentoGeopolitico::OcupacaoAliada, "ocupacao_militar", 72_150_000, 160_000),
        ("BRA", "Estados Unidos do Brasil", (42, 138, 62), "Rio de Janeiro", None, AlinhamentoGeopolitico::BlocoOcidental, "trabalhismo_estatal", 46_500_000, 48_000),
        ("IND", "Índia Britânica (Raj)", (215, 120, 75), "Nova Délhi", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 390_000_000, 220_000),
        ("FIC", "Indochina Francesa", (85, 120, 180), "Hanói", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 28_000_000, 16_000),
        ("INS", "Índias Orientais Holandesas", (225, 135, 50), "Batávia", Some("NLD"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 72_000_000, 45_000),
        ("CGO", "Congo Belga", (220, 185, 60), "Léopoldville", Some("BEL"), AlinhamentoGeopolitico::BlocoOcidental, "extrativismo_colonial", 12_000_000, 8_000),
        ("RUA", "Ruanda-Urundi", (200, 170, 50), "Usumbura", Some("BEL"), AlinhamentoGeopolitico::BlocoOcidental, "extrativismo_colonial", 4_100_000, 2_000),
        ("AOF", "África Ocidental Francesa", (75, 110, 170), "Dakar", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 18_000_000, 10_000),
        ("AEF", "África Equatorial Francesa", (65, 100, 160), "Brazzaville", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 5_000_000, 3_000),
        ("DZA", "Argélia Francesa", (90, 130, 190), "Argel", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 8_500_000, 9_000),
        ("MOR", "Protetorado Francês de Marrocos", (80, 115, 175), "Rabat", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 8_300_000, 7_000),
        ("TUN", "Protetorado Francês da Tunísia", (85, 120, 180), "Túnis", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 3_200_000, 3_000),
        ("MDG", "Madagascar Francês", (70, 105, 165), "Antananarivo", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 4_000_000, 2_500),
        ("CMR", "Camarões Francês", (75, 110, 170), "Yaoundé", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 4_200_000, 2_500),
        ("TGO", "Togolândia Francesa", (75, 110, 170), "Lomé", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 1_250_000, 800),
        ("CFS", "Costa Francesa dos Somalis", (75, 110, 170), "Djibuti", Some("FRA"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 60_000, 100),
        ("POR", "Portugal (Estado Novo)", (145, 95, 60), "Lisboa", None, AlinhamentoGeopolitico::BlocoOcidental, "corporativismo_autoritario", 8_100_000, 14_000),
        ("ANG", "Angola Portuguesa", (135, 85, 50), "Luanda", Some("POR"), AlinhamentoGeopolitico::BlocoOcidental, "extrativismo_colonial", 4_100_000, 3_000),
        ("MOZ", "Moçambique Português", (130, 80, 45), "Lourenço Marques", Some("POR"), AlinhamentoGeopolitico::BlocoOcidental, "extrativismo_colonial", 5_700_000, 3_500),
        ("GNB", "Guiné Portuguesa", (125, 75, 40), "Bissau", Some("POR"), AlinhamentoGeopolitico::BlocoOcidental, "extrativismo_colonial", 480_000, 300),
        ("CPV", "Cabo Verde Português", (120, 70, 35), "Praia", Some("POR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 170_000, 150),
        ("STP", "São Tomé e Príncipe", (115, 65, 30), "São Tomé", Some("POR"), AlinhamentoGeopolitico::BlocoOcidental, "extrativismo_colonial", 55_000, 60),
        ("TIM", "Timor Português", (110, 60, 25), "Díli", Some("POR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 430_000, 250),
        ("MAC", "Macau Português", (105, 55, 20), "Macau", Some("POR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 200_000, 200),
        ("NGA", "Nigéria Britânica", (195, 55, 65), "Lagos", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 28_000_000, 15_000),
        ("GHA", "Costa do Ouro Britânica", (190, 50, 60), "Acra", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 4_200_000, 3_000),
        ("SLE", "Serra Leoa Britânica", (185, 45, 55), "Freetown", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 1_900_000, 1_200),
        ("GMB", "Gâmbia Britânica", (180, 40, 50), "Bathurst", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 260_000, 200),
        ("KEN", "Quênia Britânico", (195, 55, 65), "Nairóbi", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 5_500_000, 3_500),
        ("UGA", "Uganda Britânico", (190, 50, 60), "Entebbe", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 4_700_000, 2_800),
        ("TZA", "Tanganica Britânica", (185, 45, 55), "Dar es Salaam", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 7_300_000, 4_000),
        ("NRH", "Rodésia do Norte", (180, 40, 50), "Lusaka", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "extrativismo_colonial", 2_200_000, 2_000),
        ("SRH", "Rodésia do Sul", (175, 35, 45), "Salisbury", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "oligarquia_colonial", 2_500_000, 3_000),
        ("NYA", "Niassalândia", (170, 30, 40), "Zomba", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 2_700_000, 1_500),
        ("BWA", "Bechuanalândia", (165, 25, 35), "Mafeking", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 380_000, 300),
        ("BAS", "Basutolândia", (160, 20, 30), "Maseru", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 600_000, 400),
        ("SWZ", "Suazilândia", (155, 15, 25), "Mbabane", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 250_000, 200),
        ("SOM", "Somalilândia Britânica", (150, 10, 20), "Hargeisa", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 2_100_000, 1_000),
        ("SUD", "Sudão Anglo-Egípcio", (185, 65, 75), "Cartum", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "condominio_colonial", 7_500_000, 5_000),
        ("ERI", "Eritreia (Admin. Britânica)", (180, 60, 70), "Asmara", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "ocupacao_militar", 1_000_000, 800),
        ("LBY", "Líbia (Admin. Britânica)", (175, 55, 65), "Trípoli", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "ocupacao_militar", 1_000_000, 900),
        ("MAL", "Malásia Britânica", (195, 65, 75), "Kuala Lumpur", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 6_150_000, 6_000),
        ("BUR", "Birmânia Britânica", (190, 60, 70), "Rangum", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 17_000_000, 12_000),
        ("CEY", "Ceilão Britânico", (185, 55, 65), "Colombo", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 6_600_000, 5_000),
        ("HKG", "Hong Kong", (180, 50, 60), "Victoria", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "governador_colonial", 1_500_000, 1_500),
        ("BRU", "Brunei Britânico", (175, 45, 55), "Brunei", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "protetorado", 45_000, 80),
        ("PAL", "Mandato da Palestina", (170, 60, 80), "Jerusalém", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "mandato_colonial", 1_750_000, 2_000),
        ("JOR", "Transjordânia", (165, 55, 75), "Amã", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "monarquia_protetorada", 400_000, 350),
        ("KWT", "Kuwait", (160, 50, 70), "Kuwait", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "protetorado", 120_000, 250),
        ("BHR", "Bahrein", (155, 45, 65), "Manama", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "protetorado", 100_000, 150),
        ("QAT", "Catar", (150, 40, 60), "Doha", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "protetorado", 25_000, 40),
        ("TRU", "Estados da Trégua", (145, 35, 55), "Abu Dhabi", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "protetorado", 70_000, 80),
        ("CYP", "Chipre Britânico", (190, 70, 70), "Nicósia", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "colonia_coroa", 450_000, 400),
        ("MLT", "Malta Britânica", (185, 65, 65), "Valletta", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "colonia_coroa", 280_000, 250),
        ("BLZ", "Honduras Britânicas", (180, 60, 60), "Belize City", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "colonia_coroa", 60_000, 70),
        ("GUY", "Guiana Britânica", (175, 55, 55), "Georgetown", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "colonia_coroa", 370_000, 400),
        ("BWI", "Índias Ocidentais Britânicas", (170, 50, 50), "Kingston", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "colonia_coroa", 2_500_000, 2_200),
        ("NEW", "Terra Nova", (165, 45, 45), "St. John's", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "dominio", 320_000, 450),
        ("KOR", "Coreia Japonesa (Chōsen)", (170, 130, 170), "Keijo (Seul)", Some("JAP"), AlinhamentoGeopolitico::OcupacaoAliada, "governador_colonial", 25_500_000, 18_000),
        ("POL", "República da Polônia", (210, 60, 70), "Varsóvia", None, AlinhamentoGeopolitico::BlocoSovietico, "governo_provisorio", 23_900_000, 45_000),
        ("CZE", "Tchecoslováquia", (70, 130, 180), "Praga", None, AlinhamentoGeopolitico::BlocoSovietico, "republica_coalizao", 12_500_000, 32_000),
        ("YUG", "Iugoslávia Democrática Federal", (60, 120, 190), "Belgrado", None, AlinhamentoGeopolitico::BlocoSovietico, "partisans_socialistas", 15_500_000, 22_000),
        ("ROU", "Reino da Romênia", (215, 175, 30), "Bucareste", None, AlinhamentoGeopolitico::BlocoSovietico, "armisticio_sovietico", 15_800_000, 25_000),
        ("HUN", "Hungria", (75, 145, 100), "Budapeste", None, AlinhamentoGeopolitico::BlocoSovietico, "armisticio_sovietico", 9_000_000, 18_000),
        ("BGR", "Bulgária", (65, 135, 90), "Sófia", None, AlinhamentoGeopolitico::BlocoSovietico, "frente_da_patria", 6_950_000, 12_000),
        ("ALB", "Albânia", (185, 35, 45), "Tirana", None, AlinhamentoGeopolitico::BlocoSovietico, "governo_democratico_hoxha", 1_100_000, 1_200),
        ("FIN", "Finlândia", (195, 205, 225), "Helsinque", None, AlinhamentoGeopolitico::NaoAlinhado, "democracia_neutralizada", 3_800_000, 14_000),
        ("SWE", "Suécia", (85, 145, 205), "Estocolmo", None, AlinhamentoGeopolitico::NaoAlinhado, "democracia_liberal", 6_700_000, 35_000),
        ("NOR", "Noruega", (195, 50, 60), "Oslo", None, AlinhamentoGeopolitico::BlocoOcidental, "governo_ocupacao_retomada", 3_100_000, 15_000),
        ("DEN", "Dinamarca", (190, 45, 55), "Copenhague", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_retomada", 4_050_000, 22_000),
        ("GRL", "Groenlândia", (180, 200, 215), "Nuuk", Some("DEN"), AlinhamentoGeopolitico::BlocoOcidental, "colonia", 22_000, 50),
        ("ISL", "Islândia", (65, 115, 175), "Reykjavík", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_republicana", 130_000, 400),
        ("IRL", "Irlanda (Éire)", (45, 145, 75), "Dublin", None, AlinhamentoGeopolitico::NaoAlinhado, "democracia_parlamentar", 2_950_000, 10_000),
        ("BEL", "Bélgica", (215, 150, 30), "Bruxelas", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_liberal", 8_300_000, 38_000),
        ("NLD", "Países Baixos", (225, 110, 35), "Amsterdã", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_liberal", 9_300_000, 39_000),
        ("LUX", "Luxemburgo", (100, 175, 225), "Luxemburgo", None, AlinhamentoGeopolitico::BlocoOcidental, "grao_ducado", 290_000, 1_500),
        ("CHE", "Suíça", (205, 45, 45), "Berna", None, AlinhamentoGeopolitico::NaoAlinhado, "neutralidade_armada", 4_400_000, 38_000),
        ("AUT", "Áustria Ocupada", (190, 180, 170), "Viena", None, AlinhamentoGeopolitico::OcupacaoAliada, "ocupacao_quatro_zonas", 6_800_000, 22_000),
        ("ITA", "Reino da Itália (Cobeligerante)", (65, 155, 95), "Roma", None, AlinhamentoGeopolitico::BlocoOcidental, "monarquia_cobeligerante", 45_000_000, 95_000),
        ("ESP", "Espanha Franquista", (195, 145, 45), "Madri", None, AlinhamentoGeopolitico::NaoAlinhado, "nacional_catolicismo", 26_800_000, 52_000),
        ("SPO", "Saara Espanhol", (185, 135, 40), "El Aaiún", Some("ESP"), AlinhamentoGeopolitico::NaoAlinhado, "colonia", 15_000, 20),
        ("GNQ", "Guiné Espanhola", (180, 130, 35), "Santa Isabel", Some("ESP"), AlinhamentoGeopolitico::NaoAlinhado, "colonia", 200_000, 150),
        ("GRC", "Grécia", (65, 120, 195), "Atenas", None, AlinhamentoGeopolitico::BlocoOcidental, "reino_em_guerra_civil", 7_300_000, 12_000),
        ("TUR", "Turquia", (185, 55, 60), "Ancara", None, AlinhamentoGeopolitico::NaoAlinhado, "republica_kemalista", 18_800_000, 26_000),
        ("IRN", "Irã (Ocupação Anglo-Soviética)", (75, 145, 85), "Teerã", None, AlinhamentoGeopolitico::NaoAlinhado, "imperio_ocupado", 15_000_000, 20_000),
        ("IRQ", "Iraque Haxemita", (65, 135, 75), "Bagdá", None, AlinhamentoGeopolitico::BlocoOcidental, "monarquia_aliada", 4_600_000, 6_000),
        ("SAU", "Arábia Saudita", (55, 125, 65), "Riad", None, AlinhamentoGeopolitico::BlocoOcidental, "monarquia_absoluta", 3_200_000, 4_000),
        ("YEM", "Iêmen Mutawaquilita", (45, 115, 55), "Saná", None, AlinhamentoGeopolitico::NaoAlinhado, "imamoto_tradicional", 4_200_000, 2_000),
        ("SYR", "Síria", (65, 140, 110), "Damasco", None, AlinhamentoGeopolitico::NaoAlinhado, "republica_em_transicao", 3_000_000, 3_500),
        ("LEB", "Líbano", (70, 145, 115), "Beirute", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_confessional", 1_150_000, 1_800),
        ("EGY", "Reino do Egito", (195, 155, 45), "Cairo", None, AlinhamentoGeopolitico::BlocoOcidental, "monarquia_constitucional", 18_500_000, 22_000),
        ("ETH", "Império Etíope", (65, 145, 75), "Adis Abeba", None, AlinhamentoGeopolitico::NaoAlinhado, "imperio_tradicional", 16_000_000, 10_000),
        ("LBR", "Libéria", (55, 95, 165), "Monróvia", None, AlinhamentoGeopolitico::BlocoOcidental, "republica_presidencialista", 800_000, 800),
        ("SAF", "União Sul-Africana", (185, 125, 55), "Pretória", None, AlinhamentoGeopolitico::BlocoOcidental, "dominio_segregacionista", 11_400_000, 38_000),
        ("AFG", "Afeganistão", (115, 135, 95), "Cabul", None, AlinhamentoGeopolitico::NaoAlinhado, "monarquia_tradicional", 7_500_000, 4_000),
        ("NPL", "Nepal", (195, 45, 65), "Catmandu", None, AlinhamentoGeopolitico::NaoAlinhado, "dinastia_rana", 7_400_000, 3_000),
        ("BTN", "Butão", (215, 145, 35), "Thimphu", Some("GBR"), AlinhamentoGeopolitico::BlocoOcidental, "protetorado", 600_000, 250),
        ("THA", "Tailândia (Sião)", (165, 75, 95), "Bangkok", None, AlinhamentoGeopolitico::NaoAlinhado, "reino_sob_armisticio", 16_500_000, 14_000),
        ("MNG", "República Popular da Mongólia", (195, 50, 50), "Ulan Bator", None, AlinhamentoGeopolitico::BlocoSovietico, "estado_socialista", 750_000, 800),
        ("CAN", "Domínio do Canadá", (195, 45, 55), "Ottawa", None, AlinhamentoGeopolitico::BlocoOcidental, "dominio_parlamentar", 12_100_000, 95_000),
        ("AST", "Austrália", (55, 95, 175), "Camberra", None, AlinhamentoGeopolitico::BlocoOcidental, "dominio_parlamentar", 7_400_000, 52_000),
        ("NZL", "Nova Zelândia", (45, 85, 165), "Wellington", None, AlinhamentoGeopolitico::BlocoOcidental, "dominio_parlamentar", 1_700_000, 16_000),
        ("PNG", "Papua e Nova Guiné", (50, 90, 170), "Port Moresby", Some("AST"), AlinhamentoGeopolitico::BlocoOcidental, "territorio_sob_tutela", 1_400_000, 800),
        ("PHI", "Filipinas", (65, 105, 185), "Manila", Some("USA"), AlinhamentoGeopolitico::BlocoOcidental, "commonwealth_pre_independencia", 18_100_000, 14_000),
        ("MEX", "México", (55, 135, 75), "Cidade do México", None, AlinhamentoGeopolitico::BlocoOcidental, "republica_pri", 23_500_000, 38_000),
        ("ARG", "Argentina", (115, 165, 215), "Buenos Aires", None, AlinhamentoGeopolitico::NaoAlinhado, "junta_militar_peronista", 15_300_000, 68_000),
        ("CHL", "Chile", (185, 55, 65), "Santiago", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_presidencialista", 5_500_000, 18_000),
        ("COL", "Colômbia", (215, 175, 45), "Bogotá", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_republicana", 10_300_000, 22_000),
        ("VEN", "Venezuela", (225, 185, 55), "Caracas", None, AlinhamentoGeopolitico::BlocoOcidental, "republica_petroleira", 4_400_000, 28_000),
        ("PER", "Peru", (185, 45, 55), "Lima", None, AlinhamentoGeopolitico::BlocoOcidental, "republica_oligarquica", 7_500_000, 14_000),
        ("BOL", "Bolívia", (175, 35, 45), "La Paz", None, AlinhamentoGeopolitico::NaoAlinhado, "governo_militar_villaroel", 2_700_000, 4_500),
        ("PRY", "Paraguai", (165, 25, 35), "Assunção", None, AlinhamentoGeopolitico::NaoAlinhado, "regime_autoritario_morinigo", 1_300_000, 2_000),
        ("URY", "Uruguai", (105, 155, 205), "Montevidéu", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_batllista", 2_200_000, 10_000),
        ("ECU", "Equador", (205, 165, 35), "Quito", None, AlinhamentoGeopolitico::BlocoOcidental, "republica_presidencialista", 3_000_000, 3_500),
        ("CUB", "Cuba", (65, 115, 185), "Havana", None, AlinhamentoGeopolitico::BlocoOcidental, "republica_democratica_grau", 5_000_000, 12_000),
        ("HTI", "Haiti", (175, 45, 55), "Porto Príncipe", None, AlinhamentoGeopolitico::BlocoOcidental, "republica_lescot", 3_000_000, 2_000),
        ("DOM", "República Dominicana", (55, 105, 175), "Ciudad Trujillo (Santo Domingo)", None, AlinhamentoGeopolitico::BlocoOcidental, "ditadura_trujillo", 2_000_000, 2_200),
        ("GTM", "Guatemala", (75, 125, 195), "Cidade da Guatemala", None, AlinhamentoGeopolitico::BlocoOcidental, "revolucao_democratica_1944", 2_800_000, 3_000),
        ("HND", "Honduras", (65, 115, 185), "Tegucigalpa", None, AlinhamentoGeopolitico::BlocoOcidental, "autoritarismo_carias", 1_300_000, 1_400),
        ("SLV", "El Salvador", (55, 105, 175), "San Salvador", None, AlinhamentoGeopolitico::BlocoOcidental, "regime_militar", 1_850_000, 1_800),
        ("NIC", "Nicarágua", (45, 95, 165), "Manágua", None, AlinhamentoGeopolitico::BlocoOcidental, "ditadura_somoza", 1_000_000, 1_200),
        ("CRI", "Costa Rica", (185, 45, 55), "San José", None, AlinhamentoGeopolitico::BlocoOcidental, "democracia_republicana", 750_000, 1_500),
        ("PAN", "Panamá", (55, 105, 175), "Cidade do Panamá", None, AlinhamentoGeopolitico::BlocoOcidental, "republica_constitucional", 750_000, 1_600),
        ("SUR", "Suriname", (215, 105, 30), "Paramaribo", Some("NLD"), AlinhamentoGeopolitico::BlocoOcidental, "colonia", 200_000, 300),
        ("OMA", "Sultanato de Mascate e Omã", (125, 150, 90), "Mascate", None, AlinhamentoGeopolitico::BlocoOcidental, "sultanato_tradicional", 450_000, 400),
        ("LIE", "Liechtenstein", (170, 60, 60), "Vaduz", None, AlinhamentoGeopolitico::NaoAlinhado, "principado", 12_000, 30),
        ("SMR", "San Marino", (130, 190, 230), "San Marino", None, AlinhamentoGeopolitico::NaoAlinhado, "republica", 13_000, 20),
        ("MCO", "Mônaco", (225, 90, 90), "Mônaco", None, AlinhamentoGeopolitico::BlocoOcidental, "principado", 20_000, 40),
        ("AND", "Andorra", (200, 150, 200), "Andorra la Vella", None, AlinhamentoGeopolitico::NaoAlinhado, "co_principado", 6_000, 10),
        ("VAT", "Vaticano", (240, 230, 130), "Vaticano", None, AlinhamentoGeopolitico::NaoAlinhado, "teocracia", 1_000, 10),
        ("PAC", "Territórios do Pacífico (Ocupação dos EUA)", (95, 130, 205), "Saipan", Some("USA"), AlinhamentoGeopolitico::OcupacaoAliada, "ocupacao_militar", 150_000, 100),
        ("ATA", "Antártica (Território Neutro)", (230, 235, 240), "Estações de Pesquisa", None, AlinhamentoGeopolitico::NaoAlinhado, "terra_nullius", 0, 0),
    ];

    for &(tag, nome, cor, capital, suserano, alinhamento, ideologia, pop, pib) in entidades {
        paises.push(DefinicaoPais1945 {
            tag: tag.to_string(),
            nome: nome.to_string(),
            cor,
            capital: capital.to_string(),
            suserano: suserano.map(|s| s.to_string()),
            alinhamento,
            ideologia: ideologia.to_string(),
            populacao_1945: pop,
            pib_milhoes_usd: pib,
            leis_iniciais: BTreeMap::new(),
        });
    }

    (regras, paises)
}
