//! Gerenciamento das 5 eras tecnológicas da Guerra Fria e evolução visual de paletas e arquitetura.

use crate::dados::{EraVisual, eras_padrao_reserva};
use crate::entrada::AnoVisual;
use bevy::prelude::*;

/// Recurso que gerencia as eras e calcula paletas interpoladas suavemente.
#[derive(Resource, Debug, Clone)]
pub struct GerenciadorEras {
    pub eras: Vec<EraVisual>,
    pub indice_atual: usize,
    /// Fator de transição suave entre a era atual e a próxima [0.0, 1.0].
    pub fator_transicao: f32,
    /// Se a iluminação noturna está ativa no mapa.
    pub modo_noturno: bool,
}

impl Default for GerenciadorEras {
    fn default() -> Self {
        let eras = eras_padrao_reserva();
        Self {
            eras,
            indice_atual: 0,
            fator_transicao: 0.0,
            modo_noturno: false,
        }
    }
}

impl GerenciadorEras {
    /// Retorna a especificação da era atual ativa.
    pub fn era_atual(&self) -> &EraVisual {
        &self.eras[self.indice_atual.min(self.eras.len().saturating_sub(1))]
    }

    /// Retorna a próxima era (ou a atual se for a última).
    pub fn proxima_era(&self) -> &EraVisual {
        let prox = (self.indice_atual + 1).min(self.eras.len().saturating_sub(1));
        &self.eras[prox]
    }

    /// Retorna a cor primária (teto / fachada principal) interpolada.
    pub fn cor_primaria_bevy(&self) -> Color {
        let c1 = rgb_tupla_para_color(self.era_atual().cor_primaria);
        let c2 = rgb_tupla_para_color(self.proxima_era().cor_primaria);
        lerp_color(c1, c2, self.fator_transicao)
    }

    /// Retorna a cor de concreto / estrutura interpolada.
    pub fn cor_concreto_bevy(&self) -> Color {
        let c1 = rgb_tupla_para_color(self.era_atual().cor_concreto);
        let c2 = rgb_tupla_para_color(self.proxima_era().cor_concreto);
        lerp_color(c1, c2, self.fator_transicao)
    }

    /// Retorna a cor da fumaça de chaminés (cinza-clara e semitransparente).
    pub fn cor_fumaca_bevy(&self) -> Color {
        if self.modo_noturno {
            Color::srgba(0.70, 0.72, 0.80, 0.25)
        } else {
            Color::srgba(0.85, 0.85, 0.88, 0.30)
        }
    }

    /// Retorna a cor das janelas dos prédios (com iluminação noturna amarelada/néon opcional).
    pub fn cor_janela_bevy(&self) -> Color {
        if self.modo_noturno {
            let c1 = rgb_tupla_para_color(self.era_atual().cor_janela_noite);
            let c2 = rgb_tupla_para_color(self.proxima_era().cor_janela_noite);
            lerp_color(c1, c2, self.fator_transicao)
        } else {
            let c1 = rgb_tupla_para_color(self.era_atual().cor_janela_dia);
            let c2 = rgb_tupla_para_color(self.proxima_era().cor_janela_dia);
            lerp_color(c1, c2, self.fator_transicao)
        }
    }

    /// Escala de altura média dos edifícios (cresce de vilas baixas até arranha-céus modernos).
    pub fn multiplicador_altura(&self) -> f32 {
        let h1 = self.era_atual().altura_predio_escala;
        let h2 = self.proxima_era().altura_predio_escala;
        h1 + (h2 - h1) * self.fator_transicao
    }

    /// Prevalência de arranha-céus modernos [0.05 a 0.85].
    pub fn taxa_arranha_ceus(&self) -> f32 {
        let a1 = self.era_atual().fator_arranha_ceus;
        let a2 = self.proxima_era().fator_arranha_ceus;
        a1 + (a2 - a1) * self.fator_transicao
    }
}

/// Converte tupla (u8, u8, u8) para Bevy Color sRGB.
#[inline]
pub fn rgb_tupla_para_color(rgb: (u8, u8, u8)) -> Color {
    Color::srgb(
        rgb.0 as f32 / 255.0,
        rgb.1 as f32 / 255.0,
        rgb.2 as f32 / 255.0,
    )
}

/// Interpolação linear entre duas cores Bevy no espaço linear.
#[inline]
pub fn lerp_color(c1: Color, c2: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let v1 = c1.to_linear().to_vec4();
    let v2 = c2.to_linear().to_vec4();
    let res = v1.lerp(v2, t);
    Color::linear_rgba(res.x, res.y, res.z, res.w)
}

/// Índice da era (0..=4) para um ano: 1945-54 = 0, 1955-64 = 1, 1965-74 = 2, 1975-84 = 3, 1985+ = 4.
pub fn indice_era_para_ano(ano: f32) -> usize {
    let a = ano.floor() as i32;
    if a < 1955 {
        0
    } else if a < 1965 {
        1
    } else if a < 1975 {
        2
    } else if a < 1985 {
        3
    } else {
        4
    }
}

/// Fator de transição [0,1] para a próxima era; ocorre nos 2 anos finais de cada era.
pub fn fator_transicao_para_ano(ano: f32) -> f32 {
    let idx = indice_era_para_ano(ano);
    if idx >= 4 {
        return 0.0;
    }
    let limiar = [1955.0, 1965.0, 1975.0, 1985.0][idx];
    if ano >= limiar - 2.0 { ((ano - (limiar - 2.0)) / 2.0).clamp(0.0, 1.0) } else { 0.0 }
}

/// Sistema Bevy que sincroniza a era ativa e a transição com o [`AnoVisual`].
pub fn sincronizar_eras_sistema(ano: Res<AnoVisual>, mut gerenciador: ResMut<GerenciadorEras>) {
    let idx = indice_era_para_ano(ano.0);
    let fator = fator_transicao_para_ano(ano.0);
    // Compara antes de escrever para não marcar o recurso como alterado todo quadro.
    if gerenciador.indice_atual != idx || (gerenciador.fator_transicao - fator).abs() > 1e-4 {
        gerenciador.indice_atual = idx;
        gerenciador.fator_transicao = fator;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gerenciador_eras_cores() {
        let g = GerenciadorEras::default();
        assert_eq!(g.indice_atual, 0);
        let cor_ini = g.cor_primaria_bevy();
        assert!(cor_ini.to_linear().to_vec4().x > 0.0);
    }

    #[test]
    fn indice_e_transicao_por_ano() {
        assert_eq!(indice_era_para_ano(1945.0), 0);
        assert_eq!(indice_era_para_ano(1954.99), 0);
        assert_eq!(indice_era_para_ano(1955.0), 1);
        assert_eq!(indice_era_para_ano(1974.0), 2);
        assert_eq!(indice_era_para_ano(1990.0), 4);
        assert_eq!(fator_transicao_para_ano(1950.0), 0.0);
        assert!((fator_transicao_para_ano(1954.0) - 0.5).abs() < 1e-5);
        assert_eq!(fator_transicao_para_ano(1989.0), 0.0);
    }

    #[test]
    fn test_lerp_color() {
        let preto = Color::srgb(0.0, 0.0, 0.0);
        let branco = Color::srgb(1.0, 1.0, 1.0);
        let cinza = lerp_color(preto, branco, 0.5);
        let v = cinza.to_linear().to_vec4();
        assert!((v.x - 0.5).abs() < 0.05);
    }
}
