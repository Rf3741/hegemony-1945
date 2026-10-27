//! Testes de integração de longo prazo (10 anos completos: 01/01/1945 a 01/01/1955, 3.652 ticks diários).
//! Verifica ausência total de `NaN`/`Inf`, preços contidos nos limites, populações não negativas,
//! crescimento demográfico e industrial plausível e estabilidade fiscal.

use sim_economia::{CenarioEconomia, DataJogo, FormulaPreco, ModoCadencia, MundoEconomico};
use std::path::Path;

fn carregar_fixtures() -> CenarioEconomia {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    CenarioEconomia::carregar_diretorio(&dir).expect("fixtures RON devem carregar sem erros")
}

/// Extrai apenas a Bélgica (país pequeno de 5 províncias e 2 estados) em cenário autônomo
/// para validar 10 anos de simulação contínua de uma economia nacional compacta.
fn construir_cenario_belgica_isolada() -> CenarioEconomia {
    let mut base = carregar_fixtures();
    base.mercados.retain(|m| m.id == "mercado_belga");
    base.paises.retain(|p| p.id == "belgica");
    base.estados.retain(|e| e.pais_id == "belgica");
    base.provincias.retain(|pr| pr.pais_id == "belgica");
    base.rotas_comerciais.clear();
    base
}

#[test]
fn teste_integracao_10_anos_pais_pequeno_estabilidade_completa() {
    let cenario = construir_cenario_belgica_isolada();
    let mut mundo = MundoEconomico::novo(cenario).expect("cenário belga isolado deve ser válido");

    assert_eq!(mundo.paises().len(), 1);
    assert_eq!(mundo.provincias().len(), 5);
    assert_eq!(mundo.data_atual(), DataJogo::nova(1945, 1, 1).unwrap());

    let pop_inicial = mundo.populacao_total_pais("belgica").unwrap();
    let alfa_inicial = mundo.alfabetizacao_media_pais("belgica").unwrap();
    let niveis_iniciais: u32 = mundo
        .provincias()
        .iter()
        .flat_map(|p| &p.predios)
        .map(|b| b.nivel)
        .sum();

    // 1945..1955 contém 2 anos bissextos (1948 e 1952): 10 * 365 + 2 = 3.652 dias
    let total_dias_10_anos = 3_652u32;
    let mut total_obras_decada = 0usize;

    for dia in 1..=total_dias_10_anos {
        let rel = mundo.executar_tick_diario();
        total_obras_decada += rel.obras_concluidas;

        // Auditoria completa mensal e no último dia
        if dia % 30 == 0 || dia == total_dias_10_anos {
            mundo.auditar_estabilidade().unwrap_or_else(|erro| {
                panic!(
                    "falha de estabilidade no dia {dia} ({}): {erro}",
                    mundo.data_atual()
                )
            });

            let pop_atual = mundo.populacao_total_pais("belgica").unwrap();
            assert!(
                pop_atual > 0.0 && pop_atual.is_finite(),
                "população nacional inválida no dia {dia}: {pop_atual}"
            );
            assert!(
                rel.pib_mundial_diario > 0.0 && rel.pib_mundial_diario.is_finite(),
                "PIB diário inválido no dia {dia}: {}",
                rel.pib_mundial_diario
            );
        }
    }

    // Data exata após 10 anos de calendário gregoriano
    assert_eq!(mundo.data_atual(), DataJogo::nova(1955, 1, 1).unwrap());
    assert_eq!(mundo.dias_decorridos(), 3_652);

    let pop_final = mundo.populacao_total_pais("belgica").unwrap();
    let sol_final = mundo.padrao_de_vida_medio_pais("belgica").unwrap();
    let alfa_final = mundo.alfabetizacao_media_pais("belgica").unwrap();
    let niveis_finais: u32 = mundo
        .provincias()
        .iter()
        .flat_map(|p| &p.predios)
        .map(|b| b.nivel)
        .sum();

    // Crescimento demográfico plausível para uma década pós-1945 (sem colapso nem explosão)
    assert!(
        pop_final > pop_inicial * 1.02 && pop_final < pop_inicial * 1.30,
        "evolução demográfica de 10 anos fora da faixa histórica plausível: {pop_inicial:.0} -> {pop_final:.0}"
    );

    // Padrão de vida e alfabetização saudáveis e dentro da escala
    assert!(
        (7.0..=45.0).contains(&sol_final),
        "padrão de vida médio final implausível: {sol_final:.2}"
    );
    assert!(
        (0.50..=1.0).contains(&alfa_final) && alfa_final >= alfa_inicial - 0.05,
        "alfabetização final implausível: {alfa_inicial:.3} -> {alfa_final:.3}"
    );

    // Expansão industrial efetiva pela fila de construção estatal e privada
    assert!(
        total_obras_decada >= 2 && niveis_finais > niveis_iniciais,
        "economia deve concluir construções ao longo de 10 anos (obras={total_obras_decada}, níveis {niveis_iniciais}->{niveis_finais})"
    );

    // Verificação explícita de todos os preços do mercado ao fim de 10 anos
    let m = mundo.obter_mercado("mercado_belga").unwrap();
    for bem in &mundo.catalogo().bens {
        let p = mundo.preco_atual(&m.id, &bem.id).unwrap();
        assert!(
            p >= bem.preco_base * 0.25 - 1e-6 && p <= bem.preco_base * 1.75 + 1e-6,
            "preço do bem '{}' explodiu ou colapsou: {p} (base {})",
            bem.id,
            bem.preco_base
        );
    }
}

