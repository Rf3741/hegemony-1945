//! Configurações gerais (vídeo, áudio, jogo, interface): persistidas em
//! ~/.config/hegemony1945/config.ron, aplicadas ao iniciar e na hora ao mudar.
use super::audio::{CanalAudio, VolumesAudio};
use super::time::RecursoRelogio;
use crate::estado::AppState;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, VideoModeSelection, WindowMode};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use gf_mapa::tempo::VelocidadeJogo;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

pub const RESOLUCOES: [(u32, u32); 6] = [(1280, 720), (1366, 768), (1600, 900), (1920, 1080), (2560, 1440), (3840, 2160)];
/// Canais na ordem em que são gravados no arquivo.
pub const CANAIS: [(CanalAudio, &str); 5] = [
    (CanalAudio::Master, "Geral"),
    (CanalAudio::Musica, "Música"),
    (CanalAudio::Ui, "Interface"),
    (CanalAudio::Sfx, "Efeitos"),
    (CanalAudio::Ambiente, "Ambiente"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ModoTela {
    #[default]
    Janela,
    SemBordas,
    Exclusiva,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigVideo {
    pub modo: ModoTela,
    pub resolucao: (u32, u32),
    pub vsync: bool,
    pub escala_ui: f32,
    /// 0 = sem limite.
    pub limite_fps: u32,
}

impl Default for ConfigVideo {
    fn default() -> Self {
        Self { modo: ModoTela::Janela, resolucao: (1600, 900), vsync: true, escala_ui: 1.0, limite_fps: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigAudioUsuario {
    /// Ordem de [`CANAIS`].
    pub volumes: [f32; 5],
    pub mudos: [bool; 5],
}

impl Default for ConfigAudioUsuario {
    fn default() -> Self {
        Self { volumes: [1.0, 0.60, 0.85, 0.80, 0.50], mudos: [false; 5] }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigPartida {
    /// 1..=5
    pub velocidade_inicial: u8,
    /// Meses entre autosaves (0 = desligado).
    pub autosave_meses: u32,
    pub pausar_em_eventos: bool,
    pub mostrar_tutorial: bool,
}

impl Default for ConfigPartida {
    fn default() -> Self {
        Self { velocidade_inicial: 1, autosave_meses: 12, pausar_em_eventos: true, mostrar_tutorial: true }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigInterface {
    /// Multiplicador do tamanho das fontes.
    pub fonte: f32,
    pub tooltips: bool,
}

impl Default for ConfigInterface {
    fn default() -> Self {
        Self { fonte: 1.0, tooltips: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QualidadeTerreno {
    Baixo,
    #[default]
    Medio,
    Alto,
    Ultra,
}

impl QualidadeTerreno {
    pub const TODAS: [(Self, &'static str); 4] = [
        (Self::Baixo, "Baixo"),
        (Self::Medio, "Médio"),
        (Self::Alto, "Alto"),
        (Self::Ultra, "Ultra"),
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigGraficos {
    pub qualidade_terreno: QualidadeTerreno,
    pub hillshade: f32,
    pub ondas: f32,
    pub sombra_costas: f32,
}

impl Default for ConfigGraficos {
    fn default() -> Self {
        Self { qualidade_terreno: QualidadeTerreno::Medio, hillshade: 0.42, ondas: 0.08, sombra_costas: 0.55 }
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ConfigJogo {
    pub video: ConfigVideo,
    pub audio: ConfigAudioUsuario,
    pub partida: ConfigPartida,
    pub interface: ConfigInterface,
    pub graficos: ConfigGraficos,
}

pub fn caminho_config() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("hegemony1945").join("config.ron")
}

impl ConfigJogo {
    pub fn carregar() -> Self {
        std::fs::read_to_string(caminho_config()).ok().and_then(|t| ron::from_str::<Self>(&t).ok()).unwrap_or_default().normalizada()
    }

    pub fn salvar(&self) {
        let p = caminho_config();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(t) = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()) {
            let _ = std::fs::write(p, t);
        }
    }

    /// Mantém os valores dentro dos limites aceitos.
    pub fn normalizada(mut self) -> Self {
        self.video.escala_ui = self.video.escala_ui.clamp(0.75, 2.0);
        self.interface.fonte = self.interface.fonte.clamp(0.8, 1.6);
        self.partida.velocidade_inicial = self.partida.velocidade_inicial.clamp(1, 5);
        self.video.resolucao.0 = self.video.resolucao.0.clamp(800, 7680);
        self.video.resolucao.1 = self.video.resolucao.1.clamp(600, 4320);
        self.graficos.hillshade = self.graficos.hillshade.clamp(0.0, 1.0);
        self.graficos.ondas = self.graficos.ondas.clamp(0.0, 0.25);
        self.graficos.sombra_costas = self.graficos.sombra_costas.clamp(0.0, 1.0);
        for v in &mut self.audio.volumes {
            *v = v.clamp(0.0, 1.0);
        }
        self
    }

    /// Configuração da janela inicial (usada no `main`).
    pub fn janela(&self) -> Window {
        let mut w = Window { title: "Hegemony: 1945".into(), ..default() };
        w.resolution = bevy::window::WindowResolution::new(self.video.resolucao.0, self.video.resolucao.1);
        w.mode = modo_bevy(self.video.modo);
        w.present_mode = if self.video.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync };
        w
    }
}

fn modo_bevy(m: ModoTela) -> WindowMode {
    match m {
        ModoTela::Janela => WindowMode::Windowed,
        ModoTela::SemBordas => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        ModoTela::Exclusiva => WindowMode::Fullscreen(MonitorSelection::Current, VideoModeSelection::Current),
    }
}

static TOOLTIPS_ATIVOS: AtomicBool = AtomicBool::new(true);

/// Consultado por `enciclopedia::dica_com` antes de montar o tooltip.
pub fn tooltips_ativos() -> bool {
    TOOLTIPS_ATIVOS.load(Ordering::Relaxed)
}

/// Meses completos desde janeiro de 1945 e se hoje é dia de autosave.
pub fn dia_de_autosave(ano: i32, mes: u8, dia: u8, intervalo_meses: u32) -> bool {
    if intervalo_meses == 0 || dia != 1 {
        return false;
    }
    let meses = (ano - 1945) * 12 + (mes as i32 - 1);
    meses > 0 && meses % intervalo_meses as i32 == 0
}

pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ConfigJogo::carregar())
            .add_systems(Update, (aplicar_janela, aplicar_audio, aplicar_tooltips, tecla_tela_cheia, salvar_com_atraso))
            .add_systems(Last, limitar_fps)
            .add_systems(OnExit(AppState::Menu), velocidade_inicial)
            .add_systems(EguiPrimaryContextPass, aplicar_estilo_egui);
    }
}

fn aplicar_janela(cfg: Res<ConfigJogo>, mut janelas: Query<&mut Window, With<PrimaryWindow>>) {
    if !cfg.is_changed() {
        return;
    }
    let Ok(mut w) = janelas.single_mut() else { return };
    let modo = modo_bevy(cfg.video.modo);
    if w.mode != modo {
        w.mode = modo;
    }
    if cfg.video.modo == ModoTela::Janela {
        let (l, a) = (cfg.video.resolucao.0 as f32, cfg.video.resolucao.1 as f32);
        if (w.resolution.width() - l).abs() > 0.5 || (w.resolution.height() - a).abs() > 0.5 {
            w.resolution.set(l, a);
        }
    }
    let pm = if cfg.video.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync };
    if w.present_mode != pm {
        w.present_mode = pm;
    }
}

fn aplicar_audio(cfg: Res<ConfigJogo>, mut vols: ResMut<VolumesAudio>, mut primeira: Local<u8>) {
    // Espera o Startup do áudio (que lê o RON do projeto) antes de impor os valores do usuário.
    if !cfg.is_changed() && *primeira > 0 {
        return;
    }
    *primeira = primeira.saturating_add(1);
    for (i, (canal, _)) in CANAIS.iter().enumerate() {
        vols.definir(*canal, cfg.audio.volumes[i]);
        vols.mutar(*canal, cfg.audio.mudos[i]);
    }
}

fn aplicar_tooltips(cfg: Res<ConfigJogo>) {
    if cfg.is_changed() {
        TOOLTIPS_ATIVOS.store(cfg.interface.tooltips, Ordering::Relaxed);
    }
}

/// F11 ou Alt+Enter alternam entre janela e tela cheia (sem bordas).
fn tecla_tela_cheia(teclas: Res<ButtonInput<KeyCode>>, mut cfg: ResMut<ConfigJogo>) {
    let alt = teclas.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    if teclas.just_pressed(KeyCode::F11) || (alt && teclas.just_pressed(KeyCode::Enter)) {
        cfg.video.modo = if cfg.video.modo == ModoTela::Janela { ModoTela::SemBordas } else { ModoTela::Janela };
    }
}

fn salvar_com_atraso(cfg: Res<ConfigJogo>, tempo: Res<Time>, mut pendente: Local<Option<f32>>, mut inicio: Local<bool>) {
    if !*inicio {
        *inicio = true; // a inserção inicial não precisa gravar
        return;
    }
    if cfg.is_changed() {
        *pendente = Some(0.6);
    }
    if let Some(t) = pendente.as_mut() {
        *t -= tempo.delta_secs();
        if *t <= 0.0 {
            cfg.salvar();
            *pendente = None;
        }
    }
}

fn limitar_fps(cfg: Res<ConfigJogo>, mut ultimo: Local<Option<std::time::Instant>>) {
    if cfg.video.limite_fps > 0
        && let Some(u) = *ultimo
    {
        let alvo = std::time::Duration::from_secs_f64(1.0 / cfg.video.limite_fps as f64);
        let dt = u.elapsed();
        if dt < alvo {
            std::thread::sleep(alvo - dt);
        }
    }
    *ultimo = Some(std::time::Instant::now());
}

fn velocidade_inicial(cfg: Res<ConfigJogo>, mut relogio: ResMut<RecursoRelogio>) {
    let v = VelocidadeJogo::TODAS.iter().copied().find(|v| v.nivel() == cfg.partida.velocidade_inicial).unwrap_or(VelocidadeJogo::V1);
    relogio.definir_velocidade(v);
}

/// Escala da UI (zoom do egui) e tamanho de fonte (multiplica os estilos do tema).
fn aplicar_estilo_egui(mut contextos: EguiContexts, cfg: Res<ConfigJogo>, mut estado: Local<(f32, f32)>) -> Result {
    let ctx = contextos.ctx_mut()?;
    if (ctx.zoom_factor() - cfg.video.escala_ui).abs() > 0.001 {
        ctx.set_zoom_factor(cfg.video.escala_ui);
    }
    // estado.0 = fator de fonte já aplicado; estado.1 = tamanho do corpo logo após a aplicação.
    let corpo = ctx.global_style().text_styles.get(&egui::TextStyle::Body).map(|f| f.size).unwrap_or(14.0);
    let (aplicado, visto) = if estado.0 == 0.0 { (1.0, corpo) } else { *estado };
    // Se outro sistema reiniciou o tema, o corpo voltou ao tamanho base: nada aplicado.
    let aplicado = if (corpo - visto).abs() > 0.01 { 1.0 } else { aplicado };
    if (aplicado - cfg.interface.fonte).abs() > 0.001 || (corpo - visto).abs() > 0.01 {
        let r = cfg.interface.fonte / aplicado;
        let mut estilo = (*ctx.global_style()).clone();
        for f in estilo.text_styles.values_mut() {
            f.size *= r;
        }
        let novo_corpo = estilo.text_styles.get(&egui::TextStyle::Body).map(|f| f.size).unwrap_or(corpo);
        ctx.set_global_style(estilo);
        *estado = (cfg.interface.fonte, novo_corpo);
    } else {
        *estado = (aplicado, corpo);
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ron_ida_e_volta() {
        let mut c = ConfigJogo::default();
        c.video.modo = ModoTela::SemBordas;
        c.audio.volumes[2] = 0.3;
        let t = ron::ser::to_string_pretty(&c, ron::ser::PrettyConfig::default()).unwrap();
        assert_eq!(ron::from_str::<ConfigJogo>(&t).unwrap(), c);
        // Campos ausentes assumem o padrão.
        assert_eq!(ron::from_str::<ConfigJogo>("(video:(vsync:false))").unwrap().video.escala_ui, 1.0);
    }

    #[test]
    fn autosave_no_intervalo() {
        assert!(!dia_de_autosave(1945, 1, 1, 12));
        assert!(dia_de_autosave(1946, 1, 1, 12));
        assert!(dia_de_autosave(1945, 7, 1, 6));
        assert!(!dia_de_autosave(1945, 7, 2, 6));
        assert!(!dia_de_autosave(1946, 1, 1, 0));
    }
}
