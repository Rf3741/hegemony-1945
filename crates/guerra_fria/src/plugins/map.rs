//! Plugin do mapa: textura política (cor por país + bordas), câmera e seleção de província.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]
use super::economy::RecursoMundo;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use super::super::estado::AppState;
use bevy::tasks::{AsyncComputeTaskPool, Task, TaskPool, block_on, futures_lite::future};
use bevy_egui::{EguiContexts, egui, EguiPrimaryContextPass};
use bevy_egui::input::EguiWantsInput;
use gf_mapa::mapa::{CatalogoProvincias, id_de_cor_provincia};
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
}

/// Cores base por província e dono (índice = id), guardadas para repintar o mapa nos modos de mapa.
#[derive(Resource)]
pub struct CoresMapa {
    pub cor_prov: Vec<[u8; 4]>,
    pub dono_prov: Vec<u32>,
    /// Índice do ESTADO (região admin-1 pai) por id de província (0 = nenhum).
    pub estado_prov: Vec<u32>,
}

/// Imagem (textura) do mapa exibida.
#[derive(Resource)]
pub struct ImagemMapa(pub Handle<Image>);

/// Província atualmente selecionada (0 = nenhuma).
#[derive(Resource, Default)]
pub struct ProvinciaSelecionada(pub u32);

#[derive(Resource, Default)]
struct ArrastoCamera {
    inicio: Option<Vec2>,
    moveu: f32,
    /// O clique começou fora da UI (pode virar seleção).
    ativo: bool,
}

const COR_OCEANO_RGBA: [u8; 4] = [28, 58, 96, 255];
/// Oceano aberto (mais profundo) e brilho da costa.
const COR_OCEANO_FUNDO: [u8; 4] = [16, 38, 70, 255];
const COR_COSTA: [u8; 3] = [70, 130, 175];

/// Relevo sombreado (cinza, mesmo tamanho do provinces.png), carregado de `assets/map/relevo.png`.
static RELEVO: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();

fn relevo_para(w: u32, h: u32, base: Option<&std::path::Path>) -> Option<&'static [u8]> {
    let r = RELEVO.get_or_init(|| {
        let p = base?.parent()?.join("relevo.png");
        let img = image::open(p).ok()?.to_luma8();
        (img.dimensions() == (w, h)).then(|| img.into_raw())
    });
    r.as_deref().filter(|v| v.len() == (w * h) as usize)
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProvinciaSelecionada>()
            .init_resource::<ArrastoCamera>()
            .add_systems(Startup, |mut c: Commands| {
                c.spawn(Camera2d);
            })
            .add_systems(OnEnter(AppState::Carregando), iniciar_carga)
            .add_systems(Update, aguardar_carga.run_if(in_state(AppState::Carregando)))
            .add_systems(EguiPrimaryContextPass, tela_carregando.run_if(in_state(AppState::Carregando)))
            .add_systems(
                Update,
                (mover_camera, selecionar_provincia)
                    .chain()
                    .run_if(crate::estado::em_partida_ou_menu),
            );
    }
}

/// Entrada pura (sem recursos ECS) para gerar o mapa em segundo plano.
pub struct EntradaMapa {
    pub caminho_png: PathBuf,
    /// Cor RGBA por id de província (índice = id).
    pub cor_prov: Vec<[u8; 4]>,
    /// Índice do dono por id de província (0 = sem dono).
    pub dono_prov: Vec<u32>,
    /// Índice do estado por id de província (0 = nenhum).
    pub estado_prov: Vec<u32>,
}

/// Resultado do processamento em segundo plano.
pub struct SaidaMapa {
    pub largura: u32,
    pub altura: u32,
    pub ids: Vec<u32>,
    pub pixels: Vec<u8>,
    /// Caixa delimitadora por id: [x_min, y_min, x_max, y_max] (inclusive).
    pub caixas: Vec<Option<[u32; 4]>>,
}

#[derive(Resource)]
struct TarefaMapa(Task<Result<SaidaMapa, String>>);

/// Retângulo dos pixels de uma província, em coordenadas de imagem.
#[derive(Resource, Default)]
pub struct CaixasProvincia(pub Vec<Option<[u32; 4]>>);

/// Sprite de realce de seleção; `deslocamento` é o deslocamento horizontal da cópia (wrap).
#[derive(Component)]
struct Realce {
    deslocamento: f32,
}

/// Normaliza a coordenada x do mundo para [-largura/2, largura/2) (wrap horizontal).
pub fn normalizar_x(x: f32, largura: f32) -> f32 {
    (x + largura / 2.0).rem_euclid(largura) - largura / 2.0
}

