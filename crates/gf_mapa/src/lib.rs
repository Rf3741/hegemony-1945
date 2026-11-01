//! # `gf_mapa` — mapa e tempo do jogo Guerra Fria (sem Bevy)
//!
//! - `mapa`: projeção de Miller, cor RGB bijetiva de províncias, `CatalogoProvincias`
//!   e esquema do cenário 1945.
//! - `tempo`: calendário (`DataSimulacao`), `RelogioSimulacao` e `VelocidadeJogo`.

pub mod mapa;
pub mod tempo;

pub use mapa::{
    ALTURA_MAPA_PADRAO, ESCALA_Y_MAPA, AlinhamentoGeopolitico, COR_OCEANO, CatalogoProvincias, DefinicaoPais1945,
    DonosProvincias1945, LARGURA_MAPA_PADRAO, ProvinciaMapa, RegrasCenario1945,
    cor_unica_provincia, desprojetar_miller, id_de_cor_provincia, projetar_miller,
};
pub use tempo::{DataSimulacao, RelogioSimulacao, VelocidadeJogo};
