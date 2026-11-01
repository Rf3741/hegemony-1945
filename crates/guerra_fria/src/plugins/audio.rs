//! Plugin de áudio (T16): efeitos por evento, volume por canal e fila de música.
//!
//! Uso: escreva `SomTocar::novo("ui.botao.clique")` (nome de evento do `sons.ron`)
//! ou `SomTocar::novo("clique_botao")` (id de efeito) num `MessageWriter<SomTocar>`.
use super::map::{ProvinciaSelecionada, RaizProjeto};
use super::time::RecursoRelogio;
use crate::estado::AppState;
use bevy::audio::Volume;
use bevy::prelude::*;
use bevy_egui::input::EguiWantsInput;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
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
            .init_resource::<MusicaEstado>()
            .init_resource::<AmbienteEstado>()
            .init_resource::<JanelaAudio>()
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
            .add_systems(Update, (sons_do_relogio, som_selecao, sons_de_ui, sons_de_eventos, tocar_sons, musica_por_clima, ambiente_por_camera).chain())
            .add_systems(EguiPrimaryContextPass, janela_audio);
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

// ---------------------------------------------------------------- sons de interface e de eventos

/// Clique em qualquer coisa do egui (botões, abas, itens) e atalhos que abrem/fecham painéis.
fn sons_de_ui(
    botoes: Res<ButtonInput<MouseButton>>,
    teclas: Res<ButtonInput<KeyCode>>,
    egui: Res<EguiWantsInput>,
    estado: Option<Res<State<AppState>>>,
    mut abertos: Local<Vec<KeyCode>>,
    mut saida: MessageWriter<SomTocar>,
) {
    if botoes.just_pressed(MouseButton::Left) && egui.is_pointer_over_area() {
        saida.write(SomTocar::novo("ui.botao.clique"));
    }
    if !matches!(estado.as_deref().map(|s| *s.get()), Some(AppState::Jogando | AppState::Pausado)) || egui.wants_keyboard_input() {
        return;
    }
    for tecla in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::KeyB, KeyCode::KeyJ] {
        if teclas.just_pressed(tecla) {
            if let Some(i) = abertos.iter().position(|t| *t == tecla) {
                abertos.remove(i);
                saida.write(SomTocar::novo("ui.painel.fechar"));
            } else {
                abertos.push(tecla);
                saida.write(SomTocar::novo("ui.painel.abrir"));
            }
        }
    }
}

/// Classifica o texto de uma notificação no evento sonoro correspondente.
pub fn evento_da_notificacao(texto: &str) -> &'static str {
    let t = texto.to_lowercase();
    if t.contains("obra concluída") || t.contains("inaugura") {
        "economia.construcao_finalizada"
    } else if t.contains("eleiç") || t.contains("eleic") {
        "politica.eleicao_concluida"
    } else if t.contains("declar") && t.contains("guerra") {
        "diplomacia.ultimato_enviado"
    } else if t.contains("calote") || t.contains("endividando") {
        "economia.falencia_fiscal"
    } else if t.contains("lei ") && (t.contains("promulg") || t.contains("aprovad")) {
        "politica.lei_promulgada"
    } else {
        "notificacao.informativa"
    }
}

/// Pop-ups de evento (carimbo), notícias do Jornal, leis trocadas e notificações do país.
#[allow(clippy::too_many_arguments)]
fn sons_de_eventos(
    ev: Option<Res<super::eventos_ui::EstadoEv>>,
    pol: Option<Res<super::politica_sim::PoliticaSim>>,
    notas: Option<ResMut<super::jogador::Notificacoes>>,
    mut anterior: Local<Option<(usize, usize, usize)>>,
    mut saida: MessageWriter<SomTocar>,
) {
    if let Some(mut n) = notas {
        for texto in n.drenar_novas() {
            saida.write(SomTocar::novo(evento_da_notificacao(&texto)));
        }
    }
    let (Some(ev), Some(pol)) = (ev, pol) else { return };
    let atual = (ev.estado.pendentes.len(), ev.estado.jornal.len(), pol.leis_trocadas.len());
    if let Some((pend, jornal, leis)) = *anterior {
        if atual.0 > pend {
            saida.write(SomTocar::novo("burocracia.carimbo_aplicado"));
        } else if atual.0 < pend {
            saida.write(SomTocar::novo("burocracia.despacho_assinado"));
        }
        if atual.1 > jornal {
            saida.write(SomTocar::novo("diplomacia.telegrama_recebido"));
        }
        if atual.2 > leis {
            saida.write(SomTocar::novo("politica.lei_promulgada"));
        }
    }
    *anterior = Some(atual);
}

