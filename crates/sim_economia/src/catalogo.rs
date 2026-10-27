//! Catálogo estático de definições econômicas carregadas de arquivos RON:
//! `Bem`, `TipoPop`, `LeiEconomica`, `MetodoProducao` e `TipoPredio`.

use crate::erro::{ErroEconomia, ResultadoEconomia, validar_id_snake_case};
use crate::tipos::{
    CategoriaBem, CategoriaNecessidade, CategoriaPredio, Estrato, GrupoLeiEconomica,
    NUM_NECESSIDADES, RegimePropriedade, SlotMetodo,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const fn padrao_true() -> bool {
    true
}

const fn padrao_um() -> f64 {
    1.0
}

/// Regra de participação e substituição de um `Bem` dentro de uma `CategoriaNecessidade`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstituicaoNecessidade {
    pub necessidade: CategoriaNecessidade,
    /// Peso intrínseco de preferência por este bem na cesta da necessidade (> 0.0).
    #[serde(default = "padrao_um")]
    pub peso: f64,
    /// Fração máxima (0.0..=1.0) que este bem pode atender sozinho na necessidade.
    #[serde(default = "padrao_um")]
    pub fracao_maxima: f64,
    /// Fração mínima (0.0..=1.0) garantida quando o bem está disponível.
    #[serde(default)]
    pub fracao_minima: f64,
}

/// Definição de um bem econômico (`bens.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bem {
    /// Identificador único em `snake_case` ASCII (ex.: `"graos"`, `"aco"`).
    pub id: String,
    /// Nome de exibição em pt-BR (ex.: `"Grãos"`, `"Aço"`).
    pub nome: String,
    pub categoria: CategoriaBem,
    /// Preço de referência quando oferta e demanda estão equilibradas (> 0.0).
    pub preco_base: f64,
    /// Se `true`, pode ser importado/exportado entre mercados; se `false`, é estritamente local ao mercado.
    #[serde(default = "padrao_true")]
    pub comercializavel: bool,
    /// Categorias de consumo das pops que este bem pode satisfazer por substituição.
    #[serde(default)]
    pub substituicoes: Vec<SubstituicaoNecessidade>,
}

/// Definição de uma profissão / tipo de população (`tipos_pop.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipoPop {
    /// Identificador único em `snake_case` ASCII (ex.: `"operarios"`, `"capitalistas"`).
    pub id: String,
    /// Nome de exibição em pt-BR.
    pub nome: String,
    pub estrato: Estrato,
    /// Multiplicador relativo sobre o `salario_base` pago pelo prédio (> 0.0).
    pub multiplicador_salario: f64,
    /// Alfabetização mínima (0.0..=1.0) exigida para promoção/contratação nesta profissão.
    #[serde(default)]
    pub alfabetizacao_minima: f64,
    /// Se `true`, participa do rateio de dividendos de prédios com `RegimePropriedade::PrivadoCapitalista`.
    #[serde(default)]
    pub recebe_dividendos_capitalista: bool,
    /// Se `true`, participa do rateio de dividendos de prédios com `RegimePropriedade::Aristocratico`.
    #[serde(default)]
    pub recebe_dividendos_aristocrata: bool,
    /// Se `true` (ex.: `camponeses`), gera sustento de subsistência na terra arável livre.
    #[serde(default)]
    pub subsistencia: bool,
    /// Fator multiplicador sobre a dependência de bens comprados no mercado formal (ex.: 0.30 para camponeses).
    #[serde(default = "padrao_um")]
    pub fator_consumo_mercado: f64,
    /// Peso político base per capita (gancho de integração com o módulo de política/grupos de interesse).
    #[serde(default = "padrao_um")]
    pub forca_politica_base: f64,
}

/// Definição de uma lei econômica (`leis_economicas.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeiEconomica {
    pub id: String,
    pub nome: String,
    pub grupo: GrupoLeiEconomica,
    pub descricao: String,
    #[serde(default)]
    pub aliquota_renda_baixo_sugerida: Option<f64>,
    #[serde(default)]
    pub aliquota_renda_medio_sugerida: Option<f64>,
    #[serde(default)]
    pub aliquota_renda_alto_sugerida: Option<f64>,
    #[serde(default)]
    pub aliquota_dividendos_sugerida: Option<f64>,
    #[serde(default)]
    pub imposto_terra_por_campones_sugerido: Option<f64>,
    #[serde(default)]
    pub tarifa_importacao_sugerida: Option<f64>,
    #[serde(default)]
    pub tarifa_exportacao_sugerida: Option<f64>,
    #[serde(default = "padrao_um")]
    pub multiplicador_eficiencia_construcao: f64,
    #[serde(default = "padrao_um")]
    pub multiplicador_volume_comercio: f64,
    #[serde(default)]
    pub exige_propriedade_estatal: bool,
}

