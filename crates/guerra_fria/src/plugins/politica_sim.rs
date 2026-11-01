//! Ponte entre o cliente e o motor político `sim_politica` (estilo Victoria 3).
//!
//! * O cenário de 1945 (`assets/data/politica`) é carregado em segundo plano na tela de carregamento.
//! * O tick diário roda numa TAREFA em outra thread, em lotes de dias, como a economia. A interface
//!   só lê um instantâneo (`VistaPolitica`) do país do jogador; ações viram `ComandoPol`.
//! * Ligação com a economia: padrão de vida (economia) escala o padrão de vida das pops (radicalismo);
//!   leis de tributação aprovadas escalam os impostos (`Comando::EscalarImposto`).
//! * Substitui a política do crate legado `simulacao` (só a legitimidade é espelhada no legado, para o HUD).

use super::economia_sim::{ALIAS_TAGS, Comando, EconomiaSim};
use super::economy::RecursoMundo;
use super::time::EventoTickDiario;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, TaskPool, block_on, futures_lite::future};
use sim_politica::{CatalogoPolitica, LiderPolitico, CenarioPolitica, DataJogo, MundoPolitico, PaisPolitica, TipoCrisePolitica};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

const MAX_ATRASO_DIAS: u32 = 120;
const DIAS_POR_LOTE: u32 = 30;
const SEMENTE: u64 = 1945;

/// Ação do jogador sobre a política (aplicada antes do próximo lote).
#[derive(Debug, Clone)]
pub enum ComandoPol {
    ProporLei { categoria: String, lei: String },
    CancelarLei,
    /// Troca o chefe de Estado (`estado`) ou de Governo de um país e aplica o ajuste de legitimidade.
    /// Aplicado ao país `tag` (não ao do jogador), por isso carrega a própria tag.
    Lider { tag: String, estado: bool, lider: LiderPolitico, delta_legitimidade: f64 },
}

/// Instantâneo do país do jogador para a interface.
#[derive(Default, Clone)]
pub struct VistaPolitica {
    pub tag: String,
    pub data: Option<DataJogo>,
    pub pais: Option<PaisPolitica>,
}

pub struct NucleoPol {
    pub mundo: MundoPolitico,
    /// Padrão de vida econômico de referência (primeira leitura) por tag.
    base_eco: BTreeMap<String, f64>,
    /// Padrão de vida político original das pops, por id de país.
    base_pops: BTreeMap<String, Vec<f64>>,
    /// IA dos países (propõe leis para todos os países que não são do jogador).
    ia: ia_paises::IaPolitica,
}

/// Resultado de um lote: mensagens e efeitos sobre a economia (categoria, lei antiga, lei nova).
#[derive(Default)]
pub struct SaidaPol {
    pub vista: VistaPolitica,
    pub mensagens: Vec<String>,
    pub leis_trocadas: Vec<(String, String, String)>,
    pub ms: f64,
}

impl NucleoPol {
    pub fn carregar(dir: &Path) -> Result<(Self, Arc<CatalogoPolitica>), String> {
        let cen = CenarioPolitica::carregar_diretorio(dir).map_err(|e| e.to_string())?;
        let mundo = MundoPolitico::novo(cen.catalogo, cen.paises, SEMENTE).map_err(|e| e.to_string())?;
        let base_pops = mundo.paises().iter().map(|p| (p.id.clone(), p.pops.iter().map(|x| x.padrao_de_vida).collect())).collect();
        let cat = Arc::new(mundo.catalogo().clone());
        Ok((Self { mundo, base_eco: BTreeMap::new(), base_pops, ia: ia_paises::IaPolitica::novo() }, cat))
    }

