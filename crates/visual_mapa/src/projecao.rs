//! Projeção geográfica -> mundo do Bevy, unificada com o `gf_mapa` (Miller).
//!
//! O mapa do cliente é um sprite `LARGURA x ALTURA` centrado na origem (x para a direita, y para
//! cima). A imagem tem y para baixo, então `mundo = (px - L/2, A/2 - py)`, com `px, py` vindos de
//! [`gf_mapa::mapa::projetar_miller`]. Assim uma cidade em (lon, lat) cai sobre o pixel certo
//! de `provinces.png`.

use bevy::prelude::*;
use gf_mapa::mapa::{
    ALTURA_MAPA_PADRAO, ESCALA_Y_MAPA, LARGURA_MAPA_PADRAO, desprojetar_miller, projetar_miller,
};

/// Dimensões do mapa em pixels (= unidades de mundo do sprite do mapa).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjecaoMapa {
    pub largura: u32,
    pub altura: u32,
}

impl Default for ProjecaoMapa {
    fn default() -> Self {
        Self { largura: LARGURA_MAPA_PADRAO, altura: ALTURA_MAPA_PADRAO }
    }
}

impl ProjecaoMapa {
    /// (longitude, latitude) em graus -> posição de mundo.
    pub fn geo_para_mundo(&self, lon: f32, lat: f32) -> Vec2 {
        let (px, py) = projetar_miller(lon as f64, lat as f64, self.largura, self.altura);
        Vec2::new(px as f32 - self.largura as f32 * 0.5, (self.altura as f32 * 0.5 - py as f32) * ESCALA_Y_MAPA)
    }

    /// Posição de mundo -> (longitude, latitude) em graus.
    pub fn mundo_para_geo(&self, p: Vec2) -> (f32, f32) {
        let px = p.x as f64 + self.largura as f64 * 0.5;
        let py = self.altura as f64 * 0.5 - p.y as f64 / ESCALA_Y_MAPA as f64;
        let (lon, lat) = desprojetar_miller(px, py, self.largura, self.altura);
        (lon as f32, lat as f32)
    }

    /// Posição de mundo -> pixel da imagem do mapa (y para baixo).
    pub fn mundo_para_pixel(&self, p: Vec2) -> (f32, f32) {
        (p.x + self.largura as f32 * 0.5, self.altura as f32 * 0.5 - p.y / ESCALA_Y_MAPA)
    }
}

/// Atalho com as dimensões padrão do mapa do jogo.
pub fn geo_para_mundo(lon: f32, lat: f32) -> Vec2 {
    ProjecaoMapa::default().geo_para_mundo(lon, lat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origem_no_centro_do_mapa() {
        let p = geo_para_mundo(0.0, 0.0);
        assert!(p.x.abs() < 0.01 && p.y.abs() < 0.01, "{p:?}");
    }

    #[test]
    fn cantos_e_sentido_dos_eixos() {
        let proj = ProjecaoMapa::default();
        let nw = proj.geo_para_mundo(-180.0, 90.0);
        assert!((nw.x + 2816.0).abs() < 0.5 && (nw.y - 1024.0 * ESCALA_Y_MAPA).abs() < 0.5, "{nw:?}");
        let leste = proj.geo_para_mundo(10.0, 0.0);
        let oeste = proj.geo_para_mundo(-10.0, 0.0);
        assert!(leste.x > oeste.x);
        let norte = proj.geo_para_mundo(0.0, 40.0);
        assert!(norte.y > 0.0);
    }

    #[test]
    fn ida_e_volta() {
        let proj = ProjecaoMapa::default();
        for (lon, lat) in [(-46.6, -23.5), (37.6, 55.7), (139.7, 35.7), (0.5, 0.5)] {
            let (l2, a2) = proj.mundo_para_geo(proj.geo_para_mundo(lon, lat));
            assert!((l2 - lon).abs() < 0.05 && (a2 - lat).abs() < 0.05, "{lon},{lat} -> {l2},{a2}");
        }
    }

    #[test]
    fn coincide_com_pixel_do_gf_mapa() {
        let proj = ProjecaoMapa::default();
        let (px, py) = projetar_miller(-74.0, 40.7, proj.largura, proj.altura);
        let (qx, qy) = proj.mundo_para_pixel(proj.geo_para_mundo(-74.0, 40.7));
        assert!((px as f32 - qx).abs() < 0.01 && (py as f32 - qy).abs() < 0.01);
    }
}
