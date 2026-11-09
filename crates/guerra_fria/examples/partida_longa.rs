//! Partida longa headless (1945-1965) para QA: roda economia, política, diplomacia, tecnologia e eventos
//! (legado incluso) com a IA ligada e procura pânicos, NaN, valores absurdos, explosão de estruturas e
//! vazamento de memória (VmRSS). Escreve o relatório em `docs/qa_partida_longa.md`.
//!
//! Uso: cargo run --example partida_longa [-- ANOS] [--sem-relatorio] [--saida=docs/arquivo.md]
use eventos::{CatalogoEventos, Condicao, EstadoEventos, Noticia, aplicar_opcao, avaliar_dia, escolha_ia};
use ia_paises::IaEconomia;
use sim_diplomacia::{Bloco, Diplomacia, InfoPais};
use sim_economia::{CenarioEconomia, ModoCadencia, MundoEconomico};
use sim_politica::{CenarioPolitica, MundoPolitico};
use sim_tecnologia::{CatalogoTec, ConfigBlocos, EntradaPais, MundoTec};
use simulacao::{AlinhamentoGeopolitico, MundoJogo, formacao::Formacoes};
use std::collections::{BTreeMap, HashSet};
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
    vistos: HashSet<String>,
    panicos: BTreeMap<String, u32>,
}
impl Achados {
    /// Registra uma ocorrência, deduplicando pela mensagem (mantém a primeira data).
    fn add(&mut self, data: &str, m: impl Into<String>) {
        let m = m.into();
        if self.vistos.len() < 400 && self.vistos.insert(m.clone()) {
            self.linhas.push(format!("{data} {m}"));
        }
    }
}

fn tem_nan(s: &str) -> bool {
    // Procura as palavras inteiras `NaN` e `inf` (não confundir com "influência" etc.).
    s.split(|c: char| !c.is_alphanumeric()).any(|w| w == "NaN" || w == "inf")
}

fn fdata(d: (i32, u8, u8)) -> String {
    format!("{:04}-{:02}-{:02}", d.0, d.1, d.2)
}

