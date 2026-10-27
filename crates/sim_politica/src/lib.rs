//! # `sim_politica`
//!
//! Biblioteca Rust pura (sem dependência do Bevy) para simulação política refinada
//! estilo *Victoria 3* ambientada no cenário da Guerra Fria a partir de `01/01/1945`.
//!
//! ## Principais Recursos e Mecânicas
//! - **Força Política das Pops e Clout**: Cálculo determinístico per capita
//!   (`riqueza × alfabetização × leis de sufrágio`) gerando a influência política de cada GI;
//! - **Grupos de Interesse (GIs)**: No governo, na oposição ou marginalizados, com cálculo
//!   contínuo de aprovação (`-20..=+20`), bens de interesse econômico, humor (`Feliz`, `Irritado`, `Radical`)
//!   e traços leais/insurrectos;
//! - **Legitimidade Governamental (`0..=100`)**: Composição da coalizão, partido do chefe de Estado,
//!   coerência ideológica interministerial e representação parlamentar;
//! - **Partidos Dinâmicos e Eleições Periódicas**: Formação dinâmica de partidos por afinidade
//!   doutrinária (modulada pela ideologia dos líderes), campanha, momento eleitoral, sufrágio por pop
//!   e alocação de cadeiras por D'Hondt ou lista de vanguarda;
//! - **Processo Legislativo Realista**: Tramitação diária por fases (`Introducao`, `Comite`, `VotacaoFinal`),
//!   probabilidades de avanço, debate parlamentar e cancelamento após 3 entraves;
//! - **Movimentos Sociais e Radicalismo**: Pops (`radicais`, `leais`, `neutros`), pressão de movimentos
//!   reformistas/reacionários, greves, conspirações de golpe militar e termômetro revolucionário rumo à guerra civil;
//! - **Sinal de Guerra Civil**: Emissão de contrato estruturado (`SinalGuerraCivil`) para o futuro motor `sim_guerra`;
//! - **Influência Externa da Guerra Fria**: Superpotências (EUA e URSS) disputando esferas de influência,
//!   financiamento clandestino de GIs e partidos, propaganda ideológica e ajuda econômica (Plano Marshall / COMECON).

#![allow(clippy::result_large_err)]

pub mod catalogo;
pub mod cenario;
pub mod entidades;
pub mod erro;
pub mod motor;
pub mod rng;
pub mod tipos;

pub use catalogo::{
    ArquetipoPartido, BaseSocialPop, CatalogoPolitica, CategoriaLeis, DefinicaoGrupoInteresse,
    DefinicaoLei, Ideologia, InteresseBem, Modificador, PreRequisitos,
};
pub use cenario::CenarioPolitica;
pub use entidades::{
    CampanhaEleitoral, CriseInterna, InstanciaGrupoInteresse, InstanciaPartido, LiderPolitico,
    ModificadorTemporario, MovimentoPolitico, ObjetivoMovimento, OperacaoInfluencia, PaisPolitica,
    PopPolitica, RegistroInfluenciaExterna, ResultadoEleicao, SinalGuerraCivil, TramitacaoLei,
};
pub use erro::{ErroPolitica, ResultadoPolitica, validar_id_snake_case};
pub use motor::{MundoPolitico, RelatorioTickPolitico};
pub use rng::RngDeterministico;
pub use tipos::{
    AlinhamentoBloco, DataJogo, Estrato, FaixaLegitimidade, FaseLegislativa, HumorGrupoInteresse,
    NivelEsfera, OrigemGuerraCivil, PapelGrupoInteresse, PosturaLei, PotenciaBloco,
    ResultadoCheckpointLei, StatusClout, TipoCrisePolitica, TipoSistemaRepresentacao,
};
