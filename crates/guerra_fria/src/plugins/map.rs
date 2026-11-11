//! Plugin do mapa: textura política (cor por país + bordas), câmera e seleção de província.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]
use super::economy::RecursoMundo;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, MaterialExtension2d, SpriteMaterial, SpriteMaterialPlugin};
use bevy::reflect::TypePath;
use bevy::window::PrimaryWindow;
use super::super::estado::AppState;
use bevy::tasks::{AsyncComputeTaskPool, Task, TaskPool, block_on, futures_lite::future};
use bevy_egui::{EguiContexts, egui, EguiPrimaryContextPass};
use bevy_egui::input::EguiWantsInput;
use gf_mapa::mapa::{CatalogoProvincias, ESCALA_Y_MAPA, id_de_cor_provincia};
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
    /// Cor do ocupante se a província estiver sob controle militar de outra nação (None = soberana).
    pub ocupante_cor: Vec<Option<[u8; 4]>>,
}

/// Paleta do mapa na GPU (textura pequena: uma cor/ocupante/dono/estado por província).
/// Repintar o mapa = reescrever só esta imagem com [`pintar_paleta`]; o shader do terreno
/// (`assets/shaders/terreno.wesl`) resolve cada pixel pelo id da província.
#[derive(Resource)]
pub struct ImagemMapa(pub Handle<Image>);

/// Província atualmente selecionada (0 = nenhuma).
#[derive(Resource, Default)]
pub struct ProvinciaSelecionada(pub u32);

#[derive(Resource, Default)]
struct LegendaOcupacao {
    fixada: bool,
    sobre_ocupacao: bool,
}

/// Parâmetros estáticos da paleta (setores de Berlim e flag do relevo). Layout = `ParamsPaleta` no shader.
#[derive(ShaderType, Debug, Clone, Default)]
pub struct ParamsPaleta {
    /// x = id de Berlim, y/z = centro em pixels, w = 1 se há setores de ocupação.
    pub berlim: Vec4,
    /// Cores dos setores (URSS, França, Reino Unido, EUA), RGBA 0..1.
    pub setores: [Vec4; 4],
    /// x = 1 se o relevo modula a cor política.
    pub flags: Vec4,
}

/// Extensão do material de sprite: o mapa é desenhado a partir de uma textura de IDs de província
/// (gerada uma vez) + uma paleta pequena; o shader aplica relevo/hillshade, fronteiras, hachura de
/// ocupação e oceano. A seleção por clique continua lendo `DadosMapa.ids` na CPU.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct TerrainMaterial {
    #[uniform(20)]
    params: Vec4,
    #[texture(21)]
    #[sampler(22)]
    relief: Handle<Image>,
    #[texture(23)]
    ids: Handle<Image>,
    #[texture(24)]
    paleta: Handle<Image>,
    #[uniform(25)]
    paleta_params: ParamsPaleta,
}

impl MaterialExtension2d for TerrainMaterial {
    fn fragment_shader() -> Option<ShaderRef> {
        Some("shaders/terreno.wesl".into())
    }

    fn alpha_mode(&self) -> Option<AlphaMode2d> {
        Some(AlphaMode2d::Blend)
    }
}

#[derive(Resource, Default)]
struct ArrastoCamera {
    /// Escala alvo do zoom suave (0 = ainda não inicializada).
    escala_alvo: f32,
    inicio: Option<Vec2>,
    moveu: f32,
    /// O clique começou fora da UI (pode virar seleção).
    ativo: bool,
}

const COR_OCEANO_RGBA: [u8; 4] = [28, 58, 96, 255];
/// Oceano aberto (mais profundo) e brilho da costa.
#[cfg(test)]
const COR_OCEANO_FUNDO: [u8; 4] = [16, 38, 70, 255];
#[cfg(test)]
const COR_COSTA: [u8; 3] = [70, 130, 175];

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProvinciaSelecionada>()
            .init_resource::<LegendaOcupacao>()
            .init_resource::<ArrastoCamera>()
            .add_plugins(SpriteMaterialPlugin::<TerrainMaterial>::default())
            .add_systems(Startup, |mut c: Commands| {
                c.spawn(Camera2d);
            })
            .add_systems(Update, (animar_ondas, atualizar_material_terreno))
            .add_systems(Update, atualizar_legenda_zonas)
            .add_systems(OnEnter(AppState::Carregando), iniciar_carga)
            .add_systems(Update, aguardar_carga.run_if(in_state(AppState::Carregando)))
            .add_systems(EguiPrimaryContextPass, legenda_zonas_ocupacao.run_if(in_state(AppState::Jogando)))
            .add_systems(
                Update,
                (mover_camera, selecionar_provincia)
                    .chain()
                    .run_if(crate::estado::em_partida_ou_menu),
            );
    }

}

fn atualizar_material_terreno(
    cfg: Option<Res<super::config::ConfigJogo>>,
    tempo: Res<Time>,
    paleta: Option<Res<ImagemMapa>>,
    mut eventos: MessageReader<AssetEvent<Image>>,
    mut materiais: ResMut<Assets<TerrainMaterial>>,
    sprites: Query<&SpriteMaterial<TerrainMaterial>>,
    mut ultimo: Local<Option<(Vec4, bool)>>,
) {
    let (hillshade, costa, agua, completo) = match cfg.as_ref() {
        None => (0.0, 0.0, 0.0, false),
        Some(cfg) => {
            let g = &cfg.graficos;
            let (h, c, a, completo) = match g.qualidade {
                super::config::QualidadeGrafica::Baixo => (0.0, 0.0, 0.0, false),
                super::config::QualidadeGrafica::Medio => (g.hillshade, 0.0, 0.0, false),
                super::config::QualidadeGrafica::Alto => (g.hillshade, 0.0, g.ondas, true),
                super::config::QualidadeGrafica::Ultra => (g.hillshade, g.sombra_costas, g.ondas, true),
            };
            (h, c, if g.agua_animada { a } else { 0.0 }, completo)
        }
    };
    // O tempo só entra quando a água animada está ligada: nos demais casos o material não muda
    // entre quadros e o bind group não é refeito à toa.
    let t = if agua > 0.0 { tempo.elapsed_secs() } else { 0.0 };
    let params = Vec4::new(hillshade, costa, agua, t);
    // A paleta reescrita (`pintar_paleta`) exige tocar o material uma vez para o bind group
    // enxergar a nova textura.
    let mut paleta_mudou = false;
    if let Some(p) = paleta.as_ref() {
        for e in eventos.read() {
            paleta_mudou |= e.is_modified(p.0.id());
        }
    } else {
        eventos.clear();
    }
    if !paleta_mudou && *ultimo == Some((params, completo)) {
        return;
    }
    *ultimo = Some((params, completo));
    for sprite in &sprites {
        if let Some(mut material) = materiais.get_mut(&sprite.0) {
            material.params = params;
            material.paleta_params.flags.y = completo as u8 as f32;
        }
    }
}

