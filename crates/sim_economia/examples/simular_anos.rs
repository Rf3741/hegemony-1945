//! Binário de simulação econômica headless de N anos (1945–1960+).
//! Gera relatório detalhado e CSV das 10 maiores economias e bens-chave.
//!
//! Uso:
//!   cargo run -p sim_economia --example simular_anos -- [--anos <N>] [--csv <caminho.csv>] [--csv-precos <caminho.csv>]

use sim_economia::{CenarioEconomia, MercadoGlobal, ModoCadencia, MundoEconomico};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

const BENS_CHAVE: &[&str] = &[
    "graos",
    "carne",
    "carvao",
    "petroleo",
    "combustiveis_refinados",
    "eletricidade",
    "minerio_ferro",
    "aco",
    "material_construcao",
    "ferramentas",
    "maquinas",
    "alimentos_processados",
    "roupas",
    "servicos",
    "transporte",
];

struct SnapshotPais {
    id: String,
    nome: String,
    pib_anual: f64,
    crescimento_ano_pct: f64,
    crescimento_acum_pct: f64,
    taxa_desemprego_pct: f64,
    sol: f64,
    divida: f64,
    tesouro: f64,
    divida_pib_pct: f64,
}

fn calcular_desemprego_pais(mundo: &MundoEconomico, pais_id: &str) -> f64 {
    mundo.taxa_desemprego_media_pais(pais_id).unwrap_or(0.0) * 100.0
}

