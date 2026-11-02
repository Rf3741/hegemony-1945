//! Crescimento macro: com a IA econômica ligada, 5 anos de simulação (partindo do mundo de 1945, depois de
//! 90 dias de aquecimento) devem dar crescimento médio do PIB mundial entre 3% e 7% ao ano (referência
//! histórica dos anos 1950: ~4-5%), sem NaN, com a população crescendo e o emprego não despencando.
use ia_paises::IaEconomia;
use sim_economia::{CenarioEconomia, ModoCadencia, MundoEconomico};
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pib_mundial(m: &MundoEconomico) -> f64 {
    m.paises().iter().map(|p| p.pib_diario).sum::<f64>() * 365.0
}

fn populacao(m: &MundoEconomico) -> f64 {
    m.provincias().iter().map(|p| p.populacao_total()).sum()
}

fn emprego(m: &MundoEconomico) -> f64 {
    let (mut emp, mut pop) = (0.0, 0.0);
    for pr in m.provincias() {
        for p in &pr.pops {
            emp += p.empregados;
            pop += p.populacao;
        }
    }
    emp / pop
}

#[test]
fn cinco_anos_crescimento_medio_entre_3_e_7_por_cento() {
    let mut cen = CenarioEconomia::carregar_diretorio(&raiz().join("assets/data/economia")).expect("economia");
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    let mut eco = MundoEconomico::novo(cen).expect("mundo economia");
    let mut ia = IaEconomia::novo();
    for _ in 0..90 {
        eco.executar_tick_diario();
    }
    let (pib0, pop0, emp0) = (pib_mundial(&eco), populacao(&eco), emprego(&eco));
    let mut anual = vec![pib0];
    for dia in 1..=(365 * 5) {
        eco.executar_tick_diario();
        ia.passo_diario(&mut eco, &[]);
        if dia % 365 == 0 {
            anual.push(pib_mundial(&eco));
        }
    }
    eco.auditar_estabilidade().expect("estabilidade da economia");

    let (pib1, pop1, emp1) = (pib_mundial(&eco), populacao(&eco), emprego(&eco));
    assert!(pib1.is_finite() && pop1.is_finite() && emp1.is_finite(), "NaN/inf no resultado");
    for p in eco.paises() {
        assert!(
            p.pib_diario.is_finite() && p.produtividade.is_finite() && p.orcamento.tesouro.is_finite(),
            "valor inválido em {}",
            p.id
        );
    }
    let cagr = (pib1 / pib0).powf(1.0 / 5.0) - 1.0;
    let por_ano: Vec<String> = anual.windows(2).map(|w| format!("{:+.1}%", (w[1] / w[0] - 1.0) * 100.0)).collect();
    eprintln!(
        "PIB mundial: {pib0:.3e} -> {pib1:.3e} ({:.2}%/ano; por ano: {}); pop {:+.1}%; emprego/pop {emp0:.3} -> {emp1:.3}",
        cagr * 100.0,
        por_ano.join(" "),
        (pop1 / pop0 - 1.0) * 100.0
    );
    assert!((0.03..=0.07).contains(&cagr), "crescimento médio fora de 3%-7% ao ano: {:.2}%", cagr * 100.0);
    assert!(pop1 > pop0, "a população mundial deveria crescer");
    assert!(emp1 >= 0.9 * emp0, "o emprego não pode desabar: {emp0:.3} -> {emp1:.3}");
}
