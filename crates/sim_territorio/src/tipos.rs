//! Tipos fundamentais, enums de domínio, eras tecnológicas e calendários.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Data do calendário histórico do jogo (cenário iniciando em 01/01/1945).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DataJogo {
    pub ano: u16,
    pub mes: u8,
    pub dia: u8,
}

impl Default for DataJogo {
    fn default() -> Self {
        Self {
            ano: 1945,
            mes: 1,
            dia: 1,
        }
    }
}

impl DataJogo {
    /// Cria uma nova data validada do calendário gregoriano.
    pub fn nova(ano: u16, mes: u8, dia: u8) -> Option<Self> {
        if !(1..=12).contains(&mes) {
            return None;
        }
        let max_dias = Self::dias_no_mes(ano, mes);
        if !(1..=max_dias).contains(&dia) {
            return None;
        }
        Some(Self { ano, mes, dia })
    }

    /// Retorna se o ano é bissexto.
    #[inline]
    pub const fn eh_bissexto(ano: u16) -> bool {
        (ano.is_multiple_of(4) && !ano.is_multiple_of(100)) || ano.is_multiple_of(400)
    }

    /// Retorna a quantidade de dias no mês.
    #[inline]
    pub const fn dias_no_mes(ano: u16, mes: u8) -> u8 {
        match mes {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::eh_bissexto(ano) {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    /// Avança exatamente 1 dia no calendário.
    #[inline]
    pub fn avancar_dia(&mut self) {
        let max = Self::dias_no_mes(self.ano, self.mes);
        if self.dia < max {
            self.dia += 1;
        } else {
            self.dia = 1;
            if self.mes < 12 {
                self.mes += 1;
            } else {
                self.mes = 1;
                self.ano += 1;
            }
        }
    }
}

impl fmt::Display for DataJogo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}/{:02}/{:04}", self.dia, self.mes, self.ano)
    }
}

/// Continentes mundiais para agregação geopolítica e descolonização.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Continente {
    AmericaNorte,
    AmericaSul,
    Europa,
    Asia,
    Africa,
    Oceania,
    OrienteMedio,
}

impl Continente {
    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::AmericaNorte => "América do Norte",
            Self::AmericaSul => "América do Sul",
            Self::Europa => "Europa",
            Self::Asia => "Ásia",
            Self::Africa => "África",
            Self::Oceania => "Oceania",
            Self::OrienteMedio => "Oriente Médio",
        }
    }
}

/// Tipo predominante de terreno físico de uma região cartográfica.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum TipoTerreno {
    #[default]
    Planicie,
    Colinas,
    Montanhas,
    Floresta,
    SelvaTropical,
    Deserto,
    Tundra,
    Pantano,
}

impl TipoTerreno {
    /// Fator de custo e atrito de construção de infraestrutura (1.0 = plano normal).
    #[inline]
    pub const fn modificador_infraestrutura(self) -> f64 {
        match self {
            Self::Planicie => 1.0,
            Self::Colinas => 1.25,
            Self::Montanhas => 1.75,
            Self::Floresta => 1.15,
            Self::SelvaTropical => 1.50,
            Self::Deserto => 1.30,
            Self::Tundra => 1.60,
            Self::Pantano => 1.45,
        }
    }
}

/// As 5 eras tecnológicas visuais do jogo (1945 a 1985).
/// Modulam a aparência 2D/3D de prédios, cidades e indústrias no mapa.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum EraTecnologicaVisual {
    #[default]
    Era1945,
    Era1955,
    Era1965,
    Era1975,
    Era1985,
}

impl EraTecnologicaVisual {
    /// Ano histórico central de referência da era.
    #[inline]
    pub const fn ano_referencia(self) -> u16 {
        match self {
            Self::Era1945 => 1945,
            Self::Era1955 => 1955,
            Self::Era1965 => 1965,
            Self::Era1975 => 1975,
            Self::Era1985 => 1985,
        }
    }

    /// Índice sequencial `0..=4` para progressão e vetores estáticos.
    #[inline]
    pub const fn indice(self) -> usize {
        match self {
            Self::Era1945 => 0,
            Self::Era1955 => 1,
            Self::Era1965 => 2,
            Self::Era1975 => 3,
            Self::Era1985 => 4,
        }
    }

    /// Converte um índice `0..=4` na era correspondente.
    #[inline]
    pub const fn a_partir_indice(idx: usize) -> Self {
        match idx {
            0 => Self::Era1945,
            1 => Self::Era1955,
            2 => Self::Era1965,
            3 => Self::Era1975,
            _ => Self::Era1985,
        }
    }

