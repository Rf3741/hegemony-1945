//! Nomes de país grandes sobre o território (estilo Age of History 3 / Victoria 3).
//!
//! Calculados uma vez ao entrar no menu a partir do mapa de províncias: o maior pedaço contíguo
//! de cada país (grade 1/8), centro de massa, eixo principal (rotação) e tamanho proporcional à
//! extensão. Caixa alta espaçada, semitransparente, com contorno. Somem ao aproximar muito.

use super::economy::RecursoMundo;
use super::map::{CoresMapa, DadosMapa};
use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};
use visual_mapa::EstiloMapa;

const CELULA: usize = 8;
const FONTE_BASE: f32 = 48.0;

#[derive(Component)]
pub struct RotuloPais {
    /// Altura do texto em unidades de mundo.
    pub altura: f32,
}

/// Filho de texto: opacidade base (contorno é mais fraco que o miolo).
#[derive(Component)]
pub struct TextoPais {
    pub alfa: f32,
    pub contorno: bool,
}

#[derive(Resource)]
struct RotulosPaisCriados;

#[derive(Debug, Clone)]
pub struct GeoPais {
    pub tag: String,
    pub centro: Vec2,
    /// Ângulo (rad, anti-horário em mundo) do eixo principal, já limitado.
    pub angulo: f32,
    /// Extensão ao longo do eixo principal e do secundário (mundo).
    pub comprimento: f32,
    pub largura: f32,
}

/// Maior componente conexo de cada dono numa grade reduzida. `donos[y*w+x]` = índice (0 = sem dono).
pub fn geometria_paises(donos: &[u32], w: usize, h: usize, n_donos: usize) -> Vec<(u32, GeoPais0)> {
    let mut visto = vec![false; w * h];
    let mut melhor: Vec<Option<GeoPais0>> = vec![None; n_donos + 1];
    for s in 0..w * h {
        if visto[s] || donos[s] == 0 {
            continue;
        }
        let d = donos[s];
        let mut fila = VecDeque::new();
        visto[s] = true;
        fila.push_back(((s % w) as i32, (s / w) as i32));
        let (mut n, mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
        while let Some((x, y)) = fila.pop_front() {
            let (fx, fy) = (x as f64, y as f64);
            n += 1.0;
            sx += fx;
            sy += fy;
            sxx += fx * fx;
            syy += fy * fy;
            sxy += fx * fy;
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let ny = y + dy;
                if ny < 0 || ny >= h as i32 {
                    continue;
                }
                let nx = x + dx;
                let i = ny as usize * w + nx.rem_euclid(w as i32) as usize;
                if !visto[i] && donos[i] == d {
                    visto[i] = true;
                    fila.push_back((nx, ny));
                }
            }
        }
        if melhor[d as usize].as_ref().is_none_or(|m| m.n < n) {
            let (mx, my) = (sx / n, sy / n);
            melhor[d as usize] = Some(GeoPais0 {
                n,
                cx: mx,
                cy: my,
                vxx: sxx / n - mx * mx + 0.08,
                vyy: syy / n - my * my + 0.08,
                vxy: sxy / n - mx * my,
            });
        }
    }
    melhor.into_iter().enumerate().filter_map(|(i, g)| g.map(|g| (i as u32, g))).collect()
}

#[derive(Debug, Clone)]
pub struct GeoPais0 {
    pub n: f64,
    pub cx: f64,
    pub cy: f64,
    pub vxx: f64,
    pub vyy: f64,
    pub vxy: f64,
}

