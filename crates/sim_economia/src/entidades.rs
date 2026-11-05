//! Entidades dinâmicas do mundo econômico serializáveis em RON:
//! `Pop`, `Predio`, `Provincia`, `Estado`, `Pais`, `Mercado`, `RotaComercial` e `ConfigSimulacao`.

use crate::catalogo::{CatalogoEconomia, TipoPredio};
use crate::tipos::{FormulaPreco, ModoCadencia, RegimePropriedade};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const fn padrao_true() -> bool {
    true
}

const fn padrao_um() -> f64 {
    1.0
}

const fn sol_padrao() -> f64 {
    10.0
}

const fn alfa_padrao() -> f64 {
    0.35
}

const fn salario_base_padrao() -> f64 {
    4.0
}

const fn infra_padrao() -> f64 {
    25.0
}

fn padrao_terreno() -> String {
    "planicie".to_string()
}

const fn niveis_obra_padrao() -> u32 {
    1
}

const fn pontos_construcao_base_padrao() -> f64 {
    10.0
}

const fn taxa_juros_padrao() -> f64 {
    0.04
}

/// Grupo populacional homogêneo (mesma profissão, cultura e religião) residente em uma `Provincia`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pop {
    pub tipo_pop_id: String,
    pub cultura: String,
    pub religiao: String,
    /// Quantidade de indivíduos economicamente ativos neste grupo (>= 0.0).
    pub populacao: f64,
    /// Quantidade atualmente empregada em prédios formais da província (0.0..=populacao).
    #[serde(default)]
    pub empregados: f64,
    /// Poupança líquida acumulada pelas famílias deste grupo (>= 0.0).
    #[serde(default)]
    pub riqueza_reservas: f64,
    /// Padrão de vida (escala 1.0 a 99.0, estilo Victoria 3).
    #[serde(default = "sol_padrao")]
    pub padrao_de_vida: f64,
    /// Taxa de alfabetização média do grupo (0.0 a 1.0).
    #[serde(default = "alfa_padrao")]
    pub alfabetizacao: f64,
    /// Renda líquida auferida no último tick diário.
    #[serde(default)]
    pub renda_ultimo_tick: f64,
    /// Despesa total em consumo de bens no último tick diário.
    #[serde(default)]
    pub gasto_consumo_ultimo_tick: f64,

    #[serde(skip)]
    pub(crate) tipo_idx: usize,
    #[serde(skip)]
    pub(crate) cultura_idx: u16,
    #[serde(skip)]
    pub(crate) religiao_idx: u16,
}

impl Pop {
    /// Contingente disponível sem emprego formal neste grupo.
    #[inline]
    pub fn desempregados(&self) -> f64 {
        (self.populacao - self.empregados).max(0.0)
    }

    /// Proporção de desempregados (0.0..=1.0).
    #[inline]
    pub fn taxa_desemprego(&self) -> f64 {
        if self.populacao <= 0.0 {
            0.0
        } else {
            (self.desempregados() / self.populacao).clamp(0.0, 1.0)
        }
    }
}

/// Capacidade inline máxima de bens de insumo ou produto por prédio compilado.
pub const MAX_FLUXOS_PREDIO: usize = 8;
/// Capacidade inline máxima de tipos de vaga profissional por prédio compilado.
pub const MAX_VAGAS_PREDIO: usize = 8;

/// Vaga e ocupação pré-compilada de uma profissão em um `Predio`.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SlotEmpregoPredio {
    pub tipo_pop_idx: u16,
    pub vagas_por_nivel: u32,
    pub empregados: f64,
}

/// Cache pré-compilado inline (`Copy`, zero alocações no heap) dos métodos ativos de um `Predio`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PredioCompilado {
    pub tipo_idx: usize,
    pub regime_propriedade: RegimePropriedade,
    pub financiado_pelo_governo: bool,
    pub num_insumos: u8,
    pub num_produtos: u8,
    pub num_vagas: u8,
    pub insumos_arr: [(u16, f64); MAX_FLUXOS_PREDIO],
    pub produtos_arr: [(u16, f64); MAX_FLUXOS_PREDIO],
    pub vagas_arr: [SlotEmpregoPredio; MAX_VAGAS_PREDIO],
    pub pontos_construcao_por_nivel: f64,
    pub burocracia_por_nivel: f64,
    pub forca_militar_por_nivel: f64,
    pub infraestrutura_por_nivel: f64,
    pub escala_efetiva_ultimo_tick: f64,
    pub fator_insumos_ultimo_tick: f64,
    pub margem_lucro_ultimo_tick: f64,
    /// Fator de qualidade do recurso do estado e bônus de insumo local (estáticos por prédio;
    /// `NAN` = ainda não calculado, preenchido no primeiro tick de produção após compilar o cache).
    pub qualidade_local: f64,
}

impl PredioCompilado {
    #[inline]
    pub fn insumos(&self) -> &[(u16, f64)] {
        &self.insumos_arr[..self.num_insumos as usize]
    }

    #[inline]
    pub fn produtos(&self) -> &[(u16, f64)] {
        &self.produtos_arr[..self.num_produtos as usize]
    }

    #[inline]
    pub fn vagas(&self) -> &[SlotEmpregoPredio] {
        &self.vagas_arr[..self.num_vagas as usize]
    }

    #[inline]
    pub fn vagas_mut(&mut self) -> &mut [SlotEmpregoPredio] {
        let n = self.num_vagas as usize;
        &mut self.vagas_arr[..n]
    }
}

impl Default for PredioCompilado {
    fn default() -> Self {
        Self {
            tipo_idx: 0,
            regime_propriedade: RegimePropriedade::PrivadoCapitalista,
            financiado_pelo_governo: false,
            num_insumos: 0,
            num_produtos: 0,
            num_vagas: 0,
            insumos_arr: [(0, 0.0); MAX_FLUXOS_PREDIO],
            produtos_arr: [(0, 0.0); MAX_FLUXOS_PREDIO],
            vagas_arr: [SlotEmpregoPredio::default(); MAX_VAGAS_PREDIO],
            pontos_construcao_por_nivel: 0.0,
            burocracia_por_nivel: 0.0,
            forca_militar_por_nivel: 0.0,
            infraestrutura_por_nivel: 0.0,
            escala_efetiva_ultimo_tick: 0.0,
            fator_insumos_ultimo_tick: 1.0,
            margem_lucro_ultimo_tick: 0.0,
            qualidade_local: f64::NAN,
        }
    }
}

