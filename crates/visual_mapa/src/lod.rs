//! Sistema de Níveis de Detalhe (LOD) por zoom de câmera.

use bevy::prelude::*;

/// Nível de detalhe atual calculado pelo zoom da câmera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NivelLod {
    /// Zoom muito distante (mundo ou continente): mapa limpo, sem prédios, apenas pontos discretos para cidades grandes.
    #[default]
    Global,
    /// Zoom intermediário (país ou região): cidades como pequenos aglomerados e nomes das principais cidades, sem fábricas individuais.
    Regional,
    /// Zoom próximo (estado / local): cidade detalhada com sprites do atlas e as indústrias agregadas ao redor.
    Local,
}

/// Limites de zoom configuráveis para o sistema de LOD.
#[derive(Resource, Debug, Clone)]
pub struct LimitesZoomVisual {
    /// Escala da câmera ortográfica acima da qual o zoom é Global (Longe). Padrão: 2.0
    pub escala_longe: f32,
    /// Escala da câmera ortográfica abaixo da qual o zoom é Local (Perto). Padrão: 0.85
    pub escala_perto: f32,
    /// Nível mínimo para uma cidade ter marcador discreto em zoom de longe. Padrão: 6
    pub nivel_min_cidade_longe: u8,
    /// Nível mínimo para uma cidade exibir rótulo de texto em zoom médio. Padrão: 5
    pub nivel_min_nome_medio: u8,
    /// Controla se partículas de fumaça estão ativas no zoom de perto. Padrão: true
    pub fumaca_ativa: bool,
}

impl Default for LimitesZoomVisual {
    fn default() -> Self {
        Self {
            escala_longe: 2.0,
            escala_perto: 0.85,
            nivel_min_cidade_longe: 7,
            nivel_min_nome_medio: 5,
            fumaca_ativa: true,
        }
    }
}

/// Recurso global que monitora o estado de LOD e a escala da câmera.
#[derive(Resource, Debug, Clone, Default)]
pub struct EstadoLod {
    pub nivel_atual: NivelLod,
    pub escala_camera: f32,
}

impl EstadoLod {
    /// Determina o nível de detalhe com base na escala de projeção ortográfica e limites configurados.
    pub fn calcular_nivel_com_limites(escala: f32, limites: &LimitesZoomVisual) -> NivelLod {
        if escala >= limites.escala_longe {
            NivelLod::Global
        } else if escala >= limites.escala_perto {
            NivelLod::Regional
        } else {
            NivelLod::Local
        }
    }

    /// Determina o nível de detalhe usando os limites padrão.
    pub fn calcular_nivel(escala: f32) -> NivelLod {
        Self::calcular_nivel_com_limites(escala, &LimitesZoomVisual::default())
    }
}

/// Marca o marcador simples (círculo) de uma cidade, visível em zoom distante e médio.
#[derive(Component, Debug, Clone, Copy)]
pub struct MarcadorLod;

/// Marca o detalhe de uma cidade (sprite do atlas ou prédios procedurais) e indústrias, visível em zoom próximo.
#[derive(Component, Debug, Clone, Copy)]
pub struct DetalheLod;

/// Visibilidade base de marcadores e detalhes para um nível de LOD.
pub fn visibilidades(nivel: NivelLod) -> (Visibility, Visibility) {
    match nivel {
        NivelLod::Global => (Visibility::Inherited, Visibility::Hidden),
        NivelLod::Regional => (Visibility::Inherited, Visibility::Hidden),
        NivelLod::Local => (Visibility::Hidden, Visibility::Inherited),
    }
}

