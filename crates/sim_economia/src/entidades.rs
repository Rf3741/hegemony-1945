//! Entidades dinâmicas do mundo econômico serializáveis em RON:
//! `Pop`, `Predio`, `Provincia`, `Estado`, `Pais`, `Mercado`, `RotaComercial` e `ConfigSimulacao`.

use crate::catalogo::CatalogoEconomia;
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

    #[serde(skip)]
    pub(crate) pais_idx: usize,
    #[serde(skip)]
    pub(crate) provincias_indices: Vec<usize>,
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
    #[serde(default)]
    pub gasto_burocracia: f64,
    #[serde(default)]
    pub gasto_militar: f64,
    #[serde(default)]
    pub gasto_construcao: f64,
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
    }

    /// Soma de todas as despesas incorridas no último tick.
    #[inline]
    pub fn despesa_total(&self) -> f64 {
        self.gasto_burocracia
            + self.gasto_militar
            + self.gasto_construcao
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
        }
    }
}