impl GeoPais0 {
    /// (ângulo em mundo, comprimento, largura) em unidades de mundo.
    pub fn eixos(&self) -> (f32, f32, f32) {
        let tr = self.vxx + self.vyy;
        let det = self.vxx * self.vyy - self.vxy * self.vxy;
        let disc = (tr * tr / 4.0 - det).max(0.0).sqrt();
        let (l1, l2) = (tr / 2.0 + disc, (tr / 2.0 - disc).max(0.01));
        let theta = 0.5 * (2.0 * self.vxy).atan2(self.vxx - self.vyy);
        // y da grade cresce para baixo; no mundo cresce para cima.
        let mut ang = -theta as f32;
        if (l1 / l2) < 1.5 {
            ang = 0.0;
        }
        let lim = 0.6f32;
        let ang_c = ang.clamp(-lim, lim);
        // se o eixo é bem vertical, mantém o texto horizontal e usa a extensão horizontal
        let (comp, larg) = if ang.abs() > lim { (2.0 * (self.vxx.sqrt()), 2.0 * self.vyy.sqrt()) } else { (4.0 * l1.sqrt(), 4.0 * l2.sqrt()) };
        let (comp, larg) = if ang.abs() > lim { (comp * 2.0, larg * 2.0) } else { (comp, larg) };
        (ang_c, comp as f32 * CELULA as f32, larg as f32 * CELULA as f32)
    }
}

fn texto_espacado(nome: &str, espacar: bool) -> String {
    let up = nome.to_uppercase();
    if !espacar {
        return up;
    }
    up.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
}

fn criar_rotulos(
    mut commands: Commands,
    dados: Res<DadosMapa>,
    cores: Res<CoresMapa>,
    mundo: Res<RecursoMundo>,
    assets: Res<AssetServer>,
) {
    commands.insert_resource(RotulosPaisCriados);
    let (w, h) = (dados.largura as usize / CELULA, dados.altura as usize / CELULA);
    // dono_prov (índice) -> tag
    let mut tag_de: HashMap<u32, &str> = HashMap::new();
    for (pid, tag) in &mundo.0.dono_por_provincia {
        if let Some(&idx) = cores.dono_prov.get(*pid as usize) {
            if idx != 0 {
                tag_de.insert(idx, tag.as_str());
            }
        }
    }
    let n_donos = cores.dono_prov.iter().copied().max().unwrap_or(0) as usize;
    let mut grade = vec![0u32; w * h];
    for gy in 0..h {
        for gx in 0..w {
            let (px, py) = (gx * CELULA + CELULA / 2, gy * CELULA + CELULA / 2);
            let id = dados.ids[py * dados.largura as usize + px] as usize;
            grade[gy * w + gx] = cores.dono_prov.get(id).copied().unwrap_or(0);
        }
    }
    let fonte: Handle<Font> = assets.load("fonts/Oswald-Bold.ttf");
    let (largura_m, altura_m) = (dados.largura as f32, dados.altura as f32);
    let mut criados = 0;
    for (idx, g) in geometria_paises(&grade, w, h, n_donos) {
        let Some(tag) = tag_de.get(&idx) else { continue };
        let Some(estado) = mundo.0.paises.get(*tag) else { continue };
        let (ang, comp, larg) = g.eixos();
        let nome = &estado.nome;
        let n_chars = nome.chars().count().max(1) as f32;
        let espacar = n_chars <= 13.0;
        let texto = texto_espacado(nome, espacar);
        let n_vis = texto.chars().count() as f32;
        // largura do texto ~ 0.5 em por caractere (Oswald condensada)
        let fs = (0.8 * comp / (n_vis * 0.5)).min(larg * 0.55).clamp(4.0, 150.0);
        let pos = Vec2::new(g.cx as f32 * CELULA as f32 + CELULA as f32 / 2.0 - largura_m / 2.0, altura_m / 2.0 - (g.cy as f32 * CELULA as f32 + CELULA as f32 / 2.0));
        let pos = Vec2::new((pos.x + largura_m / 2.0).rem_euclid(largura_m) - largura_m / 2.0, pos.y);
        let esc = fs / FONTE_BASE;
        let o = FONTE_BASE * 0.045;
        commands
            .spawn((
                RotuloPais { altura: fs },
                Transform::from_translation(pos.extend(4.0)).with_rotation(Quat::from_rotation_z(ang)).with_scale(Vec3::splat(esc)),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                let mk = || {
                    let mut tf = TextFont::from_font_size(FONTE_BASE);
                    tf.font = fonte.clone().into();
                    tf
                };
                for (dx, dy) in [(o, 0.0), (-o, 0.0), (0.0, o), (0.0, -o)] {
                    p.spawn((
                        Text2d::new(texto.clone()),
                        mk(),
                        TextColor(Color::srgba(0.03, 0.04, 0.07, 0.0)),
                        Transform::from_xyz(dx, dy, 0.0),
                        TextoPais { alfa: 0.55, contorno: true },
                    ));
                }
                p.spawn((
                    Text2d::new(texto.clone()),
                    mk(),
                    TextColor(Color::srgba(0.97, 0.96, 0.92, 0.0)),
                    Transform::from_xyz(0.0, 0.0, 0.1),
                    TextoPais { alfa: 1.0, contorno: false },
                ));
            });
        criados += 1;
    }
    info!("rotulos_pais: {criados} nomes de país criados.");
}