    /// Nome descritivo da era visual em português brasileiro.
    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::Era1945 => "Era 1945: Pós-Guerra e Reconstrução (Vapor e Alvenaria)",
            Self::Era1955 => "Era 1955: Era Atômica e Fordismo (Subúrbios e Pré-Moldados)",
            Self::Era1965 => "Era 1965: Era Espacial e Modernismo (Concreto, Vidro e Aviação)",
            Self::Era1975 => "Era 1975: Automação Inicial e Centrais Nucleares",
            Self::Era1985 => "Era 1985: Microeletrônica, Robótica e Parques Tecnológicos",
        }
    }

    /// Retorna a próxima era visual no ciclo histórico, se houver.
    #[inline]
    pub const fn proxima(self) -> Option<Self> {
        match self {
            Self::Era1945 => Some(Self::Era1955),
            Self::Era1955 => Some(Self::Era1965),
            Self::Era1965 => Some(Self::Era1975),
            Self::Era1975 => Some(Self::Era1985),
            Self::Era1985 => None,
        }
    }
}

/// Nível de tamanho e densidade de uma cidade no mapa (escala de 1 a 8).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum NivelCidade {
    #[default]
    PovoadoRural = 1,
    VilaComercial = 2,
    CidadePequena = 3,
    CidadeMedia = 4,
    GrandeCidade = 5,
    MetropoleRegional = 6,
    GrandeMetropole = 7,
    MegalopoleGlobal = 8,
}

impl NivelCidade {
    /// Converte para o valor numérico `1..=8`.
    #[inline]
    pub const fn valor(self) -> u8 {
        self as u8
    }

    /// Cria a partir de inteiro `1..=8` com saturação.
    #[inline]
    pub const fn a_partir_u8(v: u8) -> Self {
        match v {
            0 | 1 => Self::PovoadoRural,
            2 => Self::VilaComercial,
            3 => Self::CidadePequena,
            4 => Self::CidadeMedia,
            5 => Self::GrandeCidade,
            6 => Self::MetropoleRegional,
            7 => Self::GrandeMetropole,
            _ => Self::MegalopoleGlobal,
        }
    }

    /// Limiar mínimo do índice contínuo de urbanização (`0.0..=100.0`) para este nível.
    #[inline]
    pub const fn limiar_indice(self) -> f64 {
        match self {
            Self::PovoadoRural => 0.0,
            Self::VilaComercial => 14.0,
            Self::CidadePequena => 26.0,
            Self::CidadeMedia => 38.0,
            Self::GrandeCidade => 50.0,
            Self::MetropoleRegional => 62.0,
            Self::GrandeMetropole => 74.0,
            Self::MegalopoleGlobal => 86.0,
        }
    }

    /// Nome padronizado em pt-BR.
    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::PovoadoRural => "Povoado Rural",
            Self::VilaComercial => "Vila Comercial",
            Self::CidadePequena => "Cidade Pequena",
            Self::CidadeMedia => "Cidade Média",
            Self::GrandeCidade => "Grande Cidade",
            Self::MetropoleRegional => "Metrópole Regional",
            Self::GrandeMetropole => "Grande Metrópole",
            Self::MegalopoleGlobal => "Megalópole Global",
        }
    }
}

/// Nível de desenvolvimento fabril/extrativo por estado (escala de 0 a 8).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum NivelIndustrial {
    #[default]
    SemIndustria = 0,
    OficinasLocais = 1,
    DistritoManufatureiro = 2,
    PoloFabrilLeve = 3,
    CentroIndustrialConsolidado = 4,
    PoloIndustriaPesada = 5,
    ComplexoSiderurgicoMecanico = 6,
    CinturaoIndustrialIntegrado = 7,
    MegacomplexoMundial = 8,
}

impl NivelIndustrial {
    #[inline]
    pub const fn valor(self) -> u8 {
        self as u8
    }

    #[inline]
    pub const fn a_partir_u8(v: u8) -> Self {
        match v {
            0 => Self::SemIndustria,
            1 => Self::OficinasLocais,
            2 => Self::DistritoManufatureiro,
            3 => Self::PoloFabrilLeve,
            4 => Self::CentroIndustrialConsolidado,
            5 => Self::PoloIndustriaPesada,
            6 => Self::ComplexoSiderurgicoMecanico,
            7 => Self::CinturaoIndustrialIntegrado,
            _ => Self::MegacomplexoMundial,
        }
    }

    /// Limiar mínimo do índice contínuo industrial (`0.0..=100.0`).
    #[inline]
    pub const fn limiar_indice(self) -> f64 {
        match self {
            Self::SemIndustria => 0.0,
            Self::OficinasLocais => 3.0,
            Self::DistritoManufatureiro => 14.0,
            Self::PoloFabrilLeve => 26.0,
            Self::CentroIndustrialConsolidado => 38.0,
            Self::PoloIndustriaPesada => 52.0,
            Self::ComplexoSiderurgicoMecanico => 66.0,
            Self::CinturaoIndustrialIntegrado => 78.0,
            Self::MegacomplexoMundial => 90.0,
        }
    }

    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::SemIndustria => "Sem Indústria Relevante",
            Self::OficinasLocais => "Oficinas e Beneficiamento Local",
            Self::DistritoManufatureiro => "Distrito Manufatureiro Inicial",
            Self::PoloFabrilLeve => "Polo Fabril de Bens de Consumo",
            Self::CentroIndustrialConsolidado => "Centro Industrial Consolidado",
            Self::PoloIndustriaPesada => "Polo de Indústria Pesada e Metalurgia",
            Self::ComplexoSiderurgicoMecanico => "Grande Complexo Siderúrgico-Mecânico",
            Self::CinturaoIndustrialIntegrado => "Cinturão Industrial Integrado",
            Self::MegacomplexoMundial => "Megacomplexo Industrial Mundial",
        }
    }
}

