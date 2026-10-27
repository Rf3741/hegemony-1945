use gf_mapa::*;

#[test]
fn miller_ida_e_volta() {
    for (lon, lat) in [(0.0, 0.0), (-46.6, -23.5), (139.7, 35.7), (170.0, 70.0)] {
        let (x, y) = projetar_miller(lon, lat, LARGURA_MAPA_PADRAO, ALTURA_MAPA_PADRAO);
        let (lo, la) = desprojetar_miller(x, y, LARGURA_MAPA_PADRAO, ALTURA_MAPA_PADRAO);
        assert!((lo - lon).abs() < 1e-6 && (la - lat).abs() < 1e-6);
    }
    let (x, y) = projetar_miller(0.0, 0.0, 1000, 500);
    assert!((x - 500.0).abs() < 1e-9 && (y - 250.0).abs() < 1e-9);
}

#[test]
fn cor_bijetiva_e_oceano() {
    assert_eq!(id_de_cor_provincia(COR_OCEANO), None);
    for id in 1..=5000u32 {
        assert_eq!(id_de_cor_provincia(cor_unica_provincia(id)), Some(id));
    }
}

#[test]
fn calendario() {
    let mut d = DataSimulacao::INICIO_1945;
    assert_eq!(d.formatar_pt_br(), "1º de janeiro de 1945");
    d.avancar_dia();
    assert_eq!(d.dia, 2);
    assert!(DataSimulacao::ano_bissexto(1948));
    assert_eq!(DataSimulacao::dias_no_mes(1948, 2), 29);
}