    /// Escala o padrão de vida das pops conforme a evolução do padrão de vida econômico (tag -> padrão).
    fn aplicar_padrao_vida(&mut self, eco: &BTreeMap<String, f64>) {
        for (tag, &pv) in eco {
            if pv <= 0.0 || !pv.is_finite() {
                continue;
            }
            let base = *self.base_eco.entry(tag.clone()).or_insert(pv);
            let fator = (pv / base.max(1e-6)).clamp(0.6, 1.5);
            let Some(id) = self.mundo.paises().iter().find(|p| &p.tag == tag).map(|p| p.id.clone()) else { continue };
            let Some(orig) = self.base_pops.get(&id).cloned() else { continue };
            if let Some(p) = self.mundo.pais_mut(&id) {
                for (pop, o) in p.pops.iter_mut().zip(orig) {
                    pop.padrao_de_vida = (o * fator).clamp(0.0, 40.0);
                }
            }
        }
    }

    pub fn aplicar(&mut self, tag: &str, c: ComandoPol) -> Result<String, String> {
        if let ComandoPol::Lider { tag: t, estado, lider, delta_legitimidade } = c {
            let p = self.mundo.paises().iter().find(|p| p.tag == t).map(|p| p.id.clone()).ok_or("país sem dados políticos")?;
            let pais = self.mundo.pais_mut(&p).ok_or("país inexistente")?;
            let nome = lider.nome.clone();
            if estado {
                pais.chefe_de_estado = Some(lider);
            } else {
                pais.chefe_de_governo = Some(lider);
            }
            pais.legitimidade = (pais.legitimidade + delta_legitimidade).clamp(0.0, 100.0);
            return Ok(if t == tag { format!("Novo líder: {nome}") } else { String::new() });
        }
        let id = self.mundo.paises().iter().find(|p| p.tag == tag).map(|p| p.id.clone()).ok_or("país sem dados políticos")?;
        let p = self.mundo.pais_mut(&id).ok_or("país inexistente")?;
        match c {
            ComandoPol::ProporLei { categoria, lei } => {
                p.iniciar_tramitacao(&categoria, &lei).map_err(|e| e.to_string())?;
                Ok(format!("Lei proposta: {lei}"))
            }
            ComandoPol::CancelarLei => {
                p.lei_em_tramitacao = None;
                Ok("Proposta de lei retirada.".into())
            }
            ComandoPol::Lider { .. } => Ok(String::new()), // tratado no início da função
        }
    }

    /// Roda `dias` dias; detecta eventos relevantes no país `tag` do jogador e guerras civis no mundo.
    pub fn avancar(&mut self, tag: &str, dias: u32) -> (Vec<String>, Vec<(String, String, String)>) {
        let mut msgs = Vec::new();
        let mut leis = Vec::new();
        let id = self.mundo.paises().iter().find(|p| p.tag == tag).map(|p| p.id.clone());
        for _ in 0..dias {
            let antes = id.as_deref().and_then(|i| self.mundo.pais(i)).map(|p| (p.leis_ativas.clone(), p.ultima_eleicao.as_ref().map(|e| e.data), crises_graves(p)));
            let data = self.mundo.data_atual();
            self.mundo.executar_tick_diario();
            let ignorar: Vec<String> = id.iter().cloned().collect();
            self.ia.passo_diario(&mut self.mundo, &ignorar);
            for s in self.mundo.drenar_eventos_guerra_civil() {
                let nome = self.mundo.pais(&s.pais_id).map_or(s.pais_tag.clone(), |p| p.nome.clone());
                msgs.push(format!("GUERRA CIVIL em {nome} ({}): {}", data, s.nome_faccao_rebelde));
            }
            let (Some(i), Some((leis_a, ele_a, crises_a))) = (id.as_deref(), antes) else { continue };
            let Some(p) = self.mundo.pais(i) else { continue };
            for (cat, nova) in &p.leis_ativas {
                if leis_a.get(cat) != Some(nova) {
                    let nome = self.mundo.catalogo().leis.get(nova).map_or(nova.clone(), |l| l.nome.clone());
                    msgs.push(format!("Lei aprovada: {nome}"));
                    leis.push((cat.clone(), leis_a.get(cat).cloned().unwrap_or_default(), nova.clone()));
                }
            }
            if let Some(e) = &p.ultima_eleicao
                && Some(e.data) != ele_a
            {
                let venc = e.partido_vencedor_id.as_ref().and_then(|v| p.partidos.get(v)).map_or("-".to_string(), |x| x.nome.clone());
                msgs.push(format!("Eleição realizada em {}: vitória de {venc}", p.nome));
            }
            for (id_c, tipo) in crises_graves(p) {
                if !crises_a.iter().any(|(i, _)| *i == id_c) {
                    msgs.push(match tipo {
                        TipoCrisePolitica::GolpeMilitar => "GOLPE MILITAR em curso no seu país!".to_string(),
                        TipoCrisePolitica::Revolucao => "REVOLUÇÃO: o regime está sendo derrubado!".to_string(),
                        _ => "Revolta popular em curso.".to_string(),
                    });
                }
            }
        }
        (msgs, leis)
    }

