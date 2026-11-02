//! Cidades v3 (estilo Victoria 3): marcadores pequenos, rótulos sem colisão com fonte pt-BR,
//! fundação/renomeação por ano e detalhes animados (prédios por nível/era/bloco, luzes à noite,
//! fumaça, guindastes no porto e tráfego urbano).
//!
//! Tudo aqui é aditivo: `cidade.rs` só cria a entidade, o marcador e o rótulo; este módulo cuida
//! do restante. Os detalhes são criados sob demanda (só no zoom próximo e dentro da tela).

use crate::cidade::{CidadeVisual, RotuloNomeCidade};
use crate::config::{CamadaConstrucoesVisivel, VisualMapaConfig};
use crate::entrada::{AnoVisual, BlocoVisual};
use crate::era::GerenciadorEras;
use crate::lod::{DetalheLod, EstadoLod, MarcadorLod, NivelLod};
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use std::collections::HashMap;

/// Tamanho-base (px) com que o texto é rasterizado; a escala do `Transform` faz o resto.
pub const FONTE_BASE_PX: f32 = 16.0;
/// Máximo de rótulos exibidos ao mesmo tempo.
pub const MAX_ROTULOS: usize = 260;
/// Máximo de partículas de fumaça urbana.
pub const MAX_FUMACA_URBANA: usize = 350;
/// Máximo de cidades detalhadas criadas por quadro.
const DETALHES_POR_QUADRO: usize = 6;

// ---------------------------------------------------------------------------------------------
// Funções puras (testáveis)
// ---------------------------------------------------------------------------------------------

/// A cidade já existe no ano dado?
pub fn existe_no_ano(ano_fundacao: Option<u16>, ano: f32) -> bool {
    ano_fundacao.map_or(true, |f| ano >= f as f32)
}

/// Nome da cidade no ano dado, aplicando a última troca de nome já ocorrida.
pub fn nome_no_ano(base: &str, mudancas: &[(u16, String)], ano: f32) -> String {
    mudancas
        .iter()
        .filter(|(a, _)| (*a as f32) <= ano)
        .max_by_key(|(a, _)| *a)
        .map_or_else(|| base.to_string(), |(_, n)| n.clone())
}

/// Raio do marcador em pixels de TELA, por nível (1..=8).
pub fn raio_marcador_px(nivel: u8, capital: bool) -> f32 {
    let r = 1.3 + 0.42 * nivel as f32;
    if capital { r + 0.7 } else { r }
}

/// Tamanho do texto em pixels de tela conforme o zoom (menor no zoom próximo).
pub fn tamanho_rotulo_px(escala_camera: f32) -> f32 {
    (11.5 * escala_camera.max(0.05).powf(0.18)).clamp(7.0, 12.0)
}

/// Nível mínimo para a cidade ter rótulo no zoom dado (a colisão filtra o resto).
pub fn nivel_min_rotulo(escala_camera: f32) -> u8 {
    if escala_camera >= 3.0 {
        7
    } else if escala_camera >= 2.0 {
        6
    } else if escala_camera >= 1.2 {
        4
    } else if escala_camera >= 0.7 {
        2
    } else {
        1
    }
}

/// Retângulo em pixels de tela (centro + meia dimensão).
#[derive(Debug, Clone, Copy)]
pub struct RetTela {
    pub cx: f32,
    pub cy: f32,
    pub hw: f32,
    pub hh: f32,
}

impl RetTela {
    pub fn colide(&self, o: &RetTela) -> bool {
        (self.cx - o.cx).abs() < self.hw + o.hw && (self.cy - o.cy).abs() < self.hh + o.hh
    }
}

/// Aceita retângulos por ordem de prioridade (já ordenados) descartando os que colidem.
/// Devolve os índices aceitos. Usa grade espacial para ficar rápido.
pub fn escolher_sem_colisao(candidatos: &[RetTela], maximo: usize) -> Vec<usize> {
    const CELULA: f32 = 80.0;
    let mut grade: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    let mut aceitos = Vec::new();
    'cand: for (i, r) in candidatos.iter().enumerate() {
        if aceitos.len() >= maximo {
            break;
        }
        let x0 = ((r.cx - r.hw) / CELULA).floor() as i32;
        let x1 = ((r.cx + r.hw) / CELULA).floor() as i32;
        let y0 = ((r.cy - r.hh) / CELULA).floor() as i32;
        let y1 = ((r.cy + r.hh) / CELULA).floor() as i32;
        for gx in x0..=x1 {
            for gy in y0..=y1 {
                if let Some(v) = grade.get(&(gx, gy)) {
                    if v.iter().any(|&j| candidatos[j].colide(r)) {
                        continue 'cand;
                    }
                }
            }
        }
        for gx in x0..=x1 {
            for gy in y0..=y1 {
                grade.entry((gx, gy)).or_default().push(i);
            }
        }
        aceitos.push(i);
    }
    aceitos
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoPredio {
    Casa,
    Medio,
    Bloco,
    Torre,
    ArranhaCeu,
}

/// Prédio gerado deterministicamente (offset local em unidades de mundo).
#[derive(Debug, Clone, Copy)]
pub struct LayoutPredio {
    pub offset: Vec2,
    pub tamanho: Vec2,
    pub nivel_req: u8,
    pub tipo: TipoPredio,
    pub semente: f32,
    pub chamine: bool,
}

struct Lcg(u64);
impl Lcg {
    fn novo(id: &str, extra: u64) -> Self {
        Self(id.bytes().fold(1945u64 ^ extra, |a, b| {
            a.wrapping_mul(31).wrapping_add(b as u64)
        }))
    }
    fn f(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as u32 as f32) / (u32::MAX >> 1) as f32
    }
}

/// Quantos prédios surgem em cada nível.
fn predios_no_nivel(nivel: u8) -> u32 {
    [3, 3, 4, 5, 6, 7, 8, 9][(nivel.clamp(1, 8) - 1) as usize]
}

