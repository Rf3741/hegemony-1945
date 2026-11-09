//! Salvar e carregar a partida (serde + RON + gzip), com slots, salvamento rápido e autosave anual.
use super::diplomacia_ui::{DiploSim, PainelDiplomacia};
use super::divergencia_sim::DivergenciaSim;
use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::eventos_ui::EstadoEv;
use super::guerra_sim::GuerraSim;
use super::jogador::PaisJogador;
use super::map::RaizProjeto;
use super::personagens_sim::ElencoRes;
use super::politica_sim::{PoliticaSalva, PoliticaSim};
use super::tecnologia_ui::TecnologiaSim;
use super::time::{EventoTickDiario, RecursoRelogio};
use crate::estado::AppState;
use bevy::prelude::*;
use bevy_egui::egui;
use eventos::EstadoEventos;
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use gf_mapa::tempo::RelogioSimulacao;
use personagens::Elenco;
use serde::{Deserialize, Serialize};
use sim_diplomacia::DiplomaciaSalva;
use sim_economia::{EstadoEconomiaSalvo, MercadoGlobal};
use sim_guerra::EstadoGuerraSalvo;
use simulacao::mundo::MundoJogo;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAGIA: &[u8; 4] = b"HG45";
pub const VERSAO: u32 = 1;
pub const SLOTS_MANUAIS: u8 = 5;

/// Onde gravar/ler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Manual(u8),
    Auto,
    Rapido,
}

impl Slot {
    pub fn arquivo(self) -> String {
        match self {
            Slot::Manual(n) => format!("slot_{n}.hg45"),
            Slot::Auto => "autosave.hg45".into(),
            Slot::Rapido => "rapido.hg45".into(),
        }
    }
    pub fn rotulo(self) -> String {
        match self {
            Slot::Manual(n) => format!("Slot {n}"),
            Slot::Auto => "Autosave anual".into(),
            Slot::Rapido => "Salvamento rápido (Ctrl+S)".into(),
        }
    }
    pub fn todos() -> Vec<Slot> {
        let mut v = vec![Slot::Rapido, Slot::Auto];
        v.extend((1..=SLOTS_MANUAIS).map(Slot::Manual));
        v
    }
}

/// Espelho do `MundoJogo` com os mapas em ordem estável (deterministico para o checksum).
#[derive(Serialize, Deserialize)]
struct MundoSalvo {
    base: MundoJogo,
    dono: Vec<(u32, String)>,
    controlador: Vec<(u32, String)>,
    populacao: Vec<(u32, u64)>,
}

fn ordenar<V: Clone>(m: &std::collections::HashMap<u32, V>) -> Vec<(u32, V)> {
    let mut v: Vec<_> = m.iter().map(|(k, x)| (*k, x.clone())).collect();
    v.sort_by_key(|(k, _)| *k);
    v
}

impl MundoSalvo {
    fn de(m: &MundoJogo) -> Self {
        let mut base = m.clone();
        base.dono_por_provincia.clear();
        base.controlador_por_provincia.clear();
        base.populacao_por_provincia.clear();
        Self { base, dono: ordenar(&m.dono_por_provincia), controlador: ordenar(&m.controlador_por_provincia), populacao: ordenar(&m.populacao_por_provincia) }
    }
    fn para(self) -> MundoJogo {
        let mut m = self.base;
        m.dono_por_provincia = self.dono.into_iter().collect();
        m.controlador_por_provincia = self.controlador.into_iter().collect();
        m.populacao_por_provincia = self.populacao.into_iter().collect();
        m
    }
}

/// Estado completo de uma partida.
#[derive(Serialize, Deserialize)]
pub struct Partida {
    pub versao: u32,
    pub pais_jogador: String,
    mundo: MundoSalvo,
    pub relogio: RelogioSimulacao,
    pub economia: EstadoEconomiaSalvo,
    pub mercado_global: MercadoGlobal,
    pub politica: PoliticaSalva,
    pub diplomacia: DiplomaciaSalva,
    pub tecnologia: BTreeMap<String, sim_tecnologia::EstadoPais>,
    pub eventos: EstadoEventos,
    #[serde(default)]
    pub divergencia: Option<simulacao::MotorDivergencia>,
    #[serde(default)]
    pub guerra: Option<EstadoGuerraSalvo>,
    #[serde(default)]
    pub personagens: Option<Elenco>,
}

