//! Partida longa headless (1945-1965) para QA: roda economia, política, diplomacia, tecnologia e eventos
//! (legado incluso) com a IA ligada e procura pânicos, NaN, valores absurdos, explosão de estruturas e
//! vazamento de memória (VmRSS). Escreve o relatório em `docs/qa_partida_longa.md`.
//!
//! Uso: cargo run --example partida_longa [-- ANOS] [--sem-relatorio] [--saida=docs/arquivo.md]
use eventos::{CatalogoEventos, EstadoEventos, aplicar_opcao, avaliar_dia, escolha_ia};
use gf_mapa::CatalogoProvincias;
use ia_paises::IaEconomia;
use sim_diplomacia::{Bloco, Diplomacia, InfoPais};
use sim_economia::{CenarioEconomia, ModoCadencia, MundoEconomico};
use sim_politica::{CenarioPolitica, MundoPolitico};
use sim_tecnologia::{CatalogoTec, ConfigBlocos, EntradaPais, MundoTec};
use simulacao::{AlinhamentoGeopolitico, MundoJogo};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

const GRANDES: [&str; 8] = ["USA", "SOV", "GBR", "FRA", "CHI", "GER", "JAP", "ITA"];
const PAISES_TABELA: [&str; 9] = ["USA", "SOV", "GBR", "FRA", "GER", "JAP", "CHI", "IND", "BRA"];
const SOVIETICOS: [&str; 10] = ["SOV", "POL", "CZE", "HUN", "BUL", "ROM", "YUG", "ALB", "MON", "GDR"];

fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok())))
        .map_or(0.0, |kb| kb / 1024.0)
}

fn norm_diplo(t: &str) -> String {
    match t {
        "BGR" => "BUL".into(),
        "GRC" => "GRE".into(),
        _ => t.to_string(),
    }
}

#[derive(Default)]
struct Achados {
    linhas: Vec<String>,
    panicos: BTreeMap<String, u32>,
}
impl Achados {
    fn add(&mut self, data: &str, m: impl Into<String>) {
        let m = m.into();
        if self.linhas.len() < 400 && !self.linhas.iter().any(|l| l[l.find(' ').unwrap_or(0)..] == m[..]) {
            self.linhas.push(format!("{data} {m}"));
        }
    }
}

