//! Estruturas e entidades da hierarquia territorial:
//! Região (adm1_code) → Estado → Região Estratégica, além de País, Pop e Reivindicações.

use crate::catalogo::Cultura;
use crate::infraestrutura::RedeInfraestruturaEstado;
use crate::tipos::{
    Continente, DataJogo, EraTecnologicaVisual, LeiCidadania, SetorEmpregoPop, StatusCulturalPop,
    TipoReivindicacao, TipoTerreno,
};
use crate::urbanizacao::{Cidade, EstadoUrbano};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Fatia demográfica regional agrupada por cultura, religião e setor de emprego.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PopRegional {
    pub cultura: String,
    pub religiao: String,
    pub setor: SetorEmpregoPop,
    pub populacao: u64,
    #[serde(default = "padrao_status_cultural")]
    pub status_cultural: StatusCulturalPop,
    /// Nível de agitação / radicalismo social (0.0 a 10.0).
    #[serde(default = "padrao_agitacao")]
    pub agitacao: f64,
}

const fn padrao_status_cultural() -> StatusCulturalPop {
    StatusCulturalPop::CidadaniaPlena
}

const fn padrao_agitacao() -> f64 {
    1.0
}

impl PopRegional {
    pub fn nova(
        cultura: impl Into<String>,
        religiao: impl Into<String>,
        setor: SetorEmpregoPop,
        populacao: u64,
    ) -> Self {
        Self {
            cultura: cultura.into(),
            religiao: religiao.into(),
            setor,
            populacao,
            status_cultural: StatusCulturalPop::CidadaniaPlena,
            agitacao: 1.0,
        }
    }
}

/// Registro de reivindicação soberana sobre um território (claim).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reivindicacao {
    pub pais_tag: String,
    pub tipo: TipoReivindicacao,
    pub forca: f64,
    pub data_inicio: DataJogo,
}

/// Unidade territorial básica (Natural Earth 10m Admin-1 com chave `adm1_code`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Regiao {
    /// Código único Natural Earth 10m Admin-1 (ex: "BRA-1311", "USA-3521").
    pub adm1_code: String,
    /// Nome internacional / local.
    pub nome: String,
    /// Nome padronizado em pt-BR.
    pub nome_pt: String,
    /// Identificador snake_case do Estado ao qual esta região pertence.
    pub estado_id: String,
    /// Tag de 3 letras do país de jure soberano.
    pub pais_tag: String,
    /// Tag de 3 letras do país de facto controlador (permite ocupação militar em guerra).
    pub controlador_tag: String,
    /// População residente total somada das pops regionais.
    pub populacao: u64,
    /// Fração urbanizada local (0.00 a 1.00).
    #[serde(default)]
    pub taxa_urbanizacao: f64,
    /// Terreno predominante.
    #[serde(default)]
    pub terreno: TipoTerreno,
    /// Acesso direto à costa marítima.
    #[serde(default)]
    pub eh_costeira: bool,
    /// Fator de destruição física de guerra (0.00 a 1.00).
    #[serde(default)]
    pub destruicao_guerra: f64,
    /// Fatias populacionais da região.
    #[serde(default)]
    pub pops: Vec<PopRegional>,
    /// Depósitos e capacidades de recursos naturais (ex: "carvao": 100).
    #[serde(default)]
    pub recursos: BTreeMap<String, u32>,
    /// Prédios instalados e seus respectivos níveis funcionais.
    #[serde(default)]
    pub predios: BTreeMap<String, u32>,
}

impl Regiao {
    /// Recalcula o total de população a partir da lista de pops.
    pub fn recalcular_populacao(&mut self) -> u64 {
        let total: u64 = self.pops.iter().map(|p| p.populacao).sum();
        self.populacao = total;
        total
    }
}

