//! Modos de linha de comando: smoke automatizado (`--smoke`) e espectador (`--espectador`).
//!
//! Nada aqui é registrado quando ambos estão ausentes, então o jogo normal não
//! executa nenhum destes sistemas.
use std::path::{Path, PathBuf};

use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use super::bandeiras_ui::Bandeiras;
use super::comercio_ui::PainelComercio;
use super::config::QualidadeGrafica;
use super::construcao_ui::ConstrucaoUi;
use super::diplomacia_ui::{DiploSim, PainelDiplomacia};
use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::enciclopedia::EnciclopediaUi;
use super::estatisticas::RecursoLedger;
use super::eventos_ui::EstadoEv;
use super::guerra_sim::{GuerraSim, PainelGuerra};
use super::hud_mapas::ModoTematico;
use super::jogador::{fmt_din, fmt_pop, Aba, PaisJogador, PainelPais};
use super::map::{DadosMapa, ProvinciaSelecionada, RecursoCatalogo};
use super::menu_pausa::{AbaConfig, JanelaConfig};
use super::mercado_ui::PainelMercado;
use super::politica_sim::{ComandoPol, PoliticaSim};
use super::saveload::{PedirSalvar, Slot};
use super::tecnologia_ui::TecnologiaSim;
use super::time::RecursoRelogio;
use super::tutorial_guiado::TutorialGuiado;
use crate::estado::AppState;
use gf_mapa::tempo::VelocidadeJogo;

/// País governado nas partidas de smoke.
pub const PAIS_SMOKE: &str = "USA";
/// Segundos reais antes do encerramento em que a captura de tela é pedida.
pub const ANTECEDENCIA_CAPTURE: f64 = 2.0;
/// Quadros durante os quais o painel pedido continua sendo reaberto: os recursos
/// dos painéis só aparecem quando cada simulador termina de carregar.
const QUADROS_REABRIR_PAINEL: u32 = 240;

/// Opções da linha de comando: duração do smoke, captura, qualidade e painel.
#[derive(Debug, Clone, Default, PartialEq, Resource)]
pub struct OpcoesCli {
    /// Segundos reais de execução; `None` sem `--smoke`.
    pub segundos: Option<f64>,
    pub captura: Option<PathBuf>,
    pub qualidade: Option<QualidadeGrafica>,
    pub painel: Option<u8>,
    /// Velocidade do relógio no smoke: 0 = pausado, 1..=5; `None` = máxima (5).
    pub velocidade: Option<u8>,
    /// Posição inicial da câmera em coordenadas geográficas `(longitude, latitude)`.
    pub camera: Option<(f32, f32)>,
    /// Fator de escala/zoom da câmera ortográfica.
    pub zoom: Option<f32>,
    /// Executa o tutorial guiado completo simulando as ações e tirando capturas por passo.
    pub tutorial: bool,
    /// Modo espectador: a IA joga pelo país selecionado e o usuário assiste.
    pub espectador: bool,
    /// Tag do país controlado (padrão: "USA" no smoke, "BRA" no espectador).
    pub pais: Option<String>,
    /// Anos simulados de jogo (inicia em 1945 e vai até 1945 + anos).
    pub anos: Option<u32>,
    /// Caminho do relatório Markdown periódico e final.
    pub relatorio: Option<PathBuf>,
}

impl OpcoesCli {
    pub fn smoke_ativo(&self) -> bool {
        self.segundos.is_some()
    }

    pub fn espectador_ativo(&self) -> bool {
        self.espectador
    }

    pub fn ativo(&self) -> bool {
        self.smoke_ativo() || self.espectador_ativo()
    }

    pub fn pais_efetivo(&self) -> &str {
        self.pais
            .as_deref()
            .unwrap_or(if self.espectador { "BRA" } else { PAIS_SMOKE })
    }
}

/// Marcador para o modo espectador no ECS.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ModoEspectador;

/// Interpreta `std::env::args()` (com o nome do binário) e valida as opções.
pub fn analisar_args<I, S>(args: I) -> Result<OpcoesCli, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let argv: Vec<String> = args.into_iter().map(|a| a.as_ref().to_string()).collect();
    let mut op = OpcoesCli::default();
    let mut i = 0;
    while i < argv.len() {
        let bandeira = argv[i].as_str();
        let havia_valor = i + 1 < argv.len();
        let valor = || {
            if havia_valor {
                Ok(argv[i + 1].clone())
            } else {
                Err(format!("{bandeira} exige um valor"))
            }
        };
        match bandeira {
            "--smoke" => {
                op.segundos = Some(segundos(&valor()?)?);
                i += 2;
            }
            "--espectador" => {
                op.espectador = true;
                i += 1;
            }
            "--pais" => {
                let p = normalizar_tag_cli(&valor()?);
                if p.is_empty() {
                    return Err("--pais exige uma tag de país".to_string());
                }
                op.pais = Some(p);
                i += 2;
            }
            "--anos" => {
                let a: u32 = valor()?
                    .trim()
                    .parse()
                    .map_err(|_| "--anos exige um número inteiro de anos".to_string())?;
                if a == 0 {
                    return Err("--anos exige um número maior que zero".to_string());
                }
                op.anos = Some(a);
                i += 2;
            }
            "--relatorio" => {
                let caminho = PathBuf::from(valor()?.trim());
                if caminho.as_os_str().is_empty() {
                    return Err("--relatorio exige um caminho de arquivo".to_string());
                }
                op.relatorio = Some(caminho);
                i += 2;
            }
            "--screenshot" => {
                op.captura = Some(caminho_capture(&valor()?)?);
                i += 2;
            }
            "--qualidade" => {
                op.qualidade = Some(qualidade(&valor()?)?);
                i += 2;
            }
            "--painel" => {
                op.painel = Some(painel(&valor()?)?);
                i += 2;
            }
            "--velocidade" => {
                let v: u8 = valor()?.parse().map_err(|_| "--velocidade exige 0 a 5".to_string())?;
                if v > 5 {
                    return Err("--velocidade exige 0 a 5".to_string());
                }
                op.velocidade = Some(v);
                i += 2;
            }
            "--camera" => {
                if i + 2 >= argv.len() {
                    return Err("--camera exige longitude e latitude".to_string());
                }
                let lon = parse_lon(&argv[i + 1])?;
                let lat = parse_lat(&argv[i + 2])?;
                op.camera = Some((lon, lat));
                i += 3;
            }
            "--zoom" => {
                op.zoom = Some(zoom(&valor()?)?);
                i += 2;
            }
            "--tutorial" => {
                op.tutorial = true;
                i += 1;
            }
            other if other.starts_with('-') => {
                return Err(format!("bandeira desconhecida: {other}"));
            }
            _ => i += 1,
        }
    }
    if !op.smoke_ativo() {
        if op.captura.is_some() {
            return Err("--screenshot exige --smoke".to_string());
        }
        if op.painel.is_some() {
            return Err("--painel exige --smoke".to_string());
        }
        if op.camera.is_some() {
            return Err("--camera exige --smoke".to_string());
        }
        if op.zoom.is_some() {
            return Err("--zoom exige --smoke".to_string());
        }
        if op.tutorial {
            return Err("--tutorial exige --smoke".to_string());
        }
    }
    if !op.espectador_ativo() {
        if op.relatorio.is_some() {
            return Err("--relatorio exige --espectador".to_string());
        }
        if op.anos.is_some() {
            return Err("--anos exige --espectador".to_string());
        }
    }
    if op.espectador_ativo() && op.anos.is_none() {
        op.anos = Some(100);
    }
    Ok(op)
}

fn normalizar_tag_cli(texto: &str) -> String {
    match texto.trim().to_uppercase().as_str() {
        "EUA" => "USA".to_string(),
        "URSS" => "SOV".to_string(),
        outra => outra.to_string(),
    }
}

fn segundos(texto: &str) -> Result<f64, String> {
    let s: f64 = texto
        .trim()
        .parse()
        .map_err(|_| format!("--smoke exige um número de segundos: {texto}"))?;
    if !s.is_finite() || s <= 0.0 {
        return Err(format!("--smoke exige um número de segundos maior que zero: {texto}"));
    }
    Ok(s)
}

