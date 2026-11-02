//! Atmosfera 2.5D do mapa: nuvens, iluminação solar e acabamento cinematográfico.
#![allow(clippy::type_complexity)]

use super::config::{ConfigJogo, QualidadeAtmosfera};
use super::time::RecursoRelogio;
use visual_mapa::config::ConfiguracaoObjetos;
use bevy::{
    camera::Hdr,
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    post_process::bloom::Bloom,
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};
use gf_mapa::tempo::DataSimulacao;

const MAP_WIDTH: f32 = 5632.0;
const MAP_HEIGHT: f32 = 2048.0;

#[derive(Resource, Debug, Clone, Copy)]
pub struct AtmosferaEstado {
    /// Fator de noite no centro do mapa, 0 = dia e 1 = noite.
    pub intensidade_noite: f32,
    /// Longitude aparente do terminador, em radianos.
    pub terminador: f32,
}

impl Default for AtmosferaEstado {
    fn default() -> Self {
        Self {
            intensidade_noite: 0.0,
            terminador: 0.0,
        }
    }
}

#[derive(Component)]
struct CamadaAtmosfera;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct AtmosferaMaterial {
    #[uniform(0)]
    dados_a: Vec4,
    #[uniform(1)]
    dados_b: Vec4,
    #[uniform(2)]
    dados_c: Vec4,
}

impl Material2d for AtmosferaMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/atmosfera.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

pub struct AtmosferaPlugin;

impl Plugin for AtmosferaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<AtmosferaMaterial>::default())
            .init_resource::<AtmosferaEstado>()
            .add_systems(Startup, criar_camada)
            .add_systems(
                Update,
                (atualizar_atmosfera, ajustar_camera_atmosfera).chain(),
            );
    }
}

fn criar_camada(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materiais: ResMut<Assets<AtmosferaMaterial>>,
) {
    commands.spawn((
        CamadaAtmosfera,
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(materiais.add(AtmosferaMaterial {
            dados_a: Vec4::ZERO,
            dados_b: Vec4::ZERO,
            dados_c: Vec4::new(0.0, MAP_WIDTH, MAP_HEIGHT, 0.0),
        })),
        Transform::from_xyz(0.0, 0.0, 2.0),
        Visibility::Hidden,
    ));
}

fn atualizar_atmosfera(
    cfg: Res<ConfigJogo>,
    relogio: Option<Res<RecursoRelogio>>,
    tempo: Res<Time>,
    mut estado: ResMut<AtmosferaEstado>,
    mut objetos: Option<ResMut<ConfiguracaoObjetos>>,
    mut materiais: ResMut<Assets<AtmosferaMaterial>>,
    camada: Query<&MeshMaterial2d<AtmosferaMaterial>, With<CamadaAtmosfera>>,
    camera: Query<&Projection, With<Camera2d>>,
) {
    let Some(handle) = camada.iter().next() else {
        return;
    };
    let Some(mut material) = materiais.get_mut(handle) else {
        return;
    };
    let qualidade = cfg.graficos.qualidade;
    let nuvens = matches!(
        qualidade,
        QualidadeAtmosfera::Alto | QualidadeAtmosfera::Ultra
    ) && cfg.graficos.nuvens;
    let noite_ligada = cfg.graficos.dia_noite || cfg.video.ciclo_dia_noite;
    let (terminador, noite_centro) = relogio.as_deref().map_or((0.0, 0.0), |r| {
        terminador_e_noite(r.data, cfg.video.intensidade_noite)
    });
    estado.terminador = terminador;
    estado.intensidade_noite = if noite_ligada { noite_centro } else { 0.0 };
    if let Some(objetos) = objetos.as_deref_mut() {
        objetos.intensidade_noite = estado.intensidade_noite;
    }

    let zoom = camera
        .single()
        .ok()
        .and_then(|p| match p {
            Projection::Orthographic(ortho) => Some(ortho.scale),
            _ => None,
        })
        .unwrap_or(1.0);
    material.dados_a = Vec4::new(
        tempo.elapsed_secs(),
        zoom,
        if nuvens { 1.0 } else { 0.0 },
        estado.intensidade_noite,
    );
    material.dados_b = Vec4::new(
        terminador,
        if cfg.video.efeito_papel { 1.0 } else { 0.0 },
        if qualidade == QualidadeAtmosfera::Medio {
            0.18
        } else {
            0.10
        },
        if qualidade != QualidadeAtmosfera::Baixo {
            0.22
        } else {
            0.0
        },
    );
    material.dados_c.x = if noite_ligada { 1.0 } else { 0.0 };
    material.dados_c.z = if qualidade == QualidadeAtmosfera::Ultra {
        1.0
    } else {
        0.0
    };
}

fn ajustar_camera_atmosfera(
    cfg: Res<ConfigJogo>,
    janela: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: Query<
        (
            Entity,
            &mut Transform,
            &mut Projection,
            Option<&mut Tonemapping>,
            Option<&Bloom>,
            Option<&Hdr>,
        ),
        With<Camera2d>,
    >,
    mut camada: Query<(&mut Transform, &mut Visibility), With<CamadaAtmosfera>>,
    mut commands: Commands,
) {
    let Ok((camera_entity, camera_tf, mut proj, tonemapping, bloom, hdr)) = cameras.single_mut()
    else {
        return;
    };
    let Ok(window) = janela.single() else { return };
    let qualidade = cfg.video.qualidade_atmosfera;
    let ativo = qualidade != QualidadeAtmosfera::Baixo;

    if ativo && hdr.is_none() {
        commands
            .entity(camera_entity)
            .insert((Hdr, DebandDither::Enabled));
    } else if !ativo && hdr.is_some() {
        commands
            .entity(camera_entity)
            .remove::<(Hdr, DebandDither, Tonemapping, Bloom)>();
    }
    if ativo && tonemapping.is_none() {
        commands.entity(camera_entity).insert(Tonemapping::AgX);
    }
    let quer_bloom = matches!(
        qualidade,
        QualidadeAtmosfera::Alto | QualidadeAtmosfera::Ultra
    );
    if quer_bloom && bloom.is_none() {
        commands.entity(camera_entity).insert(Bloom {
            intensity: 0.045,
            ..default()
        });
    } else if !quer_bloom && bloom.is_some() {
        commands.entity(camera_entity).remove::<Bloom>();
    }

    let escala = match &mut *proj {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => return,
    };
    let altura = window.height() * escala + 24.0;
    let largura = window.width() * escala + 24.0;
    if let Ok((mut tf, mut vis)) = camada.single_mut() {
        tf.translation.x = camera_tf.translation.x;
        tf.translation.y = camera_tf.translation.y;
        tf.scale = Vec3::new(largura, altura, 1.0);
        *vis = if ativo {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn terminador_e_noite(data: DataSimulacao, intensidade: f32) -> (f32, f32) {
    let dias_mes = [31_u32, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let dia_ano = dias_mes[..data.mes.saturating_sub(1) as usize]
        .iter()
        .copied()
        .sum::<u32>()
        + data.dia.saturating_sub(1) as u32;
    let fase = (dia_ano as f32 / 365.0) * std::f32::consts::TAU;
    let noite = ((-fase.cos() + 1.0) * 0.5 * intensidade).clamp(0.0, 1.0);
    (fase, noite)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn intensidade_da_noite_fica_no_intervalo() {
        let (_, noite) = terminador_e_noite(
            DataSimulacao {
                ano: 1945,
                mes: 6,
                dia: 21,
            },
            1.0,
        );
        assert!((0.0..=1.0).contains(&noite));
    }
}
