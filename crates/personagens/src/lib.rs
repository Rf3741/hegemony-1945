//! Líderes históricos de 1945: carga dos dados, traços, envelhecimento, morte e sucessão.
//!
//! Núcleo puro (sem Bevy): o cliente chama [`Elenco::avancar_dia`] a cada dia de jogo e reage
//! aos [`EventoPersonagem`] devolvidos.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Data civil simplificada (ano, mês, dia); ordenável.
pub type Data = (i32, u8, u8);

/// Converte "AAAA-MM-DD" em [`Data`].
pub fn parse_data(s: &str) -> Option<Data> {
    let mut it = s.split('-');
    let a = it.next()?.parse().ok()?;
    let m = it.next()?.parse().ok()?;
    let d = it.next()?.parse().ok()?;
    Some((a, m, d))
}

/// Registro de um personagem histórico (formato de `personagens.ron`).
#[derive(Debug, Clone, Deserialize)]
pub struct Personagem {
    pub id: String,
    pub nome: String,
    pub pais: String,
    pub nascimento: String,
    #[serde(default)]
    pub morte: Option<String>,
    pub cargo_inicial: String,
    pub ideologia: String,
    pub grupo_interesse: String,
    pub tracos: Vec<String>,
    pub data_entrada: String,
    #[serde(default)]
    pub biografia: String,
}

impl Personagem {
    pub fn data_nascimento(&self) -> Data {
        parse_data(&self.nascimento).unwrap_or((1900, 1, 1))
    }

    /// Idade em anos completos na data dada.
    pub fn idade(&self, hoje: Data) -> i32 {
        let n = self.data_nascimento();
        let mut i = hoje.0 - n.0;
        if (hoje.1, hoje.2) < (n.1, n.2) {
            i -= 1;
        }
        i.max(0)
    }
}

/// Traço de personalidade com efeitos numéricos (formato de `tracos.ron`).
#[derive(Debug, Clone, Deserialize)]
pub struct Traco {
    pub id: String,
    pub nome: String,
    pub categoria: String,
    pub descricao: String,
    #[serde(default)]
    pub efeitos: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct Transicao {
    data: String,
    pais: String,
    papel: String,
    id: String,
}

/// Cargo nacional ocupado por um líder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Papel {
    Estado,
    Governo,
}

impl Papel {
    pub fn nome(self) -> &'static str {
        match self {
            Papel::Estado => "Chefe de Estado",
            Papel::Governo => "Chefe de Governo",
        }
    }
}

/// O que aconteceu num dia.
#[derive(Debug, Clone, PartialEq)]
pub enum EventoPersonagem {
    Morte { id: String, nome: String, pais: String, idade: i32, data: Data },
    /// `antigo` é `None` se o cargo estava vago.
    Sucessao { pais: String, papel: Papel, antigo: Option<String>, novo: String, data: Data },
}

/// Erro de carga.
#[derive(Debug)]
pub struct ErroCarga(pub String);