/// Instância de um prédio produtivo, governamental, militar ou de infraestrutura em uma `Provincia`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predio {
    pub tipo_predio_id: String,
    pub nivel: u32,
    pub metodos_ativos: Vec<String>,
    /// Mapa serializável de empregados por `tipo_pop_id` (sincronizado ao carregar/exportar).
    #[serde(default)]
    pub empregados_por_tipo: BTreeMap<String, f64>,
    /// Salário base diário por trabalhador de multiplicador 1.0 neste prédio.
    #[serde(default = "salario_base_padrao")]
    pub salario_base: f64,
    /// Caixa operacional acumulado pelo prédio (>= 0.0).
    #[serde(default)]
    pub reservas_caixa: f64,
    /// Se `true`, eventuais déficits operacionais são cobertos pelo tesouro nacional.
    #[serde(default)]
    pub subsidiado: bool,
    #[serde(default)]
    pub receita_ultimo_tick: f64,
    #[serde(default)]
    pub custo_insumos_ultimo_tick: f64,
    #[serde(default)]
    pub folha_salarial_ultimo_tick: f64,
    #[serde(default)]
    pub lucro_ultimo_tick: f64,
    #[serde(default)]
    pub dividendos_ultimo_tick: f64,

    #[serde(skip)]
    pub(crate) cache: PredioCompilado,
}

impl Predio {
    /// Escala efetiva de produção no último tick (níveis equivalentes, já com ocupação, insumos e acesso).
    #[inline]
    pub fn escala_efetiva(&self) -> f64 {
        self.cache.escala_efetiva_ultimo_tick
    }

    /// Fator de disponibilidade de insumos no último tick (0..1).
    #[inline]
    pub fn fator_insumos(&self) -> f64 {
        self.cache.fator_insumos_ultimo_tick
    }

    /// Margem de lucro do último tick (lucro / receita).
    #[inline]
    pub fn margem_lucro(&self) -> f64 {
        self.cache.margem_lucro_ultimo_tick
    }

    /// Insumos por nível por dia: `(índice do bem no catálogo, quantidade)`.
    #[inline]
    pub fn insumos_por_nivel(&self) -> &[(u16, f64)] {
        self.cache.insumos()
    }

    /// Produtos por nível por dia: `(índice do bem no catálogo, quantidade)`.
    #[inline]
    pub fn produtos_por_nivel(&self) -> &[(u16, f64)] {
        self.cache.produtos()
    }

    /// Vagas por tipo de pop: `(índice do tipo de pop, vagas por nível, empregados)`.
    pub fn vagas_por_tipo(&self) -> impl Iterator<Item = (usize, u32, f64)> + '_ {
        self.cache.vagas().iter().map(|v| (v.tipo_pop_idx as usize, v.vagas_por_nivel, v.empregados))
    }

    /// Total de trabalhadores atualmente empregados neste prédio.
    #[inline]
    pub fn total_empregados(&self) -> f64 {
        if self.cache.num_vagas > 0 {
            self.cache.vagas().iter().map(|v| v.empregados).sum()
        } else {
            self.empregados_por_tipo.values().sum()
        }
    }

    /// Capacidade total de vagas do prédio considerando o `nivel` atual.
    #[inline]
    pub fn total_vagas(&self) -> f64 {
        let nivel = self.nivel as f64;
        self.cache
            .vagas()
            .iter()
            .map(|v| (v.vagas_por_nivel as f64) * nivel)
            .sum()
    }

    /// Taxa de preenchimento das vagas de trabalho (0.0..=1.0).
    #[inline]
    pub fn taxa_ocupacao(&self) -> f64 {
        let vagas = self.total_vagas();
        if vagas <= 0.0 {
            if self.nivel > 0 { 1.0 } else { 0.0 }
        } else {
            (self.total_empregados() / vagas).clamp(0.0, 1.0)
        }
    }

    /// Retorna quantos trabalhadores de um `tipo_pop_id` específico trabalham neste prédio.
    pub fn empregados_no_tipo(&self, catalogo: &CatalogoEconomia, tipo_pop_id: &str) -> f64 {
        if let Some(tp_idx) = catalogo.indice_tipo_pop(tipo_pop_id)
            && self.cache.num_vagas > 0
        {
            return self
                .cache
                .vagas()
                .iter()
                .find(|v| v.tipo_pop_idx as usize == tp_idx)
                .map_or(0.0, |v| v.empregados);
        }
        self.empregados_por_tipo
            .get(tipo_pop_id)
            .copied()
            .unwrap_or(0.0)
    }
}

/// Unidade geográfica local contendo populações (`Pop`), recursos e prédios (`Predio`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provincia {
    pub id: String,
    pub nome: String,
    /// Código cartográfico externo opcional (ex.: `"USA-3521"` de `regioes_1945.ron`).
    #[serde(default)]
    pub codigo_externo: Option<String>,
    pub estado_id: String,
    pub pais_id: String,
    pub terra_aravel_total: u32,
    #[serde(default)]
    pub limites_recursos: BTreeMap<String, u32>,
    #[serde(default = "infra_padrao")]
    pub infraestrutura_base: f64,
    #[serde(default)]
    pub pops: Vec<Pop>,
    #[serde(default)]
    pub predios: Vec<Predio>,

    #[serde(skip)]
    pub(crate) estado_idx: usize,
    #[serde(skip)]
    pub(crate) pais_idx: usize,
    #[serde(skip)]
    pub(crate) mercado_idx: usize,
    #[serde(skip)]
    pub(crate) atracao_migratoria: f64,
    #[serde(skip)]
    pub(crate) vagas_abertas_cache: f64,
    #[serde(skip)]
    pub(crate) terra_aravel_ocupada: u32,
}

impl Provincia {
    /// População total somada de todos os grupos da província.
    pub fn populacao_total(&self) -> f64 {
        self.pops.iter().map(|p| p.populacao).sum()
    }

