//! Testes da evolução histórica do valor dos recursos por oferta e demanda (`ciclos_demanda.ron`).

use sim_economia::{
    CatalogoEconomia, CenarioEconomia, ConfigSimulacao, DataJogo, Estado, Mercado, ModoCadencia,
    MundoEconomico, Pais, Provincia, ler_str_ron,
};
use std::path::Path;

fn carregar_catalogo_real() -> CatalogoEconomia {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
    CatalogoEconomia::carregar_diretorio(&dir).expect("catálogo real com ciclos_demanda.ron deve carregar")
}

fn criar_mundo_compacto_historico(semente: u64) -> MundoEconomico {
    let catalogo = carregar_catalogo_real();
    let mut config = ConfigSimulacao::default();
    config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;

    let mercados: Vec<Mercado> = ler_str_ron(
        r#"[
            (id: "mercado_usa", nome: "Mercado EUA", pais_lider_id: "usa"),
            (id: "mercado_exportador", nome: "Mercado Exportador Mineral", pais_lider_id: "can"),
        ]"#,
        "mercados_teste",
    )
    .unwrap();

    let paises: Vec<Pais> = ler_str_ron(
        r#"[
            (
                id: "usa",
                tag: Some("USA"),
                nome: "Estados Unidos",
                mercado_id: "mercado_usa",
                culturas_primarias: ["ocidental"],
                religiao_oficial: "protestante",
                orcamento: (
                    tesouro: 50000000.0,
                    divida: 0.0,
                    limite_credito: 100000000.0,
                    taxa_juros_anual_base: 0.035,
                    taxa_juros_efetiva_anual: 0.035,
                    em_calote: false,
                ),
                politica_fiscal: (
                    lei_sistema_economico_id: "intervencionismo",
                    lei_tributaria_id: "imposto_renda_progressivo",
                    lei_comercial_id: "livre_cambio",
                    aliquota_imposto_renda_baixo: 0.10,
                    aliquota_imposto_renda_medio: 0.15,
                    aliquota_imposto_renda_alto: 0.25,
                    aliquota_imposto_dividendos: 0.20,
                    imposto_sobre_terra_por_campones: 0.02,
                    imposto_sobre_terra_por_nivel_rural: 4.0,
                    aliquota_consumo_geral: 0.04,
                    bens_tributados_consumo: {},
                    tarifa_importacao: 0.05,
                    tarifa_exportacao: 0.02,
                    nivel_investimento_educacao: 4,
                    multiplicador_salario_governo: 1.0,
                    multiplicador_salario_militar: 1.0,
                ),
                pontos_construcao_base: 0.0,
                pib_diario: 8000000.0,
            ),
            (
                id: "can",
                tag: Some("CAN"),
                nome: "Canadá",
                mercado_id: "mercado_exportador",
                culturas_primarias: ["ocidental"],
                religiao_oficial: "protestante",
                orcamento: (
                    tesouro: 20000000.0,
                    divida: 0.0,
                    limite_credito: 50000000.0,
                    taxa_juros_anual_base: 0.035,
                    taxa_juros_efetiva_anual: 0.035,
                    em_calote: false,
                ),
                politica_fiscal: (
                    lei_sistema_economico_id: "intervencionismo",
                    lei_tributaria_id: "imposto_renda_progressivo",
                    lei_comercial_id: "livre_cambio",
                    aliquota_imposto_renda_baixo: 0.10,
                    aliquota_imposto_renda_medio: 0.15,
                    aliquota_imposto_renda_alto: 0.25,
                    aliquota_imposto_dividendos: 0.20,
                    imposto_sobre_terra_por_campones: 0.02,
                    imposto_sobre_terra_por_nivel_rural: 4.0,
                    aliquota_consumo_geral: 0.04,
                    bens_tributados_consumo: {},
                    tarifa_importacao: 0.05,
                    tarifa_exportacao: 0.02,
                    nivel_investimento_educacao: 4,
                    multiplicador_salario_governo: 1.0,
                    multiplicador_salario_militar: 1.0,
                ),
                pontos_construcao_base: 0.0,
                pib_diario: 2000000.0,
            ),
        ]"#,
        "paises_teste",
    )
    .unwrap();

    let estados: Vec<Estado> = ler_str_ron(
        r#"[
            (id: "usa_oeste", nome: "Oeste Americano", pais_id: "usa", incorporado: true, infraestrutura_bonus: 500.0),
            (id: "can_ontario", nome: "Ontário", pais_id: "can", incorporado: true, infraestrutura_bonus: 400.0),
        ]"#,
        "estados_teste",
    )
    .unwrap();

    let provincias: Vec<Provincia> = ler_str_ron(
        r#"[
            (
                id: "usa_prov",
                nome: "Colorado",
                estado_id: "usa_oeste",
                pais_id: "usa",
                terra_aravel_total: 200,
                limites_recursos: {"uranio": 20, "petroleo": 40, "carvao": 60, "ferro": 50, "bauxita": 30, "cobre": 30},
                infraestrutura_base: 500.0,
                pops: [
                    (tipo_pop_id: "mineiros", cultura: "ocidental", religiao: "protestante", populacao: 120000.0, empregados: 90000.0, riqueza_reservas: 400000.0, padrao_de_vida: 20.0, alfabetizacao: 0.85),
                    (tipo_pop_id: "operarios", cultura: "ocidental", religiao: "protestante", populacao: 360000.0, empregados: 280000.0, riqueza_reservas: 1000000.0, padrao_de_vida: 21.0, alfabetizacao: 0.86),
                    (tipo_pop_id: "engenheiros", cultura: "ocidental", religiao: "protestante", populacao: 60000.0, empregados: 44000.0, riqueza_reservas: 600000.0, padrao_de_vida: 28.0, alfabetizacao: 0.95),
                    (tipo_pop_id: "gerentes", cultura: "ocidental", religiao: "protestante", populacao: 40000.0, empregados: 30000.0, riqueza_reservas: 500000.0, padrao_de_vida: 30.0, alfabetizacao: 0.95),
                    (tipo_pop_id: "funcionarios", cultura: "ocidental", religiao: "protestante", populacao: 50000.0, empregados: 36000.0, riqueza_reservas: 400000.0, padrao_de_vida: 24.0, alfabetizacao: 0.90),
                    (tipo_pop_id: "burocratas", cultura: "ocidental", religiao: "protestante", populacao: 30000.0, empregados: 20000.0, riqueza_reservas: 360000.0, padrao_de_vida: 26.0, alfabetizacao: 0.94),
                    (tipo_pop_id: "militares", cultura: "ocidental", religiao: "protestante", populacao: 40000.0, empregados: 30000.0, riqueza_reservas: 320000.0, padrao_de_vida: 22.0, alfabetizacao: 0.88),
                    (tipo_pop_id: "capitalistas", cultura: "ocidental", religiao: "protestante", populacao: 30000.0, empregados: 20000.0, riqueza_reservas: 2000000.0, padrao_de_vida: 42.0, alfabetizacao: 0.98),
                    (tipo_pop_id: "camponeses", cultura: "ocidental", religiao: "protestante", populacao: 160000.0, empregados: 80000.0, riqueza_reservas: 240000.0, padrao_de_vida: 15.0, alfabetizacao: 0.70),
                ],
                predios: [
                    (tipo_predio_id: "mina_uranio", nivel: 6, metodos_ativos: [], empregados_por_tipo: {"mineiros": 1680.0, "engenheiros": 240.0, "militares": 240.0, "burocratas": 120.0}, reservas_caixa: 300000.0),
                    (tipo_predio_id: "poco_petroleo", nivel: 12, metodos_ativos: [], reservas_caixa: 600000.0),
                    (tipo_predio_id: "refinaria_petroleo", nivel: 14, metodos_ativos: [], reservas_caixa: 700000.0),
                    (tipo_predio_id: "mina_carvao", nivel: 18, metodos_ativos: [], reservas_caixa: 900000.0),
                    (tipo_predio_id: "mina_ferro", nivel: 14, metodos_ativos: [], reservas_caixa: 700000.0),
                    (tipo_predio_id: "siderurgica", nivel: 14, metodos_ativos: [], reservas_caixa: 700000.0),
                    (tipo_predio_id: "mina_bauxita", nivel: 10, metodos_ativos: [], reservas_caixa: 500000.0),
                    (tipo_predio_id: "refinaria_aluminio", nivel: 10, metodos_ativos: [], reservas_caixa: 500000.0),
                    (tipo_predio_id: "mina_cobre", nivel: 10, metodos_ativos: [], reservas_caixa: 500000.0),
                    (tipo_predio_id: "fabrica_eletronicos", nivel: 8, metodos_ativos: [], reservas_caixa: 400000.0),
                    (tipo_predio_id: "industria_plasticos", nivel: 8, metodos_ativos: [], reservas_caixa: 400000.0),
                    (tipo_predio_id: "industria_quimica", nivel: 10, metodos_ativos: [], reservas_caixa: 500000.0),
                    (tipo_predio_id: "fabrica_automoveis", nivel: 12, metodos_ativos: [], reservas_caixa: 600000.0),
                    (tipo_predio_id: "fabrica_maquinas", nivel: 12, metodos_ativos: [], reservas_caixa: 600000.0),
                    (tipo_predio_id: "fabrica_ferramentas", nivel: 12, metodos_ativos: [], reservas_caixa: 600000.0),
                    (tipo_predio_id: "serraria", nivel: 12, metodos_ativos: [], reservas_caixa: 600000.0),
                    (tipo_predio_id: "usina_termeletrica", nivel: 16, metodos_ativos: [], reservas_caixa: 800000.0),
                    (tipo_predio_id: "fazenda_graos", nivel: 25, metodos_ativos: [], reservas_caixa: 800000.0),
                ],
            ),
            (
                id: "can_prov",
                nome: "Grande Lago do Urso",
                estado_id: "can_ontario",
                pais_id: "can",
                terra_aravel_total: 150,
                limites_recursos: {"uranio": 30, "borracha": 30, "carvao": 40},
                infraestrutura_base: 300.0,
                pops: [
                    (tipo_pop_id: "mineiros", cultura: "ocidental", religiao: "protestante", populacao: 60000.0, empregados: 45000.0, riqueza_reservas: 200000.0, padrao_de_vida: 20.0, alfabetizacao: 0.85),
                    (tipo_pop_id: "operarios", cultura: "ocidental", religiao: "protestante", populacao: 180000.0, empregados: 140000.0, riqueza_reservas: 500000.0, padrao_de_vida: 21.0, alfabetizacao: 0.86),
                    (tipo_pop_id: "engenheiros", cultura: "ocidental", religiao: "protestante", populacao: 30000.0, empregados: 22000.0, riqueza_reservas: 300000.0, padrao_de_vida: 28.0, alfabetizacao: 0.95),
                    (tipo_pop_id: "gerentes", cultura: "ocidental", religiao: "protestante", populacao: 20000.0, empregados: 15000.0, riqueza_reservas: 250000.0, padrao_de_vida: 30.0, alfabetizacao: 0.95),
                    (tipo_pop_id: "funcionarios", cultura: "ocidental", religiao: "protestante", populacao: 25000.0, empregados: 18000.0, riqueza_reservas: 200000.0, padrao_de_vida: 24.0, alfabetizacao: 0.90),
                    (tipo_pop_id: "burocratas", cultura: "ocidental", religiao: "protestante", populacao: 15000.0, empregados: 10000.0, riqueza_reservas: 180000.0, padrao_de_vida: 26.0, alfabetizacao: 0.94),
                    (tipo_pop_id: "militares", cultura: "ocidental", religiao: "protestante", populacao: 20000.0, empregados: 15000.0, riqueza_reservas: 160000.0, padrao_de_vida: 22.0, alfabetizacao: 0.88),
                    (tipo_pop_id: "capitalistas", cultura: "ocidental", religiao: "protestante", populacao: 15000.0, empregados: 10000.0, riqueza_reservas: 1000000.0, padrao_de_vida: 42.0, alfabetizacao: 0.98),
                    (tipo_pop_id: "camponeses", cultura: "ocidental", religiao: "protestante", populacao: 80000.0, empregados: 40000.0, riqueza_reservas: 120000.0, padrao_de_vida: 15.0, alfabetizacao: 0.70),
                ],
                predios: [
                    (tipo_predio_id: "mina_uranio", nivel: 8, metodos_ativos: [], empregados_por_tipo: {"mineiros": 2240.0, "engenheiros": 320.0, "militares": 320.0, "burocratas": 160.0}, reservas_caixa: 400000.0),
                    (tipo_predio_id: "plantacao_borracha", nivel: 12, metodos_ativos: [], reservas_caixa: 600000.0),
                    (tipo_predio_id: "mina_carvao", nivel: 12, metodos_ativos: [], reservas_caixa: 600000.0),
                    (tipo_predio_id: "fabrica_ferramentas", nivel: 8, metodos_ativos: [], reservas_caixa: 400000.0),
                    (tipo_predio_id: "serraria", nivel: 8, metodos_ativos: [], reservas_caixa: 400000.0),
                    (tipo_predio_id: "usina_termeletrica", nivel: 8, metodos_ativos: [], reservas_caixa: 400000.0),
                    (tipo_predio_id: "fazenda_graos", nivel: 15, metodos_ativos: [], reservas_caixa: 500000.0),
                ],
            ),
        ]"#,
        "provincias_teste",
    )
    .unwrap();

    let cenario = CenarioEconomia {
        data_inicial: DataJogo {
            ano: 1945,
            mes: 1,
            dia: 1,
        },
        config,
        catalogo,
        mercados,
        paises,
        estados,
        provincias,
        rotas_comerciais: Vec::new(),
        blocos_comerciais: Vec::new(),
    };

    let mut mundo = MundoEconomico::novo(cenario).expect("mundo compacto deve inicializar");
    mundo.definir_semente_ciclos(semente);
    mundo
}