/// Estilo arquitetônico predominante para personalização da malha visual.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum EstiloArquitetonico {
    #[default]
    OcidentalCorporativo,
    SocialistaPlanejado,
    ColonialTradicional,
    DesenvolvimentistaModerno,
    TradicionalHistorico,
}

/// Status jurídico-social de uma população em relação ao país soberano/controlador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusCulturalPop {
    /// Cultura primária/titular do Estado, com cidadania e direitos políticos plenos.
    CidadaniaPlena,
    /// Cultura legalmente aceita (sem discriminação salarial ou perseguição ativa).
    AceitaSecundaria,
    /// Cultura discriminada por lei (perda de renda, sem voto pleno, maior agitação).
    Discriminada,
    /// Cultura ativamente perseguida/alvo de expulsão territorial pós-guerra ou partição.
    Perseguida,
}

impl StatusCulturalPop {
    /// Multiplicador de agitação política / radicalismo (1.0 = normal).
    #[inline]
    pub const fn multiplicador_agitacao(self) -> f64 {
        match self {
            Self::CidadaniaPlena => 1.0,
            Self::AceitaSecundaria => 1.15,
            Self::Discriminada => 2.20,
            Self::Perseguida => 4.50,
        }
    }

    /// Propensão relativa à emigração voluntária ou fuga de refugiados.
    #[inline]
    pub const fn propensao_emigracao(self) -> f64 {
        match self {
            Self::CidadaniaPlena => 0.05,
            Self::AceitaSecundaria => 0.15,
            Self::Discriminada => 0.70,
            Self::Perseguida => 1.00,
        }
    }
}

/// Lei de cidadania e direitos nacionais do país.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum LeiCidadania {
    /// Apartheid ou segregação racial compulsória: apenas primárias têm direitos plenos.
    SegregacaoApartheid,
    /// Domínio colonial: cidadania restrita à metrópole; povos colonizados são tutelados.
    DominioColonialEstrito,
    /// Jus sanguinis étnico: etnia titular possui direitos plenos; minorias são secundárias.
    #[default]
    CidadaniaEtnicaNacional,
    /// Herança cultural ou linguística ampla aceita.
    CidadaniaCulturalAmpla,
    /// Jus soli universal e naturalização cívica sem distinção étnica.
    CidadaniaCivicaJusSoli,
    /// Internacionalismo proletário revolucionário: igualdade a todas as nacionalidades operárias.
    InternacionalismoProletario,
}

/// Tipos de reivindicação jurídica/diplomática sobre um estado ou território.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoReivindicacao {
    Historica,
    Irredentista,
    UnificacaoNacional,
    Colonial,
    SegurancaFronteira,
}

/// Política/método de execução da transferência de território entre soberanias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModoTransferenciaTerritorial {
    /// Cessão diplomática ou unificação voluntária sem expulsão forçada de civis.
    IntegracaoPacifica,
    /// Ocupação ou anexação militar forçada com fuga espontânea de refugiados.
    ConquistaMilitar,
    /// Partição bilateral acordada com troca mútua de minorias (estilo Índia/Paquistão 1947).
    ParticaoComTrocaPopulacional,
    /// Expulsão compulsória das minorias do país anterior (estilo Acordo de Potsdam 1945).
    ExpulsaoForcadaMinorias,
}

/// Os 5 modais fundamentais de infraestrutura territorial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoInfraestrutura {
    Estradas,
    Ferrovias,
    Portos,
    Aeroportos,
    RedeEletrica,
}

impl TipoInfraestrutura {
    pub const TODOS: [Self; 5] = [
        Self::Estradas,
        Self::Ferrovias,
        Self::Portos,
        Self::Aeroportos,
        Self::RedeEletrica,
    ];

    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::Estradas => "Malha Rodoviária",
            Self::Ferrovias => "Malha Ferroviária",
            Self::Portos => "Infraestrutura Portuária",
            Self::Aeroportos => "Aeródromos e Aviação Civil",
            Self::RedeEletrica => "Rede Elétrica e Transmissão",
        }
    }
}

/// Caminho político-institucional de descolonização ou secessão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViaDescolonizacao {
    IndependenciaNegociada,
    ParticaoPosColonial,
    GuerraLibertacaoSecessao,
}

/// Setor de atividade econômica de uma fatia demográfica.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum SetorEmpregoPop {
    #[default]
    Rural,
    Industrial,
    Servicos,
    AdministrativoMilitar,
}
