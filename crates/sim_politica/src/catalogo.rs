//! Catálogo de definições estáticas do mundo político:
//! `Ideologia`, `DefinicaoLei`, `CategoriaLeis`, `DefinicaoGrupoInteresse`, `ArquetipoPartido` e `CatalogoPolitica`.

use crate::erro::{ErroPolitica, ResultadoPolitica, validar_id_snake_case};
use crate::tipos::{AlinhamentoBloco, PosturaLei};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Modificador numérico de atributo ou efeito governamental.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modificador {
    pub id: String,
    pub valor: f64,
}

/// Pré-requisitos e incompatibilidades para a promulgação de uma lei.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreRequisitos {
    #[serde(default)]
    pub leis_obrigatorias: Vec<String>,
    #[serde(default)]
    pub leis_proibidas: Vec<String>,
    #[serde(default)]
    pub somente_soberano: bool,
}

/// Definição estática de uma lei institucional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefinicaoLei {
    pub id: String,
    pub nome: String,
    pub descricao: String,
    pub grupo_lei: String,
    #[serde(default)]
    pub efeitos: Vec<Modificador>,
    #[serde(default)]
    pub pre_requisitos: PreRequisitos,
    #[serde(default)]
    pub apoiadores: Vec<String>,
    #[serde(default)]
    pub opositores: Vec<String>,

    // Atributos operacionais do sistema eleitoral e distribuição de poder
    #[serde(default)]
    pub sem_eleicoes: bool,
    #[serde(default)]
    pub eleicoes_partido_unico: bool,
    #[serde(default)]
    pub voto_censitario: bool,
    #[serde(default)]
    pub voto_capacitario: bool,
    #[serde(default)]
    pub sufragio_universal: bool,
    #[serde(default)]
    pub sufragio_masculino: bool,
    #[serde(default)]
    pub limiar_padrao_vida_voto: Option<f64>,
    #[serde(default)]
    pub alfabetizacao_minima_voto: Option<f64>,
    #[serde(default)]
    pub multiplicadores_estrato: Option<[f64; 3]>,
}

impl DefinicaoLei {
    /// Avalia o peso eleitoral de um Pop de determinado estrato, alfabetização e SoL perante esta lei.
    pub fn multiplicador_eleitoral_pop(
        &self,
        estrato_idx: usize,
        alfabetizacao: f64,
        sol: f64,
    ) -> f64 {
        if self.sem_eleicoes {
            // Em autocracia/sem eleições, voto popular formal não existe
            return 0.0;
        }

        if self.eleicoes_partido_unico {
            // Eleições de vanguarda enquadram todos os cidadãos em listas plebiscitárias
            return 1.0;
        }

        if self.voto_censitario {
            let limiar = self.limiar_padrao_vida_voto.unwrap_or(14.0);
            if sol < limiar {
                return 0.0;
            }
            return 1.0 + 0.08 * (sol - limiar);
        }

        if self.voto_capacitario {
            let min_alfa = self.alfabetizacao_minima_voto.unwrap_or(0.35);
            if alfabetizacao < min_alfa {
                return 0.0;
            }
            return 0.5 + 1.2 * alfabetizacao;
        }

        if self.sufragio_masculino {
            // Representa metade da população adulta
            return 0.50;
        }

        if self.sufragio_universal {
            return 1.0;
        }

        // Padrão caso não seja lei especificamente eleitoral: usa multiplicadores de estrato se houver
        if let Some(pesos) = self.multiplicadores_estrato
            && estrato_idx < 3
        {
            return pesos[estrato_idx];
        }

        1.0
    }
}

/// Categoria institucional de leis (ex.: `"forma_governo"`, `"sistema_eleitoral"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoriaLeis {
    pub categoria: String,
    pub nome: String,
    pub descricao: String,
    pub leis: Vec<DefinicaoLei>,
}

/// Regra de atração social de uma profissão (tipo de Pop) para um Grupo de Interesse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseSocialPop {
    pub tipo_pop: String,
    pub peso_atracao: f64,
}

