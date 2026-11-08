//! Testes das mecânicas de fechar temporariamente, reabrir com retomada e demolir construções,
//! bem como persistência em salvar/carregar e compatibilidade retroativa (`#[serde(default)]`).

use sim_economia::{
    CUSTO_MANUTENCAO_FECHADO_POR_NIVEL, CenarioEconomia, EstadoEconomiaSalvo, ModoCadencia,
    MundoEconomico, Predio, estado_niveis_recurso,
};
use std::path::PathBuf;

fn dir_fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn carregar_mundo_fixtures() -> MundoEconomico {
    let mut cenario = CenarioEconomia::carregar_diretorio(&dir_fixtures()).expect("fixtures");
    cenario.config.modo_cadencia = ModoCadencia::DiarioCompleto;
    for est in &mut cenario.estados {
        if est.terra_aravel == 0 {
            est.terra_aravel = 50;
        }
    }
    MundoEconomico::novo(cenario).expect("mundo")
}

#[test]
fn fechar_para_producao_e_consumo_demite_trabalhadores_e_cobra_manutencao_baixa() {
    let mut mundo = carregar_mundo_fixtures();
    mundo.avancar_dias(5);

    // Siderúrgica em Liège (Bélgica): consome minério de ferro + carvão e produz aço
    let prov_antes = mundo.obter_provincia("liege").unwrap();
    let sid_antes = prov_antes
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "usina_siderurgica")
        .expect("usina_siderurgica em Liège");
    assert!(!sid_antes.fechado);
    assert!(sid_antes.escala_efetiva() > 0.0, "siderúrgica deve produzir antes de fechar");
    assert!(sid_antes.receita_ultimo_tick > 0.0);
    assert!(sid_antes.custo_insumos_ultimo_tick > 100.0);
    let emp_sid_antes = sid_antes.total_empregados();
    assert!(emp_sid_antes > 100.0, "siderúrgica deve ter empregados ativos");

    let desemp_prov_antes: f64 = prov_antes.pops.iter().map(|p| p.desempregados()).sum();
    let m_belga = mundo.indice_mercado_do_pais("belgica").unwrap();
    let g_aco = mundo.catalogo().indice_bem("aco").unwrap();
    let g_carvao = mundo.catalogo().indice_bem("carvao").unwrap();
    let oferta_aco_antes = mundo.dados_mercado_bem(m_belga, g_aco).1;
    let demanda_carvao_antes = mundo.dados_mercado_bem(m_belga, g_carvao).2;
    assert!(oferta_aco_antes > 0.0);
    assert!(demanda_carvao_antes > 0.0);

    // Fecha a siderúrgica
    let res = mundo
        .fechar_predio("belgica", "liege", "usina_siderurgica")
        .expect("fechar usina_siderurgica");
    assert!(
        (res.trabalhadores_demitidos - emp_sid_antes).abs() < 1e-4,
        "todos os empregados do prédio fechado devem ser desligados"
    );
    assert!(res.fracao_forca_trabalho_provincia > 0.0);

    // Imediatamente após fechar, desempregados na província sobem pelo mesmo montante
    let prov_logo = mundo.obter_provincia("liege").unwrap();
    let desemp_prov_logo: f64 = prov_logo.pops.iter().map(|p| p.desempregados()).sum();
    assert!(
        desemp_prov_logo >= desemp_prov_antes + emp_sid_antes - 1.0,
        "desempregados na província devem aumentar imediatamente ({desemp_prov_antes:.0} -> {desemp_prov_logo:.0})"
    );

    // Avança 1 dia para verificar produção = 0, consumo de insumos = 0 e custo de manutenção baixo
    mundo.executar_tick_diario();

    let prov_depois = mundo.obter_provincia("liege").unwrap();
    let sid_depois = prov_depois
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "usina_siderurgica")
        .unwrap();
    assert!(sid_depois.fechado);
    assert_eq!(sid_depois.escala_efetiva(), 0.0, "prédio fechado não deve produzir");
    assert_eq!(sid_depois.total_empregados(), 0.0, "prédio fechado não deve recontratar");
    assert_eq!(sid_depois.receita_ultimo_tick, 0.0);
    assert_eq!(sid_depois.folha_salarial_ultimo_tick, 0.0);
    let manut_esperada = (sid_depois.nivel as f64) * CUSTO_MANUTENCAO_FECHADO_POR_NIVEL;
    assert!(
        (sid_depois.lucro_ultimo_tick - (-manut_esperada)).abs() < 1e-6,
        "prédio fechado deve ter custo baixo de manutenção (-{manut_esperada}), deu {}",
        sid_depois.lucro_ultimo_tick
    );

    let oferta_aco_depois = mundo.dados_mercado_bem(m_belga, g_aco).1;
    let demanda_carvao_depois = mundo.dados_mercado_bem(m_belga, g_carvao).2;

    assert!(
        oferta_aco_depois < oferta_aco_antes * 0.7,
        "oferta de aço deve cair drasticamente ao fechar a siderúrgica ({oferta_aco_antes:.1} -> {oferta_aco_depois:.1})"
    );
    assert!(
        demanda_carvao_depois < demanda_carvao_antes,
        "consumo de carvão (insumo) deve cair ao fechar a siderúrgica ({demanda_carvao_antes:.1} -> {demanda_carvao_depois:.1})"
    );
}

