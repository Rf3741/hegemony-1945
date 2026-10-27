//! Teste headless: carrega o cenário real de 1945 e roda 5 anos de ticks sem NaN nem pânico.
use simulacao::mundo::MundoJogo;
use std::path::PathBuf;

fn checar(mundo: &MundoJogo, ctx: &str) {
    for (tag, p) in &mundo.paises {
        let o = &p.orcamento;
        for (nome, v) in [
            ("tesouro", o.tesouro),
            ("divida", o.divida_soberana),
            ("limite_credito", o.limite_credito),
            ("receita_impostos", o.receita_impostos_diaria),
            ("despesa_adm", o.despesa_administrativa_diaria),
            ("saldo", o.saldo_diario()),
            ("pib", p.pib_anual_estimado),
            ("padrao_vida", p.padrao_vida_medio() as f64),
            ("legitimidade", p.politica.legitimidade as f64),
        ] {
            assert!(v.is_finite(), "{ctx}: {tag} {nome} = {v}");
        }
    }
    for (id, m) in &mundo.mercados {
        for (b, v) in &m.precos {
            assert!(v.is_finite() && *v > 0.0, "{ctx}: mercado {id} preço {b:?} = {v}");
        }
    }
}

#[test]
fn cinco_anos_de_ticks_sem_nan() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (_, mut mundo) = MundoJogo::carregar_de_arquivos(
        raiz.join("assets/map/provinces.ron"),
        raiz.join("data/scenario_1945/countries_1945.ron"),
        raiz.join("data/scenario_1945/province_owners_1945.ron"),
    )
    .expect("cenário 1945");
    checar(&mundo, "inicio");
    let pop0 = mundo.paises["USA"].populacao_total();
    for dia in 0..365 * 5 {
        mundo.executar_tick_diario();
        if dia % 90 == 0 {
            checar(&mundo, &format!("dia {dia}"));
        }
    }
    checar(&mundo, "fim");
    for tag in ["USA", "SOV"] {
        let p = &mundo.paises[tag];
        assert!(p.pib_anual_estimado > 0.0, "{tag} PIB = {}", p.pib_anual_estimado);
        eprintln!("{tag}: PIB {:.0} pop {} tesouro {:.0} divida {:.0} pv {:.1} leg {:.1}", p.pib_anual_estimado, p.populacao_total(), p.orcamento.tesouro, p.orcamento.divida_soberana, p.padrao_vida_medio(), p.politica.legitimidade);
    }
    let pop5 = mundo.paises["USA"].populacao_total();
    assert!(pop5 < pop0 * 2, "população explodiu: {pop0} -> {pop5}");
}