/// Interesse econômico de um GI no preço e disponibilidade de determinado bem de consumo ou insumo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteresseBem {
    pub bem_id: String,
    pub peso: f64,
    /// Se `true`, o grupo deseja preços baixos (ex.: operários querendo comida barata);
    /// se `false`, o grupo lucra com preços elevados (ex.: industriais querendo aço ou latifundiários querendo café).
    pub deseja_barato: bool,
}

/// Definição estática de um Grupo de Interesse (GI).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefinicaoGrupoInteresse {
    pub id: String,
    pub nome: String,
    pub descricao: String,
    pub ideologias: Vec<String>,
    pub base_social_pops: Vec<BaseSocialPop>,
    #[serde(default)]
    pub leis_preferidas: Vec<String>,
    #[serde(default)]
    pub leis_odiadas: Vec<String>,
    #[serde(default)]
    pub bens_importantes: Vec<InteresseBem>,
    #[serde(default)]
    pub modificadores_leal: Vec<Modificador>,
    #[serde(default)]
    pub modificadores_insurrecto: Vec<Modificador>,
}

impl DefinicaoGrupoInteresse {
    /// Determina a postura padrão do grupo em relação a uma lei específica.
    pub fn postura_padrao_lei(&self, lei_id: &str) -> PosturaLei {
        if self.leis_preferidas.iter().any(|id| id == lei_id) {
            PosturaLei::Apoia
        } else if self.leis_odiadas.iter().any(|id| id == lei_id) {
            PosturaLei::Opoe
        } else {
            PosturaLei::Neutro
        }
    }
}

/// Definição estática de uma corrente ideológica.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ideologia {
    pub id: String,
    pub nome: String,
    pub descricao: String,
    pub alinhamento_padrao: AlinhamentoBloco,
    pub principios: Vec<String>,
    #[serde(default)]
    pub leis_favorecidas: Vec<String>,
    #[serde(default)]
    pub leis_rejeitadas: Vec<String>,
}

impl Ideologia {
    /// Postura ditada por esta ideologia para uma lei.
    pub fn postura_lei(&self, lei_id: &str) -> PosturaLei {
        if self.leis_favorecidas.iter().any(|id| id == lei_id) {
            PosturaLei::EndossaFortemente
        } else if self.leis_rejeitadas.iter().any(|id| id == lei_id) {
            PosturaLei::RejeitaFortemente
        } else {
            PosturaLei::Neutro
        }
    }

    /// Calcula a distância ideológica normalizada `0.0..=1.0` entre esta ideologia e outra.
    pub fn distancia(&self, outra: &Ideologia) -> f64 {
        if self.id == outra.id {
            return 0.0;
        }

        // Confronto direto entre blocos antagônicos
        let mut dist_base = match (self.alinhamento_padrao, outra.alinhamento_padrao) {
            (AlinhamentoBloco::BlocoOcidental, AlinhamentoBloco::BlocoSovietico)
            | (AlinhamentoBloco::BlocoSovietico, AlinhamentoBloco::BlocoOcidental) => 0.85,
            (AlinhamentoBloco::EixoDerrotado, _) | (_, AlinhamentoBloco::EixoDerrotado) => 0.90,
            (a, b) if a == b => 0.20,
            _ => 0.50,
        };

        // Sobreposição de princípios comuns reduz a distância
        let principios_a: BTreeSet<&str> = self.principios.iter().map(|s| s.as_str()).collect();
        let principios_b: BTreeSet<&str> = outra.principios.iter().map(|s| s.as_str()).collect();
        let comuns = principios_a.intersection(&principios_b).count();

        if comuns > 0 {
            dist_base = (dist_base - 0.12 * comuns as f64).max(0.05);
        }

        dist_base.clamp(0.0, 1.0)
    }
}

/// Arquétipo padrão para formação dinâmica de partidos políticos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArquetipoPartido {
    pub id: String,
    pub nome: String,
    pub descricao: String,
    pub ideologias_afins: Vec<String>,
    pub grupos_naturais: Vec<String>,
    #[serde(default)]
    pub prioridade: u32,
}

