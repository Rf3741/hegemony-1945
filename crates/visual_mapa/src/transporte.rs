//! Malha de transporte (ferrovias e rodovias) com linhas conectando cidades que engrossam com o nível.

use crate::cidade::{CidadeVisual, IndiceEntidades};
use crate::entrada::EntradaVisual;
use crate::lod::DetalheLod;
use bevy::prelude::*;

/// Tipo de infraestrutura viária ou ferroviária.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoTransporte {
    EstradaTerra,
    Rodovia,
    Ferrovia,
    FerroviaPesada,
}

impl TipoTransporte {
    pub fn de_str(s: &str) -> Self {
        match s {
            "rodovia" => Self::Rodovia,
            "ferrovia" => Self::Ferrovia,
            "ferrovia_pesada" => Self::FerroviaPesada,
            _ => Self::EstradaTerra,
        }
    }

    pub fn cor_base(&self) -> Color {
        match self {
            Self::EstradaTerra => Color::srgb(0.55, 0.47, 0.33),   // Marrom terra batida
            Self::Rodovia => Color::srgb(0.38, 0.42, 0.46),        // Asfalto cinza
            Self::Ferrovia => Color::srgb(0.25, 0.28, 0.32),       // Aço de trilhos
            Self::FerroviaPesada => Color::srgb(0.18, 0.20, 0.24), // Aço escuro reforçado
        }
    }
}

/// Segmento de linha viária/ferroviária entre duas cidades.
#[derive(Component, Debug, Clone)]
pub struct LinhaTransporte {
    pub id: String,
    pub origem_id: String,
    pub destino_id: String,
    pub tipo: TipoTransporte,
    pub nivel: u8,
    pub ativa: bool,
}

/// Espessura (unidades de mundo) por nível da via.
pub fn espessura_por_nivel(nivel: u8) -> f32 {
    match nivel {
        0 | 1 => 1.0,
        2 => 1.8,
        3 => 2.8,
        4 => 4.0,
        _ => 5.5,
    }
}

/// Sincroniza as conexões de [`EntradaVisual`]: cria as novas (quando as duas cidades já
/// existem) e atualiza espessura/visibilidade das existentes.
pub fn sincronizar_transporte_sistema(
    mut commands: Commands,
    entrada: Res<EntradaVisual>,
    mut indice: ResMut<IndiceEntidades>,
    cidades: Query<&CidadeVisual>,
    mut linhas: Query<(&mut LinhaTransporte, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut recursos: Local<Option<[Handle<ColorMaterial>; 4]>>,
) {
    let mats = recursos.get_or_insert_with(|| {
        [TipoTransporte::EstradaTerra, TipoTransporte::Rodovia, TipoTransporte::Ferrovia, TipoTransporte::FerroviaPesada]
            .map(|t| materials.add(ColorMaterial::from_color(t.cor_base())))
    });
    let mut novas = 0;
    for c in &entrada.conexoes {
        if let Some(&e) = indice.conexoes.get(&c.id) {
            if let Ok((mut l, mut tr)) = linhas.get_mut(e) {
                l.nivel = c.nivel;
                l.ativa = c.ativa;
                tr.scale.y = if c.ativa { espessura_por_nivel(c.nivel) } else { 0.0 };
            }
            continue;
        }
        let (Some(&ea), Some(&eb)) = (indice.cidades.get(&c.origem_id), indice.cidades.get(&c.destino_id)) else {
            continue; // as cidades podem chegar depois; tenta de novo na próxima sincronia
        };
        let (Ok(a), Ok(b)) = (cidades.get(ea), cidades.get(eb)) else { continue };
        let (pa, pb) = (a.posicao_mundo, b.posicao_mundo);
        let delta = pb - pa;
        let comprimento = delta.length();
        if comprimento < 1.0 {
            continue;
        }
        let tipo = TipoTransporte::de_str(&c.tipo);
        let m = match tipo {
            TipoTransporte::EstradaTerra => 0,
            TipoTransporte::Rodovia => 1,
            TipoTransporte::Ferrovia => 2,
            TipoTransporte::FerroviaPesada => 3,
        };
        let esp = if c.ativa { espessura_por_nivel(c.nivel) } else { 0.0 };
        let ent = commands
            .spawn((
                Mesh2d(meshes.add(Rectangle::new(comprimento, 1.0))),
                MeshMaterial2d(mats[m].clone()),
                Transform::from_translation(((pa + pb) * 0.5).extend(8.0))
                    .with_rotation(Quat::from_rotation_z(delta.y.atan2(delta.x)))
                    .with_scale(Vec3::new(1.0, esp, 1.0)),
                LinhaTransporte {
                    id: c.id.clone(),
                    origem_id: c.origem_id.clone(),
                    destino_id: c.destino_id.clone(),
                    tipo,
                    nivel: c.nivel,
                    ativa: c.ativa,
                },
                DetalheLod,
                Visibility::Hidden,
            ))
            .id();
        indice.conexoes.insert(c.id.clone(), ent);
        novas += 1;
    }
    if novas > 0 {
        info!("visual_mapa: {novas} conexões criadas.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipo_transporte() {
        assert_eq!(TipoTransporte::de_str("ferrovia"), TipoTransporte::Ferrovia);
        assert_eq!(TipoTransporte::de_str("rodovia"), TipoTransporte::Rodovia);
        assert_eq!(TipoTransporte::de_str("x"), TipoTransporte::EstradaTerra);
    }

    #[test]
    fn espessura_monotonica() {
        assert!(espessura_por_nivel(1) < espessura_por_nivel(3));
        assert!(espessura_por_nivel(3) < espessura_por_nivel(5));
    }
}
