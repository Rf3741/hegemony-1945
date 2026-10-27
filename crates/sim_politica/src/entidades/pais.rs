//! Entidade política nacional (`PaisPolitica`), reunindo leis, gabinete ministerial,
//! grupos de interesse, partidos, movimentos sociais, eleições e legitimidade.

use crate::entidades::crise::CriseInterna;
use crate::entidades::eleicao::{CampanhaEleitoral, ResultadoEleicao};
use crate::entidades::grupo_interesse::InstanciaGrupoInteresse;
use crate::entidades::influencia::RegistroInfluenciaExterna;
use crate::entidades::lei_tramitacao::TramitacaoLei;
use crate::entidades::lider::LiderPolitico;
use crate::entidades::movimento::MovimentoPolitico;
use crate::entidades::partido::InstanciaPartido;
use crate::entidades::pop::PopPolitica;
use crate::erro::{ErroPolitica, ResultadoPolitica, validar_id_snake_case};
use crate::tipos::{FaixaLegitimidade, PapelGrupoInteresse, StatusClout};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Entidade política e institucional de um país no cenário da Guerra Fria.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaisPolitica {
    /// Identificador único em snake_case ASCII (ex.: `"estados_unidos"` ou `"belgica"`).
    pub id: String,
    /// Tag diplomática de 3 caracteres maiúsculos ASCII (ex.: `"USA"`, `"SOV"`, `"BRA"`).
    pub tag: String,
    /// Nome de exibição em pt-BR.
    pub nome: String,
    /// Leis ativas por categoria institucional (`categoria_id -> lei_id`).
    pub leis_ativas: BTreeMap<String, String>,
    /// Grupos de Interesse atuantes no país (`gi_id -> instancia`).
    pub grupos_interesse: BTreeMap<String, InstanciaGrupoInteresse>,
    /// Partidos políticos organizados (`partido_id -> instancia`).
    #[serde(default)]
    pub partidos: BTreeMap<String, InstanciaPartido>,
    /// Culturas titulares da nação (fora delas, pops sofrem discriminação sob certas leis).
    #[serde(default)]
    pub culturas_primarias: Vec<String>,
    /// Coleção de grupos populacionais (Pops) residentes na nação.
    #[serde(default)]
    pub pops: Vec<PopPolitica>,
    /// Chefe de Estado em exercício (Presidente, Monarca ou Secretário-Geral).
    pub chefe_de_estado: Option<LiderPolitico>,
    /// ID do Grupo de Interesse ao qual o Chefe de Estado pertence ou é filiado.
    pub gi_chefe_estado: Option<String>,
    /// Pontuação de legitimidade do gabinete governamental (0.0 a 100.0).
    #[serde(default = "padrao_legitimidade")]
    pub legitimidade: f64,
    /// Faixa qualitativa da legitimidade.
    #[serde(default = "padrao_faixa_legitimidade")]
    pub faixa_legitimidade: FaixaLegitimidade,
    /// Projeto de lei em tramitação parlamentar (se houver).
    #[serde(default)]
    pub lei_em_tramitacao: Option<TramitacaoLei>,
    /// Movimentos políticos ativos no país.
    #[serde(default)]
    pub movimentos: Vec<MovimentoPolitico>,
    /// Crises sociopolíticas graves em andamento.
    #[serde(default)]
    pub crises_ativas: Vec<CriseInterna>,
    /// Campanha eleitoral em curso.
    #[serde(default)]
    pub campanha_eleitoral: Option<CampanhaEleitoral>,
    /// Ticks diários restantes até o próximo pleito eleitoral programado.
    #[serde(default = "padrao_intervalo_eleicao")]
    pub dias_para_proxima_eleicao: u32,
    /// Registro da última eleição ocorrida.
    #[serde(default)]
    pub ultima_eleicao: Option<ResultadoEleicao>,
    /// Registro da influência geopolítica das superpotências (EUA e URSS).
    #[serde(default)]
    pub influencia_externa: RegistroInfluenciaExterna,
    /// Preços atuais de bens no mercado nacional (para avaliação de aprovação de IGs).
    #[serde(default)]
    pub precos_mercado: BTreeMap<String, f64>,
    /// Se `true`, reconvoca automaticamente uma coalizão majoritária após cada eleição.
    #[serde(default = "padrao_true")]
    pub auto_reformar_governo_eleicao: bool,
}

const fn padrao_legitimidade() -> f64 {
    65.0
}

const fn padrao_faixa_legitimidade() -> FaixaLegitimidade {
    FaixaLegitimidade::Aceitavel
}

const fn padrao_intervalo_eleicao() -> u32 {
    1460 // 4 anos
}

const fn padrao_true() -> bool {
    true
}