    pub fn vista(&self, tag: &str) -> VistaPolitica {
        VistaPolitica { tag: tag.to_string(), data: Some(self.mundo.data_atual()), pais: self.mundo.paises().iter().find(|p| p.tag == tag).cloned() }
    }
}

fn crises_graves(p: &PaisPolitica) -> Vec<(String, TipoCrisePolitica)> {
    p.crises_ativas.iter().filter(|c| !c.resolvida && c.tipo_crise != TipoCrisePolitica::Greve).map(|c| (c.id.clone(), c.tipo_crise)).collect()
}

/// Aplica comandos, sincroniza a economia, avança `dias` e monta a vista.
pub fn executar_lote(nucleo: &mut NucleoPol, tag: &str, cmds: Vec<ComandoPol>, dias: u32, pv_eco: &BTreeMap<String, f64>) -> SaidaPol {
    let mut saida = SaidaPol::default();
    for c in cmds {
        match nucleo.aplicar(tag, c) {
            Ok(m) if m.is_empty() => {}
            Ok(m) => saida.mensagens.push(m),
            Err(e) => saida.mensagens.push(format!("Não foi possível: {e}")),
        }
    }
    nucleo.aplicar_padrao_vida(pv_eco);
    let t = Instant::now();
    let (m, l) = nucleo.avancar(tag, dias);
    saida.ms = if dias > 0 { t.elapsed().as_secs_f64() * 1000.0 / dias as f64 } else { 0.0 };
    saida.mensagens.extend(m);
    saida.leis_trocadas = l;
    saida.vista = nucleo.vista(tag);
    saida
}

/// Tag política (grafia do cenário político/econômico) a partir da tag do legado.
pub fn tag_politica(legado: &str) -> String {
    ALIAS_TAGS.iter().find(|(l, _)| *l == legado).map_or(legado, |(_, e)| *e).to_string()
}

fn tag_legado(pol: &str) -> String {
    ALIAS_TAGS.iter().find(|(_, e)| *e == pol).map_or(pol, |(l, _)| *l).to_string()
}

/// Estado da política no ECS. `nucleo` fica `None` enquanto a thread processa um lote.
#[derive(Resource)]
pub struct PoliticaSim {
    pub nucleo: Option<NucleoPol>,
    tarefa: Option<Task<(NucleoPol, SaidaPol)>>,
    pub catalogo: Arc<CatalogoPolitica>,
    pub vista: VistaPolitica,
    pendentes: u32,
    comandos: Vec<ComandoPol>,
    /// Tag (cenário político) do país do jogador.
    pub tag: Option<String>,
    pub mensagens: Vec<String>,
    pub leis_trocadas: Vec<(String, String, String)>,
    pub ms_por_dia: f64,
    pv_eco: BTreeMap<String, f64>,
}

impl PoliticaSim {
    pub fn enviar(&mut self, c: ComandoPol) {
        self.comandos.push(c);
    }
}

#[derive(Resource)]
struct CargaPolitica(Task<Result<(NucleoPol, Arc<CatalogoPolitica>), String>>);

pub struct PoliticaSimPlugin;

impl Plugin for PoliticaSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(crate::estado::AppState::Carregando), iniciar_carga)
            .add_systems(Update, aguardar_carga.run_if(in_state(crate::estado::AppState::Carregando)))
            .add_systems(
                Update,
                (acumular_dias, definir_pais, sincronizar_economia, recolher, despachar).chain().run_if(resource_exists::<PoliticaSim>),
            );
    }
}

