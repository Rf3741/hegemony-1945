//! Balanço inicial de 1945: desemprego, calote, saldo e PIB dos países grandes.
use sim_economia::{CenarioEconomia, MundoEconomico};
use std::path::Path;

fn carregar() -> MundoEconomico {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
    let mut cen = CenarioEconomia::carregar_diretorio(&dir).unwrap();
    cen.config.modo_cadencia = sim_economia::ModoCadencia::DiarioComAjusteSemanal;
    MundoEconomico::novo(cen).unwrap()
}

fn relatorio(m: &MundoEconomico, rotulo: &str) {
    for p in m.paises() {
        let o = &p.orcamento;
        eprintln!(
            "{rotulo} {:>14} desemp {:5.1}% calote {} PIBano ${:>8.1}bi saldo/dia {:>10.0} tesouro {:>12.0} divida {:>12.0} pop {:>6.1}M SoL {:.1}",
            p.id,
            100.0 * m.taxa_desemprego_media_pais(&p.id).unwrap_or(0.0),
            o.em_calote as u8,
            p.pib_diario * 365.0 / 1e9,
            o.saldo_diario,
            o.tesouro,
            o.divida,
            m.populacao_total_pais(&p.id).unwrap_or(0.0) / 1e6,
            m.padrao_de_vida_medio_pais(&p.id).unwrap_or(0.0),
        );
    }
}

#[test]
#[ignore = "diagnóstico manual"]
fn diagnostico_balanco() {
    let mut m = carregar();
    relatorio(&m, "d0");
    for ano in 1..=3 {
        m.avancar_dias(365);
        relatorio(&m, &format!("a{ano}"));
    }
}

#[test]
#[ignore = "diagnóstico manual"]
fn diagnostico_tipos() {
    let m = carregar();
    for id in ["bra", "usa", "sov", "gbr", "ind"] {
        let p = m.obter_pais(id).unwrap();
        let mut agg: std::collections::BTreeMap<String, (f64, f64)> = Default::default();
        for pr in m.provincias().iter().filter(|x| x.pais_id == id) {
            for pop in &pr.pops {
                let e = agg.entry(pop.tipo_pop_id.clone()).or_default();
                e.0 += pop.populacao;
                e.1 += pop.empregados;
            }
        }
        eprintln!("== {id} pib {:.1}bi", p.pib_diario * 365.0 / 1e9);
        for (k, (a, b)) in agg {
            eprintln!("   {k:14} pop {:>10.0} emp {:>10.0} ({:.0}%)", a, b, 100.0 * b / a.max(1.0));
        }
    }
}

#[test]
#[ignore = "diagnóstico manual"]
fn diagnostico_dias() {
    let mut m = carregar();
    let id = std::env::var("PAIS").unwrap_or("bra".into());
    let mut dia = 0;
    for alvo in [0, 1, 2, 5, 10, 20, 30, 60, 120, 240] {
        while dia < alvo { m.executar_tick_diario(); dia += 1; }
        let p = m.obter_pais(&id).unwrap();
        let (mut emp, mut niv, mut nb, mut lucro_neg) = (0.0, 0u32, 0, 0);
        let mut rend = 0.0; let mut ops = 0.0;
        for pr in m.provincias().iter().filter(|x| x.pais_id == id) {
            for b in &pr.predios { if b.nivel>0 { nb+=1; niv+=b.nivel; if b.margem_lucro() < 0.0 {lucro_neg+=1;} emp += b.total_empregados(); } }
            for pop in &pr.pops { rend += pop.renda_ultimo_tick; if pop.tipo_pop_id=="operarios" {ops+=pop.populacao;} }
        }
        eprintln!("dia {dia:>3} emp_pred {:>12.0} predios {nb} niveis {niv} margem<0 {lucro_neg} renda_pops {:>12.0} PIBd {:>12.0} desemp {:.1}% saldo {:.0} rec {:.0} desp {:.0} (burocr {:.0} milit {:.0} constr {:.0} subs {:.0} juros {:.0})",
          emp, rend, p.pib_diario, 100.0*m.taxa_desemprego_media_pais(&id).unwrap(), p.orcamento.saldo_diario, p.orcamento.receita_total(), p.orcamento.despesa_total(), p.orcamento.gasto_burocracia, p.orcamento.gasto_militar, p.orcamento.gasto_construcao, p.orcamento.gasto_subsidios, p.orcamento.gasto_juros);
    }
}