#[test]
fn reabrir_cobra_custo_entra_em_retomada_e_volta_a_produzir() {
    let mut mundo = carregar_mundo_fixtures();
    mundo.avancar_dias(5);

    mundo
        .fechar_predio("belgica", "liege", "usina_siderurgica")
        .expect("fechar usina_siderurgica");
    mundo.executar_tick_diario();

    let tesouro_antes = mundo.obter_pais("belgica").unwrap().orcamento.tesouro;
    let custo_previsto = mundo
        .custo_reabrir_predio("belgica", "liege", "usina_siderurgica")
        .expect("custo reabrir");
    assert!(custo_previsto > 0.0);

    let res = mundo
        .reabrir_predio("belgica", "liege", "usina_siderurgica", false)
        .expect("reabrir usina_siderurgica");
    assert!((res.valor_monetario - custo_previsto).abs() < 1e-6);

    let tesouro_logo = mundo.obter_pais("belgica").unwrap().orcamento.tesouro;
    assert!(
        (tesouro_antes - tesouro_logo - custo_previsto).abs() < 1e-4,
        "tesouro deve pagar o custo de reabertura"
    );

    let sid_logo = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "usina_siderurgica")
        .unwrap();
    assert!(!sid_logo.fechado);
    assert_eq!(sid_logo.dias_retomada, Predio::TEMPO_RETOMADA_DIAS);
    assert!(sid_logo.esta_em_retomada());
    assert!(sid_logo.fator_retomada() < 1.0 && sid_logo.fator_retomada() >= 0.25);

    // No 1º dia após reabrir já opera em escala parcial (retomada)
    mundo.executar_tick_diario();
    let sid_dia1 = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "usina_siderurgica")
        .unwrap();
    let escala_dia1 = sid_dia1.escala_efetiva();
    assert!(escala_dia1 > 0.0, "deve voltar a produzir já na retomada");
    assert!(sid_dia1.dias_retomada < Predio::TEMPO_RETOMADA_DIAS);

    // Após concluir o tempo de retomada, atinge escala superior à do 1º dia e zera dias_retomada
    mundo.avancar_dias(Predio::TEMPO_RETOMADA_DIAS as u32 + 5);
    let sid_final = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "usina_siderurgica")
        .unwrap();
    assert_eq!(sid_final.dias_retomada, 0);
    assert!(!sid_final.esta_em_retomada());
    assert!(
        sid_final.escala_efetiva() > escala_dia1 * 1.5,
        "após o tempo de retomada a produção deve superar a escala inicial ({escala_dia1:.2} -> {:.2})",
        sid_final.escala_efetiva()
    );
}