/// Entrada pura (sem recursos ECS) para gerar o mapa em segundo plano.
pub struct EntradaMapa {
    pub caminho_png: PathBuf,
    /// Cor RGBA por id de província (índice = id).
    pub cor_prov: Vec<[u8; 4]>,
    /// Índice do dono por id de província (0 = sem dono).
    _dono_prov: Vec<u32>,
    /// Índice do estado por id de província (0 = nenhum).
    _estado_prov: Vec<u32>,
    /// Cor do ocupante por id de província (hachura de zona de ocupação já na carga inicial).
    _ocupante_cor: Vec<Option<[u8; 4]>>,
}

/// Berlim é uma única província no mapa; para mostrar os 4 setores de ocupação o pintor divide os
/// pixels dela em torno do centroide (leste = URSS, oeste: norte França, centro Reino Unido, sul EUA).
pub struct SetoresBerlim {
    pub id: u32,
    pub centro_px: (u32, u32),
    /// Ordem: URSS, França, Reino Unido, EUA.
    pub cores: [[u8; 4]; 4],
}
pub static SETORES_BERLIM: std::sync::OnceLock<SetoresBerlim> = std::sync::OnceLock::new();

/// Resultado do processamento em segundo plano.
pub struct SaidaMapa {
    pub largura: u32,
    pub altura: u32,
    pub ids: Vec<u32>,
    /// Textura de IDs para a GPU (Rg8, ver [`preparar_ids_gpu`]).
    pub ids_gpu: Vec<u8>,
    /// Maior id de província presente na imagem.
    pub id_max: u32,
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
    let py = (altura as f32 / 2.0 - pos.y / ESCALA_Y_MAPA).floor();
    if py < 0.0 || py >= altura as f32 {
        return None;
    }
    Some((px.min(largura - 1), py as u32))
}

/// Lê o PNG de províncias e gera ids, textura de IDs para a GPU e caixas delimitadoras. Função pura.
/// Não pinta nada: a cor vem da paleta (ver [`bytes_paleta`]) e é resolvida no shader.
pub fn gerar_mapa(ent: &EntradaMapa) -> Result<SaidaMapa, String> {
    let img = image::open(&ent.caminho_png).map_err(|e| e.to_string())?.to_rgb8();
    let (w, h) = img.dimensions();
    let ids: Vec<u32> = img
        .pixels()
        .map(|p| id_de_cor_provincia((p[0], p[1], p[2])).unwrap_or(0))
        .collect();
    let id_max = ids.iter().copied().max().unwrap_or(0);
    if id_max > ID_MAX_GPU {
        return Err(format!("id de província {id_max} não cabe na textura de IDs (máx. {ID_MAX_GPU})"));
    }
    let caixas = calcular_caixas(&ids, w, ent.cor_prov.len().max(id_max as usize + 1));
    let ids_gpu = preparar_ids_gpu(&ids, w, h);
    Ok(SaidaMapa { largura: w, altura: h, ids, ids_gpu, id_max, caixas })
}

/// Maior id de província que a textura de IDs da GPU comporta (13 bits).
pub const ID_MAX_GPU: u32 = 8191;
/// Colunas da textura de paleta (as províncias ocupam `id % 4096`, bloco de 4 linhas por `id / 4096`).
pub const PALETA_LARGURA: usize = 4096;

/// Caixa delimitadora [x_min, y_min, x_max, y_max] (inclusive) por id de província.
pub fn calcular_caixas(ids: &[u32], w: u32, n: usize) -> Vec<Option<[u32; 4]>> {
    let mut caixas: Vec<Option<[u32; 4]>> = vec![None; n];
    for (i, &id) in ids.iter().enumerate() {
        if id == 0 {
            continue;
        }
        let (xu, yu) = ((i as u32) % w, (i as u32) / w);
        if let Some(slot) = caixas.get_mut(id as usize) {
            *slot = Some(match *slot {
                None => [xu, yu, xu, yu],
                Some([a, b, c, d]) => [a.min(xu), b.min(yu), c.max(xu), d.max(yu)],
            });
        }
    }
    caixas
}

/// Textura de IDs (2 bytes/pixel, formato Rg8) lida pelo shader do terreno:
/// Terra: `r = id & 255`; `g = (id >> 8) & 31 | perto << 5 | fronteira << 6`, onde `fronteira` marca
/// o pixel que tem vizinho de outro id (a única região onde o modo barato calcula bordas) e `perto`
/// marca até 3 pixels de uma fronteira (região dos anéis de brilho, só no modo completo).
/// Oceano: `g = 128` e `r = costa` (0..=3, distância à terra mais próxima nas quatro direções).
pub fn preparar_ids_gpu(ids: &[u32], w: u32, h: u32) -> Vec<u8> {
    let (wu, hu) = (w as usize, h as usize);
    // Pixels de fronteira: algum vizinho (x com wrap, y sem) tem outro id.
    let mut perto = vec![false; wu * hu];
    let mut fronteira = vec![false; wu * hu];
    for y in 0..hu {
        for x in 0..wu {
            let i = y * wu + x;
            let id = ids[i];
            let dir = y * wu + (x + 1) % wu;
            if ids[dir] != id {
                perto[i] = true;
                perto[dir] = true;
            }
            if y + 1 < hu && ids[i + wu] != id {
                perto[i] = true;
                perto[i + wu] = true;
            }
        }
    }
    fronteira = perto.clone();
    // Dilata 3 passos (vizinhança de 4): cobre tudo que o shader consulta para o anel de brilho.
    for _ in 0..3 {
        let anterior = perto.clone();
        for y in 0..hu {
            for x in 0..wu {
                let i = y * wu + x;
                if anterior[i] {
                    continue;
                }
                perto[i] = anterior[y * wu + (x + 1) % wu]
                    || anterior[y * wu + (x + wu - 1) % wu]
                    || (y + 1 < hu && anterior[i + wu])
                    || (y > 0 && anterior[i - wu]);
            }
        }
    }
    let mut out = vec![0u8; wu * hu * 2];
    for y in 0..hu {
        for x in 0..wu {
            let i = y * wu + x;
            let id = ids[i].min(ID_MAX_GPU);
            let mut costa = 0u32;
            if id == 0 {
                'costa: for r in 1..=3i32 {
                    for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r)] {
                        let nx = (x as i32 + dx).rem_euclid(wu as i32) as usize;
                        let ny = y as i32 + dy;
                        if ny >= 0 && (ny as usize) < hu && ids[ny as usize * wu + nx] != 0 {
                            costa = r as u32;
                            break 'costa;
                        }
                    }
                }
            }
            if id == 0 {
                out[i * 2] = costa as u8;
                out[i * 2 + 1] = 128 | ((perto[i] as u8) << 5) | ((fronteira[i] as u8) << 6);
            } else {
                out[i * 2] = (id & 255) as u8;
                out[i * 2 + 1] = (((id >> 8) & 31) | ((perto[i] as u32) << 5) | ((fronteira[i] as u32) << 6)) as u8;
            }
        }
    }
    out
}

