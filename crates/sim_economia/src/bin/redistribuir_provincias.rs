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
            let mut total_niveis_industria = 0u64;
            let mut total_niveis_servicos = 0u64;
            let mut total_niveis_governo = 0u64;
            let mut total_vagas_est = 0.0;
            for prov in &provincias {
                let pop_total: f64 = prov.pops.iter().map(|p| p.populacao).sum();
                pop_total_usa += pop_total;
                
                let mut niveis_ind = 0u64;
                let mut niveis_serv = 0u64;
                let mut niveis_gov = 0u64;
                let mut vagas_prov = 0.0;
                for b in &prov.predios {
                    if b.tipo_predio_id.contains("fabrica") || b.tipo_predio_id.contains("usina") || b.tipo_predio_id.contains("siderurgica") || b.tipo_predio_id.contains("industria") || b.tipo_predio_id == "serraria" || b.tipo_predio_id == "estaleiro_naval" {
                        niveis_ind += b.nivel as u64;
                    }
                    if b.tipo_predio_id.contains("setor_") || b.tipo_predio_id.contains("porto_comercial") {
                        niveis_serv += b.nivel as u64;
                    }
                    if b.tipo_predio_id.contains("administracao") || b.tipo_predio_id.contains("quartel") || b.tipo_predio_id.contains("militar") || b.tipo_predio_id.contains("governo") || b.tipo_predio_id == "complexo_militar_quartel" {
                        niveis_gov += b.nivel as u64;
                    }
                    // Estimar vagas (usar método público total_vagas se cache compilado, senão estimar)
                    let vn = if b.tipo_predio_id.starts_with("fabrica_") || b.tipo_predio_id.contains("industria") || b.tipo_predio_id.contains("usina") || b.tipo_predio_id.contains("siderurgica") || b.tipo_predio_id == "serraria" || b.tipo_predio_id == "estaleiro_naval" {
                        800.0
                    } else if b.tipo_predio_id.starts_with("setor_") || b.tipo_predio_id.contains("porto_comercial") {
                        300.0
                    } else if b.tipo_predio_id.starts_with("mina_") || b.tipo_predio_id.starts_with("poco_") || b.tipo_predio_id.starts_with("campo_") || b.tipo_predio_id == "porto_pesca" {
                        200.0
                    } else if b.tipo_predio_id.starts_with("fazenda_") || b.tipo_predio_id.starts_with("plantacao_") || b.tipo_predio_id.starts_with("rancho_") || b.tipo_predio_id == "pecuaria_pesca" {
                        100.0
                    } else {
                        500.0
                    };
                    vagas_prov += vn * b.nivel as f64;
                }
                total_niveis_industria += niveis_ind;
                total_niveis_servicos += niveis_serv;
                total_niveis_governo += niveis_gov;
                total_vagas_est += vagas_prov;
                
                if niveis_ind > 0 || niveis_serv > 0 || niveis_gov > 0 || pop_total > 1000.0 {
                    println!("  {}: pop={:.0} ind_niv={} serv_niv={} gov_niv={} vagas_est={:.0}", prov.nome, pop_total, niveis_ind, niveis_serv, niveis_gov, vagas_prov);
                }
            }
            println!("  TOTAL USA: pop={:.0} ({:.1}M) ind_niv={} serv_niv={} gov_niv={} vagas_est={:.0}", pop_total_usa, pop_total_usa / 1e6, total_niveis_industria, total_niveis_servicos, total_niveis_governo, total_vagas_est);
        }
    }
}