//! Flipbook pré-renderizado da abertura, mantido fora da GPU até cada quadro ser usado.
use super::audio::{CanalAudio, VolumesAudio};
use super::map::RaizProjeto;
use crate::estado::AppState;
use bevy::audio::{AudioSink, Volume};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

pub const DURACAO_MINIMA: f32 = 8.0;
const FPS: f32 = 24.0;
const QUADROS: usize = 240;
const LOOP_INICIO: usize = 204;

#[derive(Resource)]
pub struct CarregamentoPronto(pub bool);

#[derive(Resource)]
struct EstadoAbertura {
    inicio: f32,
    receptor: Option<Mutex<Receiver<(usize, Vec<u8>)>>>,
    imagem: Handle<Image>,
    indice: usize,
    pronto: bool,
    audio_iniciado: bool,
}

#[derive(Component)]
struct SpriteAbertura;

#[derive(Component)]
struct AudioAbertura;

pub struct AberturaPlugin;

impl Plugin for AberturaPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CarregamentoPronto(false))
            // O OnEnter do estado inicial roda antes do Startup: preparar e iniciar juntos,
            // encadeados, para o EstadoAbertura já existir quando iniciar_abertura rodar.
            .add_systems(OnEnter(AppState::Carregando), (preparar_abertura, iniciar_abertura).chain())
            .add_systems(OnExit(AppState::Carregando), encerrar_abertura)
            .add_systems(Update, (atualizar_abertura, ajustar_sprite, controlar_audio_abertura).chain().run_if(in_state(AppState::Carregando)))
            .add_systems(EguiPrimaryContextPass, desenhar_abertura.run_if(in_state(AppState::Carregando)));
    }
}

fn preparar_abertura(mut commands: Commands, mut imagens: ResMut<Assets<Image>>) {
    let imagem = imagens.add(Image::default());
    commands.insert_resource(EstadoAbertura {
        inicio: 0.0,
        receptor: None,
        imagem,
        indice: 0,
        pronto: false,
        audio_iniciado: true,
    });
}

fn iniciar_abertura(
    mut estado: ResMut<EstadoAbertura>,
    raiz: Res<RaizProjeto>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    estado.inicio = 0.0;
    estado.indice = 0;
    estado.pronto = false;
    estado.audio_iniciado = true;
    let (tx, rx) = mpsc::sync_channel(4);
    let pasta = raiz.0.join("assets/abertura/quadros");
    thread::spawn(move || {
        for indice in 0..QUADROS {
            let caminho = pasta.join(format!("{indice:04}.jpg"));
            let Ok(bytes) = std::fs::read(&caminho) else {
                warn!("abertura: não foi possível ler {}", caminho.display());
                break;
            };
            let Ok(img) = image::load_from_memory(&bytes) else {
                warn!("abertura: quadro inválido {}", caminho.display());
                break;
            };
            if tx.send((indice, img.to_rgba8().into_raw())).is_err() {
                break;
            }
        }
    });
    estado.receptor = Some(Mutex::new(rx));
    commands.spawn((
        Sprite::from_image(estado.imagem.clone()),
        SpriteAbertura,
        Transform::from_xyz(0.0, 0.0, 100.0),
    ));
    commands.spawn((
        AudioPlayer::new(asset_server.load("abertura/abertura.ogg")),
        PlaybackSettings::DESPAWN,
        AudioAbertura,
    ));
}

