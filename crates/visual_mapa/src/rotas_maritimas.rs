//! Roteamento marítimo realista sobre a máscara terra/mar do mapa (`provinces.png`).
//!
//! Módulo puro (sem Bevy): carrega a máscara (preto `(0,0,0)` = oceano), monta uma grade de
//! navegação (células de 1 px; estreitos de 1-2 px não se perdem), liga os CANAIS e ESTREITOS como "portais"
//! (a máscara de 7 km/px fecha Suez, Panamá, Kiel, Corinto, Gibraltar e Bósforo) e acha o caminho
//! mínimo com A*. O caminho é suavizado por linha de visada na máscara em resolução total, então
//! nenhum segmento atravessa terra (exceto os portais, que são os canais).
//!
//! Os nós do grafo são: células de mar aberto (grade) + extremidades dos portais. Penaliza-se a
//! navegação rente à costa para as rotas contornarem cabos em vez de raspar a terra.

use bevy::prelude::Vec2;
use gf_mapa::mapa::{ALTURA_MAPA_PADRAO, ESCALA_Y_MAPA, LARGURA_MAPA_PADRAO, projetar_miller};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::path::Path;

/// Fator de redução da grade de navegação em relação ao bitmap.
pub const FATOR_GRADE: usize = 1;

/// Canais e estreitos monitorados no relatório de passagens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Canal {
    Suez,
    Panama,
    Kiel,
    Corinto,
    Gibraltar,
    Bosforo,
}

impl Canal {
    pub const TODOS: [Canal; 6] =
        [Canal::Suez, Canal::Panama, Canal::Kiel, Canal::Corinto, Canal::Gibraltar, Canal::Bosforo];

    pub fn nome(self) -> &'static str {
        match self {
            Canal::Suez => "Suez",
            Canal::Panama => "Panamá",
            Canal::Kiel => "Kiel",
            Canal::Corinto => "Corinto",
            Canal::Gibraltar => "Gibraltar",
            Canal::Bosforo => "Bósforo",
        }
    }
}

/// Portal: ligação entre dois pontos de mar através de um canal/estreito fechado na máscara.
/// `canal = None` = estreito apenas de conectividade (não contado no relatório).
struct DefPortal {
    canal: Option<Canal>,
    a: (f64, f64),
    b: (f64, f64),
}

const PORTAIS: &[DefPortal] = &[
    DefPortal { canal: Some(Canal::Suez), a: (32.56, 31.35), b: (32.55, 29.90) },
    DefPortal { canal: Some(Canal::Panama), a: (-79.92, 9.40), b: (-79.52, 8.85) },
    DefPortal { canal: Some(Canal::Kiel), a: (9.05, 53.98), b: (10.20, 54.38) },
    DefPortal { canal: Some(Canal::Corinto), a: (22.96, 38.05), b: (23.05, 37.85) },
    DefPortal { canal: Some(Canal::Gibraltar), a: (-6.25, 35.95), b: (-4.85, 36.05) },
    DefPortal { canal: Some(Canal::Bosforo), a: (29.12, 41.28), b: (28.95, 40.85) },
    // Dardanelos (não contado: fica no mesmo trânsito do Bósforo).
    DefPortal { canal: None, a: (27.10, 40.45), b: (25.90, 40.00) },
    // Sund/Belts dinamarqueses (conectividade Báltico <-> Mar do Norte).
    DefPortal { canal: None, a: (11.0, 57.6), b: (12.9, 55.8) },
    DefPortal { canal: None, a: (12.9, 55.8), b: (13.6, 54.9) },
];

/// Máscara terra/mar em resolução total do bitmap.
pub struct Mascara {
    pub largura: usize,
    pub altura: usize,
    /// `true` = oceano.
    oceano: Vec<bool>,
}

