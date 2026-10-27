//! Plugin de áudio (T16): efeitos por evento, volume por canal e fila de música.
//!
//! Uso: escreva `SomTocar::novo("ui.botao.clique")` (nome de evento do `sons.ron`)
//! ou `SomTocar::novo("clique_botao")` (id de efeito) num `MessageWriter<SomTocar>`.
use super::map::{ProvinciaSelecionada, RaizProjeto};
use super::time::RecursoRelogio;
use crate::estado::AppState;
use bevy::audio::Volume;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::path::Path;

/// Pasta (relativa a `assets/`) de onde vêm os arquivos de som.
const PASTA_SONS: &str = "sons";
const ARQ_CONFIG: &str = "assets/data/sons.ron";

// ---------------------------------------------------------------- dados (RON)

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "CanalAudio")]
pub struct CanalSom {
    pub id: String,
    pub nome: String,
    pub volume: f32,
    pub mutado: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EfeitoSonoro {
    pub id: String,
    pub arquivo: String,
    pub canal: String,
    pub volume: f32,
    pub variacao_pitch: f32,
    pub limite_concorrencia: u32,
    pub cooldown_ms: u32,
    #[serde(default)]
    pub descricao: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MapeamentoEvento {
    pub evento: String,
    pub efeito_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigAudio {
    pub versao: u32,
    pub taxa_amostragem_padrao: u32,
    pub canais: Vec<CanalSom>,
    pub efeitos: Vec<EfeitoSonoro>,
    pub mapeamentos: Vec<MapeamentoEvento>,
}

impl ConfigAudio {
    pub fn de_ron(texto: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(texto)
    }
}

// ---------------------------------------------------------------- recursos

/// Canais de volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanalAudio {
    Master,
    Ui,
    Sfx,
    Musica,
    Ambiente,
}

impl CanalAudio {
    pub fn de_id(id: &str) -> Option<Self> {
        Some(match id {
            "master" => Self::Master,
            "ui" => Self::Ui,
            "sfx" => Self::Sfx,
            "musica" => Self::Musica,
            "ambiente" => Self::Ambiente,
            _ => return None,
        })
    }
}

/// Volume (0..1) e mudo por canal; editável em tempo de execução.
#[derive(Resource, Debug, Clone)]
pub struct VolumesAudio {
    canais: HashMap<CanalAudio, (f32, bool)>,
}

impl Default for VolumesAudio {
    fn default() -> Self {
        let canais = [
            (CanalAudio::Master, 1.0),
            (CanalAudio::Ui, 0.85),
            (CanalAudio::Sfx, 0.80),
            (CanalAudio::Musica, 0.60),
            (CanalAudio::Ambiente, 0.50),
        ]
        .into_iter()
        .map(|(c, v)| (c, (v, false)))
        .collect();
        Self { canais }
    }
}

impl VolumesAudio {
    pub fn de_config(cfg: &ConfigAudio) -> Self {
        let mut v = Self::default();
        for c in &cfg.canais {
            if let Some(canal) = CanalAudio::de_id(&c.id) {
                v.canais.insert(canal, (c.volume.clamp(0.0, 1.0), c.mutado));
            }
        }
        v
    }
    pub fn volume(&self, canal: CanalAudio) -> f32 {
        self.canais.get(&canal).map_or(1.0, |c| c.0)
    }
    pub fn definir(&mut self, canal: CanalAudio, volume: f32) {
        let m = self.canais.get(&canal).map_or(false, |c| c.1);
        self.canais.insert(canal, (volume.clamp(0.0, 1.0), m));
    }
    pub fn mutar(&mut self, canal: CanalAudio, mutado: bool) {
        let v = self.volume(canal);
        self.canais.insert(canal, (v, mutado));
    }
    pub fn mutado(&self, canal: CanalAudio) -> bool {
        self.canais.get(&canal).is_some_and(|c| c.1)
    }
    /// Ganho efetivo do canal (master × canal; 0 se mutado).
    pub fn ganho(&self, canal: CanalAudio) -> f32 {
        let g = |c| if self.mutado(c) { 0.0 } else { self.volume(c) };
        if canal == CanalAudio::Master { g(canal) } else { g(CanalAudio::Master) * g(canal) }
    }
}

/// Catálogo carregado do `sons.ron`, com índices por id e por evento.
#[derive(Resource, Debug, Default)]
pub struct CatalogoSons {
    pub config: Option<ConfigAudio>,
    efeito_por_id: HashMap<String, usize>,
    efeito_por_evento: HashMap<String, usize>,
}

impl CatalogoSons {
    pub fn novo(config: ConfigAudio) -> Self {
        let efeito_por_id: HashMap<_, _> = config.efeitos.iter().enumerate().map(|(i, e)| (e.id.clone(), i)).collect();
        let efeito_por_evento = config
            .mapeamentos
            .iter()
            .filter_map(|m| efeito_por_id.get(&m.efeito_id).map(|&i| (m.evento.clone(), i)))
            .collect();
        Self { config: Some(config), efeito_por_id, efeito_por_evento }
    }
    /// Resolve um nome de evento (`tempo.pausar`) ou um id de efeito (`pausa`).
    pub fn resolver(&self, nome: &str) -> Option<&EfeitoSonoro> {
        let i = self.efeito_por_evento.get(nome).or_else(|| self.efeito_por_id.get(nome))?;
        self.config.as_ref()?.efeitos.get(*i)
    }
}

/// Fila de músicas (caminhos relativos a `assets/sons/`); vazia até haver arquivos.
#[derive(Resource, Debug, Default)]
pub struct FilaMusica(pub VecDeque<String>);

/// Pedido de tocar um som (nome de evento ou id de efeito).
#[derive(Message, Debug, Clone)]
pub struct SomTocar(pub String);

impl SomTocar {
    pub fn novo(nome: impl Into<String>) -> Self {
        Self(nome.into())
    }
}

#[derive(Component)]
struct SomAtivo(String);

#[derive(Component)]
struct MusicaAtiva;

#[derive(Resource, Default)]
struct EstadoSons {
    ultimo: HashMap<String, f64>,
    semente: u32,
}

impl EstadoSons {
    /// Número pseudoaleatório em [-1, 1] (xorshift; sem dependência extra).
    fn ruido(&mut self) -> f32 {
        let mut x = self.semente.max(1);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.semente = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Volume final de um efeito (volume do efeito × ganho do canal).
pub fn volume_efeito(e: &EfeitoSonoro, vols: &VolumesAudio) -> f32 {
    let canal = CanalAudio::de_id(&e.canal).unwrap_or(CanalAudio::Sfx);
    (e.volume * vols.ganho(canal)).clamp(0.0, 1.5)
}

// ---------------------------------------------------------------- plugin

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SomTocar>()
            .init_resource::<VolumesAudio>()
            .init_resource::<CatalogoSons>()
            .init_resource::<FilaMusica>()
            .init_resource::<EstadoSons>()
            .add_systems(Startup, carregar_config)
            .add_systems(OnEnter(AppState::Pausado), |mut e: MessageWriter<SomTocar>| {
                e.write(SomTocar::novo("tempo.pausar"));
            })
            .add_systems(OnExit(AppState::Pausado), |mut e: MessageWriter<SomTocar>| {
                e.write(SomTocar::novo("tempo.despausar"));
            })
            .add_systems(OnTransition { exited: AppState::Menu, entered: AppState::Jogando }, |mut e: MessageWriter<SomTocar>| {
                e.write(SomTocar::novo("ui.botao.clique"));
            })
            .add_systems(Update, (sons_do_relogio, som_selecao, tocar_sons, tocar_musica, ajustar_volume_musica).chain());
    }
}

fn carregar_config(raiz: Option<Res<RaizProjeto>>, mut catalogo: ResMut<CatalogoSons>, mut vols: ResMut<VolumesAudio>, mut estado: ResMut<EstadoSons>) {
    estado.semente = 0x9E37_79B9;
    let base = raiz.map(|r| r.0.clone()).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let texto = match std::fs::read_to_string(base.join(ARQ_CONFIG)) {
        Ok(t) => t,
        Err(e) => return warn!("áudio desativado: {ARQ_CONFIG} ilegível ({e})"),
    };
    match ConfigAudio::de_ron(&texto) {
        Ok(cfg) => {
            *vols = VolumesAudio::de_config(&cfg);
            *catalogo = CatalogoSons::novo(cfg);
        }
        Err(e) => warn!("áudio desativado: {ARQ_CONFIG} inválido ({e})"),
    }
}

/// Pausa/despausa pelo relógio (Espaço ou botão) e troca de velocidade.
fn sons_do_relogio(relogio: Option<Res<RecursoRelogio>>, mut anterior: Local<Option<(bool, u8)>>, mut saida: MessageWriter<SomTocar>) {
    let Some(r) = relogio else { return };
    let atual = (r.pausado, r.velocidade.nivel() as u8);
    if let Some((pausado, nivel)) = *anterior {
        if atual.0 != pausado {
            saida.write(SomTocar::novo(if atual.0 { "tempo.pausar" } else { "tempo.despausar" }));
        } else if atual.1 != nivel {
            saida.write(SomTocar::novo("ui.botao.clique"));
        }
    }
    *anterior = Some(atual);
}

fn som_selecao(sel: Option<Res<ProvinciaSelecionada>>, mut saida: MessageWriter<SomTocar>) {
    if let Some(s) = sel
        && s.is_changed()
        && !s.is_added()
        && s.0 != 0
    {
        saida.write(SomTocar::novo("ui.painel.abrir"));
    }
}

#[allow(clippy::too_many_arguments)]
fn tocar_sons(
    mut leitor: MessageReader<SomTocar>,
    catalogo: Res<CatalogoSons>,
    vols: Res<VolumesAudio>,
    time: Res<Time>,
    mut estado: ResMut<EstadoSons>,
    ativos: Query<&SomAtivo>,
    servidor: Option<Res<AssetServer>>,
    mut commands: Commands,
) {
    let Some(servidor) = servidor else {
        leitor.clear();
        return;
    };
    let agora = time.elapsed_secs_f64();
    let mut novos: HashMap<String, u32> = HashMap::new();
    for pedido in leitor.read() {
        let Some(e) = catalogo.resolver(&pedido.0) else { continue };
        if let Some(&t) = estado.ultimo.get(&e.id)
            && (agora - t) * 1000.0 < e.cooldown_ms as f64
        {
            continue;
        }
        let em_uso = ativos.iter().filter(|a| a.0 == e.id).count() as u32 + novos.get(&e.id).copied().unwrap_or(0);
        if e.limite_concorrencia > 0 && em_uso >= e.limite_concorrencia {
            continue;
        }
        let volume = volume_efeito(e, &vols);
        if volume <= 0.0 {
            continue;
        }
        let velocidade = 1.0 + e.variacao_pitch * estado.ruido();
        estado.ultimo.insert(e.id.clone(), agora);
        *novos.entry(e.id.clone()).or_default() += 1;
        commands.spawn((
            AudioPlayer::new(servidor.load(format!("{PASTA_SONS}/{}", e.arquivo))),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)).with_speed(velocidade),
            SomAtivo(e.id.clone()),
        ));
    }
}

