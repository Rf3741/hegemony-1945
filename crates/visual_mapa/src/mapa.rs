//! Modos de mapa (Político, Industrial, Urbanização): estado, transição suave e paletas de cor.
//! A pintura das regiões em si é do cliente (T19); aqui ficam as cores por modo.

use crate::era::lerp_color;
use bevy::prelude::*;

/// Modos de visualização temáticos do mapa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ModoMapa {
    #[default]
    Politico,
    Industrial,
    Urbanizacao,
}

impl ModoMapa {
    pub fn nome_pt(&self) -> &'static str {
        match self {
            Self::Politico => "Modo Político",
            Self::Industrial => "Modo Industrial",
            Self::Urbanizacao => "Modo Urbanização",
        }
    }
}

/// Recurso que armazena o modo de mapa ativo e interpola transições suaves entre modos.
#[derive(Resource, Debug, Clone)]
pub struct EstadoModoMapa {
    pub modo_atual: ModoMapa,
    pub modo_alvo: ModoMapa,
    /// Progresso da transição suave entre 0.0 (modo_atual) e 1.0 (modo_alvo).
    pub progresso: f32,
}

impl Default for EstadoModoMapa {
    fn default() -> Self {
        Self {
            modo_atual: ModoMapa::Politico,
            modo_alvo: ModoMapa::Politico,
            progresso: 1.0,
        }
    }
}

impl EstadoModoMapa {
    /// Dispara a transição para um novo modo de mapa.
    pub fn transicionar_para(&mut self, novo_modo: ModoMapa) {
        if self.modo_alvo != novo_modo {
            self.modo_atual = self.modo_alvo;
            self.modo_alvo = novo_modo;
            self.progresso = 0.0;
        }
    }

    /// Atualiza o progresso da animação de transição linear suave.
    pub fn atualizar(&mut self, dt: f32) {
        if self.progresso < 1.0 {
            // Transição dura aproximadamente 0.45 segundo
            self.progresso = (self.progresso + dt * 2.2).min(1.0);
            if self.progresso >= 1.0 {
                self.modo_atual = self.modo_alvo;
            }
        }
    }

    /// Retorna a cor calculada para uma dada região interpolando entre o modo atual e o alvo.
    pub fn calcular_cor_regiao(
        &self,
        pais_tag: &str,
        densidade_industrial: f32,
        taxa_urbanizacao: f32,
    ) -> Color {
        let cor_ini = self.cor_por_modo(self.modo_atual, pais_tag, densidade_industrial, taxa_urbanizacao);
        let cor_fim = self.cor_por_modo(self.modo_alvo, pais_tag, densidade_industrial, taxa_urbanizacao);
        lerp_color(cor_ini, cor_fim, self.progresso)
    }

    fn cor_por_modo(
        &self,
        modo: ModoMapa,
        pais_tag: &str,
        densidade_industrial: f32,
        taxa_urbanizacao: f32,
    ) -> Color {
        match modo {
            ModoMapa::Politico => cor_pais_historica(pais_tag),
            ModoMapa::Industrial => cor_heatmap_industrial(densidade_industrial),
            ModoMapa::Urbanizacao => cor_gradiente_urbanizacao(taxa_urbanizacao),
        }
    }
}

