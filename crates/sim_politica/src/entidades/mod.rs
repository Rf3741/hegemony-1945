//! Entidades dinâmicas do mundo político: Pops, Grupos de Interesse, Partidos, Líderes,
//! Países, Eleições, Movimentos e Crises.

pub mod crise;
pub mod eleicao;
pub mod grupo_interesse;
pub mod influencia;
pub mod lei_tramitacao;
pub mod lider;
pub mod movimento;
pub mod pais;
pub mod partido;
pub mod pop;

pub use crise::{CriseInterna, SinalGuerraCivil};
pub use eleicao::{CampanhaEleitoral, ResultadoEleicao};
pub use grupo_interesse::{InstanciaGrupoInteresse, ModificadorTemporario};
pub use influencia::{OperacaoInfluencia, RegistroInfluenciaExterna};
pub use lei_tramitacao::TramitacaoLei;
pub use lider::LiderPolitico;
pub use movimento::{MovimentoPolitico, ObjetivoMovimento};
pub use pais::PaisPolitica;
pub use partido::InstanciaPartido;
pub use pop::PopPolitica;