/// Converte posição do mundo em pixel da imagem, com wrap horizontal.
/// Retorna `None` se estiver fora na vertical.
pub fn pixel_do_mundo(pos: Vec2, largura: u32, altura: u32) -> Option<(u32, u32)> {
    let px = (pos.x + largura as f32 / 2.0).floor().rem_euclid(largura as f32) as u32;
    let py = (altura as f32 / 2.0 - pos.y).floor();
    if py < 0.0 || py >= altura as f32 {
        return None;
    }
    Some((px.min(largura - 1), py as u32))
}

/// Gera ids, pixels coloridos (com bordas) e caixas delimitadoras. Função pura.
pub fn gerar_mapa(ent: &EntradaMapa) -> Result<SaidaMapa, String> {
    let img = image::open(&ent.caminho_png).map_err(|e| e.to_string())?.to_rgb8();
    let (w, h) = img.dimensions();
    let ids: Vec<u32> = img
        .pixels()
        .map(|p| id_de_cor_provincia((p[0], p[1], p[2])).unwrap_or(0))
        .collect();
    relevo_para(w, h, Some(&ent.caminho_png));
    let (pixels, caixas) = colorir(&ids, w, h, &ent.cor_prov, &ent.dono_prov, &ent.estado_prov);
    Ok(SaidaMapa { largura: w, altura: h, ids, pixels, caixas })
}

/// Colore o mapa (cor do dono + bordas de país/província) e calcula as caixas.
pub fn colorir(
    ids: &[u32],
    w: u32,
    h: u32,
    cor_prov: &[[u8; 4]],
    dono_prov: &[u32],
    estado_prov: &[u32],
) -> (Vec<u8>, Vec<Option<[u32; 4]>>) {
    let (wu, hu) = (w as usize, h as usize);
    let dono = |id: u32| dono_prov.get(id as usize).copied().unwrap_or(0);
    let relevo = RELEVO.get().and_then(|r| r.as_deref()).filter(|v| v.len() == wu * hu);
    let estado = |id: u32| estado_prov.get(id as usize).copied().unwrap_or(0);
    let mut caixas: Vec<Option<[u32; 4]>> = vec![None; cor_prov.len()];
    let mut dados = vec![0u8; wu * hu * 4];
    let mut borda = vec![0u8; wu * hu];
    for y in 0..hu {
        for x in 0..wu {
            let i = y * wu + x;
            let id = ids[i];
            let mut c = if id == 0 { COR_OCEANO_FUNDO } else { cor_prov.get(id as usize).copied().unwrap_or(COR_OCEANO_RGBA) };
            if id != 0 {
                if let Some(r) = relevo {
                    // 60% cor do país + 40% cor do país modulada pelo relevo sombreado.
                    let k = (((r[i] as f32 - 185.0) / 45.0) + 1.0).clamp(0.3, 1.6);
                    let f = 0.6 + 0.4 * k;
                    for ch in c.iter_mut().take(3) {
                        *ch = (*ch as f32 * f).clamp(0.0, 255.0) as u8;
                    }
                }
            } else {
                // Brilho suave na costa (1 a 3 px de terra).
                let mut d = 0u8;
                'costa: for r in 1..=3i32 {
                    for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r)] {
                        let nx = (x as i32 + dx).rem_euclid(wu as i32) as usize;
                        let ny = y as i32 + dy;
                        if ny >= 0 && (ny as usize) < hu && ids[ny as usize * wu + nx] != 0 {
                            d = r as u8;
                            break 'costa;
                        }
                    }
                }
                if d > 0 {
                    let t = [0.40, 0.22, 0.10][d as usize - 1];
                    for k in 0..3 {
                        c[k] = (c[k] as f32 + (COR_COSTA[k] as f32 - c[k] as f32) * t) as u8;
                    }
                }
            }
            if id != 0 {
                if let Some(slot) = caixas.get_mut(id as usize) {
                    let (xu, yu) = (x as u32, y as u32);
                    *slot = Some(match *slot {
                        None => [xu, yu, xu, yu],
                        Some([a, b, cc, d]) => [a.min(xu), b.min(yu), cc.max(xu), d.max(yu)],
                    });
                }
            }
            {
                // Vizinhos à direita (com wrap horizontal) e abaixo; marca os DOIS lados da borda
                // (país = 3, estado = 2, província = 1) para a fronteira ficar com 2 px de espessura.
                for (nx, ny) in [((x + 1) % wu, y), (x, y + 1)] {
                    if ny < hu {
                        let ni = ny * wu + nx;
                        let v = ids[ni];
                        if v != id {
                            // 3 = país, 2 = estado, 1 = província
                            let tipo = if v == 0 || dono(v) != dono(id) { 3 } else if estado(v) != estado(id) { 2 } else { 1 };
                            borda[i] = borda[i].max(tipo);
                            borda[ni] = borda[ni].max(tipo);
                        }
                    }
                }
            }
            dados[i * 4..i * 4 + 4].copy_from_slice(&c);
        }
    }
    aplicar_bordas(&mut dados, &borda, ids, wu, hu);
    (dados, caixas)
}