/// Definição de um método de produção (`metodos_producao.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetodoProducao {
    pub id: String,
    pub nome: String,
    pub slot: SlotMetodo,
    /// Bens consumidos por nível de prédio por dia (`bem_id -> quantidade`).
    #[serde(default)]
    pub insumos: BTreeMap<String, f64>,
    /// Bens produzidos por nível de prédio por dia (`bem_id -> quantidade`).
    #[serde(default)]
    pub produtos: BTreeMap<String, f64>,
    /// Vagas de trabalho por nível de prédio (`tipo_pop_id -> delta_trabalhadores`).
    /// Pode ser negativo em métodos do slot `Automacao` (poupadores de mão de obra).
    #[serde(default)]
    pub empregos: BTreeMap<String, i32>,
    /// Define o regime de propriedade quando usado no slot `Propriedade` (ou método único).
    #[serde(default)]
    pub regime_propriedade: Option<RegimePropriedade>,
    /// Pontos de construção civil gerados por nível por dia quando totalmente ocupado.
    #[serde(default)]
    pub pontos_construcao_gerados: f64,
    /// Capacidade burocrática gerada por nível por dia.
    #[serde(default)]
    pub burocracia_gerada: f64,
    /// Projeção/capacidade militar gerada por nível por dia.
    #[serde(default)]
    pub forca_militar_gerada: f64,
    /// Infraestrutura local gerada por nível.
    #[serde(default)]
    pub infraestrutura_gerada: f64,
}

/// Definição de um arquétipo de prédio (`tipos_predio.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipoPredio {
    pub id: String,
    pub nome: String,
    pub categoria: CategoriaPredio,
    /// Quantidade total de pontos de construção necessários para erguer 1 nível.
    pub custo_pontos_construcao: f64,
    /// Se `true`, cada nível consome 1 unidade de `terra_aravel_total` da província.
    #[serde(default)]
    pub usa_terra_aravel: bool,
    /// Se preenchido, limita o nível máximo na província a `limites_recursos[recurso]`.
    #[serde(default)]
    pub recurso_natural_requerido: Option<String>,
    /// Se `true`, salários e insumos são custeados pelo orçamento nacional (administração, quartéis, construção estatal).
    #[serde(default)]
    pub financiado_pelo_governo: bool,
    /// Lista de IDs de `MetodoProducao` compatíveis com este tipo de prédio.
    pub metodos_permitidos: Vec<String>,
    /// Métodos de produção ativados por padrão quando um novo prédio é construído.
    pub metodos_padrao: Vec<String>,
}

/// Representação pré-indexada de um candidato de substituição para execução rápida no tick.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CandidatoSubstituicao {
    pub bem_idx: usize,
    pub peso: f64,
    pub fracao_maxima: f64,
    pub fracao_minima: f64,
}

/// Representação pré-indexada de um `MetodoProducao`.
#[derive(Debug, Clone)]
pub(crate) struct MetodoCompilado {
    pub slot: SlotMetodo,
    pub insumos: Vec<(usize, f64)>,
    pub produtos: Vec<(usize, f64)>,
    pub empregos: Vec<(usize, i32)>,
    pub regime_propriedade: Option<RegimePropriedade>,
    pub pontos_construcao_gerados: f64,
    pub burocracia_gerada: f64,
    pub forca_militar_gerada: f64,
    pub infraestrutura_gerada: f64,
}

/// Resumo pré-compilado dos métodos padrão de um `TipoPredio` para avaliação rápida de obras privadas.
#[derive(Debug, Clone, Default)]
pub(crate) struct TipoPredioPadraoCompilado {
    pub financiado_pelo_governo: bool,
    pub custo_pontos_construcao: f64,
    pub insumos_padrao: Vec<(usize, f64)>,
    pub produtos_padrao: Vec<(usize, f64)>,
    pub folha_base_padrao: f64,
}