fn atualizar_abertura(
    time: Res<Time>,
    mut estado: ResMut<EstadoAbertura>,
    mut imagens: ResMut<Assets<Image>>,
    pronto: Res<CarregamentoPronto>,
    mut proximo: ResMut<NextState<AppState>>,
    teclas: Res<ButtonInput<KeyCode>>,
    botoes: Res<ButtonInput<MouseButton>>,
    opcoes: Option<Res<super::smoke::OpcoesCli>>,
) {
    estado.inicio += time.delta_secs();
    if opcoes.as_ref().is_some_and(|o| o.ativo()) && pronto.0 {
        proximo.set(AppState::Menu);
        return;
    }
    // Depois que o decodificador termina, o receptor fica None: não pode haver `return` aqui,
    // senão a regra de saída abaixo nunca mais roda e a tela trava no último quadro.
    if let Some(receptor) = estado.receptor.take() {
    let mut desconectado = false;
    if let Ok(receptor_guard) = receptor.lock() {
    loop {
        match receptor_guard.try_recv() {
            Ok((indice, pixels)) => {
                if indice == 0 || indice <= estado.indice + 1 {
                    if let Some(mut img) = imagens.get_mut(&estado.imagem) {
                        *img = Image::new(
                            bevy::render::render_resource::Extent3d {
                                width: 1280,
                                height: 720,
                                depth_or_array_layers: 1,
                            },
                            bevy::render::render_resource::TextureDimension::D2,
                            pixels,
                            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                            bevy::asset::RenderAssetUsages::default(),
                        );
                        estado.indice = indice;
                        estado.pronto = true;
                    }
                }
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                desconectado = true;
                break;
            }
        }
    }
    }
    if !desconectado {
        estado.receptor = Some(receptor);
    }
    }
    // A saída não depende do vídeo: se os quadros falharem ao carregar, a regra dos 8 s + jogo
    // carregado ainda leva ao menu (senão a tela de carregamento ficaria presa para sempre).
    if estado.pronto {
        let indice = indice_loop(estado.inicio);
        if indice > estado.indice && indice < QUADROS {
            estado.indice = indice;
        }
    }
    let pular = botoes.get_just_pressed().next().is_some();
    if regra_saida_abertura(estado.inicio, pronto.0)
        && (teclas.just_pressed(KeyCode::Escape) || teclas.just_pressed(KeyCode::Space) || pular)
    {
        proximo.set(AppState::Menu);
    } else if regra_saida_abertura(estado.inicio, pronto.0) {
        proximo.set(AppState::Menu);
    }
}

fn ajustar_sprite(
    janela: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut sprites: Query<&mut Transform, With<SpriteAbertura>>,
) {
    let Ok(janela) = janela.single() else { return };
    let escala = (janela.width() / 1280.0).min(janela.height() / 720.0);
    for mut transform in &mut sprites {
        transform.scale = Vec3::splat(escala.max(0.01));
    }
}

fn controlar_audio_abertura(
    estado: Res<EstadoAbertura>,
    vols: Res<VolumesAudio>,
    mut audio: Query<&mut AudioSink, With<AudioAbertura>>,
) {
    if !estado.audio_iniciado {
        return;
    }
    for mut sink in &mut audio {
        sink.set_volume(Volume::Linear(vols.ganho(CanalAudio::Musica)));
    }
}

fn desenhar_abertura(
    mut contextos: EguiContexts,
    carregado: Res<CarregamentoPronto>,
) -> Result {
    let ctx = contextos.ctx_mut()?;
    if !carregado.0 {
        egui::Area::new("abertura_carregando".into())
            .anchor(egui::Align2::RIGHT_BOTTOM, [-18.0, -14.0])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Carregando…").size(13.0).color(egui::Color32::from_white_alpha(190)));
            });
    }
    ctx.request_repaint_after(Duration::from_secs_f32(1.0 / FPS));
    Ok(())
}

pub fn regra_saida_abertura(segundos: f32, jogo_carregado: bool) -> bool {
    jogo_carregado && segundos >= DURACAO_MINIMA
}

pub fn indice_loop(segundos: f32) -> usize {
    let quadro = (segundos * FPS) as usize;
    if quadro < QUADROS {
        quadro
    } else {
        LOOP_INICIO + (quadro - QUADROS) % (QUADROS - LOOP_INICIO)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn so_sai_depois_de_oito_segundos_e_carga() {
        assert!(!regra_saida_abertura(7.99, true));
        assert!(!regra_saida_abertura(8.0, false));
        assert!(regra_saida_abertura(8.0, true));
    }

    #[test]
    fn loop_retorna_ao_trecho_final() {
        assert_eq!(indice_loop(10.0), LOOP_INICIO);
        assert_eq!(indice_loop(10.1), LOOP_INICIO + 2);
    }
}

/// Ao sair do carregamento, remove o vídeo (sprite por cima do mapa) e o áudio da abertura e
/// solta a textura e o decodificador; sem isso o pôster ficava cobrindo o mapa na partida.
fn encerrar_abertura(
    mut commands: Commands,
    entidades: Query<Entity, Or<(With<SpriteAbertura>, With<AudioAbertura>)>>,
    mut estado: Option<ResMut<EstadoAbertura>>,
    mut imagens: ResMut<Assets<Image>>,
) {
    for entidade in &entidades {
        commands.entity(entidade).despawn();
    }
    if let Some(estado) = estado.as_mut() {
        estado.receptor = None;
        imagens.remove(&estado.imagem);
    }
}