    /// Média ponderada pela população do padrão de vida na província.
    pub fn padrao_de_vida_medio(&self) -> f64 {
        let pop_total = self.populacao_total();
        if pop_total <= 0.0 {
            return 10.0;
        }
        let soma: f64 = self
            .pops
            .iter()
            .map(|p| p.populacao * p.padrao_de_vida)
            .sum();
        soma / pop_total
    }

    /// Média ponderada pela população da alfabetização na província (0.0..=1.0).
    pub fn alfabetizacao_media(&self) -> f64 {
        let pop_total = self.populacao_total();
        if pop_total <= 0.0 {
            return 0.0;
        }
        let soma: f64 = self
            .pops
            .iter()
            .map(|p| p.populacao * p.alfabetizacao)
            .sum();
        soma / pop_total
    }

    /// Quantidade de terra arável disponível para agricultura de subsistência ou novas fazendas.
    #[inline]
    pub fn terra_aravel_livre(&self) -> u32 {
        self.terra_aravel_total
            .saturating_sub(self.terra_aravel_ocupada)
    }
}

/// Região administrativa (`Estado`) que agrupa províncias dentro de um `Pais`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estado {
    pub id: String,
    pub nome: String,
    pub pais_id: String,
    #[serde(default = "padrao_true")]
    pub incorporado: bool,
    #[serde(default)]
    pub infraestrutura_bonus: f64,
    #[serde(default = "padrao_um")]
    pub acesso_mercado: f64,

    /// Capacidade total de infraestrutura operacional do estado (Victoria 3).
    #[serde(default)]
    pub infraestrutura_capacidade: f64,
    /// Uso total de infraestrutura consumido por todos os níveis de prédios produtivos.
    #[serde(default)]
    pub infraestrutura_uso: f64,
    /// Penalidade de infraestrutura / debuff sobre a produtividade quando uso > capacidade (0.0 a 0.95).
    #[serde(default)]
    pub penalidade_infraestrutura: f64,
    /// Terreno predominante do estado (ex.: "planicie", "montanhas", "colinas", "selva", "deserto", "floresta", "pantano", "tundra").
    #[serde(default = "padrao_terreno")]
    pub terreno: String,
    /// Indica se o estado tem litoral marítimo (habilita portos comerciais e escoamento naval).
    #[serde(default)]
    pub eh_costeiro: bool,
    /// Nível de urbanização do estado (pontos gerados por população urbana e prédios industriais/cívicos).
    #[serde(default)]
    pub urbanizacao: f64,
    /// Parcela da capacidade oriunda de prédios de ferrovia (`rede_ferroviaria`).
    #[serde(default)]
    pub infraestrutura_ferrovias: f64,
    /// Parcela da capacidade oriunda de prédios rodoviários (`rodovia`).
    #[serde(default)]
    pub infraestrutura_rodovias: f64,
    /// Parcela da capacidade oriunda de portos comerciais (`porto_comercial`).
    #[serde(default)]
    pub infraestrutura_portos: f64,
    /// Parcela da capacidade oriunda do terreno e geografia base.
    #[serde(default)]
    pub infraestrutura_base_terreno: f64,
    /// Parcela da capacidade oriunda da urbanização.
    #[serde(default)]
    pub infraestrutura_urbanizacao: f64,

    /// Limite agregado de terra arável do estado (capacidade total para fazendas e plantações).
    #[serde(default)]
    pub terra_aravel: u32,
    /// Culturas agrícolas permitidas no estado ("graos", "cafe", "cacau", "acucar", "algodao", "frutas", "tabaco", "gado", etc.).
    #[serde(default)]
    pub culturas_permitidas: Vec<String>,
    /// Níveis máximos para recursos extrativos/minerais ("carvao", "ferro", "ouro_prata", "cobre", "bauxita", "petroleo", "gas_natural", "uranio", "madeira", "pesca", "borracha", etc.).
    #[serde(default)]
    pub limites_recursos: BTreeMap<String, u32>,
    /// Fatores de qualidade/produtividade do recurso (ex.: "cafe": 1.25 para terra muito apta).
    #[serde(default)]
    pub qualidade_recursos: BTreeMap<String, f64>,

    #[serde(skip)]
    pub(crate) pais_idx: usize,
    #[serde(skip)]
    pub(crate) provincias_indices: Vec<usize>,
    #[serde(skip)]
    pub(crate) terra_aravel_ocupada: u32,
    /// Índices diários (ver `motor::cadeia`): níveis por tipo de prédio, produção local por bem e
    /// bônus de cadeia local por tipo de prédio. Vazios até o primeiro tick.
    #[serde(skip)]
    pub(crate) niveis_por_tipo: Vec<u32>,
    #[serde(skip)]
    pub(crate) producao_local_bem: Vec<f32>,
    #[serde(skip)]
    pub(crate) bonus_cadeia_tipo: Vec<f32>,
}

impl Estado {
    /// Níveis já construídos de um tipo de prédio (índice do catálogo) em todo o estado.
    #[inline]
    pub fn niveis_do_tipo(&self, tipo_idx: usize) -> u32 {
        self.niveis_por_tipo.get(tipo_idx).copied().unwrap_or(0)
    }

    /// Produção diária local (equivalente em escala) do bem de índice `bem_idx` neste estado.
    #[inline]
    pub fn producao_local(&self, bem_idx: usize) -> f64 {
        self.producao_local_bem.get(bem_idx).copied().unwrap_or(0.0) as f64
    }

    /// Bônus de produtividade (0.0..=0.2) de um processador por obter insumos no próprio estado
    /// (e, em menor grau, nos demais estados do país).
    #[inline]
    pub fn bonus_cadeia(&self, tipo_idx: usize) -> f64 {
        self.bonus_cadeia_tipo.get(tipo_idx).copied().unwrap_or(0.0) as f64
    }

