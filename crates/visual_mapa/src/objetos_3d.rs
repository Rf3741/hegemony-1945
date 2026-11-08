//! Acabamento 2.5D barato: faces oblíquas sombreadas, topo em profundidade e sombra projetada no chão.

use crate::cidade_v3::PredioUrbano;
use crate::config::ConfiguracaoObjetos;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d};

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(crate) struct VolumeMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material2d for VolumeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/objetos_2d.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Component)]
pub(crate) struct VolumeAplicado;

#[derive(Component)]
pub(crate) struct SombraObjeto;

#[derive(Resource)]
pub(crate) struct RecursosObjetos3d {
    quad: Handle<Mesh>,
}

/// Transformações locais (relativas a um `PredioUrbano` com `Anchor::BOTTOM_CENTER`)
/// da parede lateral direita, da laje superior oblíqua e da sombra projetada a sudeste.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GeometriaVolume {
    pub pos_face: Vec3,
    pub esc_face: Vec3,
    pub pos_topo: Vec3,
    pub esc_topo: Vec3,
    pub pos_sombra: Vec3,
    pub esc_sombra: Vec3,
}

pub(crate) fn geometria_volume_predio(tamanho: Vec2, extrusao: f32) -> GeometriaVolume {
    let w = tamanho.x.max(0.4);
    let h = tamanho.y.max(0.4);
    let fator_ext = (extrusao / 0.22).clamp(0.4, 2.0);
    let h_telhado = (h * 0.08).clamp(0.15, 0.5);

    let larg_face = (w * 0.26 * fator_ext).clamp(0.14, 0.80);
    let dy = (larg_face * 0.36).clamp(0.06, 0.30);
    let pos_face = Vec3::new(w * 0.5 + larg_face * 0.5, (h + dy) * 0.5, -0.015);
    let esc_face = Vec3::new(larg_face, h + dy, 1.0);

    let larg_topo = w * 1.02 + larg_face * 0.85;
    let alt_topo = (dy + h_telhado * 0.65).clamp(0.12, 0.52);
    let pos_topo = Vec3::new(
        larg_face * 0.42,
        h + h_telhado * 0.55 + alt_topo * 0.35,
        -0.008,
    );
    let esc_topo = Vec3::new(larg_topo, alt_topo, 1.0);

    let larg_sombra = w * 1.08 + larg_face * 1.25;
    let alt_sombra = (h * 0.14 * fator_ext).clamp(0.20, 0.95);
    let pos_sombra = Vec3::new(w * 0.20 + larg_face * 0.60, -alt_sombra * 0.32, -0.08);
    let esc_sombra = Vec3::new(larg_sombra, alt_sombra, 1.0);

    GeometriaVolume {
        pos_face,
        esc_face,
        pos_topo,
        esc_topo,
        pos_sombra,
        esc_sombra,
    }
}

pub(crate) fn cores_volume(intensidade_sombra: f32) -> [LinearRgba; 3] {
    // Cores convertidas de sRGB arquitetônico para espaço linear (nunca claras a ponto de gerar manchas brancas).
    let face = Color::srgba(0.13, 0.15, 0.20, 0.78).to_linear();
    let topo = Color::srgba(0.34, 0.33, 0.31, 0.74).to_linear();
    let alfa_sombra = (intensidade_sombra.clamp(0.0, 1.0) * 1.35).clamp(0.12, 0.52);
    let sombra = Color::srgba(0.02, 0.03, 0.06, alfa_sombra).to_linear();
    [face, topo, sombra]
}

pub(crate) fn criar_recursos_objetos_3d(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(RecursosObjetos3d {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
    });
}

