//! Perfil do tick econômico por fase + impressão digital do estado (para checar determinismo).
//! Uso: cargo run -p sim_economia --example perfil_tick [-- DIAS]
use sim_economia::{CenarioEconomia, ModoCadencia, MundoEconomico};
use std::path::Path;
use std::time::Instant;

fn main() {
    let dias: u32 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(180);
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cen = CenarioEconomia::carregar_diretorio(&raiz.join("assets/data/economia")).unwrap();
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    let mut m = MundoEconomico::novo(cen).unwrap();
    for _ in 0..90 {
        m.executar_tick_diario();
    }
    let mut t = [0.0f64; 7];
    let ini = Instant::now();
    for _ in 0..dias {
        let (_, f) = m.executar_tick_diario_perfilado();
        for (a, b) in t.iter_mut().zip([f.producao_ms, f.distribuicao_ms, f.consumo_ms, f.mercado_ms, f.demografia_ms, f.trabalho_ms, f.governo_ms]) {
            *a += b;
        }
    }
    let total = ini.elapsed().as_secs_f64() * 1000.0 / dias as f64;
    let nomes = ["producao", "distribuicao", "consumo", "mercado", "demografia", "trabalho", "governo"];
    for (n, v) in nomes.iter().zip(t) {
        println!("{n:13} {:7.2} ms/dia", v / dias as f64);
    }
    println!("TOTAL         {total:7.2} ms/dia");
    // Impressão digital do estado (bits exatos dos f64) para comparar antes/depois.
    let mut h: u64 = 1469598103934665603;
    let mut mix = |x: f64| h = (h ^ x.to_bits()).wrapping_mul(1099511628211);
    for p in m.paises() {
        mix(p.orcamento.tesouro);
        mix(p.orcamento.divida);
        mix(p.pib_diario);
        mix(p.fundo_investimento_privado);
    }
    for pr in m.provincias() {
        for pop in &pr.pops {
            mix(pop.populacao);
            mix(pop.riqueza_reservas);
            mix(pop.padrao_de_vida);
        }
    }
    for mk in m.mercados() {
        for v in mk.precos_atuais.values() {
            mix(*v);
        }
    }
    println!("HASH {h:016x}");
}