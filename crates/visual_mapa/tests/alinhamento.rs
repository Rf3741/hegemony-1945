//! Alinhamento: as cidades projetadas devem cair em terra no `provinces.png` do jogo.
use visual_mapa::ProjecaoMapa;
use visual_mapa::dados::{CidadeDados, ler_ron, raiz_dados_padrao};

#[test]
fn cidades_caem_em_terra_no_provinces_png() {
    let caminho = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/map/provinces.png");
    let img = image::open(caminho).expect("provinces.png").to_rgba8();
    let proj = ProjecaoMapa { largura: img.width(), altura: img.height() };
    assert_eq!((img.width(), img.height()), (5632, 2048));

    // Capitais/metrópoles conhecidas: devem cair em pixel que não é oceano (preto).
    let conhecidas = [
        ("Moscou", 37.62, 55.75), ("Paris", 2.35, 48.86), ("Berlim", 13.40, 52.52),
        ("Washington", -77.04, 38.91), ("Sao Paulo", -46.63, -23.55), ("Delhi", 77.21, 28.61),
        ("Pequim", 116.40, 39.90), ("Cairo", 31.24, 30.04), ("Mexico", -99.13, 19.43),
        ("Nairobi", 36.82, -1.29),
    ];
    for (nome, lon, lat) in conhecidas {
        let (px, py) = proj.mundo_para_pixel(proj.geo_para_mundo(lon, lat));
        let p = img.get_pixel(px as u32, py as u32).0;
        assert!(p[0] != 0 || p[1] != 0 || p[2] != 0, "{nome} caiu no oceano em ({px},{py})");
    }

    // Em massa: a grande maioria das cidades de demonstração fica em terra (algumas costeiras
    // podem cair em pixel de mar por causa da resolução).
    let cidades: Vec<CidadeDados> = ler_ron(&raiz_dados_padrao().join("demo/cidades.ron")).unwrap();
    let em_terra = cidades
        .iter()
        .filter(|c| {
            let (px, py) = proj.mundo_para_pixel(proj.geo_para_mundo(c.posicao.0, c.posicao.1));
            let p = img.get_pixel((px as u32).min(img.width() - 1), (py as u32).min(img.height() - 1)).0;
            p[0] != 0 || p[1] != 0 || p[2] != 0
        })
        .count();
    let frac = em_terra as f32 / cidades.len() as f32;
    println!("cidades em terra: {em_terra}/{} ({:.1}%)", cidades.len(), frac * 100.0);
    assert!(frac > 0.85, "so {:.1}% das cidades caem em terra", frac * 100.0);
}