/// Unidade político-administrativa intermediária (`Estado`), que agrupa regiões admin-1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Estado {
    /// Identificador snake_case ASCII único do estado (ex: "estado_sao_paulo", "california").
    pub id: String,
    /// Nome de exibição em pt-BR.
    pub nome: String,
    /// Identificador snake_case da região estratégica que contém este estado.
    pub regiao_estrategica_id: String,
    /// Tag de 3 letras do país proprietário de jure.
    pub dono_tag: String,
    /// Tag de 3 letras do país controlador de facto.
    pub controlador_tag: String,
    /// Se `true`, é território metropolitano incorporado; se `false`, é dependência ou colônia.
    #[serde(default = "padrao_true")]
    pub incorporado: bool,
    /// Cidade sede ou capital estadual, se houver.
    #[serde(default)]
    pub capital_cidade_id: Option<String>,
    /// Tags de países que possuem núcleo legítimo (core) neste estado.
    #[serde(default)]
    pub nucleos: BTreeSet<String>,
    /// Mapa de reivindicações ativas (`tag_pais -> Reivindicacao`).
    #[serde(default)]
    pub reivindicacoes: BTreeMap<String, Reivindicacao>,
    /// Lista de cidades existentes no estado.
    #[serde(default)]
    pub cidades: Vec<Cidade>,
    /// Malha dos 5 modais de infraestrutura integrados.
    pub rede_infraestrutura: RedeInfraestruturaEstado,
    /// Indicadores de desenvolvimento urbano, industrialização e níveis visuais.
    #[serde(default)]
    pub desenvolvimento_urbano: EstadoUrbano,
    /// Forçamento manual opcional de era visual no estado.
    #[serde(default)]
    pub era_visual_override: Option<EraTecnologicaVisual>,
    /// Métodos de produção ativos por tipo de prédio neste estado (`tipo_predio_id -> [metodo_id]`).
    #[serde(default)]
    pub metodos_ativos: BTreeMap<String, Vec<String>>,
    /// Agitação e descontentamento médio no estado (0.0 a 100.0).
    #[serde(default)]
    pub agitacao_social: f64,
}

const fn padrao_true() -> bool {
    true
}

impl Estado {
    pub fn novo(
        id: impl Into<String>,
        nome: impl Into<String>,
        regiao_estrategica_id: impl Into<String>,
        dono_tag: impl Into<String>,
        eh_costeiro: bool,
        nivel_infra_base: u32,
    ) -> Self {
        let d_tag = dono_tag.into();
        let mut nucleos = BTreeSet::new();
        nucleos.insert(d_tag.clone());

        Self {
            id: id.into(),
            nome: nome.into(),
            regiao_estrategica_id: regiao_estrategica_id.into(),
            controlador_tag: d_tag.clone(),
            dono_tag: d_tag,
            incorporado: true,
            capital_cidade_id: None,
            nucleos,
            reivindicacoes: BTreeMap::new(),
            cidades: Vec::new(),
            rede_infraestrutura: RedeInfraestruturaEstado::novo(eh_costeiro, nivel_infra_base),
            desenvolvimento_urbano: EstadoUrbano::default(),
            era_visual_override: None,
            metodos_ativos: BTreeMap::new(),
            agitacao_social: 0.0,
        }
    }

    /// Retorna `true` se o país indicado possui núcleo legítimo neste estado.
    #[inline]
    pub fn tem_nucleo(&self, tag: &str) -> bool {
        self.nucleos.contains(tag)
    }

    /// Retorna `true` se o país possui reivindicação ativa (claim).
    #[inline]
    pub fn tem_reivindicacao(&self, tag: &str) -> bool {
        self.reivindicacoes.contains_key(tag)
    }

    /// Adiciona núcleo de soberania ao estado.
    pub fn adicionar_nucleo(&mut self, tag: impl Into<String>) {
        self.nucleos.insert(tag.into());
    }

    /// Remove núcleo de soberania.
    pub fn remover_nucleo(&mut self, tag: &str) {
        self.nucleos.remove(tag);
    }

    /// Registra uma nova reivindicação soberana.
    pub fn adicionar_reivindicacao(
        &mut self,
        tag: impl Into<String>,
        tipo: TipoReivindicacao,
        data: DataJogo,
    ) {
        let t = tag.into();
        self.reivindicacoes.insert(
            t.clone(),
            Reivindicacao {
                pais_tag: t,
                tipo,
                forca: 50.0,
                data_inicio: data,
            },
        );
    }