/// Cor da fronteira entre países (escura, quase preta).
const COR_FRONTEIRA_PAIS: [u8; 4] = [15, 15, 15, 255];

/// Pinta fronteiras: países = linha escura de 2 px com brilho (clareamento) de 2 px na cor do
/// país por dentro; províncias/estados = linha sutil de 1 px (cor do país escurecida).
fn aplicar_bordas(dados: &mut [u8], borda: &[u8], ids: &[u32], w: usize, h: usize) {
    let viz = |i: usize| -> [Option<usize>; 4] {
        let (x, y) = (i % w, i / w);
        [
            Some(y * w + (x + 1) % w),
            Some(y * w + (x + w - 1) % w),
            (y + 1 < h).then(|| (y + 1) * w + x),
            (y > 0).then(|| (y - 1) * w + x),
        ]
    };
    // Anel de brilho: 1 = adjacente à fronteira de país, 2 = a duas casas.
    let mut anel = vec![0u8; w * h];
    for i in 0..w * h {
        if borda[i] != 3 && ids[i] != 0 && viz(i).iter().flatten().any(|&n| borda[n] == 3) {
            anel[i] = 1;
        }
    }
    for i in 0..w * h {
        if borda[i] != 3 && anel[i] == 0 && ids[i] != 0 && viz(i).iter().flatten().any(|&n| anel[n] == 1) {
            anel[i] = 2;
        }
    }
    for i in 0..w * h {
        let px = &mut dados[i * 4..i * 4 + 4];
        if borda[i] == 3 {
            px.copy_from_slice(&COR_FRONTEIRA_PAIS);
        } else if borda[i] > 0 {
            // estado: marcada; província: bem sutil
            let f = if borda[i] == 2 { 0.72 } else { 0.92 };
            for c in px.iter_mut().take(3) {
                *c = (*c as f32 * f) as u8;
            }
        } else if anel[i] > 0 {
            let f = if anel[i] == 1 { 0.10 } else { 0.04 };
            for c in px.iter_mut().take(3) {
                *c = (*c as f32 + (255.0 - *c as f32) * f) as u8;
            }
        }
    }
}

/// RGBA do realce: branco translúcido apenas nos pixels da província dentro da caixa.
pub fn pixels_realce(ids: &[u32], largura: u32, id: u32, caixa: [u32; 4]) -> Vec<u8> {
    let [x0, y0, x1, y1] = caixa;
    let (bw, bh) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize);
    let mut out = vec![0u8; bw * bh * 4];
    for by in 0..bh {
        let linha = (y0 as usize + by) * largura as usize + x0 as usize;
        for bx in 0..bw {
            if ids[linha + bx] == id {
                out[(by * bw + bx) * 4..(by * bw + bx) * 4 + 4].copy_from_slice(&[255, 255, 255, 90]);
            }
        }
    }
    out
}

fn iniciar_carga(mut commands: Commands, raiz: Res<RaizProjeto>, catalogo: Res<RecursoCatalogo>, mundo: Res<RecursoMundo>) {
    let max_id = catalogo.0.provincias.iter().map(|p| p.id).max().unwrap_or(0) as usize + 1;
    let mut cor_prov = vec![COR_OCEANO_RGBA; max_id];
    let mut dono_prov = vec![0u32; max_id];
    let mut estado_prov = vec![0u32; max_id];
    let mut estados_idx: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    let mut indices: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for prov in &catalogo.0.provincias {
        let n_est = estados_idx.len() as u32 + 1;
        estado_prov[prov.id as usize] = *estados_idx.entry(prov.adm1_code.as_str()).or_insert(n_est);
        if let Some(tag) = mundo.dono_por_provincia.get(&prov.id)
            && let Some(pais) = mundo.paises.get(tag)
        {
            cor_prov[prov.id as usize] = [pais.cor.0, pais.cor.1, pais.cor.2, 255];
            let n = indices.len() as u32 + 1;
            dono_prov[prov.id as usize] = *indices.entry(tag.as_str()).or_insert(n);
        }
    }
    commands.insert_resource(CoresMapa { cor_prov: cor_prov.clone(), dono_prov: dono_prov.clone(), estado_prov: estado_prov.clone() });
    let entrada = EntradaMapa { caminho_png: raiz.0.join("assets/map/provinces.png"), cor_prov, dono_prov, estado_prov };
    let tarefa = AsyncComputeTaskPool::get_or_init(TaskPool::default).spawn(async move { gerar_mapa(&entrada) });
    commands.insert_resource(TarefaMapa(tarefa));
}

