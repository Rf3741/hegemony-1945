//! Diagnóstico de crescimento macro: roda só economia + IA econômica (sem janela) e imprime, por ano,
//! PIB nominal, índice de preços, PIB real, população, emprego, níveis de prédios e investimento.
//! Uso: cargo run --example crescimento [-- ANOS]
use ia_paises::IaEconomia;
use sim_economia::{CenarioEconomia, ModoCadencia, MundoEconomico};
use std::path::Path;
use std::time::Instant;

const TAGS: [&str; 9] = ["USA", "SOV", "GBR", "FRA", "GER", "JAP", "CHI", "IND", "BRA"];

fn indice_precos(m: &MundoEconomico) -> f64 {
    let (mut s, mut n) = (0.0f64, 0.0f64);
    for mk in m.mercados() {
        for b in &m.catalogo().bens {
            if let Some(p) = m.preco_atual(&mk.id, &b.id) {
                s += (p / b.preco_base.max(1e-9)).ln();
                n += 1.0;
            }
        }
    }
    (s / n.max(1.0)).exp()
}

fn main() {
    let anos: u32 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(10);
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cen = CenarioEconomia::carregar_diretorio(&raiz.join("assets/data/economia")).unwrap();
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    let mut m = MundoEconomico::novo(cen).unwrap();
    let mut ia = IaEconomia::novo();
    for _ in 0..90 {
        m.executar_tick_diario();
    }
    let t0 = Instant::now();
    let (mut t_sim, mut t_ia) = (0.0f64, 0.0f64);
    let mut fases = [0.0f64; 7];
    let mut dias = 0u64;
    for ano in 0..=anos {
        if ano > 0 {
            for _ in 0..365 {
                let i0 = Instant::now();
                let (_, f) = m.executar_tick_diario_perfilado();
                for (a, b) in fases.iter_mut().zip([f.producao_ms, f.distribuicao_ms, f.consumo_ms, f.mercado_ms, f.demografia_ms, f.trabalho_ms, f.governo_ms]) { *a += b; }
                t_sim += i0.elapsed().as_secs_f64();
                let i1 = Instant::now();
                ia.passo_diario(&mut m, &[]);
                t_ia += i1.elapsed().as_secs_f64();
                dias += 1;
            }
        }
        let pib: f64 = m.paises().iter().map(|p| p.pib_diario).sum::<f64>() * 365.0;
        let pop: f64 = m.provincias().iter().map(|p| p.populacao_total()).sum();
        let (mut emp, mut ativos, mut niveis, mut vagas) = (0.0, 0.0, 0u64, 0.0);
        for pr in m.provincias() {
            for pp in &pr.pops {
                emp += pp.empregados;
                ativos += pp.populacao;
            }
            for b in &pr.predios {
                niveis += b.nivel as u64;
                vagas += b.total_vagas();
            }
        }
        let fundo: f64 = m.paises().iter().map(|p| p.fundo_investimento_privado).sum();
        let aporte: f64 = m.paises().iter().map(|p| p.aporte_fundo_privado_ultimo_tick).sum();
        let tes: f64 = m.paises().iter().map(|p| p.orcamento.tesouro).sum();
        let div: f64 = m.paises().iter().map(|p| p.orcamento.divida).sum();
        let fila: usize = m.paises().iter().map(|p| p.fila_construcao.len()).sum();
        let ip = indice_precos(&m);
        print!(
            "A{ano} pib={pib:.3e} ip={ip:.3} real={:.3e} pop={pop:.3e} emp/pop={:.3} niveis={niveis} vagas={vagas:.3e} fundo={fundo:.2e} aporte_d={aporte:.2e} tes={tes:.2e} div={div:.2e} fila={fila} |",
            pib / ip,
            emp / ativos
        );
        for t in TAGS {
            if let Some(p) = m.paises().iter().find(|p| p.tag.as_deref() == Some(t) || p.id == t) {
                print!(" {t}={:.2e}", p.pib_diario * 365.0);
            }
        }
        println!();
        for t in TAGS {
            if let Some(p) = m.paises().iter().find(|p| p.tag.as_deref() == Some(t) || p.id == t) {
                let (mut cons, mut tot) = (0u64, 0u64);
                for pr in m.provincias().iter().filter(|pr| pr.pais_id == p.id) {
                    for b in &pr.predios {
                        tot += b.nivel as u64;
                        if m.catalogo().tipos_predio.iter().any(|tp| tp.id == b.tipo_predio_id && tp.categoria == sim_economia::CategoriaPredio::Construcao) {
                            cons += b.nivel as u64;
                        }
                    }
                }
                if t == "USA" && ano == 2 {
                    let mut c = std::collections::BTreeMap::new();
                    for i in &p.fila_construcao { *c.entry(i.tipo_predio_id.clone()).or_insert(0) += 1; }
                    println!("   fila USA: {c:?}");
                }
                if (t == "GER" || t == "JAP") && ano == anos {
                    let mut ativos = 0.0; let mut emp = 0.0; let mut sub = 0.0; let mut neg = 0.0; let mut tot = 0.0;
                    for pr in m.provincias().iter().filter(|pr| pr.pais_id == p.id) {
                        for pp in &pr.pops { ativos += pp.populacao; emp += pp.empregados; if pp.padrao_de_vida < 5.0 { sub += pp.populacao; } }
                        for b in &pr.predios { tot += b.nivel as f64; if b.lucro_ultimo_tick < 0.0 { neg += b.nivel as f64; } }
                    }
                    let mk = m.obter_mercado(&p.mercado_id).unwrap();
                    let mut pr_rel: Vec<(String, f64)> = mk.precos_atuais.iter().map(|(g, v)| (g.clone(), v / m.catalogo().bens.iter().find(|b| &b.id == g).map_or(1.0, |b| b.preco_base))).collect();
                    pr_rel.sort_by(|a, b| a.1.total_cmp(&b.1));
                    println!("   diag {t}: pop={ativos:.3e} emp={emp:.3e} pobres={sub:.2e} niveis={tot:.0} prejuizo={neg:.0} baratos={:?} caros={:?}", &pr_rel[..4], &pr_rel[pr_rel.len()-3..]);
                }
                if ano == 0 || ano == anos {
                    let (mut sp, mut ap, mut w, mut r) = (0.0, 0.0, 0.0, 0.0);
                    for pr in m.provincias().iter().filter(|pr| pr.pais_id == p.id) {
                        for pp in &pr.pops { sp += pp.padrao_de_vida * pp.populacao; ap += pp.alfabetizacao * pp.populacao; w += pp.populacao; r += sim_economia::taxa_crescimento_anual_pop(pp.padrao_de_vida, pp.alfabetizacao) * pp.populacao; }
                    }
                    println!("   demo {t}: sol={:.1} alfa={:.2} r_anual={:.4}", sp / w, ap / w, r / w);
                }
                let priv_n = p.fila_construcao.iter().filter(|i| i.privado).count();
                println!("   {t}: pts_aplic/dia={:.1} fila={} (priv {}) niveis={tot} construcao={cons} fundo={:.2e} tes={:.2e} div={:.2e} rec={:.2e}", p.pontos_aplicados_ultimo_tick(), p.fila_construcao.len(), priv_n, p.fundo_investimento_privado, p.orcamento.tesouro, p.orcamento.divida, p.orcamento.receita_total());
            }
        }
    }
    let d = dias.max(1) as f64;
    eprintln!("fases ms/dia [prod, distr, consumo, mercado, demog, trabalho, governo] = {:?}", fases.iter().map(|x| (x / d * 100.0).round() / 100.0).collect::<Vec<_>>());
    eprintln!("{:.2} ms/dia (sim {:.2} + ia {:.2})", t0.elapsed().as_secs_f64() * 1000.0 / d, t_sim * 1000.0 / d, t_ia * 1000.0 / d);
}
