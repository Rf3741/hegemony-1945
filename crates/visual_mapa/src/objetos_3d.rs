//! Acabamento 2.5D barato: faces iluminadas e sombras no chão.

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
#[allow(dead_code)]
pub(crate) struct VolumeAplicado;

#[derive(Component)]
pub(crate) struct SombraObjeto;

#[derive(Resource)]
#[allow(dead_code)]
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
/// Prédios urbanos (cidade_v3) já possuem acabamento 2.5D próprio com telhados,
/// sombras laterais e janelas desenhados nos próprios sprites. Adicionar quads de volume
/// (`VolumeMaterial` com `mat_topo` claro) criava manchas brancas no mapa ao dar zoom.
/// Indústrias utilizam sprites isométricos de atlas e navios já haviam sido removidos.
pub(crate) fn materializar_volumes_sistema(
    _cfg: Res<ConfiguracaoObjetos>,
    _rec: Option<Res<RecursosObjetos3d>>,
) {
}

pub(crate) fn atualizar_intensidade_noite_sistema(
    cfg: Res<ConfiguracaoObjetos>,
    mut sombras: Query<&mut Visibility, With<SombraObjeto>>,
) {
    if !cfg.is_changed() {
        return;
    }
    for mut v in &mut sombras {
        *v = if cfg.qualidade.tem_sombras() && cfg.sombras_ativas {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
