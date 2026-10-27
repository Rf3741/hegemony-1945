//! Plugin do mapa: textura política (cor por país + bordas), câmera e seleção de província.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]
use super::economy::RecursoMundo;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use bevy_egui::input::EguiWantsInput;
use simulacao::mapa::{CatalogoProvincias, id_de_cor_provincia};
use std::path::PathBuf;

#[derive(Resource)]
pub struct RaizProjeto(pub PathBuf);

#[derive(Resource)]
pub struct RecursoCatalogo(pub CatalogoProvincias);

/// Dados do mapa em memória: id de província por pixel e a textura exibida.
#[derive(Resource)]
pub struct DadosMapa {
    pub largura: u32,
    pub altura: u32,
    pub ids: Vec<u32>,
    pub textura: Handle<Image>,
}

/// Província atualmente selecionada (0 = nenhuma).
#[derive(Resource, Default)]
pub struct ProvinciaSelecionada(pub u32);

#[derive(Resource, Default)]
struct ArrastoCamera {
    inicio: Option<Vec2>,
    moveu: f32,
}

const COR_OCEANO_RGBA: [u8; 4] = [28, 58, 96, 255];

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProvinciaSelecionada>()
            .init_resource::<ArrastoCamera>()
            .add_systems(Startup, carregar_mapa)
            .add_systems(Update, (mover_camera, selecionar_provincia));
    }
}

fn carregar_mapa(
    mut commands: Commands,
    raiz: Res<RaizProjeto>,
    catalogo: Res<RecursoCatalogo>,
    mundo: Res<RecursoMundo>,
    mut imagens: ResMut<Assets<Image>>,
) {
    let img = image::open(raiz.0.join("assets/map/provinces.png"))
        .expect("falha ao abrir provinces.png")
        .to_rgb8();
    let (w, h) = img.dimensions();

    let ids: Vec<u32> = img
        .pixels()
        .map(|p| id_de_cor_provincia((p[0], p[1], p[2])).unwrap_or(0))
        .collect();

    // Cor de cada província conforme o dono em 1945.
    let mut cor_prov = vec![COR_OCEANO_RGBA; catalogo.0.provincias.iter().map(|p| p.id).max().unwrap_or(0) as usize + 1];
    for prov in &catalogo.0.provincias {
        if let Some(tag) = mundo.dono_por_provincia.get(&prov.id)
            && let Some(pais) = mundo.paises.get(tag)
        {
            cor_prov[prov.id as usize] = [pais.cor.0, pais.cor.1, pais.cor.2, 255];
        }
    }

    let dono_idx = |id: u32| -> &str {
        mundo.dono_por_provincia.get(&id).map(String::as_str).unwrap_or("")
    };

    let (wu, hu) = (w as usize, h as usize);
    let mut dados = vec![0u8; wu * hu * 4];
    for y in 0..hu {
        for x in 0..wu {
            let i = y * wu + x;
            let id = ids[i];
            let mut c = if id == 0 { COR_OCEANO_RGBA } else { cor_prov[id as usize] };
            if id != 0 {
                let mut borda_pais = false;
                let mut borda_prov = false;
                for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                    if nx < wu && ny < hu {
                        let vizinho = ids[ny * wu + nx];
                        if vizinho != id {
                            if vizinho == 0 || dono_idx(vizinho) != dono_idx(id) {
                                borda_pais = true;
                            } else {
                                borda_prov = true;
                            }
                        }
                    }
                }
                if borda_pais {
                    c = [15, 15, 15, 255];
                } else if borda_prov {
                    c = [
                        (c[0] as f32 * 0.72) as u8,
                        (c[1] as f32 * 0.72) as u8,
                        (c[2] as f32 * 0.72) as u8,
                        255,
                    ];
                }
            }
            dados[i * 4..i * 4 + 4].copy_from_slice(&c);
        }
    }

    let mut imagem = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        dados,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    imagem.sampler = ImageSampler::nearest();
    let handle = imagens.add(imagem);

    commands.spawn(Camera2d);
    commands.spawn(Sprite::from_image(handle.clone()));
    commands.insert_resource(DadosMapa { largura: w, altura: h, ids, textura: handle });
}

