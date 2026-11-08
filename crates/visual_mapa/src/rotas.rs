//! Estradas e ferrovias ligando as cidades.
//!
//! - O grafo liga cada cidade às vizinhas mais próximas do MESMO país (sem cruzar oceano, se a
//!   [`MascaraTerra`] existir). A espessura/infraestrutura vem do nível das duas cidades (que
//!   cresce com a economia) e a ferrovia aparece só entre cidades grandes.
//! - A malha é gerada uma vez e refeita apenas quando a "assinatura" (níveis das cidades e era
//!   por quinquênio) muda. Estradas só aparecem no zoom médio e de perto (LOD) e respeitam a
//!   tecla V ([`CamadaConstrucoesVisivel`]).
//! - Estrada de terra = marrom; rodovia = asfalto com contorno; ferrovia = trilho + dormentes.

use crate::config::CamadaConstrucoesVisivel;
use crate::entrada::{AnoVisual, EntradaVisual};
use crate::linhas::ConstrutorLinhas;
use crate::lod::EstadoLod;
use crate::projecao::ProjecaoMapa;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

/// Máscara de terra (1 = terra, 0 = oceano) na resolução do mapa, para não ligar cidades por mar.
/// Opcional: o cliente a preenche a partir de `DadosMapa`.
#[derive(Resource, Debug, Clone)]
pub struct MascaraTerra {
    pub largura: u32,
    pub altura: u32,
    pub terra: Vec<bool>,
}

impl MascaraTerra {
    fn eh_terra(&self, p: Vec2) -> bool {
        let px = (p.x + self.largura as f32 * 0.5).floor().rem_euclid(self.largura as f32) as u32;
        let py = (self.altura as f32 * 0.5 - p.y / gf_mapa::mapa::ESCALA_Y_MAPA).floor();
        if py < 0.0 || py >= self.altura as f32 {
            return false;
        }
        self.terra[py as usize * self.largura as usize + px.min(self.largura - 1) as usize]
    }

    /// Fração de amostras em terra ao longo do segmento.
    pub fn fracao_terra(&self, a: Vec2, b: Vec2) -> f32 {
        let n = ((b - a).length() / 3.0).ceil().max(2.0) as usize;
        let ok = (0..=n).filter(|&i| self.eh_terra(a.lerp(b, i as f32 / n as f32))).count();
        ok as f32 / (n + 1) as f32
    }
}

/// Parâmetros do grafo.
#[derive(Resource, Debug, Clone)]
pub struct ConfigRotas {
    pub ativo: bool,
    /// Vizinhas mais próximas ligadas por cidade.
    pub vizinhas: usize,
    /// Distância máxima de uma ligação (unidades de mundo).
    pub dist_max: f32,
    pub largura_mapa: f32,
    /// Nível mínimo (das duas pontas) para haver ferrovia.
    pub nivel_min_ferrovia: u8,
    /// Ano a partir do qual rodovias asfaltadas substituem estradas de terra.
    pub ano_rodovias: f32,
}

impl Default for ConfigRotas {
    fn default() -> Self {
        Self { ativo: true, vizinhas: 2, dist_max: 90.0, largura_mapa: 5632.0, nivel_min_ferrovia: 4, ano_rodovias: 1950.0 }
    }
}

/// Entrada mínima do grafo.
#[derive(Debug, Clone)]
pub struct NoRota {
    pub pos: Vec2,
    pub nivel: u8,
    pub pais: String,
}

/// Tipo de via ligando duas cidades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViaTipo {
    Terra,
    Rodovia,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Aresta {
    pub a: usize,
    pub b: usize,
    /// Infraestrutura 1..=5 (espessura).
    pub infra: u8,
    pub ferrovia: bool,
    /// Ferrovia pesada (dupla).
    pub pesada: bool,
}

/// Nível de cidade 1..=8 -> infraestrutura 1..=5.
pub fn infra_do_nivel(nivel: u8) -> u8 {
    match nivel {
        0..=2 => 1,
        3..=4 => 2,
        5 => 3,
        6..=7 => 4,
        _ => 5,
    }
}