impl Mascara {
    /// Carrega de `provinces.png` (oceano = RGB `(0,0,0)`).
    pub fn carregar(caminho: &Path) -> Result<Self, String> {
        let img = image::open(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?.to_rgb8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        let oceano = img.pixels().map(|p| p.0 == [0, 0, 0]).collect();
        Ok(Self { largura: w, altura: h, oceano })
    }

    /// Máscara sintética (testes): `f(x, y) -> é oceano`.
    pub fn de_funcao(largura: usize, altura: usize, f: impl Fn(usize, usize) -> bool) -> Self {
        let mut oceano = Vec::with_capacity(largura * altura);
        for y in 0..altura {
            for x in 0..largura {
                oceano.push(f(x, y));
            }
        }
        Self { largura, altura, oceano }
    }

    #[inline]
    pub fn eh_oceano(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.largura && (y as usize) < self.altura
            && self.oceano[y as usize * self.largura + x as usize]
    }

    /// Mundo (origem no centro, y para cima) -> pixel.
    pub fn mundo_para_pixel(&self, p: Vec2) -> (i32, i32) {
        ((p.x + self.largura as f32 * 0.5).floor() as i32, (self.altura as f32 * 0.5 - p.y / ESCALA_Y_MAPA).floor() as i32)
    }

    pub fn pixel_para_mundo(&self, x: f32, y: f32) -> Vec2 {
        Vec2::new(x - self.largura as f32 * 0.5, (self.altura as f32 * 0.5 - y) * ESCALA_Y_MAPA)
    }

    pub fn mundo_eh_oceano(&self, p: Vec2) -> bool {
        let (x, y) = self.mundo_para_pixel(p);
        self.eh_oceano(x, y)
    }

    /// Linha de visada livre: todos os pixels do segmento (e vizinhos laterais a 1 px) são mar.
    pub fn linha_livre(&self, a: Vec2, b: Vec2) -> bool {
        let d = b - a;
        let n = d.length().ceil().max(1.0) as i32;
        let perp = if d.length() > 0.0 { Vec2::new(-d.y, d.x).normalize() } else { Vec2::ZERO };
        for i in 0..=n {
            let p = a + d * (i as f32 / n as f32);
            for off in [0.0f32, 1.0, -1.0] {
                if off != 0.0 && i % 2 == 1 {
                    continue;
                }
                let q = p + perp * off;
                if !self.mundo_eh_oceano(q) {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Clone, Copy, PartialEq)]
struct No {
    f: f32,
    idx: u32,
}
impl Eq for No {}
impl Ord for No {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.total_cmp(&self.f) // min-heap
    }
}
impl PartialOrd for No {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Rota marítima suavizada, em coordenadas de mundo.
#[derive(Debug, Clone)]
pub struct Rota {
    pub pontos: Vec<Vec2>,
    /// Distância acumulada até cada ponto.
    pub acumulado: Vec<f32>,
    /// Passagens por canal: (distância ao longo da rota, canal).
    pub marcas: Vec<(f32, Canal)>,
}

impl Rota {
    pub fn comprimento(&self) -> f32 {
        *self.acumulado.last().unwrap_or(&0.0)
    }

    /// Posição e direção (unitária) na distância `d` ao longo da rota.
    pub fn amostrar(&self, d: f32) -> (Vec2, Vec2) {
        let n = self.pontos.len();
        if n < 2 {
            return (self.pontos.first().copied().unwrap_or(Vec2::ZERO), Vec2::X);
        }
        let d = d.clamp(0.0, self.comprimento());
        let i = match self.acumulado.binary_search_by(|v| v.total_cmp(&d)) {
            Ok(i) => i.min(n - 2),
            Err(i) => i.saturating_sub(1).min(n - 2),
        };
        let seg = self.pontos[i + 1] - self.pontos[i];
        let len = (self.acumulado[i + 1] - self.acumulado[i]).max(1e-4);
        (self.pontos[i] + seg * ((d - self.acumulado[i]) / len), seg / len)
    }

    fn de_pontos(pontos: Vec<Vec2>, marcas_pos: Vec<(usize, Canal)>) -> Self {
        let mut acumulado = Vec::with_capacity(pontos.len());
        let mut s = 0.0;
        for (i, p) in pontos.iter().enumerate() {
            if i > 0 {
                s += p.distance(pontos[i - 1]);
            }
            acumulado.push(s);
        }
        // marca no ponto médio do segmento (i-1 -> i) do portal
        let marcas = marcas_pos
            .into_iter()
            .map(|(i, c)| ((acumulado[i] + acumulado[i - 1]) * 0.5, c))
            .collect();
        Self { pontos, acumulado, marcas }
    }
}

/// Resultado de uma busca: custo e rota.
#[derive(Debug, Clone)]
pub struct Percurso {
    pub custo: f32,
    pub rota: Rota,
}

struct PortalCelula {
    destino: u32,
    canal: Option<Canal>,
}

/// Grade de navegação + portais + buscador A*.
pub struct Roteador {
    pub mascara: Mascara,
    gl: usize,
    ga: usize,
    /// célula é mar (todos os pixels 2x2 são mar)
    mar: Vec<bool>,
    /// 1 = célula de mar vizinha de terra (custo maior)
    costa: Vec<bool>,
    /// componente conexa (incluindo portais); 0 = terra
    comp: Vec<u32>,
    comp_principal: u32,
    portais: HashMap<u32, Vec<PortalCelula>>,
    /// Portais que não conseguiram ser ancorados no mar (para diagnóstico).
    pub portais_falhos: Vec<usize>,
}

impl Roteador {
    pub fn carregar(caminho_png: &Path) -> Result<Self, String> {
        Ok(Self::novo(Mascara::carregar(caminho_png)?, true))
    }

    /// `com_portais`: usa [`PORTAIS`] (coordenadas reais; só faz sentido no mapa do jogo).
    pub fn novo(mascara: Mascara, com_portais: bool) -> Self {
        let (gl, ga) = (mascara.largura / FATOR_GRADE, mascara.altura / FATOR_GRADE);
        let mut mar = vec![false; gl * ga];
        for cy in 0..ga {
            for cx in 0..gl {
                let (x, y) = ((cx * FATOR_GRADE) as i32, (cy * FATOR_GRADE) as i32);
                mar[cy * gl + cx] = (0..FATOR_GRADE as i32)
                    .all(|dy| (0..FATOR_GRADE as i32).all(|dx| mascara.eh_oceano(x + dx, y + dy)));
            }
        }
        let mut costa = vec![false; gl * ga];
        for cy in 0..ga as i32 {
            for cx in 0..gl as i32 {
                if !mar[cy as usize * gl + cx as usize] {
                    continue;
                }
                'viz: for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if nx < 0 || ny < 0 || nx >= gl as i32 || ny >= ga as i32 {
                            continue;
                        }
                        if !mar[ny as usize * gl + nx as usize] {
                            costa[cy as usize * gl + cx as usize] = true;
                            break 'viz;
                        }
                    }
                }
            }
        }
        let mut r = Self {
            mascara,
            gl,
            ga,
            mar,
            costa,
            comp: Vec::new(),
            comp_principal: 0,
            portais: HashMap::new(),
            portais_falhos: Vec::new(),
        };
        if com_portais {
            r.ancorar_portais();
        }
        r.rotular_componentes();
        r
    }

    fn idx(&self, cx: usize, cy: usize) -> u32 {
        (cy * self.gl + cx) as u32
    }
    fn xy(&self, i: u32) -> (usize, usize) {
        (i as usize % self.gl, i as usize / self.gl)
    }

    fn geo_para_celula(&self, lon: f64, lat: f64) -> (i32, i32) {
        let (px, py) = projetar_miller(lon, lat, self.mascara.largura as u32, self.mascara.altura as u32);
        (px as i32 / FATOR_GRADE as i32, py as i32 / FATOR_GRADE as i32)
    }

    /// Centro (mundo) da célula `i`.
    pub fn celula_para_mundo(&self, i: u32) -> Vec2 {
        let (cx, cy) = self.xy(i);
        let f = FATOR_GRADE as f32;
        self.mascara.pixel_para_mundo(cx as f32 * f + f * 0.5, cy as f32 * f + f * 0.5)
    }

    /// Célula de mar mais próxima de (cx, cy) dentro do raio (anéis crescentes).
    fn mar_proximo(&self, cx: i32, cy: i32, raio: i32, so_principal: bool) -> Option<u32> {
        for r in 0..=raio {
            let mut melhor: Option<(i32, u32)> = None;
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    if x < 0 || y < 0 || x >= self.gl as i32 || y >= self.ga as i32 {
                        continue;
                    }
                    let i = self.idx(x as usize, y as usize);
                    if !self.mar[i as usize] {
                        continue;
                    }
                    if so_principal && self.comp_principal != 0 && self.comp[i as usize] != self.comp_principal {
                        continue;
                    }
                    let d = dx * dx + dy * dy;
                    if melhor.is_none_or(|(bd, _)| d < bd) {
                        melhor = Some((d, i));
                    }
                }
            }
            if let Some((_, i)) = melhor {
                return Some(i);
            }
        }
        None
    }

    fn ancorar_portais(&mut self) {
        for (k, p) in PORTAIS.iter().enumerate() {
            let (ax, ay) = self.geo_para_celula(p.a.0, p.a.1);
            let (bx, by) = self.geo_para_celula(p.b.0, p.b.1);
            let (a, b) = (self.mar_proximo(ax, ay, 14, false), self.mar_proximo(bx, by, 14, false));
            match (a, b) {
                (Some(a), Some(b)) if a != b => {
                    self.portais.entry(a).or_default().push(PortalCelula { destino: b, canal: p.canal });
                    self.portais.entry(b).or_default().push(PortalCelula { destino: a, canal: p.canal });
                }
                _ => self.portais_falhos.push(k),
            }
        }
    }

    fn rotular_componentes(&mut self) {
        let n = self.gl * self.ga;
        let mut comp = vec![0u32; n];
        let mut prox = 1u32;
        let mut tamanho: HashMap<u32, usize> = HashMap::new();
        let mut pilha = Vec::new();
        for s in 0..n {
            if !self.mar[s] || comp[s] != 0 {
                continue;
            }
            comp[s] = prox;
            pilha.push(s as u32);
            let mut cont = 0;
            while let Some(c) = pilha.pop() {
                cont += 1;
                let (cx, cy) = self.xy(c);
                let visitar = |v: u32, pilha: &mut Vec<u32>, comp: &mut Vec<u32>| {
                    if self.mar[v as usize] && comp[v as usize] == 0 {
                        comp[v as usize] = prox;
                        pilha.push(v);
                    }
                };
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= self.gl as i32 || ny >= self.ga as i32 {
                        continue;
                    }
                    if dx != 0 && dy != 0
                        && (!self.mar[self.idx(nx as usize, cy) as usize] || !self.mar[self.idx(cx, ny as usize) as usize])
                    {
                        continue;
                    }
                    visitar(self.idx(nx as usize, ny as usize), &mut pilha, &mut comp);
                }
                if let Some(ps) = self.portais.get(&c) {
                    for p in ps {
                        visitar(p.destino, &mut pilha, &mut comp);
                    }
                }
            }
            tamanho.insert(prox, cont);
            prox += 1;
        }
        self.comp_principal = tamanho.iter().max_by_key(|(_, t)| **t).map(|(c, _)| *c).unwrap_or(0);
        self.comp = comp;
    }

    /// Ancora um ponto de mundo (ex.: porto) no mar principal mais próximo. `raio_px` em pixels.
    pub fn ancorar(&self, p: Vec2, raio_px: i32) -> Option<u32> {
        let (px, py) = self.mascara.mundo_para_pixel(p);
        self.mar_proximo(px / FATOR_GRADE as i32, py / FATOR_GRADE as i32, raio_px / FATOR_GRADE as i32, true)
    }

    pub fn ancorar_geo(&self, lon: f32, lat: f32, raio_px: i32) -> Option<u32> {
        let (cx, cy) = self.geo_para_celula(lon as f64, lat as f64);
        self.mar_proximo(cx, cy, raio_px / FATOR_GRADE as i32, true)
    }

    /// Caminho mínimo (A*) entre duas células de mar. `None` se desconectadas.
    pub fn caminho(&self, de: u32, para: u32) -> Option<Percurso> {
        self.caminho_sem(de, para, &[])
    }

    /// Como [`Self::caminho`], mas sem usar os canais `fechados` (a rota contorna, ex.: Suez fechado
    /// faz o navio dar a volta pela África). `None` se o destino só é alcançável pelo canal fechado.
    pub fn caminho_sem(&self, de: u32, para: u32, fechados: &[Canal]) -> Option<Percurso> {
        if self.comp[de as usize] == 0 || self.comp[de as usize] != self.comp[para as usize] {
            return None;
        }
        let n = self.gl * self.ga;
        let mut g = vec![f32::INFINITY; n];
        let mut pai = vec![u32::MAX; n];
        // célula destino -> 0 = passo normal; 1 = portal sem canal contado; 2+k = Canal::TODOS[k]
        let mut via = vec![0u8; n];
        let alvo = self.celula_para_mundo(para);
        let h = |i: u32| self.celula_para_mundo(i).distance(alvo) / FATOR_GRADE as f32;
        let mut heap = BinaryHeap::new();
        g[de as usize] = 0.0;
        heap.push(No { f: h(de), idx: de });
        while let Some(No { idx, .. }) = heap.pop() {
            if idx == para {
                break;
            }
            let gi = g[idx as usize];
            let (cx, cy) = self.xy(idx);
            let mut relaxar = |v: u32, custo: f32, portal: Option<Option<Canal>>, heap: &mut BinaryHeap<No>| {
                let ng = gi + custo;
                if ng < g[v as usize] {
                    g[v as usize] = ng;
                    pai[v as usize] = idx;
                    via[v as usize] = match portal {
                        None => 0,
                        Some(None) => 1,
                        Some(Some(c)) => 2 + Canal::TODOS.iter().position(|x| *x == c).unwrap() as u8,
                    };
                    heap.push(No { f: ng + h(v), idx: v });
                }
            };
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
                if nx < 0 || ny < 0 || nx >= self.gl as i32 || ny >= self.ga as i32 {
                    continue;
                }
                let v = self.idx(nx as usize, ny as usize);
                if !self.mar[v as usize] {
                    continue;
                }
                // diagonal só se os dois ortogonais forem mar (não "vazar" entre cantos de terra)
                if dx != 0 && dy != 0 {
                    let o1 = self.idx(nx as usize, cy);
                    let o2 = self.idx(cx, ny as usize);
                    if !self.mar[o1 as usize] || !self.mar[o2 as usize] {
                        continue;
                    }
                }
                let base = if dx != 0 && dy != 0 { std::f32::consts::SQRT_2 } else { 1.0 };
                let fator = if self.costa[v as usize] { 1.7 } else { 1.0 };
                relaxar(v, base * fator, None, &mut heap);
            }
            if let Some(ps) = self.portais.get(&idx) {
                for p in ps {
                    if p.canal.is_some_and(|c| fechados.contains(&c)) {
                        continue;
                    }
                    let d = self.celula_para_mundo(idx).distance(self.celula_para_mundo(p.destino)) / FATOR_GRADE as f32;
                    relaxar(p.destino, d.max(0.5), Some(p.canal), &mut heap);
                }
            }
        }
        if !g[para as usize].is_finite() {
            return None;
        }
        // reconstrução
        let mut cel = vec![para];
        let mut cur = para;
        while cur != de {
            cur = pai[cur as usize];
            cel.push(cur);
        }
        cel.reverse();
        // pedaços separados por passos de portal
        let mut trechos: Vec<Vec<u32>> = vec![vec![cel[0]]];
        let mut canais_entre: Vec<Option<Canal>> = Vec::new();
        for w in cel.windows(2) {
            let (a, b) = (w[0], w[1]);
            let _ = a;
            let v = via[b as usize];
            if v != 0 {
                canais_entre.push(if v >= 2 { Some(Canal::TODOS[v as usize - 2]) } else { None });
                trechos.push(vec![b]);
            } else {
                trechos.last_mut().unwrap().push(b);
            }
        }
        let mut pontos: Vec<Vec2> = Vec::new();
        let mut marcas_pos: Vec<(usize, Canal)> = Vec::new();
        for (k, t) in trechos.iter().enumerate() {
            if k > 0 {
                if let Some(c) = canais_entre[k - 1] {
                    marcas_pos.push((pontos.len(), c)); // segmento (len-1 -> len) é o portal
                }
            }
            let mundo: Vec<Vec2> = t.iter().map(|&c| self.celula_para_mundo(c)).collect();
            pontos.extend(self.suavizar(&mundo));
        }
        Some(Percurso { custo: g[para as usize] * FATOR_GRADE as f32, rota: Rota::de_pontos(pontos, marcas_pos) })
    }

    /// "String pulling": remove pontos intermediários com linha de visada livre.
    fn suavizar(&self, p: &[Vec2]) -> Vec<Vec2> {
        if p.len() <= 2 {
            return p.to_vec();
        }
        let mut saida = vec![p[0]];
        let mut i = 0;
        while i < p.len() - 1 {
            let limite = (i + 80).min(p.len() - 1);
            let mut j = limite;
            while j > i + 1 && !self.mascara.linha_livre(p[i], p[j]) {
                j -= 1;
            }
            saida.push(p[j]);
            i = j;
        }
        saida
    }
}

