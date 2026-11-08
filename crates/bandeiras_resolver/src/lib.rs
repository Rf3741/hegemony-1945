//! Resolvedor de bandeira dinamica (puro, sem Bevy).
//!
//! `resolver_bandeira(tag, &EstadoPais) -> VarianteBandeira` escolhe, entre as variantes de
//! `variantes.ron`, a de condicao MAIS ESPECIFICA (mais campos exigidos); empate -> maior
//! `prioridade`; novo empate -> a primeira declarada. Sem nenhuma que case, devolve a base.
//!
//! Gates de sanidade sobre os nomes (o catalogo e gerado e contem regras genericas que casariam
//! com qualquer pais):
//! * variante que exige a lei de religiao de estado so vale com o grupo religioso no governo;
//! * regra so de ideologia (sem lei/status/evento) so vale apos mudanca real de regime e com a
//!   ideologia do partido no poder;
//! * enquanto `mudanca_regime` e falso, o nome mostrado e o nome historico do cenário, sem a
//!   legenda de ocupação (que vai na hachura e na legenda do mapa).

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
    /// O grupo religioso (clero) participa do governo: condição para variantes teocráticas.
    pub grupo_religioso_no_poder: bool,
    /// Houve mudança real de regime desde o início do cenário (revolução, golpe, nova lei de
    /// forma de governo ou novo partido no poder). As regras genéricas de ideologia só valem aqui.
    pub mudanca_regime: bool,
    /// Nome histórico do cenário (`data/scenario_1945/countries_1945.ron`). Enquanto
    /// `mudanca_regime` é falso, é este nome que aparece: nada foi mudado no país.
    pub nome_cenario: String,
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

/// Flags de eventos e regimes históricos por data (tag, flag, desde, até).
pub const FLAGS_DATADAS: &[(&str, &str, (i32, u8, u8), (i32, u8, u8))] = &[
    // Canadá: a Folha de Bordo (15 fev 1965)
    ("CAN", "bandeira_folha_de_bordo", (1965, 2, 15), (9999, 1, 1)),
    // Líbia: Jamahiriya (2 set 1977)
    ("LBY", "jamahiriya", (1977, 9, 2), (9999, 1, 1)),
    // RAU: união Egito e Síria (1 fev 1958 a 28 set 1961)
    ("EGY", "uniao_arabe", (1958, 2, 1), (1961, 9, 28)),
    ("SYR", "uniao_arabe", (1958, 2, 1), (1961, 9, 28)),
    // Japão: Constituição do Japão (3 mai 1947 em diante, abandono formal de "Império")
    ("JAP", "jap_constituicao_1947", (1947, 5, 3), (9999, 1, 1)),
    // Índia: Independência e República (15 ago 1947 em diante)
    ("IND", "ind_republica", (1947, 8, 15), (9999, 1, 1)),
    // União Soviética / Rússia: Dissolução da URSS (26 dez 1991 em diante)
    ("SOV", "rus_federacao", (1991, 12, 26), (9999, 1, 1)),
    // França: Quarta República (27 out 1946 a 3 out 1958) e Quinta República (4 out 1958 em diante)
    ("FRA", "quarta_republica", (1946, 10, 27), (1958, 10, 3)),
    ("FRA", "quinta_republica", (1958, 10, 4), (9999, 1, 1)),
    // Itália: República Italiana após o referendo de 2 jun 1946
    ("ITA", "ita_republica", (1946, 6, 2), (9999, 1, 1)),
    // Alemanha: RFA (23 mai 1949 a 2 out 1990) e reunificação (3 out 1990 em diante)
    ("GER", "rfa", (1949, 5, 23), (1990, 10, 2)),
    ("GER", "reunificacao_alema", (1990, 10, 3), (9999, 1, 1)),
    // Egito: República de 18 jun 1953 (exceto no período RAU)
    ("EGY", "republica_egito", (1953, 6, 18), (1958, 1, 31)),
    ("EGY", "republica_egito", (1961, 9, 29), (9999, 1, 1)),
    // Iraque: Revolução de 14 jul 1958
    ("IRQ", "republica_iraque", (1958, 7, 14), (9999, 1, 1)),
    // Brasil: República Federativa do Brasil pela Constituição de 15 mar 1967
    ("BRA", "bra_federativa", (1967, 3, 15), (9999, 1, 1)),
    // Portugal: Revolução dos Cravos (25 abr 1974)
    ("POR", "por_democratica", (1974, 4, 25), (9999, 1, 1)),
    // Espanha: Restauração/Constituição democrática (20 nov 1975 em diante)
    ("ESP", "esp_transicao", (1975, 11, 20), (9999, 1, 1)),
    // Grécia: Terceira República Helênica (8 dez 1974 em diante)
    ("GRE", "gre_republica", (1974, 12, 8), (9999, 1, 1)),
];