pub fn dir_politica(raiz: &Path) -> PathBuf {
    raiz.join("assets/data/politica")
}

fn iniciar_carga(mut commands: Commands, raiz: Res<super::map::RaizProjeto>) {
    let dir = dir_politica(&raiz.0);
    let t = AsyncComputeTaskPool::get_or_init(TaskPool::default).spawn(async move { NucleoPol::carregar(&dir) });
    commands.insert_resource(CargaPolitica(t));
}

fn aguardar_carga(mut commands: Commands, carga: Option<ResMut<CargaPolitica>>) {
    let Some(mut carga) = carga else { return };
    let Some(res) = block_on(future::poll_once(&mut carga.0)) else { return };
    commands.remove_resource::<CargaPolitica>();
    match res {
        Ok((n, cat)) => commands.insert_resource(PoliticaSim {
            nucleo: Some(n),
            tarefa: None,
            catalogo: cat,
            vista: VistaPolitica::default(),
            pendentes: 0,
            comandos: Vec::new(),
            tag: None,
            mensagens: Vec::new(),
            leis_trocadas: Vec::new(),
            ms_por_dia: 0.0,
            pv_eco: BTreeMap::new(),
        }),
        Err(e) => error!("falha ao carregar a política: {e}"),
    }
}

fn acumular_dias(mut leitor: MessageReader<EventoTickDiario>, mut pol: ResMut<PoliticaSim>) {
    for _ in leitor.read() {
        pol.pendentes += 1;
    }
    pol.pendentes = pol.pendentes.min(MAX_ATRASO_DIAS);
}

fn definir_pais(jogador: Option<Res<super::jogador::PaisJogador>>, mut pol: ResMut<PoliticaSim>) {
    let Some(j) = jogador else { return };
    let t = tag_politica(&j.0);
    if pol.tag.as_deref() != Some(t.as_str()) {
        pol.tag = Some(t);
        pol.vista.tag.clear();
    }
}

/// Liga a economia à política: padrão de vida por país e efeitos de leis tributárias nos impostos.
fn sincronizar_economia(mut pol: ResMut<PoliticaSim>, eco: Option<ResMut<EconomiaSim>>) {
    let Some(mut eco) = eco else { return };
    if !eco.vista.ranking.is_empty() {
        let por_id: BTreeMap<&str, f64> = eco.vista.ranking.iter().map(|r| (r.id.as_str(), r.padrao_vida)).collect();
        let mut m = BTreeMap::new();
        for p in pol.nucleo.as_ref().map(|n| n.mundo.paises()).unwrap_or(&[]) {
            if let Some(id) = eco.id_pais(&p.tag)
                && let Some(&pv) = por_id.get(id.as_str())
            {
                m.insert(p.tag.clone(), pv);
            }
        }
        if !m.is_empty() {
            pol.pv_eco = m;
        }
    }
    for (cat, antiga, nova) in std::mem::take(&mut pol.leis_trocadas) {
        if cat == "tributacao" {
            let f = fator_imposto(&nova) / fator_imposto(&antiga).max(0.1);
            if (f - 1.0).abs() > 1e-6 {
                eco.enviar(Comando::EscalarImposto(f));
            }
        }
    }
}

/// Nível relativo de arrecadação de cada lei de tributação.
pub fn fator_imposto(lei: &str) -> f64 {
    match lei {
        "imposto_regressivo_consumo" => 0.8,
        "imposto_renda_progressivo" => 1.25,
        "apropriacao_excedente_estatal" => 1.4,
        "tributo_colonial_compulsorio" => 0.9,
        _ => 1.0,
    }
}