impl Partida {
    #[allow(clippy::too_many_arguments)]
    pub fn montar(
        pais: &str,
        mundo: &MundoJogo,
        relogio: &RelogioSimulacao,
        economia: (EstadoEconomiaSalvo, MercadoGlobal),
        politica: PoliticaSalva,
        diplomacia: DiplomaciaSalva,
        tecnologia: BTreeMap<String, sim_tecnologia::EstadoPais>,
        eventos: EstadoEventos,
        divergencia: Option<simulacao::MotorDivergencia>,
        guerra: Option<EstadoGuerraSalvo>,
        personagens: Option<Elenco>,
    ) -> Self {
        Self {
            versao: VERSAO,
            pais_jogador: pais.to_string(),
            mundo: MundoSalvo::de(mundo),
            relogio: relogio.clone(),
            economia: economia.0,
            mercado_global: economia.1,
            politica,
            diplomacia,
            tecnologia,
            eventos,
            divergencia,
            guerra,
            personagens,
        }
    }
    pub fn mundo(self_: &Partida) -> MundoJogo {
        MundoSalvo { base: self_.mundo.base.clone(), dono: self_.mundo.dono.clone(), controlador: self_.mundo.controlador.clone(), populacao: self_.mundo.populacao.clone() }.para()
    }
    pub fn data_formatada(&self) -> String {
        self.relogio.data.formatar_pt_br()
    }
}

/// Resumo gravado no cabeçalho do arquivo (lido sem descompactar o corpo).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Meta {
    pub pais: String,
    pub data: String,
    pub versao: u32,
}

fn fnv64(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for x in b {
        h = (h ^ *x as u64).wrapping_mul(0x100000001b3);
    }
    h
}

/// Checksum do estado (FNV-1a do RON canônico).
pub fn checksum(p: &Partida) -> Result<u64, String> {
    Ok(fnv64(ron::to_string(p).map_err(|e| e.to_string())?.as_bytes()))
}

/// Formato: "HG45" | checksum u64 | tamanho meta u32 | meta (RON) | corpo (RON em gzip).
pub fn codificar(p: &Partida) -> Result<Vec<u8>, String> {
    let texto = ron::to_string(p).map_err(|e| e.to_string())?;
    let meta = ron::to_string(&Meta { pais: p.pais_jogador.clone(), data: p.data_formatada(), versao: p.versao }).map_err(|e| e.to_string())?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
    gz.write_all(texto.as_bytes()).map_err(|e| e.to_string())?;
    let corpo = gz.finish().map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(corpo.len() + meta.len() + 16);
    out.extend_from_slice(MAGIA);
    out.extend_from_slice(&fnv64(texto.as_bytes()).to_le_bytes());
    out.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    out.extend_from_slice(meta.as_bytes());
    out.extend_from_slice(&corpo);
    Ok(out)
}

