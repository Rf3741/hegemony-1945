//! Binário para redistribuir população e prédios em `provincias.ron`.
//!
//! Uso:
//!   cargo run -p sim_economia --bin redistribuir_provincias -- [--input <dir>] [--output <dir>] [--dry-run]

use sim_economia::{CenarioEconomia, redistribuir_provincias};
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
        cen.salvar_diretorio(&output_dir).expect("falha ao salvar cenário");
        println!("\n✅ Cenário salvo em {}", output_dir.display());
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
            for prov in &provincias {
                let pop_total: f64 = prov.pops.iter().map(|p| p.populacao).sum();
                match prov.nome.as_str() {
                    "Guam" => {
                        let tem_mina_ferro = prov.predios.iter().any(|b| b.tipo_predio_id == "mina_ferro");
                        println!("  Guam: pop={:.0} mina_ferro={}", pop_total, tem_mina_ferro);
                    }
                    "Minnesota" => {
                        let n_ferro: u32 = prov.predios.iter().filter(|b| b.tipo_predio_id == "mina_ferro").map(|b| b.nivel).sum();
                        println!("  Minnesota: mina_ferro={}", n_ferro);
                    }
                    "Iowa" => {
                        let n_ferro: u32 = prov.predios.iter().filter(|b| b.tipo_predio_id == "mina_ferro").map(|b| b.nivel).sum();
                        println!("  Iowa: mina_ferro={}", n_ferro);
                    }
                    "Texas" => {
                        let n_petroleo: u32 = prov.predios.iter().filter(|b| b.tipo_predio_id == "poco_petroleo").map(|b| b.nivel).sum();
                        println!("  Texas: poco_petroleo={}", n_petroleo);
                    }
                    "Nova Iorque" => {
                        let n_petroleo: u32 = prov.predios.iter().filter(|b| b.tipo_predio_id == "poco_petroleo").map(|b| b.nivel).sum();
                        println!("  Nova Iorque: poco_petroleo={}", n_petroleo);
                    }
                    _ => {}
                }
            }
        }
    }
}