/// Cores históricas de 1945 fiéis ao catálogo nacional (EUA azul navy, URSS vermelho rubro, etc.).
pub fn cor_pais_historica(tag: &str) -> Color {
    match tag {
        "USA" => Color::srgb(0.17, 0.37, 0.68), // Azul naval americano #2B5EAE
        "SOV" => Color::srgb(0.68, 0.10, 0.10), // Vermelho comissário soviético #AE1A1A
        "GBR" => Color::srgb(0.75, 0.20, 0.25), // Vermelho imperial britânico #C03340
        "FRA" => Color::srgb(0.18, 0.45, 0.84), // Azul real francês #2E73D6
        "GER" => Color::srgb(0.33, 0.33, 0.37), // Cinza feldgrau ocupado #54545E
        "JAP" => Color::srgb(0.92, 0.86, 0.70), // Areia imperial japonesa #EBDCB2
        "CHI" => Color::srgb(0.25, 0.60, 0.88), // Azul celeste Kuomintang #409AE0
        "CCP" => Color::srgb(0.88, 0.25, 0.18), // Laranja avermelhado revolucionário
        "BRA" => Color::srgb(0.14, 0.57, 0.25), // Verde floresta luso-brasileiro #249140
        "IND" => Color::srgb(0.86, 0.59, 0.39), // Caqui Raj britânico #DC9763
        "CAN" => Color::srgb(0.82, 0.30, 0.32), // Vermelho folha de bordo
        "AST" => Color::srgb(0.22, 0.52, 0.58), // Azul oceânico australiano
        "ITA" => Color::srgb(0.20, 0.60, 0.38), // Verde italiano
        "SWE" => Color::srgb(0.30, 0.50, 0.75), // Azul nórdico neutro
        "TUR" => Color::srgb(0.65, 0.22, 0.22), // Carmesim otomano
        "ARG" => Color::srgb(0.45, 0.68, 0.88), // Azul celeste platino
        "MEX" => Color::srgb(0.18, 0.52, 0.34), // Verde asteca
        "EGY" => Color::srgb(0.80, 0.70, 0.45), // Dourado do Nilo
        "SAF" => Color::srgb(0.70, 0.55, 0.30), // Ocre sul-africano
        "POL" => Color::srgb(0.78, 0.24, 0.32), // Carmim polonês
        _ => Color::srgb(0.40, 0.44, 0.42),     // Neutro / Não-alinhado #66706B
    }
}

/// Heatmap para o modo industrial (do cinza/marrom agrário ao verde esmeralda e ciano de alta tecnologia).
pub fn cor_heatmap_industrial(densidade: f32) -> Color {
    let d = densidade.clamp(0.0, 1.0);
    if d < 0.25 {
        let t = d / 0.25;
        lerp_color(Color::srgb(0.23, 0.21, 0.18), Color::srgb(0.36, 0.32, 0.24), t)
    } else if d < 0.60 {
        let t = (d - 0.25) / 0.35;
        lerp_color(Color::srgb(0.36, 0.32, 0.24), Color::srgb(0.31, 0.52, 0.38), t)
    } else {
        let t = (d - 0.60) / 0.40;
        lerp_color(Color::srgb(0.31, 0.52, 0.38), Color::srgb(0.24, 0.75, 0.62), t)
    }
}

/// Gradiente para o modo de urbanização (do bege rural ao ocre e marrom urbano denso).
pub fn cor_gradiente_urbanizacao(taxa: f32) -> Color {
    let u = taxa.clamp(0.0, 1.0);
    if u < 0.30 {
        let t = u / 0.30;
        lerp_color(Color::srgb(0.85, 0.81, 0.72), Color::srgb(0.76, 0.70, 0.55), t)
    } else if u < 0.70 {
        let t = (u - 0.30) / 0.40;
        lerp_color(Color::srgb(0.76, 0.70, 0.55), Color::srgb(0.58, 0.45, 0.30), t)
    } else {
        let t = (u - 0.70) / 0.30;
        lerp_color(Color::srgb(0.58, 0.45, 0.30), Color::srgb(0.32, 0.18, 0.12), t)
    }
}

/// Componente que marca um setor territorial/região do mapa mundi.
#[derive(Component, Debug, Clone)]
pub struct SetorRegiao {
    pub adm1_code: String,
    pub nome: String,
    pub pais_tag: String,
    pub centroide: Vec2,
    pub densidade_industrial: f32,
    pub taxa_urbanizacao: f32,
}

/// Sistema Bevy que atualiza a transição suave de modos de mapa.
pub fn atualizar_transicao_mapa_sistema(
    time: Res<Time>,
    mut estado_mapa: ResMut<EstadoModoMapa>,
    estilo: Option<Res<crate::estilo::EstiloMapa>>,
) {
    // `transicao_modo_s` é a duração desejada; o passo base dura ~0.45 s.
    let dur = estilo.map_or(0.45, |e| e.transicao_modo_s.max(0.05));
    estado_mapa.atualizar(time.delta_secs() * 0.45 / dur);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transicao_modos_mapa() {
        let mut estado = EstadoModoMapa::default();
        assert_eq!(estado.modo_atual, ModoMapa::Politico);

        estado.transicionar_para(ModoMapa::Industrial);
        assert_eq!(estado.modo_alvo, ModoMapa::Industrial);
        assert_eq!(estado.progresso, 0.0);

        estado.atualizar(0.5);
        assert!(estado.progresso > 0.9);

        estado.atualizar(0.5);
        assert_eq!(estado.modo_atual, ModoMapa::Industrial);
        assert_eq!(estado.progresso, 1.0);
    }
}