fn cabecalho(b: &[u8]) -> Result<(u64, Meta, &[u8]), String> {
    if b.len() < 16 || &b[..4] != MAGIA {
        return Err("arquivo de save inválido".into());
    }
    let soma = u64::from_le_bytes(b[4..12].try_into().unwrap());
    let n = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
    let meta_txt = b.get(16..16 + n).ok_or("cabeçalho truncado")?;
    let meta: Meta = ron::from_str(std::str::from_utf8(meta_txt).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok((soma, meta, &b[16 + n..]))
}

pub fn ler_meta(caminho: &Path) -> Option<Meta> {
    let mut f = std::fs::File::open(caminho).ok()?;
    let mut buf = vec![0u8; 4096];
    let n = f.read(&mut buf).ok()?;
    cabecalho(&buf[..n]).ok().map(|(_, m, _)| m)
}

pub fn decodificar(b: &[u8]) -> Result<Partida, String> {
    let (soma, meta, corpo) = cabecalho(b)?;
    if meta.versao != VERSAO {
        return Err(format!("versão de save incompatível ({})", meta.versao));
    }
    let mut texto = String::new();
    GzDecoder::new(corpo).read_to_string(&mut texto).map_err(|e| e.to_string())?;
    if fnv64(texto.as_bytes()) != soma {
        return Err("checksum do save não confere (arquivo corrompido)".into());
    }
    ron::from_str(&texto).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------------------------
// Integração com o Bevy
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct InfoSlot {
    pub meta: Option<Meta>,
    pub bytes: u64,
    /// Quando o arquivo foi gravado (relógio real).
    pub modificado: Option<std::time::SystemTime>,
}

/// Lista de slots (atualizada ao salvar e ao abrir os menus).
#[derive(Resource, Default)]
pub struct Slots {
    pub lista: Vec<(Slot, InfoSlot)>,
    pub mensagem: String,
    /// Aviso curto ("Jogo salvo", "Carregado"); `aviso_id` muda a cada novo aviso.
    pub aviso: String,
    pub aviso_id: u32,
}

impl Slots {
    pub fn avisar(&mut self, texto: &str) {
        self.aviso = texto.into();
        self.aviso_id = self.aviso_id.wrapping_add(1);
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct PedirSalvar(pub Slot);
#[derive(Message, Debug, Clone, Copy)]
pub struct PedirCarregar(pub Slot);
#[derive(Message, Debug, Clone, Copy)]
pub struct AtualizarSlots;

pub fn dir_saves(raiz: &Path) -> PathBuf {
    raiz.join("saves")
}

pub(crate) fn varrer(raiz: &Path) -> Vec<(Slot, InfoSlot)> {
    Slot::todos()
        .into_iter()
        .map(|s| {
            let c = dir_saves(raiz).join(s.arquivo());
            let md = std::fs::metadata(&c).ok();
            let bytes = md.as_ref().map(|m| m.len()).unwrap_or(0);
            (s, InfoSlot { meta: ler_meta(&c), bytes, modificado: md.and_then(|m| m.modified().ok()) })
        })
        .collect()
}

/// Lista os slots; chama `escolher(slot)` no clique em "Carregar".
pub fn lista_slots(ui: &mut egui::Ui, slots: &Slots, mut carregar: impl FnMut(Slot), mut salvar: Option<&mut dyn FnMut(Slot)>) {
    for (s, info) in &slots.lista {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(s.rotulo()).strong());
            match &info.meta {
                Some(m) => {
                    ui.label(format!("{} - {} ({:.1} MB)", m.pais, m.data, info.bytes as f64 / 1e6));
                }
                None => {
                    ui.weak("vazio");
                }
            }
            if info.meta.is_some() && ui.button("Carregar").clicked() {
                carregar(*s);
            }
            if let Some(f) = salvar.as_mut()
                && !matches!(s, Slot::Auto)
                && ui.button("Salvar").clicked()
            {
                f(*s);
            }
        });
    }
    if !slots.mensagem.is_empty() {
        ui.label(&slots.mensagem);
    }
}

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Slots>()
            .add_message::<PedirSalvar>()
            .add_message::<PedirCarregar>()
            .add_message::<AtualizarSlots>()
            .add_systems(Startup, |mut s: ResMut<Slots>, raiz: Res<RaizProjeto>| s.lista = varrer(&raiz.0))
            .add_systems(OnEnter(AppState::Menu), |mut s: ResMut<Slots>, raiz: Res<RaizProjeto>| s.lista = varrer(&raiz.0))
            .add_systems(Update, (teclas_rapidas, autosave_anual).run_if(in_state(AppState::Jogando)))
            .add_systems(Update, (executar_salvar, executar_carregar).chain());
    }
}

