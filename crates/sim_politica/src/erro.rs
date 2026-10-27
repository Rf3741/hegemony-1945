//! Tipos de erro e validações do motor político `sim_politica`.

use std::path::PathBuf;
use thiserror::Error;

/// Tipo padrão de resultado para operações políticas.
pub type ResultadoPolitica<T> = Result<T, ErroPolitica>;

/// Categorias de erro da simulação política.
#[derive(Debug, Error)]
pub enum ErroPolitica {
    /// Falha de leitura ou escrita em arquivo.
    #[error("falha de E/S no caminho '{caminho}': {origem}")]
    Io {
        caminho: PathBuf,
        #[source]
        origem: std::io::Error,
    },

    /// Erro de desserialização RON.
    #[error("erro ao ler RON em '{contexto}': {origem}")]
    DesserializacaoRon {
        contexto: String,
        #[source]
        origem: ron::error::SpannedError,
    },

    /// Erro de serialização RON.
    #[error("erro ao serializar RON em '{contexto}': {origem}")]
    SerializacaoRon {
        contexto: String,
        #[source]
        origem: ron::Error,
    },

    /// Identificador que não segue a convenção `snake_case` ASCII.
    #[error(
        "identificador inválido '{id}' em {entidade}: use apenas snake_case ASCII (a-z, 0-9, '_')"
    )]
    IdentificadorInvalido { entidade: &'static str, id: String },

    /// Dois registros usaram o mesmo identificador.
    #[error("identificador duplicado '{id}' em {entidade}")]
    IdentificadorDuplicado { entidade: &'static str, id: String },

    /// Referência a uma entidade inexistente.
    #[error("{origem_entidade} '{origem_id}' referencia {alvo_entidade} inexistente '{alvo_id}'")]
    ReferenciaInexistente {
        origem_entidade: &'static str,
        origem_id: String,
        alvo_entidade: &'static str,
        alvo_id: String,
    },

    /// Valor numérico inválido ou fora dos limites de domínio.
    #[error("valor inválido em {entidade} '{id}': {detalhe}")]
    ValorInvalido {
        entidade: &'static str,
        id: String,
        detalhe: String,
    },

    /// Ação política ilegal no estado atual.
    #[error("ação política inválida para o país '{pais_id}': {detalhe}")]
    AcaoInvalida { pais_id: String, detalhe: String },

    /// Violação de invariante de simulação política.
    #[error("invariante política violada em {contexto}: {detalhe}")]
    InvarianteViolada { contexto: String, detalhe: String },
}

/// Valida se uma string segue estritamente o formato `snake_case` ASCII:
/// não vazia, inicia com `[a-z]`, termina com `[a-z0-9]`, contém apenas `[a-z0-9_]`,
/// e não possui caracteres sublinhados consecutivos `"__"`.
pub fn validar_id_snake_case(entidade: &'static str, id: &str) -> ResultadoPolitica<()> {
    let bytes = id.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_lowercase()
        || bytes[bytes.len() - 1] == b'_'
        || id.contains("__")
        || !bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
    {
        return Err(ErroPolitica::IdentificadorInvalido {
            entidade,
            id: id.to_string(),
        });
    }
    Ok(())
}