impl PaisPolitica {
    /// Cria um novo país político com identificadores validados.
    pub fn novo(
        id: impl Into<String>,
        tag: impl Into<String>,
        nome: impl Into<String>,
    ) -> ResultadoPolitica<Self> {
        let id_str = id.into();
        validar_id_snake_case("PaisPolitica", &id_str)?;

        Ok(Self {
            id: id_str,
            tag: tag.into(),
            nome: nome.into(),
            leis_ativas: BTreeMap::new(),
            grupos_interesse: BTreeMap::new(),
            partidos: BTreeMap::new(),
            culturas_primarias: Vec::new(),
            pops: Vec::new(),
            chefe_de_estado: None,
            gi_chefe_estado: None,
            legitimidade: 65.0,
            faixa_legitimidade: FaixaLegitimidade::Aceitavel,
            lei_em_tramitacao: None,
            movimentos: Vec::new(),
            crises_ativas: Vec::new(),
            campanha_eleitoral: None,
            dias_para_proxima_eleicao: 1460,
            ultima_eleicao: None,
            influencia_externa: RegistroInfluenciaExterna::default(),
            precos_mercado: BTreeMap::new(),
            auto_reformar_governo_eleicao: true,
        })
    }

    /// Retorna a população nacional total somando todos os Pops.
    pub fn populacao_total(&self) -> f64 {
        self.pops.iter().map(|p| p.populacao).sum()
    }

    /// Retorna a quantidade total de cidadãos radicalizados.
    pub fn total_radicais(&self) -> f64 {
        self.pops.iter().map(|p| p.radicais).sum()
    }

    /// Retorna a quantidade total de cidadãos leais.
    pub fn total_leais(&self) -> f64 {
        self.pops.iter().map(|p| p.leais).sum()
    }

    /// Retorna a média ponderada do padrão de vida nacional.
    pub fn padrao_de_vida_medio(&self) -> f64 {
        let tot = self.populacao_total();
        if tot <= 0.0 {
            return 10.0;
        }
        let soma: f64 = self
            .pops
            .iter()
            .map(|p| p.padrao_de_vida * p.populacao)
            .sum();
        soma / tot
    }

    /// Retorna a taxa média ponderada de alfabetização nacional.
    pub fn alfabetizacao_media(&self) -> f64 {
        let tot = self.populacao_total();
        if tot <= 0.0 {
            return 0.35;
        }
        let soma: f64 = self
            .pops
            .iter()
            .map(|p| p.alfabetizacao * p.populacao)
            .sum();
        soma / tot
    }

    /// Reforma a composição ministerial do governo, admitindo apenas grupos elegíveis.
    pub fn reformar_governo(&mut self, novos_grupos_governo: &[String]) -> ResultadoPolitica<()> {
        let eh_partido_unico = self
            .leis_ativas
            .get("forma_governo")
            .map(|s| s.as_str() == "estado_partido_unico" || s.as_str() == "partido_unico")
            .unwrap_or(false);

        // Valida se todos os grupos existem e se algum grupo marginalizado está sendo incluído ilegalmente
        for gi_id in novos_grupos_governo {
            let gi = self.grupos_interesse.get(gi_id).ok_or_else(|| {
                ErroPolitica::ReferenciaInexistente {
                    origem_entidade: "PaisPolitica",
                    origem_id: self.id.clone(),
                    alvo_entidade: "InstanciaGrupoInteresse",
                    alvo_id: gi_id.clone(),
                }
            })?;

            if gi.status == StatusClout::Marginalizado && !eh_partido_unico {
                return Err(ErroPolitica::AcaoInvalida {
                    pais_id: self.id.clone(),
                    detalhe: format!(
                        "grupo '{}' está marginalizado (clout < 5%) e não pode integrar o governo",
                        gi_id
                    ),
                });
            }
        }

        // Aplica os novos papéis
        for (id, gi) in &mut self.grupos_interesse {
            if gi.status == StatusClout::Marginalizado && !eh_partido_unico {
                gi.papel = PapelGrupoInteresse::Marginalizado;
            } else if novos_grupos_governo.iter().any(|novo_id| novo_id == id) {
                gi.papel = PapelGrupoInteresse::NoGoverno;
            } else {
                gi.papel = PapelGrupoInteresse::NaOposicao;
            }
        }

        Ok(())
    }

    /// Inicia a tramitação formal de um novo projeto de lei.
    pub fn iniciar_tramitacao(
        &mut self,
        categoria: &str,
        nova_lei_id: &str,
    ) -> ResultadoPolitica<()> {
        if self.lei_em_tramitacao.is_some() {
            return Err(ErroPolitica::AcaoInvalida {
                pais_id: self.id.clone(),
                detalhe: "já existe uma lei em tramitação no país".into(),
            });
        }

        let lei_atual = self
            .leis_ativas
            .get(categoria)
            .cloned()
            .unwrap_or_else(|| "nenhuma".to_string());

        if lei_atual == nova_lei_id {
            return Err(ErroPolitica::AcaoInvalida {
                pais_id: self.id.clone(),
                detalhe: format!("a lei '{}' já está em vigor nesta categoria", nova_lei_id),
            });
        }

        self.lei_em_tramitacao = Some(TramitacaoLei::nova(categoria, nova_lei_id, lei_atual));
        Ok(())
    }

    /// Promulga imediatamente uma lei sem passar pelo processo de tramitação (para inicialização ou decretos).
    pub fn promulgar_lei_instantanea(&mut self, categoria: &str, nova_lei_id: &str) {
        self.leis_ativas
            .insert(categoria.to_string(), nova_lei_id.to_string());
        if let Some(tram) = &self.lei_em_tramitacao
            && tram.categoria == categoria
        {
            self.lei_em_tramitacao = None;
        }
    }
}
