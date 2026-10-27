//! Tratamento de erros e validações para o motor `sim_guerra`.

use std::path::PathBuf;
use thiserror::Error;

/// Tipo de retorno padronizado para operações de guerra, logística e tratados.
pub type ResultadoGuerra<T> = Result<T, ErroGuerra>;

/// Erros estruturais, de esquema, domínio e integridade de simulação em `sim_guerra`.
#[derive(Debug, Error)]
pub enum ErroGuerra {
    #[error("falha de E/S no caminho '{caminho}': {origem}")]
    Io {
        caminho: PathBuf,
        #[source]
        origem: std::io::Error,
    },

    #[error("erro ao ler RON em '{contexto}': {origem}")]
    DesserializacaoRon {
        contexto: String,
        #[source]
        origem: ron::error::SpannedError,
    },

    #[error("erro ao serializar RON em '{contexto}': {origem}")]
    SerializacaoRon {
        contexto: String,
        #[source]
        origem: ron::Error,
    },

    #[error(
        "identificador inválido '{id}' em {entidade}: use apenas snake_case ASCII (a-z, 0-9, '_')"
    )]
    IdentificadorInvalido { entidade: &'static str, id: String },

    #[error(
        "tag de país inválida '{tag}': deve conter exatamente 3 letras maiúsculas ASCII (ex: 'USA', 'SOV')"
    )]
    TagPaisInvalida { tag: String },

    #[error(
        "código adm1_code inválido '{codigo}': use identificador alfanumérico com hífen ou sublinhado"
    )]
    Adm1CodeInvalido { codigo: String },

    #[error("identificador duplicado '{id}' em {entidade}")]
    IdentificadorDuplicado { entidade: &'static str, id: String },

    #[error("{origem_entidade} '{origem_id}' referencia {alvo_entidade} inexistente '{alvo_id}'")]
    ReferenciaInexistente {
        origem_entidade: &'static str,
        origem_id: String,
        alvo_entidade: &'static str,
        alvo_id: String,
    },

    #[error("valor inválido em {entidade} '{id}': {detalhe}")]
    ValorInvalido {
        entidade: &'static str,
        id: String,
        detalhe: String,
    },

    #[error("regra de domínio violada em {contexto}: {detalhe}")]
    RegraDominioViolada { contexto: String, detalhe: String },

    #[error("invariante do grafo ou estado militar violada em {contexto}: {detalhe}")]
    InvarianteViolada { contexto: String, detalhe: String },
}

/// Valida se uma string é um identificador `snake_case` ASCII rigoroso.
pub fn validar_id_snake_case(entidade: &'static str, id: &str) -> ResultadoGuerra<()> {
    let bytes = id.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_lowercase()
        || bytes[bytes.len() - 1] == b'_'
        || id.contains("__")
        || !bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
    {
        return Err(ErroGuerra::IdentificadorInvalido {
            entidade,
            id: id.to_string(),
        });
    }
    Ok(())
}

/// Valida se uma tag de país possui exatamente 3 letras maiúsculas ASCII (ex: "USA", "SOV").
pub fn validar_tag_pais(tag: &str) -> ResultadoGuerra<()> {
    let bytes = tag.as_bytes();
    if bytes.len() != 3 || !bytes.iter().all(|b| b.is_ascii_uppercase()) {
        return Err(ErroGuerra::TagPaisInvalida {
            tag: tag.to_string(),
        });
    }
    Ok(())
}

/// Valida formato de código de região admin-1 (como "USA-3521", "SOV-1689", "regiao_01").
pub fn validar_adm1_code(codigo: &str) -> ResultadoGuerra<()> {
    if codigo.is_empty()
        || !codigo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ErroGuerra::Adm1CodeInvalido {
            codigo: codigo.to_string(),
        });
    }
    Ok(())
}
