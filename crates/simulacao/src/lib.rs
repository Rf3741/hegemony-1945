//! # `simulacao` — Motor Puro da Guerra Fria (1945+)
//!
//! Crate de biblioteca Rust **sem dependência do Bevy**, responsável pela lógica determinística de:
//! - Calendário e controle de velocidades (`tempo`);
//! - Projeção cartográfica de Miller, codificação RGB bijetiva de províncias e esquema do cenário 1945 (`mapa`);
//! - Pops, 26 bens, 23 prédios, mercados por oferta/demanda, impostos, orçamento e dívida (`economia`);
//! - 7 categorias institucionais de leis, 11 grupos de interesse, legitimidade, eleições e esferas EUA/URSS (`politica`);
//! - Orquestração diária do mundo (`mundo`);
//! - Concordância de artigo com o nome dos países das mensagens (`artigo`).

pub mod artigo;
pub mod divergencia;
pub mod economia;
pub mod erro;
pub mod formacao;
pub mod mapa;
pub mod mundo;
pub mod politica;
pub mod tempo;

pub use divergencia::{DesfechoGuerraFria, EventoDivergencia, MotorDivergencia, RngDeterministico};

pub use economia::{
    BemId, Estrato, Mercado, NivelImposto, OrcamentoNacional, Pop, Predio, Profissao, TipoPredio,
    calcular_penalidade_escassez, calcular_preco_alvo_mercado,
};
pub use erro::{ErroSimulacao, ResultadoSimulacao};
pub use mapa::{
    ALTURA_MAPA_PADRAO, AlinhamentoGeopolitico, COR_OCEANO, CatalogoProvincias, DefinicaoPais1945,
    DonosProvincias1945, LARGURA_MAPA_PADRAO, ProvinciaMapa, RegrasCenario1945,
    cor_unica_provincia, desprojetar_miller, id_de_cor_provincia, projetar_miller,
};
pub use mundo::{EstadoPais, MundoJogo};
pub use politica::{
    CategoriaLei, EsferaInfluencia, EstadoGrupoInteresse, EstadoPolitico, GrupoInteresseId,
    ReformaLeiAtiva, construir_politica_inicial,
};
pub use tempo::{DataSimulacao, RelogioSimulacao, VelocidadeJogo};