fn recolher(mut pol: ResMut<PoliticaSim>, mut legado: Option<ResMut<RecursoMundo>>) {
    if let Some(t) = pol.tarefa.as_mut()
        && let Some((n, s)) = block_on(future::poll_once(t))
    {
        pol.tarefa = None;
        pol.nucleo = Some(n);
        pol.vista = s.vista;
        pol.mensagens.extend(s.mensagens);
        pol.leis_trocadas.extend(s.leis_trocadas);
        pol.ms_por_dia = if pol.ms_por_dia == 0.0 { s.ms } else { 0.8 * pol.ms_por_dia + 0.2 * s.ms };
        // Espelha a legitimidade no legado, para o HUD antigo.
        if let (Some(l), Some(p)) = (legado.as_mut(), pol.vista.pais.as_ref())
            && let Some(lp) = l.paises.get_mut(&tag_legado(&p.tag))
        {
            lp.politica.legitimidade = p.legitimidade as f32 as _;
        }
    }
}

fn despachar(mut pol: ResMut<PoliticaSim>) {
    let Some(tag) = pol.tag.clone() else { return };
    let precisa_vista = pol.vista.tag != tag;
    if pol.nucleo.is_none() || (pol.pendentes == 0 && pol.comandos.is_empty() && !precisa_vista) {
        return;
    }
    let dias = pol.pendentes.min(DIAS_POR_LOTE);
    pol.pendentes -= dias;
    let cmds = std::mem::take(&mut pol.comandos);
    let pv = pol.pv_eco.clone();
    let mut n = pol.nucleo.take().expect("núcleo");
    pol.tarefa = Some(AsyncComputeTaskPool::get_or_init(TaskPool::default).spawn(async move {
        let s = executar_lote(&mut n, &tag, cmds, dias, &pv);
        (n, s)
    }));
}

/// Conjunto de tags com eleições (para testes/diagnóstico).
#[allow(dead_code)]
pub fn paises_com_eleicao(m: &MundoPolitico) -> BTreeSet<String> {
    m.paises().iter().filter(|p| p.ultima_eleicao.is_some()).map(|p| p.tag.clone()).collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn dir() -> PathBuf {
        dir_politica(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
    }

    #[test]
    fn cinco_anos_cenario_real_sem_panico_com_eleicoes() {
        let (mut n, _cat) = NucleoPol::carregar(&dir()).expect("cenário político");
        assert!(n.mundo.paises().len() >= 150);
        let t = Instant::now();
        let mut msgs = Vec::new();
        for _ in 0..(5 * 12) {
            let (m, _) = n.avancar("USA", 30);
            msgs.extend(m);
        }
        n.mundo.auditar_estabilidade().expect("invariantes");
        let ms = t.elapsed().as_secs_f64() * 1000.0 / (5.0 * 360.0);
        let com_eleicao = paises_com_eleicao(&n.mundo);
        eprintln!("1800 dias; {ms:.2} ms/dia; {} países com eleição; msgs: {}", com_eleicao.len(), msgs.len());
        for m in msgs.iter().take(10) {
            eprintln!("  {m}");
        }
        assert!(com_eleicao.len() >= 5, "poucas eleições: {}", com_eleicao.len());
        for tag in ["USA", "GBR", "FRA"] {
            let p = n.mundo.paises().iter().find(|p| p.tag == tag).unwrap();
            eprintln!("{tag}: eleição {:?} próxima em {} dias", p.ultima_eleicao.as_ref().map(|e| e.data), p.dias_para_proxima_eleicao);
        }
    }

    #[test]
    fn propor_lei_e_ligacao_economia() {
        let (mut n, _) = NucleoPol::carregar(&dir()).unwrap();
        let cat = n.mundo.pais_mut("usa").map(|p| p.leis_ativas.get("tributacao").cloned());
        eprintln!("usa tributação: {cat:?}");
        let r = n.aplicar("USA", ComandoPol::ProporLei { categoria: "tributacao".into(), lei: "imposto_regressivo_consumo".into() });
        eprintln!("{r:?}");
        let pv: BTreeMap<String, f64> = [("USA".to_string(), 10.0)].into();
        let s = executar_lote(&mut n, "USA", vec![], 5, &pv);
        let pv2: BTreeMap<String, f64> = [("USA".to_string(), 6.0)].into();
        let _ = executar_lote(&mut n, "USA", vec![], 5, &pv2);
        assert!(s.vista.pais.is_some());
        assert!(r.is_ok(), "{r:?}");
    }
}