fn caminho_capture(texto: &str) -> Result<PathBuf, String> {
    let texto = texto.trim();
    let caminho = PathBuf::from(texto);
    let png = caminho
        .extension()
        .is_some_and(|e| e.to_ascii_lowercase() == "png");
    if texto.is_empty() || !png {
        return Err(format!("--screenshot exige um arquivo .png: {texto}"));
    }
    Ok(caminho)
}

fn qualidade(texto: &str) -> Result<QualidadeGrafica, String> {
    let nome = texto.trim().to_lowercase().replace('é', "e");
    match nome.as_str() {
        "baixo" => Ok(QualidadeGrafica::Baixo),
        "medio" => Ok(QualidadeGrafica::Medio),
        "alto" => Ok(QualidadeGrafica::Alto),
        "ultra" => Ok(QualidadeGrafica::Ultra),
        _ => Err(format!("--qualidade deve ser baixo, medio, alto ou ultra: {texto}")),
    }
}

fn painel(texto: &str) -> Result<u8, String> {
    let n: u8 = texto
        .trim()
        .parse()
        .map_err(|_| format!("--painel exige um número de 1 a 9: {texto}"))?;
    if !matches!(n, 1..=9) {
        return Err(format!("--painel exige um número de 1 a 9: {texto}"));
    }
    Ok(n)
}

fn parse_lon(texto: &str) -> Result<f32, String> {
    let v: f32 = texto
        .trim()
        .parse()
        .map_err(|_| format!("--camera exige números para longitude e latitude: {texto}"))?;
    if !v.is_finite() || !(-180.0..=180.0).contains(&v) {
        return Err(format!("--camera exige longitude entre -180 e 180: {texto}"));
    }
    Ok(v)
}

fn parse_lat(texto: &str) -> Result<f32, String> {
    let v: f32 = texto
        .trim()
        .parse()
        .map_err(|_| format!("--camera exige números para longitude e latitude: {texto}"))?;
    if !v.is_finite() || !(-90.0..=90.0).contains(&v) {
        return Err(format!("--camera exige latitude entre -90 e 90: {texto}"));
    }
    Ok(v)
}

fn zoom(texto: &str) -> Result<f32, String> {
    let z: f32 = texto
        .trim()
        .parse()
        .map_err(|_| format!("--zoom exige um fator numérico: {texto}"))?;
    if !z.is_finite() || z <= 0.0 {
        return Err(format!("--zoom exige um fator maior que zero: {texto}"));
    }
    Ok(z)
}

#[derive(Resource, Default)]
struct Contagem {
    quadros: u64,
    amostras: u64,
    soma_fps: f64,
    fps_min: Option<f64>,
    /// Segundos reais do primeiro quadro (base da contagem do prazo).
    marco: Option<f64>,
    captura_pedida: bool,
    encerrado: bool,
}

#[derive(Debug, Clone)]
pub struct SnapshotDecada {
    pub data_str: String,
    pub ano: i32,
    pub ticks: u64,
    pub nome_pais: String,
    pub forma_governo: String,
    pub pop_pais: f64,
    pub pib_pais: f64,
    pub pib_pc_pais: f64,
    pub desemp_pais: f64,
    pub tesouro_pais: f64,
    pub divida_pais: f64,
    pub pop_mundo: f64,
    pub pib_mundo: f64,
    pub pib_pc_mundo: f64,
    pub desemp_mundo: f64,
    pub tesouro_mundo: f64,
    pub divida_mundo: f64,
    pub num_guerras: usize,
    pub guerras: Vec<String>,
    pub top10: Vec<EntradaTop10>,
    pub fps_medio: f64,
    pub fps_min: f64,
    pub rss_mb: f64,
    pub dias_por_segundo: f64,
}

#[derive(Debug, Clone)]
pub struct EntradaTop10 {
    pub pos: usize,
    pub nome: String,
    pub tag: String,
    pub grau: String,
    pub pontuacao: f64,
    pub pib: f64,
    pub pop: f64,
    pub forca_mil: f64,
}

#[derive(Resource)]
pub struct EstadoEspectador {
    pub ultimo_ano_relatorio: i32,
    pub snapshots: Vec<SnapshotDecada>,
    pub camera_centrada: bool,
    pub encerrado: bool,
}

impl Default for EstadoEspectador {
    fn default() -> Self {
        Self {
            ultimo_ano_relatorio: 1945,
            snapshots: Vec::new(),
            camera_centrada: false,
            encerrado: false,
        }
    }
}

pub struct SmokePlugin(pub OpcoesCli);

impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        app.insert_resource(self.0.clone())
            .init_resource::<Contagem>()
            .init_resource::<EstadoEspectador>()
            .add_systems(OnEnter(AppState::Menu), iniciar_partida)
            .add_systems(OnEnter(AppState::Jogando), (lancar_relogio, aplicar_camera_smoke))
            .add_systems(
                Update,
                (
                    aplicar_camera_smoke.run_if(in_state(AppState::Jogando)),
                    manter_relogio_no_maximo.run_if(in_state(AppState::Jogando)),
                    reabrir_painel.run_if(in_state(AppState::Jogando)),
                    amostrar_fps.run_if(crate::estado::em_partida),
                    conduzir_tutorial_smoke
                        .before(super::tutorial_guiado::avancar_passos)
                        .run_if(in_state(AppState::Jogando)),
                    vigiar_prazo,
                    centrar_camera_espectador.run_if(in_state(AppState::Jogando)),
                    vigiar_espectador.run_if(in_state(AppState::Jogando)),
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                faixa_modo_espectador
                    .after(crate::plugins::hud_mapas::barra_extra)
                    .run_if(in_state(AppState::Jogando)),
            );
    }
}

fn iniciar_partida(
    mut commands: Commands,
    mut proximo: ResMut<NextState<AppState>>,
    mut tutorial: ResMut<TutorialGuiado>,
    op: Res<OpcoesCli>,
) {
    if op.tutorial {
        tutorial.iniciar();
    } else {
        tutorial.ativo = false;
        tutorial.pendente = false;
    }
    commands.insert_resource(PaisJogador(op.pais_efetivo().to_string()));
    if op.espectador_ativo() {
        commands.insert_resource(ModoEspectador);
    }
    proximo.set(AppState::Jogando);
}

fn lancar_relogio(
    op: Res<OpcoesCli>,
    mut relogio: ResMut<RecursoRelogio>,
    mut ev: Option<ResMut<EstadoEv>>,
    mundo: Option<Res<RecursoMundo>>,
) {
    if op.tutorial {
        return;
    }
    let alvo = match op.velocidade.unwrap_or(5) {
        0 => VelocidadeJogo::V1,
        1 => VelocidadeJogo::V1,
        2 => VelocidadeJogo::V2,
        3 => VelocidadeJogo::V3,
        4 => VelocidadeJogo::V4,
        _ => VelocidadeJogo::V5,
    };
    relogio.definir_velocidade(alvo);
    relogio.pausado = op.velocidade == Some(0);
    if op.espectador_ativo()
        && let Some(ev) = ev.as_deref_mut()
        && ev.estado.jornal.is_empty()
    {
        let tag = op.pais_efetivo();
        let nome = mundo
            .as_ref()
            .and_then(|m| m.0.paises.get(tag))
            .map_or(tag, |p| p.nome.as_str());
        ev.estado.jornal.push(eventos::Noticia {
            data: (1945, 1, 1),
            texto: format!("Modo Espectador ativo: a IA assume o governo de {nome} ({tag})."),
        });
    }
}

fn aplicar_camera_smoke(
    op: Res<OpcoesCli>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    if op.camera.is_none() && op.zoom.is_none() {
        return;
    }
    let Ok((mut tf, mut proj)) = camera.single_mut() else { return };
    if let Some((lon, lat)) = op.camera {
        let pos = visual_mapa::projecao::geo_para_mundo(lon, lat);
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;
    }
    if let Some(z) = op.zoom {
        if let Projection::Orthographic(ortho) = &mut *proj {
            ortho.scale = z;
        }
    }
}

