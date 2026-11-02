//! Modo smoke: o jogo roda sozinho, para testes automatizados sem humano.
//!
//! Nada aqui é registrado quando `--smoke` está ausente, então o jogo normal não
//! executa nenhum destes sistemas.
use std::path::PathBuf;

use bevy::diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{save_to_disk, Screenshot};

use super::comercio_ui::PainelComercio;
use super::config::QualidadeGrafica;
use super::diplomacia_ui::PainelDiplomacia;
use super::enciclopedia::EnciclopediaUi;
use super::estatisticas::RecursoLedger;
use super::guerra_sim::PainelGuerra;
use super::jogador::{PaisJogador, PainelPais};
use super::mercado_ui::PainelMercado;
use super::menu_pausa::{AbaConfig, JanelaConfig};
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
            .add_systems(OnEnter(AppState::Jogando), lancar_relogio)
            .add_systems(
                Update,
                (
                    manter_relogio_no_maximo.run_if(in_state(AppState::Jogando)),
                    reabrir_painel.run_if(in_state(AppState::Jogando)),
                    amostrar_fps.run_if(crate::estado::em_partida),
                    vigiar_prazo,
                ),
            );
    }
}

fn iniciar_partida(
    mut commands: Commands,
    mut proximo: ResMut<NextState<AppState>>,
    mut tutorial: ResMut<TutorialGuiado>,
) {
    tutorial.ativo = false;
    tutorial.pendente = false;
    commands.insert_resource(PaisJogador(PAIS_SMOKE.to_string()));
    proximo.set(AppState::Jogando);
}

fn lancar_relogio(mut relogio: ResMut<RecursoRelogio>) {
    relogio.definir_velocidade(VelocidadeJogo::V5);
    relogio.pausado = false;
}

/// Eventos e formações pausam o tempo; a partida de smoke precisa dele correndo.
fn manter_relogio_no_maximo(mut relogio: ResMut<RecursoRelogio>) {
    if relogio.pausado || relogio.velocidade != VelocidadeJogo::V5 {
        relogio.definir_velocidade(VelocidadeJogo::V5);
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

/// Abre a partida, pede a captura e encerra no prazo, imprimindo o resultado.
fn vigiar_prazo(
    op: Res<OpcoesCli>,
    tempo: Res<Time<Real>>,
    jogador: Option<Res<PaisJogador>>,
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
        assert!(analisar_args(args(&["--smoke", "10", "--painel", "3"])).is_ok());
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
    }

    #[test]
    fn rejeita_qualidade_invalida_e_bandeira_desconhecida() {
        assert!(analisar_args(args(&["--qualidade", "maxima"])).is_err());
        assert!(analisar_args(args(&["--qualidade"])).is_err());
        assert!(analisar_args(args(&["--smok", "10"])).is_err());
    }
}