fn coletar_top_paises(
    mundo: &MundoEconomico,
    top_n: usize,
    pib_base: &HashMap<String, f64>,
    pib_ano_anterior: &HashMap<String, f64>,
) -> Vec<SnapshotPais> {
    let mut lista: Vec<SnapshotPais> = mundo
        .paises()
        .iter()
        .map(|p| {
            let pib_anual = p.pib_diario * 365.0;
            let pib0 = pib_base.get(&p.id).copied().unwrap_or(pib_anual);
            let pib_ant = pib_ano_anterior.get(&p.id).copied().unwrap_or(pib_anual);

            let cresc_ano = if pib_ant > 1.0 {
                (pib_anual - pib_ant) / pib_ant * 100.0
            } else {
                0.0
            };
            let cresc_acum = if pib0 > 1.0 {
                (pib_anual - pib0) / pib0 * 100.0
            } else {
                0.0
            };

            let desemprego = calcular_desemprego_pais(mundo, &p.id);
            let sol = mundo.padrao_de_vida_medio_pais(&p.id).unwrap_or(10.0);
            let divida = p.orcamento.divida;
            let tesouro = p.orcamento.tesouro;
            let divida_pib = if pib_anual > 1.0 {
                divida / pib_anual * 100.0
            } else {
                0.0
            };

            SnapshotPais {
                id: p.id.clone(),
                nome: p.nome.clone(),
                pib_anual,
                crescimento_ano_pct: cresc_ano,
                crescimento_acum_pct: cresc_acum,
                taxa_desemprego_pct: desemprego,
                sol,
                divida,
                tesouro,
                divida_pib_pct: divida_pib,
            }
        })
        .collect();

    lista.sort_by(|a, b| b.pib_anual.total_cmp(&a.pib_anual));
    lista.truncate(top_n);
    lista
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut anos: u32 = 15; // Padrão: 1945 a 1960
    let mut caminho_csv = PathBuf::from("relatorio_economias_1945_1960.csv");
    let mut caminho_csv_precos = PathBuf::from("relatorio_precos_1945_1960.csv");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--anos" => {
                if i + 1 < args.len() {
                    anos = args[i + 1].parse().unwrap_or(15);
                    i += 1;
                }
            }
            "--csv" => {
                if i + 1 < args.len() {
                    caminho_csv = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            "--csv-precos" => {
                if i + 1 < args.len() {
                    caminho_csv_precos = PathBuf::from(&args[i + 1]);
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!("===============================================================================");
    println!(" SIMULAÇÃO ECONÔMICA HISTÓRICA (GUERRA FRIA) — {} ANOS (1945+)", anos);
    println!(" CSV Economias: {:?}", caminho_csv);
    println!(" CSV Preços:    {:?}", caminho_csv_precos);
    println!("===============================================================================");

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
    let t_carga = Instant::now();
    let mut cen = CenarioEconomia::carregar_diretorio(&dir).expect("carregar cenário");
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    let dur_carga = t_carga.elapsed();

    let mut mundo = MundoEconomico::novo(cen).expect("criar mundo econômico");
    println!("Cenário carregado em {:?}: {} províncias, {} países, {} mercados",
        dur_carga, mundo.provincias().len(), mundo.paises().len(), mundo.mercados().len());

    let mut global = MercadoGlobal::novo(&mundo);
    let m_usa = mundo.indice_mercado_do_pais("usa");
    global.acompanhar_mercado(m_usa);

    // Aquecimento inicial de 90 dias
    print!("Aquecimento (90 dias)... ");
    std::io::stdout().flush().ok();
    let t_aquece = Instant::now();
    for _ in 0..90 {
        mundo.executar_tick_diario();
        mundo.atualizar_mercado_global(&mut global);
    }
    println!("concluído em {:?}", t_aquece.elapsed());

    // Coleta baseline inicial (Ano 0 / 1945 pós-aquecimento)
    let mut pib_base: HashMap<String, f64> = HashMap::new();
    let mut pib_ano_anterior: HashMap<String, f64> = HashMap::new();
    for p in mundo.paises() {
        let pib = p.pib_diario * 365.0;
        pib_base.insert(p.id.clone(), pib);
        pib_ano_anterior.insert(p.id.clone(), pib);
    }

    let mut f_csv = File::create(&caminho_csv).expect("criar arquivo CSV de economias");
    writeln!(
        f_csv,
        "ano,ano_calendario,data,ranking,pais_id,pais_nome,pib_anual_usd,crescimento_ano_pct,crescimento_acum_pct,desemprego_pct,padrao_vida,divida_usd,tesouro_usd,divida_pib_pct"
    ).unwrap();

    let mut f_csv_p = File::create(&caminho_csv_precos).expect("criar arquivo CSV de preços");
    writeln!(
        f_csv_p,
        "ano,ano_calendario,data,bem_id,preco_mundial,preco_base,razao_base"
    ).unwrap();

    // Registro do Ano 0 (1945)
    let top0 = coletar_top_paises(&mundo, 10, &pib_base, &pib_ano_anterior);
    let data0 = mundo.data_atual();
    for (rk, s) in top0.iter().enumerate() {
        writeln!(
            f_csv,
            "0,{},{},{},{},\"{}\",{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}",
            data0.ano, data0, rk + 1, s.id, s.nome, s.pib_anual, s.crescimento_ano_pct, s.crescimento_acum_pct,
            s.taxa_desemprego_pct, s.sol, s.divida, s.tesouro, s.divida_pib_pct
        ).unwrap();
    }
    for &bem_id in BENS_CHAVE {
        if let Some(b_idx) = mundo.indice_bem(bem_id) {
            let pm = global.preco_mundial[b_idx];
            let pb = mundo.catalogo().bens[b_idx].preco_base;
            writeln!(f_csv_p, "0,{},{},{},{:.2},{:.2},{:.4}", data0.ano, data0, bem_id, pm, pb, pm / pb).unwrap();
        }
    }

    println!("\n--- BASELINE INICIAL (1945) TOP 10 ---");
    imprimir_tabela_top10(&top0, &data0.to_string());

    // Loop anual
    let t_sim_total = Instant::now();
    for ano_num in 1..=anos {
        let t_ano = Instant::now();
        // 365 dias por ano (ou 366 se bissexto)
        let data_antes = mundo.data_atual();
        let dias_no_ano = if sim_economia::tipos::DataJogo::eh_bissexto(data_antes.ano) { 366 } else { 365 };

        for _ in 0..dias_no_ano {
            mundo.executar_tick_diario();
            mundo.atualizar_mercado_global(&mut global);
        }

        let dur_ano = t_ano.elapsed();
        let data_atual = mundo.data_atual();

        let top_atuais = coletar_top_paises(&mundo, 10, &pib_base, &pib_ano_anterior);

        // Atualiza pib_ano_anterior para o próximo ano
        pib_ano_anterior.clear();
        for p in mundo.paises() {
            pib_ano_anterior.insert(p.id.clone(), p.pib_diario * 365.0);
        }

        // CSV economias
        for (rk, s) in top_atuais.iter().enumerate() {
            writeln!(
                f_csv,
                "{},{},{},{},{},\"{}\",{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}",
                ano_num, data_atual.ano, data_atual, rk + 1, s.id, s.nome, s.pib_anual, s.crescimento_ano_pct,
                s.crescimento_acum_pct, s.taxa_desemprego_pct, s.sol, s.divida, s.tesouro, s.divida_pib_pct
            ).unwrap();
        }

        // CSV preços
        for &bem_id in BENS_CHAVE {
            if let Some(b_idx) = mundo.indice_bem(bem_id) {
                let pm = global.preco_mundial[b_idx];
                let pb = mundo.catalogo().bens[b_idx].preco_base;
                writeln!(f_csv_p, "{},{},{},{},{:.2},{:.2},{:.4}", ano_num, data_atual.ano, data_atual, bem_id, pm, pb, pm / pb).unwrap();
            }
        }

        println!("\n>>> ANO {} / {} ({}) — Simulado em {:.2}s ({:.1} ms/dia)",
            ano_num, anos, data_atual, dur_ano.as_secs_f64(), dur_ano.as_secs_f64() * 1000.0 / dias_no_ano as f64);
        imprimir_tabela_top10(&top_atuais, &data_atual.to_string());
        imprimir_resumo_precos(&mundo, &global);
    }

    let dur_total = t_sim_total.elapsed();
    println!("\n===============================================================================");
    println!(" SIMULAÇÃO CONCLUÍDA EM {:?}", dur_total);
    println!(" Arquivos gerados:");
    println!("   - Economias: {}", caminho_csv.display());
    println!("   - Preços:    {}", caminho_csv_precos.display());
    println!("===============================================================================");
}

fn imprimir_tabela_top10(top: &[SnapshotPais], _data: &str) {
    println!("{:<4} {:<6} {:<24} {:>14} {:>10} {:>10} {:>8} {:>6} {:>14}",
        "RK", "ID", "PAÍS", "PIB (BI USD)", "CRESC/ANO", "CRESC ACUM", "DESEMP", "SoL", "DÍVIDA (BI)");
    println!("{:-<100}", "");
    for (rk, s) in top.iter().enumerate() {
        println!("{:<4} {:<6} {:<24} {:>14.2} {:>9.2}% {:>9.2}% {:>7.1}% {:>6.1} {:>14.2}",
            rk + 1,
            s.id,
            truncar(&s.nome, 24),
            s.pib_anual / 1e9,
            s.crescimento_ano_pct,
            s.crescimento_acum_pct,
            s.taxa_desemprego_pct,
            s.sol,
            s.divida / 1e9,
        );
    }
}

fn imprimir_resumo_precos(mundo: &MundoEconomico, global: &MercadoGlobal) {
    print!("Preços-chave (razão vs base): ");
    for &bem_id in &["graos", "carvao", "petroleo", "aco", "material_construcao", "maquinas"] {
        if let Some(b_idx) = mundo.indice_bem(bem_id) {
            let pm = global.preco_mundial[b_idx];
            let pb = mundo.catalogo().bens[b_idx].preco_base;
            print!("{}={:.2}x  ", bem_id, pm / pb);
        }
    }
    println!();
}

fn truncar(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let mut res: String = s.chars().take(max_len - 1).collect();
        res.push('…');
        res
    }
}