/// Catálogo completo e validado de regras estáticas da economia.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogoEconomia {
    pub bens: Vec<Bem>,
    pub tipos_pop: Vec<TipoPop>,
    pub leis: Vec<LeiEconomica>,
    pub metodos_producao: Vec<MetodoProducao>,
    pub tipos_predio: Vec<TipoPredio>,

    #[serde(skip)]
    pub(crate) indice_bens: BTreeMap<String, usize>,
    #[serde(skip)]
    pub(crate) indice_tipos_pop: BTreeMap<String, usize>,
    #[serde(skip)]
    pub(crate) indice_leis: BTreeMap<String, usize>,
    #[serde(skip)]
    pub(crate) indice_metodos: BTreeMap<String, usize>,
    #[serde(skip)]
    pub(crate) indice_tipos_predio: BTreeMap<String, usize>,
    #[serde(skip)]
    pub(crate) candidatos_por_necessidade: [Vec<CandidatoSubstituicao>; NUM_NECESSIDADES],
    #[serde(skip)]
    pub(crate) metodos_compilados: Vec<MetodoCompilado>,
    #[serde(skip)]
    pub(crate) tipos_predio_padrao: Vec<TipoPredioPadraoCompilado>,
    #[serde(skip)]
    pub(crate) indice_recursos: BTreeMap<String, u16>,
    #[serde(skip)]
    pub(crate) recurso_idx_por_tipo_predio: Vec<Option<u16>>,
    #[serde(skip)]
    pub(crate) tipo_pop_campones_idx: Option<usize>,
}

/// Função auxiliar genérica para desserializar um arquivo RON com contexto de erro detalhado.
pub fn ler_arquivo_ron<T: for<'de> Deserialize<'de>>(caminho: &Path) -> ResultadoEconomia<T> {
    let conteudo = fs::read_to_string(caminho).map_err(|origem| ErroEconomia::Io {
        caminho: caminho.to_path_buf(),
        origem,
    })?;
    ron::from_str(&conteudo).map_err(|origem| ErroEconomia::DesserializacaoRon {
        contexto: caminho.display().to_string(),
        origem: Box::new(origem),
    })
}

/// Função auxiliar genérica para desserializar uma string RON com contexto de erro.
pub fn ler_str_ron<T: for<'de> Deserialize<'de>>(
    conteudo: &str,
    contexto: &str,
) -> ResultadoEconomia<T> {
    ron::from_str(conteudo).map_err(|origem| ErroEconomia::DesserializacaoRon {
        contexto: contexto.to_string(),
        origem: Box::new(origem),
    })
}

impl CatalogoEconomia {
    /// Carrega o catálogo estático a partir de um diretório contendo:
    /// `bens.ron`, `tipos_pop.ron`, `leis_economicas.ron`, `metodos_producao.ron` e `tipos_predio.ron`.
    pub fn carregar_diretorio(dir: &Path) -> ResultadoEconomia<Self> {
        let bens: Vec<Bem> = ler_arquivo_ron(&dir.join("bens.ron"))?;
        let tipos_pop: Vec<TipoPop> = ler_arquivo_ron(&dir.join("tipos_pop.ron"))?;
        let leis: Vec<LeiEconomica> = ler_arquivo_ron(&dir.join("leis_economicas.ron"))?;
        let metodos_producao: Vec<MetodoProducao> =
            ler_arquivo_ron(&dir.join("metodos_producao.ron"))?;
        let tipos_predio: Vec<TipoPredio> = ler_arquivo_ron(&dir.join("tipos_predio.ron"))?;

        let mut catalogo = Self {
            bens,
            tipos_pop,
            leis,
            metodos_producao,
            tipos_predio,
            ..Default::default()
        };
        catalogo.validar_e_indexar()?;
        Ok(catalogo)
    }

