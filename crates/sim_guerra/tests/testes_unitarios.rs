//! Bateria abrangente de testes unitários para todas as mecânicas de `sim_guerra`.

use sim_guerra::{
    CenarioGuerra, ClausulaPaz, DataJogo, GrafoCompilado, MundoGuerra, PosturaOperacional,
    PropostaPaz, RngDeterministico, validar_adm1_code, validar_id_snake_case, validar_tag_pais,
};
use std::collections::HashMap;
use std::path::Path;

fn carregar_cenario_teste() -> CenarioGuerra {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    CenarioGuerra::carregar_diretorio(&dir).expect("fixtures devem carregar sem erros")
}

#[test]
fn test_validacoes_identificadores() {
    assert!(validar_id_snake_case("entidade", "divisao_infantaria_1945").is_ok());
    assert!(validar_id_snake_case("entidade", "plano_a").is_ok());
    assert!(validar_id_snake_case("entidade", "Infantaria").is_err());
    assert!(validar_id_snake_case("entidade", "com__duplo").is_err());
    assert!(validar_id_snake_case("entidade", "fim_").is_err());

    assert!(validar_tag_pais("USA").is_ok());
    assert!(validar_tag_pais("SOV").is_ok());
    assert!(validar_tag_pais("BRA").is_ok());
    assert!(validar_tag_pais("US").is_err());
    assert!(validar_tag_pais("USAA").is_err());
    assert!(validar_tag_pais("usa").is_err());

    assert!(validar_adm1_code("USA-3521").is_ok());
    assert!(validar_adm1_code("SOV-1689").is_ok());
    assert!(validar_adm1_code("regiao_01").is_ok());
    assert!(validar_adm1_code("").is_err());
    assert!(validar_adm1_code("USA 3521").is_err());
}

#[test]
fn test_carregamento_catalogo_e_cenario() {
    let cenario = carregar_cenario_teste();
    assert_eq!(cenario.paises.len(), 2);
    assert_eq!(cenario.regioes.len(), 6);
    assert_eq!(cenario.arestas.len(), 5);
    assert_eq!(cenario.catalogo.terrenos.len(), 5);
    assert_eq!(cenario.catalogo.unidades.len(), 4);
    assert_eq!(cenario.catalogo.templates.len(), 3);
}

#[test]
fn test_serializacao_ron_roundtrip() {
    let cenario = carregar_cenario_teste();
    let ron_str = cenario.para_ron_string().expect("deve serializar para RON");
    let cenario_de_volta =
        CenarioGuerra::de_ron_str(&ron_str).expect("deve desserializar de volta de RON");

    assert_eq!(cenario.paises.len(), cenario_de_volta.paises.len());
    assert_eq!(cenario.regioes.len(), cenario_de_volta.regioes.len());
}

#[test]
fn test_grafo_compilado_csr() {
    let cenario = carregar_cenario_teste();
    let mut mapa_indices = HashMap::new();
    for (i, r) in cenario.regioes.iter().enumerate() {
        mapa_indices.insert(r.adm1_code.clone(), i);
    }

    let grafo = GrafoCompilado::construir(cenario.regioes.len(), &cenario.arestas, &mapa_indices)
        .expect("grafo deve compilar");

    // Região intermediária USA-0002 tem 2 vizinhos (USA-0001 e USA-0003)
    let idx_usa2 = mapa_indices["USA-0002"];
    assert_eq!(grafo.grau(idx_usa2), 2);
    assert!(grafo.sao_adjacentes(idx_usa2, mapa_indices["USA-0001"]));
    assert!(grafo.sao_adjacentes(idx_usa2, mapa_indices["USA-0003"]));
    assert!(!grafo.sao_adjacentes(idx_usa2, mapa_indices["SOV-0001"]));
}

#[test]
fn test_estatisticas_templates_divisao() {
    let cenario = carregar_cenario_teste();
    let tmpl_inf = &cenario.catalogo.templates_compilados[0];

    assert!(tmpl_inf.efetivos_max >= 10000);
    assert!(tmpl_inf.ataque_leve > 100.0);
    assert!(tmpl_inf.defesa > 150.0);
    assert_eq!(tmpl_inf.entrincheiramento_max, 18.0); // Bônus por ter engenharia
}

