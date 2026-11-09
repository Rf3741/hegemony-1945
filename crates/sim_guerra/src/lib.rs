#![allow(clippy::result_large_err, clippy::too_many_arguments)]
//! # `sim_guerra`
//!
//! Biblioteca Rust pura (sem dependência do Bevy) para simulação de guerra, frentes operacionais,
//! logística de suprimento, batalhas e tratados de paz para o cenário Guerra Fria a partir de 1945.
//!
//! ## Principais Recursos
//! - **Grafo de Regiões em CSR**: Representação compacta em memória contígua para as 4.596 regiões admin-1 do Natural Earth.
//! - **Rede de Suprimentos Logística**: Cálculo determinístico a partir de capitais e portos sobre infraestrutura e terreno.
//! - **Frentes Automáticas**: Detecção de fronteiras contíguas entre beligerantes com avanço e recuo região a região.
//! - **Combate Tático e Operacional**: Largura de combate, terreno, entrincheiramento, suprimento, moral, organização, generais e tecnologia.
//! - **Ocupação e Controle**: Separação estrita entre `dono_tag` (soberania de jure) e `controlador_tag` (controle de facto).
//! - **Pontuação e Cansaço de Guerra**: Evolução contínua baseada em território, baixas relativas e capitulações.
//! - **API de Tratados de Paz**: Anexações, mudança de regime, zonas de ocupação, reparações e armistícios.

pub mod catalogo;
pub mod cenario;
pub mod entidades;
pub mod erro;
pub mod gerador;
pub mod grafo;
pub mod motor;
pub mod rng;
pub mod tipos;

pub use catalogo::{
    CatalogoGuerra, EstatisticasDivisao, TecnologiaMilitar, TemplateDivisao, TipoTerreno,
    TipoUnidade, TracoComandante, ler_arquivo_ron, ler_str_ron,
};
pub use cenario::CenarioGuerra;
pub use entidades::{
    Comandante, ConfigGuerra, Divisao, Exercito, Frente, Guerra, PaisMilitar, RegiaoMapa,
};
pub use erro::{
    ErroGuerra, ResultadoGuerra, validar_adm1_code, validar_id_snake_case, validar_tag_pais,
};
pub use gerador::{ParametrosCenarioSintetico, gerar_cenario_sintetico};
pub use grafo::{ArestaRegiao, GrafoCompilado, VizinhoCompilado};
pub use motor::paz::{AvaliacaoPaz, PropostaPaz, TratadoPazAssinado};
pub use motor::{EstadoGuerraSalvo, MundoGuerra, RelatorioTickGuerra};
pub use rng::RngDeterministico;
pub use tipos::{ClasseUnidade, ClausulaPaz, DataJogo, PosturaOperacional, TipoResultadoBatalha};