#[test]
fn teste_integracao_10_anos_multinacional_comercio_e_formula_classica() {
    let mut cenario = carregar_fixtures();
    // Exercita também a fórmula clássica direta e a cadência híbrida semanal ao longo de 10 anos
    cenario.config.formula_preco = FormulaPreco::Classica;
    cenario.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;

    let mut mundo = MundoEconomico::novo(cenario).expect("cenário multinacional deve carregar");

    // 521 semanas + 5 dias = 3.652 dias (10 anos: 01/01/1945 -> 01/01/1955)
    for semana in 1..=521 {
        let rel = mundo.executar_tick_semanal();
        assert!(rel.pib_mundial_diario.is_finite() && rel.pib_mundial_diario > 0.0);
        if semana % 26 == 0 {
            mundo
                .auditar_estabilidade()
                .expect("invariantes semestrais devem permanecer íntegras");
        }
    }
    mundo.avancar_dias(5);
    assert_eq!(mundo.data_atual(), DataJogo::nova(1955, 1, 1).unwrap());
    mundo.auditar_estabilidade().unwrap();

    for pais_id in ["belgica", "estados_unidos", "uniao_sovietica", "brasil"] {
        let pop = mundo.populacao_total_pais(pais_id).unwrap();
        let sol = mundo.padrao_de_vida_medio_pais(pais_id).unwrap();
        let pais = mundo.obter_pais(pais_id).unwrap();
        assert!(
            pop > 50_000.0,
            "população de '{pais_id}' deve permanecer positiva ({pop})"
        );
        assert!(
            (5.0..=55.0).contains(&sol),
            "SoL de '{pais_id}' fora de faixa ({sol})"
        );
        assert!(
            pais.pib_diario > 0.0,
            "PIB diário de '{pais_id}' deve ser positivo"
        );
    }

    // Garante que o estado final após 10 anos serializa e recarrega em RON perfeitamente
    let exportado = mundo.exportar_cenario();
    let ron_texto = exportado.para_ron_string().unwrap();
    let recarregado = CenarioEconomia::de_ron_str(&ron_texto).unwrap();
    let mundo_pos_load = MundoEconomico::novo(recarregado).unwrap();
    mundo_pos_load.auditar_estabilidade().unwrap();
}
