use simulacao::economia::{
    Predio, TipoPredio, calcular_penalidade_escassez, calcular_preco_alvo_mercado,
};
use simulacao::mapa::{
    AlinhamentoGeopolitico, CatalogoProvincias, DefinicaoPais1945, DonosProvincias1945,
    ProvinciaMapa, cor_unica_provincia, desprojetar_miller, id_de_cor_provincia, projetar_miller,
};
use simulacao::mundo::MundoJogo;
use simulacao::politica::{
    EsferaInfluencia, GrupoInteresseId, construir_politica_inicial,
};
use simulacao::tempo::{DataSimulacao, RelogioSimulacao, VelocidadeJogo};
use std::collections::{BTreeMap, HashSet};

#[test]
fn test_calendario_e_formatacao() {
    let mut data = DataSimulacao::INICIO_1945;
    assert_eq!(data.ano, 1945);
    assert_eq!(data.mes, 1);
    assert_eq!(data.dia, 1);
    assert_eq!(data.formatar_pt_br(), "1º de janeiro de 1945");

    data.avancar_dia();
    assert_eq!(data.dia, 2);
    assert_eq!(data.formatar_pt_br(), "2 de janeiro de 1945");

    assert!(!DataSimulacao::ano_bissexto(1945));
    assert!(DataSimulacao::ano_bissexto(1948));
    assert_eq!(DataSimulacao::dias_no_mes(1945, 2), 28);
    assert_eq!(DataSimulacao::dias_no_mes(1948, 2), 29);

    let mut fim_ano = DataSimulacao::nova(1945, 12, 31);
    fim_ano.avancar_dia();
    assert_eq!(fim_ano, DataSimulacao::nova(1946, 1, 1));
}

#[test]
fn test_relogio_simulacao() {
    let mut relogio = RelogioSimulacao::novo();
    assert!(relogio.pausado);
    assert_eq!(relogio.atualizar(1.0), 0);

    relogio.alternar_pausa();
    assert!(!relogio.pausado);
    relogio.definir_velocidade(VelocidadeJogo::V1); // 1 tick por segundo

    let ticks = relogio.atualizar(1.05);
    assert_eq!(ticks, 1);
    assert_eq!(relogio.data.dia, 2);
}

#[test]
fn test_codificacao_cores_provincias() {
    assert_eq!(id_de_cor_provincia((0, 0, 0)), None);

    let mut set = HashSet::new();
    for id in 1..=5000 {
        let cor = cor_unica_provincia(id);
        assert_ne!(cor, (0, 0, 0), "provincia {} nao pode ter cor preta", id);
        assert!(set.insert(cor), "colisao de cor na provincia {}", id);

        let decodificado = id_de_cor_provincia(cor);
        assert_eq!(decodificado, Some(id));
    }
}

#[test]
fn test_projecao_miller() {
    let w = 5632;
    let h = 2048;

    let coords = [(0.0, 0.0), (-43.2, -22.9), (-74.0, 40.7), (37.6, 55.7)];
    for &(lon, lat) in &coords {
        let (px, py) = projetar_miller(lon, lat, w, h);
        assert!(px >= 0.0 && px <= w as f64);
        assert!(py >= 0.0 && py <= h as f64);

        let (lon_rev, lat_rev) = desprojetar_miller(px, py, w, h);
        assert!((lon - lon_rev).abs() < 1e-3);
        assert!((lat - lat_rev).abs() < 1e-3);
    }
}

#[test]
fn test_formula_preco_mercado() {
    let p0 = 50.0;
    // Equilibrio
    assert_eq!(calcular_preco_alvo_mercado(p0, 100.0, 100.0), 50.0);

    // Escassez extrema (demanda alta, oferta zero)
    let p_max = calcular_preco_alvo_mercado(p0, 0.0, 100.0);
    assert_eq!(p_max, 50.0 * 1.75);

    // Superabundancia (oferta alta, demanda zero)
    let p_min = calcular_preco_alvo_mercado(p0, 100.0, 0.0);
    assert_eq!(p_min, 50.0 * 0.25);

    // Penalidade de escassez
    assert_eq!(calcular_penalidade_escassez(100.0, 100.0), 0.0);
    assert!(calcular_penalidade_escassez(20.0, 100.0) > 0.0);
}

