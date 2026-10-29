//! Construtor de malhas de linhas (polilinhas com espessura e cor por vértice).
//!
//! Todas as camadas lineares do mapa (rios, estradas, ferrovias) usam este construtor: a malha é
//! gerada uma única vez e só é refeita quando os dados mudam.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Acumula quadriláteros e produz um único [`Mesh`] com cores por vértice.
#[derive(Default)]
pub struct ConstrutorLinhas {
    pos: Vec<[f32; 3]>,
    cor: Vec<[f32; 4]>,
    idx: Vec<u32>,
}

impl ConstrutorLinhas {
    pub fn novo() -> Self {
        Self::default()
    }

    pub fn vazio(&self) -> bool {
        self.idx.is_empty()
    }

    pub fn vertices(&self) -> usize {
        self.pos.len()
    }

    /// Segmento reto de `a` a `b` com largura `w`. `ext` prolonga as pontas (esconde frestas nas juntas).
    pub fn segmento(&mut self, a: Vec2, b: Vec2, w: f32, ext: f32, cor: Color) {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 || w <= 0.0 {
            return;
        }
        let dir = d / len;
        let n = Vec2::new(-dir.y, dir.x) * (w * 0.5);
        let (a, b) = (a - dir * ext, b + dir * ext);
        self.quad([a - n, a + n, b + n, b - n], [cor; 4]);
    }

    /// Segmento com cor diferente em cada ponta (gradientes, brilho).
    pub fn segmento_grad(&mut self, a: Vec2, b: Vec2, w: f32, ca: Color, cb: Color) {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 || w <= 0.0 {
            return;
        }
        let n = Vec2::new(-d.y, d.x) / len * (w * 0.5);
        self.quad([a - n, a + n, b + n, b - n], [ca, ca, cb, cb]);
    }

    /// Polilinha com largura constante.
    pub fn polilinha(&mut self, pts: &[Vec2], w: f32, cor: Color) {
        for par in pts.windows(2) {
            self.segmento(par[0], par[1], w, w * 0.45, cor);
        }
    }

    /// Quadrilátero qualquer (ordem dos vértices em volta).
    pub fn quad(&mut self, p: [Vec2; 4], c: [Color; 4]) {
        let base = self.pos.len() as u32;
        for i in 0..4 {
            self.pos.push([p[i].x, p[i].y, 0.0]);
            self.cor.push(c[i].to_linear().to_f32_array());
        }
        self.idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub fn construir(self) -> Mesh {
        let n = self.pos.len();
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.cor);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n]);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
        m.insert_indices(Indices::U32(self.idx));
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polilinha_gera_quads() {
        let mut c = ConstrutorLinhas::novo();
        c.polilinha(&[Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0)], 2.0, Color::WHITE);
        assert_eq!(c.vertices(), 8);
        assert!(!c.vazio());
    }
}
