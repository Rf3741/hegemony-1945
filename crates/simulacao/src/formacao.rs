//! Descolonização e países formáveis: faz surgirem países NOVOS no mapa na data histórica
//! (Índia/Paquistão 1947, Israel 1948, RFA/RDA 1949, RPC 1949, independências africanas ~1960...).
//!
//! Lógica pura (sem Bevy). Os dados vêm de `data/formaveis/formaveis.ron` e
//! `data/formaveis/independencias_existentes.ron`. A condição usada é a data histórica; as províncias
//! são escolhidas pelo prefixo do código adm1 (`PAK-1234`) entre as que pertencem aos países de origem.

use crate::economia::{Mercado, OrcamentoNacional};
use crate::erro::{ErroSimulacao, ResultadoSimulacao};
use crate::mapa::{AlinhamentoGeopolitico, CatalogoProvincias};
use crate::mundo::{EstadoPais, MundoJogo, gerar_pops_iniciais, gerar_predios_iniciais};
use crate::politica::construir_politica_inicial;
use crate::tempo::DataSimulacao;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct CondicaoFormacao {
    #[serde(default)]
    pub descricao: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlocoFormacao {
    pub data_historica: String,
    pub tipo: String,
    #[serde(default)]
    pub origem_tags: Vec<String>,
    pub condicao: CondicaoFormacao,
    #[serde(default)]
    pub prefixos_adm1: Vec<String>,
}

/// País formável (formato de `nacoes_formaveis/formaveis.ron`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Pais")]
pub struct Formavel {
    pub tag: String,
    pub nome: String,
    pub cor_mapa: (u8, u8, u8),
    pub capital: String,
    #[serde(default)]
    pub tipo_governo: String,
    #[serde(default)]
    pub ideologia_dominante: String,
    #[serde(default)]
    pub suserano: Option<String>,
    pub alinhamento_inicial: String,
    #[serde(default)]
    pub populacao_total: u64,
    #[serde(default)]
    pub leis_iniciais: BTreeMap<String, String>,
    pub formacao: BlocoFormacao,
}

/// Tag que já existia e passa a soberana (Gana, Nigéria...).
#[derive(Debug, Clone, Deserialize)]
pub struct Independencia {
    pub tag: String,
    pub data_historica: String,
    #[serde(default)]
    pub tipo: String,
    #[serde(default)]
    pub remove_suserano: bool,
}

/// Resultado de uma formação, para notícia/evento/integração nos outros módulos.
#[derive(Debug, Clone, PartialEq)]
pub struct EventoFormacao {
    pub tag: String,
    pub nome: String,
    pub data: DataSimulacao,
    pub tipo: String,
    pub origens: Vec<String>,
    pub provincias: Vec<u32>,
    pub populacao: u64,
    pub descricao: String,
    /// `true` se o país não existia antes (precisa ser criado em economia/política/diplomacia).
    pub pais_novo: bool,
    /// `true` se foi só uma independência de tag existente.
    pub independencia: bool,
}

/// Formáveis efêmeros pulados; os sucessores herdam a origem deles (WIF -> BWI, MFD -> AOF).
const IGNORADOS: [&str; 2] = ["WIF", "MFD"];

pub fn parse_data(s: &str) -> Option<DataSimulacao> {
    let mut it = s.split('-');
    let a = it.next()?.parse().ok()?;
    let m = it.next()?.parse().ok()?;
    let d = it.next()?.parse().ok()?;
    Some(DataSimulacao::nova(a, m, d))
}

/// Países que dividem o mesmo prefixo adm1 com outro: separa pelo centroide `(lon, lat)`.
fn pertence_ao_lado(tag: &str, c: (f32, f32)) -> bool {
    match tag {
        "VDR" => c.1 >= 17.0,
        "RVN" => c.1 < 17.0,
        // RDA: leste do Elba/Turíngia e Berlim; RFA: o resto.
        "GDR" => c.0 >= 10.9 && c.1 >= 50.2,
        "FRG" => !(c.0 >= 10.9 && c.1 >= 50.2),
        _ => true,
    }
}

#[derive(Debug, Clone)]
pub struct Formacoes {
    formaveis: Vec<Formavel>,
    independencias: Vec<Independencia>,
    feitos: BTreeSet<String>,
    /// id da província -> (prefixo adm1, centroide)
    indice: HashMap<u32, (String, (f32, f32))>,
}

impl Formacoes {
    pub fn carregar(
        dir: impl AsRef<Path>,
        catalogo: &CatalogoProvincias,
    ) -> ResultadoSimulacao<Self> {
        let ler = |nome: &str| -> ResultadoSimulacao<String> {
            let p = dir.as_ref().join(nome);
            std::fs::read_to_string(&p).map_err(|fonte| ErroSimulacao::Io {
                caminho: p.display().to_string(),
                fonte,
            })
        };
        let erro_ron = |ctx: &str| {
            let ctx = ctx.to_string();
            move |e: ron::error::SpannedError| ErroSimulacao::RonDe {
                contexto: ctx.clone(),
                fonte: Box::new(e),
            }
        };
        let formaveis: Vec<Formavel> =
            ron::from_str(&ler("formaveis.ron")?).map_err(erro_ron("formaveis.ron"))?;
        let independencias: Vec<Independencia> =
            ron::from_str(&ler("independencias_existentes.ron")?)
                .map_err(erro_ron("independencias_existentes.ron"))?;
        Ok(Self::novo(formaveis, independencias, catalogo))
    }

    pub fn novo(
        mut formaveis: Vec<Formavel>,
        independencias: Vec<Independencia>,
        catalogo: &CatalogoProvincias,
    ) -> Self {
        // Sucessores de formáveis ignorados herdam a origem deles.
        let origem_ignorados: BTreeMap<String, Vec<String>> = formaveis
            .iter()
            .filter(|f| IGNORADOS.contains(&f.tag.as_str()))
            .map(|f| (f.tag.clone(), f.formacao.origem_tags.clone()))
            .collect();
        formaveis.retain(|f| !IGNORADOS.contains(&f.tag.as_str()));
        for f in &mut formaveis {
            let mut novas = Vec::new();
            for o in &f.formacao.origem_tags {
                match origem_ignorados.get(o) {
                    Some(h) => novas.extend(h.iter().cloned()),
                    None => novas.push(o.clone()),
                }
            }
            f.formacao.origem_tags = novas;
        }
        formaveis.sort_by_key(|f| parse_data(&f.formacao.data_historica));
        let indice = catalogo
            .provincias
            .iter()
            .map(|p| {
                let pref = p.adm1_code.split('-').next().unwrap_or("").to_string();
                (p.id, (pref, p.centroide))
            })
            .collect();
        Self { formaveis, independencias, feitos: BTreeSet::new(), indice }
    }

    pub fn formaveis(&self) -> &[Formavel] {
        &self.formaveis
    }

    pub fn ja_formado(&self, tag: &str) -> bool {
        self.feitos.contains(tag)
    }

    /// Aplica tudo o que já passou da data histórica. Devolve os eventos em ordem cronológica.
    pub fn processar(&mut self, mundo: &mut MundoJogo) -> Vec<EventoFormacao> {
        let hoje = mundo.data;
        let mut eventos = Vec::new();
        let mut itens: Vec<(DataSimulacao, bool, usize)> = Vec::new();
        for (i, f) in self.formaveis.iter().enumerate() {
            if let Some(d) = parse_data(&f.formacao.data_historica)
                && d <= hoje
                && !self.feitos.contains(&f.tag)
            {
                itens.push((d, false, i));
            }
        }
        for (i, ind) in self.independencias.iter().enumerate() {
            if let Some(d) = parse_data(&ind.data_historica)
                && d <= hoje
                && !self.feitos.contains(&ind.tag)
            {
                itens.push((d, true, i));
            }
        }
        itens.sort();
        for (d, indep, i) in itens {
            if indep {
                let ind = self.independencias[i].clone();
                self.feitos.insert(ind.tag.clone());
                if let Some(e) = aplicar_independencia(mundo, &ind, d) {
                    eventos.push(e);
                }
            } else {
                let f = self.formaveis[i].clone();
                self.feitos.insert(f.tag.clone());
                if let Some(e) = self.aplicar_formacao(mundo, &f, d) {
                    eventos.push(e);
                }
            }
        }
        eventos
    }

    fn aplicar_formacao(
        &self,
        mundo: &mut MundoJogo,
        f: &Formavel,
        data: DataSimulacao,
    ) -> Option<EventoFormacao> {
        let b = &f.formacao;
        let mut provs: Vec<u32> = mundo
            .dono_por_provincia
            .iter()
            .filter(|(id, dono)| {
                **dono != f.tag
                    && b.origem_tags.contains(dono)
                    && self.indice.get(id).is_some_and(|(pref, c)| {
                        b.prefixos_adm1.contains(pref) && pertence_ao_lado(&f.tag, *c)
                    })
            })
            .map(|(id, _)| *id)
            .collect();
        provs.sort_unstable();
        let existe = mundo.paises.contains_key(&f.tag);
        if provs.is_empty() && !existe {
            return None;
        }

        // População e origem das províncias transferidas.
        let mut pop_por_origem: BTreeMap<String, u64> = BTreeMap::new();
        let mut pop_trans = 0u64;
        for id in &provs {
            let p = mundo.populacao_por_provincia.get(id).copied().unwrap_or(0);
            pop_trans += p;
            *pop_por_origem.entry(mundo.dono_por_provincia[id].clone()).or_default() += p;
        }
        let origem_principal = pop_por_origem
            .iter()
            .max_by_key(|(_, p)| **p)
            .map(|(t, _)| t.clone())
            .or_else(|| b.origem_tags.first().cloned());

        // Reduz as pops/PIB dos países de origem na proporção transferida e tira as províncias deles.
        let mut pib_ref = 0.0_f64;
        let mut pib_trans = 0.0_f64;
        for (o, pop_o) in &pop_por_origem {
            let total_o: u64 = mundo
                .dono_por_provincia
                .iter()
                .filter(|(_, d)| *d == o)
                .map(|(id, _)| mundo.populacao_por_provincia.get(id).copied().unwrap_or(0))
                .sum();
            let frac = if total_o == 0 { 0.0 } else { (*pop_o as f64 / total_o as f64).min(1.0) };
            if let Some(po) = mundo.paises.get_mut(o) {
                pib_trans += po.pib_anual_estimado * frac;
                pib_ref = pib_ref.max(po.pib_anual_estimado);
                for pop in &mut po.pops {
                    pop.ativos = (pop.ativos as f64 * (1.0 - frac)).round() as u64;
                    pop.dependentes = (pop.dependentes as f64 * (1.0 - frac)).round() as u64;
                }
                po.pib_anual_estimado *= 1.0 - frac;
                po.provincias_ids.retain(|id| !provs.contains(id));
            }
        }
        for id in &provs {
            mundo.dono_por_provincia.insert(*id, f.tag.clone());
            mundo.controlador_por_provincia.insert(*id, f.tag.clone());
        }

        let populacao = if pop_trans > 0 { pop_trans } else { f.populacao_total };
        let mercado_lider = f.suserano.clone().unwrap_or_else(|| f.tag.clone());
        let mercado_id = format!("MKT_{mercado_lider}");
        mundo.mercados.entry(mercado_id.clone()).or_insert_with(|| {
            Mercado::novo(
                mercado_id.clone(),
                format!("Mercado Comum ({mercado_lider})"),
                mercado_lider.clone(),
            )
        });

        if existe {
            let p = mundo.paises.get_mut(&f.tag).expect("existe");
            p.provincias_ids.extend(provs.iter().copied());
            p.provincias_ids.sort_unstable();
            p.suserano = f.suserano.clone();
        } else {
            let pib = if pib_trans > 0.0 { pib_trans } else { 10_000_000.0 };
            let pib_pc = if populacao > 0 { pib / populacao as f64 } else { 500.0 };
            let sol = (6.0 + pib_pc / 450.0).clamp(6.5, 26.0) as f32;
            let alfab = (0.12 + pib_pc / 6500.0).clamp(0.10, 0.96) as f32;
            let urb = (pib_pc / 8500.0).clamp(0.08, 0.72);
            let capital_prov = provs.first().copied().unwrap_or(0);
            let pops = gerar_pops_iniciais(capital_prov, populacao, urb, sol, alfab, &f.tag);
            let predios = gerar_predios_iniciais(populacao, (pib / 1_000_000.0) as u64, 0.0);
            let alinh = match f.alinhamento_inicial.as_str() {
                "bloco_ocidental" => AlinhamentoGeopolitico::BlocoOcidental,
                "bloco_sovietico" => AlinhamentoGeopolitico::BlocoSovietico,
                "ocupacao_aliada" => AlinhamentoGeopolitico::OcupacaoAliada,
                _ => AlinhamentoGeopolitico::NaoAlinhado,
            };
            let score = match alinh {
                AlinhamentoGeopolitico::BlocoOcidental => -55.0,
                AlinhamentoGeopolitico::BlocoSovietico => 65.0,
                AlinhamentoGeopolitico::NaoAlinhado => 0.0,
                AlinhamentoGeopolitico::OcupacaoAliada => -10.0,
            };
            let politica = construir_politica_inicial(&f.tag, &f.leis_iniciais, score);
            let orcamento = OrcamentoNacional {
                tesouro: pib * 0.04,
                limite_credito: pib * 0.50,
                fundo_investimento: pib * 0.015,
                divida_soberana: pib * 0.03,
                ..OrcamentoNacional::default()
            };
            mundo.paises.insert(
                f.tag.clone(),
                EstadoPais {
                    tag: f.tag.clone(),
                    nome: f.nome.clone(),
                    cor: f.cor_mapa,
                    capital: f.capital.clone(),
                    suserano: f.suserano.clone(),
                    alinhamento_inicial: alinh,
                    ideologia: f.ideologia_dominante.clone(),
                    mercado_id,
                    provincias_ids: provs.clone(),
                    pops,
                    predios,
                    orcamento,
                    politica,
                    pib_anual_estimado: pib,
                },
            );
        }
        let _ = (origem_principal, pib_ref);
        Some(EventoFormacao {
            tag: f.tag.clone(),
            nome: f.nome.clone(),
            data,
            tipo: b.tipo.clone(),
            origens: b.origem_tags.clone(),
            provincias: provs,
            populacao,
            descricao: b.condicao.descricao.clone(),
            pais_novo: !existe,
            independencia: false,
        })
    }
}

fn aplicar_independencia(
    mundo: &mut MundoJogo,
    ind: &Independencia,
    data: DataSimulacao,
) -> Option<EventoFormacao> {
    let pais = mundo.paises.get_mut(&ind.tag)?;
    let tinha = pais.suserano.is_some();
    if ind.remove_suserano {
        pais.suserano = None;
    }
    let mercado_id = format!("MKT_{}", ind.tag);
    pais.mercado_id = mercado_id.clone();
    let (nome, tag, provs) = (pais.nome.clone(), pais.tag.clone(), pais.provincias_ids.clone());
    let populacao = pais.populacao_total();
    mundo
        .mercados
        .entry(mercado_id.clone())
        .or_insert_with(|| Mercado::novo(mercado_id, format!("Mercado ({tag})"), tag.clone()));
    if !tinha {
        return None;
    }
    Some(EventoFormacao {
        tag: tag.clone(),
        descricao: format!("{nome} conquista a independência ({}).", ind.tipo),
        nome,
        data,
        tipo: ind.tipo.clone(),
        origens: Vec::new(),
        provincias: provs,
        populacao,
        pais_novo: false,
        independencia: true,
    })
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::path::PathBuf;

    fn raiz() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn carregar() -> (CatalogoProvincias, MundoJogo, Formacoes) {
        let r = raiz();
        let (cat, mundo) = MundoJogo::carregar_de_arquivos(
            r.join("assets/map/provinces.ron"),
            r.join("data/scenario_1945/countries_1945.ron"),
            r.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenario");
        let f = Formacoes::carregar(r.join("data/formaveis"), &cat).expect("formaveis");
        (cat, mundo, f)
    }

    #[test]
    fn india_paquistao_1947_transfere_provincias() {
        let (_c, mut mundo, mut f) = carregar();
        assert!(f.formaveis().len() >= 85);
        assert!(!mundo.paises.contains_key("PAK"));
        let ind_antes = mundo.paises["IND"].provincias_ids.len();
        let pop_ind = mundo.paises["IND"].populacao_total();
        mundo.data = DataSimulacao::nova(1947, 8, 13);
        assert!(f.processar(&mut mundo).iter().all(|e| e.tag != "PAK"));
        mundo.data = DataSimulacao::nova(1947, 8, 14);
        let ev = f.processar(&mut mundo);
        let pak = ev.iter().find(|e| e.tag == "PAK").expect("PAK surge");
        assert!(pak.pais_novo && pak.provincias.len() >= 20);
        assert!(mundo.paises.contains_key("PAK"));
        assert!(pak.provincias.iter().all(|id| mundo.dono_por_provincia[id] == "PAK"));
        assert_eq!(mundo.paises["IND"].provincias_ids.len(), ind_antes - pak.provincias.len());
        assert!(mundo.paises["PAK"].populacao_total() > 0);
        assert!(mundo.paises["IND"].populacao_total() < pop_ind);
        // idempotente
        assert!(f.processar(&mut mundo).iter().all(|e| e.tag != "PAK"));
    }

    #[test]
    fn independencia_remove_suserano_e_alemanha_se_divide() {
        let (_c, mut mundo, mut f) = carregar();
        assert!(mundo.paises["GHA"].suserano.is_some());
        mundo.data = DataSimulacao::nova(1957, 3, 6);
        let ev = f.processar(&mut mundo);
        assert!(ev.iter().any(|e| e.tag == "GHA" && e.independencia));
        assert!(mundo.paises["GHA"].suserano.is_none());
        assert!(ev.iter().any(|e| e.tag == "FRG"));
        assert!(ev.iter().any(|e| e.tag == "GDR"));
        assert!(mundo.paises["FRG"].provincias_ids.len() > 0);
        assert!(mundo.paises["GDR"].provincias_ids.len() > 0);
    }
}