    /// Rendimento do recurso local para um prédio extrativo: qualidade do depósito x riqueza
    /// (quantidade do depósito, 0.8..1.2). Sem depósito, rendimento baixo (0.20).
    /// Demais prédios: qualidade do recurso (cultura não permitida rende 0.25).
    pub fn rendimento_recurso(&self, def: &TipoPredio) -> f64 {
        let q = self.fator_qualidade_predio(def).max(0.0);
        match &def.recurso_natural_requerido {
            Some(rec) if !self.limites_recursos.is_empty() => {
                let lim = self.limite_recurso(rec);
                if lim == 0 {
                    0.40 * q
                } else {
                    q * (0.8 + 0.4 * (lim as f64 / 120.0).clamp(0.0, 1.0))
                }
            }
            _ if def.usa_terra_aravel && !self.culturas_permitidas.is_empty() => {
                if !self.cultura_permitida(&def.id) {
                    0.25 * q
                } else {
                    q
                }
            }
            _ => q,
        }
    }

    /// Saldo líquido de capacidade de infraestrutura (positivo = folga, negativo = déficit/gargalo).
    #[inline]
    pub fn saldo_infraestrutura(&self) -> f64 {
        self.infraestrutura_capacidade - self.infraestrutura_uso
    }

    /// Taxa de utilização da infraestrutura (0.0 a 1.0 = normal, > 1.0 = sobrecarga).
    #[inline]
    pub fn taxa_utilizacao_infraestrutura(&self) -> f64 {
        if self.infraestrutura_capacidade <= 1e-6 {
            if self.infraestrutura_uso > 0.0 { 2.0 } else { 0.0 }
        } else {
            self.infraestrutura_uso / self.infraestrutura_capacidade
        }
    }

    /// Fator de modificação de relevo / terreno geográfico sobre a infraestrutura base.
    #[inline]
    pub fn modificador_terreno(&self) -> f64 {
        match self.terreno.to_lowercase().as_str() {
            "montanhas" | "montanha" => 0.50,
            "colinas" | "colina" => 0.80,
            "deserto" => 0.60,
            "selva" | "floresta_tropical" | "selva_tropical" => 0.60,
            "floresta" => 0.85,
            "pantano" => 0.65,
            "tundra" => 0.55,
            _ => 1.00,
        }
    }

    /// Índices das províncias pertencentes a este estado no `MundoEconomico`.
    #[inline]
    pub fn provincias_indices(&self) -> &[usize] {
        &self.provincias_indices
    }

    /// Quantidade de terra arável livre para novas fazendas no estado.
    #[inline]
    pub fn terra_aravel_livre(&self) -> u32 {
        self.terra_aravel.saturating_sub(self.terra_aravel_ocupada)
    }

    /// Quantidade de terra arável atualmente ocupada por prédios agrícolas no estado.
    #[inline]
    pub fn terra_aravel_ocupada(&self) -> u32 {
        self.terra_aravel_ocupada
    }

    /// Verifica se uma cultura ou tipo de prédio agrícola é permitido neste estado.
    pub fn cultura_permitida(&self, tipo_predio_id: &str) -> bool {
        if self.culturas_permitidas.is_empty() {
            return true;
        }
        let cultura_alvo = match tipo_predio_id {
            "fazenda_graos" | "graos" => "graos",
            "rancho_pecuario" | "pecuaria_pesca" | "gado" | "pecuaria" => "gado",
            "fazenda_hortifruti" | "frutas" | "hortifruti" => "frutas",
            "plantacao_algodao" | "algodao" => "algodao",
            "plantacao_acucar" | "acucar" => "acucar",
            "plantacao_cafe_cha" | "cafe" | "cafe_cha" | "cha" => "cafe",
            "plantacao_tabaco" | "tabaco" => "tabaco",
            "plantacao_borracha" | "borracha" => "borracha",
            outro => outro,
        };
        self.culturas_permitidas
            .iter()
            .any(|c| c == cultura_alvo || c == tipo_predio_id)
    }

    /// Retorna o limite de níveis para um determinado recurso mineral/extrativo.
    #[inline]
    pub fn limite_recurso(&self, recurso: &str) -> u32 {
        if let Some(&lim) = self.limites_recursos.get(recurso) {
            return lim;
        }
        match recurso {
            "ferro" => self.limites_recursos.get("minerio_ferro").copied().unwrap_or(0),
            "minerio_ferro" => self.limites_recursos.get("ferro").copied().unwrap_or(0),
            "ouro" => self.limites_recursos.get("ouro_prata").copied().unwrap_or(0),
            "ouro_prata" => self.limites_recursos.get("ouro").copied().unwrap_or(0),
            "pesca" => self.limites_recursos.get("peixe").copied().unwrap_or(0),
            "peixe" => self.limites_recursos.get("pesca").copied().unwrap_or(0),
            "borracha" => self.limites_recursos.get("borracha_natural").copied().unwrap_or(0),
            "borracha_natural" => self.limites_recursos.get("borracha").copied().unwrap_or(0),
            "gas" => self.limites_recursos.get("gas_natural").copied().unwrap_or(0),
            "gas_natural" => self.limites_recursos.get("gas").copied().unwrap_or(0),
            _ => 0,
        }
    }

    /// Fator multiplicador de qualidade/produtividade para um prédio (padrão 1.0).
    pub fn fator_qualidade_predio(&self, def: &TipoPredio) -> f64 {
        if self.qualidade_recursos.is_empty() {
            return 1.0;
        }
        if let Some(rec) = &def.recurso_natural_requerido {
            if let Some(&f) = self.qualidade_recursos.get(rec.as_str()) {
                return f;
            }
            let alt = match rec.as_str() {
                "minerio_ferro" => "ferro",
                "ferro" => "minerio_ferro",
                "ouro_prata" => "ouro",
                "ouro" => "ouro_prata",
                "peixe" => "pesca",
                "pesca" => "peixe",
                "borracha_natural" => "borracha",
                "borracha" => "borracha_natural",
                "gas_natural" => "gas",
                "gas" => "gas_natural",
                _ => "",
            };
            if !alt.is_empty() {
                if let Some(&f) = self.qualidade_recursos.get(alt) {
                    return f;
                }
            }
        }
        let cult = match def.id.as_str() {
            "fazenda_graos" => "graos",
            "rancho_pecuario" | "pecuaria_pesca" => "gado",
            "fazenda_hortifruti" => "frutas",
            "plantacao_algodao" => "algodao",
            "plantacao_acucar" => "acucar",
            "plantacao_cafe_cha" => "cafe",
            "plantacao_tabaco" => "tabaco",
            "plantacao_borracha" => "borracha",
            _ => &def.id,
        };
        if let Some(&f) = self.qualidade_recursos.get(cult) {
            return f;
        }
        if let Some(&f) = self.qualidade_recursos.get(&def.id) {
            return f;
        }
        1.0
    }
}