    /// Remove reivindicação.
    pub fn remover_reivindicacao(&mut self, tag: &str) {
        self.reivindicacoes.remove(tag);
    }
}

/// Região estratégica macro para teatros operacionais, comércio e agrupamento geográfico.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegiaoEstrategica {
    pub id: String,
    pub nome: String,
    pub continente: Continente,
    #[serde(default)]
    pub maritima: bool,
    #[serde(default)]
    pub estados: Vec<String>,
}

impl RegiaoEstrategica {
    pub fn nova(
        id: impl Into<String>,
        nome: impl Into<String>,
        continente: Continente,
        maritima: bool,
    ) -> Self {
        Self {
            id: id.into(),
            nome: nome.into(),
            continente,
            maritima,
            estados: Vec::new(),
        }
    }
}

/// Registro institucional e demográfico de um país no mapa territorial.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaisTerritorial {
    /// Tag de 3 letras maiúsculas (ex: "USA", "SOV", "BRA").
    pub tag: String,
    /// Nome oficial em português brasileiro.
    pub nome: String,
    /// Estado onde se localiza a capital nacional.
    pub capital_estado_id: String,
    /// Culturas titulares com cidadania primária.
    pub culturas_primarias: Vec<String>,
    /// Religião hegemônica ou oficial.
    pub religiao_principal: String,
    /// Lei ativa de cidadania e direitos culturais.
    #[serde(default)]
    pub lei_cidadania: LeiCidadania,
    /// Metrópole/Suserano se for colônia, mandato ou protetorado (`None` se soberano).
    #[serde(default)]
    pub suserano_tag: Option<String>,
    /// Alinhamento inicial geopolítico na Guerra Fria.
    #[serde(default)]
    pub alinhamento: Option<String>,
    /// Tipo de regime ou forma institucional de governo.
    #[serde(default)]
    pub tipo_governo: Option<String>,
    /// Conjunto de identificadores snake_case das tecnologias descobertas pelo país.
    #[serde(default)]
    pub tecnologias_desbloqueadas: BTreeSet<String>,
    /// Era visual efetiva nacional.
    #[serde(default)]
    pub era_visual: EraTecnologicaVisual,
    /// Teto máximo da era permitido pelas tecnologias ativas.
    #[serde(default)]
    pub era_teto_tecnologico: EraTecnologicaVisual,
    /// Pontuação integrada contínua de modernidade (0.0 a 100.0).
    #[serde(default)]
    pub indice_modernidade: f64,
}

impl PaisTerritorial {
    pub fn novo(
        tag: impl Into<String>,
        nome: impl Into<String>,
        capital_estado_id: impl Into<String>,
        culturas_primarias: Vec<String>,
        religiao_principal: impl Into<String>,
    ) -> Self {
        Self {
            tag: tag.into(),
            nome: nome.into(),
            capital_estado_id: capital_estado_id.into(),
            culturas_primarias,
            religiao_principal: religiao_principal.into(),
            lei_cidadania: LeiCidadania::CidadaniaEtnicaNacional,
            suserano_tag: None,
            alinhamento: None,
            tipo_governo: None,
            tecnologias_desbloqueadas: BTreeSet::new(),
            era_visual: EraTecnologicaVisual::Era1945,
            era_teto_tecnologico: EraTecnologicaVisual::Era1945,
            indice_modernidade: 0.0,
        }
    }

    /// Retorna `true` se o país é plenamente soberano (não é colônia).
    #[inline]
    pub fn eh_soberano(&self) -> bool {
        self.suserano_tag.is_none()
    }

    /// Retorna `true` se o país é colônia ou protetorado de outra metrópole.
    #[inline]
    pub fn eh_colonia(&self) -> bool {
        self.suserano_tag.is_some()
    }