/// Altura (em linhas) da textura de paleta para `n` províncias.
pub fn altura_paleta(n: usize) -> usize {
    n.div_ceil(PALETA_LARGURA).max(1) * 4
}

/// Bytes RGBA8 da paleta para `n` ids. Por id: linha 0 = cor; linha 1 = cor do ocupante (alfa 255
/// se há ocupação, 0 se não); linha 2 = dono (r,g = u16 little-endian) e estado (b,a).
/// É isto, alguns KB, que muda quando o mapa é repintado; a textura de IDs nunca muda.
pub fn bytes_paleta(
    n: usize,
    cor_prov: &[[u8; 4]],
    dono_prov: &[u32],
    estado_prov: &[u32],
    ocupante_cor: Option<&[Option<[u8; 4]>]>,
) -> Vec<u8> {
    let altura = altura_paleta(n);
    let mut out = vec![0u8; PALETA_LARGURA * altura * 4];
    let mut gravar = |id: usize, linha: usize, v: [u8; 4]| {
        let y = (id / PALETA_LARGURA) * 4 + linha;
        let o = (y * PALETA_LARGURA + id % PALETA_LARGURA) * 4;
        out[o..o + 4].copy_from_slice(&v);
    };
    for id in 0..n.min((altura / 4) * PALETA_LARGURA) {
        // Alfa da linha 0: 254 = província ocupada (o shader só lê a linha 1 nesse caso).
        let mut cor = cor_prov.get(id).copied().unwrap_or(COR_OCEANO_RGBA);
        cor[3] = 255;
        if let Some(Some(c)) = ocupante_cor.and_then(|o| o.get(id)) {
            cor[3] = 254;
            gravar(id, 1, [c[0], c[1], c[2], 255]);
        }
        gravar(id, 0, cor);
        let dono = dono_prov.get(id).copied().unwrap_or(0).min(u16::MAX as u32) as u16;
        let estado = estado_prov.get(id).copied().unwrap_or(0).min(u16::MAX as u32) as u16;
        gravar(id, 2, [dono as u8, (dono >> 8) as u8, estado as u8, (estado >> 8) as u8]);
    }
    out
}

/// Repinta o mapa: reescreve só a paleta (sem tocar na textura de IDs nem nos pixels do mapa).
/// Custa microssegundos, contra ~600 ms da repintura antiga da textura de 11,5 M pixels.
pub fn pintar_paleta(
    imagens: &mut Assets<Image>,
    paleta: &ImagemMapa,
    cor_prov: &[[u8; 4]],
    dono_prov: &[u32],
    estado_prov: &[u32],
    ocupante_cor: Option<&[Option<[u8; 4]>]>,
) {
    let Some(mut img) = imagens.get_mut(&paleta.0) else { return };
    let n = (img.texture_descriptor.size.height as usize / 4) * PALETA_LARGURA;
    img.data = Some(bytes_paleta(n, cor_prov, dono_prov, estado_prov, ocupante_cor));
}

/// Colore o mapa (cor do dono + bordas de país/província) e calcula as caixas.
/// Pintor de REFERÊNCIA em CPU (sem o relevo, que só existe no shader): o jogo usa o shader do
/// terreno; isto serve aos testes de equivalência.
#[cfg(test)]
pub fn colorir(
    ids: &[u32],
    w: u32,
    h: u32,
    cor_prov: &[[u8; 4]],
    dono_prov: &[u32],
    estado_prov: &[u32],
) -> (Vec<u8>, Vec<Option<[u32; 4]>>) {
    colorir_com_hachura(ids, w, h, cor_prov, dono_prov, estado_prov, None)
}

/// Colore o mapa com suporte a hachura listrada para províncias ocupadas por forças inimigas.
#[cfg(test)]
pub fn colorir_com_hachura(
    ids: &[u32],
    w: u32,
    h: u32,
    cor_prov: &[[u8; 4]],
    dono_prov: &[u32],
    estado_prov: &[u32],
    ocupante_cor: Option<&[Option<[u8; 4]>]>,
) -> (Vec<u8>, Vec<Option<[u32; 4]>>) {
    let (wu, hu) = (w as usize, h as usize);
    let dono = |id: u32| dono_prov.get(id as usize).copied().unwrap_or(0);
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
                if let Some(ocupados) = ocupante_cor {
                    if let Some(Some(cor_ocup)) = ocupados.get(id as usize) {
                        // Hachura diagonal estilo Victoria 3: faixas de 4px na cor do ocupante militar
                        if (x + y) % 8 < 4 {
                            c = *cor_ocup;
                            if let Some(b) = SETORES_BERLIM.get().filter(|b| b.id == id) {
                                let (cx, cy) = b.centro_px;
                                let k = if x as u32 >= cx { 0 } else if (y as u32) + 1 < cy { 1 } else if (y as u32) > cy + 1 { 3 } else { 2 };
                                c = b.cores[k];
                            }
                        }
                    }
                }
            } else {
                // Oceano com textura: gradiente por latitude (mais claro no equador e perto dos polos
                // mais escuro) e ruído suave de baixa frequência.
                let (fx, fy) = (x as f32, y as f32);
                let lat = ((fy / hu as f32) - 0.5).abs() * 2.0;
                let ruido = (fx * 0.021 + fy * 0.013).sin() * 0.5 + (fx * 0.047 - fy * 0.031).sin() * 0.3 + (fx * 0.09 + fy * 0.07).sin() * 0.2;
                let f = 1.0 + (0.22 - lat * 0.40) + ruido * 0.045;
                for ch in c.iter_mut().take(3) {
                    *ch = (*ch as f32 * f).clamp(0.0, 255.0) as u8;
                }
                // Brilho suave na costa (1 a 3 px de terra) e espuma clara na linha d'água.
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
                    let t = [0.55, 0.30, 0.14][d as usize - 1];
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
#[cfg(test)]
const COR_FRONTEIRA_PAIS: [u8; 4] = [15, 15, 15, 255];

/// Pinta fronteiras: países = linha escura de 2 px com brilho (clareamento) de 2 px na cor do
/// país por dentro; províncias/estados = linha sutil de 1 px (cor do país escurecida).
#[cfg(test)]
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
    // Zonas de ocupação (Alemanha/Áustria): controlador diferente do dono -> hachura na cor do ocupante.
    let mut ocupante_cor: Vec<Option<[u8; 4]>> = vec![None; max_id];
    for prov in &catalogo.0.provincias {
        let dono = mundo.dono_por_provincia.get(&prov.id);
        if let Some(ctrl) = mundo.controlador_por_provincia.get(&prov.id)
            && Some(ctrl) != dono
            && let Some(pais) = mundo.paises.get(ctrl)
        {
            ocupante_cor[prov.id as usize] = Some([pais.cor.0, pais.cor.1, pais.cor.2, 255]);
            if prov.adm1_code == "DEU-1599" {
                let cor = |t: &str| mundo.paises.get(t).map_or([200, 200, 200, 255], |p| [p.cor.0, p.cor.1, p.cor.2, 255]);
                let _ = SETORES_BERLIM.set(SetoresBerlim {
                    id: prov.id,
                    centro_px: prov.centroide_px,
                    cores: [cor("SOV"), cor("FRA"), cor("GBR"), cor("USA")],
                });
            }
        }
    }
    commands.insert_resource(CoresMapa { cor_prov: cor_prov.clone(), dono_prov: dono_prov.clone(), estado_prov: estado_prov.clone(), ocupante_cor: ocupante_cor.clone() });
    let entrada = EntradaMapa { caminho_png: raiz.0.join("assets/map/provinces.png"), cor_prov, _dono_prov: dono_prov, _estado_prov: estado_prov, _ocupante_cor: ocupante_cor };
    let tarefa = AsyncComputeTaskPool::get_or_init(TaskPool::default).spawn(async move { gerar_mapa(&entrada) });
    commands.insert_resource(TarefaMapa(tarefa));
}