#[test]
fn catalogo_carrega_ciclos_demanda_ron_valido() {
    let cat = carregar_catalogo_real();
    assert!(
        cat.ciclos_demanda.curvas.len() >= 12,
        "esperava >= 12 curvas históricas em ciclos_demanda.ron, achou {}",
        cat.ciclos_demanda.curvas.len()
    );
    for bem_esperado in [
        "uranio",
        "petroleo",
        "combustiveis_refinados",
        "carvao",
        "borracha_natural",
        "plasticos_sinteticos",
        "aco",
        "aluminio",
        "eletronicos",
    ] {
        assert!(
            cat.ciclos_demanda
                .curvas
                .iter()
                .any(|c| c.bem_id == bem_esperado),
            "faltando curva histórica para '{bem_esperado}'"
        );
    }
}

#[test]
fn uranio_sobe_na_corrida_pos_1945_e_cai_com_tratados_e_superoferta() {
    let mut mundo = criar_mundo_compacto_historico(42);

    // Estabiliza preços iniciais em meados de 1945 (30 dias)
    mundo.avancar_dias(30);
    let preco_1945 = mundo.preco_atual("mercado_usa", "uranio").unwrap();
    let preco_exp_1945 = mundo.preco_atual("mercado_exportador", "uranio").unwrap();

    // Avança até 1954 (~9 anos: corrida nuclear EUA/URSS + bombas H)
    mundo.registrar_tecnologia_pesquisada("usa", "bomba_atomica_fissao_implosao");
    mundo.registrar_tecnologia_pesquisada("usa", "bomba_termonuclear_fusao_hidrogenio");
    mundo.avancar_dias(365 * 9);
    let preco_1954 = mundo.preco_atual("mercado_usa", "uranio").unwrap();
    let preco_exp_1954 = mundo.preco_atual("mercado_exportador", "uranio").unwrap();

    let lucro_mina_1954 = mundo
        .obter_provincia("can_prov")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_uranio")
        .unwrap()
        .lucro_ultimo_tick;

    assert!(
        preco_1954 > preco_1945 * 1.15,
        "preço do urânio nos EUA deveria subir na corrida nuclear pós-1945: 1945={preco_1945:.2}, 1954={preco_1954:.2}"
    );
    assert!(
        preco_exp_1954 > preco_exp_1945 * 1.12,
        "preço do urânio no exportador deveria subir na corrida nuclear: 1945={preco_exp_1945:.2}, 1954={preco_exp_1954:.2}"
    );
    assert!(
        lucro_mina_1954 > 0.0,
        "mina de urânio deve ser lucrativa no auge da corrida nuclear (lucro={lucro_mina_1954:.1})"
    );

    // Avança até 1987 (~33 anos adicionais: centrífugas, TNP, Three Mile Island, Chernobyl, INF)
    for pais in ["usa", "can"] {
        mundo.registrar_tecnologia_pesquisada(pais, "separacao_isotopica_centrifuga_gasosa");
        mundo.registrar_tecnologia_pesquisada(pais, "tratados_desarmamento_inf_start_verificacao");
    }
    mundo.avancar_dias(365 * 33);
    mundo.registrar_evento_historico("desastre_nuclear_chernobyl_1986");
    mundo.registrar_evento_historico("tratado_inf_desarmamento_1987");
    mundo.avancar_dias(60);

    let preco_1987 = mundo.preco_atual("mercado_usa", "uranio").unwrap();
    let preco_exp_1987 = mundo.preco_atual("mercado_exportador", "uranio").unwrap();
    let lucro_mina_1987 = mundo
        .obter_provincia("can_prov")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_uranio")
        .unwrap()
        .lucro_ultimo_tick;

    assert!(
        preco_1987 < preco_1954 * 0.55 && preco_exp_1987 < preco_exp_1945 * 0.70,
        "preço do urânio deve cair forte com tratados e excesso de oferta: 1945={preco_exp_1945:.2}, 1954={preco_1954:.2}, 1987_usa={preco_1987:.2}, 1987_exp={preco_exp_1987:.2}"
    );
    assert!(
        lucro_mina_1987 < lucro_mina_1954 * 0.25,
        "rentabilidade da mina de urânio deve despencar após o fim da corrida: 1954={lucro_mina_1954:.1}, 1987={lucro_mina_1987:.1}"
    );
    mundo.auditar_estabilidade().unwrap();
}

