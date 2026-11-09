//! Entidades de domínio militar serializáveis em RON:
//! regiões do mapa, países militares, comandantes, divisões, exércitos, frentes, guerras e configuração.

use crate::tipos::{DataJogo, EnvolvimentoGuerra, LadoGuerra, PosturaOperacional};
use serde::{Deserialize, Serialize};

const fn padrao_true() -> bool {
    true
}

const fn padrao_um() -> f64 {
    1.0
}

const fn padrao_moral() -> f64 {
    0.80
}

const fn padrao_infra() -> u32 {
    25
}

const fn padrao_pv() -> f64 {
    1.0
}

/// Estado estático e dinâmico de uma região admin-1 no mapa militar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegiaoMapa {
    /// Código único da região no Natural Earth ou no cenário (ex: "USA-3521", "regiao_01").
    pub adm1_code: String,
    /// Nome de exibição em português do Brasil.
    pub nome_pt: String,
    /// Tag de 3 letras do país soberano (dono de jure).
    pub dono_tag: String,
    /// Tag de 3 letras do país com controle militar efetivo no momento (dono ou ocupante).
    pub controlador_tag: String,
    /// Identificador do tipo de terreno no catálogo (ex: "planicie", "montanha").
    pub terreno_id: String,
    /// Índice de infraestrutura de transporte e energia (1 a 100).
    #[serde(default = "padrao_infra")]
    pub infraestrutura: u32,
    /// Capacidade portuária (0 a 100). Valores > 0 atuam como fontes de suprimento naval.
    #[serde(default)]
    pub nivel_porto: u32,
    /// Nível de fortificações fixas e casamatas defensivas (0 a 10).
    #[serde(default)]
    pub nivel_fortificacao: u8,
    /// Pontuação estratégica de vitória da região (capitais e pólos industriais valem mais).
    #[serde(default = "padrao_pv")]
    pub pontos_vitoria: f64,
    /// População residente estimada da região.
    #[serde(default)]
    pub populacao: u64,
    /// Índice de destruição física acumulada por combates e bombardeios (0.00 a 1.00).
    #[serde(default)]
    pub destruicao_guerra: f64,
    /// Nível de resistência civil e partisans em território ocupado (0.00 a 1.00).
    #[serde(default)]
    pub resistencia_ocupacao: f64,
}

impl RegiaoMapa {
    /// Retorna se a região está atualmente ocupada por uma força militar externa.
    #[inline]
    pub fn esta_ocupada(&self) -> bool {
        self.dono_tag != self.controlador_tag
    }
}

/// Estado militar, institucional e logístico de um país beligerante.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaisMilitar {
    /// Tag de 3 letras maiúsculas ASCII (ex: "USA", "SOV").
    pub tag: String,
    /// Nome oficial do país em português.
    pub nome_pt: String,
    /// Código adm1_code da capital política e centro de comando logístico.
    pub capital_adm1: String,
    /// Doutrina militar ativa.
    #[serde(default)]
    pub doutrina_id: String,
    /// Lista de tecnologias militares ativas pesquisadas pelo país.
    #[serde(default)]
    pub tecnologias_ativas: Vec<String>,
    /// Cansaço de guerra acumulado (0.00 a 100.00).
    #[serde(default)]
    pub cansaco_guerra: f64,
    /// Total acumulado de baixas de efetivos militares em combate e atrito.
    #[serde(default)]
    pub baixas_totais: u64,
    /// Reserva humana mobilizável disponível para repor divisões.
    #[serde(default)]
    pub populacao_mobilizavel: u64,
    /// Estoque nacional de suprimentos militares (0.0 a 1.0 de atendimento nacional).
    #[serde(default = "padrao_um")]
    pub estoque_suprimento: f64,
    /// Eficiência da frota de comboios e marinha mercante para abastecer portos (0.0 a 1.0).
    #[serde(default = "padrao_um")]
    pub eficiencia_naval: f64,
}

/// Oficial general comandante de exército com habilidades e traços táticos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comandante {
    pub id: String,
    pub nome_pt: String,
    pub pais_tag: String,
    #[serde(default = "padrao_um_u8")]
    pub habilidade_ataque: u8,
    #[serde(default = "padrao_um_u8")]
    pub habilidade_defesa: u8,
    #[serde(default = "padrao_um_u8")]
    pub habilidade_logistica: u8,
    #[serde(default = "padrao_um_u8")]
    pub habilidade_planejamento: u8,
    #[serde(default)]
    pub experiencia: f64,
    #[serde(default)]
    pub tracos: Vec<String>,
}

const fn padrao_um_u8() -> u8 {
    1
}

/// Divisão militar ativa no tabuleiro de operações.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Divisao {
    pub id: String,
    pub nome_pt: String,
    pub pais_tag: String,
    pub template_id: String,
    /// Região onde a divisão está atualmente posicionada.
    pub regiao_adm1: String,
    /// Número atual de soldados vivos na unidade.
    pub efetivos_atuais: u32,
    /// Proporção de equipamentos pesados e armas intactos (0.00 a 1.00).
    #[serde(default = "padrao_um")]
    pub equipamento_fracao: f64,
    /// Coesão operacional e fôlego de combate (0.0 a max do template).
    pub organizacao: f64,
    /// Disposição e bravura das tropas (0.0 a 1.0).
    #[serde(default = "padrao_moral")]
    pub moral: f64,
    /// Nível de entrincheiramento e casamatas temporárias acumuladas no local (0.0 a max).
    #[serde(default)]
    pub entrincheiramento: f64,
    /// Atendimento logístico local recebido no último tick (0.00 a 1.00).
    #[serde(default = "padrao_um")]
    pub suprimento_atual: f64,
    /// Experiência operacional veterana da divisão (0.00 a 1.00).
    #[serde(default)]
    pub experiencia: f64,
}

