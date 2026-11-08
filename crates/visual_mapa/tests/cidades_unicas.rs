//! Teste de sanidade geográfica: nenhuma cidade com o mesmo nome a mais de 300 km
//! de outra do mesmo nome no mesmo país, salvo exceções históricas reais.

use std::collections::HashMap;
use std::path::Path;
use visual_mapa::dados::{CidadeDados, ler_ron, raiz_dados_padrao};

fn haversine_km(lat1: f32, lon1: f32, lat2: f32, lon2: f32) -> f32 {
    let r = 6371.0f32;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    r * c
}

/// Exceções históricas reais onde duas cidades homônimas distantes coexistem no mesmo país.
fn eh_excecao_historica(pais: &str, nome: &str) -> bool {
    matches!(
        (pais, nome),
        // Estados Unidos: Kansas City (KS) e Kansas City (MO) - adjacentes
        ("USA", "Kansas City")
        // Raj Britânico / Índia: Hyderabad (Sindh, Paquistão) e Hyderabad (Deccan/Telangana, Índia)
        | ("RAJ", "Hyderabad")
        // China: Suzhou (Anhui, 宿州) e Suzhou (Jiangsu, 苏州)
        | ("CHI", "Suzhou")
        // China: Jining (Shandong, 济宁) e Jining (Mongólia Interior, 集宁)
        | ("CHI", "Jining")
        // China: Yichun (Jiangxi, 宜春) e Yichun (Heilongjiang, 伊春)
        | ("CHI", "Yichun")
    )
}

#[test]
fn demo_cidades_sem_duplicatas_distantes() {
    let cidades: Vec<CidadeDados> =
        ler_ron(&raiz_dados_padrao().join("demo/cidades.ron")).expect("demo/cidades.ron");

    let mut por_pais_nome: HashMap<(String, String), Vec<&CidadeDados>> = HashMap::new();
    for c in &cidades {
        por_pais_nome
            .entry((c.pais_tag.clone(), c.nome.clone()))
            .or_default()
            .push(c);
    }

    let mut violacoes = Vec::new();
    for ((pais, nome), grupo) in &por_pais_nome {
        if grupo.len() <= 1 || eh_excecao_historica(pais, nome) {
            continue;
        }
        for i in 0..grupo.len() {
            for j in (i + 1)..grupo.len() {
                let dist = haversine_km(
                    grupo[i].posicao.1,
                    grupo[i].posicao.0,
                    grupo[j].posicao.1,
                    grupo[j].posicao.0,
                );
                if dist > 300.0 {
                    violacoes.push(format!(
                        "[{pais}] {nome}: dist={dist:.1} km entre {} em {:?} e {} em {:?}",
                        grupo[i].id, grupo[i].posicao, grupo[j].id, grupo[j].posicao
                    ));
                }
            }
        }
    }

    assert!(
        violacoes.is_empty(),
        "Cidades duplicadas a mais de 300 km no demo:\n{}",
        violacoes.join("\n")
    );

    // Validação específica para o Brasil: Natal deve ser única e no RN
    let natais: Vec<_> = cidades
        .iter()
        .filter(|c| c.pais_tag == "BRA" && c.nome == "Natal")
        .collect();
    assert_eq!(natais.len(), 1, "Deve existir exatamente 1 Natal no Brasil");
    assert_eq!(natais[0].adm1_code, "BRA-9439", "Natal deve ser no Rio Grande do Norte");
}

#[derive(serde::Deserialize)]
struct CidadeTerritorio {
    id: String,
    nome: String,
    latitude: f32,
    longitude: f32,
    pais_tag: String,
    estado_id: String,
}

#[test]
fn territorio_cidades_sem_duplicatas_distantes() {
    let caminho = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/data/territorio/cidades.ron"
    );
    let texto = std::fs::read_to_string(Path::new(caminho)).expect("territorio cidades.ron");
    let cidades: HashMap<String, CidadeTerritorio> =
        ron::from_str(&texto).expect("parse territorio cidades.ron");

    let mut por_pais_nome: HashMap<(String, String), Vec<&CidadeTerritorio>> = HashMap::new();
    for c in cidades.values() {
        por_pais_nome
            .entry((c.pais_tag.clone(), c.nome.clone()))
            .or_default()
            .push(c);
    }

    let mut violacoes = Vec::new();
    for ((pais, nome), grupo) in &por_pais_nome {
        if grupo.len() <= 1 || eh_excecao_historica(pais, nome) {
            continue;
        }
        for i in 0..grupo.len() {
            for j in (i + 1)..grupo.len() {
                let dist = haversine_km(
                    grupo[i].latitude,
                    grupo[i].longitude,
                    grupo[j].latitude,
                    grupo[j].longitude,
                );
                if dist > 300.0 {
                    violacoes.push(format!(
                        "[{pais}] {nome}: dist={dist:.1} km entre {} ({}) e {} ({})",
                        grupo[i].id, grupo[i].estado_id, grupo[j].id, grupo[j].estado_id
                    ));
                }
            }
        }
    }

    assert!(
        violacoes.is_empty(),
        "Cidades duplicadas a mais de 300 km no território:\n{}",
        violacoes.join("\n")
    );

    // Validação específica para o Brasil: Natal deve ser única e no RN
    let natais: Vec<_> = cidades
        .values()
        .filter(|c| c.pais_tag == "BRA" && c.nome == "Natal")
        .collect();
    assert_eq!(natais.len(), 1, "Deve existir exatamente 1 Natal no Brasil");
    assert_eq!(
        natais[0].estado_id, "bra_rio_grande_do_norte",
        "Natal deve ser no Rio Grande do Norte"
    );
}