/// Posição de mundo de (lon, lat) no mapa padrão.
pub fn geo_mundo(lon: f32, lat: f32) -> Vec2 {
    crate::projecao::geo_para_mundo(lon, lat)
}

/// Caminho padrão da máscara no repositório.
pub const MASCARA_PADRAO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/map/provinces.png");

/// Dimensões padrão (documentação/testes).
pub const DIM_PADRAO: (u32, u32) = (LARGURA_MAPA_PADRAO, ALTURA_MAPA_PADRAO);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rota_contorna_ilha_sintetica() {
        // mar com um paredão de terra no meio, com abertura embaixo
        let m = Mascara::de_funcao(200, 100, |x, y| !(x >= 98 && x < 104 && y < 80));
        let r = Roteador::novo(m, false);
        let a = r.ancorar(Vec2::new(-80.0, 0.0), 10).unwrap();
        let b = r.ancorar(Vec2::new(80.0, 0.0), 10).unwrap();
        let p = r.caminho(a, b).unwrap();
        assert!(p.rota.comprimento() > 170.0);
        for w in p.rota.pontos.windows(2) {
            assert!(r.mascara.linha_livre(w[0], w[1]));
        }
    }

    #[test]
    fn rota_amostrar() {
        let rota = Rota::de_pontos(vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0)], vec![]);
        let (p, d) = rota.amostrar(15.0);
        assert!((p - Vec2::new(10.0, 5.0)).length() < 1e-4 && (d - Vec2::Y).length() < 1e-4);
    }

    fn real() -> Option<Roteador> {
        Roteador::carregar(Path::new(MASCARA_PADRAO)).ok()
    }

    #[test]
    fn mapa_real_portais_e_rotas() {
        let Some(r) = real() else { return };
        assert!(r.portais_falhos.is_empty(), "portais sem ancora: {:?}", r.portais_falhos);
        let nova_york = r.ancorar_geo(-74.0, 40.7, 60).expect("NY");
        let londres = r.ancorar_geo(0.5, 51.5, 80).expect("Londres");
        let bombaim = r.ancorar_geo(72.8, 18.9, 60).expect("Bombaim");
        let sfo = r.ancorar_geo(-122.4, 37.8, 60).expect("SF");
        let odessa = r.ancorar_geo(30.7, 46.5, 80).expect("Odessa");
        let leningrado = r.ancorar_geo(30.2, 59.9, 80).expect("Leningrado");
        let nyl = r.caminho(nova_york, londres).expect("NY-Londres");
        assert!(nyl.rota.marcas.is_empty(), "NY-Londres não deve usar canal: {:?}", nyl.rota.marcas);
        let lb = r.caminho(londres, bombaim).expect("Londres-Bombaim");
        assert!(lb.rota.marcas.iter().any(|(_, c)| *c == Canal::Suez), "{:?}", lb.rota.marcas);
        let ns = r.caminho(nova_york, sfo).expect("NY-SF");
        assert!(ns.rota.marcas.iter().any(|(_, c)| *c == Canal::Panama), "{:?}", ns.rota.marcas);
        let oly = r.caminho(odessa, londres).expect("Odessa-Londres");
        assert!(oly.rota.marcas.iter().any(|(_, c)| *c == Canal::Bosforo), "{:?}", oly.rota.marcas);
        assert!(r.caminho(leningrado, londres).is_some(), "Báltico desconectado");
        // nenhum segmento fora de portal atravessa terra
        for p in [&nyl, &lb, &ns, &oly] {
            let portais: Vec<f32> = p.rota.marcas.iter().map(|m| m.0).collect();
            for (i, w) in p.rota.pontos.windows(2).enumerate() {
                let meio = (p.rota.acumulado[i] + p.rota.acumulado[i + 1]) * 0.5;
                if portais.iter().any(|d| (d - meio).abs() < 1.0) {
                    continue;
                }
                if !r.mascara.linha_livre(w[0], w[1]) {
                    // segmentos de portais não contados (Dardanelos etc.) podem falhar; tolera só curtos
                    if w[0].distance(w[1]) > 60.0 {
                        panic!("segmento longo atravessa terra: {:?}->{:?}", w[0], w[1]);
                    }
                }
            }
        }
    }

    #[test]
    fn suez_fechado_contorna_a_africa() {
        let Some(r) = real() else { return };
        let londres = r.ancorar_geo(0.5, 51.5, 80).expect("Londres");
        let bombaim = r.ancorar_geo(72.8, 18.9, 60).expect("Bombaim");
        let aberto = r.caminho(londres, bombaim).expect("aberto");
        assert!(aberto.rota.marcas.iter().any(|(_, c)| *c == Canal::Suez));
        let fechado = r.caminho_sem(londres, bombaim, &[Canal::Suez]).expect("contorno");
        assert!(fechado.rota.marcas.iter().all(|(_, c)| *c != Canal::Suez));
        assert!(fechado.custo > aberto.custo * 1.3, "contorno {} vs {}", fechado.custo, aberto.custo);
    }
}
