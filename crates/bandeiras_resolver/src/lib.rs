//! Resolvedor de bandeira dinamica (puro, sem Bevy).
//!
//! `resolver_bandeira(tag, &EstadoPais) -> VarianteBandeira` escolhe, entre as variantes de
//! `variantes.ron`, a de condicao MAIS ESPECIFICA (mais campos exigidos); empate -> maior
//! `prioridade`; novo empate -> a primeira declarada. Sem nenhuma que case, devolve a base.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
pub enum Status {
    #[default]
    Independente,
    Colonia,
    Protetorado,
    Satelite,
    Fantoche,
}

/// Composicao feita pela UI em tempo de execucao sobre o arquivo da variante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
pub enum Sobreposicao {
    #[default]
    Nenhuma,
    CantaoSuserano,
    CantaoOcupante,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Condicao {
    pub forma_governo: Vec<String>,
    pub categoria_ideologia: Vec<String>,
    pub status: Vec<Status>,
    pub religiao_estatal: Vec<String>,
    pub flags: Vec<String>,
    pub partido: Vec<String>,
    pub exige_ocupado: bool,
    pub exige_nao_ocupado: bool,
    pub exige_guerra_civil: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Variante {
    pub id: String,
    pub arquivo: String,
    pub nome: String,
    pub cor: (u8, u8, u8),
    #[serde(default)]
    pub condicao: Condicao,
    #[serde(default)]
    pub prioridade: i32,
    #[serde(default)]
    pub historica: bool,
    #[serde(default)]
    pub sobreposicao: Sobreposicao,
    #[serde(default)]
    pub descricao: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaisBandeiras {
    pub tag: String,
    pub base: Variante,
    pub variantes: Vec<Variante>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dados {
    pub versao: u32,
    pub categorias_ideologia: HashMap<String, String>,
    pub paises: Vec<PaisBandeiras>,
}

/// Estado do pais relevante para a aparencia. A simulacao preenche isto a cada decisao/tick.
#[derive(Debug, Clone, Default)]
pub struct EstadoPais {
    pub ano: i32,
    pub mes: u8,
    /// id da lei de forma de governo (ex.: "republica_parlamentarista")
    pub forma_governo: String,
    /// id da lei de religiao estatal (ex.: "religiao_oficial_obrigatoria")
    pub religiao_estatal: String,
    pub ideologia_dominante: String,
    pub ideologia_partido: Option<String>,
    pub ideologia_chefe: Option<String>,
    pub partido: Option<String>,
    pub status: Status,
    pub suserano: Option<String>,
    pub ocupante: Option<String>,
    pub guerra_civil: bool,
    /// marcadores de eventos (ex.: "rfa", "uniao_arabe", "reunificacao_alema")
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarianteBandeira {
    pub id: String,
    pub arquivo: String,
    pub nome: String,
    pub cor: (u8, u8, u8),
    pub sobreposicao: Sobreposicao,
    /// tag da bandeira a compor no cantao (suserano ou ocupante), se houver sobreposicao
    pub tag_cantao: Option<String>,
    pub historica: bool,
    pub e_base: bool,
}

pub struct Catalogo {
    dados: Dados,
    indice: HashMap<String, usize>,
}

impl EstadoPais {
    /// Ideologia efetiva: partido governante > chefe de estado > ideologia dominante.
    pub fn ideologia_efetiva(&self) -> &str {
        self.ideologia_partido
            .as_deref()
            .or(self.ideologia_chefe.as_deref())
            .unwrap_or(&self.ideologia_dominante)
    }
}

impl Condicao {
    pub fn especificidade(&self) -> usize {
        usize::from(!self.forma_governo.is_empty())
            + usize::from(!self.categoria_ideologia.is_empty())
            + usize::from(!self.status.is_empty())
            + usize::from(!self.religiao_estatal.is_empty())
            + self.flags.len()
            + usize::from(!self.partido.is_empty())
            + usize::from(self.exige_ocupado)
            + usize::from(self.exige_nao_ocupado)
            + usize::from(self.exige_guerra_civil)
    }
}

impl Catalogo {
    pub fn de_str(texto: &str) -> Result<Self, ron::error::SpannedError> {
        let dados: Dados = ron::from_str(texto)?;
        let indice = dados.paises.iter().enumerate().map(|(i, p)| (p.tag.clone(), i)).collect();
        Ok(Self { dados, indice })
    }

    pub fn de_arquivo(caminho: &str) -> Result<Self, String> {
        let t = std::fs::read_to_string(caminho).map_err(|e| e.to_string())?;
        Self::de_str(&t).map_err(|e| e.to_string())
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.dados.paises.iter().map(|p| p.tag.as_str())
    }

    pub fn pais(&self, tag: &str) -> Option<&PaisBandeiras> {
        self.indice.get(tag).map(|&i| &self.dados.paises[i])
    }

    pub fn dados(&self) -> &Dados {
        &self.dados
    }

    fn casa(&self, c: &Condicao, e: &EstadoPais) -> bool {
        let cat = self.dados.categorias_ideologia.get(e.ideologia_efetiva()).map(String::as_str);
        (c.forma_governo.is_empty() || c.forma_governo.contains(&e.forma_governo))
            && (c.categoria_ideologia.is_empty() || cat.is_some_and(|k| c.categoria_ideologia.iter().any(|x| x == k)))
            && (c.status.is_empty() || c.status.contains(&e.status))
            && (c.religiao_estatal.is_empty() || c.religiao_estatal.contains(&e.religiao_estatal))
            && c.flags.iter().all(|f| e.flags.contains(f))
            && (c.partido.is_empty() || e.partido.as_ref().is_some_and(|p| c.partido.contains(p)))
            && (!c.exige_ocupado || e.ocupante.is_some())
            && (!c.exige_nao_ocupado || e.ocupante.is_none())
            && (!c.exige_guerra_civil || e.guerra_civil)
    }

    pub fn resolver_bandeira(&self, tag: &str, e: &EstadoPais) -> VarianteBandeira {
        let Some(p) = self.pais(tag) else {
            return VarianteBandeira {
                id: "desconhecida".into(),
                arquivo: String::new(),
                nome: tag.into(),
                cor: (128, 128, 128),
                sobreposicao: Sobreposicao::Nenhuma,
                tag_cantao: None,
                historica: false,
                e_base: true,
            };
        };
        let mut melhor: Option<(&Variante, (usize, i32))> = None;
        for v in &p.variantes {
            if !self.casa(&v.condicao, e) {
                continue;
            }
            let chave = (v.condicao.especificidade(), v.prioridade);
            if melhor.is_none_or(|(_, k)| chave > k) {
                melhor = Some((v, chave));
            }
        }
        let (v, base) = match melhor {
            Some((v, _)) => (v, false),
            None => (&p.base, true),
        };
        let tag_cantao = match v.sobreposicao {
            Sobreposicao::CantaoSuserano => e.suserano.clone(),
            Sobreposicao::CantaoOcupante => e.ocupante.clone(),
            Sobreposicao::Nenhuma => None,
        };
        VarianteBandeira {
            id: v.id.clone(),
            arquivo: v.arquivo.clone(),
            nome: v.nome.clone(),
            cor: v.cor,
            sobreposicao: v.sobreposicao,
            tag_cantao,
            historica: v.historica,
            e_base: base,
        }
    }
}

static EMBUTIDO: OnceLock<Catalogo> = OnceLock::new();

/// Catalogo embutido no binario (`variantes.ron` do crate).
pub fn catalogo_embutido() -> &'static Catalogo {
    EMBUTIDO.get_or_init(|| {
        Catalogo::de_str(include_str!("../variantes.ron")).expect("variantes.ron invalido")
    })
}

/// Atalho pedido: resolve usando o catalogo embutido.
pub fn resolver_bandeira(tag: &str, estado: &EstadoPais) -> VarianteBandeira {
    catalogo_embutido().resolver_bandeira(tag, estado)
}