#[test]
fn test_rng_determinismo() {
    let mut rng1 = RngDeterministico::novo(12345);
    let mut rng2 = RngDeterministico::novo(12345);

    for _ in 0..100 {
        assert_eq!(rng1.proximo_u64(), rng2.proximo_u64());
        assert_eq!(rng1.proximo_f64(), rng2.proximo_f64());
        assert_eq!(rng1.intervalo_u32(10, 50), rng2.intervalo_u32(10, 50));
    }
}

#[test]
fn test_avanco_calendario() {
    let mut d = DataJogo::nova(1945, 1, 31).unwrap();
    d.avancar_dia();
    assert_eq!(d.dia, 1);
    assert_eq!(d.mes, 2);
    assert_eq!(d.ano, 1945);

    let mut d_fim_ano = DataJogo::nova(1945, 12, 31).unwrap();
    d_fim_ano.avancar_dia();
    assert_eq!(d_fim_ano.dia, 1);
    assert_eq!(d_fim_ano.mes, 1);
    assert_eq!(d_fim_ano.ano, 1946);
}

#[test]
fn test_posturas_operacionais() {
    assert!(PosturaOperacional::OfensivaEquilibrada.autoriza_ataque());
    assert!(PosturaOperacional::AssaltoAgressivo.autoriza_ataque());
    assert!(!PosturaOperacional::DefesaEmProfundidade.autoriza_ataque());
    assert!(!PosturaOperacional::ContencaoGuarnicao.autoriza_ataque());

    assert!(
        PosturaOperacional::AssaltoAgressivo.modificador_ataque()
            > PosturaOperacional::DefesaEmProfundidade.modificador_ataque()
    );
    assert!(
        PosturaOperacional::DefesaEmProfundidade.modificador_defesa()
            > PosturaOperacional::AssaltoAgressivo.modificador_defesa()
    );
}

#[test]
fn test_execucao_ticks_e_batalhas() {
    let cenario = carregar_cenario_teste();
    let mut mundo = MundoGuerra::novo(cenario).expect("mundo deve inicializar");

    assert_eq!(mundo.guerras().len(), 1);
    assert!(mundo.guerras()[0].ativa);

    let relatorio = mundo.executar_tick_diario();
    assert_eq!(relatorio.guerras_ativas, 1);
    assert!(relatorio.batalhas_ocorridas >= 1);
    assert!(relatorio.baixas_totais_dia > 0);
}

#[test]
fn test_api_tratado_paz_manualmente() {
    let cenario = carregar_cenario_teste();
    let mut mundo = MundoGuerra::novo(cenario).expect("mundo deve inicializar");

    let guerra_id = mundo.guerras()[0].id.clone();
    let proposta = PropostaPaz {
        guerra_id,
        proponente_tag: "USA".to_string(),
        clausulas: vec![ClausulaPaz::PazBranca],
    };

    let tratado = mundo
        .propor_paz(proposta)
        .expect("tratado de paz deve ser aplicado");
    assert_eq!(tratado.clausulas_executadas.len(), 1);
    assert!(!mundo.guerras()[0].ativa);
}

#[test]
fn test_declaracao_nova_guerra() {
    let cenario = carregar_cenario_teste();
    let mut mundo = MundoGuerra::novo(cenario).expect("mundo deve inicializar");

    // Encerra guerra inicial
    let guerra_ini = mundo.guerras()[0].id.clone();
    let _ = mundo.propor_paz(PropostaPaz {
        guerra_id: guerra_ini,
        proponente_tag: "USA".to_string(),
        clausulas: vec![ClausulaPaz::PazBranca],
    });

    // Declara nova guerra
    let res = mundo.declarar_guerra("SOV", "USA", "Segunda Guerra de Retaliação");
    assert!(res.is_ok());
    assert_eq!(mundo.guerras().len(), 2);
    assert!(mundo.guerras()[1].ativa);
}