#[test]
fn demolir_reduz_niveis_libera_espaco_e_devolve_parte_do_valor() {
    let mut mundo = carregar_mundo_fixtures();
    mundo.avancar_dias(3);

    // Verifica fazenda_graos em campinas_santos (usa terra arável) e mina_carvao em charleroi (usa recurso carvao)
    let terra_livre_prov_antes = mundo
        .obter_provincia("campinas_santos")
        .unwrap()
        .terra_aravel_livre();
    let terra_livre_est_antes = mundo
        .obter_estado("estado_sao_paulo")
        .unwrap()
        .terra_aravel_livre();
    let tesouro_bra_antes = mundo.obter_pais("brasil").unwrap().orcamento.tesouro;

    let reembolso_esp = mundo
        .reembolso_demolir_predio("brasil", "campinas_santos", "fazenda_graos", 1)
        .expect("reembolso demolir");
    assert!(reembolso_esp > 0.0);

    let res_dem = mundo
        .demolir_predio("brasil", "campinas_santos", "fazenda_graos", 1, false)
        .expect("demolir 1 nível de fazenda_graos");
    assert!((res_dem.valor_monetario - reembolso_esp).abs() < 1e-6);

    let terra_livre_prov_depois = mundo
        .obter_provincia("campinas_santos")
        .unwrap()
        .terra_aravel_livre();
    let terra_livre_est_depois = mundo
        .obter_estado("estado_sao_paulo")
        .unwrap()
        .terra_aravel_livre();
    let tesouro_bra_depois = mundo.obter_pais("brasil").unwrap().orcamento.tesouro;

    assert_eq!(
        terra_livre_prov_depois,
        terra_livre_prov_antes + 1,
        "demolir fazenda deve liberar 1 unidade de terra arável na província"
    );
    assert_eq!(
        terra_livre_est_depois,
        terra_livre_est_antes + 1,
        "demolir fazenda deve liberar 1 unidade de terra arável no estado"
    );
    assert!(
        tesouro_bra_depois >= tesouro_bra_antes + reembolso_esp - 1e-6,
        "tesouro deve receber o reembolso parcial da demolição"
    );

    // Demolir todos os níveis de mina_carvao em Charleroi libera os níveis de recurso mineral no estado
    let est_valonia = mundo.obter_estado("valonia").unwrap();
    let niv_carvao_antes = estado_niveis_recurso(
        mundo.catalogo(),
        est_valonia,
        mundo.provincias(),
        "carvao",
    );
    assert!(niv_carvao_antes >= 1);

    let res_mina = mundo
        .demolir_predio("belgica", "charleroi", "mina_carvao", 99, true)
        .expect("demolir toda mina_carvao");
    assert_eq!(res_mina.nivel_restante, 0);

    let est_valonia_dep = mundo.obter_estado("valonia").unwrap();
    let niv_carvao_depois = estado_niveis_recurso(
        mundo.catalogo(),
        est_valonia_dep,
        mundo.provincias(),
        "carvao",
    );
    assert!(
        niv_carvao_depois < niv_carvao_antes,
        "demolir mina deve liberar espaço da jazida no estado ({niv_carvao_antes} -> {niv_carvao_depois})"
    );
    assert!(
        !mundo
            .obter_provincia("charleroi")
            .unwrap()
            .predios
            .iter()
            .any(|b| b.tipo_predio_id == "mina_carvao"),
        "prédio com nível 0 deve ser removido da província"
    );
    mundo.auditar_estabilidade().expect("invariantes após demolição");
}

#[test]
fn salvar_carregar_preserva_estados_de_predio_e_aceita_save_antigo() {
    let mut mundo = carregar_mundo_fixtures();
    mundo.avancar_dias(4);

    // Fecha um prédio e coloca outro em retomada
    mundo
        .fechar_predio("belgica", "liege", "usina_siderurgica")
        .unwrap();
    mundo
        .fechar_predio("belgica", "antuerpia", "usina_eletrica")
        .unwrap();
    mundo
        .reabrir_predio("belgica", "antuerpia", "usina_eletrica", false)
        .unwrap();
    mundo.executar_tick_diario();

    let salvo = mundo.salvar_estado();
    let ron_salvo = ron::to_string(&salvo).expect("serializar EstadoEconomiaSalvo");

    // 1. Roundtrip completo preserva fechado, dias_retomada e ciclos_demanda
    let salvo_lido: EstadoEconomiaSalvo = ron::from_str(&ron_salvo).expect("desserializar");
    let mundo2 = MundoEconomico::restaurar_estado(mundo.catalogo().clone(), salvo_lido)
        .expect("restaurar");

    let sid2 = mundo2
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "usina_siderurgica")
        .unwrap();
    assert!(sid2.fechado);
    assert!(sid2.dias_fechado >= 1);

    let usi2 = mundo2
        .obter_provincia("antuerpia")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "usina_eletrica")
        .unwrap();
    assert!(!usi2.fechado);
    assert!(usi2.dias_retomada > 0);

    // 2. Compatibilidade com save antigo em RON (sem os campos novos de Predio)
    let ron_pretty =
        ron::ser::to_string_pretty(&salvo, ron::ser::PrettyConfig::default()).expect("pretty ron");
    let ron_antigo: String = ron_pretty
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("fechado:")
                && !t.starts_with("dias_retomada:")
                && !t.starts_with("dias_prejuizo:")
                && !t.starts_with("dias_fechado:")
                && !t.starts_with("lucro_ema:")
                && !t.starts_with("tendencia_lucro:")
                && !t.starts_with("dia_ultima_mudanca_estado:")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let salvo_antigo: EstadoEconomiaSalvo =
        ron::from_str(&ron_antigo).expect("save antigo deve abrir via #[serde(default)]");
    let mut mundo_antigo =
        MundoEconomico::restaurar_estado(mundo.catalogo().clone(), salvo_antigo)
            .expect("restaurar save antigo");
    mundo_antigo.executar_tick_diario();
    mundo_antigo
        .auditar_estabilidade()
        .expect("mundo restaurado de save antigo estável");
}
