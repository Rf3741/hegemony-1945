//! Tratamento de erros e validações para o crate `sim_territorio`.

use std::path::PathBuf;
use thiserror::Error;

/// Tipo de resultado padrão do `sim_territorio`.
pub type ResultadoTerritorio<T> = Result<T, ErroTerritorio>;

/// Enumeração de todos os erros possíveis na simulação territorial.
#[derive(Debug, Error)]
pub enum ErroTerritorio {
    /// Falha de entrada/saída no sistema de arquivos.
    #[error("falha de E/S no caminho '{caminho}': {origem}")]
    Io {
        caminho: PathBuf,
        #[source]
        origem: std::io::Error,
    },

    /// Erro de sintaxe ou desserialização em arquivo RON.
    #[error("erro ao desserializar RON em '{contexto}': {origem}")]
    DesserializacaoRon {
        contexto: String,
        #[source]
        origem: ron::error::SpannedError,
    },

    /// Erro de serialização para formato RON.
    #[error("erro ao serializar RON em '{contexto}': {origem}")]
    SerializacaoRon {
        contexto: String,
        #[source]
        origem: ron::Error,
    },

    /// Identificador fora do padrão snake_case ASCII estrito.
    #[error(
        "identificador inválido '{id}' em {entidade}: use apenas snake_case ASCII (a-z, 0-9, '_')"
    )]
    IdentificadorInvalido { entidade: &'static str, id: String },

    /// Tag de país inválida (deve ter exatamente 3 letras maiúsculas ASCII).
    #[error(
        "tag de país inválida '{tag}': esperava exatamente 3 letras maiúsculas ASCII (ex: 'USA', 'SOV', 'BRA')"
    )]
    TagInvalida { tag: String },

    /// Código de região Natural Earth 10m adm1_code inválido.
    #[error("código adm1_code inválido '{codigo}': formato esperado ex: 'BRA-1311', 'USA-3521'")]
    Adm1CodeInvalido { codigo: String },

    /// Identificador duplicado encontrado em um catálogo ou cenário.
    #[error("identificador duplicado '{id}' em {entidade}")]
    IdentificadorDuplicado { entidade: &'static str, id: String },

    /// Referência a uma entidade que não existe.
    #[error("{origem_entidade} '{origem_id}' referencia {alvo_entidade} inexistente '{alvo_id}'")]
    ReferenciaInexistente {
        origem_entidade: &'static str,
        origem_id: String,
        alvo_entidade: &'static str,
        alvo_id: String,
    },

    /// Parâmetro numérico ou estado inválido.
    #[error("valor inválido em {entidade} '{id}': {detalhe}")]
    ValorInvalido {
        entidade: &'static str,
        id: String,
        detalhe: String,
    },

    /// Capacidade de infraestrutura ou suporte territorial excedida.
    #[error(
        "capacidade excedida no estado '{estado_id}' para modal '{modal}': disponível {disponivel:.1}, demandado {demandado:.1}"
    )]
    CapacidadeExcedida {
        estado_id: String,
        modal: String,
        disponivel: f64,
        demandado: f64,
    },

    /// Invariante de integridade territorial violada.
    #[error("invariante territorial violada em {contexto}: {detalhe}")]
    InvarianteViolada { contexto: String, detalhe: String },
}

/// Valida se uma string é um identificador `snake_case` ASCII estrito:
/// - Não vazia
/// - Começa com letra minúscula ASCII (`a-z`)
/// - Contém apenas letras minúsculas ASCII, dígitos e sublinhados
/// - Não termina com sublinhado
/// - Não possui sublinhados duplos (`__`)
pub fn validar_id_snake_case(entidade: &'static str, id: &str) -> ResultadoTerritorio<()> {
    let bytes = id.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_lowercase()
        || bytes[bytes.len() - 1] == b'_'
        || id.contains("__")
        || !bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
    {
        return Err(ErroTerritorio::IdentificadorInvalido {
            entidade,
            id: id.to_string(),
        });
    }
    Ok(())
}

/// Valida se uma tag de país possui exatamente 3 letras maiúsculas ASCII (`[A-Z]{3}`).
pub fn validar_tag_pais(tag: &str) -> ResultadoTerritorio<()> {
    let bytes = tag.as_bytes();
    if bytes.len() == 3 && bytes.iter().all(|b| b.is_ascii_uppercase()) {
        Ok(())
    } else {
        Err(ErroTerritorio::TagInvalida {
            tag: tag.to_string(),
        })
    }
}

/// Valida se um código de região adm1_code é válido (não vazio, ASCII imprimível sem espaços).
pub fn validar_adm1_code(codigo: &str) -> ResultadoTerritorio<()> {
    if codigo.trim().is_empty()
        || codigo.contains(' ')
        || !codigo.is_ascii()
        || codigo.len() < 3
        || codigo.len() > 32
    {
        return Err(ErroTerritorio::Adm1CodeInvalido {
            codigo: codigo.to_string(),
        });
    }
    Ok(())
}