fn aguardar_carga(
    mut commands: Commands,
    tarefa: Option<ResMut<TarefaMapa>>,
    mut imagens: ResMut<Assets<Image>>,
    mut proximo: ResMut<NextState<AppState>>,
    economia: Option<Res<super::economia_sim::EconomiaSim>>,
) {
    // O menu só abre quando a economia (sim_economia) também terminou de carregar.
    if economia.is_none() {
        return;
    }
    let Some(mut tarefa) = tarefa else { return };
    let Some(res) = block_on(future::poll_once(&mut tarefa.0)) else { return };
    commands.remove_resource::<TarefaMapa>();
    let saida = match res {
        Ok(s) => s,
        Err(e) => {
            error!("falha ao gerar o mapa: {e}");
            return;
        }
    };
    let mut imagem = Image::new(
        Extent3d { width: saida.largura, height: saida.altura, depth_or_array_layers: 1 },
        TextureDimension::D2,
        saida.pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    imagem.sampler = ImageSampler::nearest();
    let handle = imagens.add(imagem);
    commands.insert_resource(ImagemMapa(handle.clone()));
    // Três cópias lado a lado para o wrap horizontal.
    for k in [-1.0f32, 0.0, 1.0] {
        commands.spawn((Sprite::from_image(handle.clone()), Transform::from_xyz(k * saida.largura as f32, 0.0, 0.0)));
        commands.spawn((
            Realce { deslocamento: k * saida.largura as f32 },
            Sprite::default(),
            Transform::from_xyz(0.0, 0.0, 1.0),
            Visibility::Hidden,
        ));
    }
    commands.insert_resource(CaixasProvincia(saida.caixas));
    commands.insert_resource(DadosMapa { largura: saida.largura, altura: saida.altura, ids: saida.ids });
    proximo.set(AppState::Menu);
}

fn tela_carregando(mut contextos: EguiContexts, tempo: Res<Time>) -> Result {
    let ctx = contextos.ctx_mut()?;
    let pontos = ".".repeat((tempo.elapsed_secs() * 2.0) as usize % 4);
    egui::Area::new("carregando".into()).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.heading(format!("Carregando o mapa e a economia de 1945{pontos}"));
    });
    Ok(())
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
    mapa: Option<Res<DadosMapa>>,
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
            arrasto.ativo = true;
        }
        if botoes.just_released(MouseButton::Left) && arrasto.inicio.is_some() {
            arrasto.inicio = None;
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
    if let Some(m) = mapa {
        tf.translation.x = normalizar_x(tf.translation.x, m.largura as f32);
    }
}