/// Constrói o grafo: k vizinhas mais próximas do mesmo país + árvore geradora mínima por país
/// (garante conectividade). Função pura e determinística.
pub fn construir_grafo(nos: &[NoRota], cfg: &ConfigRotas, mascara: Option<&MascaraTerra>) -> Vec<Aresta> {
    let n = nos.len();
    let mut pares: HashSet<(usize, usize)> = HashSet::new();
    let mut por_pais: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, no) in nos.iter().enumerate() {
        por_pais.entry(no.pais.as_str()).or_default().push(i);
    }
    let ligavel = |i: usize, j: usize, limite: f32| -> bool {
        let d = nos[i].pos.distance(nos[j].pos);
        d > 1.0 && d <= limite && mascara.is_none_or(|m| m.fracao_terra(nos[i].pos, nos[j].pos) >= 0.97)
    };
    let mut tags: Vec<&&str> = por_pais.keys().collect();
    tags.sort();
    for tag in tags {
        let idx = &por_pais[*tag];
        // k mais próximas
        for &i in idx {
            let mut cand: Vec<(f32, usize)> = idx
                .iter()
                .filter(|&&j| j != i)
                .map(|&j| (nos[i].pos.distance_squared(nos[j].pos), j))
                .collect();
            cand.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
            let mut feitas = 0;
            for (_, j) in cand {
                if feitas >= cfg.vizinhas {
                    break;
                }
                if ligavel(i, j, cfg.dist_max) {
                    pares.insert((i.min(j), i.max(j)));
                    feitas += 1;
                }
            }
        }
        // MST (Kruskal) sobre arestas candidatas curtas, para ligar ilhas de componentes.
        if idx.len() > 1 && idx.len() <= 600 {
            let mut e: Vec<(f32, usize, usize)> = Vec::new();
            for (ai, &i) in idx.iter().enumerate() {
                for &j in &idx[ai + 1..] {
                    let d = nos[i].pos.distance(nos[j].pos);
                    if d <= cfg.dist_max * 1.5 {
                        e.push((d, i, j));
                    }
                }
            }
            e.sort_by(|x, y| x.0.total_cmp(&y.0).then((x.1, x.2).cmp(&(y.1, y.2))));
            let mut pai: HashMap<usize, usize> = idx.iter().map(|&i| (i, i)).collect();
            fn raiz(pai: &mut HashMap<usize, usize>, x: usize) -> usize {
                let mut r = x;
                while pai[&r] != r {
                    r = pai[&r];
                }
                let mut c = x;
                while pai[&c] != r {
                    let p = pai[&c];
                    pai.insert(c, r);
                    c = p;
                }
                r
            }
            for (_, i, j) in e {
                let (ri, rj) = (raiz(&mut pai, i), raiz(&mut pai, j));
                if ri != rj && ligavel(i, j, cfg.dist_max * 1.5) {
                    pai.insert(ri, rj);
                    pares.insert((i.min(j), i.max(j)));
                }
            }
        }
    }
    let mut ordenados: Vec<(usize, usize)> = pares.into_iter().filter(|&(a, b)| a < n && b < n).collect();
    ordenados.sort();
    ordenados
        .into_iter()
        .map(|(a, b)| {
            let nivel = nos[a].nivel.min(nos[b].nivel);
            let ferrovia = nos[a].nivel >= cfg.nivel_min_ferrovia && nos[b].nivel >= cfg.nivel_min_ferrovia;
            Aresta { a, b, infra: infra_do_nivel(nivel), ferrovia, pesada: ferrovia && nivel >= 7 }
        })
        .collect()
}

/// Ponto determinístico em [-1, 1] a partir de um par de índices (curvatura das vias).
fn ruido(a: usize, b: usize, k: u64) -> f32 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (a, b, k).hash(&mut h);
    (h.finish() % 20001) as f32 / 10000.0 - 1.0
}

/// Caminho levemente sinuoso entre dois pontos.
pub fn caminho_sinuoso(a: Vec2, b: Vec2, ia: usize, ib: usize) -> Vec<Vec2> {
    let d = b - a;
    let len = d.length();
    let n = ((len / 28.0).ceil() as usize).clamp(2, 14);
    let perp = Vec2::new(-d.y, d.x) / len.max(1e-3);
    let amp = len * 0.07;
    let (f1, f2) = (ruido(ia, ib, 1), ruido(ia, ib, 2));
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let env = (t * std::f32::consts::PI).sin();
            let off = env * amp * (f1 * (t * std::f32::consts::PI * 1.0).sin() + 0.45 * f2 * (t * std::f32::consts::TAU).sin());
            a.lerp(b, t) + perp * off
        })
        .collect()
}

/// Malhas finais (estradas de terra, rodovias, ferrovias).
pub struct MalhasRotas {
    pub estradas: ConstrutorLinhas,
    pub ferrovias: ConstrutorLinhas,
}

const COR_TERRA: Color = Color::srgba(0.45, 0.36, 0.24, 0.45);
const COR_ASFALTO: Color = Color::srgba(0.80, 0.78, 0.72, 0.5);
const COR_CONTORNO: Color = Color::srgba(0.17, 0.16, 0.15, 0.35);
const COR_LEITO: Color = Color::srgba(0.12, 0.12, 0.14, 0.55);
const COR_DORMENTE: Color = Color::srgb(0.28, 0.20, 0.13);
const COR_TRILHO: Color = Color::srgb(0.88, 0.88, 0.90);

