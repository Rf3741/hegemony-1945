//! Benchmark de desempenho do motor de guerra em escala mundial:
//! - 4.600 regiões (escala completa Natural Earth 10m Admin-1)
//! - 20 guerras simultâneas ativas
//! - Meta rigorosa de desempenho: execução de um tick diário em menos de 3,0 ms.

use sim_guerra::{
    CatalogoGuerra, MundoGuerra, ParametrosCenarioSintetico, gerar_cenario_sintetico,
};
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

fn main() {
    let dir_fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let catalogo = CatalogoGuerra::carregar_diretorio(&dir_fixtures)
        .expect("catálogo militar deve carregar sem erros");

    let params = ParametrosCenarioSintetico {
        num_regioes: 4_600,
        num_paises: 50,
        num_guerras: 20,
        divisoes_por_pais: 8,
        seed: 19450101,
    };

    println!("Inicializando cenário mundial de 4.600 regiões e 20 guerras simultâneas...");
    let cenario = gerar_cenario_sintetico(catalogo, params);

    let total_divisoes = cenario.divisoes.len();
    let total_regioes = cenario.regioes.len();
    let total_guerras = cenario.guerras.len();
    let total_paises = cenario.paises.len();

    let mut mundo =
        MundoGuerra::novo(cenario).expect("mundo de benchmark deve inicializar sem erros");

    let (aquecimento, amostras) = if cfg!(debug_assertions) {
        (3usize, 10usize)
    } else {
        (15usize, 100usize)
    };

    println!("Executando {aquecimento} ticks de aquecimento...");
    for _ in 0..aquecimento {
        black_box(mundo.executar_tick_diario());
    }

    println!("Medindo {amostras} ticks diários...");
    let mut tempos_ms = Vec::with_capacity(amostras);

    for _ in 0..amostras {
        let inicio = Instant::now();
        let relatorio = mundo.executar_tick_diario();
        black_box(relatorio);
        tempos_ms.push(inicio.elapsed().as_secs_f64() * 1_000.0);
    }

    tempos_ms.sort_by(|a, b| a.total_cmp(b));
    let media_ms: f64 = tempos_ms.iter().sum::<f64>() / (tempos_ms.len() as f64);
    let p50_ms = tempos_ms[tempos_ms.len() / 2];
    let p95_ms = tempos_ms[(tempos_ms.len() * 95 / 100).min(tempos_ms.len() - 1)];
    let min_ms = tempos_ms[0];
    let max_ms = *tempos_ms.last().unwrap_or(&0.0);

    println!("\n=== Benchmark sim_guerra (Escala Global 1945) ===");
    println!(
        "Escala: {} regiões | {} países | {} divisões | {} guerras ativas",
        total_regioes, total_paises, total_divisoes, total_guerras
    );
    println!(
        "Tick diário ({amostras} amostras): média = {media_ms:.3} ms | p50 = {p50_ms:.3} ms | p95 = {p95_ms:.3} ms | min = {min_ms:.3} ms | max = {max_ms:.3} ms"
    );

    if !cfg!(debug_assertions) {
        assert!(
            media_ms < 3.0,
            "Meta de desempenho violada: média de {media_ms:.3} ms excedeu o teto de 3,0 ms"
        );
        println!("SUCESSO: Meta de < 3,0 ms atingida com folga ({media_ms:.3} ms)!");
    } else {
        println!(
            "Nota: Executado em modo debug. Para validação de meta oficial (< 3,0 ms), execute em modo --release."
        );
    }
}
