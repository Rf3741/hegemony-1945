//! Teste de legitimidade: democracias consolidadas se mantêm na faixa histórica
//! de 55..=85 após ~4 anos (1460 dias) simulados, sem eventos extremos.

use sim_politica::*;
use std::path::PathBuf;

fn pais_estados_unidos_e_catalogo() -> (CatalogoPolitica, PaisPolitica) {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/politica");
    let catalogo =
        CatalogoPolitica::de_arquivo_ron(raiz.join("catalogo_politica.ron")).expect("catalogo");
    let paises: Vec<PaisPolitica> = ron::from_str(
        &std::fs::read_to_string(raiz.join("paises_politica.ron")).expect("paises"),
    )
    .expect("paises ron");
    let pais = paises
        .into_iter()
        .find(|p| p.tag == "USA")
        .expect("USA no cenario");
    (catalogo, pais)
}

#[test]
fn democracia_estavel_mantem_legitimidade_na_faixa_historica_apos_quatro_anos() {
    let (catalogo, pais) = pais_estados_unidos_e_catalogo();

    // O pais tem 5 das 6 leis democraticas fundamentais (falta
    // cidadania_civica_jus_soli), ou seja, eh uma democracia consolidada.
    let num_democraticas = [
        "sufragio_universal",
        "garantias_constitucionais_amplas",
        "imprensa_livre",
        "estado_laico",
        "sindicatos_livres_regulamentados",
        "cidadania_civica_jus_soli",
    ]
    .iter()
    .filter(|lei| pais.leis_ativas.values().any(|v| v == *lei))
    .count();
    assert!(num_democraticas >= 4, "esperava democracia consolidada: {num_democraticas}");

    let mut mundo = MundoPolitico::novo(catalogo, vec![pais], 19_450_101)
        .expect("constroi mundo politico");
    let _relatorio = mundo.executar_periodo(1460);

    let eua = mundo.pais("usa").expect("USA apos simulacao");
    mundo.auditar_estabilidade().unwrap();

    assert!(
        (55.0..=85.0).contains(&eua.legitimidade),
        "legitimidade dos EUA fora da faixa historica: {}",
        eua.legitimidade
    );
}