pub fn construir_malhas(nos: &[NoRota], arestas: &[Aresta], ano: f32, cfg: &ConfigRotas) -> MalhasRotas {
    let mut estradas = ConstrutorLinhas::novo();
    let mut ferrovias = ConstrutorLinhas::novo();
    for e in arestas {
        let pts = caminho_sinuoso(nos[e.a].pos, nos[e.b].pos, e.a, e.b);
        let infra = e.infra as f32;
        let tipo = if ano >= cfg.ano_rodovias && e.infra >= 3 { ViaTipo::Rodovia } else { ViaTipo::Terra };
        // Contorno primeiro (todas as estradas de um mesmo tipo no mesmo buffer: contorno e miolo
        // são desenhados em ordem por aresta, o que basta para vias que raramente se cruzam).
        match tipo {
            ViaTipo::Terra => estradas.polilinha(&pts, 0.35 + 0.05 * infra, COR_TERRA),
            ViaTipo::Rodovia => {
                estradas.polilinha(&pts, 0.55 + 0.08 * infra, COR_ASFALTO);
            }
        }
        if e.ferrovia {
            let off = if tipo == ViaTipo::Rodovia || e.infra >= 2 { 1.2 } else { 0.0 };
            let trilha = deslocar(&pts, off);
            ferrovia(&mut ferrovias, &trilha, infra, e.pesada);
            if e.pesada {
                ferrovia(&mut ferrovias, &deslocar(&pts, off + 1.4), infra, false);
            }
        }
    }
    MalhasRotas { estradas, ferrovias }
}

fn deslocar(pts: &[Vec2], d: f32) -> Vec<Vec2> {
    if d == 0.0 {
        return pts.to_vec();
    }
    (0..pts.len())
        .map(|i| {
            let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(pts.len() - 1)]);
            let dir = (b - a).normalize_or_zero();
            pts[i] + Vec2::new(-dir.y, dir.x) * d
        })
        .collect()
}

/// Ferrovia: uma linha fina e escura (discreta).
fn ferrovia(c: &mut ConstrutorLinhas, pts: &[Vec2], _infra: f32, _pesada: bool) {
    c.polilinha(pts, 0.4, COR_LEITO);
}

#[derive(Component, Debug, Clone, Copy)]
pub struct CamadaRotas;

#[derive(Resource, Default)]
pub struct EstadoRotas {
    assinatura: u64,
    pub arestas: usize,
    pub entidades: Vec<Entity>,
}

fn assinatura(entrada: &EntradaVisual, ano: f32, tem_mascara: bool) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    entrada.cidades.len().hash(&mut h);
    for c in &entrada.cidades {
        (c.id.as_str(), c.nivel.floor() as u8).hash(&mut h);
    }
    ((ano / 5.0).floor() as i32).hash(&mut h);
    tem_mascara.hash(&mut h);
    h.finish()
}

pub fn reconstruir_rotas_sistema(
    mut commands: Commands,
    cfg: Res<ConfigRotas>,
    entrada: Res<EntradaVisual>,
    ano: Res<AnoVisual>,
    proj: Option<Res<crate::config::VisualMapaConfig>>,
    mascara: Option<Res<MascaraTerra>>,
    mut estado: ResMut<EstadoRotas>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materiais: ResMut<Assets<ColorMaterial>>,
) {
    if !cfg.ativo || entrada.cidades.is_empty() {
        return;
    }
    let sig = assinatura(&entrada, ano.0, mascara.is_some());
    if sig == estado.assinatura {
        return;
    }
    estado.assinatura = sig;
    for e in estado.entidades.drain(..) {
        commands.entity(e).despawn();
    }
    let p: ProjecaoMapa = proj.map(|c| c.projecao).unwrap_or_default();
    let nos: Vec<NoRota> = entrada
        .cidades
        .iter()
        .map(|c| NoRota { pos: p.geo_para_mundo(c.posicao.0, c.posicao.1), nivel: c.nivel.floor().clamp(1.0, 8.0) as u8, pais: c.pais_tag.clone() })
        .collect();
    let arestas = construir_grafo(&nos, &cfg, mascara.as_deref());
    estado.arestas = arestas.len();
    let m = construir_malhas(&nos, &arestas, ano.0, &cfg);
    let mat = materiais.add(ColorMaterial::from_color(Color::WHITE));
    for (constr, z) in [(m.estradas, 3.0f32), (m.ferrovias, 3.2)] {
        if constr.vazio() {
            continue;
        }
        let h = meshes.add(constr.construir());
        for k in [-1.0f32, 0.0, 1.0] {
            let id = commands
                .spawn((
                    Mesh2d(h.clone()),
                    MeshMaterial2d(mat.clone()),
                    Transform::from_xyz(k * cfg.largura_mapa, 0.0, z),
                    Visibility::Hidden,
                    CamadaRotas,
                ))
                .id();
            estado.entidades.push(id);
        }
    }
    info!("visual_mapa: malha de rotas refeita ({} ligações entre {} cidades).", estado.arestas, nos.len());
}