/// Mercados com bens precificados abaixo de 20% do preço-base (a tabela viva do motor, não o espelho
/// do último tick). Uma formação de país não deve derrubar o nível de preços de mercado nenhum.
fn mercados_colapsados(eco: &MundoEconomico) -> Vec<(String, usize)> {
    let catalogo = eco.catalogo();
    let mut out = Vec::new();
    for m in eco.mercados() {
        let n = catalogo
            .bens
            .iter()
            .filter(|b| eco.preco_atual(&m.id, &b.id).is_some_and(|p| p < 0.2 * b.preco_base))
            .count();
        if n > 0 {
            out.push((m.id.clone(), n));
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let anos: i32 = args.iter().find_map(|a| a.parse().ok()).unwrap_or(20);
    let escrever = !args.iter().any(|a| a == "--sem-relatorio");
    let raiz: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap();
    let t_carga = Instant::now();
    let rss0 = rss_mb();

    // Legado (províncias, donos, política antiga)
    let (cat_prov, mut legado) = MundoJogo::carregar_de_arquivos(
        raiz.join("assets/map/provinces.ron"),
        raiz.join("data/scenario_1945/countries_1945.ron"),
        raiz.join("data/scenario_1945/province_owners_1945.ron"),
    )
    .expect("legado");
    // Descolonização e formáveis (o cliente os roda no plugin `formacao_sim`; aqui é headless)
    let mut formacoes = Formacoes::carregar(&raiz.join("data/formaveis"), &cat_prov).expect("formáveis");
    // Economia
    let mut cen = CenarioEconomia::carregar_diretorio(&raiz.join("assets/data/economia")).expect("cenário economia");
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    let mut eco = MundoEconomico::novo(cen).expect("economia");
    let mut ia_eco = IaEconomia::novo();
    let mut id_por_tag: BTreeMap<String, String> = eco.paises().iter().filter_map(|p| p.tag.clone().map(|t| (t, p.id.clone()))).collect();
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
    let (mut t_eco, mut t_pol, mut t_dip, mut t_tec, mut t_ev, mut t_leg, mut t_form) =
        (0.0f64, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let mut anuais: Vec<String> = Vec::new();
    let mut por_pais: Vec<String> = Vec::new();
    #[allow(unused_assignments)] // o último ano grava o PIB de referência sem que ninguém o leia
    let mut pib_ant = 0.0f64;
    let mut max_estr = BTreeMap::<&str, usize>::new();
    let total_dias = anos as u64 * 365;
    let t_total = Instant::now();
    let mut dia_global = 0u64;
    let mut noticias_ia = 0usize;
    // Cobertura de formações e eventos
    let mut disparado_em: BTreeMap<String, (i32, u8, u8)> = BTreeMap::new();
    let mut contagem_disparos: BTreeMap<String, u32> = BTreeMap::new();
    let mut linhas_formacao: Vec<String> = Vec::new();
    let mut pop_max_por_prov = 0usize;
    let mut paises_eco = eco.paises().len();
    let mut tesouro_mes_ant: BTreeMap<String, f64> = BTreeMap::new();

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

    let mut ano_amostra = legado.data.ano;
    let (mut t_ano, mut dias_ano) = (0.0f64, 0u32);

    /// Linha de resumo do ano que acabou de fechar (`$ano`) mais o custo real daquele ano.
    macro_rules! fecho_ano {
        ($ano:expr) => {{
            let ano = $ano;
            let pib: f64 = eco.paises().iter().map(|p| p.pib_diario).sum::<f64>() * 365.0;
            let pop: f64 = eco.paises().iter().map(|p| eco.populacao_total_pais(&p.id).unwrap_or(0.0)).sum();
            let n_tec: usize = tec.paises.values().map(|e| e.conhecidas.len()).sum();
            let ms_ano = t_ano * 1000.0 / (dias_ano.max(1) as f64);
            let resumo = format!(
                "| {ano} | {:.0} | {:.0} | {:.0} | {:.1} | {:.2e} | {:.2e} | {} | {} | {} | {:.1} | {:.0} |",
                t_total.elapsed().as_secs_f64(),
                rss_mb(),
                t_ano,
                ms_ano,
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
        }};
    }

    while dia_global < total_dias {
        dia_global += 1;
        let t_dia = Instant::now();
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

        // Descolonização/formáveis + integração nos mesmos motores que o cliente usa
        // (economia, política, diplomacia, jornal), seguida de auditoria imediata: uma formação
        // que destrói preços ou alíquotas precisa aparecer no mesmo dia, não um mês depois.
        let i = Instant::now();
        let mut formou = false;
        guarda!("formacoes", ds, {
            for e in formacoes.processar(&mut legado) {
                let dd = e.data;
                if e.pais_novo {
                    formou = true;
                }
                linhas_formacao.push(format!(
                    "| {:04}-{:02}-{:02} | {} | {} | {:?} | {} | {} | {} |",
                    dd.ano, dd.mes, dd.dia, e.tag, e.nome, e.origens, e.provincias.len(), e.adm1.len(), e.populacao
                ));
                est.jornal.push(Noticia {
                    data: (dd.ano, dd.mes, dd.dia),
                    texto: format!("{} ({}): {} províncias.", e.nome, e.tag, e.provincias.len()),
                });
                let bloco = legado
                    .paises
                    .get(&e.tag)
                    .map(|p| match p.alinhamento_inicial {
                        AlinhamentoGeopolitico::BlocoOcidental => Bloco::Ocidental,
                        AlinhamentoGeopolitico::BlocoSovietico => Bloco::Sovietico,
                        _ => Bloco::NaoAlinhado,
                    })
                    .unwrap_or(Bloco::NaoAlinhado);
                dip.adicionar_pais(InfoPais { tag: norm_diplo(&e.tag), bloco });
                if e.pais_novo {
                    if let Err(err) = eco.adicionar_pais_formado(&e.tag, &e.nome, &e.origens, &e.adm1) {
                        ach.add(&ds, format!("formacao: economia recusou {}: {err}", e.tag));
                    } else if let Some(id) =
                        eco.paises().iter().find(|p| p.tag.as_deref() == Some(e.tag.as_str())).map(|p| p.id.clone())
                    {
                        id_por_tag.insert(e.tag.clone(), id);
                    }
                    if let Err(err) = pol.adicionar_pais_formado(&e.tag, &e.nome, e.populacao, &e.leis_iniciais, &e.ideologia) {
                        ach.add(&ds, format!("formacao: política recusou {}: {err}", e.tag));
                    }
                }
            }
            t_form += i.elapsed().as_secs_f64();
        });
        if formou {
            paises_eco = eco.paises().len();
            if let Err(e) = eco.auditar_estabilidade() {
                ach.add(&ds, format!("formacao.auditar: {e}"));
            }
            let colapso = mercados_colapsados(&eco);
            for (m, n) in colapso {
                ach.add(&ds, format!("formacao: mercado {m} com {n} bens sob 20% do preço-base"));
            }
        }

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
                        let alfa = if *pp > 0.0 { pa / pp } else { 0.5 };
                        let pib = p.pib_diario * 365.0;
                        let pib_pc = if *pp > 0.0 { pib / pp } else { 0.0 };
                        let univ_eco = ((pib / 4.0e10).sqrt() * 2.2 + (pib_pc / 400.0).clamp(0.0, 6.0)) * (0.3 + 0.7 * alfa);
                        let niveis_univ = (*u).max(univ_eco);
                        entradas.insert(tag.clone(), EntradaPais { niveis_universidade: niveis_univ, alfabetizacao: alfa, ..Default::default() });
                    }
                }
            }
            let ano = legado.data.ano;
            let tags: Vec<String> = tec.paises.keys().cloned().collect();
            for tag in tags {
                let e = entradas.get(&tag).copied().unwrap_or_default();
                tec.escolher_ia(&tag, ano, &e);
                if let Some(feita) = tec.tick_pais(&tag, ano, &e) {
                    if let Some(id) = id_por_tag.get(&tag) {
                        eco.registrar_tecnologia_pesquisada(id, &feita);
                        // Tecnologia civil (produção/sociedade) eleva a produtividade do país (+0,6%), como na ponte do jogo.
                        if tec.catalogo.obter(&feita).is_some_and(|x| matches!(x.ramo, sim_tecnologia::Ramo::Producao | sim_tecnologia::Ramo::Sociedade)) {
                            eco.aplicar_ganho_produtividade(id, 0.006);
                        }
                    }
                }
            }
        });
        t_tec += i.elapsed().as_secs_f64();

        let i = Instant::now();
        guarda!("eventos", ds, {
            let hoje = (legado.data.ano, legado.data.mes, legado.data.dia);
            for id in avaliar_dia(&cate, &mut est, &mut legado, jogador) {
                eco.registrar_evento_historico(&id);
                *contagem_disparos.entry(id.clone()).or_default() += 1;
                disparado_em.entry(id).or_insert(hoje);
            }
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
            for (m, n) in mercados_colapsados(&eco) {
                ach.add(&ds, format!("economia: mercado {m} com {n} bens sob 20% do preço-base"));
            }
            let mut tesouro_mes_novo: BTreeMap<String, f64> = BTreeMap::new();
            for p in eco.paises() {
                let t = p.orcamento.tesouro;
                if let Some(&ant) = tesouro_mes_ant.get(&p.id) {
                    if ant > 0.0 && t > ant * 4.0 {
                        ach.add(&ds, format!("economia: tesouro de {} saltou de {ant:.0} para {t:.0} em 30 dias", p.id));
                    }
                }
                tesouro_mes_novo.insert(p.id.clone(), t);
                if let Some(d) = eco.taxa_desemprego_media_pais(&p.id) {
                    if !d.is_finite() {
                        ach.add(&ds, format!("economia: desemprego não finito em {}", p.id));
                    } else if d > 0.6 {
                        ach.add(&ds, format!("economia: desemprego absurdo em {}: {:.0}%", p.id, d * 100.0));
                    }
                }
            }
            tesouro_mes_ant = tesouro_mes_novo;
            pop_max_por_prov = pop_max_por_prov.max(eco.provincias().iter().map(|pr| pr.pops.len()).max().unwrap_or(0));
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
            tot("paises_na_economia", paises_eco);
            tot("pops_max_por_provincia", pop_max_por_prov);
            tot("tags_legado", legado.paises.len());
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
        // Resumo anual: na virada do ANO CIVIL. Amostragem a cada 365 dias desloca a linha nos anos
        // bissextos (chegava a repetir um ano e pular outro) e esconde o custo real de cada ano.
        t_ano += t_dia.elapsed().as_secs_f64();
        dias_ano += 1;
        if legado.data.ano != ano_amostra {
            fecho_ano!(ano_amostra);
            ano_amostra = legado.data.ano;
            t_ano = 0.0;
            dias_ano = 0;
        }
    }
    if dias_ano > 0 {
        fecho_ano!(ano_amostra);
    }

    // Verificação final
    let ds = format!("{:04}-{:02}-{:02}", legado.data.ano, legado.data.mes, legado.data.dia);
    if let Err(e) = eco.auditar_estabilidade() {
        ach.add(&ds, format!("FINAL economia.auditar: {e}"));
    }
    if let Err(e) = pol.auditar_estabilidade() {
        ach.add(&ds, format!("FINAL politica.auditar: {e}"));
    }

    // Cobertura do catálogo de eventos versus o que aconteceu na partida
    let tags_vividas: HashSet<String> = legado.paises.keys().cloned().collect();
    let ids_catalogo: HashSet<&str> = cate.eventos.iter().map(|e| e.id.as_str()).collect();
    let fim = (legado.data.ano, legado.data.mes, legado.data.dia);
    let mut nunca_disparados: Vec<String> = Vec::new();
    let mut fora_da_data: Vec<String> = Vec::new();
    let mut alvos_inexistentes: Vec<String> = Vec::new();
    let mut refs_quebradas: Vec<String> = Vec::new();
    let mut condicoes_sem_suporte: BTreeMap<String, u32> = BTreeMap::new();
    for ev in &cate.eventos {
        for c in &ev.condicoes {
            match c {
                Condicao::Desconhecida(s) => {
                    *condicoes_sem_suporte.entry(s.clone()).or_default() += 1;
                }
                Condicao::PosEvento(id) => {
                    if !ids_catalogo.contains(id.as_str()) {
                        refs_quebradas.push(format!("{} aponta pos_evento:{id} (id inexistente)", ev.id));
                    }
                }
                _ => {}
            }
        }
        if !ev.paises.is_empty() && !ev.paises.iter().any(|t| tags_vividas.contains(t.as_str())) {
            alvos_inexistentes.push(format!("{} [{}] alvos {:?} não existem no mapa", ev.id, fdata(ev.data), ev.paises));
        }
        if ev.generico {
            continue;
        }
        match disparado_em.get(&ev.id) {
            None if ev.data <= fim => nunca_disparados.push(format!("{} [{}] alvos {:?}", ev.id, fdata(ev.data), ev.paises)),
            Some(q) if *q != ev.data => fora_da_data.push(format!("{} esperado [{}] disparado [{}]", ev.id, fdata(ev.data), fdata(*q))),
            _ => {}
        }
    }
    let repetidos: Vec<String> = contagem_disparos
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(id, n)| format!("{id} disparou {n} vezes"))
        .collect();
    let seg = t_total.elapsed().as_secs_f64();
    let rss_fim = rss_mb();
    let dias = total_dias as f64;
    let ms = |t: f64| t * 1000.0 / dias;
    let mut r = String::new();
    let _ = writeln!(r, "# QA: partida longa headless\n");
    let _ = writeln!(r, "Gerado por `cargo run --example partida_longa` (crate guerra_fria). Perfil dev (opt-level 1 no workspace, 3 nas dependências, overflow checks ligados).\n");
    let _ = writeln!(r, "- Período: 1945-01-01 + {anos} anos ({total_dias} dias); IA ligada: tecnologia (`escolher_ia` para todos), eventos (jogador USA tratado como IA via `escolha_ia`), diplomacia (`tick_dia`), política, economia + legado; descolonização/formações (`Formacoes::processar`, integradas em economia, política e diplomacia como no cliente).");
    let _ = writeln!(r, "- Carga: {carga_s:.1}s (inclui 90 dias de aquecimento da economia). Tempo total da simulação: {seg:.0}s.");
    let _ = writeln!(r, "- RSS: inicial {rss0:.0} MB, após carga {:.0} MB, final {rss_fim:.0} MB.", rss_mb().min(rss_fim));
    let _ = writeln!(r, "- Eventos de IA resolvidos pelo \"jogador\": {noticias_ia}.\n");
    let _ = writeln!(r, "## Tempo médio por dia (ms)\n\n| Sistema | ms/dia |\n|---|---|");
    for (n, t) in [("economia (sim_economia)", t_eco), ("legado (simulacao::executar_tick_economia)", t_leg), ("formações (descolonização)", t_form), ("política", t_pol), ("diplomacia", t_dip), ("tecnologia", t_tec), ("eventos", t_ev)] {
        let _ = writeln!(r, "| {n} | {:.2} |", ms(t));
    }
    let _ = writeln!(r, "\n## Resumo anual\n\n| Ano | t(s) | RSS MB | s/ano | ms/dia | PIB mundial/ano | pop. mundial | techs conhecidas | notícias | pactos | tensão | dias eco |\n|---|---|---|---|---|---|---|---|---|---|---|---|");
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
    let _ = writeln!(r, "\n## Descolonização e formáveis\n");
    if linhas_formacao.is_empty() {
        let _ = writeln!(r, "Nenhum país foi formado na janela — verifique `data/formaveis`.");
    } else {
        let _ = writeln!(r, "| Data | Tag | Nome | Origens | Províncias | ADM1 | População |\n|---|---|---|---|---|---|---|");
        for l in &linhas_formacao {
            let _ = writeln!(r, "{l}");
        }
    }
    let _ = writeln!(
        r,
        "\n## Cobertura de eventos\n\n- catálogo: {} eventos; únicos disparados: {}; pendências resolvidas: {noticias_ia}.",
        cate.eventos.len(),
        disparado_em.len()
    );
    let _ = writeln!(r, "- repetidos: {}; alvos inexistentes: {}; referências quebradas: {}; condições sem suporte: {} chaves.", repetidos.len(), alvos_inexistentes.len(), refs_quebradas.len(), condicoes_sem_suporte.len());
    for (titulo, itens) in [
        ("Eventos históricos que nunca dispararam na janela", &nunca_disparados),
        ("Eventos disparados fora da data histórica", &fora_da_data),
        ("Eventos com alvo inexistente no mapa", &alvos_inexistentes),
        ("Referências pos_evento quebradas", &refs_quebradas),
        ("Eventos disparados mais de uma vez", &repetidos),
    ] {
        if itens.is_empty() {
            continue;
        }
        let _ = writeln!(r, "\n### {titulo} ({} items)\n", itens.len());
        for l in itens.iter().take(80) {
            let _ = writeln!(r, "- {l}");
        }
        if itens.len() > 80 {
            let _ = writeln!(r, "- ... {} restantes", itens.len() - 80);
        }
    }
    if !condicoes_sem_suporte.is_empty() {
        let _ = writeln!(r, "\n### Condições que o cliente não sabe avaliar (viram `true`)\n");
        for (c, n) in &condicoes_sem_suporte {
            let _ = writeln!(r, "- {c} (x{n})");
        }
    }
    println!("{r}");
    if escrever {
        let saida = args.iter().find_map(|a| a.strip_prefix("--saida=")).unwrap_or("docs/qa_partida_longa.md");
        let _ = std::fs::write(raiz.join(saida), &r);
    }
}