pub fn flags_do_pais(tag: &str, d: (i32, u8, u8)) -> Vec<String> {
    FLAGS_DATADAS
        .iter()
        .filter(|(t, _, de, ate)| *t == tag && d >= *de && d <= *ate)
        .map(|(_, f, _, _)| f.to_string())
        .collect()
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

    /// Regra pura de ideologia (sem lei, sem status, sem evento): é genérica, vale para qualquer
    /// país e por isso só pode renomear depois de uma mudança real de regime.
    fn so_ideologia(&self) -> bool {
        !self.categoria_ideologia.is_empty()
            && self.forma_governo.is_empty()
            && self.status.is_empty()
            && self.religiao_estatal.is_empty()
            && self.flags.is_empty()
            && self.partido.is_empty()
            && !self.exige_ocupado
            && !self.exige_nao_ocupado
            && !self.exige_guerra_civil
    }

    /// Exige a lei de religião de estado: nome teocrático/religioso.
    fn exige_religiao_de_estado(&self) -> bool {
        !self.religiao_estatal.is_empty()
    }
}

/// Nome do cenário sem a legenda de ocupação: a ocupação se mostra pela hachura e pela legenda do
/// mapa, nunca no nome (`"Alemanha Ocupada (Conselho de Controle)"` → `"Alemanha"`). Nomes que não
/// falam de ocupação passam como estão, inclusive os que trazem um qualifier entre parênteses.
pub fn sem_ocupacao(nome: &str) -> String {
    if !nome.contains("Ocupad") {
        return nome.to_string();
    }
    let ate_par = nome.split_once(" (").map_or(nome, |(a, _)| a);
    ate_par.replace("Ocupada", "").replace("Ocupado", "").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Nome do cenário para mostrar: com a legenda de ocupação ("Alemanha Ocupada", "Japão Ocupado")
/// enquanto o país tem ocupante, e sem ela depois da desocupação.
fn nome_de_cenario(e: &EstadoPais) -> String {
    if e.ocupante.is_some() { e.nome_cenario.clone() } else { sem_ocupacao(&e.nome_cenario) }
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

    fn casa(&self, v: &Variante, e: &EstadoPais) -> bool {
        let c = &v.condicao;
        // Nome religioso ("Estado Teocrático", "República Islâmica", "Emirado"): além da lei de
        // religião de estado, o grupo religioso tem de dominar o governo.
        if c.exige_religiao_de_estado() && !e.grupo_religioso_no_poder {
            return false;
        }
        // Regra genérica de ideologia ("República Social", "República Revolucionária", "República
        // Popular"): vale só com mudança real de regime e com a ideologia do partido no poder, nunca
        // pela crença pessoal do chefe de Estado ou pelo rótulo estático do cenário. As variantes
        // históricas (declaradas por país) já são escolha do autor do catálogo.
        if !v.historica && c.so_ideologia() {
            return e.mudanca_regime
                && e.ideologia_partido.as_deref().is_some_and(|ideo| self.categoria(ideo).is_some_and(|k| c.categoria_ideologia.iter().any(|x| x == k)));
        }
        let cat = self.categoria(e.ideologia_efetiva());
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

    fn categoria(&self, ideologia: &str) -> Option<&str> {
        self.dados.categorias_ideologia.get(ideologia).map(String::as_str)
    }

    pub fn resolver_bandeira(&self, tag: &str, e: &EstadoPais) -> VarianteBandeira {
        let e_efetivo;
        let e_ref = if e.ano > 0 {
            let mut clone = e.clone();
            let d = (e.ano, e.mes.max(1), 1);
            for f in flags_do_pais(tag, d) {
                if !clone.flags.contains(&f) {
                    clone.flags.push(f);
                }
            }
            e_efetivo = clone;
            &e_efetivo
        } else {
            e
        };
        let Some(p) = self.pais(tag) else {
            return VarianteBandeira {
                id: "desconhecida".into(),
                arquivo: String::new(),
                nome: if e_ref.nome_cenario.is_empty() { tag.into() } else { nome_de_cenario(e_ref) },
                cor: (128, 128, 128),
                sobreposicao: Sobreposicao::Nenhuma,
                tag_cantao: None,
                historica: false,
                e_base: true,
            };
        };
        let mut melhor: Option<(&Variante, (usize, i32))> = None;
        for v in &p.variantes {
            if !self.casa(v, e_ref) {
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
        // Enquanto o país está com o regime com que nasceu no cenário, o nome mostrado é o do
        // cenário; a legenda de ocupação só sai quando não há mais ocupante. Exceção: variantes
        // ligadas a um marco datado (flag de evento), que valem na data.
        let nome = if e_ref.mudanca_regime || e_ref.nome_cenario.is_empty() || !v.condicao.flags.is_empty() {
            v.nome.clone()
        } else {
            nome_de_cenario(e_ref)
        };
        let tag_cantao = match v.sobreposicao {
            Sobreposicao::CantaoSuserano => e_ref.suserano.clone(),
            Sobreposicao::CantaoOcupante => e_ref.ocupante.clone(),
            Sobreposicao::Nenhuma => None,
        };
        VarianteBandeira {
            id: v.id.clone(),
            arquivo: v.arquivo.clone(),
            nome,
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