fn selecionar_provincia(
    botoes: Res<ButtonInput<MouseButton>>,
    arrasto: Res<ArrastoCamera>,
    janela: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    mapa: Option<Res<DadosMapa>>,
    caixas: Option<Res<CaixasProvincia>>,
    mut selecionada: ResMut<ProvinciaSelecionada>,
    mut imagens: ResMut<Assets<Image>>,
    mut realces: Query<(&Realce, &mut Sprite, &mut Transform, &mut Visibility)>,
    egui: Res<EguiWantsInput>,
) {
    if !botoes.just_released(MouseButton::Left) || arrasto.moveu > 4.0 || !arrasto.ativo {
        return;
    }
    if egui.is_pointer_over_area() || egui.wants_pointer_input() {
        return;
    }
    let Some(caixas) = caixas else { return };
    let (Some(mapa), Ok(janela), Ok((cam, gt))) = (mapa, janela.single(), camera.single()) else { return };
    let Some(cursor) = janela.cursor_position() else { return };
    let Ok(mundo_pos) = cam.viewport_to_world_2d(gt, cursor) else { return };
    let id = match pixel_do_mundo(mundo_pos, mapa.largura, mapa.altura) {
        Some((px, py)) => mapa.ids[py as usize * mapa.largura as usize + px as usize],
        None => 0,
    };
    selecionada.0 = id;
    let caixa = caixas.0.get(id as usize).copied().flatten();
    for (r, mut sprite, mut tf, mut vis) in &mut realces {
        match (id != 0, caixa) {
            (true, Some(cx)) => {
                let (bw, bh) = ((cx[2] - cx[0] + 1), (cx[3] - cx[1] + 1));
                let dados = pixels_realce(&mapa.ids, mapa.largura, id, cx);
                let mut img = Image::new(
                    Extent3d { width: bw, height: bh, depth_or_array_layers: 1 },
                    TextureDimension::D2,
                    dados,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::RENDER_WORLD,
                );
                img.sampler = ImageSampler::nearest();
                sprite.image = imagens.add(img);
                let cxm = (cx[0] + cx[2] + 1) as f32 / 2.0 - mapa.largura as f32 / 2.0;
                let cym = mapa.altura as f32 / 2.0 - (cx[1] + cx[3] + 1) as f32 / 2.0;
                tf.translation.x = cxm + r.deslocamento;
                tf.translation.y = cym;
                *vis = Visibility::Visible;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn wrap_x() {
        assert_eq!(normalizar_x(0.0, 100.0), 0.0);
        assert_eq!(normalizar_x(60.0, 100.0), -40.0);
        assert_eq!(normalizar_x(-60.0, 100.0), 40.0);
    }

    #[test]
    fn pixel_com_wrap_e_limites() {
        assert_eq!(pixel_do_mundo(Vec2::new(-50.0, 25.0), 100, 50), Some((0, 0)));
        assert_eq!(pixel_do_mundo(Vec2::new(51.0, 0.0), 100, 50), Some((1, 25)));
        assert_eq!(pixel_do_mundo(Vec2::new(-51.0, 0.0), 100, 50), Some((99, 25)));
        assert_eq!(pixel_do_mundo(Vec2::new(0.0, 30.0), 100, 50), None);
    }

    #[test]
    fn colorir_caixas_e_bordas() {
        // 4x2: ids 1 1 2 0 / 1 1 2 0
        let ids = [1, 1, 2, 0, 1, 1, 2, 0];
        let cor = [COR_OCEANO_RGBA, [200, 0, 0, 255], [0, 200, 0, 255]];
        let dono = [0, 1, 2];
        let (px, caixas) = colorir(&ids, 4, 2, &cor, &dono, &[0, 1, 2]);
        assert_eq!(caixas[1], Some([0, 0, 1, 1]));
        assert_eq!(caixas[2], Some([2, 0, 2, 1]));
        // pixel (1,0) tem vizinho de outro país à direita: borda escura.
        assert_eq!(&px[4..8], &[15, 15, 15, 255]);
        // pixel (2,0) também é fronteira de país (2 px de espessura).
        assert_eq!(&px[8..12], &[15, 15, 15, 255]);
        // pixel (0,0): com o wrap horizontal encosta no oceano (pixel 3), então também é fronteira.
        assert_eq!(&px[0..4], &[15, 15, 15, 255]);
    }

    #[test]
    fn gerar_mapa_real_com_provincias_e_tempo() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cat: CatalogoProvincias =
            ron::from_str(&std::fs::read_to_string(raiz.join("assets/map/provinces.ron")).unwrap()).unwrap();
        let n = cat.provincias.len() + 1;
        let cor_prov = vec![[100, 100, 100, 255]; n];
        // estado por província: índice do adm1; dono: tudo igual (só bordas de província/estado)
        let mut idx = std::collections::HashMap::new();
        let mut estado_prov = vec![0u32; n];
        for p in &cat.provincias {
            let k = idx.len() as u32 + 1;
            estado_prov[p.id as usize] = *idx.entry(p.adm1_code.clone()).or_insert(k);
        }
        let ent = EntradaMapa { caminho_png: raiz.join("assets/map/provinces.png"), cor_prov, dono_prov: vec![1; n], estado_prov };
        let t = std::time::Instant::now();
        let s = gerar_mapa(&ent).unwrap();
        eprintln!("gerar_mapa: {:?}", t.elapsed());
        assert_eq!(s.pixels.len(), (s.largura * s.altura * 4) as usize);
        let com_caixa = s.caixas.iter().flatten().count();
        assert!(com_caixa > 6000, "{com_caixa}");
    }

    #[test]
    fn realce_so_nos_pixels_da_provincia() {
        let ids = [1, 2, 1, 1];
        let r = pixels_realce(&ids, 2, 1, [0, 0, 1, 1]);
        assert_eq!(r[3], 90);
        assert_eq!(r[7], 0);
        assert_eq!(r[11], 90);
    }
}
