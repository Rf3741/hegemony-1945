//! Subdivisao de estados (admin-1) em provincias: contagens, hierarquia e bitmap.

use gf_mapa::{CatalogoProvincias, id_de_cor_provincia};
use std::collections::HashSet;
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn ler(rel: &str) -> CatalogoProvincias {
    ron::from_str(&std::fs::read_to_string(raiz().join(rel)).unwrap()).unwrap()
}

#[test]
fn contagens_e_estado_valido() {
    let c = ler("assets/map/provinces.ron");
    let estados = ler("data/estados_mapa/provinces.ron");
    assert!(c.provincias.len() > 6000, "poucas provincias: {}", c.provincias.len());
    assert!(c.provincias.len() < 65000);
    let validos: HashSet<&str> = estados.provincias.iter().map(|e| e.adm1_code.as_str()).collect();
    for p in &c.provincias {
        assert!(validos.contains(p.adm1_code.as_str()), "estado invalido {}", p.adm1_code);
        assert!(!p.estado.is_empty() && !p.nome.is_empty());
    }
    // todo estado tem ao menos uma provincia
    let com: HashSet<&str> = c.provincias.iter().map(|p| p.adm1_code.as_str()).collect();
    assert_eq!(com.len(), validos.len());
    let bra = c.provincias.iter().filter(|p| p.adm0_moderno == "BRA").count();
    assert!(bra > 100 && bra < 400, "Brasil: {bra}");
}

#[test]
fn nenhum_pixel_sem_dono() {
    let c = ler("assets/map/provinces.ron");
    let img = image::open(raiz().join("assets/map/provinces.png")).unwrap().to_rgb8();
    assert_eq!(img.dimensions(), (c.largura, c.altura));
    let ids: HashSet<u32> = c.provincias.iter().map(|p| p.id).collect();
    let mut com_pixel = HashSet::new();
    let mut terra = 0u64;
    for px in img.pixels() {
        let cor = (px[0], px[1], px[2]);
        if cor == (0, 0, 0) {
            continue;
        }
        let id = id_de_cor_provincia(cor).expect("pixel com cor invalida (sem provincia)");
        assert!(ids.contains(&id), "pixel de id inexistente {id}");
        com_pixel.insert(id);
        terra += 1;
    }
    assert!(terra > 1_000_000);
    // quase toda provincia tem pixels (estados minusculos sem pixel no mapa de estados sao mantidos sem territorio)
    assert!(com_pixel.len() * 100 > ids.len() * 90, "{} de {}", com_pixel.len(), ids.len());
    for p in &c.provincias {
        if com_pixel.contains(&p.id) {
            let cor = img.get_pixel(p.centroide_px.0, p.centroide_px.1);
            assert_eq!(id_de_cor_provincia((cor[0], cor[1], cor[2])), Some(p.id), "centroide_px fora da provincia {}", p.id);
        }
    }
}