    /// Determina o status jurídico-social de uma cultura sob as leis deste país.
    pub fn determinar_status_cultural(
        &self,
        cultura: &Cultura,
        culturas_primarias_defs: &[&Cultura],
        eh_estado_incorporado: bool,
    ) -> StatusCulturalPop {
        // 1. Cultura primária é sempre Cidadania Plena
        if self.culturas_primarias.iter().any(|p| p == &cultura.id) {
            return StatusCulturalPop::CidadaniaPlena;
        }

        // Verifica compartilhamento de características culturais com as primárias
        let mesmo_grupo_linguistico = culturas_primarias_defs
            .iter()
            .any(|cp| cp.grupo_linguistico == cultura.grupo_linguistico);
        let mesma_heranca = culturas_primarias_defs
            .iter()
            .any(|cp| cp.heranca_cultural == cultura.heranca_cultural);

        match self.lei_cidadania {
            LeiCidadania::SegregacaoApartheid => {
                // Em apartheid, qualquer cultura diferente da primária é discriminada ou perseguida
                if mesma_heranca {
                    StatusCulturalPop::Discriminada
                } else {
                    StatusCulturalPop::Perseguida
                }
            }
            LeiCidadania::DominioColonialEstrito => {
                if !eh_estado_incorporado {
                    StatusCulturalPop::Discriminada
                } else if mesma_heranca {
                    StatusCulturalPop::AceitaSecundaria
                } else {
                    StatusCulturalPop::Discriminada
                }
            }
            LeiCidadania::CidadaniaEtnicaNacional => {
                if mesmo_grupo_linguistico && mesma_heranca {
                    StatusCulturalPop::AceitaSecundaria
                } else {
                    StatusCulturalPop::Discriminada
                }
            }
            LeiCidadania::CidadaniaCulturalAmpla => {
                if mesma_heranca || mesmo_grupo_linguistico {
                    StatusCulturalPop::AceitaSecundaria
                } else {
                    StatusCulturalPop::Discriminada
                }
            }
            LeiCidadania::CidadaniaCivicaJusSoli => {
                if eh_estado_incorporado {
                    StatusCulturalPop::AceitaSecundaria
                } else {
                    StatusCulturalPop::Discriminada
                }
            }
            LeiCidadania::InternacionalismoProletario => StatusCulturalPop::AceitaSecundaria,
        }
    }
}

/// Movimento anticolonial ou separatista que busca a autodeterminação e independência.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MovimentoIndependencia {
    pub id: String,
    /// Tag do país que será emancipado / fundado (ex: "ALG", "IND", "PAK").
    pub pais_pretendente_tag: String,
    pub nome_pais_pretendente: String,
    /// Tag da metrópole ou país ocupante contra o qual se reivindica a secessão.
    pub metropole_tag: String,
    /// Estados territoriais visados para a secessão.
    pub estados_alvo: Vec<String>,
    /// Fervor anticolonial acumulado (0.0 a 100.0).
    #[serde(default)]
    pub fervor_independencia: f64,
    /// Apoio clandestino de superpotência externa (ex: "SOV" ou "USA").
    #[serde(default)]
    pub apoio_externo_tag: Option<String>,
    /// Culturas primárias que assumirão o poder no novo país.
    pub culturas_primarias_pretendente: Vec<String>,
    /// Lei de cidadania inicial adotada pelo novo Estado.
    #[serde(default)]
    pub lei_cidadania_inicial: LeiCidadania,
    /// Se o movimento está ativo.
    #[serde(default = "padrao_true")]
    pub ativo: bool,
    /// Se eclodiu guerra armada de libertação nacional.
    #[serde(default)]
    pub em_guerra: bool,
}

/// Estrutura para leitura e compatibilidade direta com `provincias_1945/regioes_1945.ron`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Regiao1945 {
    pub adm1_code: String,
    pub nome: String,
    pub nome_pt: String,
    pub pais_tag: String,
    pub populacao: u64,
    pub taxa_urbanizacao: f32,
    pub culturas: BTreeMap<String, f32>,
    pub religioes: BTreeMap<String, f32>,
    pub recursos: BTreeMap<String, u32>,
    pub infraestrutura: u32,
    pub predios: BTreeMap<String, u32>,
    pub destruicao_guerra: f32,
}
