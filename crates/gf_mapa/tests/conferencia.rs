//! Conferencia de `adm1_code`: `assets/map/provinces.ron` x `regioes_1945_canonico.ron` x `estados.ron`.
//!
//! Os dados de staging ficam fora do repositorio; informe `GF_STAGING` (padrao:
//! `../guerra-fria-staging` ao lado do workspace). Sem staging, os testes que dependem dele sao pulados.
//! `GF_ESCREVER_RELATORIO=1` regrava `docs/gf_mapa_conferencia.md`.

use gf_mapa::{CatalogoProvincias, cor_unica_provincia, id_de_cor_provincia};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn catalogo() -> CatalogoProvincias {
    let txt = std::fs::read_to_string(raiz().join("assets/map/provinces.ron")).unwrap();
    ron::from_str(&txt).unwrap()
}

fn staging() -> Option<PathBuf> {
    let p = std::env::var_os("GF_STAGING")
        .map(PathBuf::from)
        .unwrap_or_else(|| raiz().join("../guerra-fria-staging"));
    p.join("provincias_1945/regioes_1945_canonico.ron").exists().then_some(p)
}

/// Codigos `adm1_code: "X"` de `regioes_1945_canonico.ron`.
fn codigos_regioes(s: &std::path::Path) -> BTreeSet<String> {
    let txt = std::fs::read_to_string(s.join("provincias_1945/regioes_1945_canonico.ron")).unwrap();
    txt.lines()
        .filter_map(|l| l.trim().strip_prefix("adm1_code: \""))
        .filter_map(|l| l.split('"').next())
        .map(String::from)
        .collect()
}

/// Codigos listados em `regioes_admin1: [ ... ]` de `estados.ron`.
fn codigos_estados(s: &std::path::Path) -> (BTreeSet<String>, usize) {
    let txt = std::fs::read_to_string(s.join("territorio_dados/estados.ron")).unwrap();
    let (mut set, mut dentro, mut n) = (BTreeSet::new(), false, 0);
    for l in txt.lines().map(str::trim) {
        if l.starts_with("id: \"") {
            n += 1;
        }
        if l.starts_with("regioes_admin1:") {
            dentro = true;
            continue;
        }
        if dentro {
            if l.starts_with(']') {
                dentro = false;
            } else if let Some(c) = l.trim_end_matches(',').strip_prefix('"') {
                set.insert(c.trim_end_matches('"').to_string());
            }
        }
    }
    (set, n)
}

#[test]
fn catalogo_ids_sequenciais_cores_bijetivas_e_adm1_unicos() {
    let c = catalogo();
    assert!(!c.provincias.is_empty());
    let mut adm1 = HashSet::new();
    let mut cores = HashSet::new();
    for (i, p) in c.provincias.iter().enumerate() {
        assert_eq!(p.id as usize, i + 1);
        assert_eq!(p.cor, cor_unica_provincia(p.id));
        assert_eq!(id_de_cor_provincia(p.cor), Some(p.id));
        assert!(adm1.insert(p.adm1_code.clone()), "adm1 duplicado {}", p.adm1_code);
        assert!(cores.insert(p.cor));
        assert!(c.obter_por_id(p.id).is_some());
    }
    assert_eq!(c.mapa_cor_para_id().len(), c.provincias.len());
}

#[test]
fn todos_adm1_de_regioes_1945_existem_em_provinces_ron() {
    let Some(s) = staging() else { return };
    let c = catalogo();
    let prov: BTreeSet<_> = c.provincias.iter().map(|p| p.adm1_code.clone()).collect();
    let regs = codigos_regioes(&s);
    assert_eq!(regs.len(), 4596);
    let faltam: Vec<_> = regs.difference(&prov).collect();
    assert!(faltam.is_empty(), "{} adm1 de regioes_1945 ausentes em provinces.ron: {:?}", faltam.len(), &faltam[..faltam.len().min(20)]);
}

#[test]
fn adm1_dos_estados_existem_em_regioes_e_provinces() {
    let Some(s) = staging() else { return };
    let c = catalogo();
    let prov: BTreeSet<_> = c.provincias.iter().map(|p| p.adm1_code.clone()).collect();
    let regs = codigos_regioes(&s);
    let (est, n) = codigos_estados(&s);
    assert_eq!(n, 930);
    assert!(est.difference(&regs).count() == 0, "estados referenciam adm1 fora de regioes_1945");
    assert!(est.difference(&prov).count() == 0, "estados referenciam adm1 fora de provinces.ron");
}

/// Gera o relatorio (so grava com `GF_ESCREVER_RELATORIO=1`).
#[test]
fn relatorio_de_divergencias() {
    let Some(s) = staging() else { return };
    let c = catalogo();
    let prov: BTreeSet<_> = c.provincias.iter().map(|p| p.adm1_code.clone()).collect();
    let regs = codigos_regioes(&s);
    let (est, n) = codigos_estados(&s);
    let lista = |t: &str, v: Vec<&String>| {
        let mut o = format!("### {t}: {}\n\n", v.len());
        for x in v.iter().take(200) { o += &format!("- `{x}`\n"); }
        o + "\n"
    };
    let d1: Vec<_> = regs.difference(&prov).collect();
    let d2: Vec<_> = prov.difference(&regs).collect();
    let d3: Vec<_> = est.difference(&prov).collect();
    let d4: Vec<_> = regs.difference(&est).collect();
    let mut md = format!(
        "# Conferencia de adm1_code (T04)\n\nGerado por `cargo test -p gf_mapa --test conferencia` com `GF_ESCREVER_RELATORIO=1`.\n\n\
## Totais\n\n- `assets/map/provinces.ron`: {} provincias\n- `regioes_1945_canonico.ron`: {} regioes\n- `territorio_dados/estados.ron`: {} estados, {} adm1 referenciados\n\n\
## Divergencias\n\n",
        prov.len(), regs.len(), n, est.len());
    md += &lista("regioes_1945 ausentes em provinces.ron (esperado 0)", d1);
    md += &lista("estados.ron ausentes em provinces.ron (esperado 0)", d3);
    md += &lista("regioes_1945 nao cobertas por nenhum estado", d4);
    md += &lista("provinces.ron sem regiao em regioes_1945", d2);
    md += "## Conclusao\n\nSe as duas primeiras listas estao em 0, `provinces.ron` usa os mesmos `adm1_code` do Natural Earth que os dados de 1945; nao e necessaria tabela `id_u32 -> adm1_code` separada (o proprio `provinces.ron` ja e essa tabela: `id` -> `adm1_code`).\n";
    if std::env::var_os("GF_ESCREVER_RELATORIO").is_some() {
        std::fs::create_dir_all(raiz().join("docs")).unwrap();
        std::fs::write(raiz().join("docs/gf_mapa_conferencia.md"), md).unwrap();
    }
}
