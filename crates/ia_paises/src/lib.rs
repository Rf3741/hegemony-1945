//! IA dos paises (estilo Victoria 3), conforme `pesquisa/IA.md`.
//!
//! - [`IaEconomia`]: postura fiscal (impostos com histerese de 90 dias), obras estatais por
//!   retorno e necessidade (complementando o setor privado) e reacao a crises (calote).
//! - [`IaPolitica`]: propoe leis conforme a ideologia dos grupos de interesse, com trava de
//!   risco de golpe.
//!
//! Tudo e deterministico (sem `HashMap`) e escalonado por `(dia + indice_do_pais) % periodo`.

mod economia;
mod politica;

pub use economia::{IaEconomia, PosturaFiscal, EstadoFiscalIa};
pub use politica::IaPolitica;
