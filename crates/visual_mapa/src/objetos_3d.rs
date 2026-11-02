//! Acabamento 2.5D barato: faces iluminadas e sombras no chão.

use crate::cidade_v3::PredioUrbano;
use crate::config::{ConfiguracaoObjetos, QualidadeObjetos};
use crate::industria::InstalacaoIndustrial;
use crate::trafego::NavioVisual;
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
        "shaders/objetos_2d.wgsl".into()
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

pub(crate) fn criar_recursos_objetos_3d(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(RecursosObjetos3d {
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
    });
}

/// Adiciona faces oblíquas e uma sombra suave sem modificar o transform do objeto original.
/// A direção (-noroeste) é fixa para manter a luz coerente com o relevo do mapa.
pub(crate) fn materializar_volumes_sistema(
    mut commands: Commands,
    cfg: Res<ConfiguracaoObjetos>,
    rec: Option<Res<RecursosObjetos3d>>,
    mut materiais: ResMut<Assets<VolumeMaterial>>,
    predios: Query<(Entity, &Transform, Option<&VolumeAplicado>), With<PredioUrbano>>,
    industrias: Query<(Entity, &Transform, Option<&VolumeAplicado>), With<InstalacaoIndustrial>>,
    navios: Query<(Entity, &Transform, Option<&VolumeAplicado>), With<NavioVisual>>,
) {
    let Some(rec) = rec else { return };
    if cfg.qualidade == QualidadeObjetos::Baixo {
        return;
    }
    let mat_face = materiais.add(VolumeMaterial {
        color: LinearRgba::new(0.20, 0.24, 0.30, 0.72),
    });
    let mat_topo = materiais.add(VolumeMaterial {
        color: LinearRgba::new(0.92, 0.88, 0.70, 0.46),
    });
    let mat_sombra = materiais.add(VolumeMaterial {
        color: LinearRgba::new(
            0.02,
            0.03,
            0.05,
            cfg.intensidade_sombra.clamp(0.0, 1.0) * 0.65,
        ),
    });

    for (entity, tr, aplicado) in predios.iter().chain(industrias.iter()).chain(navios.iter()) {
        if aplicado.is_some() {
            continue;
        }
        commands.entity(entity).insert(VolumeAplicado);
        let escala = tr.scale;
        commands.entity(entity).with_children(|c| {
            c.spawn((
                Mesh2d(rec.quad.clone()),
                MeshMaterial2d(mat_face.clone()),
                Transform::from_xyz(
                    escala.x * 0.48 * cfg.extrusao,
                    -escala.y * 0.10 * cfg.extrusao,
                    -0.02,
                )
                .with_scale(Vec3::new(escala.x * 0.12, escala.y * 0.92, 1.0)),
            ));
            c.spawn((
                Mesh2d(rec.quad.clone()),
                MeshMaterial2d(mat_topo.clone()),
                Transform::from_xyz(-escala.x * 0.05, escala.y * 0.48 * cfg.extrusao, 0.02)
                    .with_scale(Vec3::new(
                        escala.x * 0.92,
                        escala.y * 0.10 * cfg.extrusao.max(0.5),
                        1.0,
                    )),
            ));
            if cfg.qualidade.tem_sombras() {
                c.spawn((
                    Mesh2d(rec.quad.clone()),
                    MeshMaterial2d(mat_sombra.clone()),
                    Transform::from_xyz(escala.x * 0.24, -escala.y * 0.24, -0.08)
                        .with_scale(Vec3::new(escala.x * 1.25, escala.y * 0.30, 1.0)),
                    SombraObjeto,
                ));
            }
        });
    }
}

pub(crate) fn atualizar_intensidade_noite_sistema(
    cfg: Res<ConfiguracaoObjetos>,
    mut sombras: Query<&mut Visibility, With<SombraObjeto>>,
) {
    if !cfg.is_changed() {
        return;
    }
    for mut v in &mut sombras {
        *v = if cfg.qualidade.tem_sombras() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