#[test]
#[ignore = "diagnóstico manual"]
fn diagnostico_predios() {
    let mut m = carregar();
    let id = std::env::var("PAIS").unwrap_or("bra".into());
    for _ in 0..std::env::var("DIAS").ok().and_then(|s| s.parse().ok()).unwrap_or(3) { m.executar_tick_diario(); }
    let mut agg: std::collections::BTreeMap<String, [f64; 7]> = Default::default();
    for pr in m.provincias().iter().filter(|x| x.pais_id == id) {
        for b in &pr.predios {
            let e = agg.entry(b.tipo_predio_id.clone()).or_default();
            e[0] += b.nivel as f64; e[1] += b.receita_ultimo_tick; e[2] += b.custo_insumos_ultimo_tick;
            e[3] += b.folha_salarial_ultimo_tick; e[4] += b.lucro_ultimo_tick; e[5] += b.total_empregados(); e[6] += b.total_vagas();
        }
    }
    eprintln!("tipo niveis receita insumos folha lucro emp vagas");
    for (k, v) in agg { eprintln!("PRED {k:28} {:>7.0} {:>12.0} {:>12.0} {:>12.0} {:>12.0} {:>10.0} {:>10.0}", v[0], v[1], v[2], v[3], v[4], v[5], v[6]); }
}

#[test]
#[ignore = "diagnóstico manual"]
fn diagnostico_infra() {
    let mut m = carregar();
    let id = std::env::var("PAIS").unwrap_or("bra".into());
    for _ in 0..2 { m.executar_tick_diario(); }
    let mut n = 0;
    let (mut sc, mut su) = (0.0, 0.0);
    for e in m.estados().iter().filter(|e| e.pais_id == id) {
        n += 1; sc += e.infraestrutura_capacidade; su += e.infraestrutura_uso;
        if n < 12 { eprintln!("EST {:28} cap {:>7.1} (terr {:.1} urb {:.1} fer {:.1} rod {:.1} por {:.1}) uso {:>8.1} acesso {:.2}", e.id, e.infraestrutura_capacidade, e.infraestrutura_base_terreno, e.infraestrutura_urbanizacao, e.infraestrutura_ferrovias, e.infraestrutura_rodovias, e.infraestrutura_portos, e.infraestrutura_uso, e.acesso_mercado); }
    }
    eprintln!("EST total {n} cap {sc:.0} uso {su:.0}");
    let mut hist = [0; 5];
    for e in m.estados() { let a = e.acesso_mercado; hist[((a*4.0).floor() as usize).min(4)] += 1; }
    eprintln!("EST hist acesso mundial (0-.25,.25-.5,.5-.75,.75-1,1) {hist:?}");
}

#[test]
#[ignore = "diagnóstico manual"]
fn diagnostico_mercado() {
    let mut m = carregar();
    let id = std::env::var("PAIS").unwrap_or("bra".into());
    let mid = m.obter_pais(&id).unwrap().mercado_id.clone();
    let mut dia = 0;
    for alvo in [1, 3, 6, 12] {
        while dia < alvo { m.executar_tick_diario(); dia += 1; }
        eprintln!("MERC dia {dia} mercado {mid}");
        for bem in &m.catalogo().bens {
            let o = m.oferta_atual(&mid, &bem.id).unwrap();
            let d = m.demanda_atual(&mid, &bem.id).unwrap();
            let p = m.preco_atual(&mid, &bem.id).unwrap();
            if o + d > 0.0 { eprintln!("MERC {:26} oferta {:>12.0} demanda {:>12.0} preco/base {:.2}", bem.id, o, d, p / bem.preco_base); }
        }
    }
}