/// Potencial regional de um estado (limites físicos de expansão, culturas permitidas e qualidade).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PotencialRegional {
    pub estado_id: String,
    #[serde(default)]
    pub terra_aravel: u32,
    #[serde(default)]
    pub culturas_permitidas: Vec<String>,
    #[serde(default)]
    pub recursos: BTreeMap<String, u32>,
    #[serde(default)]
    pub qualidade_recursos: BTreeMap<String, f64>,
}

/// Parâmetros tributários, tarifários e de gastos institucionais de um `Pais`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoliticaFiscal {
    pub lei_sistema_economico_id: String,
    pub lei_tributaria_id: String,
    pub lei_comercial_id: String,
    /// Imposto sobre renda salarial do estrato baixo (0.0..=0.80).
    pub aliquota_imposto_renda_baixo: f64,
    /// Imposto sobre renda salarial do estrato médio (0.0..=0.80).
    pub aliquota_imposto_renda_medio: f64,
    /// Imposto sobre renda salarial do estrato alto (0.0..=0.80).
    pub aliquota_imposto_renda_alto: f64,
    /// Imposto sobre dividendos privados (capitalistas e aristocratas).
    pub aliquota_imposto_dividendos: f64,
    /// Tributo per capita diário cobrado sobre trabalhadores em regime de subsistência na terra.
    #[serde(default)]
    pub imposto_sobre_terra_por_campones: f64,
    /// Imposto territorial diário por nível de prédio rural agrícola privado.
    #[serde(default)]
    pub imposto_sobre_terra_por_nivel_rural: f64,
    /// Alíquota geral sobre consumo de bens (IVA/vendas, 0.0..=0.50).
    #[serde(default)]
    pub aliquota_consumo_geral: f64,
    /// Alíquotas seletivas adicionais sobre bens específicos (`bem_id -> aliquota`).
    #[serde(default)]
    pub bens_tributados_consumo: BTreeMap<String, f64>,
    /// Tarifa cobrada sobre o valor base das importações de outros mercados.
    #[serde(default)]
    pub tarifa_importacao: f64,
    /// Tarifa cobrada sobre o valor base das exportações para outros mercados.
    #[serde(default)]
    pub tarifa_exportacao: f64,
    /// Alíquotas seletivas adicionais de tarifa de importação sobre bens específicos (`bem_id -> tarifa`).
    #[serde(default)]
    pub tarifas_importacao_bens: BTreeMap<String, f64>,
    /// Alíquotas seletivas adicionais de tarifa de exportação sobre bens específicos (`bem_id -> tarifa`).
    #[serde(default)]
    pub tarifas_exportacao_bens: BTreeMap<String, f64>,
    /// Nível geral configurado de abertura de mercado (0.0 a 1.0), se definido.
    #[serde(default)]
    pub nivel_abertura_mercado: Option<f64>,
    /// Nível da instituição pública de educação (0 a 5).
    #[serde(default)]
    pub nivel_investimento_educacao: u8,
    /// Multiplicador de salários pagos a prédios governamentais (0.5 a 1.5).
    #[serde(default = "padrao_um")]
    pub multiplicador_salario_governo: f64,
    /// Multiplicador de salários pagos a prédios militares (0.5 a 1.5).
    #[serde(default = "padrao_um")]
    pub multiplicador_salario_militar: f64,
}

impl PoliticaFiscal {
    /// Tarifa efetiva de importação para um determinado bem (usando sobreposição específica ou a tarifa geral).
    #[inline]
    pub fn tarifa_importacao_bem(&self, bem_id: &str) -> f64 {
        self.tarifas_importacao_bens
            .get(bem_id)
            .copied()
            .unwrap_or(self.tarifa_importacao)
            .max(0.0)
    }

    /// Tarifa efetiva de exportação para um determinado bem (usando sobreposição específica ou a tarifa geral).
    #[inline]
    pub fn tarifa_exportacao_bem(&self, bem_id: &str) -> f64 {
        self.tarifas_exportacao_bens
            .get(bem_id)
            .copied()
            .unwrap_or(self.tarifa_exportacao)
            .max(0.0)
    }

    /// Nível de abertura comercial (0.0 = totalmente fechado/autárquico, 1.0 = livre-câmbio total).
    #[inline]
    pub fn nivel_abertura(&self) -> f64 {
        if let Some(n) = self.nivel_abertura_mercado {
            n.clamp(0.0, 1.0)
        } else {
            (1.0 - (self.tarifa_importacao * 1.5 + self.tarifa_exportacao * 0.5)).clamp(0.0, 1.0)
        }
    }

    /// Define o nível de abertura geral ajustando as tarifas proporcionais (estilo Victoria 3).
    pub fn definir_nivel_abertura(&mut self, nivel: f64) {
        let n = nivel.clamp(0.0, 1.0);
        self.nivel_abertura_mercado = Some(n);
        self.tarifa_importacao = (0.60 * (1.0 - n)).clamp(0.0, 0.60);
        self.tarifa_exportacao = (0.30 * (1.0 - n)).clamp(0.0, 0.30);
    }
}

/// Estado contábil do tesouro, dívida soberana e fluxo diário de receitas e despesas do `Pais`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrcamentoGoverno {
    pub tesouro: f64,
    #[serde(default)]
    pub divida: f64,
    #[serde(default)]
    pub limite_credito: f64,
    #[serde(default = "taxa_juros_padrao")]
    pub taxa_juros_anual_base: f64,
    #[serde(default = "taxa_juros_padrao")]
    pub taxa_juros_efetiva_anual: f64,
    #[serde(default)]
    pub em_calote: bool,

    // Demonstrativo do último tick diário
    #[serde(default)]
    pub receita_imposto_renda: f64,
    #[serde(default)]
    pub receita_imposto_consumo: f64,
    #[serde(default)]
    pub receita_imposto_terra: f64,
    #[serde(default)]
    pub receita_dividendos_estatais: f64,
    #[serde(default)]
    pub receita_tarifas: f64,
    /// Imposto sobre turismo e hotelaria (parte do gasto de turistas estrangeiros).
    #[serde(default)]
    pub receita_turismo: f64,
    #[serde(default)]
    pub gasto_burocracia: f64,
    #[serde(default)]
    pub gasto_militar: f64,
    #[serde(default)]
    pub gasto_construcao: f64,
    /// Materiais (cimento, aço, máquinas) consumidos pelas obras públicas no último tick.
    #[serde(default)]
    pub gasto_obras_materiais: f64,
    #[serde(default)]
    pub gasto_subsidios: f64,
    #[serde(default)]
    pub gasto_juros: f64,
    #[serde(default)]
    pub saldo_diario: f64,
}