impl std::fmt::Display for ErroCarga {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ErroCarga {}

/// Estado vivo dos personagens do mundo.
#[derive(Debug, Clone)]
pub struct Elenco {
    pers: BTreeMap<String, Personagem>,
    tracos: BTreeMap<String, Traco>,
    mortos: BTreeMap<String, Data>,
    cargos: BTreeMap<(String, Papel), String>,
    transicoes: Vec<(Data, Transicao)>,
    ultima_data: Data,
    semente: u64,
}

fn mistura(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^ (x >> 33)
}

fn hash_texto(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

const PRENOMES: [&str; 12] = ["Antonio", "Carlos", "Henri", "Johann", "Mikhail", "Ahmed", "Pedro", "Hans", "Giorgio", "Ivan", "Tomas", "Jorge"];
const SOBRENOMES: [&str; 12] = ["Almeida", "Moreau", "Keller", "Petrov", "Haddad", "Rossi", "Santos", "Novak", "Larsen", "Duarte", "Ferreira", "Costa"];

impl Elenco {
    /// Carrega `personagens.ron`, `stubs_lideres.ron`, `tracos.ron` e `transicoes.ron` de `dir`.
    pub fn carregar(dir: &Path, inicio: Data) -> Result<Self, ErroCarga> {
        let ler = |n: &str| std::fs::read_to_string(dir.join(n)).map_err(|e| ErroCarga(format!("{n}: {e}")));
        let mut pers: Vec<Personagem> = ron::from_str(&ler("personagens.ron")?).map_err(|e| ErroCarga(format!("personagens.ron: {e}")))?;
        let stubs: Vec<Personagem> = ron::from_str(&ler("stubs_lideres.ron")?).map_err(|e| ErroCarga(format!("stubs_lideres.ron: {e}")))?;
        pers.extend(stubs);
        let tracos: Vec<Traco> = ron::from_str(&ler("tracos.ron")?).map_err(|e| ErroCarga(format!("tracos.ron: {e}")))?;
        let trans: Vec<Transicao> = match ler("transicoes.ron") {
            Ok(t) => ron::from_str(&t).map_err(|e| ErroCarga(format!("transicoes.ron: {e}")))?,
            Err(_) => Vec::new(),
        };
        Ok(Self::novo(pers, tracos, trans, inicio))
    }

    fn novo(pers: Vec<Personagem>, tracos: Vec<Traco>, trans: Vec<Transicao>, inicio: Data) -> Self {
        let mut e = Self {
            pers: pers.into_iter().map(|p| (p.id.clone(), p)).collect(),
            tracos: tracos.into_iter().map(|t| (t.id.clone(), t)).collect(),
            mortos: BTreeMap::new(),
            cargos: BTreeMap::new(),
            transicoes: trans.into_iter().filter_map(|t| Some((parse_data(&t.data)?, t))).collect(),
            ultima_data: inicio,
            semente: 1945,
        };
        e.transicoes.sort_by_key(|(d, _)| *d);
        e.inicializar(inicio);
        e
    }

    /// Define os ocupantes de cada cargo na data inicial e marca os já falecidos.
    fn inicializar(&mut self, inicio: Data) {
        let ids: Vec<String> = self.pers.keys().cloned().collect();
        for id in &ids {
            if let Some(m) = self.pers[id].morte.as_deref().and_then(parse_data)
                && m <= inicio
            {
                self.mortos.insert(id.clone(), m);
            }
        }
        let paises: BTreeSet<String> = self.pers.values().map(|p| p.pais.clone()).collect();
        for pais in paises {
            let cand = |cargo: &str| -> Option<String> {
                self.pers
                    .values()
                    .filter(|p| p.pais == pais && p.cargo_inicial == cargo && !self.mortos.contains_key(&p.id))
                    .filter(|p| parse_data(&p.data_entrada).is_some_and(|d| d <= inicio))
                    .map(|p| p.id.clone())
                    .next()
            };
            let estado = cand("chefe_de_estado");
            let governo = cand("chefe_de_governo");
            if let Some(i) = estado.clone().or_else(|| governo.clone()) {
                self.cargos.insert((pais.clone(), Papel::Estado), i);
            }
            if let Some(i) = governo.or(estado) {
                self.cargos.insert((pais.clone(), Papel::Governo), i);
            }
        }
        // Países sem chefe: o primeiro líder de GI/oposição assume os dois cargos.
        for pais in self.pers.values().map(|p| p.pais.clone()).collect::<BTreeSet<_>>() {
            if !self.cargos.contains_key(&(pais.clone(), Papel::Estado))
                && let Some(i) = self.melhor_sucessor(&pais, Papel::Estado, inicio, None)
            {
                self.cargos.insert((pais.clone(), Papel::Estado), i.clone());
                self.cargos.entry((pais, Papel::Governo)).or_insert(i);
            }
        }
    }

    pub fn personagem(&self, id: &str) -> Option<&Personagem> {
        self.pers.get(id)
    }

    pub fn traco(&self, id: &str) -> Option<&Traco> {
        self.tracos.get(id)
    }

    pub fn todos(&self) -> impl Iterator<Item = &Personagem> {
        self.pers.values()
    }

    pub fn vivo(&self, id: &str) -> bool {
        !self.mortos.contains_key(id)
    }

    pub fn data_morte(&self, id: &str) -> Option<Data> {
        self.mortos.get(id).copied()
    }

    /// Ocupante atual de um cargo.
    pub fn lider(&self, pais: &str, papel: Papel) -> Option<&Personagem> {
        self.cargos.get(&(pais.to_string(), papel)).and_then(|i| self.pers.get(i))
    }

    pub fn paises(&self) -> BTreeSet<&str> {
        self.pers.values().map(|p| p.pais.as_str()).collect()
    }

    /// Líderes de grupos de interesse e da oposição vivos do país.
    pub fn lideres_gi(&self, pais: &str) -> Vec<&Personagem> {
        self.pers
            .values()
            .filter(|p| p.pais == pais && self.vivo(&p.id) && matches!(p.cargo_inicial.as_str(), "lider_grupo_interesse" | "lider_oposicao"))
            .collect()
    }

    /// Soma dos efeitos dos traços dos chefes de Estado e de Governo (cada pessoa conta uma vez).
    pub fn efeitos_pais(&self, pais: &str) -> BTreeMap<String, f64> {
        let mut soma = BTreeMap::new();
        let e = self.lider(pais, Papel::Estado).map(|p| p.id.as_str());
        let g = self.lider(pais, Papel::Governo).map(|p| p.id.as_str());
        for id in [e, g.filter(|g| Some(*g) != e)].into_iter().flatten() {
            if let Some(p) = self.pers.get(id) {
                for t in &p.tracos {
                    if let Some(t) = self.tracos.get(t) {
                        for (k, v) in &t.efeitos {
                            *soma.entry(k.clone()).or_insert(0.0) += v;
                        }
                    }
                }
            }
        }
        soma
    }

    /// Ajuste de legitimidade (pontos, -8..8) causado pelos traços dos líderes do país.
    pub fn ajuste_legitimidade(&self, pais: &str) -> f64 {
        let ef = self.efeitos_pais(pais);
        let g = |k: &str| ef.get(k).copied().unwrap_or(0.0);
        let s = g("estabilidade_politica")
            + g("legitimidade_publica")
            + g("estabilidade_institucional")
            + g("estabilidade_tradicional")
            + g("legitimidade_revolucionaria")
            + 0.5 * g("autoridade_governamental")
            + 0.5 * g("apelo_popular")
            - 0.5 * g("agitacao_politica")
            - g("radicalismo_popular");
        (s * 20.0).clamp(-8.0, 8.0)
    }

    /// Chance de morte natural por dia (só para quem não tem data de morte histórica).
    fn risco_diario(idade: i32) -> f64 {
        let anual = 0.0004 * ((idade as f64 - 30.0) / 11.0).exp();
        (anual.min(0.9)) / 365.0
    }

    fn morre_hoje(&self, p: &Personagem, hoje: Data) -> bool {
        if let Some(m) = p.morte.as_deref().and_then(parse_data) {
            return m == hoje;
        }
        let h = mistura(self.semente ^ hash_texto(&p.id) ^ ((hoje.0 as u64) << 16 | (hoje.1 as u64) << 8 | hoje.2 as u64));
        ((h >> 11) as f64 / (1u64 << 53) as f64) < Self::risco_diario(p.idade(hoje))
    }

    fn afinidade(cargo: &str, papel: Papel) -> u32 {
        match (papel, cargo) {
            (Papel::Estado, "chefe_de_estado") | (Papel::Governo, "chefe_de_governo") => 0,
            (Papel::Estado, "chefe_de_governo") | (Papel::Governo, "chefe_de_estado") => 1,
            (_, "lider_oposicao") => 2,
            (_, "ministro") => 3,
            (_, "lider_grupo_interesse") => 4,
            (_, "comandante_militar") => 5,
            _ => 6,
        }
    }

    /// Melhor candidato vivo do país, ainda sem cargo igual, excluindo `excluir`.
    fn melhor_sucessor(&self, pais: &str, papel: Papel, hoje: Data, excluir: Option<&str>) -> Option<String> {
        self.pers
            .values()
            .filter(|p| p.pais == pais && self.vivo(&p.id) && Some(p.id.as_str()) != excluir)
            .filter(|p| parse_data(&p.data_entrada).is_some_and(|d| d <= hoje))
            .filter(|p| p.nascimento.as_str() < "1930" && p.idade(hoje) >= 30)
            .min_by_key(|p| (Self::afinidade(&p.cargo_inicial, papel), p.id.clone()))
            .map(|p| p.id.clone())
    }

    fn gerar_sucessor(&mut self, pais: &str, anterior: Option<&str>, hoje: Data) -> String {
        let id = format!("sucessor_{}_{}_{}", pais.to_lowercase(), hoje.0, self.pers.len());
        let h = mistura(hash_texto(&id));
        let nome = format!("{} {}", PRENOMES[(h % 12) as usize], SOBRENOMES[((h >> 8) % 12) as usize]);
        let ant = anterior.and_then(|a| self.pers.get(a)).cloned();
        let ids: Vec<&String> = self.tracos.keys().collect();
        let mut tracos: Vec<String> = Vec::new();
        for k in 0..3u64 {
            if !ids.is_empty() {
                let t = ids[(mistura(h ^ k) % ids.len() as u64) as usize].clone();
                if !tracos.contains(&t) {
                    tracos.push(t);
                }
            }
        }
        let p = Personagem {
            id: id.clone(),
            nome,
            pais: pais.to_string(),
            nascimento: format!("{}-06-15", hoje.0 - 45 - (h % 15) as i32),
            morte: None,
            cargo_inicial: "chefe_de_estado".into(),
            ideologia: ant.as_ref().map_or("conservadorismo".into(), |a| a.ideologia.clone()),
            grupo_interesse: ant.as_ref().map_or("burocratas".into(), |a| a.grupo_interesse.clone()),
            tracos,
            data_entrada: format!("{}-{:02}-{:02}", hoje.0, hoje.1, hoje.2),
            biografia: "Político de carreira que assumiu o poder após a morte do antecessor.".into(),
        };
        self.pers.insert(id.clone(), p);
        id
    }

    fn definir(&mut self, pais: &str, papel: Papel, novo: String, hoje: Data, saida: &mut Vec<EventoPersonagem>) {
        let antigo = self.cargos.insert((pais.to_string(), papel), novo.clone());
        if antigo.as_deref() != Some(novo.as_str()) {
            saida.push(EventoPersonagem::Sucessao { pais: pais.to_string(), papel, antigo, novo, data: hoje });
        }
    }

    /// Processa um dia: transições históricas, mortes e sucessões.
    pub fn avancar_dia(&mut self, hoje: Data) -> Vec<EventoPersonagem> {
        let mut saida = Vec::new();
        self.ultima_data = hoje;
        // Mortes.
        let morrem: Vec<String> = self.pers.values().filter(|p| self.vivo(&p.id)).filter(|p| self.morre_hoje(p, hoje)).map(|p| p.id.clone()).collect();
        for id in &morrem {
            self.mortos.insert(id.clone(), hoje);
            let p = &self.pers[id];
            saida.push(EventoPersonagem::Morte { id: id.clone(), nome: p.nome.clone(), pais: p.pais.clone(), idade: p.idade(hoje), data: hoje });
        }
        // Transições históricas do dia (ignoradas se o personagem já morreu).
        let hist: Vec<Transicao> = self.transicoes.iter().filter(|(d, _)| *d == hoje).map(|(_, t)| t.clone()).collect();
        for t in hist {
            if !self.vivo(&t.id) || !self.pers.contains_key(&t.id) {
                continue;
            }
            for papel in [Papel::Estado, Papel::Governo] {
                let quer = match t.papel.as_str() {
                    "estado" => papel == Papel::Estado,
                    "governo" => papel == Papel::Governo,
                    _ => true,
                };
                if quer {
                    self.definir(&t.pais, papel, t.id.clone(), hoje, &mut saida);
                }
            }
        }
        // Cargos vagos por morte.
        let vagos: Vec<(String, Papel, String)> = self
            .cargos
            .iter()
            .filter(|(_, id)| !self.vivo(id))
            .map(|((pais, papel), id)| (pais.clone(), *papel, id.clone()))
            .collect();
        for (pais, papel, antigo) in vagos {
            // O outro cargo pode já ter um sucessor vivo; reaproveita para o Estado.
            let outro = if papel == Papel::Estado { Papel::Governo } else { Papel::Estado };
            let do_outro = self.cargos.get(&(pais.clone(), outro)).filter(|i| self.vivo(i)).cloned();
            let novo = match (papel, do_outro) {
                (Papel::Estado, Some(g)) => g,
                _ => match self.melhor_sucessor(&pais, papel, hoje, Some(&antigo)) {
                    Some(s) => s,
                    None => self.gerar_sucessor(&pais, Some(&antigo), hoje),
                },
            };
            self.definir(&pais, papel, novo, hoje, &mut saida);
            // O ex-ocupante (já morto) continua registrado como `antigo` no evento.
        }
        saida
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::path::PathBuf;

    fn elenco() -> Elenco {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/personagens");
        Elenco::carregar(&dir, (1945, 1, 1)).expect("carga")
    }

    fn nome(e: &Elenco, p: &str, c: Papel) -> String {
        e.lider(p, c).map(|x| x.id.clone()).unwrap_or_default()
    }

    #[test]
    fn carrega_tudo_e_lideres_iniciais() {
        let e = elenco();
        assert!(e.todos().count() >= 580);
        assert_eq!(nome(&e, "SOV", Papel::Estado), "iosif_stalin");
        assert_eq!(nome(&e, "USA", Papel::Estado), "franklin_d_roosevelt");
        assert_eq!(nome(&e, "GBR", Papel::Governo), "winston_churchill");
        assert!(e.paises().len() >= 50);
        for p in e.paises() {
            assert!(e.lider(p, Papel::Estado).is_some(), "{p} sem chefe de estado");
            assert!(e.lider(p, Papel::Governo).is_some(), "{p} sem chefe de governo");
        }
    }

    #[test]
    fn roosevelt_morre_e_truman_assume() {
        let mut e = elenco();
        let mut ev = Vec::new();
        let mut d = (1945, 1, 1);
        while d <= (1945, 4, 13) {
            ev.extend(e.avancar_dia(d));
            d = proximo(d);
        }
        assert!(ev.iter().any(|x| matches!(x, EventoPersonagem::Morte { id, .. } if id == "franklin_d_roosevelt")));
        assert_eq!(nome(&e, "USA", Papel::Estado), "harry_s_truman");
    }

    fn proximo((a, m, d): Data) -> Data {
        if d < 28 { (a, m, d + 1) } else if m < 12 { (a, m + 1, 1) } else { (a + 1, 1, 1) }
    }

    #[test]
    fn stalin_morre_em_1953_e_ha_sucessor() {
        let mut e = elenco();
        let mut d = (1945, 1, 1);
        while d < (1953, 4, 1) {
            e.avancar_dia(d);
            d = proximo(d);
        }
        assert_eq!(e.data_morte("iosif_stalin"), Some((1953, 3, 5)));
        let novo = e.lider("SOV", Papel::Estado).unwrap();
        assert_ne!(novo.id, "iosif_stalin");
        assert!(e.vivo(&novo.id));
    }

    #[test]
    fn transicao_historica_attlee() {
        let mut e = elenco();
        let mut d = (1945, 1, 1);
        while d < (1945, 8, 1) {
            e.avancar_dia(d);
            d = proximo(d);
        }
        assert_eq!(nome(&e, "GBR", Papel::Governo), "clement_attlee");
    }

    #[test]
    fn simula_15_anos_sempre_com_lideres_vivos() {
        let mut e = elenco();
        let mut d = (1945, 1, 1);
        let mut mortes = 0;
        while d < (1960, 1, 1) {
            for ev in e.avancar_dia(d) {
                if matches!(ev, EventoPersonagem::Morte { .. }) {
                    mortes += 1;
                }
            }
            for p in e.paises() {
                for c in [Papel::Estado, Papel::Governo] {
                    let l = e.lider(p, c).unwrap_or_else(|| panic!("{p} sem {c:?} em {d:?}"));
                    assert!(e.vivo(&l.id), "{p} {c:?} morto {} em {d:?}", l.id);
                }
            }
            d = proximo(d);
        }
        assert!(mortes > 20);
    }

    #[test]
    fn efeitos_agregados() {
        let e = elenco();
        let _ = e.efeitos_pais("USA");
        let a = e.ajuste_legitimidade("USA");
        assert!((-8.0..=8.0).contains(&a));
    }
}
