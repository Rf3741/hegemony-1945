//! Cenário REAL de 1945 (assets/data/economia): carga, 5 anos de simulação com mercado global,
//! estabilidade numérica e tempo médio do tick.
use sim_economia::{CenarioEconomia, MercadoGlobal, MundoEconomico};
use std::path::Path;
use std::time::Instant;

fn carregar() -> (MundoEconomico, std::time::Duration) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
    let t = Instant::now();
    let cen = CenarioEconomia::carregar_diretorio(&dir).expect("cenário real deve carregar");
    let mundo = MundoEconomico::novo(cen).expect("mundo real deve ser válido");
    (mundo, t.elapsed())
}

#[test]
fn cinco_anos_cenario_real_com_mercado_global() {
    let (mut mundo, t_carga) = carregar();
    let mut global = MercadoGlobal::novo(&mundo);
    let m_usa = mundo.indice_mercado_do_pais("usa");
    global.acompanhar_mercado(m_usa);
    let pib0_usa = mundo.obter_pais("usa").unwrap().pib_diario;
    let _ = pib0_usa;

    // Jogador (EUA) enfileira uma obra pública para exercitar a fila
    let prov_usa = mundo.provincias().iter().find(|p| p.pais_id == "usa" && mundo.pode_construir(&p.id, "siderurgica")).map(|p| p.id.clone());
    let obra_enfileirada = if let Some(pid) = &prov_usa {
        mundo.enfileirar_construcao("usa", pid, "siderurgica", false).is_ok()
    } else {
        false
    };

    let dias = 1826u32; // 1945-1949 (5 anos, 1 bissexto)
    let t = Instant::now();
    let mut max_tick = std::time::Duration::ZERO;
    let mut obras = 0usize;
    for dia in 1..=dias {
        let t0 = Instant::now();
        let rel = mundo.executar_tick_diario();
        mundo.atualizar_mercado_global(&mut global);
        max_tick = max_tick.max(t0.elapsed());
        obras += rel.obras_concluidas;
        if dia % 60 == 0 || dia == dias {
            mundo.auditar_estabilidade().unwrap_or_else(|e| panic!("dia {dia}: {e}"));
            assert!(rel.pib_mundial_diario.is_finite() && rel.pib_mundial_diario > 0.0);
        }
    }
    let medio = t.elapsed() / dias;

    // Preços mundiais e nacionais em faixa razoável, sem NaN
    for (b, bem) in mundo.catalogo().bens.iter().enumerate() {
        let pm = global.preco_mundial[b];
        assert!(pm.is_finite() && pm >= bem.preco_base * 0.25 - 1e-6 && pm <= bem.preco_base * 1.75 + 1e-6, "preço mundial de {}: {pm}", bem.id);
        for m in mundo.mercados() {
            let p = mundo.preco_atual(&m.id, &bem.id).unwrap();
            assert!(p.is_finite() && p > 0.0 && p >= bem.preco_base * 0.25 - 1e-6 && p <= bem.preco_base * 1.75 + 1e-6, "{} / {}: {p}", m.id, bem.id);
        }
        assert_eq!(global.historico_mundial[b].len(), (dias as usize).min(global.config.historico_max_dias));
    }
    for id in ["usa", "sov"] {
        let p = mundo.obter_pais(id).unwrap();
        assert!(p.pib_diario > 0.0 && p.pib_diario.is_finite(), "PIB de {id}: {}", p.pib_diario);
        assert!(p.orcamento.tesouro.is_finite() && p.orcamento.divida.is_finite());
        eprintln!("{id}: PIB diário {:.0}, tesouro {:.0}, dívida {:.0}, SoL {:.1}", p.pib_diario, p.orcamento.tesouro, p.orcamento.divida, mundo.padrao_de_vida_medio_pais(id).unwrap());
    }
    eprintln!("carga+indexação: {t_carga:?}; tick médio (com mercado global): {medio:?}; máx: {max_tick:?}; obras concluídas: {obras}; obra enfileirada: {obra_enfileirada}");
    let (ouro, aco) = (mundo.indice_bem("aco").unwrap(), mundo.indice_bem("graos").unwrap());
    eprintln!("aço mundial {:.2} (var 30d {:+.1}%), grãos {:.2}", global.preco_mundial[ouro], 100.0 * global.variacao(ouro, 30, false), global.preco_mundial[aco]);
}

#[test]
fn obra_publica_consome_materiais_e_custa_ao_orcamento() {
    let (mut mundo, _) = carregar();
    let pid = mundo
        .provincias()
        .iter()
        .find(|p| p.pais_id == "usa" && mundo.pode_construir(&p.id, "siderurgica"))
        .map(|p| p.id.clone())
        .expect("EUA devem ter província para siderúrgica");
    mundo.enfileirar_construcao("usa", &pid, "siderurgica", false).unwrap();
    for _ in 0..3 {
        mundo.executar_tick_diario();
    }
    let usa = mundo.obter_pais("usa").unwrap();
    assert!(usa.pontos_aplicados_ultimo_tick > 0.0, "a obra deve receber pontos");
    assert!(usa.orcamento.gasto_obras_materiais > 0.0, "obra pública deve custar materiais ao orçamento");
    let est = mundo.custo_materiais_obra("usa", "siderurgica").unwrap();
    eprintln!("custo estimado da siderúrgica: {est:.0}; gasto/dia: {:.0}; PIB/dia {:.0}", usa.orcamento.gasto_obras_materiais, usa.pib_diario);
    assert!(est > 0.0);
}