/// Adiciona faces oblíquas 2.5D e sombra projetada no chão aos prédios urbanos (`PredioUrbano`)
/// usando `Sprite::custom_size` (espaço local `Anchor::BOTTOM_CENTER`), mantendo luz coerente de noroeste.
/// Instalações industriais usam sprites isométricos próprios do atlas e ficam de fora.
pub(crate) fn materializar_volumes_sistema(
    mut commands: Commands,
    cfg: Res<ConfiguracaoObjetos>,
    rec: Option<Res<RecursosObjetos3d>>,
    mut materiais: ResMut<Assets<VolumeMaterial>>,
    predios: Query<(Entity, &Sprite), (With<PredioUrbano>, Without<VolumeAplicado>)>,
    mut cache: Local<Option<(u32, [Handle<VolumeMaterial>; 3])>>,
) {
    let Some(rec) = rec else { return };
    if !cfg.qualidade.tem_faces() {
        return;
    }
    let mut novos = predios.iter().peekable();
    if novos.peek().is_none() {
        return;
    }

    let chave_sombra = cfg.intensidade_sombra.to_bits();
    let [mat_face, mat_topo, mat_sombra] = match cache.as_ref() {
        Some((k, mats)) if *k == chave_sombra => mats.clone(),
        _ => {
            let [c_face, c_topo, c_sombra] = cores_volume(cfg.intensidade_sombra);
            let mats = [
                materiais.add(VolumeMaterial { color: c_face }),
                materiais.add(VolumeMaterial { color: c_topo }),
                materiais.add(VolumeMaterial { color: c_sombra }),
            ];
            *cache = Some((chave_sombra, mats.clone()));
            mats
        }
    };

    let vis_sombra = if cfg.qualidade.tem_sombras() && cfg.sombras_ativas {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for (entity, sprite) in novos {
        let tamanho = sprite.custom_size.unwrap_or(Vec2::new(1.5, 2.2));
        let g = geometria_volume_predio(tamanho, cfg.extrusao);
        commands
            .entity(entity)
            .insert(VolumeAplicado)
            .with_children(|c| {
                c.spawn((
                    Mesh2d(rec.quad.clone()),
                    MeshMaterial2d(mat_face.clone()),
                    Transform::from_translation(g.pos_face).with_scale(g.esc_face),
                ));
                c.spawn((
                    Mesh2d(rec.quad.clone()),
                    MeshMaterial2d(mat_topo.clone()),
                    Transform::from_translation(g.pos_topo).with_scale(g.esc_topo),
                ));
                c.spawn((
                    Mesh2d(rec.quad.clone()),
                    MeshMaterial2d(mat_sombra.clone()),
                    Transform::from_translation(g.pos_sombra).with_scale(g.esc_sombra),
                    vis_sombra,
                    SombraObjeto,
                ));
            });
    }
}

pub(crate) fn atualizar_intensidade_noite_sistema(
    cfg: Res<ConfiguracaoObjetos>,
    mut sombras: Query<&mut Visibility, With<SombraObjeto>>,
    mut ultimo: Local<Option<bool>>,
) {
    let mostrar = cfg.qualidade.tem_sombras() && cfg.sombras_ativas;
    if *ultimo == Some(mostrar) {
        return;
    }
    *ultimo = Some(mostrar);
    let alvo = if mostrar {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut sombras {
        if *v != alvo {
            *v = alvo;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_acompanha_altura_real_do_predio_e_ancora_base() {
        let g_casa = geometria_volume_predio(Vec2::new(1.4, 1.2), 0.22);
        let g_torre = geometria_volume_predio(Vec2::new(1.8, 9.0), 0.22);

        // Parede lateral começa na base y = 0 e acompanha toda a altura do prédio.
        let base_torre = g_torre.pos_face.y - g_torre.esc_face.y * 0.5;
        assert!(base_torre.abs() < 1e-4, "base lateral fora do chão: {base_torre}");
        assert!(g_torre.esc_face.y > 9.0);
        assert!(g_torre.pos_face.x > 0.9, "parede lateral deve ficar à direita da fachada");

        // Topo oblíquo fica no alto do edifício (acima de h), nunca no pé.
        assert!(g_torre.pos_topo.y > 9.0);
        assert!(g_casa.pos_topo.y > 1.2);

        // Sombra fica projetada ao sul/sudeste da base (y < 0, x > 0) e atrás da fachada (z < 0).
        assert!(g_torre.pos_sombra.y < 0.0);
        assert!(g_torre.pos_sombra.x > 0.0);
        assert!(g_torre.pos_sombra.z < g_torre.pos_face.z);
    }

    #[test]
    fn cores_lineares_dos_volumes_nunca_estouram_para_branco() {
        let [face, topo, sombra] = cores_volume(0.28);
        for c in [face, topo, sombra] {
            assert!(c.red <= 0.15 && c.green <= 0.15 && c.blue <= 0.15, "cor linear clara demais: {c:?}");
            assert!(c.alpha > 0.0 && c.alpha <= 1.0);
        }
    }
}
