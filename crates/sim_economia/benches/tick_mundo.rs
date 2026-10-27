//! Benchmark simples de desempenho do tick diário em escala mundial
//! (>3.000 províncias e 150 países, meta: < 5,0 ms por tick em modo `--release`).

use sim_economia::{
    CatalogoEconomia, MundoEconomico, ParametrosEscalaMundial, TemposFasesTick,
    gerar_cenario_escala_mundial,
};
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

fn main() {
    let dir_fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let catalogo = CatalogoEconomia::carregar_diretorio(&dir_fixtures)
        .expect("catálogo de fixtures deve carregar sem erros");

    let params = ParametrosEscalaMundial {
        num_paises: 150,
        num_provincias: 3_200,
        num_mercados: 60,
        num_rotas_comerciais: 180,
    };

    let cenario = gerar_cenario_escala_mundial(catalogo, params);
    let total_predios: usize = cenario.provincias.iter().map(|p| p.predios.len()).sum();
    let total_pops: usize = cenario.provincias.iter().map(|p| p.pops.len()).sum();

    let mut mundo =
        MundoEconomico::novo(cenario).expect("mundo de benchmark deve inicializar sem erros");

    let (aquecimento, amostras) = if cfg!(debug_assertions) {
        (3usize, 10usize)
    } else {
        (30usize, 200usize)
    };

    for _ in 0..aquecimento {
        black_box(mundo.executar_tick_diario());
    }

    let mut tempos_ms = Vec::with_capacity(amostras);
    let mut soma_fases = TemposFasesTick::default();
    for _ in 0..amostras {
        let inicio = Instant::now();
        let (rel, fases) = mundo.executar_tick_diario_perfilado();
        black_box(rel);
        tempos_ms.push(inicio.elapsed().as_secs_f64() * 1_000.0);
        soma_fases.producao_ms += fases.producao_ms;
        soma_fases.distribuicao_ms += fases.distribuicao_ms;
        soma_fases.consumo_ms += fases.consumo_ms;
        soma_fases.mercado_ms += fases.mercado_ms;
        soma_fases.demografia_ms += fases.demografia_ms;
        soma_fases.trabalho_ms += fases.trabalho_ms;
        soma_fases.governo_ms += fases.governo_ms;
    }

    mundo
        .auditar_estabilidade()
        .expect("mundo de 3.200 províncias deve manter invariantes estáveis");

    tempos_ms.sort_by(|a, b| a.total_cmp(b));
    let media_ms: f64 = tempos_ms.iter().sum::<f64>() / (tempos_ms.len() as f64);
    let p50_ms = tempos_ms[tempos_ms.len() / 2];
    let p95_ms = tempos_ms[(tempos_ms.len() * 95 / 100).min(tempos_ms.len() - 1)];
    let min_ms = tempos_ms[0];
    let max_ms = *tempos_ms.last().unwrap_or(&0.0);

    println!("=== Benchmark sim_economia (Escala Mundial 1945) ===");
    println!(
        "Escala: {} países | {} estados | {} províncias | {} mercados | {} prédios | {} grupos de pop",
        mundo.paises().len(),
        mundo.estados().len(),
        mundo.provincias().len(),
        mundo.mercados().len(),
        total_predios,
        total_pops,
    );
    println!(
        "Tick diário ({amostras} amostras): média = {media_ms:.3} ms | p50 = {p50_ms:.3} ms | p95 = {p95_ms:.3} ms | min = {min_ms:.3} ms | max = {max_ms:.3} ms"
    );
    let n = amostras as f64;
    println!(
        "Fases médias: prod={:.3}ms | dist={:.3}ms | cons={:.3}ms | merc={:.3}ms | demo={:.3}ms | trab={:.3}ms | gov={:.3}ms",
        soma_fases.producao_ms / n,
        soma_fases.distribuicao_ms / n,
        soma_fases.consumo_ms / n,
        soma_fases.mercado_ms / n,
        soma_fases.demografia_ms / n,
        soma_fases.trabalho_ms / n,
        soma_fases.governo_ms / n,
    );

    if !cfg!(debug_assertions) {
        assert!(
            media_ms < 5.0,
            "meta de desempenho violada: média de {media_ms:.3} ms excedeu o limite de 5,0 ms"
        );
    }
}
