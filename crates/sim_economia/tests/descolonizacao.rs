use sim_economia::{CenarioEconomia, MundoEconomico};

#[test]
fn particao_india_cria_pais_mercado_e_reduz_metropole() {
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cenario = CenarioEconomia::carregar_diretorio(&raiz.join("assets/data/economia")).unwrap();
    let mut mundo = MundoEconomico::novo(cenario).unwrap();
    let ind = mundo.paises().iter().find(|p| p.tag.as_deref() == Some("IND")).unwrap().id.clone();
    let antes = mundo.populacao_total_pais(&ind).unwrap();
    mundo.executar_tick_diario();
    let pib_ind_antes = mundo.paises().iter().find(|p| p.id == ind).unwrap().pib_diario;
    mundo
        .adicionar_pais_formado("PAK", "Paquistão", &["IND".to_string()], &["PAK".to_string()])
        .unwrap();
    let pak = mundo.paises().iter().find(|p| p.tag.as_deref() == Some("PAK")).unwrap();
    assert!(mundo.populacao_total_pais(&pak.id).unwrap() > 0.0);
    assert!(mundo.populacao_total_pais(&ind).unwrap() < antes);
    assert!(mundo.mercados().iter().any(|m| m.id == "mercado_pak"));
    mundo.executar_tick_diario();
    let pak = mundo.paises().iter().find(|p| p.tag.as_deref() == Some("PAK")).unwrap();
    assert!(pak.pib_diario > 0.0);
    let pib_ind_depois = mundo.paises().iter().find(|p| p.id == ind).unwrap().pib_diario;
    assert!(pib_ind_depois > 0.0 && pib_ind_depois < pib_ind_antes);
}