fn aguardar_carga(
    mut commands: Commands,
    tarefa: Option<ResMut<TarefaMapa>>,
    mut imagens: ResMut<Assets<Image>>,
    mut materiais: ResMut<Assets<TerrainMaterial>>,
    asset_server: Res<AssetServer>,
    economia: Option<Res<super::economia_sim::EconomiaSim>>,
    _cfg: Option<Res<super::config::ConfigJogo>>,
    mut abertura: ResMut<super::abertura::CarregamentoPronto>,
    cores: Option<Res<CoresMapa>>,
    raiz: Option<Res<RaizProjeto>>,
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
    // Textura de IDs (fixa) + paleta (única coisa que muda ao repintar o mapa).
    let mut ids_img = Image::new(
        Extent3d { width: saida.largura, height: saida.altura, depth_or_array_layers: 1 },
        TextureDimension::D2,
        saida.ids_gpu,
        TextureFormat::Rg8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    ids_img.sampler = ImageSampler::nearest();
    let ids_handle = imagens.add(ids_img);
    let n_paleta = (saida.id_max as usize + 1).max(cores.as_ref().map_or(0, |c| c.cor_prov.len()));
    let mut paleta_img = Image::new(
        Extent3d { width: PALETA_LARGURA as u32, height: altura_paleta(n_paleta) as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        match cores.as_ref() {
            Some(c) => bytes_paleta(n_paleta, &c.cor_prov, &c.dono_prov, &c.estado_prov, Some(&c.ocupante_cor)),
            None => bytes_paleta(n_paleta, &[], &[], &[], None),
        },
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    paleta_img.sampler = ImageSampler::nearest();
    let paleta_handle = imagens.add(paleta_img);
    // Sprite-base: um pixel branco (só fornece tingimento/alfa); o mapa em si vem do shader.
    let mut branco = Image::new(
        Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        TextureDimension::D2,
        vec![255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    branco.sampler = ImageSampler::nearest();
    let branco_handle = imagens.add(branco);
    // Relevo: só modula a cor política se existir e tiver o mesmo tamanho da textura de IDs.
    let caminho_relevo = raiz.as_ref().map(|r| r.0.join("assets/map/relevo.png"));
    let relevo_ok = caminho_relevo
        .as_ref()
        .and_then(|c| image::image_dimensions(c).ok())
        .is_some_and(|d| d == (saida.largura, saida.altura));
    let relevo = if caminho_relevo.as_ref().is_some_and(|c| c.exists()) {
        asset_server.load("map/relevo.png")
    } else {
        let mut cinza = Image::new(
            Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            TextureDimension::D2,
            vec![128],
            TextureFormat::R8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        );
        cinza.sampler = ImageSampler::linear();
        imagens.add(cinza)
    };
    let mut paleta_params = ParamsPaleta { flags: Vec4::new(relevo_ok as u8 as f32, 0.0, 0.0, 0.0), ..default() };
    if let Some(b) = SETORES_BERLIM.get() {
        paleta_params.berlim = Vec4::new(b.id as f32, b.centro_px.0 as f32, b.centro_px.1 as f32, 1.0);
        for (k, c) in b.cores.iter().enumerate() {
            paleta_params.setores[k] = Vec4::new(c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, 1.0);
        }
    }
    let material = materiais.add(TerrainMaterial {
        params: Vec4::new(0.0, 0.0, 0.0, 0.0),
        relief: relevo,
        ids: ids_handle,
        paleta: paleta_handle.clone(),
        paleta_params,
    });
    commands.insert_resource(ImagemMapa(paleta_handle));
    // Três cópias lado a lado para o wrap horizontal.
    for k in [-1.0f32, 0.0, 1.0] {
        commands.spawn((
            Sprite {
                image: branco_handle.clone(),
                custom_size: Some(Vec2::new(saida.largura as f32, saida.altura as f32)),
                ..default()
            },
            SpriteMaterial(material.clone()),
            Transform::from_xyz(k * saida.largura as f32, 0.0, 0.0)
                .with_scale(Vec3::new(1.0, ESCALA_Y_MAPA, 1.0)),
        ));
        commands.spawn((
            Realce { deslocamento: k * saida.largura as f32 },
            Sprite::default(),
            Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::new(1.0, ESCALA_Y_MAPA, 1.0)),
            Visibility::Hidden,
        ));
    }
    criar_ondas(&mut commands, &mut imagens, &saida.ids, saida.largura, saida.altura);
    commands.insert_resource(CaixasProvincia(saida.caixas));
    commands.insert_resource(DadosMapa { largura: saida.largura, altura: saida.altura, ids: saida.ids });
    abertura.0 = true;
}

/// Camada de ondas do oceano: duas texturas de baixa resolução (1/4) que só cobrem o mar aberto
/// e se alternam em fade, dando um leve movimento de brilho na água.
#[derive(Component)]
struct OndasOceano {
    fase: f32,
}

fn criar_ondas(commands: &mut Commands, imagens: &mut Assets<Image>, ids: &[u32], w: u32, h: u32) {
    const D: u32 = 4;
    let (lw, lh) = (w / D, h / D);
    let mut handles = Vec::new();
    for variante in 0..2u32 {
        let mut px = [170u8, 215, 245, 0].repeat((lw * lh) as usize);
        for ly in 0..lh {
            for lx in 0..lw {
                // Só mar aberto: todo o bloco DxD é oceano.
                let mut mar = true;
                'b: for dy in 0..D {
                    for dx in 0..D {
                        if ids[((ly * D + dy) * w + lx * D + dx) as usize] != 0 {
                            mar = false;
                            break 'b;
                        }
                    }
                }
                if !mar {
                    continue;
                }
                let (fx, fy) = (lx as f32, ly as f32);
                let v = if variante == 0 {
                    (fx * 0.35 + fy * 0.18).sin() * (fy * 0.27 - fx * 0.11).cos()
                } else {
                    (fx * 0.22 - fy * 0.31 + 2.0).sin() * (fx * 0.15 + fy * 0.24).cos()
                };
                let a = (v.max(0.0).powf(1.5) * 255.0) as u8;
                let i = ((ly * lw + lx) * 4) as usize;
                px[i..i + 4].copy_from_slice(&[170, 215, 245, a]);
            }
        }
        let mut img = Image::new(
            Extent3d { width: lw, height: lh, depth_or_array_layers: 1 },
            TextureDimension::D2,
            px,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        img.sampler = ImageSampler::linear();
        handles.push(imagens.add(img));
    }
    for (ik, k) in [-1.0f32, 0.0, 1.0].into_iter().enumerate() {
        for (i, hdl) in handles.iter().enumerate() {
            commands.spawn((
                OndasOceano { fase: i as f32 * std::f32::consts::PI },
                Sprite { image: hdl.clone(), custom_size: Some(Vec2::new(w as f32, h as f32 * ESCALA_Y_MAPA)), color: Color::srgba(1.0, 1.0, 1.0, 0.0), ..default() },
                Transform::from_xyz(k * w as f32, 0.0, 0.2 + (ik * 2 + i) as f32 * 0.001),
                Visibility::Hidden,
            ));
        }
    }
}

fn animar_ondas(
    tempo: Res<Time>,
    estilo: Res<visual_mapa::EstiloMapa>,
    mut ondas: Query<(&OndasOceano, &mut Sprite, &mut Visibility)>,
) {
    let t = tempo.elapsed_secs() * estilo.oceano_ondas_vel;
    for (o, mut sp, mut vis) in &mut ondas {
        if !estilo.oceano_ondas_ativo {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
            continue;
        }
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
        // Fade senoidal contínuo (sem corte em zero): as duas camadas se cruzam suavemente.
        let a = (0.5 + 0.5 * (t + o.fase).sin()) * estilo.oceano_ondas_alfa;
        sp.color = Color::srgba(1.0, 1.0, 1.0, a);
    }
}

fn atualizar_legenda_zonas(
    teclas: Res<ButtonInput<KeyCode>>,
    mapa: Option<Res<DadosMapa>>,
    cores: Option<Res<CoresMapa>>,
    camera: Query<&Transform, With<Camera2d>>,
    mut estado: ResMut<LegendaOcupacao>,
) {
    if teclas.just_pressed(KeyCode::KeyH) {
        estado.fixada = !estado.fixada;
    }
    estado.sobre_ocupacao = false;
    let (Some(mapa), Some(cores), Ok(camera)) = (mapa, cores, camera.single()) else {
        return;
    };
    if let Some((x, y)) = pixel_do_mundo(
        Vec2::new(camera.translation.x, camera.translation.y),
        mapa.largura,
        mapa.altura,
    ) {
        let id = mapa.ids[y as usize * mapa.largura as usize + x as usize] as usize;
        estado.sobre_ocupacao = cores.ocupante_cor.get(id).is_some_and(Option::is_some);
    }
}

/// Legenda "Zona de ocupação" (canto inferior esquerdo) sob demanda ou sobre uma zona ocupada.
fn legenda_zonas_ocupacao(
    mut contextos: EguiContexts,
    mundo: Res<RecursoMundo>,
    modo: Res<super::hud_mapas::ModoTematico>,
    estado: Res<LegendaOcupacao>,
) -> Result {
    if *modo != super::hud_mapas::ModoTematico::Politico || (!estado.fixada && !estado.sobre_ocupacao) {
        return Ok(());
    }
    let mut ocupantes = std::collections::BTreeSet::new();
    for (id, dono) in &mundo.dono_por_provincia {
        if let Some(controlador) = mundo.controlador_por_provincia.get(id)
            && controlador != dono
        {
            ocupantes.insert(controlador.as_str());
        }
    }
    if ocupantes.is_empty() {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?;
    egui::Area::new("legenda_zonas".into()).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -88.0]).show(ctx, |ui| {
        egui::Frame::popup(ui.style())
            .fill(ui_refinada::menu::PAPEL)
            .stroke(egui::Stroke::new(1.5, ui_refinada::menu::VERMELHO))
            .inner_margin(8.0)
            .show(ui, |ui| {
            ui.label(egui::RichText::new("Zona de ocupação (hachura) — H fixa").color(ui_refinada::menu::TINTA).strong());
            for tag in ocupantes {
                if let Some(p) = mundo.paises.get(tag) {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(p.cor.0, p.cor.1, p.cor.2));
                        ui.label(&p.nome);
                    });
                }
            }
        });
    });
    Ok(())
}

