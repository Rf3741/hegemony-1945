//! Turismo e hotelaria: atratividade histórica, exportação de serviços, imposto e queda por risco.
use sim_economia::{CenarioEconomia, MundoEconomico};
use std::path::Path;

fn carregar() -> MundoEconomico {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
    let mut cen = CenarioEconomia::carregar_diretorio(&dir).unwrap();
    cen.config.modo_cadencia = sim_economia::ModoCadencia::DiarioComAjusteSemanal;
    MundoEconomico::novo(cen).unwrap()
}

#[test]
fn turismo_segue_a_atratividade_historica() {
    let mut m = carregar();
    m.executar_tick_diario();
    let r = |id: &str| m.turismo_receita_pais(id);
    // destinos históricos recebem muito mais que países atrás da Cortina de Ferro
    assert!(r("fra") > 0.0 && r("ita") > 0.0 && r("esp") > 0.0 && r("che") > 0.0);
    assert!(r("fra") > 5.0 * r("pol"), "fra {} pol {}", r("fra"), r("pol"));
    assert!(r("ita") > 5.0 * r("cze"));
    assert!(r("mex") > r("sov"));
    // exportação de serviços é pequena frente ao PIB
    let pib_fra = m.obter_pais("fra").unwrap().pib_diario;
    assert!(r("fra") < 0.05 * pib_fra, "turismo {} pib {}", r("fra"), pib_fra);
    // imposto entra no orçamento
    assert!(m.obter_pais("fra").unwrap().orcamento.receita_turismo > 0.0);
}

#[test]
fn guerra_derruba_o_turismo() {
    let mut m = carregar();
    m.executar_tick_diario();
    let antes = m.turismo_receita_pais("gre");
    m.definir_risco_turismo("gre", 0.9);
    m.executar_tick_diario();
    let depois = m.turismo_receita_pais("gre");
    assert!(depois < 0.3 * antes, "antes {antes} depois {depois}");
    assert!(m.turismo_fator_seguranca_pais("gre") < 0.2);
}