const GRANDES: [&str; 10] = ["usa", "sov", "gbr", "fra", "ger", "jap", "ind", "chi", "bra", "ita"];

#[test]
fn balanco_1945_estavel_nos_paises_grandes() {
    let mut m = carregar();
    let pib0: Vec<f64> = GRANDES.iter().map(|id| m.obter_pais(id).unwrap().pib_diario * 365.0).collect();
    for id in GRANDES {
        let d = m.taxa_desemprego_media_pais(id).unwrap();
        assert!((0.03..=0.15).contains(&d), "{id}: desemprego inicial {:.1}%", 100.0 * d);
    }
    // 1 ano de jogo: sem calote, desemprego plausível, PIB sem colapso, orçamento sem sangria
    for dia in 1..=365 {
        m.executar_tick_diario();
        if dia % 30 == 0 {
            for id in GRANDES {
                assert!(!m.obter_pais(id).unwrap().orcamento.em_calote, "{id}: calote no dia {dia}");
            }
        }
    }
    for (i, id) in GRANDES.iter().enumerate() {
        let p = m.obter_pais(id).unwrap();
        let d = m.taxa_desemprego_media_pais(id).unwrap();
        let pib = p.pib_diario * 365.0;
        eprintln!("{id}: desemp {:.1}% PIB {:.1} bi (inicial {:.1}) saldo/dia {:.0}", 100.0 * d, pib / 1e9, pib0[i] / 1e9, p.orcamento.saldo_diario);
        assert!(d <= 0.20, "{id}: desemprego {:.1}% após 1 ano", 100.0 * d);
        assert!(pib >= 0.25 * pib0[i], "{id}: PIB colapsou ({:.1} bi)", pib / 1e9);
        assert!(p.orcamento.saldo_diario > -0.02 * p.pib_diario, "{id}: déficit diário excessivo");
        m.auditar_estabilidade().unwrap();
    }
}

fn niveis_totais(m: &MundoEconomico) -> (u64, u64) {
    let (mut n, mut b) = (0u64, 0u64);
    for p in m.provincias() {
        for pr in &p.predios {
            n += pr.nivel as u64;
            b += 1;
        }
    }
    (n, b)
}

#[test]
fn construcao_automatica_5_anos_ritmo_realista_e_tick_rapido() {
    let mut m = carregar();
    let (n0, b0) = niveis_totais(&m);
    let niveis_pais = |m: &MundoEconomico, id: &str| -> u64 {
        m.provincias().iter().filter(|p| p.pais_id == id).flat_map(|p| p.predios.iter()).map(|b| b.nivel as u64).sum()
    };
    let (usa0, ger0) = (niveis_pais(&m, "usa"), niveis_pais(&m, "ger"));
    let t = std::time::Instant::now();
    let mut maxima_fila = 0usize;
    let mut obras = 0usize;
    let mut tempos = Vec::with_capacity(1826);
    for dia in 1..=1826u32 {
        let t0 = std::time::Instant::now();
        let r = m.executar_tick_diario();
        tempos.push(t0.elapsed().as_secs_f64() * 1000.0);
        obras += r.obras_concluidas;
        if dia % 30 == 0 {
            maxima_fila = maxima_fila.max(m.paises().iter().map(|p| p.fila_construcao.len()).max().unwrap_or(0));
        }
    }
    let _ = t;
    tempos.sort_by(|a, b| a.total_cmp(b));
    let medio_ms = tempos[tempos.len() / 2]; // mediana: robusta a CPU compartilhada
    let (n1, b1) = niveis_totais(&m);
    eprintln!("prédios 1945: {b0} ({n0} níveis); 1950: {b1} ({n1} níveis); obras: {obras}; fila máx: {maxima_fila}; tick mediano {medio_ms:.1} ms");
    eprintln!("EUA níveis {usa0} -> {}; ALE {ger0} -> {}", niveis_pais(&m, "usa"), niveis_pais(&m, "ger"));
    // Meta: < 30 ms (medido 29 ms com a máquina ociosa); limite do teste folgado por causa de CPU compartilhada.
    assert!(medio_ms < 120.0, "tick médio {medio_ms:.1} ms");
    assert!(n1 as f64 <= n0 as f64 * 1.25, "níveis explodiram: {n0} -> {n1}");
    assert!(maxima_fila <= 6, "fila por país grande demais: {maxima_fila}");
}