fn mover_camera(
    teclas: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    scroll: Res<AccumulatedMouseScroll>,
    egui: Res<EguiWantsInput>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    janela: Query<&Window, With<PrimaryWindow>>,
    botoes: Res<ButtonInput<MouseButton>>,
    mut arrasto: ResMut<ArrastoCamera>,
) {
    let Ok((mut tf, mut proj)) = camera.single_mut() else { return };
    let Projection::Orthographic(ortho) = &mut *proj else { return };
    let sobre_ui = egui.wants_pointer_input() || egui.is_pointer_over_area();

    let mut dir = Vec2::ZERO;
    if teclas.pressed(KeyCode::KeyW) || teclas.pressed(KeyCode::ArrowUp) { dir.y += 1.0; }
    if teclas.pressed(KeyCode::KeyS) || teclas.pressed(KeyCode::ArrowDown) { dir.y -= 1.0; }
    if teclas.pressed(KeyCode::KeyA) || teclas.pressed(KeyCode::ArrowLeft) { dir.x -= 1.0; }
    if teclas.pressed(KeyCode::KeyD) || teclas.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }
    tf.translation += (dir * 900.0 * ortho.scale * time.delta_secs()).extend(0.0);

    if scroll.delta.y != 0.0 && !sobre_ui {
        ortho.scale = (ortho.scale * (1.0 - scroll.delta.y * 0.12)).clamp(0.1, 4.0);
    }

    let Ok(janela) = janela.single() else { return };
    if let Some(pos) = janela.cursor_position() {
        if botoes.just_pressed(MouseButton::Left) && !sobre_ui {
            arrasto.inicio = Some(pos);
            arrasto.moveu = 0.0;
        }
        if botoes.pressed(MouseButton::Left)
            && let Some(ant) = arrasto.inicio
        {
            let d = pos - ant;
            arrasto.moveu += d.length();
            tf.translation.x -= d.x * ortho.scale;
            tf.translation.y += d.y * ortho.scale;
            arrasto.inicio = Some(pos);
        }
    }
}

fn selecionar_provincia(
    botoes: Res<ButtonInput<MouseButton>>,
    arrasto: Res<ArrastoCamera>,
    janela: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mapa: Option<Res<DadosMapa>>,
    mut selecionada: ResMut<ProvinciaSelecionada>,
    mut imagens: ResMut<Assets<Image>>,
    egui: Res<EguiWantsInput>,
) {
    if !botoes.just_released(MouseButton::Left) || arrasto.moveu > 4.0 {
        return;
    }
    if egui.is_pointer_over_area() {
        return;
    }
    let (Some(mapa), Ok(janela), Ok((cam, gt))) = (mapa, janela.single(), camera.single()) else { return };
    let Some(cursor) = janela.cursor_position() else { return };
    let Ok(mundo_pos) = cam.viewport_to_world_2d(gt, cursor) else { return };
    let px = (mundo_pos.x + mapa.largura as f32 / 2.0).floor();
    let py = (mapa.altura as f32 / 2.0 - mundo_pos.y).floor();
    if px < 0.0 || py < 0.0 || px >= mapa.largura as f32 || py >= mapa.altura as f32 {
        selecionada.0 = 0;
        return;
    }
    let id = mapa.ids[py as usize * mapa.largura as usize + px as usize];
    let anterior = selecionada.0;
    selecionada.0 = id;
    if anterior != id {
        realcar(&mapa, &mut imagens, anterior, false);
        realcar(&mapa, &mut imagens, id, true);
    }
}

/// Clareia (ou restaura) os pixels de uma província para indicar seleção.
fn realcar(mapa: &DadosMapa, imagens: &mut Assets<Image>, id: u32, ligar: bool) {
    if id == 0 {
        return;
    }
    let Some(mut img) = imagens.get_mut(&mapa.textura) else { return };
    let Some(dados) = img.data.as_mut() else { return };
    for (i, &pid) in mapa.ids.iter().enumerate() {
        if pid == id {
            for k in 0..3 {
                let v = dados[i * 4 + k] as i32;
                dados[i * 4 + k] = if ligar { (v + 70).min(255) } else { (v - 70).max(0) } as u8;
            }
        }
    }
}
