//! Teste de regressão de desempenho: captura valores de referência (PIB, tesouro, população)
//! após 365 dias e verifica que otimizações não alteram o resultado numérico.

use sim_economia::{CenarioEconomia, MercadoGlobal, ModoCadencia, MundoEconomico};
use std::path::Path;

fn carregar_fixtures() -> CenarioEconomia {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    CenarioEconomia::carregar_diretorio(&dir).expect("fixtures RON devem carregar sem erros")
}

#[test]
#[ignore = "benchmark de regressão — rodar explicitamente com --ignored"]
fn benchmark_regressao_365_dias_valores_referencia() {
    let mut cenario = carregar_fixtures();
    cenario.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;

    let mut mundo = MundoEconomico::novo(cenario).expect("mundo deve inicializar");
    let mut global = MercadoGlobal::novo(&mundo);
    let m_usa = mundo.indice_mercado_do_pais("usa");
    global.acompanhar_mercado(m_usa);

    // Aquecimento (90 dias)
    for _ in 0..90 {
        mundo.executar_tick_diario();
        mundo.atualizar_mercado_global(&mut global);
    }

    // Simular 365 dias
    for _ in 0..365 {
        mundo.executar_tick_diario();
        mundo.atualizar_mercado_global(&mut global);
    }

    // Valores de referência capturados após otimização (atualize estes se mudar regras econômicas)
    // EUA (id: estados_unidos)
    let usa = mundo.obter_pais("estados_unidos").expect("EUA devem existir");
    let pop_usa = mundo.populacao_total_pais("estados_unidos").expect("população EUA");
    // Valores de referência (debug build, fixtures): PIB/dia=1159581, Tesouro=50196747, Pop=168884
    assert!((usa.pib_diario - 1_159_581.0).abs() < 100.0, "PIB diário EUA divergiu: {}", usa.pib_diario);
    assert!((usa.orcamento.tesouro - 50_196_747.0).abs() < 100.0, "Tesouro EUA divergiu: {}", usa.orcamento.tesouro);
    assert!((pop_usa - 168_884.0).abs() < 100.0, "População EUA divergiu: {}", pop_usa);

    // URSS (id: uniao_sovietica)
    let sov = mundo.obter_pais("uniao_sovietica").expect("URSS deve existir");
    let pop_sov = mundo.populacao_total_pais("uniao_sovietica").expect("população URSS");
    // Valores de referência: PIB/dia=267533, Tesouro=32288781, Pop=171204
    assert!((sov.pib_diario - 267_533.0).abs() < 100.0, "PIB diário URSS divergiu: {}", sov.pib_diario);
    assert!((sov.orcamento.tesouro - 32_288_781.0).abs() < 100.0, "Tesouro URSS divergiu: {}", sov.orcamento.tesouro);
    assert!((pop_sov - 171_204.0).abs() < 100.0, "População URSS divergiu: {}", pop_sov);

    // Bélgica
    let bel = mundo.obter_pais("belgica").expect("Bélgica deve existir");
    // Valores de referência: PIB/dia=655553, Tesouro=33985342
    assert!((bel.pib_diario - 655_553.0).abs() < 100.0, "PIB diário Bélgica divergiu: {}", bel.pib_diario);
    assert!((bel.orcamento.tesouro - 33_985_342.0).abs() < 100.0, "Tesouro Bélgica divergiu: {}", bel.orcamento.tesouro);

    // Brasil
    let bra = mundo.obter_pais("brasil").expect("Brasil deve existir");
    let pop_bra = mundo.populacao_total_pais("brasil").expect("população Brasil");
    // Valores de referência: PIB/dia=553096, Tesouro=23575697, Pop=155820
    assert!((bra.pib_diario - 553_096.0).abs() < 100.0, "PIB diário Brasil divergiu: {}", bra.pib_diario);
    assert!((bra.orcamento.tesouro - 23_575_697.0).abs() < 100.0, "Tesouro Brasil divergiu: {}", bra.orcamento.tesouro);
    assert!((pop_bra - 155_820.0).abs() < 100.0, "População Brasil divergiu: {}", pop_bra);

    // Mundo
    let pib_mundial = mundo.paises().iter().map(|p| p.pib_diario).sum::<f64>();
    let pop_mundial = mundo.provincias().iter().map(|p| p.populacao_total()).sum::<f64>();
    // Valores de referência: PIB/dia=2635762, Pop=678199
    assert!((pib_mundial - 2_635_762.0).abs() < 100.0, "PIB mundial divergiu: {}", pib_mundial);
    assert!((pop_mundial - 678_199.0).abs() < 100.0, "População mundial divergiu: {}", pop_mundial);

    println!("✅ Valores de referência conferidos após 365 dias");
    println!("EUA: PIB/dia={:.0}, Tesouro={:.0}, Pop={:.0}", usa.pib_diario, usa.orcamento.tesouro, pop_usa);
    println!("URSS: PIB/dia={:.0}, Tesouro={:.0}, Pop={:.0}", sov.pib_diario, sov.orcamento.tesouro, pop_sov);
    println!("Brasil: PIB/dia={:.0}, Tesouro={:.0}, Pop={:.0}", bra.pib_diario, bra.orcamento.tesouro, pop_bra);
    println!("Mundial: PIB/dia={:.0}, Pop={:.0}", pib_mundial, pop_mundial);
}

#[test]
fn benchmark_desempenho_tick_diario() {
    let mut cenario = carregar_fixtures();
    cenario.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;

    let mut mundo = MundoEconomico::novo(cenario).expect("mundo deve inicializar");
    let mut global = MercadoGlobal::novo(&mundo);
    let m_usa = mundo.indice_mercado_do_pais("usa");
    global.acompanhar_mercado(m_usa);

    // Aquecimento
    for _ in 0..30 {
        mundo.executar_tick_diario();
    }

    use std::time::Instant;
    let amostras = 100;
    let mut tempos = Vec::with_capacity(amostras);

    for _ in 0..amostras {
        let inicio = Instant::now();
        mundo.executar_tick_diario();
        mundo.atualizar_mercado_global(&mut global);
        tempos.push(inicio.elapsed().as_secs_f64() * 1000.0);
    }

    tempos.sort_by(|a, b| a.total_cmp(b));
    let media: f64 = tempos.iter().sum::<f64>() / amostras as f64;
    let p50 = tempos[amostras / 2];
    let p95 = tempos[amostras * 95 / 100];

    println!("=== Benchmark Tick Diário (100 amostras, release) ===");
    println!("Média: {:.2} ms | p50: {:.2} ms | p95: {:.2} ms", media, p50, p95);

    // Meta: < 5 ms por tick em release
    // Descomente para fazer o teste falhar se exceder:
    // assert!(media < 5.0, "Média {:.2} ms excede meta de 5 ms", media);
}