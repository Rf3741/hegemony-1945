//! # `sim_territorio`
//!
//! Crate Rust puro (sem dependência do Bevy) para simulação de hierarquia territorial,
//! desenvolvimento urbano e industrial, eras tecnológicas visuais, infraestrutura
//! e descolonização para grande estratégia da Guerra Fria a partir de `01/01/1945`.
//!
//! ## Os 5 Pilares do Sistema:
//! 1. **Hierarquia Territorial e Transferência de Soberania**:
//!    - 3 Níveis espaciais: `Regiao` (Natural Earth 10m `adm1_code`) → `Estado` → `RegiaoEstrategica`.
//!    - Soberania de jure (`dono_tag`), controle de facto (`controlador_tag`), núcleos (`nucleos`) e reivindicações (`reivindicacoes`).
//!    - Transferência territorial com efeitos profundos nas pops: recálculo de cidadania,
//!      discriminação cultural, fuga de refugiados, migração voluntária e expulsões compulsórias.
//! 2. **Desenvolvimento Urbano e Crescimento de Cidades**:
//!    - Cidades em cada estado cujo tamanho cresce dinamicamente conforme população urbana,
//!      empregos industriais e de serviços, e conectividade de infraestrutura.
//!    - Urbanização calculada organicamente a partir dos prédios construídos (estilo Victoria 3).
//!    - `NivelCidade` (escala discreta de 1 a 8 com histerese) e `NivelIndustrial` (0 a 8) por estado,
//!      acompanhados de índices contínuos suavizados para renderização visual no mapa.
//! 3. **Eras Tecnológicas Visuais (1945, 1955, 1965, 1975, 1985)**:
//!    - Derivadas das tecnologias descobertas e dos métodos de produção adotados nos prédios.
//!    - Cálculo em dois níveis: Era Nacional do País e Era Local do Estado, permitindo que
//!      polos industriais avancem visualmente antes de áreas agrícolas atrasadas.
//! 4. **Infraestrutura com Capacidade e Demanda de Uso**:
//!    - 5 modais integrados: `estradas`, `ferrovias`, `portos`, `aeroportos` e `rede_eletrica`.
//!    - Capacidade efetiva (modulada por terreno, era e dano) vs. uso real demandado.
//!    - Cálculo dinâmico de `acesso_mercado`, `cobertura_eletrica` e limite de suprimento militar.
//! 5. **Descolonização e Independência com Secessão de Estados**:
//!    - Fervor anticolonial (0 a 100), custo de manutenção imperial e atrito de extração colonial.
//!    - 3 caminhos históricos: independência negociada, partição pós-colonial e guerras de libertação.

#![allow(clippy::result_large_err, clippy::collapsible_if)]

pub mod catalogo;
pub mod descolonizacao;
pub mod entidades;
pub mod eras;
pub mod erro;
pub mod infraestrutura;
pub mod mundo;
pub mod tipos;
pub mod transferencia;
pub mod urbanizacao;

// Re-exports ergonômicos
pub use catalogo::{
    CatalogoTerritorio, ClassePredioUrbano, Cultura, DefinicaoMetodoEra, DefinicaoPredioUrbano,
    DefinicaoTecnologiaEra, PilarTecnologico, ler_arquivo_ron, ler_str_ron,
};
pub use descolonizacao::{
    RelatorioDescolonizacao, atualizar_fervor_diario, conceder_independencia_negociada,
    custo_manutencao_colonial, eficiencia_extrativa_colonial, executar_particao_colonial,
    proclamar_secessao_armada, vitoria_guerra_libertacao,
};
pub use entidades::{
    Estado, MovimentoIndependencia, PaisTerritorial, PopRegional, Regiao, Regiao1945,
    RegiaoEstrategica, Reivindicacao,
};
pub use eras::{
    calcular_era_visual_estado, calcular_era_visual_pais, determinar_estilo_arquitetonico,
};
pub use erro::{
    ErroTerritorio, ResultadoTerritorio, validar_adm1_code, validar_id_snake_case, validar_tag_pais,
};
pub use infraestrutura::{
    CAPACIDADE_BASE_AEROPORTOS, CAPACIDADE_BASE_ESTRADAS, CAPACIDADE_BASE_FERROVIAS,
    CAPACIDADE_BASE_PORTOS, CAPACIDADE_BASE_REDE_ELETRICA, EstadoModalInfra,
    RedeInfraestruturaEstado,
};
pub use mundo::{CenarioTerritorio, MundoTerritorial};
pub use tipos::{
    Continente, DataJogo, EraTecnologicaVisual, EstiloArquitetonico, LeiCidadania,
    ModoTransferenciaTerritorial, NivelCidade, NivelIndustrial, SetorEmpregoPop, StatusCulturalPop,
    TipoInfraestrutura, TipoReivindicacao, TipoTerreno, ViaDescolonizacao,
};
pub use urbanizacao::{Cidade, DescritorVisualEstado, EstadoUrbano};