/// Raio (mundo) da mancha urbana por nível.
pub fn raio_urbano(nivel: u8) -> f32 {
    1.6 + 1.05 * nivel.clamp(1, 8) as f32
}

/// Diâmetro de referência da mancha urbana detalhada, em unidades de mundo.
///
/// Inclui ruas e o anel de prédios além do disco-base. A escala da raiz é ajustada
/// por zoom para que esse diâmetro nunca ultrapasse o teto em pixels de tela.
pub fn diametro_detalhe_mundo(nivel: u8) -> f32 {
    raio_urbano(nivel.saturating_add(2).min(8)) * 3.0
}

/// Escala da raiz dos detalhes, limitada ao tamanho configurado para a tela.
pub fn escala_detalhe_cidade(
    nivel: u8,
    escala_camera: f32,
    estilo: &crate::estilo::EstiloMapa,
) -> f32 {
    (estilo.cidades_detalhe_max_px.max(1.0) * escala_camera.max(0.01)
        / diametro_detalhe_mundo(nivel))
    .min(1.0)
}

/// Gera os prédios dos níveis `1..=ate_nivel`. Varia por bloco e era (0..=4).
pub fn gerar_layout(id: &str, ate_nivel: u8, bloco: BlocoVisual, era: usize) -> Vec<LayoutPredio> {
    let mut rng = Lcg::novo(id, 7);
    let mut v = Vec::new();
    for nivel in 1..=ate_nivel.min(8) {
        for _ in 0..predios_no_nivel(nivel) {
            let r =
                raio_urbano(nivel) * (0.35 + 0.65 * rng.f()) + if nivel > 1 { 0.8 } else { 0.0 };
            let a = rng.f() * std::f32::consts::TAU;
            let offset = Vec2::new(r * a.cos() * 1.15, r * a.sin() * 0.62);
            // Centro mais alto: tipo cresce com (nivel, proximidade do centro).
            let centro = 1.0 - (r / raio_urbano(nivel.max(2))).min(1.0);
            let nota = nivel as f32 + centro * 2.5 + rng.f() * 1.5 + era as f32 * 0.35;
            let tipo = if nota >= 10.5 && bloco != BlocoVisual::TerceiroMundo || nota >= 12.5 {
                TipoPredio::ArranhaCeu
            } else if nota >= 8.0 {
                TipoPredio::Torre
            } else if nota >= 5.0 {
                TipoPredio::Bloco
            } else if nota >= 3.0 {
                TipoPredio::Medio
            } else {
                TipoPredio::Casa
            };
            let s = rng.f();
            let (mut w, mut h) = match tipo {
                TipoPredio::Casa => (1.0 + s * 0.6, 0.9 + s * 0.6),
                TipoPredio::Medio => (1.3 + s * 0.6, 1.9 + s * 1.4),
                TipoPredio::Bloco => (1.9 + s * 0.9, 2.8 + s * 1.8),
                TipoPredio::Torre => (1.5 + s * 0.6, 5.0 + s * 2.6),
                TipoPredio::ArranhaCeu => (1.4 + s * 0.5, 7.5 + s * 4.5),
            };
            match bloco {
                BlocoVisual::Sovietica => {
                    w *= 1.3;
                    h = h.min(8.0) * 0.9;
                }
                BlocoVisual::TerceiroMundo => h *= 0.75,
                BlocoVisual::Ocidental => {}
            }
            let industrial = era <= 2 && matches!(tipo, TipoPredio::Bloco | TipoPredio::Medio);
            let chamine = industrial
                && rng.f()
                    < if bloco == BlocoVisual::Ocidental {
                        0.20
                    } else {
                        0.28
                    };
            v.push(LayoutPredio {
                offset,
                tamanho: Vec2::new(w, h),
                nivel_req: nivel,
                tipo,
                semente: rng.f(),
                chamine,
            });
        }
    }
    v
}