#[test]
fn petroleo_sobe_anos_50_70_e_dispara_choques_e_carvao_borracha_caem() {
    let mut mundo = criar_mundo_compacto_historico(777);
    mundo.definir_gatilhos_historicos_autonomos(false);

    mundo.avancar_dias(45);
    let pet_1945 = mundo.preco_atual("mercado_usa", "petroleo").unwrap();
    let carvao_dem_1945 = mundo.demanda_atual("mercado_usa", "carvao").unwrap();
    let borr_1945 = mundo
        .preco_atual("mercado_exportador", "borracha_natural")
        .unwrap();
    let elet_1945 = mundo.fator_demanda_historica_bem("eletronicos");
    let alum_1945 = mundo.fator_demanda_historica_bem("aluminio");

    // Avança até 1971 (26 anos)
    for pais in ["usa", "can"] {
        mundo.registrar_tecnologia_pesquisada(pais, "urbanismo_rodoviario_suburbano");
        mundo.registrar_tecnologia_pesquisada(pais, "borracha_sintetica_elastomeros");
        mundo.registrar_tecnologia_pesquisada(pais, "gasodutos_oleodutos_transcontinentais");
        mundo.registrar_tecnologia_pesquisada(pais, "metalurgia_aluminio_ligas_leves");
        mundo.registrar_tecnologia_pesquisada(pais, "computadores_pessoais_escritorio");
    }
    mundo.avancar_dias(365 * 26);

    let pet_1971 = mundo.preco_atual("mercado_usa", "petroleo").unwrap();
    let carvao_fator_1971 = mundo.fator_demanda_historica_bem("carvao");
    let borr_1971 = mundo
        .preco_atual("mercado_exportador", "borracha_natural")
        .unwrap();
    let elet_1971 = mundo.fator_demanda_historica_bem("eletronicos");
    let alum_1971 = mundo.fator_demanda_historica_bem("aluminio");

    assert!(
        pet_1971 > pet_1945 * 1.05,
        "petróleo deve valorizar entre 1945 ({pet_1945:.2}) e 1971 ({pet_1971:.2})"
    );
    assert!(
        carvao_fator_1971 < 0.72,
        "carvão deve perder espaço estrutural para petróleo/gás: fator={carvao_fator_1971:.2} (dem45={carvao_dem_1945:.1})"
    );
    assert!(
        borr_1971 < borr_1945 * 0.85,
        "borracha natural deve cair com a sintética: 1945={borr_1945:.2} -> 1971={borr_1971:.2}"
    );
    assert!(
        elet_1971 > elet_1945 * 1.60 && alum_1971 > alum_1945 * 1.45,
        "eletrônicos ({elet_1971:.2}) e alumínio ({alum_1971:.2}) devem crescer fortemente na era moderna"
    );

    // Dispara o Primeiro Choque do Petróleo (1973)
    assert!(mundo.registrar_evento_historico("primeiro_choque_petroleo_opep_1973"));
    mundo.avancar_dias(45);
    let pet_choque_1973 = mundo.preco_atual("mercado_usa", "petroleo").unwrap();
    assert!(
        pet_choque_1973 > pet_1971 * 1.08,
        "choque de 1973 deve elevar o preço do petróleo acima da tendência: pré={pet_1971:.2}, choque={pet_choque_1973:.2}"
    );

    mundo.auditar_estabilidade().unwrap();
}

#[test]
fn sementes_diferentes_geram_variacao_sem_decorar_futuro() {
    let mut m_a = criar_mundo_compacto_historico(1001);
    let mut m_b = criar_mundo_compacto_historico(9009);

    m_a.avancar_dias(365 * 8);
    m_b.avancar_dias(365 * 8);

    let u_a = m_a.preco_atual("mercado_usa", "uranio").unwrap();
    let u_b = m_b.preco_atual("mercado_usa", "uranio").unwrap();
    let p_a = m_a.fator_demanda_historica_bem("petroleo");
    let p_b = m_b.fator_demanda_historica_bem("petroleo");

    assert!(
        (u_a - u_b).abs() > 0.25 || (p_a - p_b).abs() > 0.01,
        "sementes distintas devem variar picos e datas: u_a={u_a:.3}, u_b={u_b:.3}, p_a={p_a:.4}, p_b={p_b:.4}"
    );
    m_a.auditar_estabilidade().unwrap();
    m_b.auditar_estabilidade().unwrap();
}