/// Lê a escala da câmera e aplica o LOD configurável a todos os marcadores, detalhes e rótulos.
pub fn atualizar_lod_sistema(
    mut estado_lod: ResMut<EstadoLod>,
    limites: Option<Res<LimitesZoomVisual>>,
    camada_visivel: Option<Res<crate::config::CamadaConstrucoesVisivel>>,
    camera: Query<&Projection, With<Camera2d>>,
    cidades: Query<&crate::cidade::CidadeVisual>,
    mut marcadores: Query<(Entity, &mut Visibility, Option<&ChildOf>), (With<MarcadorLod>, Without<DetalheLod>)>,
    mut detalhes: Query<&mut Visibility, (With<DetalheLod>, Without<MarcadorLod>)>,
    mut rotulos: Query<(&mut Visibility, &ChildOf), (With<crate::cidade::RotuloNomeCidade>, Without<MarcadorLod>, Without<DetalheLod>)>,
    agregados: Query<Entity, With<crate::predio::AgregadoEstadoVisual>>,
    mut commands: Commands,
    particulas_fumaca: Query<Entity, With<crate::industria::ParticulaFumaca>>,
) {
    let visivel = camada_visivel.map_or(true, |c| c.0);
    let lim_padrao = LimitesZoomVisual::default();
    let lim = limites.as_deref().unwrap_or(&lim_padrao);

    let escala = camera.single().ok().map(|proj| match proj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    }).unwrap_or(1.0);

    if (estado_lod.escala_camera - escala).abs() > 1e-4 {
        estado_lod.escala_camera = escala;
    }
    let novo = EstadoLod::calcular_nivel_com_limites(escala, lim);
    estado_lod.nivel_atual = novo;

    // Se a camada estiver desligada (tecla V): oculta absolutamente tudo
    if !visivel {
        for (_, mut v, _) in &mut marcadores {
            *v = Visibility::Hidden;
        }
        for mut v in &mut detalhes {
            *v = Visibility::Hidden;
        }
        for (mut v, _) in &mut rotulos {
            *v = Visibility::Hidden;
        }
        for e in &particulas_fumaca {
            commands.entity(e).despawn();
        }
        return;
    }

    // Se estiver fora do zoom de perto (Local), remove qualquer fumaça existente
    if novo != NivelLod::Local {
        for e in &particulas_fumaca {
            commands.entity(e).despawn();
        }
    }

    // 1. Atualizar Detalhes (sprites de atlas, prédios agregados por estado, ferrovias):
    // Visíveis SOMENTE em zoom de perto (Local)
    let vis_detalhes = if novo == NivelLod::Local {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut detalhes {
        *v = vis_detalhes;
    }

    // 2. Atualizar Marcadores Simples de Cidades:
    // - Global (Longe): apenas cidades grandes (nível >= 6 ou capitais)
    // - Regional (Médio): todas as cidades (pequenos aglomerados)
    // - Local (Perto): ocultos (substituídos pelos sprites detalhados)
    for (ent, mut v, child_of) in &mut marcadores {
        // Badges legados de estado nunca devem poluir o mapa
        if agregados.contains(ent) || child_of.is_some_and(|c| agregados.contains(c.parent())) {
            *v = Visibility::Hidden;
            continue;
        }

        if let Some(child) = child_of {
            if let Ok(cidade) = cidades.get(child.parent()) {
                *v = match novo {
                    NivelLod::Global => {
                        if cidade.nivel_atual_inteiro >= lim.nivel_min_cidade_longe || cidade.eh_capital {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        }
                    }
                    NivelLod::Regional => Visibility::Inherited,
                    NivelLod::Local => Visibility::Hidden,
                };
                continue;
            }
        }
        *v = match novo {
            NivelLod::Global | NivelLod::Regional => Visibility::Inherited,
            NivelLod::Local => Visibility::Hidden,
        };
    }

    // 3. Rótulos: posicionamento, tamanho e colisão ficam em `cidade_v3::layout_rotulos_sistema`.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculo_lod() {
        assert_eq!(EstadoLod::calcular_nivel(4.0), NivelLod::Global);
        assert_eq!(EstadoLod::calcular_nivel(1.5), NivelLod::Regional);
        assert_eq!(EstadoLod::calcular_nivel(0.5), NivelLod::Local);
        assert_eq!(EstadoLod::calcular_nivel(2.2), NivelLod::Global);
        assert_eq!(EstadoLod::calcular_nivel(0.85), NivelLod::Regional);
    }

    #[test]
    fn visibilidade_por_lod() {
        assert_eq!(visibilidades(NivelLod::Global), (Visibility::Inherited, Visibility::Hidden));
        assert_eq!(visibilidades(NivelLod::Regional), (Visibility::Inherited, Visibility::Hidden));
        assert_eq!(visibilidades(NivelLod::Local), (Visibility::Hidden, Visibility::Inherited));
    }
}