#[test]
fn test_economia_predio_e_escala() {
    let mut p = Predio::novo(TipoPredio::Siderurgica, 1, 0.0);
    assert_eq!(p.fator_economia_escala(), 1.0);

    p.nivel = 11;
    assert_eq!(p.fator_economia_escala(), 1.10);

    p.destruicao_guerra = 0.50;
    p.recuperar_dano_paz_diario();
    assert!(p.destruicao_guerra < 0.50);
}

#[test]
fn test_politica_e_legitimidade() {
    let mut leis = BTreeMap::new();
    leis.insert("governo".to_string(), "sufragio_universal".to_string());
    leis.insert("economia".to_string(), "capitalismo_de_intervencao".to_string());

    let mut estado = construir_politica_inicial("USA", &leis, -100.0);
    assert!(estado.possui_eleicoes());
    assert!(estado.legitimidade > 0.0);
    assert_eq!(estado.esfera_atual(), EsferaInfluencia::EsferaOcidentalConsolidada);

    // Coalizao
    let mut coalizao = std::collections::BTreeSet::new();
    coalizao.insert(GrupoInteresseId::Industriais);
    coalizao.insert(GrupoInteresseId::TrabalhadoresSindicalizados);
    estado.definir_coalizao(&coalizao);
    assert!(estado.legitimidade > 30.0);
}

#[test]
fn test_simulacao_mundo_sintetico() {
    let provs = vec![
        ProvinciaMapa {
            id: 1,
            adm1_code: "USA-001".to_string(),
            nome: "Distrito de Columbia".to_string(),
            cor: cor_unica_provincia(1),
            centroide: (-77.0, 38.9),
            centroide_px: (1610, 580),
            area: 177.0,
            vizinhos: vec![],
            pais_moderno: "Estados Unidos".to_string(),
            adm0_moderno: "USA".to_string(),
        },
        ProvinciaMapa {
            id: 2,
            adm1_code: "SOV-001".to_string(),
            nome: "Moscou".to_string(),
            cor: cor_unica_provincia(2),
            centroide: (37.6, 55.7),
            centroide_px: (3400, 380),
            area: 2561.0,
            vizinhos: vec![],
            pais_moderno: "Rússia".to_string(),
            adm0_moderno: "RUS".to_string(),
        },
    ];

    let catalogo = CatalogoProvincias {
        largura: 5632,
        altura: 2048,
        projecao: "miller".to_string(),
        provincias: provs,
    };

    let paises_def = vec![
        DefinicaoPais1945 {
            tag: "USA".to_string(),
            nome: "Estados Unidos".to_string(),
            cor: (45, 95, 175),
            capital: "Washington".to_string(),
            suserano: None,
            alinhamento: AlinhamentoGeopolitico::BlocoOcidental,
            ideologia: "democracia_liberal".to_string(),
            populacao_1945: 140_000_000,
            pib_milhoes_usd: 1_400_000,
            leis_iniciais: BTreeMap::new(),
        },
        DefinicaoPais1945 {
            tag: "SOV".to_string(),
            nome: "União Soviética".to_string(),
            cor: (175, 28, 28),
            capital: "Moscou".to_string(),
            suserano: None,
            alinhamento: AlinhamentoGeopolitico::BlocoSovietico,
            ideologia: "comunismo".to_string(),
            populacao_1945: 170_000_000,
            pib_milhoes_usd: 350_000,
            leis_iniciais: BTreeMap::new(),
        },
    ];

    let mut por_id = BTreeMap::new();
    por_id.insert(1, "USA".to_string());
    por_id.insert(2, "SOV".to_string());

    let donos = DonosProvincias1945 {
        por_id,
        por_adm1: BTreeMap::new(),
    };

    let mut mundo = MundoJogo::construir_cenario_1945(&catalogo, &paises_def, &donos)
        .expect("construcao do cenario");

    assert_eq!(mundo.paises.len(), 2);
    assert_eq!(mundo.data.formatar_curto(), "01/01/1945");

    // Roda 10 ticks diarios e verifica estabilidade
    for _ in 0..10 {
        mundo.executar_tick_diario();
    }
    assert_eq!(mundo.ticks_executados, 10);
    assert_eq!(mundo.data.dia, 11);
}
