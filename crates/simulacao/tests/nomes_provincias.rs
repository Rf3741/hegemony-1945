//! Teste dos topônimos: nenhuma província dos EUA (nem dos países principais) com nome genérico.
use simulacao::mapa::CatalogoProvincias;
use simulacao::mundo::MundoJogo;
use std::collections::BTreeMap;

/// Palavras que sozinhas não identificam um lugar ("Leste", "Ocidental", "Centro"...).
const GENERICOS: &[&str] = &[
    "Leste",
    "Oeste",
    "Norte",
    "Sul",
    "Centro",
    "Oriental",
    "Ocidental",
    "Nordeste",
    "Noroeste",
    "Sudeste",
    "Sudoeste",
];

/// "Província Ocidental 3" -> "Ocidental"; o rótulo puro é que pode ser genérico.
fn nome_base(nome: &str) -> &str {
    let nome = nome.trim();
    match nome.rsplit_once(' ') {
        Some((base, sufixo)) if !base.is_empty() && sufixo.chars().all(|c| c.is_ascii_digit()) => base,
        _ => nome,
    }
}

fn eh_generico(nome: &str) -> bool {
    GENERICOS.contains(&nome_base(nome))
}

/// Tags que aparecem nas capturas de smoke e nos painéis de país.
const PRINCIPAIS: &[&str] = &["USA", "SOV", "GBR", "FRA", "CHI"];

fn carregar() -> (CatalogoProvincias, MundoJogo) {
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    MundoJogo::carregar_de_arquivos(
        raiz.join("assets/map/provinces.ron"),
        raiz.join("data/scenario_1945/countries_1945.ron"),
        raiz.join("data/scenario_1945/province_owners_1945.ron"),
    )
    .expect("cenário 1945")
}

#[test]
fn sem_nomes_genericos_dos_paises_principais() {
    let (catalogo, mundo) = carregar();
    let nome_por_id: BTreeMap<u32, &str> =
        catalogo.provincias.iter().map(|p| (p.id, p.nome.as_str())).collect();

    let mut total_genericas = 0;
    let mut por_tag: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in &catalogo.provincias {
        if !eh_generico(&p.nome) {
            continue;
        }
        total_genericas += 1;
        let dono = mundo.dono_por_provincia.get(&p.id).cloned().unwrap_or_default();
        por_tag.entry(dono).or_default().push(p.adm1_code.clone());
    }
    eprintln!("províncias com nome genérico no catálogo: {total_genericas}");
    for tag in PRINCIPAIS {
        let lista = por_tag.get(*tag).map_or(&[][..], |v| v.as_slice());
        eprintln!("{tag}: {} genéricas {:?}", lista.len(), lista);
    }

    for (tag, p) in &mundo.paises {
        if !PRINCIPAIS.contains(&tag.as_str()) {
            continue;
        }
        for id in &p.provincias_ids {
            let nome = nome_por_id.get(id).copied().unwrap_or_default();
            assert!(!eh_generico(nome), "{tag} tem província genérica: {nome} (id {id})");
        }
    }
}