fn teclas_rapidas(teclas: Res<ButtonInput<KeyCode>>, mut s: MessageWriter<PedirSalvar>, mut c: MessageWriter<PedirCarregar>) {
    let ctrl = teclas.pressed(KeyCode::ControlLeft) || teclas.pressed(KeyCode::ControlRight);
    if ctrl && teclas.just_pressed(KeyCode::KeyS) {
        s.write(PedirSalvar(Slot::Rapido));
    }
    if ctrl && teclas.just_pressed(KeyCode::KeyL) {
        c.write(PedirCarregar(Slot::Rapido));
    }
}

/// Grava o autosave a cada N meses (Configurações > Jogo; padrão 12 = todo 1º de janeiro).
fn autosave_anual(
    mut ticks: MessageReader<EventoTickDiario>,
    mut s: MessageWriter<PedirSalvar>,
    cfg: Res<super::config::ConfigJogo>,
    opcoes: Option<Res<super::smoke::OpcoesCli>>,
) {
    if opcoes.as_ref().is_some_and(|o| o.ativo()) {
        ticks.clear();
        return;
    }
    for t in ticks.read() {
        if super::config::dia_de_autosave(t.nova_data.ano, t.nova_data.mes, t.nova_data.dia, cfg.partida.autosave_meses) {
            s.write(PedirSalvar(Slot::Auto));
        }
    }
}

type RecursosSalvar<'w> = (
    Res<'w, RaizProjeto>,
    Res<'w, RecursoMundo>,
    Res<'w, RecursoRelogio>,
    Option<Res<'w, PaisJogador>>,
    Option<ResMut<'w, EconomiaSim>>,
    Option<ResMut<'w, PoliticaSim>>,
    Option<Res<'w, DiploSim>>,
    Option<Res<'w, TecnologiaSim>>,
    Option<Res<'w, EstadoEv>>,
    Option<Res<'w, DivergenciaSim>>,
    Option<Res<'w, GuerraSim>>,
    Option<Res<'w, ElencoRes>>,
);

fn executar_salvar(mut pedidos: MessageReader<PedirSalvar>, mut espera: Local<Vec<Slot>>, mut slots: ResMut<Slots>, rec: RecursosSalvar) {
    espera.extend(pedidos.read().map(|p| p.0));
    if espera.is_empty() {
        return;
    }
    let (raiz, mundo, relogio, jogador, eco, pol, diplo, tec, ev, div, guerra_sim, elenco) = rec;
    let (Some(jogador), Some(mut eco), Some(mut pol), Some(diplo), Some(tec), Some(ev)) = (jogador, eco, pol, diplo, tec, ev) else {
        espera.clear();
        slots.mensagem = "Nada para salvar ainda.".into();
        return;
    };
    // Se a economia/política estão processando um lote, tenta de novo no próximo quadro.
    let Some(economia) = eco.estado_salvo() else { return };
    let Some(politica) = pol.estado_salvo() else { return };
    let divergencia = div.map(|d| d.0.clone());
    let guerra = guerra_sim.and_then(|g| g.0.salvar_estado().into());
    let personagens = elenco.map(|e| e.elenco.clone());
    let partida = Partida::montar(&jogador.0, &mundo.0, &relogio.0, economia, politica, diplo.0.salvar_estado(), tec.mundo.salvar_estado(), ev.estado.clone(), divergencia, guerra, personagens);
    let bytes = match codificar(&partida) {
        Ok(b) => b,
        Err(e) => {
            slots.mensagem = format!("Falha ao salvar: {e}");
            espera.clear();
            return;
        }
    };
    let dir = dir_saves(&raiz.0);
    let _ = std::fs::create_dir_all(&dir);
    for slot in espera.drain(..) {
        let caminho = dir.join(slot.arquivo());
        let tmp = caminho.with_extension("tmp");
        match std::fs::write(&tmp, &bytes).and_then(|_| std::fs::rename(&tmp, &caminho)) {
            Ok(()) => {
                slots.mensagem = format!("Salvo em {} ({}).", slot.rotulo(), partida.data_formatada());
                slots.avisar("Jogo salvo");
            }
            Err(e) => slots.mensagem = format!("Falha ao gravar {}: {e}", caminho.display()),
        }
    }
    slots.lista = varrer(&raiz.0);
}