/// Toca a próxima música da fila quando nenhuma está ativa (fila vazia = silêncio).
fn tocar_musica(mut fila: ResMut<FilaMusica>, ativa: Query<(), With<MusicaAtiva>>, vols: Res<VolumesAudio>, servidor: Option<Res<AssetServer>>, mut commands: Commands) {
    let Some(servidor) = servidor else { return };
    if !ativa.is_empty() || fila.0.is_empty() {
        return;
    }
    let Some(arquivo) = fila.0.pop_front() else { return };
    commands.spawn((
        AudioPlayer::new(servidor.load(format!("{PASTA_SONS}/{arquivo}"))),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(vols.ganho(CanalAudio::Musica))),
        MusicaAtiva,
    ));
}

fn ajustar_volume_musica(vols: Res<VolumesAudio>, mut sinks: Query<&mut AudioSink, With<MusicaAtiva>>) {
    if vols.is_changed() {
        for mut s in &mut sinks {
            s.set_volume(Volume::Linear(vols.ganho(CanalAudio::Musica)));
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn carregar() -> ConfigAudio {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(ARQ_CONFIG);
        ConfigAudio::de_ron(&std::fs::read_to_string(p).expect("sons.ron")).expect("parse")
    }

    #[test]
    fn parseia_e_arquivos_existem() {
        let cfg = carregar();
        assert_eq!(cfg.canais.len(), 5);
        assert!(cfg.efeitos.len() >= 39);
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets").join(PASTA_SONS);
        for e in &cfg.efeitos {
            assert!(base.join(&e.arquivo).exists(), "faltando {}", e.arquivo);
            assert!(CanalAudio::de_id(&e.canal).is_some(), "canal {}", e.canal);
        }
    }

    #[test]
    fn mapeamentos_apontam_para_efeitos() {
        let cfg = carregar();
        let cat = CatalogoSons::novo(cfg.clone());
        for m in &cfg.mapeamentos {
            assert!(cat.resolver(&m.evento).is_some(), "evento sem efeito: {}", m.evento);
        }
        assert_eq!(cat.resolver("ui.botao.clique").unwrap().arquivo, "sfx/clique_botao.wav");
        assert_eq!(cat.resolver("tempo.pausar").unwrap().id, "pausa");
        assert_eq!(cat.resolver("despausa").unwrap().id, "despausa");
        assert!(cat.resolver("inexistente").is_none());
    }

    #[test]
    fn volumes_por_canal() {
        let cfg = carregar();
        let mut v = VolumesAudio::de_config(&cfg);
        let clique = CatalogoSons::novo(cfg).resolver("clique_botao").cloned().unwrap();
        assert!((volume_efeito(&clique, &v) - 0.80 * 0.85).abs() < 1e-5);
        v.definir(CanalAudio::Master, 0.5);
        assert!((volume_efeito(&clique, &v) - 0.80 * 0.85 * 0.5).abs() < 1e-5);
        v.mutar(CanalAudio::Ui, true);
        assert_eq!(volume_efeito(&clique, &v), 0.0);
    }

    #[test]
    fn ruido_limitado() {
        let mut e = EstadoSons { semente: 1, ..default() };
        for _ in 0..1000 {
            assert!((-1.0..=1.0).contains(&e.ruido()));
        }
    }
}