// ---------------------------------------------------------------- música por clima

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clima {
    Menu,
    Paz,
    Tensao,
    Crise,
}

impl Clima {
    /// Faixa (relativa a `assets/`) do clima. Ver `assets/musica/CREDITOS.md`.
    pub fn arquivo(self) -> &'static str {
        match self {
            Self::Menu | Self::Paz => "musica/menu_dvorak_largo.ogg",
            Self::Tensao => "musica/tensao_grieg_mountain_king.ogg",
            Self::Crise => "musica/crise_holst_mars.ogg",
        }
    }
    /// Clima pela tensão mundial (0..100), com histerese de 5 pontos em torno dos limiares 40 e 75.
    pub fn da_tensao(tensao: f32, atual: Option<Clima>) -> Clima {
        let h = |limiar: f32, acima: bool| if acima { limiar - 5.0 } else { limiar };
        let tensao_ou_mais = tensao >= h(40.0, matches!(atual, Some(Self::Tensao | Self::Crise)));
        let crise = tensao >= h(75.0, atual == Some(Self::Crise));
        if crise {
            Self::Crise
        } else if tensao_ou_mais {
            Self::Tensao
        } else {
            Self::Paz
        }
    }
}

/// Segundos que dura o crossfade entre faixas.
const SEGUNDOS_CROSSFADE: f32 = 3.0;

#[derive(Component)]
struct MusicaAtiva {
    clima: Clima,
    /// Ganho do fade (0..1).
    fade: f32,
    /// Vai a 1 na faixa atual e a 0 nas que estão saindo.
    alvo: f32,
}

#[derive(Resource, Default)]
struct MusicaEstado {
    clima: Option<Clima>,
}

#[allow(clippy::too_many_arguments)]
fn musica_por_clima(
    estado: Option<Res<State<AppState>>>,
    diplo: Option<Res<super::diplomacia_ui::DiploSim>>,
    vols: Res<VolumesAudio>,
    time: Res<Time<Real>>,
    servidor: Option<Res<AssetServer>>,
    mut ms: ResMut<MusicaEstado>,
    mut faixas: Query<(Entity, &mut MusicaAtiva, Option<&mut AudioSink>)>,
    mut commands: Commands,
) {
    let Some(servidor) = servidor else { return };
    let Some(estado) = estado else { return };
    let desejado = match *estado.get() {
        AppState::Carregando | AppState::Menu => Clima::Menu,
        _ => Clima::da_tensao(diplo.map_or(0.0, |d| d.0.tensao), ms.clima.filter(|c| *c != Clima::Menu)),
    };
    if ms.clima != Some(desejado) {
        // Menu e Paz usam o mesmo arquivo: não reinicia a faixa.
        let mesma = faixas.iter().any(|(_, f, _)| f.alvo > 0.0 && f.clima.arquivo() == desejado.arquivo());
        if mesma {
            for (_, mut f, _) in &mut faixas {
                if f.alvo > 0.0 {
                    f.clima = desejado;
                }
            }
        } else {
            for (_, mut f, _) in &mut faixas {
                f.alvo = 0.0;
            }
            commands.spawn((
                AudioPlayer::new(servidor.load(desejado.arquivo())),
                PlaybackSettings::LOOP.with_volume(Volume::SILENT),
                MusicaAtiva { clima: desejado, fade: 0.0, alvo: 1.0 },
            ));
        }
        ms.clima = Some(desejado);
    }
    let passo = time.delta_secs() / SEGUNDOS_CROSSFADE;
    let ganho = vols.ganho(CanalAudio::Musica);
    for (e, mut f, sink) in &mut faixas {
        f.fade = if f.alvo > f.fade { (f.fade + passo).min(f.alvo) } else { (f.fade - passo).max(f.alvo) };
        if f.alvo == 0.0 && f.fade <= 0.0 {
            commands.entity(e).despawn();
        } else if let Some(mut s) = sink {
            s.set_volume(Volume::Linear(f.fade * ganho));
        }
    }
}