#[derive(bevy::ecs::system::SystemParam)]
struct RecursosCarregar<'w> {
    raiz: Res<'w, RaizProjeto>,
    mundo: ResMut<'w, RecursoMundo>,
    relogio: ResMut<'w, RecursoRelogio>,
    eco: Option<ResMut<'w, EconomiaSim>>,
    pol: Option<ResMut<'w, PoliticaSim>>,
    diplo: Option<ResMut<'w, DiploSim>>,
    tec: Option<ResMut<'w, TecnologiaSim>>,
    ev: Option<ResMut<'w, EstadoEv>>,
    div: Option<ResMut<'w, DivergenciaSim>>,
    guerra_sim: Option<ResMut<'w, GuerraSim>>,
    elenco: Option<ResMut<'w, ElencoRes>>,
    painel_diplo: Option<ResMut<'w, PainelDiplomacia>>,
    estado: Res<'w, State<AppState>>,
    prox: ResMut<'w, NextState<AppState>>,
}

fn executar_carregar(mut pedidos: MessageReader<PedirCarregar>, mut espera: Local<Option<Slot>>, mut slots: ResMut<Slots>, mut commands: Commands, mut r: RecursosCarregar) {
    if let Some(p) = pedidos.read().last() {
        *espera = Some(p.0);
    }
    let Some(slot) = *espera else { return };
    let (Some(eco), Some(pol), Some(diplo), Some(tec), Some(ev)) = (r.eco.as_mut(), r.pol.as_mut(), r.diplo.as_mut(), r.tec.as_mut(), r.ev.as_mut()) else {
        *espera = None;
        slots.mensagem = "O jogo ainda está carregando.".into();
        return;
    };
    if eco.nucleo.is_none() || pol.nucleo.is_none() {
        return; // aguarda o lote terminar
    }
    *espera = None;
    let caminho = dir_saves(&r.raiz.0).join(slot.arquivo());
    let partida = match std::fs::read(&caminho).map_err(|e| e.to_string()).and_then(|b| decodificar(&b)) {
        Ok(p) => p,
        Err(e) => {
            slots.mensagem = format!("Falha ao carregar: {e}");
            return;
        }
    };
    let Partida { pais_jogador, relogio, economia, mercado_global, politica, diplomacia, tecnologia, eventos, divergencia, guerra, personagens, .. } = {
        let m = Partida::mundo(&partida);
        r.mundo.0 = m;
        partida
    };
    if let Err(e) = eco.restaurar_estado(economia, mercado_global).and_then(|_| pol.restaurar_estado(politica)) {
        slots.mensagem = format!("Falha ao restaurar: {e}");
        return;
    }
    diplo.0.restaurar_estado(diplomacia);
    tec.restaurar_estado(tecnologia);
    ev.estado = eventos;
    r.relogio.0 = relogio;
    // Restaura divergência (MotorDivergencia)
    if let Some(div) = r.div.as_mut()
        && let Some(divergencia) = divergencia
    {
        div.0 = divergencia;
    }
    // Restaura guerra (MundoGuerra)
    if let Some(guerra_sim) = r.guerra_sim.as_mut()
        && let Some(guerra) = guerra
    {
        if let Ok(mut mundo_guerra) = sim_guerra::MundoGuerra::restaurar_estado(guerra_sim.0.cenario.catalogo.clone(), guerra) {
            guerra_sim.0 = mundo_guerra;
        }
    }
    // Restaura personagens (Elenco)
    if let Some(elenco_res) = r.elenco.as_mut()
        && let Some(personagens) = personagens
    {
        elenco_res.elenco = personagens;
    }
    if let Some(p) = r.painel_diplo.as_mut() {
        p.repintar = true;
    }
    commands.insert_resource(PaisJogador(pais_jogador));
    slots.mensagem = format!("Carregado: {}.", slot.rotulo());
    slots.avisar("Carregado");
    if *r.estado.get() == AppState::Menu {
        r.prox.set(AppState::Jogando);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use sim_diplomacia::{Bloco, Diplomacia, InfoPais};

    fn mundo_teste() -> MundoJogo {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenário")
        .1
    }

    #[test]
    fn salvar_carregar_bate_checksum() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut nucleo = super::super::economia_sim::Nucleo::carregar(&super::super::economia_sim::dir_economia(&raiz), 3).expect("economia");
        let usa = nucleo.id_pais("USA").unwrap();
        let (prov_usa, tipo_usa) = nucleo
            .mundo
            .provincias()
            .iter()
            .filter(|p| p.pais_id == usa)
            .find_map(|p| p.predios.iter().find(|b| b.nivel > 0 && !b.fechado).map(|b| (p.id.clone(), b.tipo_predio_id.clone())))
            .expect("prédio aberto nos EUA");
        nucleo.mundo.fechar_predio(&usa, &prov_usa, &tipo_usa).expect("fecha prédio antes de salvar");
        nucleo.mundo.registrar_tecnologia_pesquisada(&usa, "reator_nuclear_experimental");
        nucleo.mundo.registrar_evento_historico("crise_suez");
        nucleo.avancar(5);
        let mut mundo = mundo_teste();
        mundo.tensao_mundial = 42.5;
        mundo.ticks_executados = 77;
        let mut diplo = Diplomacia::novo(vec![InfoPais { tag: "USA".into(), bloco: Bloco::Ocidental }, InfoPais { tag: "SOV".into(), bloco: Bloco::Sovietico }]);
        diplo.tensao = 33.0;
        let mut ev = EstadoEventos::default();
        ev.disparados.insert("teste".into(), Some("a".into()));
        let mut relogio = RelogioSimulacao::novo();
        relogio.total_ticks = 123;
        let eco = (nucleo.mundo.salvar_estado(), nucleo.global.clone());
        let pol = PoliticaSalva { mundo: sim_politica::EstadoPoliticoSalvo { data: Default::default(), dias_decorridos: 9, paises: vec![], rng: Default::default(), eventos_guerra_civil: vec![] }, base_eco: BTreeMap::new(), base_pops: BTreeMap::new() };
        let p1 = Partida::montar("USA", &mundo, &relogio, eco, pol, diplo.salvar_estado(), BTreeMap::new(), ev, None, None, None);
        let c1 = checksum(&p1).unwrap();
        let bytes = codificar(&p1).unwrap();
        let p2 = decodificar(&bytes).unwrap();
        assert_eq!(c1, checksum(&p2).unwrap(), "checksum após salvar/carregar");
        assert_eq!(p2.pais_jogador, "USA");
        assert_eq!(Partida::mundo(&p2).tensao_mundial, 42.5);
        assert_eq!(p2.relogio.total_ticks, 123);
        // Restaura a economia a partir do salvo e confere de novo.
        let mut n2 = super::super::economia_sim::Nucleo::carregar(&super::super::economia_sim::dir_economia(&raiz), 0).expect("economia");
        n2.mundo = sim_economia::MundoEconomico::restaurar_estado(n2.catalogo.as_ref().clone(), p2.economia.clone()).expect("restaura");
        let reg = n2.mundo.salvar_estado();
        fn assert_ron<T: serde::Serialize>(nome: &str, a: &T, b: &T) {
            assert!(ron::to_string(a).unwrap() == ron::to_string(b).unwrap(), "{nome}");
        }
        assert_ron("data", &reg.data, &p2.economia.data);
        assert_eq!(reg.dias_decorridos, p2.economia.dias_decorridos);
        assert_ron("config", &reg.config, &p2.economia.config);
        assert_ron("mercados", &reg.mercados, &p2.economia.mercados);
        assert_ron("paises", &reg.paises, &p2.economia.paises);
        assert_ron("estados", &reg.estados, &p2.economia.estados);
        assert_ron("provincias", &reg.provincias, &p2.economia.provincias);
        assert_ron("rotas", &reg.rotas_comerciais, &p2.economia.rotas_comerciais);
        assert_ron("blocos", &reg.blocos_comerciais, &p2.economia.blocos_comerciais);
        assert_ron("ciclos_demanda", &reg.ciclos_demanda, &p2.economia.ciclos_demanda);
        let b_restaurado = n2
            .mundo
            .obter_provincia(&prov_usa)
            .unwrap()
            .predios
            .iter()
            .find(|b| b.tipo_predio_id == tipo_usa)
            .unwrap();
        assert!(b_restaurado.fechado && b_restaurado.dias_fechado >= 5);
        // Corrupção é detectada.
        let mut ruim = bytes.clone();
        let n = ruim.len();
        ruim[n - 20] ^= 0xFF;
        assert!(decodificar(&ruim).is_err());
    }

    #[test]
    fn salvar_carregar_divergencia_roundtrip() {
        let mut motor = simulacao::MotorDivergencia::novo(19450815);
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (_, mut mundo) = MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenário");
        // Avança alguns anos para gerar estado
        for ano in 1960..=1970 {
            motor.avaliar_passo(&mut mundo, simulacao::tempo::DataSimulacao::nova(ano, 6, 1), 35.0, &[]);
        }

        let p1 = Partida::montar(
            "USA", &mundo, &RelogioSimulacao::novo(),
            (sim_economia::EstadoEconomiaSalvo { data: Default::default(), dias_decorridos: 0, config: Default::default(), mercados: vec![], paises: vec![], estados: vec![], provincias: vec![], rotas_comerciais: vec![], blocos_comerciais: vec![], ciclos_demanda: Default::default() }, sim_economia::MercadoGlobal::default()),
            PoliticaSalva { mundo: sim_politica::EstadoPoliticoSalvo { data: Default::default(), dias_decorridos: 0, paises: vec![], rng: Default::default(), eventos_guerra_civil: vec![] }, base_eco: BTreeMap::new(), base_pops: BTreeMap::new() },
            sim_diplomacia::Diplomacia::novo(vec![]).salvar_estado(),
            BTreeMap::new(),
            EstadoEventos::default(),
            Some(motor),
            None,
            None,
        );
        let bytes = codificar(&p1).unwrap();
        let p2 = decodificar(&bytes).unwrap();
        let d1 = p1.divergencia.as_ref().unwrap();
        let d2 = p2.divergencia.as_ref().unwrap();
        assert_eq!(d2.semente, d1.semente);
        assert_eq!(d2.rng, d1.rng);
        assert_eq!(d2.desfecho, d1.desfecho);
        assert_eq!(d2.revolucoes_feitas, d1.revolucoes_feitas);
        assert_eq!(d2.golpes_feitos, d1.golpes_feitos);
        assert_eq!(d2.noticias_pendentes, d1.noticias_pendentes);
        assert_eq!(d2.guerras_extras_ativas, d1.guerras_extras_ativas);
        assert_eq!(d2.baixas_civis_acumuladas, d1.baixas_civis_acumuladas);
        assert_eq!(d2.baixas_militares_acumuladas, d1.baixas_militares_acumuladas);
    }

    #[test]
    fn salvar_carregar_guerra_roundtrip() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let catalogo_mapa: gf_mapa::mapa::CatalogoProvincias = ron::from_str(&std::fs::read_to_string(raiz.join("assets/map/provinces.ron")).unwrap()).unwrap();
        let mundo_base = MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenário")
        .1;
        let (mut mundo_guerra, _) = super::super::guerra_sim::construir_guerra_1945(&raiz, &catalogo_mapa, &mundo_base).expect("guerra 1945");
        // Declara uma guerra de teste
        let _ = mundo_guerra.declarar_guerra("USA", "SOV", "Guerra de Teste");
        let guerras_antes = mundo_guerra.guerras().len();
        let divisoes_antes = mundo_guerra.divisoes().len();
        let rng_antes = mundo_guerra.rng;

        let estado_guerra = mundo_guerra.salvar_estado();
        let p1 = Partida::montar(
            "USA", &mundo_base, &RelogioSimulacao::novo(),
            (sim_economia::EstadoEconomiaSalvo { data: Default::default(), dias_decorridos: 0, config: Default::default(), mercados: vec![], paises: vec![], estados: vec![], provincias: vec![], rotas_comerciais: vec![], blocos_comerciais: vec![], ciclos_demanda: Default::default() }, sim_economia::MercadoGlobal::default()),
            PoliticaSalva { mundo: sim_politica::EstadoPoliticoSalvo { data: Default::default(), dias_decorridos: 0, paises: vec![], rng: Default::default(), eventos_guerra_civil: vec![] }, base_eco: BTreeMap::new(), base_pops: BTreeMap::new() },
            sim_diplomacia::Diplomacia::novo(vec![]).salvar_estado(),
            BTreeMap::new(),
            EstadoEventos::default(),
            None,
            Some(estado_guerra),
            None,
        );
        let bytes = codificar(&p1).unwrap();
        let p2 = decodificar(&bytes).unwrap();
        let g1 = p1.guerra.as_ref().unwrap();
        let g2 = p2.guerra.as_ref().unwrap();
        assert_eq!(g2.cenario.guerras.len(), guerras_antes);
        assert_eq!(g2.cenario.divisoes.len(), divisoes_antes);
        assert_eq!(g2.rng, rng_antes);
        assert_eq!(g2.frentes_ativas.len(), 0); // sem tick, sem frentes
    }

    #[test]
    fn salvar_carregar_personagens_roundtrip() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/personagens");
        let elenco = personagens::Elenco::carregar(&dir, super::super::personagens_sim::INICIO).expect("carregar elenco");
        let lider_usa_original = elenco.lider("USA", personagens::Papel::Estado).unwrap().id.clone();
        let lider_sov_original = elenco.lider("SOV", personagens::Papel::Estado).unwrap().id.clone();

        let p1 = Partida::montar(
            "USA", &MundoJogo::carregar_de_arquivos(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/map/provinces.ron"),
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/scenario_1945/countries_1945.ron"),
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/scenario_1945/province_owners_1945.ron"),
            )
            .expect("cenário")
            .1,
            &RelogioSimulacao::novo(),
            (sim_economia::EstadoEconomiaSalvo { data: Default::default(), dias_decorridos: 0, config: Default::default(), mercados: vec![], paises: vec![], estados: vec![], provincias: vec![], rotas_comerciais: vec![], blocos_comerciais: vec![], ciclos_demanda: Default::default() }, sim_economia::MercadoGlobal::default()),
            PoliticaSalva { mundo: sim_politica::EstadoPoliticoSalvo { data: Default::default(), dias_decorridos: 0, paises: vec![], rng: Default::default(), eventos_guerra_civil: vec![] }, base_eco: BTreeMap::new(), base_pops: BTreeMap::new() },
            sim_diplomacia::Diplomacia::novo(vec![]).salvar_estado(),
            BTreeMap::new(),
            EstadoEventos::default(),
            None,
            None,
            Some(elenco),
        );
        let bytes = codificar(&p1).unwrap();
        let p2 = decodificar(&bytes).unwrap();
        let e1 = p1.personagens.as_ref().unwrap();
        let e2 = p2.personagens.as_ref().unwrap();
        assert_eq!(e2.lider("USA", personagens::Papel::Estado).unwrap().id, lider_usa_original);
        assert_eq!(e2.lider("SOV", personagens::Papel::Estado).unwrap().id, lider_sov_original);
        // Compara serialização completa para garantir que tudo bate
        assert_eq!(ron::to_string(e1).unwrap(), ron::to_string(e2).unwrap());
    }
}
