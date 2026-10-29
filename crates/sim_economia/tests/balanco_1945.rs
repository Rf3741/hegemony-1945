//! Balanço inicial de 1945: desemprego, calote, saldo e PIB dos países grandes.
use sim_economia::{CenarioEconomia, MundoEconomico};
use std::path::Path;

fn carregar() -> MundoEconomico {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
    MundoEconomico::novo(CenarioEconomia::carregar_diretorio(&dir).unwrap()).unwrap()
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
fn diagnostico_balanco() {
    let mut m = carregar();
    relatorio(&m, "d0");
    for ano in 1..=3 {
        m.avancar_dias(365);
        relatorio(&m, &format!("a{ano}"));
    }
}

#[test]
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
fn diagnostico_predios() {
    let mut m = carregar();
    let id = std::env::var("PAIS").unwrap_or("bra".into());
    for _ in 0..3 { m.executar_tick_diario(); }
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