impl Divisao {
    /// Determina se a divisão tem condições de lutar ativamente na linha de frente.
    #[inline]
    pub fn pronta_para_combate(&self) -> bool {
        self.efetivos_atuais > 100 && self.organizacao >= 5.0
    }
}

/// Grupamento operacional de divisões sob comando unificado e postura tática.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exercito {
    pub id: String,
    pub nome_pt: String,
    pub pais_tag: String,
    pub comandante_id: Option<String>,
    pub divisoes_ids: Vec<String>,
    #[serde(default)]
    pub postura: PosturaOperacional,
    #[serde(default)]
    pub frente_id: Option<u32>,
    #[serde(default)]
    pub regiao_objetivo_adm1: Option<String>,
}

/// Frente de combate contígua gerada automaticamente ao longo da fronteira entre beligerantes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frente {
    pub id: u32,
    pub nome_pt: String,
    pub guerra_id: String,
    pub lado_a_tag: String,
    pub lado_b_tag: String,
    pub regioes_lado_a: Vec<String>,
    pub regioes_lado_b: Vec<String>,
    pub pares_fronteira: Vec<(String, String)>,
    #[serde(default = "padrao_true")]
    pub ativa: bool,
}

/// Guerra ativa entre coalizões de países com pontuação, baixas e histórico.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guerra {
    pub id: String,
    pub nome_pt: String,
    pub atacantes: Vec<String>,
    pub defensores: Vec<String>,
    #[serde(default)]
    pub data_inicio: DataJogo,
    /// Pontuação de guerra (-100.0 a +100.0 sob ótica dos atacantes).
    #[serde(default)]
    pub pontuacao_guerra: f64,
    #[serde(default)]
    pub baixas_atacantes: u64,
    #[serde(default)]
    pub baixas_defensores: u64,
    #[serde(default)]
    pub batalhas_vencidas_atacantes: u32,
    #[serde(default)]
    pub batalhas_vencidas_defensores: u32,
    #[serde(default)]
    pub capitulou_atacantes: bool,
    #[serde(default)]
    pub capitulou_defensores: bool,
    #[serde(default = "padrao_true")]
    pub ativa: bool,
    /// Envolvimentos de terceiros (guerra por procuração): países que não são beligerantes diretos
    /// mas apoiam um dos lados em diferentes níveis (0 a 5).
    #[serde(default)]
    pub envolvimentos: Vec<EnvolvimentoGuerra>,
}

impl Guerra {
    /// Verifica se um país é beligerante ativo nesta guerra.
    #[inline]
    pub fn participa(&self, tag: &str) -> bool {
        self.atacantes.iter().any(|t| t == tag) || self.defensores.iter().any(|t| t == tag)
    }

    /// Verifica se o país faz parte da coalizão atacante.
    #[inline]
    pub fn eh_atacante(&self, tag: &str) -> bool {
        self.atacantes.iter().any(|t| t == tag)
    }

    /// Verifica se o país faz parte da coalizão defensora.
    #[inline]
    pub fn eh_defensor(&self, tag: &str) -> bool {
        self.defensores.iter().any(|t| t == tag)
    }

    /// Verifica se dois países estão em lados opostos desta guerra.
    #[inline]
    pub fn sao_inimigos(&self, tag_a: &str, tag_b: &str) -> bool {
        (self.eh_atacante(tag_a) && self.eh_defensor(tag_b))
            || (self.eh_defensor(tag_a) && self.eh_atacante(tag_b))
    }
}

/// Configurações globais de balanceamento e execução da simulação militar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigGuerra {
    /// Se verdadeiro, avalia e aplica tratados de paz automaticamente ao atingir capitulação ou exaustão máxima.
    #[serde(default = "padrao_true")]
    pub resolver_paz_automatica: bool,
    /// Fração mínima de pontos de vitória que um país precisa perder (com perda da capital) para capitular.
    #[serde(default = "padrao_limiar_capitulacao")]
    pub limiar_capitulacao_vitoria: f64,
    /// Nível de cansaço de guerra (0 a 100) que força armistício mútuo em guerras estagnadas.
    #[serde(default = "padrao_limiar_cansaco")]
    pub limiar_cansaco_armisticio: f64,
    /// Permite pequena variação estocástica determinística nos dados de combate (baseada na seed).
    #[serde(default = "padrao_true")]
    pub variacao_tatica_aleatoria: bool,
    /// Taxa de recuperação de organização por dia fora de combate para unidades supridas.
    #[serde(default = "padrao_recup_org")]
    pub taxa_recuperacao_org_base: f64,
    /// Taxa máxima de entrincheiramento que uma unidade parada ganha por dia.
    #[serde(default = "padrao_taxa_entrinch")]
    pub taxa_acumulo_entrincheiramento: f64,
    /// Fator de escala sobre dano de combate e baixas.
    #[serde(default = "padrao_um")]
    pub fator_dano_combate: f64,
}

const fn padrao_limiar_capitulacao() -> f64 {
    0.80
}

const fn padrao_limiar_cansaco() -> f64 {
    95.0
}

const fn padrao_recup_org() -> f64 {
    4.0
}

const fn padrao_taxa_entrinch() -> f64 {
    1.5
}

impl Default for ConfigGuerra {
    fn default() -> Self {
        Self {
            resolver_paz_automatica: true,
            limiar_capitulacao_vitoria: 0.80,
            limiar_cansaco_armisticio: 95.0,
            variacao_tatica_aleatoria: true,
            taxa_recuperacao_org_base: 4.0,
            taxa_acumulo_entrincheiramento: 1.5,
            fator_dano_combate: 1.0,
        }
    }
}
