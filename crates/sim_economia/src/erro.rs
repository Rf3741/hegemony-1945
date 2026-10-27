//! Tipos de erro do motor econômico `sim_economia`.

use std::path::PathBuf;
use thiserror::Error;

/// Resultado padrão das operações de carregamento, validação e simulação econômica.
pub type ResultadoEconomia<T> = Result<T, ErroEconomia>;

/// Erros de carregamento RON, validação de esquema e invariantes de simulação.
#[derive(Debug, Error)]
pub enum ErroEconomia {
    /// Falha de leitura ou escrita em arquivo de dados.
    #[error("falha de E/S no caminho '{caminho}': {origem}")]
    Io {
        caminho: PathBuf,
        #[source]
        origem: std::io::Error,
    },

    /// Erro ao desserializar um arquivo ou string no formato RON.
    #[error("erro ao ler RON em '{contexto}': {origem}")]
    DesserializacaoRon {
        contexto: String,
        #[source]
        origem: Box<ron::error::SpannedError>,
    },

    /// Erro ao serializar dados para o formato RON.
    #[error("erro ao serializar RON em '{contexto}': {origem}")]
    SerializacaoRon {
        contexto: String,
        #[source]
        origem: Box<ron::Error>,
    },

    /// Identificador fora do padrão `snake_case` ASCII.
    #[error(
        "identificador inválido '{id}' em {entidade}: use apenas snake_case ASCII (a-z, 0-9, '_')"
    )]
    IdentificadorInvalido { entidade: &'static str, id: String },

    /// Dois registros do mesmo tipo usaram o mesmo identificador.
    #[error("identificador duplicado '{id}' em {entidade}")]
    IdentificadorDuplicado { entidade: &'static str, id: String },

    /// Uma entidade referenciou um identificador que não existe no catálogo ou cenário.
    #[error("{origem_entidade} '{origem_id}' referencia {alvo_entidade} inexistente '{alvo_id}'")]
    ReferenciaInexistente {
        origem_entidade: &'static str,
        origem_id: String,
        alvo_entidade: &'static str,
        alvo_id: String,
    },

    /// Parâmetro numérico ou regra de domínio inválida nos dados de entrada.
    #[error("valor inválido em {entidade} '{id}': {detalhe}")]
    ValorInvalido {
        entidade: &'static str,
        id: String,
        detalhe: String,
    },

    /// Limite físico da província (terra arável ou recurso natural) insuficiente para construção.
    #[error(
        "capacidade excedida na província '{provincia_id}' para '{recurso}': disponível {disponivel}, necessário {necessario}"
    )]
    CapacidadeExcedida {
        provincia_id: String,
        recurso: String,
        disponivel: u32,
        necessario: u32,
    },

    /// Violação de invariante numérica ou econômica durante auditoria de estabilidade.
    #[error("invariante econômica violada em {contexto}: {detalhe}")]
    InvarianteViolada { contexto: String, detalhe: String },
}

/// Verifica se uma string segue estritamente o formato `snake_case` ASCII:
/// não vazia, começa com `[a-z]`, contém apenas `[a-z0-9_]`, não termina com `'_'` e não possui `"__"`.
pub fn validar_id_snake_case(entidade: &'static str, id: &str) -> ResultadoEconomia<()> {
    let bytes = id.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_lowercase()
        || bytes[bytes.len() - 1] == b'_'
        || id.contains("__")
        || !bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
    {
        return Err(ErroEconomia::IdentificadorInvalido {
            entidade,
            id: id.to_string(),
        });
    }
    Ok(())
}