/// Catálogo global de definições políticas do jogo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CatalogoPolitica {
    pub ideologias: BTreeMap<String, Ideologia>,
    pub categorias_leis: BTreeMap<String, CategoriaLeis>,
    pub leis: BTreeMap<String, DefinicaoLei>,
    pub grupos_interesse: BTreeMap<String, DefinicaoGrupoInteresse>,
    pub arquetipos_partido: BTreeMap<String, ArquetipoPartido>,
}

impl CatalogoPolitica {
    /// Valida e indexa todas as referências cruzadas do catálogo.
    pub fn validar(&self) -> ResultadoPolitica<()> {
        // Validação de identificadores de ideologias
        for (id, ideol) in &self.ideologias {
            validar_id_snake_case("Ideologia", id)?;
            if ideol.id != *id {
                return Err(ErroPolitica::ValorInvalido {
                    entidade: "Ideologia",
                    id: id.clone(),
                    detalhe: "chave no mapa difere do campo id interno".into(),
                });
            }
        }

        // Validação de categorias e leis
        for (cat_id, cat) in &self.categorias_leis {
            validar_id_snake_case("CategoriaLeis", cat_id)?;
            for lei in &cat.leis {
                validar_id_snake_case("DefinicaoLei", &lei.id)?;
                if lei.grupo_lei != *cat_id {
                    return Err(ErroPolitica::ValorInvalido {
                        entidade: "DefinicaoLei",
                        id: lei.id.clone(),
                        detalhe: format!(
                            "lei está na categoria '{}' mas seu grupo_lei é '{}'",
                            cat_id, lei.grupo_lei
                        ),
                    });
                }
            }
        }

        // Validação de grupos de interesse
        for (gi_id, gi) in &self.grupos_interesse {
            validar_id_snake_case("DefinicaoGrupoInteresse", gi_id)?;
            for ideol_id in &gi.ideologias {
                if !self.ideologias.contains_key(ideol_id) {
                    return Err(ErroPolitica::ReferenciaInexistente {
                        origem_entidade: "DefinicaoGrupoInteresse",
                        origem_id: gi_id.clone(),
                        alvo_entidade: "Ideologia",
                        alvo_id: ideol_id.clone(),
                    });
                }
            }
        }

        // Validação de partidos
        for (part_id, part) in &self.arquetipos_partido {
            validar_id_snake_case("ArquetipoPartido", part_id)?;
            for gi_id in &part.grupos_naturais {
                if !self.grupos_interesse.contains_key(gi_id) {
                    return Err(ErroPolitica::ReferenciaInexistente {
                        origem_entidade: "ArquetipoPartido",
                        origem_id: part_id.clone(),
                        alvo_entidade: "DefinicaoGrupoInteresse",
                        alvo_id: gi_id.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Desserializa um catálogo a partir de uma string no formato RON.
    pub fn de_str_ron(conteudo: &str) -> ResultadoPolitica<Self> {
        let cat: Self =
            ron::from_str(conteudo).map_err(|origem| ErroPolitica::DesserializacaoRon {
                contexto: "CatalogoPolitica::de_str_ron".into(),
                origem,
            })?;
        cat.validar()?;
        Ok(cat)
    }

    /// Carrega e valida um catálogo a partir de um arquivo RON em disco.
    pub fn de_arquivo_ron<P: AsRef<Path>>(caminho: P) -> ResultadoPolitica<Self> {
        let caminho_buf = caminho.as_ref().to_path_buf();
        let conteudo = fs::read_to_string(&caminho_buf).map_err(|origem| ErroPolitica::Io {
            caminho: caminho_buf.clone(),
            origem,
        })?;
        let mut cat = Self::de_str_ron(&conteudo)?;

        // Preenche o índice plano de leis se ainda não estiver preenchido
        if cat.leis.is_empty() {
            for cat_leis in cat.categorias_leis.values() {
                for lei in &cat_leis.leis {
                    cat.leis.insert(lei.id.clone(), lei.clone());
                }
            }
        }

        cat.validar()?;
        Ok(cat)
    }
}