/// Verdadeiro se o ponteiro está sobre qualquer UI egui: janelas/áreas flutuantes, widget em uso
/// ou painéis da cadeia raiz (barras do topo e painel lateral), que ficam na camada de fundo e por
/// isso não contam em `is_pointer_over_area`.
fn ponteiro_sobre_ui(egui: &EguiWantsInput, contextos: &mut EguiContexts, janela: &Window) -> bool {
    if egui.wants_pointer_input() || egui.is_pointer_over_area() {
        return true;
    }
    let Some(cursor) = janela.cursor_position() else { return false };
    let Ok(ctx) = contextos.ctx_mut() else { return false };
    let ppp = ctx.pixels_per_point().max(0.01);
    let pos = egui::pos2(cursor.x * janela.scale_factor() / ppp, cursor.y * janela.scale_factor() / ppp);
    !super::ui::area_livre_mapa(ctx).contains(pos)
}

fn mover_camera(
    teclas: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    scroll: Res<AccumulatedMouseScroll>,
    egui: Res<EguiWantsInput>,
    mut contextos: EguiContexts,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    janela: Query<&Window, With<PrimaryWindow>>,
    botoes: Res<ButtonInput<MouseButton>>,
    mut arrasto: ResMut<ArrastoCamera>,
    mapa: Option<Res<DadosMapa>>,
    estilo: Option<Res<visual_mapa::EstiloMapa>>,
) {
    let Ok((mut tf, mut proj)) = camera.single_mut() else { return };
    let Projection::Orthographic(ortho) = &mut *proj else { return };
    let sobre_ui = janela.single().is_ok_and(|j| ponteiro_sobre_ui(&egui, &mut contextos, j));

    let mut dir = Vec2::ZERO;
    if teclas.pressed(KeyCode::KeyW) || teclas.pressed(KeyCode::ArrowUp) { dir.y += 1.0; }
    if teclas.pressed(KeyCode::KeyS) || teclas.pressed(KeyCode::ArrowDown) { dir.y -= 1.0; }
    if teclas.pressed(KeyCode::KeyA) || teclas.pressed(KeyCode::ArrowLeft) { dir.x -= 1.0; }
    if teclas.pressed(KeyCode::KeyD) || teclas.pressed(KeyCode::ArrowRight) { dir.x += 1.0; }
    tf.translation += (dir * 900.0 * ortho.scale * time.delta_secs()).extend(0.0);

    if scroll.delta.y != 0.0 && !sobre_ui {
        let base = if arrasto.escala_alvo > 0.0 { arrasto.escala_alvo } else { ortho.scale };
        arrasto.escala_alvo = (base * (1.0 - scroll.delta.y * 0.12)).clamp(0.1, 4.0);
    }
    // Zoom suave: aproxima a escala do alvo de forma exponencial (independente do FPS).
    let taxa = estilo.as_ref().map_or(0.0, |e| e.zoom_suave);
    if arrasto.escala_alvo > 0.0 {
        if taxa <= 0.0 || (ortho.scale - arrasto.escala_alvo).abs() < 0.0005 {
            ortho.scale = arrasto.escala_alvo;
            arrasto.escala_alvo = 0.0;
        } else {
            let k = 1.0 - (-taxa * time.delta_secs()).exp();
            ortho.scale += (arrasto.escala_alvo - ortho.scale) * k;
        }
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
    mut contextos: EguiContexts,
) {
    if !botoes.just_released(MouseButton::Left) || arrasto.moveu > 4.0 || !arrasto.ativo {
        return;
    }
    if janela.single().is_ok_and(|j| ponteiro_sobre_ui(&egui, &mut contextos, j)) {
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
                tf.translation.y = cym * ESCALA_Y_MAPA;
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
        assert_eq!(pixel_do_mundo(Vec2::new(-50.0, 25.0 * ESCALA_Y_MAPA), 100, 50), Some((0, 0)));
        assert_eq!(pixel_do_mundo(Vec2::new(51.0, 0.0), 100, 50), Some((1, 25)));
        assert_eq!(pixel_do_mundo(Vec2::new(-51.0, 0.0), 100, 50), Some((99, 25)));
        assert_eq!(pixel_do_mundo(Vec2::new(0.0, 30.0 * ESCALA_Y_MAPA), 100, 50), None);
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
        let ent = EntradaMapa { caminho_png: raiz.join("assets/map/provinces.png"), cor_prov, _dono_prov: vec![1; n], _estado_prov: estado_prov, _ocupante_cor: vec![None; n] };
        let t = std::time::Instant::now();
        let s = gerar_mapa(&ent).unwrap();
        eprintln!("gerar_mapa: {:?}", t.elapsed());
        assert_eq!(s.ids_gpu.len(), (s.largura * s.altura * 2) as usize);
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

    // ---------------------------------------------------------------------------------------
    // Paleta na GPU: equivalência com o pintor de referência, isolamento e tempo de repintura.
    // ---------------------------------------------------------------------------------------

    /// Transcrição em Rust, linha a linha, das funções de `assets/shaders/terreno.wesl` (sem relevo
    /// nem setores de Berlim), lendo SÓ a textura de IDs e a paleta como o shader faz.
    struct EmulaShader<'a> {
        ids: &'a [u8],
        pal: &'a [u8],
        w: i32,
        h: i32,
        completo: bool,
    }

    impl EmulaShader<'_> {
        fn raw(&self, x: i32, y: i32) -> (u32, u32) {
            let i = (y * self.w + x.rem_euclid(self.w)) as usize * 2;
            (self.ids[i] as u32, self.ids[i + 1] as u32)
        }
        fn id_de(a: u32, b: u32) -> u32 {
            if b & 128 != 0 { 0 } else { a | ((b & 31) << 8) }
        }
        fn pid(&self, x: i32, y: i32) -> i32 {
            if y < 0 || y >= self.h {
                return -1;
            }
            let (a, b) = self.raw(x, y);
            Self::id_de(a, b) as i32
        }
        fn pal(&self, id: u32, linha: usize) -> [u8; 4] {
            let (c, l) = (id as usize % PALETA_LARGURA, (id as usize / PALETA_LARGURA) * 4 + linha);
            let o = (l * PALETA_LARGURA + c) * 4;
            [self.pal[o], self.pal[o + 1], self.pal[o + 2], self.pal[o + 3]]
        }
        fn dono_estado(&self, id: u32) -> (u32, u32) {
            let p = self.pal(id, 2);
            (p[0] as u32 | (p[1] as u32) << 8, p[2] as u32 | (p[3] as u32) << 8)
        }
        fn tipo(&self, a: u32, b: u32) -> u32 {
            let (da, db) = (self.dono_estado(a), self.dono_estado(b));
            if b == 0 || db.0 != da.0 {
                3
            } else if db.1 != da.1 {
                2
            } else {
                1
            }
        }
        fn borda(&self, x: i32, y: i32) -> u32 {
            let id = self.pid(x, y) as u32;
            let mut m = 0;
            let dir = self.pid(x + 1, y) as u32;
            if dir != id {
                m = m.max(self.tipo(id, dir));
            }
            let esq = self.pid(x - 1, y) as u32;
            if esq != id {
                m = m.max(self.tipo(esq, id));
            }
            let bai = self.pid(x, y + 1);
            if bai >= 0 && bai as u32 != id {
                m = m.max(self.tipo(id, bai as u32));
            }
            let cim = self.pid(x, y - 1);
            if cim >= 0 && cim as u32 != id {
                m = m.max(self.tipo(cim as u32, id));
            }
            m
        }
        fn borda_pais(&self, x: i32, y: i32) -> bool {
            y >= 0 && y < self.h && self.borda(x, y) == 3
        }
        fn anel1(&self, x: i32, y: i32) -> bool {
            if y < 0 || y >= self.h || self.pid(x, y) == 0 || self.borda(x, y) == 3 {
                return false;
            }
            self.borda_pais(x + 1, y) || self.borda_pais(x - 1, y) || self.borda_pais(x, y + 1) || self.borda_pais(x, y - 1)
        }
        fn anel2(&self, x: i32, y: i32) -> bool {
            if self.pid(x, y) == 0 || self.borda(x, y) == 3 || self.anel1(x, y) {
                return false;
            }
            self.anel1(x + 1, y) || self.anel1(x - 1, y) || self.anel1(x, y + 1) || self.anel1(x, y - 1)
        }
        fn cor(&self, x: i32, y: i32) -> [u8; 4] {
            let (r, g) = self.raw(x, y);
            let id = Self::id_de(r, g);
            let perto = if self.completo { (g >> 5) & 1 == 1 } else { (g >> 6) & 1 == 1 };
            let costa = if id == 0 { r } else { 0 };
            let mut c = [0f32; 3];
            let mut alfa = 255.0f32;
            if id == 0 {
                let (fx, fy) = (x as f32, y as f32);
                let lat = ((fy / self.h as f32) - 0.5).abs() * 2.0;
                let ruido = if self.completo { (fx * 0.021 + fy * 0.013).sin() * 0.5 + (fx * 0.047 - fy * 0.031).sin() * 0.3 + (fx * 0.09 + fy * 0.07).sin() * 0.2 } else { 0.0 };
                let f = 1.0 + (0.22 - lat * 0.40) + ruido * 0.045;
                for k in 0..3 {
                    c[k] = (COR_OCEANO_FUNDO[k] as f32 * f).clamp(0.0, 255.0);
                }
                if costa > 0 {
                    let t = [0.55, 0.30, 0.14][costa as usize - 1];
                    for k in 0..3 {
                        c[k] += (COR_COSTA[k] as f32 - c[k]) * t;
                    }
                }
            } else {
                let pc = self.pal(id, 0);
                c = [pc[0] as f32, pc[1] as f32, pc[2] as f32];
                alfa = 255.0;
                if pc[3] < 255 && (x + y) % 8 < 4 {
                    let oc = self.pal(id, 1);
                    c = [oc[0] as f32, oc[1] as f32, oc[2] as f32];
                }
            }
            if perto {
                let b = self.borda(x, y);
                if b == 3 {
                    c = [15.0; 3];
                    alfa = 255.0;
                } else if b > 0 {
                    let f = if b == 2 { 0.72 } else { 0.92 };
                    c = c.map(|v| (v * f).floor());
                } else if !self.completo {
                } else if self.anel1(x, y) {
                    c = c.map(|v| v + (255.0 - v) * 0.10);
                } else if self.anel2(x, y) {
                    c = c.map(|v| v + (255.0 - v) * 0.04);
                }
            }
            [c[0] as u8, c[1] as u8, c[2] as u8, alfa as u8]
        }
    }

    fn comparar_emulacao_com_referencia(ids: &[u32], w: u32, h: u32, cor_prov: &[[u8; 4]], dono: &[u32], estado: &[u32], ocup: &[Option<[u8; 4]>]) {
        let (ref_px, _) = colorir_com_hachura(ids, w, h, cor_prov, dono, estado, Some(ocup));
        let ids_gpu = preparar_ids_gpu(ids, w, h);
        let pal = bytes_paleta(cor_prov.len(), cor_prov, dono, estado, Some(ocup));
        let emu = EmulaShader { ids: &ids_gpu, pal: &pal, w: w as i32, h: h as i32, completo: true };
        let barato = EmulaShader { ids: &ids_gpu, pal: &pal, w: w as i32, h: h as i32, completo: false };
        let mut piores = 0i32;
        let mut com_borda = 0usize;
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let e = emu.cor(x, y);
                let o = (y as usize * w as usize + x as usize) * 4;
                let r = &ref_px[o..o + 4];
                if r[..3] == [15, 15, 15] {
                    com_borda += 1;
                }
                // Modo barato (Baixo/Médio): fronteiras idênticas; sem anéis de brilho nem ruído do
                // oceano, o resto só pode diferir por esses efeitos (<= ~10% de clareamento).
                let b = barato.cor(x, y);
                let na_borda = emu.borda(x, y) > 0;
                for k in 0..3 {
                    let d = (b[k] as i32 - r[k] as i32).abs();
                    assert!(d <= if na_borda { 3 } else { 26 }, "modo barato ({x},{y}) canal {k}: {b:?} x {r:?}");
                }
                for k in 0..4 {
                    let d = (e[k] as i32 - r[k] as i32).abs();
                    piores = piores.max(d);
                    assert!(d <= 3, "pixel ({x},{y}) canal {k}: emulado {e:?} x referência {r:?}");
                }
            }
        }
        assert!(com_borda > 0, "o recorte precisa ter fronteiras de país");
        eprintln!("emulação x referência: pior diferença por canal = {piores}");
    }

    #[test]
    fn shader_emulado_igual_ao_pintor_de_referencia_sintetico() {
        // Mapa pequeno com wrap, oceano, costa, países, estados e ocupação.
        let (w, h) = (48u32, 30u32);
        let mut ids = vec![0u32; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                // blocos de 8x6 com ids 1..: ruído de oceano em alguns blocos
                let bloco = (x / 8) + (y / 6) * 6;
                ids[(y * w + x) as usize] = if bloco % 5 == 0 { 0 } else { bloco + 1 };
            }
        }
        let n = 64;
        let cor: Vec<[u8; 4]> = (0..n).map(|i| [(i * 37 % 256) as u8, (i * 91 % 256) as u8, (i * 53 % 256) as u8, 255]).collect();
        let dono: Vec<u32> = (0..n as u32).map(|i| (i / 3) % 4 + u32::from(i > 0 && i % 7 != 0)).collect();
        let estado: Vec<u32> = (0..n as u32).map(|i| i / 2).collect();
        let ocup: Vec<Option<[u8; 4]>> = (0..n).map(|i| (i % 4 == 2).then_some([250, 10, 10, 255])).collect();
        comparar_emulacao_com_referencia(&ids, w, h, &cor, &dono, &estado, &ocup);
    }

    #[test]
    fn shader_emulado_igual_ao_pintor_de_referencia_no_mapa_real() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let img = image::open(raiz.join("assets/map/provinces.png")).unwrap().to_rgb8();
        let (w, h) = img.dimensions();
        // Recorte de 520x340 em torno da Europa/Mediterrâneo (costa, fronteiras e ilhas).
        let (x0, y0, cw, ch) = (w / 2 - 60, h / 4, 520u32, 340u32);
        let mut ids = Vec::new();
        for y in y0..y0 + ch {
            for x in x0..x0 + cw {
                let p = img.get_pixel(x, y);
                ids.push(id_de_cor_provincia((p[0], p[1], p[2])).unwrap_or(0));
            }
        }
        let n = ids.iter().copied().max().unwrap() as usize + 1;
        let cor: Vec<[u8; 4]> = (0..n).map(|i| [(i * 37 % 256) as u8, (i * 91 % 256) as u8, (i * 53 % 256) as u8, 255]).collect();
        let dono: Vec<u32> = (0..n as u32).map(|i| (i / 9) % 11 + 1).collect();
        let estado: Vec<u32> = (0..n as u32).map(|i| i / 3 + 1).collect();
        let ocup: Vec<Option<[u8; 4]>> = (0..n).map(|i| (i % 13 == 0).then_some([30, 200, 60, 255])).collect();
        comparar_emulacao_com_referencia(&ids, cw, ch, &cor, &dono, &estado, &ocup);
    }

    #[test]
    fn preparar_ids_gpu_preserva_o_id_de_cada_pixel_para_a_selecao() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let img = image::open(raiz.join("assets/map/provinces.png")).unwrap().to_rgb8();
        let (w, h) = img.dimensions();
        let ids: Vec<u32> = img.pixels().map(|p| id_de_cor_provincia((p[0], p[1], p[2])).unwrap_or(0)).collect();
        assert!(ids.iter().copied().max().unwrap() <= ID_MAX_GPU);
        let t = std::time::Instant::now();
        let gpu = preparar_ids_gpu(&ids, w, h);
        eprintln!("preparar_ids_gpu (uma vez, no carregamento): {:?}", t.elapsed());
        for (i, &id) in ids.iter().enumerate() {
            let dec = if gpu[i * 2 + 1] & 128 != 0 { 0 } else { gpu[i * 2] as u32 | ((gpu[i * 2 + 1] as u32 & 31) << 8) };
            assert_eq!(dec, id, "pixel {i}");
        }
        // A seleção por clique continua usando `DadosMapa.ids` + `pixel_do_mundo`: mesma província
        // que a textura de IDs mostra naquele pixel.
        for (px, py) in [(0.0, 0.0), (12.0, -40.0), (-900.0, 250.0), (1500.0, -300.0)] {
            let pos = Vec2::new(px, py * ESCALA_Y_MAPA);
            let (x, y) = pixel_do_mundo(pos, w, h).unwrap();
            let i = y as usize * w as usize + x as usize;
            let da_gpu = if gpu[i * 2 + 1] & 128 != 0 { 0 } else { gpu[i * 2] as u32 | ((gpu[i * 2 + 1] as u32 & 31) << 8) };
            assert_eq!(ids[i], da_gpu);
        }
        let caixas = calcular_caixas(&ids, w, ids.iter().copied().max().unwrap() as usize + 1);
        assert!(caixas.iter().flatten().count() > 6000);
    }

    #[test]
    fn paleta_muda_a_cor_de_uma_provincia_sem_tocar_nas_outras() {
        let n = 7741;
        let cor: Vec<[u8; 4]> = (0..n).map(|i| [(i % 251) as u8, (i % 241) as u8, (i % 239) as u8, 255]).collect();
        let dono: Vec<u32> = (0..n as u32).map(|i| i % 190).collect();
        let estado: Vec<u32> = (0..n as u32).map(|i| i / 5).collect();
        let antes = bytes_paleta(n, &cor, &dono, &estado, None);
        let mut cor2 = cor.clone();
        cor2[5000] = [1, 2, 3, 255];
        let depois = bytes_paleta(n, &cor2, &dono, &estado, None);
        let diferentes: Vec<usize> = (0..antes.len() / 4).filter(|&i| antes[i * 4..i * 4 + 4] != depois[i * 4..i * 4 + 4]).collect();
        // Uma única célula: id 5000 -> coluna 5000 % 4096, linha (5000 / 4096) * 4 + 0.
        assert_eq!(diferentes, vec![4 * PALETA_LARGURA + 5000 % PALETA_LARGURA]);
        let o = diferentes[0] * 4;
        assert_eq!(&depois[o..o + 4], &[1, 2, 3, 255]);
        // Dono/estado e ocupante se codificam em células próprias e não vazam para a cor.
        let mut ocup = vec![None; n];
        ocup[17] = Some([9, 8, 7, 255]);
        let com_ocup = bytes_paleta(n, &cor, &dono, &estado, Some(&ocup));
        let dif: Vec<usize> = (0..antes.len() / 4).filter(|&i| antes[i * 4..i * 4 + 4] != com_ocup[i * 4..i * 4 + 4]).collect();
        assert_eq!(dif, vec![17, PALETA_LARGURA + 17]); // alfa 254 na linha 0 + cor do ocupante na linha 1
    }

    #[test]
    fn repintar_a_paleta_leva_microssegundos_e_nao_mexe_na_textura_de_ids() {
        // 11,5 M pixels de IDs ficam fixos; repintar = só a paleta de ~7,7 mil províncias.
        let n = 7741;
        let cor: Vec<[u8; 4]> = (0..n).map(|i| [(i % 251) as u8, 10, 20, 255]).collect();
        let dono: Vec<u32> = vec![1; n];
        let estado: Vec<u32> = vec![2; n];
        let ocup = vec![None; n];
        let mut imagens = Assets::<Image>::default();
        let handle = imagens.add(Image::new(
            Extent3d { width: PALETA_LARGURA as u32, height: altura_paleta(n) as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            bytes_paleta(n, &cor, &dono, &estado, None),
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::RENDER_WORLD,
        ));
        let paleta = ImagemMapa(handle.clone());
        let mut melhor = std::time::Duration::MAX;
        for k in 0..20u8 {
            let cor_k: Vec<[u8; 4]> = cor.iter().map(|c| [c[0].wrapping_add(k), c[1], c[2], 255]).collect();
            let t = std::time::Instant::now();
            pintar_paleta(&mut imagens, &paleta, &cor_k, &dono, &estado, Some(&ocup));
            melhor = melhor.min(t.elapsed());
        }
        eprintln!("repintura via paleta: {melhor:?} (antes: ~600-690 ms)");
        assert!(melhor.as_millis() < 5, "repintura da paleta levou {melhor:?}");
        let img = imagens.get(&handle).unwrap();
        assert_eq!(img.data.as_ref().unwrap().len(), PALETA_LARGURA * altura_paleta(n) * 4);
    }
}