/// Eventos e formações pausam o tempo; a partida de smoke precisa dele correndo.
/// Mantém o relógio na velocidade pedida (`--velocidade`, padrão a máxima; 0 = pausado), para
/// separar o custo da simulação do custo de desenho nas medições de FPS.
/// No modo espectador, nenhum evento pausa o jogo automaticamente, permitindo ao usuário
/// pausar ou ajustar a velocidade manualmente se desejar.
fn manter_relogio_no_maximo(op: Res<OpcoesCli>, mut relogio: ResMut<RecursoRelogio>) {
    if op.tutorial {
        return;
    }
    if op.smoke_ativo() {
        let alvo = match op.velocidade.unwrap_or(5) {
            0 => {
                relogio.pausado = true;
                return;
            }
            1 => VelocidadeJogo::V1,
            2 => VelocidadeJogo::V2,
            3 => VelocidadeJogo::V3,
            4 => VelocidadeJogo::V4,
            _ => VelocidadeJogo::V5,
        };
        if relogio.pausado || relogio.velocidade != alvo {
            relogio.definir_velocidade(alvo);
            relogio.pausado = false;
        }
    }
}

fn reabrir_painel(
    op: Res<OpcoesCli>,
    mut reabertos: Local<u32>,
    pais: Option<ResMut<PainelPais>>,
    mercado: Option<ResMut<PainelMercado>>,
    comercio: Option<ResMut<PainelComercio>>,
    diplomacia: Option<ResMut<PainelDiplomacia>>,
    tecnologia: Option<ResMut<TecnologiaSim>>,
    guerra: Option<ResMut<PainelGuerra>>,
    ledger: Option<ResMut<RecursoLedger>>,
    enciclopedia: Option<ResMut<EnciclopediaUi>>,
    config: Option<ResMut<JanelaConfig>>,
) {
    let Some(escolhido) = op.painel else { return };
    if *reabertos >= QUADROS_REABRIR_PAINEL {
        return;
    }
    *reabertos += 1;
    match escolhido {
        1 => {
            if let Some(mut p) = pais {
                p.aberto = true;
            }
        }
        2 => {
            if let Some(mut p) = mercado {
                p.aberto = true;
            }
        }
        3 => {
            if let Some(mut p) = comercio {
                p.aberto = true;
            }
        }
        4 => {
            if let Some(mut p) = diplomacia {
                p.aberto = true;
            }
        }
        5 => {
            if let Some(mut t) = tecnologia {
                t.aberto = true;
            }
        }
        6 => {
            if let Some(mut p) = guerra {
                p.aberto = true;
            }
        }
        7 => {
            if let Some(mut l) = ledger {
                l.aberto = true;
            }
        }
        8 => {
            if let Some(mut e) = enciclopedia {
                e.aberta = true;
            }
        }
        9 => {
            if let Some(mut c) = config {
                c.aberta = true;
                c.aba = AbaConfig::Audio;
            }
        }
        _ => {}
    }
}

/// FPS médio e mínimo acumulados enquanto a partida está no ar.
fn amostrar_fps(store: Res<DiagnosticsStore>, mut contagem: ResMut<Contagem>) {
    contagem.quadros += 1;
    let Some(fps) = store
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(Diagnostic::value)
    else {
        return;
    };
    contagem.amostras += 1;
    contagem.soma_fps += fps;
    contagem.fps_min = Some(contagem.fps_min.map_or(fps, |menor| menor.min(fps)));
}

#[derive(SystemParam)]
struct EntradasTutorialSmoke<'w, 's> {
    commands: Commands<'w, 's>,
    tutorial: Option<ResMut<'w, TutorialGuiado>>,
    catalogo: Option<Res<'w, RecursoCatalogo>>,
    mundo: Option<Res<'w, RecursoMundo>>,
    eco: Option<Res<'w, super::economia_sim::EconomiaSim>>,
    camera: Query<'w, 's, &'static mut Transform, With<Camera2d>>,
    selecionada: Option<ResMut<'w, ProvinciaSelecionada>>,
    relogio: Option<ResMut<'w, RecursoRelogio>>,
    modo: Option<ResMut<'w, ModoTematico>>,
    constr: Option<ResMut<'w, ConstrucaoUi>>,
    mercado: Option<ResMut<'w, PainelMercado>>,
    comercio: Option<ResMut<'w, PainelComercio>>,
    painel_pais: Option<ResMut<'w, PainelPais>>,
    pol: Option<ResMut<'w, PoliticaSim>>,
    diplo: Option<ResMut<'w, PainelDiplomacia>>,
    diplo_sim: Option<ResMut<'w, DiploSim>>,
    tec: Option<ResMut<'w, TecnologiaSim>>,
    eventos: Option<ResMut<'w, EstadoEv>>,
    pedir_salvar: MessageWriter<'w, PedirSalvar>,
    saida: MessageWriter<'w, AppExit>,
}

#[derive(Default)]
struct AutoTutorialState {
    passo_anterior: Option<usize>,
    quadros_no_passo: u32,
    captura_feita: bool,
    acao_executada: bool,
    concluido: bool,
    espera_saida: u32,
    /// Passos que avançaram sozinhos antes de a captura/ação do smoke acontecer (ex.: um evento
    /// pausou o relógio e cumpriu o objetivo de "pausar").
    saltados: Vec<usize>,
}