impl OrcamentoGoverno {
    /// Soma de todas as receitas arrecadadas no último tick.
    #[inline]
    pub fn receita_total(&self) -> f64 {
        self.receita_imposto_renda
            + self.receita_imposto_consumo
            + self.receita_imposto_terra
            + self.receita_dividendos_estatais
            + self.receita_tarifas
            + self.receita_turismo
    }

    /// Soma de todas as despesas incorridas no último tick.
    #[inline]
    pub fn despesa_total(&self) -> f64 {
        self.gasto_burocracia
            + self.gasto_militar
            + self.gasto_construcao
            + self.gasto_obras_materiais
            + self.gasto_subsidios
            + self.gasto_juros
    }
}

/// Projeto individual na fila nacional de construção civil.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemConstrucao {
    pub provincia_id: String,
    pub tipo_predio_id: String,
    #[serde(default)]
    pub progresso_pontos: f64,
    /// Se 0.0 na entrada RON, será preenchido automaticamente com `TipoPredio::custo_pontos_construcao`.
    #[serde(default)]
    pub custo_total_pontos: f64,
    /// Se `true`, financiado pelo fundo de investimento privado em vez do orçamento estatal direto.
    #[serde(default)]
    pub privado: bool,
    /// Retorno ou lucro líquido diário estimado por nível do projeto (Victoria 3).
    #[serde(default)]
    pub retorno_esperado: f64,
    /// Breve justificativa de mercado da decisão do setor privado.
    #[serde(default)]
    pub justificativa: String,
    /// Níveis entregues pela obra (lote). Economias grandes constroem em lotes: `custo_total_pontos` já
    /// inclui todos os níveis e o limite diário de pontos por obra escala com este valor.
    #[serde(default = "niveis_obra_padrao")]
    pub niveis: u32,

    #[serde(skip)]
    pub(crate) provincia_idx: usize,
    #[serde(skip)]
    pub(crate) tipo_predio_idx: usize,
}

/// Nação soberana com orçamento, leis fiscais, participação em mercado e fila de construção.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pais {
    pub id: String,
    /// Tag geopolítica opcional de 3 letras (ex.: `"USA"`, `"SOV"`, `"BRA"`, compatível com `nacoes/paises.ron`).
    #[serde(default)]
    pub tag: Option<String>,
    pub nome: String,
    pub mercado_id: String,
    /// Países com os quais esta nação possui acordo comercial bilateral (redução drástica ou isenção de tarifas).
    #[serde(default)]
    pub acordos_comerciais: Vec<String>,
    #[serde(default)]
    pub culturas_primarias: Vec<String>,
    #[serde(default)]
    pub religiao_oficial: String,
    pub politica_fiscal: PoliticaFiscal,
    pub orcamento: OrcamentoGoverno,
    /// Capacidade gratuita base de pontos de construção por dia do país.
    #[serde(default = "pontos_construcao_base_padrao")]
    pub pontos_construcao_base: f64,
    #[serde(default)]
    pub fila_construcao: Vec<ItemConstrucao>,
    /// Reserva acumulada pelo reinvestimento de capitalistas/aristocratas para construções privadas.
    #[serde(default)]
    pub fundo_investimento_privado: f64,
    /// Aporte total diário recebido pelo fundo privado a partir de lucros/dividendos no último tick.
    #[serde(default)]
    pub aporte_fundo_privado_ultimo_tick: f64,
    /// Despesa total diária do fundo privado com materiais de construção no último tick.
    #[serde(default)]
    pub gasto_fundo_privado_ultimo_tick: f64,
    /// Pontos de construção efetivamente alocados ao setor privado no último tick.
    #[serde(default)]
    pub pontos_privados_ultimo_tick: f64,
    /// Pontos de construção efetivamente alocados ao setor público/estatal no último tick.
    #[serde(default)]
    pub pontos_publicos_ultimo_tick: f64,
    #[serde(default)]
    pub burocracia_produzida: f64,
    #[serde(default)]
    pub burocracia_necessaria: f64,
    #[serde(default = "padrao_um")]
    pub eficiencia_arrecadacao: f64,
    #[serde(default)]
    pub forca_militar_total: f64,
    #[serde(default)]
    pub pib_diario: f64,
    /// Produtividade total dos fatores (índice, 1.0 em 1945): multiplica o produto físico dos prédios
    /// privados. Cresce com educação (alfabetização) e com o progresso tecnológico do país.
    #[serde(default = "padrao_um")]
    pub produtividade: f64,

    #[serde(skip)]
    pub(crate) mercado_idx: usize,
    #[serde(skip)]
    pub(crate) provincias_indices: Vec<usize>,
    #[serde(skip)]
    pub(crate) estados_indices: Vec<usize>,
    #[serde(skip)]
    pub(crate) aliquota_consumo_por_bem: Vec<f64>,
    #[serde(skip)]
    pub(crate) mult_eficiencia_construcao_lei: f64,
    #[serde(skip)]
    pub(crate) mult_volume_comercio_lei: f64,
    #[serde(skip)]
    pub(crate) exige_propriedade_estatal_lei: bool,
    #[serde(skip)]
    pub(crate) utilizacao_construcao_ultimo_tick: f64,
    /// Demanda por bem (índice do catálogo) gerada pelas obras no tick anterior; entra no mercado no tick seguinte.
    #[serde(skip)]
    pub(crate) demanda_obras: Vec<f64>,
    /// Pontos de construção efetivamente aplicados à fila no último tick.
    #[serde(skip)]
    pub pontos_aplicados_ultimo_tick: f64,
}

