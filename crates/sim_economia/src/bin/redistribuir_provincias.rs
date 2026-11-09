//! Binário para redistribuir população e prédios em `provincias.ron`.
//!
//! Uso:
//!   cargo run -p sim_economia --bin redistribuir_provincias -- [--input <dir>] [--output <dir>] [--dry-run]

use sim_economia::{CenarioEconomia, redistribuir_provincias, cenario::escrever_provincias_compacto};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_dir = PathBuf::from("../../assets/data/economia");
    let mut output_dir = PathBuf::from("../../assets/data/economia");
    let mut dry_run = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                if i + 1 < args.len() {
                    input_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--dry-run" => dry_run = true,
            _ => {}
        }
        i += 1;
    }

    println!("=== REDISTRIBUIÇÃO DE PROVÍNCIAS ===");
    println!("Input:  {}", input_dir.display());
    println!("Output: {}", output_dir.display());
    println!("Dry-run: {}", dry_run);

    // Carregar cenário atual
    let mut cen = CenarioEconomia::carregar_diretorio(&input_dir).expect("falha ao carregar cenário");

    // Executar redistribuição
    redistribuir_provincias(&mut cen);

    if !dry_run {
        // Salvar APENAS provincias.ron no formato compacto
        escrever_provincias_compacto(&output_dir.join("provincias.ron"), &cen.provincias)
            .expect("falha ao salvar provincias.ron");
        println!("\n✅ provincias.ron salvo em {}", output_dir.display());
    } else {
        println!("\n🔍 Dry-run: cenário NÃO salvo.");
    }

    // Relatório de verificação
    imprimir_relatorio_verificacao(&cen);
}

fn imprimir_relatorio_verificacao(cen: &CenarioEconomia) {
    println!("\n=== RELATÓRIO DE VERIFICAÇÃO ===");

    // Agrupar províncias por país
    let mut por_pais: std::collections::BTreeMap<String, Vec<&sim_economia::entidades::Provincia>> = std::collections::BTreeMap::new();
    for prov in &cen.provincias {
        por_pais.entry(prov.pais_id.clone()).or_default().push(prov);
    }

    for (pais_id, provincias) in por_pais {
        println!("\n--- {} ({} províncias) ---", pais_id.to_uppercase(), provincias.len());

        // Exemplos específicos para EUA
        if pais_id == "usa" {
            let mut pop_total_usa = 0.0;
            for prov in &provincias {
                let pop_total: f64 = prov.pops.iter().map(|p| p.populacao).sum();
                pop_total_usa += pop_total;
                println!("  {}: pop={:.0}", prov.nome, pop_total);
            }
            println!("  TOTAL USA: pop={:.0} ({:.1}M)", pop_total_usa, pop_total_usa / 1e6);
        }
    }
}