#[test]
#[ignore = "diagnóstico manual"]
fn diagnostico_escala() {
    let mut m = carregar();
    let id = std::env::var("PAIS").unwrap_or("bra".into());
    for _ in 0..std::env::var("DIAS").ok().and_then(|s| s.parse().ok()).unwrap_or(200) { m.executar_tick_diario(); }
    let mut agg: std::collections::BTreeMap<String, [f64; 4]> = Default::default();
    for pr in m.provincias().iter().filter(|x| x.pais_id == id) {
        for b in &pr.predios {
            let e = agg.entry(b.tipo_predio_id.clone()).or_default();
            e[0] += b.nivel as f64; e[1] += b.escala_efetiva(); e[2] += b.fator_insumos() * b.nivel as f64; e[3] += b.taxa_ocupacao() * b.nivel as f64;
        }
    }
    for (k, v) in agg { eprintln!("ESC {k:28} niveis {:>6.0} escala/nivel {:.2} fator_ins {:.2} ocup {:.2}", v[0], v[1] / v[0], v[2] / v[0], v[3] / v[0]); }
    let a: Vec<f64> = m.estados().iter().filter(|e| e.pais_id == id).map(|e| e.acesso_mercado).collect();
    eprintln!("ESC acesso medio {:.2}", a.iter().sum::<f64>() / a.len() as f64);
}

#[test]
#[ignore = "diagnóstico manual"]
fn diagnostico_suprimento() {
    let mut m = carregar();
    for _ in 0..200 { m.executar_tick_diario(); }
    let mid = m.obter_pais("bra").unwrap().mercado_id.clone();
    for bem in &m.catalogo().bens {
        let o = m.oferta_atual(&mid, &bem.id).unwrap();
        let d = m.demanda_atual(&mid, &bem.id).unwrap();
        let sup = if d <= 1e-6 || 2.0 * o >= d { 1.0 } else { 2.0 * o / d };
        eprintln!("SUP {:26} sup {:.2} oferta {:>10.0} demanda {:>10.0} preco/base {:.2}", bem.id, sup, o, d, m.preco_atual(&mid, &bem.id).unwrap() / bem.preco_base);
    }
}

#[test]
fn cinco_anos_desemprego_e_pib_dos_grandes_estaveis() {
    let mut m = carregar();
    let pib0: Vec<f64> = GRANDES.iter().map(|id| m.obter_pais(id).unwrap().pib_diario).collect();
    for dia in 1..=1826 {
        m.executar_tick_diario();
        if dia % 365 == 0 {
            for (i, id) in GRANDES.iter().enumerate() {
                let p = m.obter_pais(id).unwrap();
                let d = m.taxa_desemprego_media_pais(id).unwrap();
                eprintln!("ano {} {id}: desemp {:.1}% PIB {:.0}% do inicial", dia / 365, 100.0 * d, 100.0 * p.pib_diario / pib0[i]);
                assert!(d < 0.18, "{id}: desemprego {:.1}% no ano {}", 100.0 * d, dia / 365);
                assert!(p.pib_diario > 0.70 * pib0[i], "{id}: PIB em {:.0}% do inicial no ano {}", 100.0 * p.pib_diario / pib0[i], dia / 365);
                assert!(!p.orcamento.em_calote, "{id}: calote");
            }
        }
    }
}

#[test]
fn populacao_exibida_na_escala_historica() {
    let m = carregar();
    for (id, mi) in [("usa", 140.0), ("sov", 170.0), ("chi", 500.0), ("ind", 400.0), ("bra", 46.0)] {
        let p = m.populacao_exibida_pais(id).unwrap() / 1e6;
        assert!((p - mi).abs() < 0.5, "{id}: {p:.1} mi (esperado {mi})");
    }
}
