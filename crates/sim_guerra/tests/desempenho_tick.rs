//! Regressão de determinismo e micro-benchmark do tick diário de guerra.
//!
//! O hash gravado foi obtido ANTES das otimizações de suprimento; qualquer mudança no
//! resultado da simulação quebra `estado_apos_n_dias_igual_ao_gravado`.

use sim_guerra::{CatalogoGuerra, MundoGuerra, ParametrosCenarioSintetico, gerar_cenario_sintetico};
use std::path::Path;
use std::time::Instant;

fn catalogo() -> CatalogoGuerra {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    CatalogoGuerra::carregar_diretorio(&dir).expect("catálogo deve carregar")
}

fn fnv(s: &str, mut h: u64) -> u64 {
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Hash determinístico de todo o estado dinâmico relevante.
fn hash_estado(m: &MundoGuerra) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for r in m.regioes() {
        h = fnv(&format!("{:?}", r), h);
    }
    for d in m.divisoes() {
        h = fnv(&format!("{:?}", d), h);
    }
    for p in m.paises() {
        h = fnv(&format!("{:?}", p), h);
    }
    for g in m.guerras() {
        h = fnv(&format!("{:?}", g), h);
    }
    for f in m.frentes() {
        h = fnv(&format!("{:?}", f), h);
    }
    h = fnv(&format!("{:?}", m.cenario.data_inicial), h);
    h
}

fn rodar(params: ParametrosCenarioSintetico, dias: usize) -> u64 {
    let mut m = MundoGuerra::novo(gerar_cenario_sintetico(catalogo(), params)).unwrap();
    for _ in 0..dias {
        m.executar_tick_diario();
    }
    hash_estado(&m)
}

#[test]
fn estado_apos_n_dias_igual_ao_gravado() {
    // Com guerra
    let com_guerra = rodar(
        ParametrosCenarioSintetico {
            num_regioes: 400,
            num_paises: 12,
            num_guerras: 2,
            divisoes_por_pais: 8,
            seed: 19450815,
        },
        120,
    );
    // Em paz (caso do início de 1945)
    let em_paz = rodar(
        ParametrosCenarioSintetico {
            num_regioes: 400,
            num_paises: 12,
            num_guerras: 0,
            divisoes_por_pais: 8,
            seed: 7,
        },
        120,
    );
    eprintln!("hash_com_guerra={com_guerra:#x} hash_em_paz={em_paz:#x}");
    assert_eq!(com_guerra, GRAVADO_COM_GUERRA, "estado com guerra divergiu");
    assert_eq!(em_paz, GRAVADO_EM_PAZ, "estado em paz divergiu");
}

const GRAVADO_COM_GUERRA: u64 = 0x536d1488c920f08b;
const GRAVADO_EM_PAZ: u64 = 0x3fc542e37801eb12;

#[test]
#[ignore]
fn bench_tick_diario_1945() {
    // Escala aproximada do jogo: ~12300 províncias, ~138 países, ~10 divisões por país, sem guerra.
    let params = ParametrosCenarioSintetico {
        num_regioes: 12300,
        num_paises: 138,
        num_guerras: 0,
        divisoes_por_pais: 10,
        seed: 1945,
    };
    let mut m = MundoGuerra::novo(gerar_cenario_sintetico(catalogo(), params)).unwrap();
    for _ in 0..5 {
        m.executar_tick_diario();
    }
    let dias = 200;
    let t = Instant::now();
    for _ in 0..dias {
        m.executar_tick_diario();
    }
    let por_dia = t.elapsed().as_secs_f64() * 1000.0 / dias as f64;
    println!("BENCH tick_diario (paz, 12300 regioes, 138 paises): {por_dia:.3} ms/dia");
}