fn tem_nan(s: &str) -> bool {
    // Procura as palavras inteiras `NaN` e `inf` (não confundir com "influência" etc.).
    s.split(|c: char| !c.is_alphanumeric()).any(|w| w == "NaN" || w == "inf")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let anos: i32 = args.iter().find_map(|a| a.parse().ok()).unwrap_or(20);
    let escrever = !args.iter().any(|a| a == "--sem-relatorio");
    let raiz: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let t_carga = Instant::now();
    let rss0 = rss_mb();

    // Legado (províncias, donos, política antiga)
    let (_c, mut legado) = MundoJogo::carregar_de_arquivos(
        raiz.join("assets/map/provinces.ron"),
        raiz.join("data/scenario_1945/countries_1945.ron"),
        raiz.join("data/scenario_1945/province_owners_1945.ron"),
    )
    .expect("legado");
    let _: CatalogoProvincias = ron::from_str(&std::fs::read_to_string(raiz.join("assets/map/provinces.ron")).unwrap()).unwrap();
    // Economia
    let mut cen = CenarioEconomia::carregar_diretorio(&raiz.join("assets/data/economia")).expect("cenário economia");
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    let mut eco = MundoEconomico::novo(cen).expect("economia");
    let mut ia_eco = IaEconomia::novo();
    let id_por_tag: BTreeMap<String, String> = eco.paises().iter().filter_map(|p| p.tag.clone().map(|t| (t, p.id.clone()))).collect();
    for _ in 0..90 {
        eco.executar_tick_diario();
    }
    // Política
    let cenp = CenarioPolitica::carregar_diretorio(&raiz.join("assets/data/politica")).expect("cenário política");
    let mut pol = MundoPolitico::novo(cenp.catalogo, cenp.paises, 1945).expect("política");
    // Tecnologia
    let cat = CatalogoTec::carregar(&raiz.join("assets/data/tecnologia")).expect("tecnologia");
    let mut blocos = ConfigBlocos { grandes_potencias: GRANDES.iter().map(|s| s.to_string()).collect(), ..Default::default() };
    for tag in cat.iniciais.keys() {
        blocos.lider_do.insert(tag.clone(), if SOVIETICOS.contains(&tag.as_str()) { "SOV" } else { "USA" }.to_string());
    }
    let mut tec = MundoTec::novo(Arc::new(cat), blocos);
    let mut entradas: BTreeMap<String, EntradaPais> = BTreeMap::new();
    // Diplomacia
    let mut infos: Vec<InfoPais> = legado
        .paises
        .values()
        .map(|p| InfoPais {
            tag: norm_diplo(&p.tag),
            bloco: match p.alinhamento_inicial {
                AlinhamentoGeopolitico::BlocoOcidental => Bloco::Ocidental,
                AlinhamentoGeopolitico::BlocoSovietico => Bloco::Sovietico,
                _ => Bloco::NaoAlinhado,
            },
        })
        .collect();
    infos.sort_by(|a, b| a.tag.cmp(&b.tag));
    let mut dip = Diplomacia::novo(infos);
    // Eventos
    let cate = CatalogoEventos::carregar(&raiz.join("assets/data/eventos")).expect("eventos");
    let mut est = EstadoEventos::default();
    let jogador = "USA";

    let carga_s = t_carga.elapsed().as_secs_f64();
    eprintln!("carga: {carga_s:.1}s, RSS {:.0} MB", rss_mb());

    let mut ach = Achados::default();
    let (mut t_eco, mut t_pol, mut t_dip, mut t_tec, mut t_ev, mut t_leg) = (0.0f64, 0.0, 0.0, 0.0, 0.0, 0.0);
    let mut anuais: Vec<String> = Vec::new();
    let mut por_pais: Vec<String> = Vec::new();
    let mut pib_ant = 0.0f64;
    let mut max_estr = BTreeMap::<&str, usize>::new();
    let total_dias = anos as u64 * 365;
    let t_total = Instant::now();
    let mut dia_global = 0u64;
    let mut noticias_ia = 0usize;

    macro_rules! guarda {
        ($nome:expr, $data:expr, $corpo:expr) => {{
            if let Err(e) = catch_unwind(AssertUnwindSafe(|| $corpo)) {
                let msg = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                let n = ach.panicos.entry(format!("{}: {}", $nome, msg)).or_insert(0);
                *n += 1;
                if *n == 1 {
                    ach.add(&$data, format!("PÂNICO em {}: {}", $nome, msg));
                }
            }
        }};
    }

    while dia_global < total_dias {
        dia_global += 1;
        let d = legado.data;
        let ds = format!("{:04}-{:02}-{:02}", d.ano, d.mes, d.dia);

        let i = Instant::now();
        guarda!("economia", ds, {
            eco.executar_tick_diario();
            ia_eco.passo_diario(&mut eco, &[]);
        });
        t_eco += i.elapsed().as_secs_f64();

        let i = Instant::now();
        guarda!("legado", ds, {
            legado.data.avancar_dia();
            legado.ticks_executados += 1;
            legado.executar_tick_economia();
        });
        t_leg += i.elapsed().as_secs_f64();

        let i = Instant::now();
        guarda!("politica", ds, {
            pol.executar_tick_diario();
            let _ = pol.drenar_eventos_guerra_civil();
        });
        t_pol += i.elapsed().as_secs_f64();

        let i = Instant::now();
        guarda!("diplomacia", ds, {
            dip.tick_dia(legado.data.ano, legado.data.mes, legado.data.dia, Some(jogador));
        });
        t_dip += i.elapsed().as_secs_f64();

        let i = Instant::now();
        guarda!("tecnologia", ds, {
            if dia_global % 30 == 1 {
                let mut por_pais: BTreeMap<&str, (f64, f64, f64)> = BTreeMap::new();
                for pr in eco.provincias() {
                    let e = por_pais.entry(pr.pais_id.as_str()).or_default();
                    for b in &pr.predios {
                        if b.tipo_predio_id == "universidade_pesquisa" {
                            e.0 += b.nivel as f64;
                        }
                    }
                    for pop in &pr.pops {
                        e.1 += pop.populacao * pop.alfabetizacao;
                        e.2 += pop.populacao;
                    }
                }
                for p in eco.paises() {
                    if let (Some(tag), Some((u, pa, pp))) = (&p.tag, por_pais.get(p.id.as_str())) {
                        entradas.insert(tag.clone(), EntradaPais { niveis_universidade: *u, alfabetizacao: if *pp > 0.0 { pa / pp } else { 0.5 }, ..Default::default() });
                    }
                }
            }
            let ano = legado.data.ano;
            let tags: Vec<String> = tec.paises.keys().cloned().collect();
            for tag in tags {
                let e = entradas.get(&tag).copied().unwrap_or_default();
                tec.escolher_ia(&tag, ano, &e);
                if let Some(feita) = tec.tick_pais(&tag, ano, &e) {
                    // Tecnologia civil (produção/sociedade) eleva a produtividade do país (+0,6%), como na ponte do jogo.
                    if tec.catalogo.obter(&feita).is_some_and(|x| matches!(x.ramo, sim_tecnologia::Ramo::Producao | sim_tecnologia::Ramo::Sociedade))
                        && let Some(id) = id_por_tag.get(&tag)
                    {
                        eco.aplicar_ganho_produtividade(id, 0.006);
                    }
                }
            }
        });
        t_tec += i.elapsed().as_secs_f64();

        let i = Instant::now();
        guarda!("eventos", ds, {
            avaliar_dia(&cate, &mut est, &mut legado, jogador);
            // O "jogador" também é IA nesta partida: resolve as decisões pendentes.
            for p in std::mem::take(&mut est.pendentes) {
                if let Some(ev) = cate.por_id(&p.evento_id) {
                    let h = (legado.data.ano, legado.data.mes, legado.data.dia);
                    let k = escolha_ia(ev, h);
                    aplicar_opcao(&mut legado, &mut est, ev, k, &p.pais);
                    noticias_ia += 1;
                }
            }
        });
        t_ev += i.elapsed().as_secs_f64();

        // Verificações mensais
        if dia_global % 30 == 0 {
            if let Err(e) = eco.auditar_estabilidade() {
                ach.add(&ds, format!("economia.auditar: {e}"));
            }
            if let Err(e) = pol.auditar_estabilidade() {
                ach.add(&ds, format!("politica.auditar: {e}"));
            }
            for p in legado.paises.values() {
                let s = format!("{:?}", p.orcamento);
                if tem_nan(&s) {
                    ach.add(&ds, format!("legado: NaN/inf no orçamento de {}", p.tag));
                }
                if p.orcamento.tesouro < -1e12 {
                    ach.add(&ds, format!("legado: tesouro absurdo em {}: {}", p.tag, p.orcamento.tesouro));
                }
                let ps = format!("{:?}", p.politica);
                if tem_nan(&ps) {
                    ach.add(&ds, format!("legado: NaN/inf na política de {}", p.tag));
                }
            }
            if !dip.tensao.is_finite() || dip.tensao.abs() > 1000.0 {
                ach.add(&ds, format!("diplomacia: tensão inválida {}", dip.tensao));
            }
            for (k, v) in dip.pontos.iter() {
                if !v.is_finite() {
                    ach.add(&ds, format!("diplomacia: pontos NaN em {k}"));
                }
            }
            for (k, e) in &tec.paises {
                if !e.buffer.is_finite() || e.buffer < 0.0 || !e.pontos_dia.is_finite() {
                    ach.add(&ds, format!("tecnologia: buffer/pontos inválidos em {k}: {} {}", e.buffer, e.pontos_dia));
                }
                if e.progresso.values().any(|v| !v.is_finite() || *v < 0.0) {
                    ach.add(&ds, format!("tecnologia: progresso inválido em {k}"));
                }
            }
            let mut tot = |k: &'static str, v: usize| {
                let m = max_estr.entry(k).or_insert(0);
                *m = (*m).max(v);
            };
            tot("jornal_eventos", est.jornal.len());
            tot("eventos_disparados", est.disparados.len());
            tot("pactos_diplomacia", dip.pactos.len());
            tot("log_diplomacia", dip.log.len());
            tot("filas_construcao(soma)", eco.paises().iter().map(|p| p.fila_construcao.len()).sum());
            tot("rotas_comerciais", eco.rotas_comerciais().len());
            tot("pendentes_eventos", est.pendentes.len());
        }
        // Verificação profunda (NaN em qualquer campo) a cada 180 dias
        if dia_global % 180 == 0 {
            for p in eco.paises() {
                if tem_nan(&format!("{:?}", p.orcamento)) || tem_nan(&format!("{:?}", p.politica_fiscal)) {
                    ach.add(&ds, format!("economia: NaN/inf (debug) no país {}", p.id));
                }
            }
            for pr in eco.provincias() {
                for pop in &pr.pops {
                    if tem_nan(&format!("{pop:?}")) {
                        ach.add(&ds, format!("economia: NaN/inf em pop da província {}", pr.id));
                        break;
                    }
                }
            }
        }
        // Resumo anual
        if dia_global % 365 == 0 {
            let ano = legado.data.ano;
            let pib: f64 = eco.paises().iter().map(|p| p.pib_diario).sum::<f64>() * 365.0;
            let pop: f64 = eco.paises().iter().map(|p| eco.populacao_total_pais(&p.id).unwrap_or(0.0)).sum();
            let n_tec: usize = tec.paises.values().map(|e| e.conhecidas.len()).sum();
            let resumo = format!(
                "| {ano} | {:.0} | {:.0} | {:.2e} | {:.2e} | {} | {} | {} | {:.1} | {:.0} |",
                t_total.elapsed().as_secs_f64(),
                rss_mb(),
                pib,
                pop,
                n_tec,
                est.jornal.len(),
                dip.pactos.len(),
                dip.tensao,
                eco.dias_decorridos() as f64
            );
            eprintln!("{resumo}");
            anuais.push(resumo);
            let cresc = if pib_ant > 0.0 { format!("{:+.1}%", (pib / pib_ant - 1.0) * 100.0) } else { "-".into() };
            pib_ant = pib;
            let mut linha = format!("| {ano} | {pib:.3e} | {cresc} |");
            for t in PAISES_TABELA {
                let v = id_por_tag.get(t).and_then(|id| eco.obter_pais(id)).map_or(0.0, |p| p.pib_diario * 365.0);
                let _ = write!(linha, " {v:.2e} |");
            }
            por_pais.push(linha);
        }
    }

    // Verificação final
    let ds = format!("{:04}-{:02}-{:02}", legado.data.ano, legado.data.mes, legado.data.dia);
    if let Err(e) = eco.auditar_estabilidade() {
        ach.add(&ds, format!("FINAL economia.auditar: {e}"));
    }
    if let Err(e) = pol.auditar_estabilidade() {
        ach.add(&ds, format!("FINAL politica.auditar: {e}"));
    }
    let seg = t_total.elapsed().as_secs_f64();
    let rss_fim = rss_mb();
    let dias = total_dias as f64;
    let ms = |t: f64| t * 1000.0 / dias;
    let mut r = String::new();
    let _ = writeln!(r, "# QA: partida longa headless\n");
    let _ = writeln!(r, "Gerado por `cargo run --example partida_longa` (crate guerra_fria). Perfil dev (opt-level 1 no workspace, 3 nas dependências, overflow checks ligados).\n");
    let _ = writeln!(r, "- Período: 1945-01-01 + {anos} anos ({total_dias} dias); IA ligada: tecnologia (`escolher_ia` para todos), eventos (jogador USA tratado como IA via `escolha_ia`), diplomacia (`tick_dia`), política, economia + legado.");
    let _ = writeln!(r, "- Carga: {carga_s:.1}s (inclui 90 dias de aquecimento da economia). Tempo total da simulação: {seg:.0}s.");
    let _ = writeln!(r, "- RSS: inicial {rss0:.0} MB, após carga {:.0} MB, final {rss_fim:.0} MB.", rss_mb().min(rss_fim));
    let _ = writeln!(r, "- Eventos de IA resolvidos pelo \"jogador\": {noticias_ia}.\n");
    let _ = writeln!(r, "## Tempo médio por dia (ms)\n\n| Sistema | ms/dia |\n|---|---|");
    for (n, t) in [("economia (sim_economia)", t_eco), ("legado (simulacao::executar_tick_economia)", t_leg), ("política", t_pol), ("diplomacia", t_dip), ("tecnologia", t_tec), ("eventos", t_ev)] {
        let _ = writeln!(r, "| {n} | {:.2} |", ms(t));
    }
    let _ = writeln!(r, "\n## Resumo anual\n\n| Ano | t(s) | RSS MB | PIB mundial/ano | pop. mundial | techs conhecidas | notícias | pactos | tensão | dias eco |\n|---|---|---|---|---|---|---|---|---|---|");
    for l in &anuais {
        let _ = writeln!(r, "{l}");
    }
    let _ = writeln!(r, "\n## PIB por país (nominal/ano)\n\n| Ano | Mundo | Cresc. | {} |\n|---|---|---|{}", PAISES_TABELA.join(" | "), "---|".repeat(PAISES_TABELA.len()));
    for l in &por_pais {
        let _ = writeln!(r, "{l}");
    }
    let _ = writeln!(r, "\n## Tamanho máximo das estruturas (explosão de entidades)\n");
    for (k, v) in &max_estr {
        let _ = writeln!(r, "- {k}: {v}");
    }
    let _ = writeln!(r, "\n## Pânicos\n");
    if ach.panicos.is_empty() {
        let _ = writeln!(r, "Nenhum pânico.");
    }
    for (k, n) in &ach.panicos {
        let _ = writeln!(r, "- {k} (x{n})");
    }
    let _ = writeln!(r, "\n## Achados (NaN, valores absurdos, auditorias)\n");
    if ach.linhas.is_empty() {
        let _ = writeln!(r, "Nenhum.");
    }
    for l in &ach.linhas {
        let _ = writeln!(r, "- {l}");
    }
    println!("{r}");
    if escrever {
        let saida = args.iter().find_map(|a| a.strip_prefix("--saida=")).unwrap_or("docs/qa_partida_longa.md");
        let _ = std::fs::write(raiz.join(saida), &r);
    }
}