    /// Valida todos os identificadores `snake_case`, valores numéricos e referências cruzadas,
    /// construindo os índices rápidos de execução.
    pub fn validar_e_indexar(&mut self) -> ResultadoEconomia<()> {
        self.indice_bens.clear();
        self.indice_tipos_pop.clear();
        self.indice_leis.clear();
        self.indice_metodos.clear();
        self.indice_tipos_predio.clear();
        self.candidatos_por_necessidade = Default::default();
        self.metodos_compilados.clear();
        self.tipos_predio_padrao.clear();
        self.indice_recursos.clear();
        self.recurso_idx_por_tipo_predio.clear();
        self.tipo_pop_campones_idx = None;

        // 1. Bens
        for (idx, bem) in self.bens.iter().enumerate() {
            validar_id_snake_case("Bem", &bem.id)?;
            if self.indice_bens.insert(bem.id.clone(), idx).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "Bem",
                    id: bem.id.clone(),
                });
            }
            if !bem.preco_base.is_finite() || bem.preco_base <= 0.0 {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "Bem",
                    id: bem.id.clone(),
                    detalhe: "preco_base deve ser finito e estritamente positivo".into(),
                });
            }
            for sub in &bem.substituicoes {
                if !sub.peso.is_finite() || sub.peso <= 0.0 {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "Bem",
                        id: bem.id.clone(),
                        detalhe: "peso de substituição deve ser > 0.0".into(),
                    });
                }
                if !sub.fracao_maxima.is_finite()
                    || !sub.fracao_minima.is_finite()
                    || sub.fracao_minima < 0.0
                    || sub.fracao_maxima > 1.0
                    || sub.fracao_minima > sub.fracao_maxima
                    || sub.fracao_maxima <= 0.0
                {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "Bem",
                        id: bem.id.clone(),
                        detalhe: "limites de fração de substituição inválidos".into(),
                    });
                }
                self.candidatos_por_necessidade[sub.necessidade.indice()].push(
                    CandidatoSubstituicao {
                        bem_idx: idx,
                        peso: sub.peso,
                        fracao_maxima: sub.fracao_maxima,
                        fracao_minima: sub.fracao_minima,
                    },
                );
            }
        }

        // 2. TiposPop
        for (idx, tp) in self.tipos_pop.iter().enumerate() {
            validar_id_snake_case("TipoPop", &tp.id)?;
            if self.indice_tipos_pop.insert(tp.id.clone(), idx).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "TipoPop",
                    id: tp.id.clone(),
                });
            }
            if !tp.multiplicador_salario.is_finite() || tp.multiplicador_salario <= 0.0 {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "TipoPop",
                    id: tp.id.clone(),
                    detalhe: "multiplicador_salario deve ser finito e > 0.0".into(),
                });
            }
            if !tp.alfabetizacao_minima.is_finite()
                || !(0.0..=1.0).contains(&tp.alfabetizacao_minima)
            {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "TipoPop",
                    id: tp.id.clone(),
                    detalhe: "alfabetizacao_minima deve estar entre 0.0 e 1.0".into(),
                });
            }
            if tp.subsistencia && self.tipo_pop_campones_idx.is_none() {
                self.tipo_pop_campones_idx = Some(idx);
            }
        }

        // 3. Leis Econômicas
        for (idx, lei) in self.leis.iter().enumerate() {
            validar_id_snake_case("LeiEconomica", &lei.id)?;
            if self.indice_leis.insert(lei.id.clone(), idx).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "LeiEconomica",
                    id: lei.id.clone(),
                });
            }
            if !lei.multiplicador_eficiencia_construcao.is_finite()
                || lei.multiplicador_eficiencia_construcao <= 0.0
                || !lei.multiplicador_volume_comercio.is_finite()
                || lei.multiplicador_volume_comercio < 0.0
            {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "LeiEconomica",
                    id: lei.id.clone(),
                    detalhe: "multiplicadores da lei econômica inválidos".into(),
                });
            }
        }

        // 4. Métodos de Produção
        for (idx, metodo) in self.metodos_producao.iter().enumerate() {
            validar_id_snake_case("MetodoProducao", &metodo.id)?;
            if self.indice_metodos.insert(metodo.id.clone(), idx).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "MetodoProducao",
                    id: metodo.id.clone(),
                });
            }

            let mut insumos_comp = Vec::with_capacity(metodo.insumos.len());
            for (bem_id, &qtd) in &metodo.insumos {
                let &bem_idx = self.indice_bens.get(bem_id).ok_or_else(|| {
                    ErroEconomia::ReferenciaInexistente {
                        origem_entidade: "MetodoProducao",
                        origem_id: metodo.id.clone(),
                        alvo_entidade: "Bem",
                        alvo_id: bem_id.clone(),
                    }
                })?;
                if !qtd.is_finite() || qtd < 0.0 {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "MetodoProducao",
                        id: metodo.id.clone(),
                        detalhe: format!("quantidade de insumo '{bem_id}' inválida"),
                    });
                }
                if qtd > 0.0 {
                    insumos_comp.push((bem_idx, qtd));
                }
            }

            let mut produtos_comp = Vec::with_capacity(metodo.produtos.len());
            for (bem_id, &qtd) in &metodo.produtos {
                let &bem_idx = self.indice_bens.get(bem_id).ok_or_else(|| {
                    ErroEconomia::ReferenciaInexistente {
                        origem_entidade: "MetodoProducao",
                        origem_id: metodo.id.clone(),
                        alvo_entidade: "Bem",
                        alvo_id: bem_id.clone(),
                    }
                })?;
                if !qtd.is_finite() || qtd < 0.0 {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "MetodoProducao",
                        id: metodo.id.clone(),
                        detalhe: format!("quantidade de produto '{bem_id}' inválida"),
                    });
                }
                if qtd > 0.0 {
                    produtos_comp.push((bem_idx, qtd));
                }
            }

            let mut empregos_comp = Vec::with_capacity(metodo.empregos.len());
            for (tp_id, &vagas) in &metodo.empregos {
                let &tp_idx = self.indice_tipos_pop.get(tp_id).ok_or_else(|| {
                    ErroEconomia::ReferenciaInexistente {
                        origem_entidade: "MetodoProducao",
                        origem_id: metodo.id.clone(),
                        alvo_entidade: "TipoPop",
                        alvo_id: tp_id.clone(),
                    }
                })?;
                if vagas != 0 {
                    empregos_comp.push((tp_idx, vagas));
                }
            }

            self.metodos_compilados.push(MetodoCompilado {
                slot: metodo.slot,
                insumos: insumos_comp,
                produtos: produtos_comp,
                empregos: empregos_comp,
                regime_propriedade: metodo.regime_propriedade,
                pontos_construcao_gerados: metodo.pontos_construcao_gerados.max(0.0),
                burocracia_gerada: metodo.burocracia_gerada.max(0.0),
                forca_militar_gerada: metodo.forca_militar_gerada.max(0.0),
                infraestrutura_gerada: metodo.infraestrutura_gerada.max(0.0),
            });
        }

        // 5. Tipos de Prédio
        for (idx, tp) in self.tipos_predio.iter().enumerate() {
            validar_id_snake_case("TipoPredio", &tp.id)?;
            if self
                .indice_tipos_predio
                .insert(tp.id.clone(), idx)
                .is_some()
            {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "TipoPredio",
                    id: tp.id.clone(),
                });
            }
            if !tp.custo_pontos_construcao.is_finite() || tp.custo_pontos_construcao <= 0.0 {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "TipoPredio",
                    id: tp.id.clone(),
                    detalhe: "custo_pontos_construcao deve ser finito e > 0.0".into(),
                });
            }
            let rec_opt = if let Some(recurso) = &tp.recurso_natural_requerido {
                validar_id_snake_case("recurso_natural_requerido", recurso)?;
                let prox_id = self.indice_recursos.len() as u16;
                let r_idx = *self
                    .indice_recursos
                    .entry(recurso.clone())
                    .or_insert(prox_id);
                Some(r_idx)
            } else {
                None
            };
            self.recurso_idx_por_tipo_predio.push(rec_opt);
            if tp.metodos_permitidos.is_empty() || tp.metodos_padrao.is_empty() {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "TipoPredio",
                    id: tp.id.clone(),
                    detalhe: "metodos_permitidos e metodos_padrao não podem ser vazios".into(),
                });
            }
            for m_id in &tp.metodos_permitidos {
                if !self.indice_metodos.contains_key(m_id) {
                    return Err(ErroEconomia::ReferenciaInexistente {
                        origem_entidade: "TipoPredio",
                        origem_id: tp.id.clone(),
                        alvo_entidade: "MetodoProducao",
                        alvo_id: m_id.clone(),
                    });
                }
            }
            for m_id in &tp.metodos_padrao {
                if !tp.metodos_permitidos.contains(m_id) {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "TipoPredio",
                        id: tp.id.clone(),
                        detalhe: format!(
                            "método padrão '{m_id}' precisa constar em metodos_permitidos"
                        ),
                    });
                }
            }
            self.validar_slots_metodos("TipoPredio", &tp.id, &tp.metodos_padrao)?;

            let mut ins_map = BTreeMap::new();
            let mut prod_map = BTreeMap::new();
            let mut vagas_map = BTreeMap::new();
            for m_id in &tp.metodos_padrao {
                let m_idx = self.indice_metodos[m_id];
                let mc = &self.metodos_compilados[m_idx];
                for &(g, q) in &mc.insumos {
                    *ins_map.entry(g).or_insert(0.0) += q;
                }
                for &(g, q) in &mc.produtos {
                    *prod_map.entry(g).or_insert(0.0) += q;
                }
                for &(pop_t, v) in &mc.empregos {
                    *vagas_map.entry(pop_t).or_insert(0i32) += v;
                }
            }
            let mut folha_base = 0.0;
            for (pop_t, v) in vagas_map {
                if v > 0 {
                    folha_base += (v as f64) * 4.0 * self.tipos_pop[pop_t].multiplicador_salario;
                }
            }
            self.tipos_predio_padrao.push(TipoPredioPadraoCompilado {
                financiado_pelo_governo: tp.financiado_pelo_governo,
                custo_pontos_construcao: tp.custo_pontos_construcao,
                insumos_padrao: ins_map.into_iter().filter(|&(_, q)| q > 0.0).collect(),
                produtos_padrao: prod_map.into_iter().filter(|&(_, q)| q > 0.0).collect(),
                folha_base_padrao: folha_base,
            });
        }

        Ok(())
    }

    /// Garante que uma lista de métodos ativos não possua dois métodos disputando o mesmo `SlotMetodo`.
    pub(crate) fn validar_slots_metodos(
        &self,
        entidade: &'static str,
        id: &str,
        metodos: &[String],
    ) -> ResultadoEconomia<()> {
        let mut slots_vistos = [false; 4];
        for m_id in metodos {
            let &m_idx = self.indice_metodos.get(m_id).ok_or_else(|| {
                ErroEconomia::ReferenciaInexistente {
                    origem_entidade: entidade,
                    origem_id: id.to_string(),
                    alvo_entidade: "MetodoProducao",
                    alvo_id: m_id.clone(),
                }
            })?;
            let slot_idx = match self.metodos_compilados[m_idx].slot {
                SlotMetodo::ProducaoPrincipal => 0,
                SlotMetodo::Automacao => 1,
                SlotMetodo::Propriedade => 2,
                SlotMetodo::Secundario => 3,
            };
            if slots_vistos[slot_idx] {
                return Err(ErroEconomia::ValorInvalido {
                    entidade,
                    id: id.to_string(),
                    detalhe: format!("mais de um método ativo para o mesmo slot ('{m_id}')"),
                });
            }
            slots_vistos[slot_idx] = true;
        }
        Ok(())
    }

    /// Busca o índice interno de um `Bem` pelo seu `id`.
    #[inline]
    pub fn indice_bem(&self, id: &str) -> Option<usize> {
        self.indice_bens.get(id).copied()
    }

    /// Busca um `Bem` pelo seu `id`.
    #[inline]
    pub fn obter_bem(&self, id: &str) -> Option<&Bem> {
        self.indice_bem(id).map(|i| &self.bens[i])
    }

    /// Busca o índice interno de um `TipoPop` pelo seu `id`.
    #[inline]
    pub fn indice_tipo_pop(&self, id: &str) -> Option<usize> {
        self.indice_tipos_pop.get(id).copied()
    }

    /// Busca um `TipoPop` pelo seu `id`.
    #[inline]
    pub fn obter_tipo_pop(&self, id: &str) -> Option<&TipoPop> {
        self.indice_tipo_pop(id).map(|i| &self.tipos_pop[i])
    }

    /// Busca uma `LeiEconomica` pelo seu `id`.
    #[inline]
    pub fn obter_lei(&self, id: &str) -> Option<&LeiEconomica> {
        self.indice_leis.get(id).map(|&i| &self.leis[i])
    }

    /// Busca um `MetodoProducao` pelo seu `id`.
    #[inline]
    pub fn obter_metodo(&self, id: &str) -> Option<&MetodoProducao> {
        self.indice_metodos
            .get(id)
            .map(|&i| &self.metodos_producao[i])
    }

    /// Busca um `TipoPredio` pelo seu `id`.
    #[inline]
    pub fn obter_tipo_predio(&self, id: &str) -> Option<&TipoPredio> {
        self.indice_tipos_predio
            .get(id)
            .map(|&i| &self.tipos_predio[i])
    }
}
