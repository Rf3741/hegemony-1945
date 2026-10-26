use thiserror::Error;

/// Erros de domínio e serialização da crate `simulacao`.
#[derive(Debug, Error)]
pub enum ErroSimulacao {
    #[error("erro de leitura/escrita de arquivo ({caminho}): {fonte}")]
    Io {
        caminho: String,
        #[source]
        fonte: std::io::Error,
    },

    #[error("erro ao desserializar RON ({contexto}): {fonte}")]
    RonDe {
        contexto: String,
        #[source]
        fonte: Box<ron::error::SpannedError>,
    },

    #[error("erro ao serializar RON ({contexto}): {fonte}")]
    RonSer {
        contexto: String,
        #[source]
        fonte: ron::Error,
    },

    #[error("inconsistência de dados do cenário: {0}")]
    CenarioInvalido(String),
}

pub type ResultadoSimulacao<T> = Result<T, ErroSimulacao>;