/// Estradas: só no zoom médio e de perto, e somente com a camada V ligada.
pub fn lod_rotas_sistema(
    lod: Res<EstadoLod>,
    camada: Option<Res<CamadaConstrucoesVisivel>>,
    estilo: Res<crate::estilo::EstiloMapa>,
    mut q: Query<&mut Visibility, With<CamadaRotas>>,
) {
    let ligada = camada.is_none_or(|c| c.0);
    let alvo = if ligada && lod.escala_camera > 0.0 && lod.escala_camera < estilo.rotas_escala_max && lod.nivel_atual != crate::lod::NivelLod::Global { Visibility::Inherited } else { Visibility::Hidden };
    for mut v in &mut q {
        if *v != alvo {
            *v = alvo;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no(x: f32, y: f32, n: u8, p: &str) -> NoRota {
        NoRota { pos: Vec2::new(x, y), nivel: n, pais: p.into() }
    }

    #[test]
    fn grafo_nao_cruza_paises_e_conecta() {
        let nos = vec![no(0.0, 0.0, 5, "A"), no(50.0, 0.0, 5, "A"), no(100.0, 0.0, 2, "A"), no(60.0, 5.0, 5, "B"), no(120.0, 5.0, 5, "B")];
        let g = construir_grafo(&nos, &ConfigRotas::default(), None);
        assert!(g.iter().all(|e| nos[e.a].pais == nos[e.b].pais));
        assert!(g.iter().any(|e| (e.a, e.b) == (0, 1) && e.ferrovia));
        assert!(g.iter().any(|e| (e.a, e.b) == (1, 2) && !e.ferrovia));
    }

    #[test]
    fn mascara_bloqueia_mar() {
        // 100x20, metade do meio é oceano
        let (w, h) = (100u32, 20u32);
        let mut terra = vec![true; (w * h) as usize];
        for y in 0..h {
            for x in 40..60 {
                terra[(y * w + x) as usize] = false;
            }
        }
        let m = MascaraTerra { largura: w, altura: h, terra };
        let nos = vec![no(-40.0, 0.0, 5, "A"), no(40.0, 0.0, 5, "A")];
        assert!(construir_grafo(&nos, &ConfigRotas::default(), Some(&m)).is_empty());
        assert_eq!(construir_grafo(&nos, &ConfigRotas::default(), None).len(), 1);
    }

    #[test]
    fn infra_monotonica() {
        assert!(infra_do_nivel(1) < infra_do_nivel(5));
        assert!(infra_do_nivel(5) < infra_do_nivel(8));
    }

    #[test]
    fn malhas_nao_vazias() {
        let nos = vec![no(0.0, 0.0, 6, "A"), no(80.0, 20.0, 6, "A")];
        let cfg = ConfigRotas::default();
        let g = construir_grafo(&nos, &cfg, None);
        let m = construir_malhas(&nos, &g, 1960.0, &cfg);
        assert!(!m.estradas.vazio() && !m.ferrovias.vazio());
    }

    #[test]
    fn grafo_com_cidades_reais() {
        let caminho = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/territorio/cidades.ron");
        if !caminho.exists() {
            return;
        }
        let proj = ProjecaoMapa::default();
        let nos: Vec<NoRota> = crate::cenario::carregar_cidades_cenario(&caminho)
            .iter()
            .map(|c| NoRota {
                pos: proj.geo_para_mundo(c.posicao.0, c.posicao.1),
                nivel: crate::entrada::populacao_urbana_para_nivel(c.populacao_urbana),
                pais: c.pais_tag.clone(),
            })
            .collect();
        let t = std::time::Instant::now();
        let cfg = ConfigRotas::default();
        let g = construir_grafo(&nos, &cfg, None);
        let m = construir_malhas(&nos, &g, 1945.0, &cfg);
        eprintln!("{} cidades -> {} ligacoes, {} vertices de estrada, {} de ferrovia em {:?}", nos.len(), g.len(), m.estradas.vertices(), m.ferrovias.vertices(), t.elapsed());
        assert!(g.len() > nos.len() / 2);
    }
}