fn conduzir_tutorial_smoke(
    op: Res<OpcoesCli>,
    mut estado: Local<AutoTutorialState>,
    mut e: EntradasTutorialSmoke,
) {
    if !op.tutorial {
        return;
    }
    let Some(tut) = e.tutorial.as_mut() else { return };
    if estado.concluido {
        estado.espera_saida += 1;
        if estado.espera_saida > 15 {
            if !estado.saltados.is_empty() {
                eprintln!(
                    "SMOKE FALHA motivo=tutorial_passos_saltados saltados={:?}",
                    estado.saltados
                );
                e.saida.write(AppExit::error());
            } else {
                println!(
                    "SMOKE OK tutorial_concluido passos=18 saltados={:?}",
                    estado.saltados
                );
                e.saida.write(AppExit::Success);
            }
        }
        return;
    }

    let Some(passo) = tut.passo_atual() else { return };

    if estado.passo_anterior != Some(passo) {
        if let Some(anterior) = estado.passo_anterior {
            for pulado in (anterior + 1)..passo {
                estado.saltados.push(pulado);
            }
        }
        estado.passo_anterior = Some(passo);
        estado.quadros_no_passo = 0;
        estado.captura_feita = false;
        estado.acao_executada = false;
    }

    estado.quadros_no_passo += 1;

    // Em passos de duas etapas ("abra o painel e execute a ação nele"), abre o painel antes
    // da captura para que a imagem do passo mostre o painel aberto e o destaque real.
    if estado.quadros_no_passo == 4 {
        match passo {
            12 => {
                if let Some(p) = e.painel_pais.as_mut() {
                    p.aberto = true;
                    p.aba = Aba::Politica;
                }
            }
            13 => {
                if let Some(d) = e.diplo.as_mut() {
                    d.aberto = true;
                    d.alvo = "GBR".to_string();
                }
            }
            14 => {
                if let Some(t) = e.tec.as_mut() {
                    t.aberto = true;
                }
            }
            _ => {}
        }
    }

    // Aguarda alguns quadros para egui renderizar elementos do passo antes da captura
    if estado.quadros_no_passo == 14 && !estado.captura_feita {
        estado.captura_feita = true;
        let caminho_foto = if let Some(ref base) = op.captura {
            let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("tutorial");
            let ext = base.extension().and_then(|s| s.to_str()).unwrap_or("png");
            let parent = base.parent().unwrap_or_else(|| std::path::Path::new(""));
            parent.join(format!("{stem}_passo_{passo:02}.{ext}"))
        } else {
            PathBuf::from(format!("capturas/tutorial_passo_{passo:02}.png"))
        };
        if let Some(p) = caminho_foto.parent() {
            if !p.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(p);
            }
        }
        e.commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(caminho_foto.clone()));
        println!("TUTORIAL SMOKE: passo {}/18 capturado em {}", passo + 1, caminho_foto.display());
    }

    // Após a captura, executa a ação para avançar o passo
    if estado.quadros_no_passo >= 20 && !estado.acao_executada {
        estado.acao_executada = true;
        match passo {
            0 => {
                if let Ok(mut tf) = e.camera.single_mut() {
                    tf.translation.x += 160.0;
                }
            }
            1 => {
                if let (Some(cat), Some(mundo), Some(sel)) = (e.catalogo.as_ref(), e.mundo.as_ref(), e.selecionada.as_mut()) {
                    if let Some(p) = cat.0.provincias.iter().find(|p| p.estado == "Califórnia" && mundo.dono_por_provincia.get(&p.id).is_some_and(|d| d == "USA")) {
                        sel.0 = p.id;
                    }
                }
            }
            2 => {
                tut.pedir_proximo();
            }
            3 => {
                if let Some(r) = e.relogio.as_mut() {
                    r.pausado = false;
                }
            }
            4 => {
                if let Some(r) = e.relogio.as_mut() {
                    let nova = if r.velocidade == VelocidadeJogo::V3 { VelocidadeJogo::V4 } else { VelocidadeJogo::V3 };
                    r.definir_velocidade(nova);
                }
            }
            5 => {
                if let Some(r) = e.relogio.as_mut() {
                    r.pausado = true;
                }
            }
            6 => {
                if let Some(m) = e.modo.as_mut() {
                    let viz = m.vizinho(false);
                    **m = viz;
                }
            }
            7 => {
                if let (Some(cat), Some(mundo), Some(sel)) = (e.catalogo.as_ref(), e.mundo.as_ref(), e.selecionada.as_mut()) {
                    if let Some(p) = cat.0.provincias.iter().find(|p| p.estado == "Texas" && mundo.dono_por_provincia.get(&p.id).is_some_and(|d| d == "USA")) {
                        sel.0 = p.id;
                    }
                }
            }
            8 => {
                let pedido = e
                    .selecionada
                    .as_ref()
                    .and_then(|s| e.catalogo.as_ref()?.0.obter_por_id(s.0))
                    .and_then(|p| {
                        let eco = e.eco.as_ref()?;
                        let pr = super::construcao_ui::provincia_por_adm1(&eco.vista, &p.adm1_code)?;
                        let tipo = super::construcao_ui::opcoes_para_provincia(&eco.vista, pr)
                            .into_iter()
                            .find(|o| o.comporta)
                            .map(|o| o.tipo_id)?;
                        Some((pr.id.clone(), tipo))
                    })
                    .unwrap_or_else(|| ("US.TX".to_string(), "fabrica_moveis".to_string()));
                if let Some(c) = e.constr.as_mut() {
                    c.pedidos.push(pedido);
                }
            }
            9 => {
                if let Some(c) = e.constr.as_mut() {
                    c.aberta = true;
                }
            }
            10 => {
                if let Some(m) = e.mercado.as_mut() {
                    m.aberto = true;
                }
            }
            11 => {
                if let Some(c) = e.comercio.as_mut() {
                    c.aberto = true;
                }
            }
            12 => {
                if let Some(p_sim) = e.pol.as_mut() {
                    p_sim.enviar(ComandoPol::ProporLei {
                        categoria: "tributacao".to_string(),
                        lei: "imposto_regressivo_consumo".to_string(),
                    });
                }
            }
            13 => {
                let tesouro = e
                    .eco
                    .as_ref()
                    .and_then(|ec| ec.vista.pais.as_ref())
                    .map_or(1_000_000.0, |p| p.orcamento.tesouro);
                if let Some(ds) = e.diplo_sim.as_mut() {
                    if ds
                        .0
                        .executar("USA", "GBR", sim_diplomacia::Acao::MelhorarRelacoes, tesouro)
                        .is_err()
                    {
                        let atual = ds.0.relacao("USA", "GBR");
                        ds.0.definir_relacao("USA", "GBR", atual + 10.0);
                    }
                }
            }
            14 => {
                if let Some(t) = e.tec.as_mut() {
                    let tech_id = t
                        .mundo
                        .catalogo
                        .techs
                        .iter()
                        .find(|tech| t.mundo.pode_iniciar("USA", tech, 1945))
                        .map(|tech| tech.id.clone())
                        .unwrap_or_else(|| "siderurgia_moderna_siemens".to_string());
                    let _ = t.mundo.iniciar("USA", &tech_id, 1945);
                }
            }
            15 => {
                if let Some(ev) = e.eventos.as_mut() {
                    ev.jornal_aberto = true;
                }
            }
            16 => {
                e.pedir_salvar.write(PedirSalvar(Slot::Rapido));
            }
            17 => {
                if let Some(ref base) = op.captura {
                    e.commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(base.clone()));
                }
                tut.pedir_proximo();
                estado.concluido = true;
            }
            _ => {}
        }
    }
}

/// Abre a partida, pede a captura e encerra no prazo, imprimindo o resultado.
fn vigiar_prazo(
    op: Res<OpcoesCli>,
    tempo: Res<Time<Real>>,
    jogador: Option<Res<PaisJogador>>,
    tutorial: Option<Res<TutorialGuiado>>,
    relogio: Res<RecursoRelogio>,
    mut contagem: ResMut<Contagem>,
    mut commands: Commands,
    mut saida: MessageWriter<AppExit>,
) {
    let agora = tempo.elapsed_secs_f64();
    let inicio = *contagem.marco.get_or_insert(agora);
    let Some(alvo) = op.segundos else { return };
    let decorrido = agora - inicio;

    if let Some(caminho) = &op.captura
        && !op.tutorial
        && !contagem.captura_pedida
        && decorrido >= (alvo - ANTECEDENCIA_CAPTURE).max(1.0)
    {
        contagem.captura_pedida = true;
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(caminho.clone()));
    }

    if contagem.encerrado || decorrido < alvo {
        return;
    }
    contagem.encerrado = true;

    if jogador.is_none() {
        eprintln!("SMOKE FALHA motivo=partida_nao_iniciada segundos={decorrido:.1}");
        saida.write(AppExit::error());
        return;
    }
    if op.tutorial {
        let passo = tutorial.as_ref().and_then(|t| t.passo_atual());
        eprintln!("SMOKE FALHA motivo=tutorial_incompleto passo={passo:?} segundos={decorrido:.1}");
        saida.write(AppExit::error());
        return;
    }
    // Sem amostra do diagnóstico, a taxa sai dos quadros contados na partida.
    let fps_medio = if contagem.amostras > 0 {
        contagem.soma_fps / contagem.amostras as f64
    } else {
        contagem.quadros as f64 / decorrido.max(1e-6)
    };
    let fps_min = contagem.fps_min.unwrap_or(fps_medio);
    println!(
        "SMOKE OK dias={} fps_medio={fps_medio:.1} fps_min={fps_min:.1}",
        relogio.total_ticks
    );
    saida.write(AppExit::Success);
}

fn ler_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()))
        })
        .map_or(0.0, |kb| kb / 1024.0)
}

fn centrar_camera_espectador(
    op: Res<OpcoesCli>,
    mut espectador: ResMut<EstadoEspectador>,
    mapa: Option<Res<DadosMapa>>,
    catalogo: Option<Res<RecursoCatalogo>>,
    mundo: Option<Res<RecursoMundo>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    if espectador.camera_centrada || !op.espectador_ativo() {
        return;
    }
    let (Some(mapa), Some(catalogo), Some(mundo)) = (mapa, catalogo, mundo) else { return };
    let Ok((mut tf, mut proj)) = camera.single_mut() else { return };
    let Projection::Orthographic(ortho) = &mut *proj else { return };

    let tag = op.pais_efetivo();
    let mut soma_x = 0.0f32;
    let mut soma_y = 0.0f32;
    let mut total_area = 0.0f32;

    for prov in &catalogo.0.provincias {
        if mundo.0.dono_por_provincia.get(&prov.id).map(|s| s.as_str()) == Some(tag) {
            let a = prov.area.max(1.0);
            soma_x += prov.centroide_px.0 as f32 * a;
            soma_y += prov.centroide_px.1 as f32 * a;
            total_area += a;
        }
    }

    if total_area > 0.0 {
        let cx = soma_x / total_area;
        let cy = soma_y / total_area;
        let mx = cx - mapa.largura as f32 / 2.0;
        let my = (mapa.altura as f32 / 2.0 - cy) * gf_mapa::mapa::ESCALA_Y_MAPA;
        tf.translation.x = mx;
        tf.translation.y = my;
        ortho.scale = 0.85;
        espectador.camera_centrada = true;
    }
}

