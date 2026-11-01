//! Estilo do mapa (limpo, estilo Victoria 3 / Age of History 3): parâmetros de zoom e opacidade
//! num único Resource, e a camada que esconde construções fora do zoom de estado.
//!
//! A escala é a `scale` da câmera ortográfica (maior = mais longe; 0.1 a 4.0).

use crate::config::CamadaConstrucoesVisivel;
use crate::industria::InstalacaoIndustrial;
use crate::predio::PredioVisual;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Resource, Debug, Clone)]
pub struct EstiloMapa {
    /// Construções só aparecem com escala abaixo disto (zoom de estado).
    pub construcoes_escala_max: f32,
    /// Máximo de ícones de construção por estado.
    pub max_icones_por_estado: usize,
    /// Nomes de cidades só com escala abaixo disto (zoom de país para dentro).
    pub cidades_escala_max: f32,
    /// Estradas/ferrovias só com escala abaixo disto.
    pub rotas_escala_max: f32,
    /// Largura dos rios em pixels de tela (principais / menores).
    pub rios_px_principal: f32,
    pub rios_px_menor: f32,
    /// Rios menores só com escala abaixo disto.
    pub rios_menores_escala_max: f32,
    /// Opacidade dos rios.
    pub rios_alfa: f32,
    /// Nomes de país: totalmente visíveis acima de `pais_escala_cheio`, somem abaixo de `pais_escala_some`.
    pub pais_escala_cheio: f32,
    pub pais_escala_some: f32,
    /// Opacidade máxima do texto de país.
    pub pais_alfa: f32,
    /// Altura mínima do texto na tela (px) para ser mostrado.
    pub pais_altura_min_px: f32,
    /// Aplica o tema de gabinete de guerra (fontes e cores) na UI egui.
    pub tema_ui: bool,
    /// Oceano com ondas animadas (intensidade 0 desliga) e velocidade.
    /// Desligado por padrão (a animação do mar piscava); ligue para ondas suaves e lentas.
    pub oceano_ondas_ativo: bool,
    pub oceano_ondas_alfa: f32,
    pub oceano_ondas_vel: f32,
    /// Zoom suave: taxa de aproximação por segundo (0 = zoom instantâneo).
    pub zoom_suave: f32,
    /// Vinheta nas bordas da tela (0 a 1) e moldura de mapa antigo.
    pub vinheta_alfa: f32,
    pub moldura_antiga: bool,
    /// Transição suave entre modos de mapa: duração em segundos.
    pub transicao_modo_s: f32,
}

impl Default for EstiloMapa {
    fn default() -> Self {
        Self {
            construcoes_escala_max: 0.30,
            max_icones_por_estado: 3,
            cidades_escala_max: 1.4,
            rotas_escala_max: 0.50,
            rios_px_principal: 1.0,
            rios_px_menor: 0.6,
            rios_menores_escala_max: 0.9,
            rios_alfa: 0.5,
            pais_escala_cheio: 0.9,
            pais_escala_some: 0.4,
            pais_alfa: 0.62,
            pais_altura_min_px: 9.0,
            tema_ui: true,
            oceano_ondas_ativo: false,
            oceano_ondas_alfa: 0.08,
            oceano_ondas_vel: 0.12,
            zoom_suave: 12.0,
            vinheta_alfa: 0.35,
            moldura_antiga: true,
            transicao_modo_s: 0.35,
        }
    }
}

/// Mostra no máximo N construções por estado e só no zoom de estado.
pub fn limitar_construcoes_sistema(
    estilo: Res<EstiloMapa>,
    camada: Option<Res<CamadaConstrucoesVisivel>>,
    camera: Query<&Projection, With<Camera2d>>,
    novos: Query<(), Added<PredioVisual>>,
    mut predios: Query<(Entity, &PredioVisual, &mut Visibility), Without<InstalacaoIndustrial>>,
    mut insts: Query<&mut Visibility, (With<InstalacaoIndustrial>, Without<PredioVisual>)>,
    mut permitidos: Local<HashSet<Entity>>,
    mut total: Local<usize>,
) {
    let escala = camera
        .single()
        .ok()
        .map(|p| match p {
            Projection::Orthographic(o) => o.scale,
            _ => 1.0,
        })
        .unwrap_or(1.0);
    let ligada = camada.is_none_or(|c| c.0) && escala < estilo.construcoes_escala_max;
    for mut v in &mut insts {
        if *v != Visibility::Hidden {
            *v = Visibility::Hidden;
        }
    }
    if !ligada {
        for (_, _, mut v) in &mut predios {
            if *v != Visibility::Hidden {
                *v = Visibility::Hidden;
            }
        }
        return;
    }
    let n = predios.iter().len();
    if !novos.is_empty() || n != *total {
        *total = n;
        let mut por_estado: HashMap<&str, Vec<(u32, Entity)>> = HashMap::new();
        for (e, p, _) in &predios {
            por_estado.entry(p.estado_id.as_str()).or_default().push((p.nivel, e));
        }
        permitidos.clear();
        for lista in por_estado.values_mut() {
            lista.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            permitidos.extend(lista.iter().take(estilo.max_icones_por_estado).map(|x| x.1));
        }
    }
    for (e, _, mut v) in &mut predios {
        let alvo = if permitidos.contains(&e) { Visibility::Inherited } else { Visibility::Hidden };
        if *v != alvo {
            *v = alvo;
        }
    }
}