impl Pais {
    /// Cria uma nação surgida durante a partida, antes de sua primeira atualização.
    pub fn novo_dinamico(id: impl Into<String>, tag: impl Into<String>, nome: impl Into<String>, mercado_id: impl Into<String>, tesouro: f64) -> Self {
        Self {
            id: id.into(),
            tag: Some(tag.into()),
            nome: nome.into(),
            mercado_id: mercado_id.into(),
            acordos_comerciais: Vec::new(),
            culturas_primarias: Vec::new(),
            religiao_oficial: "religiao_local".into(),
            politica_fiscal: PoliticaFiscal {
                lei_sistema_economico_id: "intervencionismo".into(),
                lei_tributaria_id: "imposto_renda_progressivo".into(),
                lei_comercial_id: "protecionismo".into(),
                aliquota_imposto_renda_baixo: 0.10,
                aliquota_imposto_renda_medio: 0.15,
                aliquota_imposto_renda_alto: 0.25,
                aliquota_imposto_dividendos: 0.20,
                imposto_sobre_terra_por_campones: 0.02,
                imposto_sobre_terra_por_nivel_rural: 4.0,
                aliquota_consumo_geral: 0.04,
                bens_tributados_consumo: BTreeMap::new(),
                tarifa_importacao: 0.08,
                tarifa_exportacao: 0.02,
                tarifas_importacao_bens: BTreeMap::new(),
                tarifas_exportacao_bens: BTreeMap::new(),
                nivel_abertura_mercado: Some(0.65),
                nivel_investimento_educacao: 1,
                multiplicador_salario_governo: 1.0,
                multiplicador_salario_militar: 1.0,
            },
            orcamento: OrcamentoGoverno {
                tesouro: tesouro.max(0.0),
                limite_credito: tesouro.max(0.0) * 0.5,
                ..OrcamentoGoverno::default()
            },
            pontos_construcao_base: 20.0,
            fila_construcao: Vec::new(),
            fundo_investimento_privado: tesouro.max(0.0) * 0.015,
            aporte_fundo_privado_ultimo_tick: 0.0,
            gasto_fundo_privado_ultimo_tick: 0.0,
            pontos_privados_ultimo_tick: 0.0,
            pontos_publicos_ultimo_tick: 0.0,
            burocracia_produzida: 0.0,
            burocracia_necessaria: 0.0,
            eficiencia_arrecadacao: 1.0,
            forca_militar_total: 0.0,
            pib_diario: 0.0,
            produtividade: 1.0,
            mercado_idx: 0,
            provincias_indices: Vec::new(),
            estados_indices: Vec::new(),
            aliquota_consumo_por_bem: Vec::new(),
            mult_eficiencia_construcao_lei: 1.0,
            mult_volume_comercio_lei: 1.0,
            exige_propriedade_estatal_lei: false,
            utilizacao_construcao_ultimo_tick: 0.0,
            demanda_obras: Vec::new(),
            pontos_aplicados_ultimo_tick: 0.0,
        }
    }

    /// Índices das províncias pertencentes a este país no `MundoEconomico`.
    #[inline]
    pub fn provincias_indices(&self) -> &[usize] {
        &self.provincias_indices
    }

    /// Índices dos estados pertencentes a este país no `MundoEconomico`.
    #[inline]
    pub fn estados_indices(&self) -> &[usize] {
        &self.estados_indices
    }

    /// Parcela teórica de pontos de construção dedicada ao setor privado segundo a lei econômica (0.0 a 1.0).
    #[inline]
    pub fn alocacao_privada_lei(&self) -> f64 {
        match self.politica_fiscal.lei_sistema_economico_id.as_str() {
            "laissez_faire" => 0.75,
            "intervencionismo" | "economia_mista" | "extrativismo_colonial" => 0.50,
            "tradicionalismo_agrario" => 0.25,
            "planificacao_central" => 0.00,
            _ => 0.50,
        }
    }

    /// Parcela teórica de pontos de construção dedicada ao setor público/estatal (0.0 a 1.0).
    #[inline]
    pub fn alocacao_estatal_lei(&self) -> f64 {
        1.0 - self.alocacao_privada_lei()
    }

    /// Indica se a lei do sistema econômico exige propriedade estatal e proíbe investimento privado autônomo.
    #[inline]
    pub fn exige_propriedade_estatal(&self) -> bool {
        self.exige_propriedade_estatal_lei
    }

    /// Itera sobre todas as obras privadas ativas ou em espera na fila nacional.
    #[inline]
    pub fn obras_privadas(&self) -> impl Iterator<Item = &ItemConstrucao> {
        self.fila_construcao.iter().filter(|i| i.privado)
    }

    /// Itera sobre todas as obras públicas/estatais na fila nacional.
    #[inline]
    pub fn obras_estatais(&self) -> impl Iterator<Item = &ItemConstrucao> {
        self.fila_construcao.iter().filter(|i| !i.privado)
    }
}

/// Mercado comum ou nacional onde bens são precificados por oferta e demanda agregadas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mercado {
    pub id: String,
    pub nome: String,
    pub pais_lider_id: String,
    #[serde(default)]
    pub precos_atuais: BTreeMap<String, f64>,
    #[serde(default)]
    pub ordens_venda: BTreeMap<String, f64>,
    #[serde(default)]
    pub ordens_compra: BTreeMap<String, f64>,
    /// Saldo líquido importado (+) ou exportado (-) por `bem_id` no último tick.
    #[serde(default)]
    pub importacoes_liquidas: BTreeMap<String, f64>,

    #[serde(skip)]
    pub(crate) pais_lider_idx: usize,
    #[serde(skip)]
    pub(crate) precos_idx: Vec<f64>,
    #[serde(skip)]
    pub(crate) oferta_idx: Vec<f64>,
    #[serde(skip)]
    pub(crate) demanda_idx: Vec<f64>,
    #[serde(skip)]
    pub(crate) fator_suprimento_idx: Vec<f64>,
    #[serde(skip)]
    pub(crate) importacoes_liquidas_idx: Vec<f64>,
}