fn faixa_modo_espectador(
    mut contextos: EguiContexts,
    op: Res<OpcoesCli>,
    relogio: Option<Res<RecursoRelogio>>,
    jogador: Option<Res<PaisJogador>>,
) {
    if !op.espectador_ativo() {
        return;
    }
    let Ok(ctx) = contextos.ctx_mut() else { return };
    egui::Area::new(egui::Id::new("faixa_modo_espectador"))
        .order(egui::Order::Foreground)
        .interactable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 76.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_black_alpha(205))
                .corner_radius(3.0)
                .inner_margin(egui::Margin::symmetric(12, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("MODO ESPECTADOR")
                                .size(12.5)
                                .strong()
                                .color(egui::Color32::from_rgb(255, 215, 0)),
                        );
                        ui.separator();
                        let tag = jogador.as_ref().map_or("BRA", |j| j.0.as_str());
                        ui.label(
                            egui::RichText::new(format!("País: {tag}"))
                                .size(12.0)
                                .color(egui::Color32::WHITE),
                        );
                        if let Some(r) = relogio {
                            ui.separator();
                            let d = r.data;
                            ui.label(
                                egui::RichText::new(format!("Ano {} ({:02}/{:02}/{})", d.ano, d.dia, d.mes, d.ano))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(200, 230, 255)),
                            );
                            ui.separator();
                            let vel_txt = if r.pausado {
                                "Pausado".to_string()
                            } else {
                                format!(
                                    "Velocidade: {}x",
                                    match r.velocidade {
                                        gf_mapa::tempo::VelocidadeJogo::V1 => 1,
                                        gf_mapa::tempo::VelocidadeJogo::V2 => 2,
                                        gf_mapa::tempo::VelocidadeJogo::V3 => 3,
                                        gf_mapa::tempo::VelocidadeJogo::V4 => 4,
                                        gf_mapa::tempo::VelocidadeJogo::V5 => 5,
                                    }
                                )
                            };
                            ui.label(
                                egui::RichText::new(vel_txt)
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(180, 255, 180)),
                            );
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new("[Esc] Encerrar com Relatório")
                                .size(11.0)
                                .italics()
                                .color(egui::Color32::from_gray(185)),
                        );
                    });
                });
        });
}

fn finito(v: f64) -> f64 {
    if v.is_finite() { v } else { 0.0 }
}

#[allow(clippy::too_many_arguments)]
fn capturar_snapshot(
    op: &OpcoesCli,
    relogio: &RecursoRelogio,
    contagem: &Contagem,
    tempo_real: f64,
    eco: Option<&EconomiaSim>,
    pol: Option<&PoliticaSim>,
    guerra: Option<&GuerraSim>,
    ledger: Option<&RecursoLedger>,
    mundo: Option<&RecursoMundo>,
    bandeiras: Option<&Bandeiras>,
) -> SnapshotDecada {
    let tag = op.pais_efetivo();
    let agora = tempo_real;
    let inicio = contagem.marco.unwrap_or(agora);
    let decorrido = (agora - inicio).max(0.001);

    let fps_medio = finito(if contagem.amostras > 0 {
        contagem.soma_fps / contagem.amostras as f64
    } else {
        contagem.quadros as f64 / decorrido
    });
    let fps_min = finito(contagem.fps_min.unwrap_or(fps_medio));
    let rss_mb = finito(ler_rss_mb());
    let dias_por_segundo = finito(relogio.total_ticks as f64 / decorrido);

    let d = relogio.data;
    let data_str = format!("{:02}/{:02}/{}", d.dia, d.mes, d.ano);

    // Economia & População
    let mut pop_pais = 0.0;
    let mut pib_pais = 0.0;
    let mut pib_pc_pais = 0.0;
    let mut desemp_pais = 0.0;
    let mut tesouro_pais = 0.0;
    let mut divida_pais = 0.0;
    let mut pop_mundo = 0.0;
    let mut pib_mundo = 0.0;
    let mut pib_pc_mundo = 0.0;
    let mut desemp_mundo = 0.0;
    let mut tesouro_mundo = 0.0;
    let mut divida_mundo = 0.0;

    if let Some(eco) = eco {
        if let Some(p) = eco.vista.ranking.iter().find(|p| p.tag == tag || p.id.to_uppercase() == tag) {
            pop_pais = finito(p.populacao);
            pib_pais = finito(p.pib_anual);
            pib_pc_pais = if pop_pais > 0.0 { finito(pib_pais / pop_pais) } else { 0.0 };
            desemp_pais = finito(p.desemprego * 100.0);
            tesouro_pais = finito(p.tesouro);
            divida_pais = finito(p.divida);
        } else if let Some(pais) = &eco.vista.pais {
            pop_pais = finito(eco.vista.pops.iter().map(|p| p.populacao).sum());
            pib_pais = finito(eco.vista.pib_anual);
            pib_pc_pais = if pop_pais > 0.0 { finito(pib_pais / pop_pais) } else { 0.0 };
            desemp_pais = finito(eco.vista.desemprego * 100.0);
            tesouro_pais = finito(pais.orcamento.tesouro);
            divida_pais = finito(pais.orcamento.divida);
        }

        pop_mundo = finito(eco.vista.ranking.iter().map(|p| finito(p.populacao)).sum());
        pib_mundo = finito(if eco.vista.pib_mundial_anual > 0.0 {
            eco.vista.pib_mundial_anual
        } else {
            eco.vista.ranking.iter().map(|p| finito(p.pib_anual)).sum()
        });
        pib_pc_mundo = if pop_mundo > 0.0 { finito(pib_mundo / pop_mundo) } else { 0.0 };
        tesouro_mundo = finito(eco.vista.ranking.iter().map(|p| finito(p.tesouro)).sum());
        divida_mundo = finito(eco.vista.ranking.iter().map(|p| finito(p.divida)).sum());
        if pop_mundo > 0.0 {
            let desemp_soma: f64 = eco.vista.ranking.iter().map(|p| finito(p.populacao) * finito(p.desemprego)).sum();
            desemp_mundo = finito(desemp_soma / pop_mundo * 100.0);
        }
    }

    // Política / Nome Oficial dinâmico e Governo legível
    let mut nome_pais = bandeiras
        .and_then(|b| b.variante(tag))
        .map(|v| v.nome)
        .or_else(|| mundo.and_then(|m| m.0.paises.get(tag)).map(|p| p.nome.clone()))
        .unwrap_or_else(|| tag.to_string());
    let mut forma_governo = "Governo Soberano".to_string();

    if let Some(pol) = pol {
        let nome_lei = |id: &str| -> String {
            pol.catalogo
                .leis
                .get(id)
                .map_or_else(|| id.replace('_', " "), |l| l.nome.clone())
        };
        let nome_partido_gov = pol.vista.pais.as_ref().and_then(|p| {
            p.ultima_eleicao
                .as_ref()
                .and_then(|e| e.partido_vencedor_id.as_ref())
                .and_then(|id| p.partidos.get(id))
                .map(|part| part.nome.clone())
        });

        if let Some(b) = pol.bandeiras.iter().find(|b| b.tag == tag) {
            let fg = nome_lei(&b.forma_governo);
            let partido_str = nome_partido_gov
                .or_else(|| {
                    b.partido
                        .as_deref()
                        .and_then(|arq| pol.catalogo.arquetipos_partido.get(arq))
                        .map(|a| a.nome.clone())
                })
                .or_else(|| b.partido.as_ref().map(|s| s.replace('_', " ")))
                .unwrap_or_else(|| "Governo em exercício".to_string());
            forma_governo = format!("{fg} ({partido_str})");
        } else if let Some(p) = &pol.vista.pais {
            if nome_pais == tag {
                nome_pais = p.nome.clone();
            }
            if let Some(fg) = p.leis_ativas.get("forma_governo") {
                forma_governo = match nome_partido_gov {
                    Some(part) => format!("{} ({part})", nome_lei(fg)),
                    None => nome_lei(fg),
                };
            }
        }
    }

    // Guerras
    let mut guerras = Vec::new();
    if let Some(guerra) = guerra {
        for w in guerra.0.guerras().iter().filter(|w| w.ativa) {
            let atk = w.atacantes.join(", ");
            let def = w.defensores.join(", ");
            guerras.push(format!(
                "{}: {} (atacantes) vs. {} (defensores) — Pontuação: {:.1} | Baixas: {} atk / {} def",
                w.nome_pt,
                atk,
                def,
                finito(w.pontuacao_guerra),
                w.baixas_atacantes,
                w.baixas_defensores
            ));
        }
    }
    let num_guerras = guerras.len();

    // Top 10
    let mut top10 = Vec::new();
    if let Some(ledger) = ledger && !ledger.rankings_cache.is_empty() {
        for e in ledger.rankings_cache.iter().take(10) {
            top10.push(EntradaTop10 {
                pos: e.rank_geral,
                nome: e.nome.clone(),
                tag: e.tag.clone(),
                grau: e.grau.rotulo().to_string(),
                pontuacao: finito(e.pontuacao_total),
                pib: finito(e.pib_anual),
                pop: finito(e.populacao),
                forca_mil: finito(e.forca_militar),
            });
        }
    } else if let Some(eco) = eco {
        let mut r = eco.vista.ranking.clone();
        r.sort_by(|a, b| finito(b.pib_anual).total_cmp(&finito(a.pib_anual)));
        for (i, p) in r.iter().take(10).enumerate() {
            top10.push(EntradaTop10 {
                pos: i + 1,
                nome: p.nome.clone(),
                tag: p.tag.clone(),
                grau: if i < 2 {
                    "Superpotência".into()
                } else if i < 8 {
                    "Grande Potência".into()
                } else {
                    "Potência Regional".into()
                },
                pontuacao: finito(p.pib_anual / 1e9),
                pib: finito(p.pib_anual),
                pop: finito(p.populacao),
                forca_mil: finito(p.forca_militar),
            });
        }
    }

    SnapshotDecada {
        data_str,
        ano: d.ano,
        ticks: relogio.total_ticks,
        nome_pais,
        forma_governo,
        pop_pais,
        pib_pais,
        pib_pc_pais,
        desemp_pais,
        tesouro_pais,
        divida_pais,
        pop_mundo,
        pib_mundo,
        pib_pc_mundo,
        desemp_mundo,
        tesouro_mundo,
        divida_mundo,
        num_guerras,
        guerras,
        top10,
        fps_medio,
        fps_min,
        rss_mb,
        dias_por_segundo,
    }
}