fn atualizar_rotulos(
    estilo: Res<EstiloMapa>,
    camera: Query<&Projection, With<Camera2d>>,
    mut rotulos: Query<(&RotuloPais, &mut Visibility, &Children)>,
    mut textos: Query<(&TextoPais, &mut TextColor)>,
    mut ultimo: Local<f32>,
) {
    let Some(escala) = camera.single().ok().map(|p| match p {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    }) else {
        return;
    };
    if (escala - *ultimo).abs() < 0.004 * escala && *ultimo != 0.0 {
        return;
    }
    *ultimo = escala;
    let t = ((escala - estilo.pais_escala_some) / (estilo.pais_escala_cheio - estilo.pais_escala_some)).clamp(0.0, 1.0);
    let fade = t * t * (3.0 - 2.0 * t);
    for (r, mut vis, filhos) in &mut rotulos {
        let px = r.altura / escala;
        // some se pequeno demais na tela; aparece gradualmente entre min e 1.6*min
        let tam = ((px - estilo.pais_altura_min_px) / (estilo.pais_altura_min_px * 0.6)).clamp(0.0, 1.0);
        let a = fade * tam * estilo.pais_alfa;
        let alvo = if a > 0.01 { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != alvo {
            *vis = alvo;
        }
        if a > 0.01 {
            for f in filhos.iter() {
                if let Ok((tp, mut cor)) = textos.get_mut(f) {
                    let al = if tp.contorno { a * 0.8 } else { a };
                    cor.0.set_alpha(al * if tp.contorno { tp.alfa / 0.55 * 0.55 } else { 1.0 });
                }
            }
        }
    }
}

pub struct RotulosPaisPlugin;

impl Plugin for RotulosPaisPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                criar_rotulos
                    .run_if(resource_exists::<DadosMapa>)
                    .run_if(resource_exists::<CoresMapa>)
                    .run_if(resource_exists::<RecursoMundo>)
                    .run_if(not(resource_exists::<RotulosPaisCriados>)),
                atualizar_rotulos,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maior_pedaco_e_eixo() {
        // Dono 1: faixa horizontal 10x2 e uma ilha 1x1; dono 2: quadrado.
        let (w, h) = (20, 8);
        let mut g = vec![0u32; w * h];
        for x in 2..12 {
            for y in 1..3 {
                g[y * w + x] = 1;
            }
        }
        g[6 * w + 15] = 1;
        let r = geometria_paises(&g, w, h, 2);
        let (_, g1) = r.iter().find(|(i, _)| *i == 1).unwrap();
        assert!((g1.n - 20.0).abs() < 1e-6, "ilha não conta");
        let (ang, comp, larg) = g1.eixos();
        assert!(ang.abs() < 0.05);
        assert!(comp > larg * 2.0);
    }

    #[test]
    fn espacado() {
        assert_eq!(texto_espacado("Peru", true), "P E R U");
    }
}
