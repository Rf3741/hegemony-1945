//! # `sim_tecnologia`
//!
//! Motor puro (sem Bevy) da arvore tecnologica 1945-1991: catalogo RON, pesquisa diaria por pais,
//! custo efetivo (antecedencia historica e difusao), desbloqueio de predios/metodos, era visual e IA.
//! Formulas em `TECNOLOGIA.md`.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum ErroTecnologia {
    #[error("falha ao ler {0}: {1}")]
    Leitura(String, String),
    #[error("RON invalido em {0}: {1}")]
    Ron(String, String),
    #[error("{0}")]
    Invalido(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ramo {
    Producao,
    Sociedade,
    Militar,
    EspacialNuclear,
}

impl Ramo {
    pub const TODOS: [Ramo; 4] = [Ramo::Producao, Ramo::Sociedade, Ramo::Militar, Ramo::EspacialNuclear];
    pub fn nome(self) -> &'static str {
        match self {
            Ramo::Producao => "Producao e Industria",
            Ramo::Sociedade => "Sociedade e Governanca",
            Ramo::Militar => "Militar Convencional",
            Ramo::EspacialNuclear => "Espacial e Nuclear",
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Modificador {
    pub id: String,
    pub valor: f64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Desbloqueios {
    pub metodos_producao: Vec<String>,
    pub predios: Vec<String>,
    pub leis: Vec<String>,
    pub unidades_militares: Vec<String>,
    pub modificadores: Vec<Modificador>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tecnologia {
    pub id: String,
    pub nome: String,
    #[serde(default)]
    pub descricao: String,
    pub ramo: Ramo,
    pub era_visual: u32,
    pub ano_historico: i32,
    pub custo: f64,
    #[serde(default)]
    pub pre_requisitos: Vec<String>,
    #[serde(default)]
    pub desbloqueia: Desbloqueios,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PesquisaInicial {
    pub tecnologia_id: String,
    pub progresso_acumulado: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TecnologiasIniciais {
    pub tag: String,
    #[serde(default)]
    pub nome: String,
    pub tecnologias_desbloqueadas: Vec<String>,
    #[serde(default)]
    pub pesquisas_ativas: Vec<PesquisaInicial>,
}

/// Catalogo imutavel de tecnologias.
#[derive(Debug, Clone)]
pub struct CatalogoTec {
    pub techs: Vec<Tecnologia>,
    pub indice: HashMap<String, usize>,
    pub iniciais: BTreeMap<String, TecnologiasIniciais>,
    /// Predio/metodo -> tecnologias que o liberam (so itens citados em alguma tech sao "travados").
    pub libera_predio: HashMap<String, Vec<usize>>,
    pub libera_metodo: HashMap<String, Vec<usize>>,
}

impl CatalogoTec {
    pub fn carregar(dir: &Path) -> Result<Self, ErroTecnologia> {
        let ler = |nome: &str| -> Result<String, ErroTecnologia> {
            let p = dir.join(nome);
            std::fs::read_to_string(&p).map_err(|e| ErroTecnologia::Leitura(p.display().to_string(), e.to_string()))
        };
        let techs: Vec<Tecnologia> =
            ron::from_str(&ler("tecnologias.ron")?).map_err(|e| ErroTecnologia::Ron("tecnologias.ron".into(), e.to_string()))?;
        let ini: Vec<TecnologiasIniciais> = ron::from_str(&ler("tecnologias_iniciais.ron")?)
            .map_err(|e| ErroTecnologia::Ron("tecnologias_iniciais.ron".into(), e.to_string()))?;
        Self::montar(techs, ini)
    }

    pub fn montar(techs: Vec<Tecnologia>, ini: Vec<TecnologiasIniciais>) -> Result<Self, ErroTecnologia> {
        let mut indice = HashMap::new();
        for (i, t) in techs.iter().enumerate() {
            if indice.insert(t.id.clone(), i).is_some() {
                return Err(ErroTecnologia::Invalido(format!("tecnologia duplicada: {}", t.id)));
            }
        }
        for t in &techs {
            for p in &t.pre_requisitos {
                if !indice.contains_key(p) {
                    return Err(ErroTecnologia::Invalido(format!("{}: pre-requisito inexistente {p}", t.id)));
                }
            }
        }
        // Ciclos (Kahn).
        let mut grau: Vec<usize> = techs.iter().map(|t| t.pre_requisitos.len()).collect();
        let mut filhos: Vec<Vec<usize>> = vec![Vec::new(); techs.len()];
        for (i, t) in techs.iter().enumerate() {
            for p in &t.pre_requisitos {
                filhos[indice[p]].push(i);
            }
        }
        let mut pilha: Vec<usize> = (0..techs.len()).filter(|&i| grau[i] == 0).collect();
        let mut vistos = 0;
        while let Some(i) = pilha.pop() {
            vistos += 1;
            for &f in &filhos[i] {
                grau[f] -= 1;
                if grau[f] == 0 {
                    pilha.push(f);
                }
            }
        }
        if vistos != techs.len() {
            return Err(ErroTecnologia::Invalido("ciclo nos pre-requisitos".into()));
        }
        let mut iniciais = BTreeMap::new();
        for i in ini {
            for id in i.tecnologias_desbloqueadas.iter().chain(i.pesquisas_ativas.iter().map(|p| &p.tecnologia_id)) {
                if !indice.contains_key(id) {
                    return Err(ErroTecnologia::Invalido(format!("{}: tecnologia inicial inexistente {id}", i.tag)));
                }
            }
            iniciais.insert(i.tag.clone(), i);
        }
        let mut libera_predio: HashMap<String, Vec<usize>> = HashMap::new();
        let mut libera_metodo: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, t) in techs.iter().enumerate() {
            for p in &t.desbloqueia.predios {
                libera_predio.entry(p.clone()).or_default().push(i);
            }
            for m in &t.desbloqueia.metodos_producao {
                libera_metodo.entry(m.clone()).or_default().push(i);
            }
        }
        Ok(Self { techs, indice, iniciais, libera_predio, libera_metodo })
    }

    pub fn obter(&self, id: &str) -> Option<&Tecnologia> {
        self.indice.get(id).map(|&i| &self.techs[i])
    }
}

// ---------------------------------------------------------------------------------------------
// Parametros e entradas
// ---------------------------------------------------------------------------------------------

pub const PONTOS_BASE: f64 = 10.0;
pub const PONTOS_UNIVERSIDADE: f64 = 8.5;
pub const PONTOS_CENTRO: f64 = 14.0;
pub const TETO_BUFFER: f64 = 1800.0;

/// Insumos diarios de um pais para a geracao de pesquisa.
#[derive(Debug, Clone, Copy)]
pub struct EntradaPais {
    pub niveis_universidade: f64,
    pub niveis_centro: f64,
    /// 0..1
    pub alfabetizacao: f64,
    /// Multiplicador de leis (1.0 = neutro).
    pub mult_leis: f64,
    /// 0..1
    pub estabilidade: f64,
    /// 0..1
    pub acesso_mercado: f64,
}

impl Default for EntradaPais {
    fn default() -> Self {
        Self { niveis_universidade: 0.0, niveis_centro: 0.0, alfabetizacao: 0.5, mult_leis: 1.0, estabilidade: 0.8, acesso_mercado: 0.5 }
    }
}

impl EntradaPais {
    /// Substituto quando nao ha dados de predios: deriva niveis equivalentes do PIB per capita (US$/ano).
    pub fn por_pib_per_capita(pib_pc: f64, alfabetizacao: f64) -> Self {
        let n = (pib_pc / 300.0).clamp(0.0, 15.0);
        Self { niveis_universidade: n, alfabetizacao, ..Default::default() }
    }
}

/// Pontos de inovacao por dia (secao 2.1 de TECNOLOGIA.md; intelectuais neutro).
pub fn pontos_diarios(e: &EntradaPais) -> f64 {
    let bruto = PONTOS_BASE + e.niveis_universidade * PONTOS_UNIVERSIDADE + e.niveis_centro * PONTOS_CENTRO;
    let f_alfa = 0.20 + 0.80 * e.alfabetizacao.clamp(0.0, 1.0);
    bruto * f_alfa * e.mult_leis * e.estabilidade.clamp(0.30, 1.0)
}

/// Penalidade de antecedencia historica (secao 3.2).
pub fn penalidade_antecedencia(ano_atual: i32, ano_historico: i32) -> f64 {
    let d = (ano_historico - ano_atual).max(0) as f64;
    (0.20 * d + 0.03 * d * d).min(5.0)
}

/// Contexto global: quem sao as grandes potencias e os lideres de bloco.
#[derive(Debug, Clone, Default)]
pub struct ConfigBlocos {
    pub grandes_potencias: Vec<String>,
    /// pais -> lider do seu bloco (USA ou SOV).
    pub lider_do: BTreeMap<String, String>,
}

/// Estado de pesquisa de um pais.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EstadoPais {
    pub conhecidas: BTreeSet<String>,
    pub ativa: Option<String>,
    /// Progresso (em fracao 0..1 do custo) de cada tech parcialmente pesquisada.
    pub progresso: BTreeMap<String, f64>,
    pub fila: Vec<String>,
    pub buffer: f64,
    pub pontos_dia: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventoTec {
    Concluida { pais: String, tec: String },
}

/// Estado global de tecnologia de todos os paises.
#[derive(Debug, Clone)]
pub struct MundoTec {
    pub catalogo: Arc<CatalogoTec>,
    pub paises: BTreeMap<String, EstadoPais>,
    pub blocos: ConfigBlocos,
    /// Quantos paises conhecem cada tech (indice no catalogo).
    pub contagem: Vec<u32>,
}

impl MundoTec {
    pub fn novo(catalogo: Arc<CatalogoTec>, blocos: ConfigBlocos) -> Self {
        let mut paises = BTreeMap::new();
        for (tag, ini) in &catalogo.iniciais {
            let mut e = EstadoPais::default();
            e.conhecidas = ini.tecnologias_desbloqueadas.iter().cloned().collect();
            for p in &ini.pesquisas_ativas {
                if let Some(t) = catalogo.obter(&p.tecnologia_id) {
                    e.progresso.insert(t.id.clone(), (p.progresso_acumulado / t.custo).clamp(0.0, 0.99));
                }
            }
            paises.insert(tag.clone(), e);
        }
        let mut m = Self { catalogo, paises, blocos, contagem: Vec::new() };
        m.recontar();
        m
    }

    fn recontar(&mut self) {
        let mut c = vec![0u32; self.catalogo.techs.len()];
        for e in self.paises.values() {
            for id in &e.conhecidas {
                if let Some(&i) = self.catalogo.indice.get(id) {
                    c[i] += 1;
                }
            }
        }
        self.contagem = c;
    }

    pub fn conhece(&self, pais: &str, tec: &str) -> bool {
        self.paises.get(pais).is_some_and(|e| e.conhecidas.contains(tec))
    }

    /// Pre-requisitos satisfeitos e nao conhecida.
    pub fn disponivel(&self, pais: &str, tec: &Tecnologia) -> bool {
        let Some(e) = self.paises.get(pais) else { return false };
        !e.conhecidas.contains(&tec.id) && tec.pre_requisitos.iter().all(|p| e.conhecidas.contains(p))
    }

    /// Pode ser iniciada: disponivel e nao mais de 10 anos adiante do tempo historico.
    pub fn pode_iniciar(&self, pais: &str, tec: &Tecnologia, ano: i32) -> bool {
        self.disponivel(pais, tec) && tec.ano_historico - ano <= 10
    }

    /// Bonus de difusao (secao 4).
    pub fn bonus_difusao(&self, pais: &str, tec: &Tecnologia, acesso_mercado: f64, alfabetizacao: f64) -> f64 {
        let n_lideres = self.blocos.grandes_potencias.iter().filter(|g| g.as_str() != pais && self.conhece(g, &tec.id)).count() as f64;
        let b_global = (0.04 * n_lideres).min(0.40);
        let b_bloco = match self.blocos.lider_do.get(pais) {
            Some(l) if l != pais && self.conhece(l, &tec.id) => 0.35,
            _ => 0.0,
        };
        let b_com = 0.15 * acesso_mercado.clamp(0.0, 1.0);
        ((b_global + b_bloco + b_com) * (0.30 + 0.70 * alfabetizacao.clamp(0.0, 1.0))).min(0.75).max(0.0)
            * if b_global + b_bloco > 0.0 { 1.0 } else { 0.0 }
    }

    /// Custo efetivo em pontos para o pais (secao 3).
    pub fn custo_efetivo(&self, pais: &str, tec: &Tecnologia, ano: i32, e: &EntradaPais) -> f64 {
        let ant = penalidade_antecedencia(ano, tec.ano_historico);
        let dif = self.bonus_difusao(pais, tec, e.acesso_mercado, e.alfabetizacao);
        let escala_eco = if e.niveis_universidade > 10.0 {
            1.0 + (e.niveis_universidade - 10.0) * 0.015
        } else {
            1.0
        };
        tec.custo * escala_eco * (1.0 + ant) * (1.0 - dif)
    }

    /// Define a pesquisa ativa (a anterior volta a fila de progresso parcial).
    pub fn iniciar(&mut self, pais: &str, tec_id: &str, ano: i32) -> Result<(), String> {
        let cat = self.catalogo.clone();
        let t = cat.obter(tec_id).ok_or("tecnologia inexistente")?;
        if !self.pode_iniciar(pais, t, ano) {
            return Err("pre-requisitos nao cumpridos, ja conhecida ou adiantada demais".into());
        }
        let e = self.paises.get_mut(pais).ok_or("pais sem dados de tecnologia")?;
        e.fila.retain(|x| x != tec_id);
        e.ativa = Some(t.id.clone());
        Ok(())
    }

    pub fn enfileirar(&mut self, pais: &str, tec_id: &str) -> Result<(), String> {
        self.catalogo.obter(tec_id).ok_or("tecnologia inexistente")?;
        let e = self.paises.get_mut(pais).ok_or("pais sem dados de tecnologia")?;
        if e.conhecidas.contains(tec_id) || e.ativa.as_deref() == Some(tec_id) || e.fila.iter().any(|x| x == tec_id) {
            return Err("ja conhecida, ativa ou na fila".into());
        }
        e.fila.push(tec_id.to_string());
        Ok(())
    }

    pub fn remover_da_fila(&mut self, pais: &str, tec_id: &str) {
        if let Some(e) = self.paises.get_mut(pais) {
            e.fila.retain(|x| x != tec_id);
        }
    }

    /// Avanca um dia para um pais. Retorna a tech concluida, se houver.
    pub fn tick_pais(&mut self, pais: &str, ano: i32, entrada: &EntradaPais) -> Option<String> {
        let cat = self.catalogo.clone();
        let pontos = pontos_diarios(entrada);
        let ativa = {
            let e = self.paises.get_mut(pais)?;
            e.pontos_dia = pontos;
            e.ativa.clone()
        };
        let Some(ativa) = ativa else {
            let e = self.paises.get_mut(pais)?;
            e.buffer = (e.buffer + pontos).min(TETO_BUFFER);
            return None;
        };
        let t = cat.obter(&ativa)?;
        let custo = self.custo_efetivo(pais, t, ano, entrada).max(1.0);
        let e = self.paises.get_mut(pais)?;
        // Buffer: +100% de velocidade ate esgotar.
        let extra = if e.buffer > 0.0 { pontos.min(e.buffer) } else { 0.0 };
        e.buffer -= extra;
        let prog = e.progresso.entry(ativa.clone()).or_insert(0.0);
        *prog += (pontos + extra) / custo;
        if *prog >= 1.0 {
            e.progresso.remove(&ativa);
            e.conhecidas.insert(ativa.clone());
            e.ativa = None;
            // Proxima da fila que seja iniciavel.
            let fila = std::mem::take(&mut e.fila);
            let mut resto = Vec::new();
            let mut proxima = None;
            for id in fila {
                if proxima.is_none() {
                    if let Some(tt) = cat.obter(&id) {
                        let ok = !e.conhecidas.contains(&id)
                            && tt.pre_requisitos.iter().all(|p| e.conhecidas.contains(p))
                            && tt.ano_historico - ano <= 10;
                        if ok {
                            proxima = Some(id);
                            continue;
                        }
                    }
                }
                resto.push(id);
            }
            e.fila = resto;
            e.ativa = proxima;
            self.recontar_tec(&ativa);
            return Some(ativa);
        }
        None
    }

    fn recontar_tec(&mut self, id: &str) {
        if let Some(&i) = self.catalogo.indice.get(id) {
            self.contagem[i] += 1;
        }
    }

    /// IA: escolhe a proxima pesquisa por prioridade balanceada, marcos historicos e corrida espacial.
    pub fn escolher_ia(&mut self, pais: &str, ano: i32, e: &EntradaPais) {
        if self.paises.get(pais).is_none_or(|x| x.ativa.is_some()) {
            return;
        }
        let cat = self.catalogo.clone();
        let militar = self.blocos.grandes_potencias.iter().any(|g| g == pais);
        let eh_superpotencia = pais == "USA" || pais == "SOV";
        let mut melhor: Option<(f64, &str)> = None;
        for t in &cat.techs {
            if !self.pode_iniciar(pais, t, ano) {
                continue;
            }
            let mut custo = self.custo_efetivo(pais, t, ano, e);
            let prog = self.paises[pais].progresso.get(&t.id).copied().unwrap_or(0.0);
            custo *= 1.0 - prog;

            // Prioridade temporal: tecnologias historicas do ano certo tem prioridade sobre antecipadas
            let diff = t.ano_historico - ano;
            let mult_tempo = if diff > 0 {
                1.0 + (diff as f64) * 0.75
            } else {
                (1.0 - (-diff as f64 * 0.025)).max(0.70)
            };

            let mut peso = match t.ramo {
                Ramo::Producao => 0.85,
                Ramo::Sociedade => 0.95,
                Ramo::Militar => if militar { 0.9 } else { 1.15 },
                Ramo::EspacialNuclear => if militar { 0.9 } else { 1.6 },
            };

            // Corrida espacial e nuclear para superpotencias e grandes potencias
            if (eh_superpotencia || militar) && t.ramo == Ramo::EspacialNuclear {
                let eh_marco_espacial = matches!(
                    t.id.as_str(),
                    "primeiro_satelite_artificial_orbita"
                        | "voo_espacial_humano_orbital_vostok"
                        | "missil_balistico_alcance_intermediario_irbm"
                        | "missil_balistico_intercontinental_icbm"
                        | "pouso_tripulado_na_lua_apollo"
                );
                if eh_marco_espacial && diff <= 1 {
                    peso *= 0.12;
                } else if diff <= 0 {
                    peso *= 0.60;
                }
            }

            // Equilibrio de ramos: se um ramo tem menos tecnologias conhecidas no pais, recebe incentivo
            let conhecidas_no_ramo = self.paises.get(pais).map_or(0, |ep| {
                ep.conhecidas.iter().filter(|id| cat.obter(id).is_some_and(|k| k.ramo == t.ramo)).count()
            });
            let fator_equilibrio = 1.0 + (conhecidas_no_ramo as f64) * 0.04;

            let score = custo * peso * mult_tempo * fator_equilibrio;
            if melhor.is_none_or(|(s, _)| score < s) {
                melhor = Some((score, &t.id));
            }
        }
        if let Some((_, id)) = melhor {
            let id = id.to_string();
            self.paises.get_mut(pais).unwrap().ativa = Some(id);
        }
    }

    // ----- desbloqueios -----

    /// Se o predio esta liberado para o pais (itens que nenhuma tech cita sao livres).
    pub fn predio_liberado(&self, pais: &str, predio: &str) -> bool {
        self.liberado(pais, self.catalogo.libera_predio.get(predio))
    }

    pub fn metodo_liberado(&self, pais: &str, metodo: &str) -> bool {
        self.liberado(pais, self.catalogo.libera_metodo.get(metodo))
    }

    fn liberado(&self, pais: &str, techs: Option<&Vec<usize>>) -> bool {
        match techs {
            None => true,
            Some(v) => self
                .paises
                .get(pais)
                .is_some_and(|e| v.iter().any(|&i| e.conhecidas.contains(&self.catalogo.techs[i].id))),
        }
    }

    /// Predio travado -> nome da primeira tech que o libera.
    pub fn bloqueios_predios(&self, pais: &str) -> BTreeMap<String, String> {
        self.catalogo
            .libera_predio
            .iter()
            .filter(|(p, _)| !self.predio_liberado(pais, p))
            .map(|(p, v)| (p.clone(), self.catalogo.techs[v[0]].nome.clone()))
            .collect()
    }

    pub fn bloqueios_metodos(&self, pais: &str) -> BTreeMap<String, String> {
        self.catalogo
            .libera_metodo
            .iter()
            .filter(|(m, _)| !self.metodo_liberado(pais, m))
            .map(|(m, v)| (m.clone(), self.catalogo.techs[v[0]].nome.clone()))
            .collect()
    }

    /// Era visual do pais: 1945/1955/1965/1975/1985, a maior era com ao menos 3 techs conhecidas.
    pub fn era_visual(&self, pais: &str) -> u32 {
        let Some(e) = self.paises.get(pais) else { return 1945 };
        let mut cont: BTreeMap<u32, u32> = BTreeMap::new();
        for id in &e.conhecidas {
            if let Some(t) = self.catalogo.obter(id) {
                *cont.entry(t.era_visual).or_default() += 1;
            }
        }
        cont.into_iter().filter(|(_, n)| *n >= 3).map(|(e, _)| e).max().unwrap_or(1945)
    }
}

impl MundoTec {
    /// Estado dinâmico (pesquisas por país) para salvar.
    pub fn salvar_estado(&self) -> BTreeMap<String, EstadoPais> {
        self.paises.clone()
    }

    pub fn restaurar_estado(&mut self, paises: BTreeMap<String, EstadoPais>) {
        self.paises = paises;
        self.recontar();
    }
}