fn formatar_relatorio_md(op: &OpcoesCli, snapshots: &[SnapshotDecada], concluido: bool) -> String {
    let mut out = String::new();
    let tag = op.pais_efetivo();
    let anos_alvo = op.anos.unwrap_or(100);
    let ultimo = snapshots.last();

    out.push_str("# Relatório de Simulação — Modo Espectador\n\n");
    out.push_str("- **Jogo:** Hegemony: 1945\n");
    out.push_str(&format!("- **País Analisado:** {tag}\n"));
    out.push_str(&format!("- **Meta da Partida:** {anos_alvo} anos (1945 a {})\n", 1945 + anos_alvo));
    let status_str = if concluido { "Concluída com Sucesso" } else { "Em Andamento (Checkpoint)" };
    out.push_str(&format!("- **Status:** {status_str}\n"));
    if let Some(u) = ultimo {
        out.push_str(&format!("- **Data Atual:** {}\n", u.data_str));
        out.push_str(&format!("- **Dias Simulados:** {}\n", u.ticks));
    }
    out.push_str("\n---\n\n");

    if let Some(u) = ultimo {
        out.push_str("## 1. Métricas de Desempenho e Recursos\n\n");
        out.push_str(&format!("- **FPS Médio:** {:.1}\n", u.fps_medio));
        out.push_str(&format!("- **FPS Mínimo:** {:.1}\n", u.fps_min));
        out.push_str(&format!("- **Memória Ocupada (RSS):** {:.1} MB\n", u.rss_mb));
        out.push_str(&format!("- **Velocidade de Simulação:** {:.1} dias/s\n\n", u.dias_por_segundo));

        out.push_str(&format!("## 2. Comparativo: {} vs. Mundo\n\n", u.nome_pais));
        out.push_str("| Indicador | País do Jogador | Mundo Total |\n");
        out.push_str("| :--- | :--- | :--- |\n");
        out.push_str(&format!("| **Nome Oficial** | {} | — |\n", u.nome_pais));
        out.push_str(&format!("| **Governo / Regime** | {} | — |\n", u.forma_governo));
        out.push_str(&format!("| **População** | {} | {} |\n", fmt_pop(u.pop_pais.max(0.0) as u64), fmt_pop(u.pop_mundo.max(0.0) as u64)));
        out.push_str(&format!("| **PIB Anual** | {} | {} |\n", fmt_din(u.pib_pais), fmt_din(u.pib_mundo)));
        out.push_str(&format!("| **PIB per capita** | US$ {:.0} | US$ {:.0} |\n", u.pib_pc_pais, u.pib_pc_mundo));
        out.push_str(&format!("| **Taxa de Desemprego** | {:.1}% | {:.1}% |\n", u.desemp_pais, u.desemp_mundo));
        out.push_str(&format!("| **Tesouro Nacional** | {} | {} |\n", fmt_din(u.tesouro_pais), fmt_din(u.tesouro_mundo)));
        out.push_str(&format!("| **Dívida Pública** | {} | {} |\n", fmt_din(u.divida_pais), fmt_din(u.divida_mundo)));
        out.push_str("\n");

        out.push_str("## 3. Conflitos Armados Ativos (Guerras)\n\n");
        if u.guerras.is_empty() {
            out.push_str("*Nenhum conflito armado ativo neste momento (paz global).*\n\n");
        } else {
            for g in &u.guerras {
                out.push_str(&format!("- {g}\n"));
            }
            out.push_str("\n");
        }

        out.push_str("## 4. Top 10 Mundial (Ranking de Potências)\n\n");
        out.push_str("| # | País | Tag | Grau | Pontuação | PIB Anual | População | Força Militar |\n");
        out.push_str("| -: | :--- | :---: | :--- | -: | -: | -: | -: |\n");
        for e in &u.top10 {
            out.push_str(&format!(
                "| {} | {} | {} | {} | {:.1} | {} | {} | {:.0} |\n",
                e.pos, e.nome, e.tag, e.grau, e.pontuacao, fmt_din(e.pib), fmt_pop(e.pop.max(0.0) as u64), e.forca_mil
            ));
        }
        out.push_str("\n");
    }

    out.push_str("## 5. Histórico Decenal da Simulação\n\n");
    out.push_str("| Ano | Data | Pop Jogador | PIB Jogador | PIB pc | Desemp | Pop Mundo | PIB Mundo | Guerras | RSS (MB) | FPS Médio |\n");
    out.push_str("| -: | :---: | -: | -: | -: | -: | -: | -: | -: | -: | -: |\n");
    for s in snapshots {
        out.push_str(&format!(
            "| {} | {} | {} | {} | US$ {:.0} | {:.1}% | {} | {} | {} | {:.1} | {:.1} |\n",
            s.ano, s.data_str, fmt_pop(s.pop_pais.max(0.0) as u64), fmt_din(s.pib_pais), s.pib_pc_pais, s.desemp_pais,
            fmt_pop(s.pop_mundo.max(0.0) as u64), fmt_din(s.pib_mundo), s.num_guerras, s.rss_mb, s.fps_medio
        ));
    }
    out.push_str("\n");

    out
}

fn salvar_relatorio_arquivo(caminho: &Path, conteudo: &str) {
    if let Some(pai) = caminho.parent() {
        let _ = std::fs::create_dir_all(pai);
    }
    if let Err(e) = std::fs::write(caminho, conteudo) {
        warn!("espectador: falha ao gravar relatório em {}: {e}", caminho.display());
    }
}