// ---------------------------------------------------------------- ambiente

#[derive(Component)]
struct AmbienteAtivo(&'static str);

#[derive(Resource, Default)]
struct AmbienteEstado {
    mar: f32,
    cidade: f32,
}

/// Mar quando a câmera está perto da costa; murmúrio de cidade com zoom bem próximo sobre terra.
#[allow(clippy::too_many_arguments)]
fn ambiente_por_camera(
    estado: Option<Res<State<AppState>>>,
    mapa: Option<Res<super::map::DadosMapa>>,
    camera: Query<(&Transform, &Projection), With<Camera2d>>,
    vols: Res<VolumesAudio>,
    time: Res<Time<Real>>,
    servidor: Option<Res<AssetServer>>,
    mut est: ResMut<AmbienteEstado>,
    mut sinks: Query<(&AmbienteAtivo, &mut AudioSink)>,
    existentes: Query<&AmbienteAtivo>,
    mut commands: Commands,
) {
    let (Some(servidor), Some(estado), Some(mapa)) = (servidor, estado, mapa) else { return };
    if !matches!(*estado.get(), AppState::Jogando | AppState::Pausado) {
        for (_, mut s) in &mut sinks {
            s.set_volume(Volume::SILENT);
        }
        return;
    }
    let (alvo_mar, alvo_cid) = match camera.single() {
        Ok((tf, Projection::Orthographic(o))) => {
            let (cx, cy) = (tf.translation.x, tf.translation.y);
            let raio = 40.0 * o.scale.max(0.2);
            let mut oceano = 0;
            let amostras = [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (0.7, 0.7), (-0.7, 0.7), (0.7, -0.7), (-0.7, -0.7)];
            for (dx, dy) in amostras {
                let id = super::map::pixel_do_mundo(Vec2::new(cx + dx * raio, cy + dy * raio), mapa.largura, mapa.altura)
                    .map_or(0, |(px, py)| mapa.ids[py as usize * mapa.largura as usize + px as usize]);
                if id == 0 {
                    oceano += 1;
                }
            }
            let frac = oceano as f32 / amostras.len() as f32;
            let perto = ((1.6 - o.scale) / 1.2).clamp(0.0, 1.0);
            // Mar: mais forte onde há costa (mistura de terra e oceano) ou mar aberto.
            let costa = if frac > 0.0 { 0.6 + 0.4 * (1.0 - (frac - 0.5).abs() * 2.0) } else { 0.0 };
            let cid = if frac < 0.2 { ((0.5 - o.scale) / 0.3).clamp(0.0, 1.0) } else { 0.0 };
            (costa * perto, cid)
        }
        _ => (0.0, 0.0),
    };
    let k = (time.delta_secs() * 0.8).min(1.0);
    est.mar += (alvo_mar - est.mar) * k;
    est.cidade += (alvo_cid - est.cidade) * k;
    let ganho = vols.ganho(CanalAudio::Ambiente);
    for (nome, nivel) in [("mar", est.mar), ("cidade", est.cidade)] {
        if !existentes.iter().any(|a| a.0 == nome) {
            commands.spawn((
                AudioPlayer::new(servidor.load(format!("{PASTA_SONS}/ambiente/{nome}.wav"))),
                PlaybackSettings::LOOP.with_volume(Volume::SILENT),
                AmbienteAtivo(if nome == "mar" { "mar" } else { "cidade" }),
            ));
        }
        for (a, mut s) in &mut sinks {
            if a.0 == nome {
                s.set_volume(Volume::Linear(nivel * ganho * 0.6));
            }
        }
    }
}

// ---------------------------------------------------------------- opções de áudio

#[derive(Resource, Default)]
struct JanelaAudio {
    aberta: bool,
}

/// Janela "Áudio" (volumes por canal): botão no menu e na pausa; a tecla 9 alterna.
fn janela_audio(
    mut contextos: EguiContexts,
    teclas: Res<ButtonInput<KeyCode>>,
    estado: Option<Res<State<AppState>>>,
    mut janela: ResMut<JanelaAudio>,
    mut vols: ResMut<VolumesAudio>,
) -> Result {
    if teclas.just_pressed(KeyCode::Digit9) || teclas.just_pressed(KeyCode::Numpad9) {
        janela.aberta = !janela.aberta;
    }
    let Some(estado) = estado else { return Ok(()) };
    if !matches!(*estado.get(), AppState::Menu | AppState::Pausado) && !janela.aberta {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?;
    if !janela.aberta {
        egui::Area::new(egui::Id::new("botao_audio")).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0]).show(ctx, |ui| {
            if ui.button("Áudio (9)").clicked() {
                janela.aberta = true;
            }
        });
        return Ok(());
    }
    let mut aberta = true;
    egui::Window::new("Áudio").open(&mut aberta).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0]).resizable(false).show(ctx, |ui| {
        for (canal, nome) in [
            (CanalAudio::Master, "Geral"),
            (CanalAudio::Musica, "Música"),
            (CanalAudio::Ui, "Interface"),
            (CanalAudio::Sfx, "Efeitos"),
            (CanalAudio::Ambiente, "Ambiente"),
        ] {
            ui.horizontal(|ui| {
                let mut v = vols.volume(canal);
                let mut mudo = vols.mutado(canal);
                if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text(nome)).changed() {
                    vols.definir(canal, v);
                }
                if ui.checkbox(&mut mudo, "mudo").changed() {
                    vols.mutar(canal, mudo);
                }
            });
        }
    });
    janela.aberta = aberta;
    Ok(())
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
    fn clima_pela_tensao() {
        assert_eq!(Clima::da_tensao(10.0, None), Clima::Paz);
        assert_eq!(Clima::da_tensao(50.0, None), Clima::Tensao);
        assert_eq!(Clima::da_tensao(80.0, None), Clima::Crise);
        // histerese: continua em crise até cair abaixo de 70
        assert_eq!(Clima::da_tensao(72.0, Some(Clima::Crise)), Clima::Crise);
        assert_eq!(Clima::da_tensao(72.0, Some(Clima::Tensao)), Clima::Tensao);
        assert_eq!(Clima::da_tensao(37.0, Some(Clima::Tensao)), Clima::Tensao);
    }

    #[test]
    fn arquivos_de_musica_e_ambiente_existem() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        for c in [Clima::Menu, Clima::Paz, Clima::Tensao, Clima::Crise] {
            assert!(assets.join(c.arquivo()).exists(), "{}", c.arquivo());
        }
        for n in ["mar", "cidade"] {
            assert!(assets.join(PASTA_SONS).join(format!("ambiente/{n}.wav")).exists());
        }
    }

    #[test]
    fn notificacoes_viram_eventos_validos() {
        let cfg = carregar();
        let cat = CatalogoSons::novo(cfg);
        for t in ["Obra concluída!", "Eleições realizadas", "Calote soberano", "Lei X promulgada", "Qualquer coisa", "Declararam guerra"] {
            assert!(cat.resolver(evento_da_notificacao(t)).is_some(), "{t}");
        }
    }

    #[test]
    fn ruido_limitado() {
        let mut e = EstadoSons { semente: 1, ..default() };
        for _ in 0..1000 {
            assert!((-1.0..=1.0).contains(&e.ruido()));
        }
    }
}
