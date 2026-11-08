//! Modo smoke: o jogo roda sozinho, para testes automatizados sem humano.
//!
//! Nada aqui é registrado quando `--smoke` está ausente, então o jogo normal não
//! executa nenhum destes sistemas.
use std::path::PathBuf;

use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};

use super::comercio_ui::PainelComercio;
use super::config::QualidadeGrafica;
use super::construcao_ui::ConstrucaoUi;
use super::diplomacia_ui::{DiploSim, PainelDiplomacia};
use super::economy::RecursoMundo;
use super::enciclopedia::EnciclopediaUi;
use super::estatisticas::RecursoLedger;
use super::eventos_ui::EstadoEv;
use super::guerra_sim::PainelGuerra;
use super::hud_mapas::ModoTematico;
use super::jogador::{Aba, PaisJogador, PainelPais};
use super::map::{ProvinciaSelecionada, RecursoCatalogo};
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
}

impl OpcoesCli {
    pub fn smoke_ativo(&self) -> bool {
        self.segundos.is_some()
    }
}

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
    Ok(op)
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

pub struct SmokePlugin(pub OpcoesCli);

impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }
        app.insert_resource(self.0.clone())
            .init_resource::<Contagem>()
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
                ),
            );
    }
}

fn iniciar_partida(
    op: Res<OpcoesCli>,
    mut commands: Commands,
    mut proximo: ResMut<NextState<AppState>>,
    mut tutorial: ResMut<TutorialGuiado>,
) {
    if op.tutorial {
        tutorial.iniciar();
    } else {
        tutorial.ativo = false;
        tutorial.pendente = false;
    }
    commands.insert_resource(PaisJogador(PAIS_SMOKE.to_string()));
    proximo.set(AppState::Jogando);
}

fn lancar_relogio(op: Res<OpcoesCli>, mut relogio: ResMut<RecursoRelogio>) {
    if op.tutorial {
        return;
    }
    relogio.definir_velocidade(VelocidadeJogo::V5);
    relogio.pausado = false;
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
fn manter_relogio_no_maximo(op: Res<OpcoesCli>, mut relogio: ResMut<RecursoRelogio>) {
    if op.tutorial {
        return;
    }
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
}