/// Rota de importação/exportação conectando dois mercados distintos para um bem comercializável.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotaComercial {
    pub id: String,
    /// Mercado exportador (onde são lançadas ordens de compra da rota).
    pub mercado_origem_id: String,
    /// Mercado importador (onde são lançadas ordens de venda da rota).
    pub mercado_destino_id: String,
    /// País que opera a rota comercial e recolhe tarifas associadas.
    pub pais_operador_id: String,
    pub bem_id: String,
    pub volume_atual: f64,
    pub volume_maximo: f64,
    /// Se `true`, o volume se ajusta dinamicamente conforme o diferencial de preços entre os mercados.
    #[serde(default = "padrao_true")]
    pub automatica: bool,
    #[serde(default)]
    pub lucro_ultimo_tick: f64,

    #[serde(skip)]
    pub(crate) origem_idx: usize,
    #[serde(skip)]
    pub(crate) destino_idx: usize,
    #[serde(skip)]
    pub(crate) pais_operador_idx: usize,
    #[serde(skip)]
    pub(crate) bem_idx: usize,
}

/// Bloco econômico ou comercial multilateral (ex.: Bloco Soviético, Bloco Ocidental/GATT, COMECON).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlocoComercial {
    pub id: String,
    pub nome: String,
    pub lider_pais_id: String,
    #[serde(default)]
    pub membros: Vec<String>,
    /// Se `true`, países do bloco não participam do livre comércio global e restringem rotas externas a acordos.
    #[serde(default)]
    pub fechado: bool,
    /// Se `true` (especialmente para o bloco soviético), ativa integração comercial socialista COMECON com livre comércio interno.
    #[serde(default)]
    pub comecon_ativo: bool,
}

/// Resumo geral do comércio internacional de um país para a interface econômica (F3).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResumoComercioPais {
    pub total_exportacoes_valor: f64,
    pub total_importacoes_valor: f64,
    pub saldo_comercial: f64,
    pub receita_tarifas_total: f64,
    pub nivel_abertura: f64,
    pub tarifa_geral_importacao: f64,
    pub tarifa_geral_exportacao: f64,
    pub acordos_comerciais: Vec<String>,
    pub bloco_id: Option<String>,
    pub bloco_nome: Option<String>,
    pub bloco_fechado: bool,
    pub comecon_ativo: bool,
    pub eh_lider_bloco: bool,
    pub membros_bloco: Vec<String>,
}

/// Detalhe de uma rota comercial com métricas de parceiro e preços para a tela de Comércio (F3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetalheRotaComercial {
    pub id: String,
    pub bem_id: String,
    pub bem_nome: String,
    pub eh_exportacao: bool,
    pub parceiro_pais_id: String,
    pub parceiro_pais_nome: String,
    pub volume_atual: f64,
    pub volume_maximo: f64,
    pub automatica: bool,
    pub preco_origem: f64,
    pub preco_destino: f64,
    pub lucro_ultimo_tick: f64,
    pub tarifa_origem: f64,
    pub tarifa_destino: f64,
    pub tem_acordo: bool,
}

/// Opção de parceiro comercial para estabelecimento de novas rotas comerciais.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcaoParceiroComercial {
    pub pais_id: String,
    pub nome: String,
    pub mercado_id: String,
    pub tem_acordo: bool,
    pub mesmo_bloco: bool,
    pub permitido_comercio: bool,
    pub razao_bloqueio: Option<String>,
}

/// Parâmetros globais de calibração e estabilidade da simulação econômica.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSimulacao {
    pub formula_preco: FormulaPreco,
    pub modo_cadencia: ModoCadencia,
    /// Multiplicador mínimo de preço em relação ao `preco_base` (padrão Victoria 3: `0.25`).
    pub multiplicador_preco_minimo: f64,
    /// Multiplicador máximo de preço em relação ao `preco_base` (padrão Victoria 3: `1.75`).
    pub multiplicador_preco_maximo: f64,
    /// Volume mínimo amortecedor no denominador de preço (evita explosão com ordens residuais).
    pub volume_minimo_mercado: f64,
    /// Peso do preço-alvo diário na média móvel exponencial (`0.0..=1.0`, ex.: `0.35`).
    pub fator_suavizacao_preco: f64,
    /// Elasticidade-preço de substituição entre bens de uma mesma necessidade (`> 0.0`).
    pub elasticidade_substituicao: f64,
    /// Máximo de pontos de construção aplicáveis a um único canteiro de obras por dia.
    pub pontos_maximos_por_obra_dia: f64,
    /// Fração máxima das vagas totais que um prédio pode contratar em um único dia.
    pub taxa_contratacao_diaria: f64,
    /// Fração máxima dos empregados que um prédio deficitário sem caixa demite por dia.
    pub taxa_demissao_diaria: f64,
    /// Se `true`, países usam o `fundo_investimento_privado` acumulado para enfileirar obras privadas automaticamente.
    pub habilitar_investimento_privado_automatico: bool,
    /// Comércio mundial automático: mercados com excedente abastecem mercados deficitários e o
    /// "resto do mundo" cobre parte do déficit residual, tudo limitado pela abertura comercial.
    #[serde(default = "padrao_true")]
    pub comercio_mundial_automatico: bool,
    /// Fração máxima do déficit local de um bem coberta por importação do resto do mundo (com abertura 1.0).
    #[serde(default = "fracao_importacao_padrao")]
    pub fracao_maxima_importacao_residual: f64,
}

const fn fracao_importacao_padrao() -> f64 {
    0.6
}

impl Default for ConfigSimulacao {
    fn default() -> Self {
        Self {
            formula_preco: FormulaPreco::VictoriaSuavizada,
            modo_cadencia: ModoCadencia::DiarioCompleto,
            multiplicador_preco_minimo: 0.25,
            multiplicador_preco_maximo: 1.75,
            volume_minimo_mercado: 5.0,
            fator_suavizacao_preco: 0.35,
            elasticidade_substituicao: 1.5,
            pontos_maximos_por_obra_dia: 25.0,
            taxa_contratacao_diaria: 0.04,
            taxa_demissao_diaria: 0.025,
            habilitar_investimento_privado_automatico: true,
            comercio_mundial_automatico: true,
            fracao_maxima_importacao_residual: 1.0,
        }
    }
}
