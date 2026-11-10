//! Binário para redistribuir população e prédios em `provincias.ron`.
//!
//! Uso:
//!   cargo run -p sim_economia --bin redistribuir_provincias -- [--input <dir>] [--output <dir>] [--dry-run]
//!
//! Por padrão, carrega o arquivo ORIGINAL de c8a2db35 (via git show) e salva no workspace atual.

use sim_economia::{CenarioEconomia, redistribuir_provincias, cenario::escrever_provincias_compacto};
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut output_dir = PathBuf::from("../../assets/data/economia");
    let mut dry_run = false;
    let mut use_original = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--output" => {
                if i + 1 < args.len() {
                    output_dir = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--dry-run" => dry_run = true,
            "--use-current" => use_original = false,
            _ => {}
        }
        i += 1;
    }

    println!("=== REDISTRIBUIÇÃO DE PROVÍNCIAS ===");
    println!("Output: {}", output_dir.display());
    println!("Dry-run: {}", dry_run);
    println!("Use original (c8a2db35): {}", use_original);

    // Carregar cenário: se use_original, usar git show para pegar c8a2db35
    let mut cen = if use_original {
        carregar_original_c8a2db35().expect("falha ao carregar cenário original c8a2db35")
    } else {
        CenarioEconomia::carregar_diretorio(&output_dir).expect("falha ao carregar cenário atual")
    };

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

fn carregar_original_c8a2db35() -> Result<CenarioEconomia, Box<dyn std::error::Error>> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let repo_root = std::path::Path::new(&manifest_dir).join("../..");
    
    // Usar e64bbb4b que tem o balanço correto (o commit e8cabeb6 quebrou)
    let commit = "e64bbb4b";
    
    // Lista de arquivos necessários (existem em e64bbb4b)
    let files = [
        "bens.ron", "canais.ron", "ciclos_demanda.ron", "config_simulacao.ron",
        "estados.ron", "leis_economicas.ron", "mercados.ron", "metodos_producao.ron",
        "paises.ron", "potenciais.ron", "provincias.ron", "rotas_comerciais.ron",
        "tipos_pop.ron", "tipos_predio.ron"
    ];

    let temp_dir = std::env::temp_dir().join("hegemony_orig_e64bbb4b");
    std::fs::create_dir_all(&temp_dir)?;

    for file in files {
        let output = Command::new("git")
            .args(["show", &format!("{}:assets/data/economia/{}", commit, file)])
            .current_dir(&repo_root)
            .output()?;
        if !output.status.success() {
            return Err(format!("git show falhou para {}: {}", file, String::from_utf8_lossy(&output.stderr)).into());
        }
        std::fs::write(temp_dir.join(file), output.stdout)?;
    }

    CenarioEconomia::carregar_diretorio(&temp_dir).map_err(Into::into)
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