/// Cor da fachada por bloco, era e tipo (variação pela semente).
pub fn cor_fachada(bloco: BlocoVisual, era: usize, tipo: TipoPredio, semente: f32) -> Color {
    let t = semente;
    let e = era.min(4) as f32 / 4.0;
    let (r, g, b) = match bloco {
        BlocoVisual::Ocidental => match tipo {
            TipoPredio::Casa => (0.66 + 0.10 * t, 0.46 + 0.08 * t, 0.38), // tijolo
            TipoPredio::Medio | TipoPredio::Bloco => {
                (0.62 + 0.12 * t, 0.58 + 0.10 * t, 0.52 + 0.08 * t)
            } // pedra
            // vidro azulado conforme a era avança
            _ => (0.60 - 0.18 * e, 0.62 - 0.04 * e + 0.06 * t, 0.66 + 0.14 * e),
        },
        BlocoVisual::Sovietica => match tipo {
            TipoPredio::Casa => (0.62 + 0.08 * t, 0.58, 0.50),
            // painéis de concreto em tons pastel
            TipoPredio::Medio | TipoPredio::Bloco => (
                0.72 + 0.08 * t,
                0.70 + 0.06 * t - 0.1 * e,
                0.60 + 0.05 * (1.0 - t),
            ),
            _ => (0.66 + 0.06 * t, 0.64, 0.56), // torres "bolo de casamento"
        },
        BlocoVisual::TerceiroMundo => match tipo {
            TipoPredio::Casa | TipoPredio::Medio => {
                // terracota, cal e pastéis
                let k = (t * 4.0) as u32;
                match k {
                    0 => (0.80, 0.50, 0.36),
                    1 => (0.90, 0.86, 0.76),
                    2 => (0.78, 0.66, 0.40),
                    _ => (0.62, 0.74, 0.70),
                }
            }
            _ => (0.70 + 0.10 * t, 0.68, 0.64 - 0.1 * e),
        },
    };
    Color::srgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

fn cor_telhado(c: Color, bloco: BlocoVisual, tipo: TipoPredio) -> Color {
    let s = c.to_srgba();
    if matches!(tipo, TipoPredio::Casa) && bloco != BlocoVisual::Sovietica {
        return Color::srgb(0.55, 0.28, 0.22); // telha
    }
    Color::srgb(
        (s.red * 0.72).min(1.0),
        (s.green * 0.72).min(1.0),
        (s.blue * 0.74).min(1.0),
    )
}

// ---------------------------------------------------------------------------------------------
// Componentes e recursos
// ---------------------------------------------------------------------------------------------

/// Fonte usada nos rótulos (Barlow SemiBold: cobre Latin-1, ou seja, acentos pt-BR).
#[derive(Resource, Clone)]
pub struct FonteRotulos(pub Handle<Font>);

/// Sombra do texto do rótulo (filha do rótulo).
#[derive(Component)]
pub struct SombraRotulo;

/// Texturas de janelas (dia/noite) geradas proceduralmente.
#[derive(Resource)]
pub struct TexturasJanelas {
    pub dia: [Handle<Image>; 3],
    pub noite: [Handle<Image>; 3],
    pub base_cidade: Handle<Mesh>,
    pub mat_base: Handle<ColorMaterial>,
}

/// Marca que a cidade já tem a raiz de detalhes (com nível/era geradas).
#[derive(Component)]
pub struct DetalhesGerados {
    pub raiz: Entity,
    pub nivel: u8,
    pub era: usize,
    pub base: Entity,
}

#[derive(Component)]
pub struct RaizDetalhes;

#[derive(Component)]
pub struct PredioUrbano {
    pub nivel_req: u8,
    pub crescimento: f32,
    pub alvo: f32,
}

#[derive(Component)]
pub struct JanelaPredio {
    pub variante: usize,
}

#[derive(Component)]
pub struct Chamine {
    pub acumulador: f32,
    pub ritmo: f32,
}

#[derive(Component)]
pub struct FumacaUrbana {
    pub idade: f32,
    pub vida: f32,
    pub vel: Vec2,
    pub tam0: f32,
}

#[derive(Component)]
pub struct JibGuindaste {
    pub fase: f32,
    pub vel: f32,
    pub comprimento: f32,
}

#[derive(Component)]
pub struct CargaGuindaste {
    pub fase: f32,
    pub vel: f32,
    pub comprimento: f32,
}

#[derive(Component)]
pub struct Veiculo {
    pub a: Vec2,
    pub b: Vec2,
    pub fase: f32,
    pub vel: f32,
}

// ---------------------------------------------------------------------------------------------
// Startup: fonte e texturas
// ---------------------------------------------------------------------------------------------

pub fn carregar_fonte_rotulos_sistema(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    fontes: Option<Res<Assets<Font>>>,
) {
    if let (Some(a), Some(_)) = (assets, fontes) {
        commands.insert_resource(FonteRotulos(a.load("fonts/Barlow-SemiBold.ttf")));
    }
}

fn imagem_janelas(cols: u32, linhas: u32, cor: [u8; 4], semente: u64, acesas: f32) -> Image {
    let (w, h) = (cols * 2 + 1, linhas * 2 + 1);
    let mut dados = vec![0u8; (w * h * 4) as usize];
    let mut rng = Lcg(semente);
    for y in 0..h {
        for x in 0..w {
            if x % 2 == 1 && y % 2 == 1 && rng.f() < acesas {
                let i = ((y * w + x) * 4) as usize;
                dados[i..i + 4].copy_from_slice(&cor);
            }
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        dados,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    img.sampler = ImageSampler::nearest();
    img
}

pub fn criar_texturas_janelas_sistema(
    mut commands: Commands,
    imagens: Option<ResMut<Assets<Image>>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materiais: Option<ResMut<Assets<ColorMaterial>>>,
) {
    let (Some(mut imagens), Some(mut meshes), Some(mut materiais)) = (imagens, meshes, materiais)
    else {
        return;
    };
    let formas = [(2, 2), (3, 5), (3, 9)];
    let dia = std::array::from_fn(|i| {
        imagens.add(imagem_janelas(
            formas[i].0,
            formas[i].1,
            [40, 55, 80, 150],
            11 + i as u64,
            1.0,
        ))
    });
    let noite = std::array::from_fn(|i| {
        imagens.add(imagem_janelas(
            formas[i].0,
            formas[i].1,
            [255, 214, 120, 255],
            91 + i as u64,
            0.62,
        ))
    });
    commands.insert_resource(TexturasJanelas {
        dia,
        noite,
        base_cidade: meshes.add(Ellipse::new(0.5, 0.32)),
        mat_base: materiais.add(ColorMaterial::from_color(Color::srgba(
            0.55, 0.50, 0.40, 0.38,
        ))),
    });
}

// ---------------------------------------------------------------------------------------------
// Fundação, renomeação e tecla de noite
// ---------------------------------------------------------------------------------------------

/// Esconde cidades ainda não fundadas e aplica trocas de nome históricas.
pub fn fundacao_nome_sistema(
    ano: Res<AnoVisual>,
    mut vistas: Local<usize>,
    mut cidades: Query<(Entity, &mut CidadeVisual, &mut Visibility)>,
    mut rotulos: Query<(&ChildOf, &mut Text2d), (With<RotuloNomeCidade>, Without<SombraRotulo>)>,
    mut sombras: Query<(&ChildOf, &mut Text2d), (With<SombraRotulo>, Without<RotuloNomeCidade>)>,
    pais_dos_rotulos: Query<&ChildOf, With<RotuloNomeCidade>>,
) {
    let total = cidades.iter().count();
    let novas = total != *vistas;
    *vistas = total;
    if !ano.is_changed() && !novas {
        return;
    }
    let mut renomeadas: HashMap<Entity, String> = HashMap::new();
    for (ent, mut c, mut vis) in &mut cidades {
        let existe = existe_no_ano(c.ano_fundacao, ano.0);
        let alvo = if existe {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != alvo {
            *vis = alvo;
        }
        if c.existe != existe {
            c.existe = existe;
        }
        let nome = nome_no_ano(&c.nome_base, &c.mudancas_nome, ano.0);
        if nome != c.nome {
            renomeadas.insert(ent, nome.clone());
            c.nome = nome;
        } else if novas {
            renomeadas.insert(ent, c.nome.clone());
        }
    }
    if renomeadas.is_empty() {
        return;
    }
    for (pai, mut t) in &mut rotulos {
        if let Some(n) = renomeadas.get(&pai.parent()) {
            if t.0 != *n {
                t.0 = n.clone();
            }
        }
    }
    for (pai, mut t) in &mut sombras {
        if let Some(cidade) = pais_dos_rotulos.get(pai.parent()).ok().map(|c| c.parent()) {
            if let Some(n) = renomeadas.get(&cidade) {
                if t.0 != *n {
                    t.0 = n.clone();
                }
            }
        }
    }
}

/// Tecla N alterna dia/noite (luzes das cidades).
pub fn alternar_noite_sistema(
    teclas: Option<Res<ButtonInput<KeyCode>>>,
    mut eras: ResMut<GerenciadorEras>,
) {
    if let Some(t) = teclas {
        if t.just_pressed(KeyCode::KeyN) {
            eras.modo_noturno = !eras.modo_noturno;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Marcadores e rótulos
// ---------------------------------------------------------------------------------------------

fn escala_da_camera(q: &Query<(&Transform, &Projection), With<Camera2d>>) -> Option<(Vec2, f32)> {
    let (tf, proj) = q.single().ok()?;
    let s = match proj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    Some((tf.translation.truncate(), s))
}

/// Mantém o tamanho dos marcadores pequeno e proporcional ao nível, independente do zoom.
pub fn ajustar_marcadores_sistema(
    cfg: Res<VisualMapaConfig>,
    camera: Query<(&Transform, &Projection), With<Camera2d>>,
    cidades: Query<&CidadeVisual>,
    mut marcadores: Query<(&ChildOf, &mut Transform), (With<MarcadorLod>, Without<Camera2d>)>,
) {
    let Some((_, escala)) = escala_da_camera(&camera) else {
        return;
    };
    let raio_mesh = 2.5 * cfg.escala_sprites;
    for (pai, mut tr) in &mut marcadores {
        let Ok(c) = cidades.get(pai.parent()) else {
            continue;
        };
        let px = raio_marcador_px(c.nivel_atual_inteiro, c.eh_capital);
        let s = px * escala / raio_mesh;
        if (tr.scale.x - s).abs() > 1e-4 {
            tr.scale = Vec3::new(s, s, 1.0);
        }
    }
}

/// Posiciona os rótulos com tamanho dependente do zoom e esconde os que colidem.
pub fn layout_rotulos_sistema(
    time: Res<Time>,
    mut acumulado: Local<f32>,
    camada: Option<Res<CamadaConstrucoesVisivel>>,
    lod: Res<EstadoLod>,
    estilo: Res<crate::estilo::EstiloMapa>,
    camera: Query<(&Transform, &Projection), With<Camera2d>>,
    janela: Query<&Window, With<PrimaryWindow>>,
    cidades: Query<&CidadeVisual>,
    mut rotulos: Query<
        (Entity, &ChildOf, &mut Visibility, &mut Transform),
        (With<RotuloNomeCidade>, Without<Camera2d>),
    >,
) {
    *acumulado += time.delta_secs();
    if *acumulado < 0.12 {
        return;
    }
    *acumulado = 0.0;
    let Some((centro, escala)) = escala_da_camera(&camera) else {
        return;
    };
    let visivel = camada.map_or(true, |c| c.0) && escala < estilo.cidades_escala_max;
    let (largura, altura) = janela
        .single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or((1600.0, 900.0));
    let px = tamanho_rotulo_px(escala);
    let nivel_min = nivel_min_rotulo(escala);
    let margem = 80.0;
    let global = lod.nivel_atual == NivelLod::Global;

    // (prioridade, rótulo, retângulo em tela, deslocamento vertical em tela)
    let mut cands: Vec<(f32, Entity, RetTela, f32)> = Vec::new();
    for (ent, pai, _, _) in rotulos.iter() {
        let Ok(c) = cidades.get(pai.parent()) else {
            continue;
        };
        if !visivel || !c.existe || c.nivel_atual_inteiro < nivel_min && !(global && c.eh_capital) {
            continue;
        }
        let rel = (c.posicao_mundo - centro) / escala;
        if rel.x.abs() > largura * 0.5 + margem || rel.y.abs() > altura * 0.5 + margem {
            continue;
        }
        let raio = raio_marcador_px(c.nivel_atual_inteiro, c.eh_capital);
        let desloc = raio + px * 0.62;
        let largura_txt = c.nome.chars().count() as f32 * px * 0.52 + 4.0;
        let ret = RetTela {
            cx: rel.x,
            cy: rel.y - desloc,
            hw: largura_txt * 0.5,
            hh: px * 0.6,
        };
        let prio = c.populacao as f32 * if c.eh_capital { 1.4 } else { 1.0 }
            + c.nivel_atual_inteiro as f32 * 1000.0;
        cands.push((prio, ent, ret, desloc));
    }
    cands.sort_by(|a, b| b.0.total_cmp(&a.0));
    let rets: Vec<RetTela> = cands.iter().map(|c| c.2).collect();
    let aceitos = escolher_sem_colisao(&rets, MAX_ROTULOS);
    let mut mostrar: HashMap<Entity, f32> = HashMap::new();
    for i in aceitos {
        mostrar.insert(cands[i].1, cands[i].3);
    }
    let esc_texto = px * escala / FONTE_BASE_PX;
    for (ent, _, mut vis, mut tr) in &mut rotulos {
        if let Some(desloc) = mostrar.get(&ent) {
            if *vis != Visibility::Inherited {
                *vis = Visibility::Inherited;
            }
            let y = -desloc * escala;
            if (tr.scale.x - esc_texto).abs() > 1e-5 || (tr.translation.y - y).abs() > 1e-4 {
                tr.scale = Vec3::new(esc_texto, esc_texto, 1.0);
                tr.translation.y = y;
            }
        } else if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Detalhes: prédios, estradas, tráfego, guindastes
// ---------------------------------------------------------------------------------------------

fn variante_janela(tipo: TipoPredio) -> usize {
    match tipo {
        TipoPredio::Casa | TipoPredio::Medio => 0,
        TipoPredio::Bloco => 1,
        TipoPredio::Torre | TipoPredio::ArranhaCeu => 2,
    }
}

fn spawn_predio(
    p: &mut ChildSpawnerCommands,
    l: &LayoutPredio,
    bloco: BlocoVisual,
    era: usize,
    noite: bool,
    tex: Option<&TexturasJanelas>,
    crescer: bool,
) {
    let cor = cor_fachada(bloco, era, l.tipo, l.semente);
    let z = 1.0 - l.offset.y * 0.02;
    let v = variante_janela(l.tipo);
    let g: f32 = if crescer { 0.0 } else { 1.0 };
    let tinta = if noite { dim(cor, 0.45) } else { cor };
    p.spawn((
        Sprite::from_color(tinta, Vec2::new(l.tamanho.x, l.tamanho.y)),
        Anchor::BOTTOM_CENTER,
        Transform::from_translation(l.offset.extend(z)).with_scale(Vec3::new(
            1.0,
            g.max(0.001),
            1.0,
        )),
        PredioUrbano {
            nivel_req: l.nivel_req,
            crescimento: g,
            alvo: 1.0,
        },
        Visibility::default(),
    ))
    .with_children(|b| {
        // telhado / topo
        let h_telhado = (l.tamanho.y * 0.08).clamp(0.15, 0.5);
        b.spawn((
            Sprite::from_color(
                cor_telhado(cor, bloco, l.tipo),
                Vec2::new(l.tamanho.x * 1.06, h_telhado),
            ),
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(0.0, l.tamanho.y, 0.01),
        ));
        // sombra lateral (profundidade)
        b.spawn((
            Sprite::from_color(
                Color::srgba(0.0, 0.0, 0.05, 0.22),
                Vec2::new(l.tamanho.x * 0.28, l.tamanho.y),
            ),
            Anchor::BOTTOM_RIGHT,
            Transform::from_xyz(l.tamanho.x * 0.5, 0.0, 0.01),
        ));
        // janelas (dia: vidro escuro; noite: luzes acesas)
        if let Some(t) = tex {
            let img = if noite {
                t.noite[v].clone()
            } else {
                t.dia[v].clone()
            };
            let mut s = Sprite::from_image(img);
            s.custom_size = Some(Vec2::new(l.tamanho.x * 0.84, (l.tamanho.y * 0.86).max(0.5)));
            b.spawn((
                s,
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(0.0, l.tamanho.y * 0.05, 0.02),
                JanelaPredio { variante: v },
            ));
        }
        // torre soviética: pináculo
        if bloco == BlocoVisual::Sovietica
            && matches!(l.tipo, TipoPredio::Torre | TipoPredio::ArranhaCeu)
        {
            b.spawn((
                Sprite::from_color(Color::srgb(0.80, 0.72, 0.40), Vec2::new(0.22, 1.6)),
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(0.0, l.tamanho.y + h_telhado, 0.01),
            ));
        }
        // antena em arranha-céus ocidentais
        if bloco == BlocoVisual::Ocidental && l.tipo == TipoPredio::ArranhaCeu {
            b.spawn((
                Sprite::from_color(Color::srgb(0.35, 0.37, 0.42), Vec2::new(0.12, 1.4)),
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(0.0, l.tamanho.y + h_telhado, 0.01),
            ));
        }
        if l.chamine {
            b.spawn((
                Sprite::from_color(Color::srgb(0.30, 0.27, 0.26), Vec2::new(0.36, 1.5)),
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(l.tamanho.x * 0.25, l.tamanho.y, 0.015),
                Chamine {
                    acumulador: l.semente * 3.0,
                    ritmo: 0.35 + l.semente * 0.5,
                },
            ));
        }
    });
}

fn dim(c: Color, f: f32) -> Color {
    let s = c.to_srgba();
    Color::srgb(s.red * f, s.green * f, s.blue * f + 0.04)
}

fn spawn_estradas_e_trafego(p: &mut ChildSpawnerCommands, id: &str, nivel: u8, era: usize) {
    let mut rng = Lcg::novo(id, 99);
    let raio = raio_urbano(nivel.min(8));
    let n_ruas = 2 + nivel as usize / 3;
    for i in 0..n_ruas {
        let ang = (i as f32 / n_ruas as f32) * std::f32::consts::PI + rng.f() * 0.4;
        let dir = Vec2::new(ang.cos() * 1.15, ang.sin() * 0.62).normalize_or_zero();
        let comp = raio * 2.0;
        let a = -dir * comp * 0.5;
        let b = dir * comp * 0.5;
        let centro = (a + b) * 0.5;
        let rot = Quat::from_rotation_z(dir.y.atan2(dir.x));
        p.spawn((
            Sprite::from_color(Color::srgba(0.18, 0.18, 0.20, 0.55), Vec2::new(comp, 0.28)),
            Transform::from_translation(centro.extend(0.2)).with_rotation(rot),
        ));
        let n_carros = 1 + (nivel as usize + 1) / 3;
        for k in 0..n_carros {
            let cores_cedo = [
                Color::srgb(0.1, 0.1, 0.12),
                Color::srgb(0.35, 0.12, 0.10),
                Color::srgb(0.25, 0.28, 0.20),
            ];
            let cores_tarde = [
                Color::srgb(0.85, 0.20, 0.18),
                Color::srgb(0.95, 0.85, 0.30),
                Color::srgb(0.25, 0.45, 0.80),
                Color::srgb(0.9, 0.9, 0.92),
            ];
            let cor = if era <= 1 {
                cores_cedo[((rng.f() * 3.0) as usize).min(2)]
            } else {
                cores_tarde[((rng.f() * 4.0) as usize).min(3)]
            };
            let (aa, bb) = if k % 2 == 0 { (a, b) } else { (b, a) };
            p.spawn((
                Sprite::from_color(cor, Vec2::new(0.55, 0.28)),
                Transform::from_translation(aa.extend(0.5)).with_rotation(rot),
                Veiculo {
                    a: aa,
                    b: bb,
                    fase: rng.f(),
                    vel: 0.05 + rng.f() * 0.07,
                },
            ));
        }
    }
}

fn spawn_guindastes(p: &mut ChildSpawnerCommands, id: &str, nivel: u8) {
    let mut rng = Lcg::novo(id, 5);
    let raio = raio_urbano(nivel.min(8));
    let n = 1 + nivel as usize / 3;
    // cais ao sul da cidade
    p.spawn((
        Sprite::from_color(Color::srgb(0.42, 0.42, 0.44), Vec2::new(raio * 1.1, 0.5)),
        Transform::from_xyz(-raio * 0.2, -raio * 0.62 - 0.25, 0.3),
    ));
    for i in 0..n {
        let x = -raio * 0.65 + i as f32 * 1.7;
        let y = -raio * 0.62;
        let alt = 3.0 + rng.f() * 0.8;
        p.spawn((
            Sprite::from_color(Color::srgb(0.88, 0.62, 0.15), Vec2::new(0.3, alt)),
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(x, y, 1.2),
        ))
        .with_children(|g| {
            let len = 2.4;
            g.spawn((
                Sprite::from_color(Color::srgb(0.80, 0.55, 0.12), Vec2::new(len, 0.22)),
                Anchor::CENTER_LEFT,
                Transform::from_xyz(0.0, alt, 0.01),
                JibGuindaste {
                    fase: rng.f() * 6.28,
                    vel: 0.35 + rng.f() * 0.3,
                    comprimento: len,
                },
            ));
            g.spawn((
                Sprite::from_color(Color::srgb(0.25, 0.45, 0.60), Vec2::new(0.55, 0.32)),
                Transform::from_xyz(0.0, alt - 0.9, 0.02),
                CargaGuindaste {
                    fase: rng.f() * 6.28,
                    vel: 0.35 + rng.f() * 0.3,
                    comprimento: len,
                },
            ));
        });
    }
}

/// Cria sob demanda (zoom próximo e dentro da tela) os detalhes de cada cidade e os estende
/// quando o nível sobe; refaz tudo quando a era muda.
pub fn materializar_detalhes_sistema(
    mut commands: Commands,
    lod: Res<EstadoLod>,
    eras: Res<GerenciadorEras>,
    estilo: Res<crate::estilo::EstiloMapa>,
    tex: Option<Res<TexturasJanelas>>,
    camera: Query<(&Transform, &Projection), With<Camera2d>>,
    janela: Query<&Window, With<PrimaryWindow>>,
    cidades: Query<(Entity, &CidadeVisual, Option<&DetalhesGerados>)>,
) {
    if lod.nivel_atual != NivelLod::Local {
        return;
    }
    let Some((centro, escala)) = escala_da_camera(&camera) else {
        return;
    };
    let (w, h) = janela
        .single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or((1600.0, 900.0));
    let meia = Vec2::new(w, h) * 0.5 * escala + Vec2::splat(40.0);
    let mut feitos = 0;
    for (ent, c, ger) in &cidades {
        if !c.existe || (c.posicao_mundo - centro).abs().cmpgt(meia).any() {
            continue;
        }
        let escala_detalhe = escala_detalhe_cidade(c.nivel_atual_inteiro, escala, &estilo);
        if let Some(g) = ger {
            commands
                .entity(g.raiz)
                .insert(Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(escala_detalhe)));
        }
        match ger {
            Some(g) if g.nivel >= c.nivel_atual_inteiro && g.era == c.era_idx => continue,
            Some(g) if g.era == c.era_idx => {
                // Cresceu: acrescenta os prédios dos novos níveis, com animação de brotar.
                let novos: Vec<LayoutPredio> =
                    gerar_layout(&c.id, c.nivel_atual_inteiro, c.bloco, c.era_idx)
                        .into_iter()
                        .filter(|l| l.nivel_req > g.nivel)
                        .collect();
                let (bloco, era, noite) = (c.bloco, c.era_idx, eras.modo_noturno);
                let t = tex.as_deref();
                commands.entity(g.raiz).with_children(|p| {
                    for l in &novos {
                        spawn_predio(p, l, bloco, era, noite, t, true);
                    }
                });
                commands.entity(g.base).insert(
                    Transform::from_xyz(0.0, 0.0, 0.0)
                        .with_scale(base_escala(c.nivel_atual_inteiro)),
                );
                commands.entity(ent).insert(DetalhesGerados {
                    raiz: g.raiz,
                    nivel: c.nivel_atual_inteiro,
                    era: g.era,
                    base: g.base,
                });
            }
            outro => {
                if let Some(g) = outro {
                    commands.entity(g.raiz).despawn(); // era mudou: refaz
                }
                if feitos >= DETALHES_POR_QUADRO {
                    continue;
                }
                feitos += 1;
                let nivel = c.nivel_atual_inteiro;
                let layout = gerar_layout(&c.id, nivel, c.bloco, c.era_idx);
                let (bloco, era, noite, id, porto) = (
                    c.bloco,
                    c.era_idx,
                    eras.modo_noturno,
                    c.id.clone(),
                    c.eh_porto,
                );
                let t = tex.as_deref();
                let mut raiz = Entity::PLACEHOLDER;
                let mut base = Entity::PLACEHOLDER;
                commands.entity(ent).with_children(|p| {
                    let mut r = p.spawn((
                        Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(escala_detalhe)),
                        Visibility::default(),
                        RaizDetalhes,
                        DetalheLod,
                    ));
                    raiz = r.id();
                    r.with_children(|rp| {
                        if let Some(t) = t {
                            base = rp
                                .spawn((
                                    Mesh2d(t.base_cidade.clone()),
                                    MeshMaterial2d(t.mat_base.clone()),
                                    Transform::from_xyz(0.0, 0.0, 0.05)
                                        .with_scale(base_escala(nivel)),
                                ))
                                .id();
                        }
                        spawn_estradas_e_trafego(rp, &id, (nivel + 2).min(8), era);
                        for l in &layout {
                            spawn_predio(rp, l, bloco, era, noite, t, false);
                        }
                        if porto && nivel >= 2 {
                            spawn_guindastes(rp, &id, nivel);
                        }
                    });
                });
                commands.entity(ent).insert(DetalhesGerados {
                    raiz,
                    nivel,
                    era,
                    base,
                });
            }
        }
    }
}

fn base_escala(nivel: u8) -> Vec3 {
    let r = raio_urbano(nivel) * 2.6;
    Vec3::new(r, r, 1.0)
}

/// Brotar dos prédios novos.
pub fn animar_predios_urbanos_sistema(
    time: Res<Time>,
    mut q: Query<(&mut PredioUrbano, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (mut p, mut tr) in &mut q {
        if (p.crescimento - p.alvo).abs() > 0.002 {
            p.crescimento += (p.alvo - p.crescimento) * (dt * 3.0).min(1.0);
            tr.scale.y = p.crescimento.max(0.001);
        } else if p.crescimento != p.alvo {
            p.crescimento = p.alvo;
            tr.scale.y = p.alvo;
        }
    }
}

/// Troca dia/noite: texturas das janelas e escurecimento das fachadas.
pub fn atualizar_noite_sistema(
    eras: Res<GerenciadorEras>,
    objetos: Res<crate::config::ConfiguracaoObjetos>,
    tex: Option<Res<TexturasJanelas>>,
    mut ultimo: Local<Option<bool>>,
    mut janelas: Query<(&JanelaPredio, &mut Sprite), Without<PredioUrbano>>,
    mut corpos: Query<&mut Sprite, (With<PredioUrbano>, Without<JanelaPredio>)>,
) {
    let noite = eras.modo_noturno || objetos.intensidade_noite > 0.1;
    if *ultimo == Some(noite) {
        return;
    }
    *ultimo = Some(noite);
    let Some(t) = tex else { return };
    for (j, mut s) in &mut janelas {
        s.image = if noite {
            t.noite[j.variante].clone()
        } else {
            t.dia[j.variante].clone()
        };
    }
    for mut s in &mut corpos {
        let c = s.color.to_srgba();
        s.color = if noite {
            Color::srgb(
                c.red * 0.45,
                c.green * 0.45,
                (c.blue * 0.45 + 0.04).min(1.0),
            )
        } else {
            Color::srgb(
                (c.red / 0.45).min(1.0),
                (c.green / 0.45).min(1.0),
                ((c.blue - 0.04) / 0.45).clamp(0.0, 1.0),
            )
        };
    }
}

/// Emite fumaça sutil das chaminés visíveis.
pub fn emitir_fumaca_urbana_sistema(
    mut commands: Commands,
    time: Res<Time>,
    lod: Res<EstadoLod>,
    objetos: Res<crate::config::ConfiguracaoObjetos>,
    eras: Res<GerenciadorEras>,
    camera: Query<(&Transform, &Projection), With<Camera2d>>,
    janela: Query<&Window, With<PrimaryWindow>>,
    existentes: Query<(), With<FumacaUrbana>>,
    mut chamines: Query<(&GlobalTransform, &mut Chamine, &Sprite)>,
) {
    if lod.nivel_atual != NivelLod::Local || !objetos.qualidade.tem_animacao() {
        return;
    }
    let Some((centro, escala)) = escala_da_camera(&camera) else {
        return;
    };
    let (w, h) = janela
        .single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or((1600.0, 900.0));
    let meia = Vec2::new(w, h) * 0.5 * escala + Vec2::splat(30.0);
    let mut total = existentes.iter().count();
    let dt = time.delta_secs();
    let cor = if eras.modo_noturno {
        Color::srgba(0.55, 0.57, 0.66, 0.30)
    } else {
        Color::srgba(0.82, 0.82, 0.84, 0.38)
    };
    for (gt, mut ch, sp) in &mut chamines {
        if total >= MAX_FUMACA_URBANA {
            break;
        }
        let pos = gt.translation();
        if (pos.truncate() - centro).abs().cmpgt(meia).any() {
            continue;
        }
        ch.acumulador += dt * ch.ritmo;
        if ch.acumulador >= 1.0 {
            ch.acumulador -= 1.0;
            let topo = pos + Vec3::new(0.0, sp.custom_size.map_or(1.5, |s| s.y) + 0.1, 3.0);
            let j = (pos.x * 12.9898 + pos.y * 78.233).sin().abs();
            commands.spawn((
                Sprite::from_color(cor, Vec2::splat(0.4)),
                Transform::from_translation(topo),
                FumacaUrbana {
                    idade: 0.0,
                    vida: 3.0 + j * 2.0,
                    vel: Vec2::new(0.25 + j * 0.2, 0.55),
                    tam0: 0.4,
                },
            ));
            total += 1;
        }
    }
}

/// Move, expande e apaga a fumaça; remove tudo fora do zoom próximo.
pub fn atualizar_fumaca_urbana_sistema(
    mut commands: Commands,
    time: Res<Time>,
    lod: Res<EstadoLod>,
    mut q: Query<(Entity, &mut FumacaUrbana, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    let fora = lod.nivel_atual != NivelLod::Local;
    for (e, mut f, mut tr, mut s) in &mut q {
        f.idade += dt;
        if fora || f.idade >= f.vida {
            commands.entity(e).despawn();
            continue;
        }
        let k = f.idade / f.vida;
        tr.translation += (f.vel * dt).extend(0.0);
        let tam = f.tam0 * (1.0 + 2.2 * k);
        s.custom_size = Some(Vec2::splat(tam));
        let mut c = s.color.to_srgba();
        c.alpha = 0.38 * (1.0 - k) * (1.0 - k).max(0.0).sqrt();
        s.color = Color::Srgba(c);
        tr.rotation = Quat::from_rotation_z(k * 1.5);
    }
}

/// Anima guindastes (giro visto de lado + carga que sobe e desce) e o tráfego.
pub fn animar_porto_e_trafego_sistema(
    time: Res<Time>,
    lod: Res<EstadoLod>,
    mut jibs: Query<(&JibGuindaste, &mut Transform), (Without<CargaGuindaste>, Without<Veiculo>)>,
    mut cargas: Query<(&CargaGuindaste, &mut Transform), (Without<JibGuindaste>, Without<Veiculo>)>,
    mut veiculos: Query<
        (&Veiculo, &mut Transform),
        (Without<JibGuindaste>, Without<CargaGuindaste>),
    >,
) {
    if lod.nivel_atual != NivelLod::Local {
        return;
    }
    let t = time.elapsed_secs();
    for (j, mut tr) in &mut jibs {
        tr.scale.x = (t * j.vel + j.fase).cos();
    }
    for (c, mut tr) in &mut cargas {
        let giro = (t * c.vel + c.fase).cos();
        tr.translation.x = c.comprimento * giro * 0.92;
        tr.translation.y = 2.1 + 0.35 * (t * 0.8 + c.fase).sin();
    }
    for (v, mut tr) in &mut veiculos {
        // vai e volta ao longo da rua
        let f = (t * v.vel + v.fase).fract();
        let tri = if f < 0.5 { f * 2.0 } else { 2.0 - f * 2.0 };
        let p = v.a.lerp(v.b, tri);
        tr.translation.x = p.x;
        tr.translation.y = p.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fundacao_brasilia() {
        assert!(!existe_no_ano(Some(1960), 1945.0));
        assert!(!existe_no_ano(Some(1960), 1959.9));
        assert!(existe_no_ano(Some(1960), 1960.0));
        assert!(existe_no_ano(None, 1945.0));
    }

    #[test]
    fn troca_de_nome() {
        let m = vec![(1961u16, "Volgogrado".to_string())];
        assert_eq!(nome_no_ano("Stalingrado", &m, 1950.0), "Stalingrado");
        assert_eq!(nome_no_ano("Stalingrado", &m, 1961.5), "Volgogrado");
        let m2 = vec![
            (1924u16, "Leningrado".to_string()),
            (1991, "São Petersburgo".to_string()),
        ];
        assert_eq!(nome_no_ano("Petrogrado", &m2, 1945.0), "Leningrado");
        assert_eq!(nome_no_ano("Petrogrado", &m2, 1992.0), "São Petersburgo");
    }

    #[test]
    fn rotulo_fica_menor_no_zoom_proximo() {
        assert!(tamanho_rotulo_px(0.3) < tamanho_rotulo_px(3.0));
        assert!(tamanho_rotulo_px(4.0) >= 7.0 && tamanho_rotulo_px(0.1) <= 15.0);
    }

    #[test]
    fn marcador_cresce_com_nivel_e_e_pequeno() {
        assert!(raio_marcador_px(8, false) > raio_marcador_px(1, false));
        assert!(raio_marcador_px(8, true) < 6.0);
    }

    #[test]
    fn colisao_esconde_o_menos_prioritario() {
        let a = RetTela {
            cx: 0.0,
            cy: 0.0,
            hw: 30.0,
            hh: 6.0,
        };
        let b = RetTela {
            cx: 20.0,
            cy: 3.0,
            hw: 30.0,
            hh: 6.0,
        };
        let c = RetTela {
            cx: 200.0,
            cy: 0.0,
            hw: 30.0,
            hh: 6.0,
        };
        assert_eq!(escolher_sem_colisao(&[a, b, c], 10), vec![0, 2]);
        assert_eq!(escolher_sem_colisao(&[a, b, c], 1), vec![0]);
    }

    #[test]
    fn layout_varia_por_bloco_e_nivel() {
        let pequeno = gerar_layout("x", 2, BlocoVisual::Ocidental, 0);
        let grande = gerar_layout("x", 8, BlocoVisual::Ocidental, 4);
        assert!(grande.len() > pequeno.len());
        assert!(grande.iter().any(|l| l.tipo == TipoPredio::ArranhaCeu));
        assert!(pequeno.iter().all(|l| l.tipo != TipoPredio::ArranhaCeu));
        let tm = gerar_layout("x", 8, BlocoVisual::TerceiroMundo, 0);
        assert_ne!(
            format!(
                "{:?}",
                cor_fachada(BlocoVisual::Sovietica, 0, TipoPredio::Bloco, 0.5)
            ),
            format!(
                "{:?}",
                cor_fachada(BlocoVisual::TerceiroMundo, 0, TipoPredio::Casa, 0.5)
            )
        );
        assert!(!tm.is_empty());
        // determinístico
        assert_eq!(
            gerar_layout("x", 5, BlocoVisual::Sovietica, 1).len(),
            gerar_layout("x", 5, BlocoVisual::Sovietica, 1).len()
        );
    }

    #[test]
    fn teto_de_detalhe_em_pixels_nao_cresce_com_zoom() {
        let estilo = crate::estilo::EstiloMapa::default();
        let teto = estilo.cidades_detalhe_max_px;
        for escala_camera in [0.8, 0.4, 0.2, 0.1] {
            let escala = escala_detalhe_cidade(8, escala_camera, &estilo);
            let diametro_tela = diametro_detalhe_mundo(8) * escala / escala_camera;
            assert!(diametro_tela <= teto + 1e-4, "{diametro_tela} > {teto}");
        }
        assert!(
            escala_detalhe_cidade(8, 0.1, &estilo) < escala_detalhe_cidade(8, 0.8, &estilo),
            "a raiz deve encolher no zoom máximo"
        );
    }
}