#[allow(clippy::too_many_arguments)]
fn vigiar_espectador(
    op: Res<OpcoesCli>,
    tempo: Res<Time<Real>>,
    teclas: Res<ButtonInput<KeyCode>>,
    relogio: Res<RecursoRelogio>,
    contagem: Res<Contagem>,
    mut espectador: ResMut<EstadoEspectador>,
    eco: Option<Res<EconomiaSim>>,
    pol: Option<Res<PoliticaSim>>,
    guerra: Option<Res<GuerraSim>>,
    ledger: Option<Res<RecursoLedger>>,
    mundo: Option<Res<RecursoMundo>>,
    bandeiras: Option<Res<Bandeiras>>,
    mut saida: MessageWriter<AppExit>,
) {
    if !op.espectador_ativo() || espectador.encerrado {
        return;
    }

    let anos_alvo = op.anos.unwrap_or(100);
    let ano_final = 1945 + anos_alvo as i32;
    let ano_atual = relogio.data.ano;
    let encerramento_pedido = teclas.just_pressed(KeyCode::Escape) || ano_atual >= ano_final;

    // Snapshot inicial: só depois que a economia carregou e a partida começou a correr.
    let dados_prontos = eco.as_ref().is_some_and(|e| !e.vista.ranking.is_empty());
    if espectador.snapshots.is_empty() && dados_prontos {
        let snap = capturar_snapshot(
            &op,
            &relogio,
            &contagem,
            tempo.elapsed_secs_f64(),
            eco.as_deref(),
            pol.as_deref(),
            guerra.as_deref(),
            ledger.as_deref(),
            mundo.as_deref(),
            bandeiras.as_deref(),
        );
        espectador.snapshots.push(snap);
        if !encerramento_pedido
            && let Some(caminho) = &op.relatorio
        {
            let md = formatar_relatorio_md(&op, &espectador.snapshots, false);
            salvar_relatorio_arquivo(caminho, &md);
        }
    }

    // Checkpoints decenais (a cada 10 anos de jogo: 1955, 1965, ..., 2045).
    let mut gravou_decada_neste_quadro = false;
    while ano_atual >= espectador.ultimo_ano_relatorio + 10 {
        espectador.ultimo_ano_relatorio += 10;
        let snap = capturar_snapshot(
            &op,
            &relogio,
            &contagem,
            tempo.elapsed_secs_f64(),
            eco.as_deref(),
            pol.as_deref(),
            guerra.as_deref(),
            ledger.as_deref(),
            mundo.as_deref(),
            bandeiras.as_deref(),
        );
        espectador.snapshots.push(snap);
        gravou_decada_neste_quadro = true;
    }
    if gravou_decada_neste_quadro
        && !encerramento_pedido
        && let Some(caminho) = &op.relatorio
    {
        let md = formatar_relatorio_md(&op, &espectador.snapshots, false);
        salvar_relatorio_arquivo(caminho, &md);
    }

    if encerramento_pedido {
        espectador.encerrado = true;
        if !gravou_decada_neste_quadro {
            let snap = capturar_snapshot(
                &op,
                &relogio,
                &contagem,
                tempo.elapsed_secs_f64(),
                eco.as_deref(),
                pol.as_deref(),
                guerra.as_deref(),
                ledger.as_deref(),
                mundo.as_deref(),
                bandeiras.as_deref(),
            );
            espectador.snapshots.push(snap);
        }
        if let Some(caminho) = &op.relatorio {
            let md = formatar_relatorio_md(&op, &espectador.snapshots, true);
            salvar_relatorio_arquivo(caminho, &md);
        }
        println!("ESPECTADOR OK");
        saida.write(AppExit::Success);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn args(lista: &[&str]) -> Vec<String> {
        lista.iter().map(|a| a.to_string()).collect()
    }

    #[test]
    fn sem_bandeiras_nao_ativa_smoke() {
        let op = analisar_args(args(&["guerra_fria"])).unwrap();
        assert_eq!(op, OpcoesCli::default());
        assert!(!op.smoke_ativo());
    }

    #[test]
    fn smoke_completo() {
        let op = analisar_args(args(&[
            "guerra_fria",
            "--smoke",
            "45",
            "--screenshot",
            "/tmp/fim.png",
            "--qualidade",
            "ultra",
            "--painel",
            "7",
        ]))
        .unwrap();
        assert_eq!(op.segundos, Some(45.0));
        assert_eq!(op.captura, Some(PathBuf::from("/tmp/fim.png")));
        assert_eq!(op.qualidade, Some(QualidadeGrafica::Ultra));
        assert_eq!(op.painel, Some(7));
    }

    #[test]
    fn qualidade_pode_ser_usada_sem_smoke() {
        let op = analisar_args(args(&["--qualidade", "baixo"])).unwrap();
        assert_eq!(op.qualidade, Some(QualidadeGrafica::Baixo));
        assert!(!op.smoke_ativo());
    }

    #[test]
    fn aceita_decimal_e_caixa_alta() {
        let op = analisar_args(args(&["--smoke", "2.5", "--qualidade", "MÉDIO"])).unwrap();
        assert_eq!(op.segundos, Some(2.5));
        assert_eq!(op.qualidade, Some(QualidadeGrafica::Medio));
    }

    #[test]
    fn painel_e_captura_exigem_smoke() {
        assert!(analisar_args(args(&["--painel", "3"])).is_err());
        assert!(analisar_args(args(&["--screenshot", "a.png"])).is_err());
        assert!(analisar_args(args(&["--camera", "10", "20"])).is_err());
        assert!(analisar_args(args(&["--zoom", "0.5"])).is_err());
        assert!(analisar_args(args(&["--tutorial"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--painel", "3"])).is_ok());
        assert!(analisar_args(args(&["--smoke", "10", "--tutorial"])).is_ok());
    }

    #[test]
    fn tutorial_smoke_valido() {
        let op = analisar_args(args(&["--smoke", "30", "--tutorial"])).unwrap();
        assert!(op.tutorial);
    }

    #[test]
    fn camera_e_zoom_validos() {
        let op = analisar_args(args(&[
            "--smoke", "10",
            "--camera", "-77.04", "38.91",
            "--zoom", "0.15",
        ]))
        .unwrap();
        assert_eq!(op.camera, Some((-77.04, 38.91)));
        assert_eq!(op.zoom, Some(0.15));
    }

    #[test]
    fn rejeita_valores_fora_do_intervalo() {
        assert!(analisar_args(args(&["--smoke", "0"])).is_err());
        assert!(analisar_args(args(&["--smoke", "-3"])).is_err());
        assert!(analisar_args(args(&["--smoke", "abc"])).is_err());
        assert!(analisar_args(args(&["--smoke"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--painel", "0"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--painel", "10"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--painel", "x"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--screenshot", "foto.jpg"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--camera", "200", "0"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--camera", "0", "100"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--camera", "abc", "0"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--camera", "0"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--zoom", "0"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--zoom", "-1"])).is_err());
        assert!(analisar_args(args(&["--smoke", "10", "--zoom", "xyz"])).is_err());
    }

    #[test]
    fn rejeita_qualidade_invalida_e_bandeira_desconhecida() {
        assert!(analisar_args(args(&["--qualidade", "maxima"])).is_err());
        assert!(analisar_args(args(&["--qualidade"])).is_err());
        assert!(analisar_args(args(&["--smok", "10"])).is_err());
    }

    #[test]
    fn espectador_padrao_e_opcoes() {
        let op = analisar_args(args(&["--espectador"])).unwrap();
        assert!(op.espectador_ativo());
        assert_eq!(op.anos, Some(100));
        assert_eq!(op.pais_efetivo(), "BRA");

        let op_completo = analisar_args(args(&[
            "guerra_fria",
            "--espectador",
            "--pais",
            "BRA",
            "--anos",
            "50",
            "--relatorio",
            "relatorio.md",
            "--qualidade",
            "medio",
        ]))
        .unwrap();
        assert!(op_completo.espectador_ativo());
        assert_eq!(op_completo.pais, Some("BRA".into()));
        assert_eq!(op_completo.pais_efetivo(), "BRA");
        assert_eq!(op_completo.anos, Some(50));
        assert_eq!(op_completo.relatorio, Some(PathBuf::from("relatorio.md")));
        assert_eq!(op_completo.qualidade, Some(QualidadeGrafica::Medio));
    }

    #[test]
    fn smoke_com_pais_customizado() {
        let op = analisar_args(args(&["--smoke", "30", "--pais", "sov"])).unwrap();
        assert!(op.smoke_ativo());
        assert_eq!(op.pais, Some("SOV".into()));
        assert_eq!(op.pais_efetivo(), "SOV");

        let op_padrao = analisar_args(args(&["--smoke", "15"])).unwrap();
        assert_eq!(op_padrao.pais_efetivo(), "USA");

        let op_alias = analisar_args(args(&["--espectador", "--pais", "eua"])).unwrap();
        assert_eq!(op_alias.pais_efetivo(), "USA");
        let op_urss = analisar_args(args(&["--smoke", "10", "--pais", "urss"])).unwrap();
        assert_eq!(op_urss.pais_efetivo(), "SOV");
    }

    #[test]
    fn validacoes_modo_espectador() {
        assert!(analisar_args(args(&["--anos", "10"])).is_err());
        assert!(analisar_args(args(&["--relatorio", "out.md"])).is_err());
        assert!(analisar_args(args(&["--espectador", "--anos", "0"])).is_err());
        assert!(analisar_args(args(&["--espectador", "--anos", "abc"])).is_err());
        assert!(analisar_args(args(&["--espectador", "--pais"])).is_err());
        assert!(analisar_args(args(&["--espectador", "--relatorio"])).is_err());
    }

    #[test]
    fn formatacao_relatorio_markdown_completa() {
        let op = analisar_args(args(&[
            "guerra_fria",
            "--espectador",
            "--pais",
            "BRA",
            "--anos",
            "100",
        ]))
        .unwrap();
        let snap = SnapshotDecada {
            data_str: "01/01/2045".into(),
            ano: 2045,
            ticks: 36525,
            nome_pais: "República Federativa do Brasil".into(),
            forma_governo: "República Presidencialista (Trabalhista)".into(),
            pop_pais: 215_000_000.0,
            pib_pais: 3_200_000_000_000.0,
            pib_pc_pais: 14_883.0,
            desemp_pais: 5.4,
            tesouro_pais: 120_000_000_000.0,
            divida_pais: 450_000_000_000.0,
            pop_mundo: 9_100_000_000.0,
            pib_mundo: 120_000_000_000_000.0,
            pib_pc_mundo: 13_186.0,
            desemp_mundo: 6.1,
            tesouro_mundo: 4_500_000_000_000.0,
            divida_mundo: 18_000_000_000_000.0,
            num_guerras: 0,
            guerras: vec![],
            top10: vec![EntradaTop10 {
                pos: 1,
                nome: "Estados Unidos".into(),
                tag: "USA".into(),
                grau: "Superpotência".into(),
                pontuacao: 980.0,
                pib: 28_000_000_000_000.0,
                pop: 380_000_000.0,
                forca_mil: 1200.0,
            }],
            fps_medio: 60.0,
            fps_min: 48.5,
            rss_mb: 812.4,
            dias_por_segundo: 29.8,
        };
        let md = formatar_relatorio_md(&op, &[snap], true);
        assert!(md.contains("Concluída com Sucesso"));
        assert!(md.contains("República Federativa do Brasil"));
        assert!(md.contains("República Presidencialista"));
        assert!(md.contains("PIB Anual"));
        assert!(md.contains("PIB per capita"));
        assert!(md.contains("Taxa de Desemprego"));
        assert!(md.contains("Tesouro Nacional"));
        assert!(md.contains("Dívida Pública"));
        assert!(md.contains("Top 10 Mundial"));
        assert!(md.contains("FPS Médio"));
        assert!(md.contains("Memória Ocupada (RSS)"));
        assert!(md.contains("dias/s"));
        assert!(!md.contains("NaN"));
    }

    #[test]
    fn simulacao_100_anos_ate_2045_estavel_pos_1991() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (cat_prov, mut mundo) = simulacao::mundo::MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenário 1945");
        let cat_ev = eventos::CatalogoEventos::carregar(&raiz.join("assets/data/eventos")).expect("eventos");
        let mut est_ev = eventos::EstadoEventos::default();
        let mut formacoes = simulacao::formacao::Formacoes::carregar(raiz.join("data/formaveis"), &cat_prov).expect("formações");
        let info_diplo: Vec<sim_diplomacia::InfoPais> = mundo
            .paises
            .values()
            .map(|p| sim_diplomacia::InfoPais {
                tag: p.tag.clone(),
                bloco: sim_diplomacia::Bloco::NaoAlinhado,
            })
            .collect();
        let mut diplo = sim_diplomacia::Diplomacia::novo(info_diplo);
        let mut tec = TecnologiaSim::carregar(&raiz).expect("tecnologia");
        let ent_tec = sim_tecnologia::EntradaPais {
            niveis_universidade: 4.0,
            alfabetizacao: 0.75,
            ..Default::default()
        };
        let mut elenco = personagens::Elenco::carregar(&raiz.join("assets/data/personagens"), (1945, 1, 1)).expect("personagens");

        let mut eventos_resolvidos = 0usize;
        while mundo.data.ano < 2045 {
            mundo.data.avancar_dia();
            let hoje = (mundo.data.ano, mundo.data.mes, mundo.data.dia);

            // Eventos históricos resolvidos automaticamente pela IA (sem fila pendente).
            eventos::avaliar_dia(&cat_ev, &mut est_ev, &mut mundo, "BRA");
            let pend = std::mem::take(&mut est_ev.pendentes);
            for p in pend {
                if let Some(ev) = cat_ev.por_id(&p.evento_id) {
                    let k = eventos::escolha_ia(ev, hoje);
                    eventos::aplicar_opcao(&mut mundo, &mut est_ev, ev, k, &p.pais);
                    eventos_resolvidos += 1;
                }
            }
            assert!(est_ev.pendentes.is_empty());

            // Formações / descolonização até e após 1991.
            let novos = formacoes.processar(&mut mundo);
            for n in novos {
                diplo.adicionar_pais(sim_diplomacia::InfoPais {
                    tag: n.tag,
                    bloco: sim_diplomacia::Bloco::NaoAlinhado,
                });
            }

            // Diplomacia global + IA diplomática do Brasil.
            diplo.tick_dia(hoje.0, hoje.1, hoje.2, None);
            if hoje.2 == 1 {
                let t = diplo.tesouro_ia.entry("BRA".into()).or_insert(500.0);
                *t = (*t + 60.0).min(1500.0);
            }
            if hoje.2 == 15 {
                super::super::diplomacia_ui::ia_diplomatica_jogador(&mut diplo, "BRA", hoje, None, None);
            }

            // Tecnologia do Brasil (IA escolhe e avança mesmo após esgotar árvore de 1991).
            tec.mundo.escolher_ia("BRA", hoje.0, &ent_tec);
            let _ = tec.mundo.tick_pais("BRA", hoje.0, &ent_tec);

            // Personagens e sucessão até 2045.
            let _ = elenco.avancar_dia(hoje);
        }

        assert_eq!(mundo.data.ano, 2045);
        assert_eq!(mundo.data.mes, 1);
        assert_eq!(mundo.data.dia, 1);
        assert!(mundo.tensao_mundial.is_finite());
        assert!(diplo.tensao.is_finite());
        assert!(eventos_resolvidos >= 10, "esperava eventos resolvidos pela IA: {eventos_resolvidos}");
        assert!(diplo.pactos_de("BRA").next().is_some(), "IA do Brasil deve firmar tratados em 100 anos");
        let concluidas_bra = tec.mundo.paises.get("BRA").map_or(0, |p| p.conhecidas.len());
        assert!(concluidas_bra >= 20, "IA do Brasil deve pesquisar tecnologias: {concluidas_bra}");
        let lider_bra = elenco.lider("BRA", personagens::Papel::Estado).expect("líder BRA em 2045");
        assert!(elenco.vivo(&lider_bra.id));
    }
}
