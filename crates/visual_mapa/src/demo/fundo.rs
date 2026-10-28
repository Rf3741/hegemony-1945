//! Fundo simples da demonstração: oceano e graticula (a cada 30 graus) na projeção do jogo.

use crate::projecao::ProjecaoMapa;
use bevy::prelude::*;

pub fn criar_fundo_demo(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let proj = ProjecaoMapa::default();
    let (l, a) = (proj.largura as f32, proj.altura as f32);
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(l, a))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.10, 0.16, 0.20)))),
        Transform::from_xyz(0.0, 0.0, -10.0),
    ));
    let grade = materials.add(ColorMaterial::from_color(Color::srgb(0.16, 0.24, 0.29)));
    for lat in (-60..=60).step_by(30) {
        let y = proj.geo_para_mundo(0.0, lat as f32).y;
        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(l, 1.5))),
            MeshMaterial2d(grade.clone()),
            Transform::from_xyz(0.0, y, -9.0),
        ));
    }
    for lon in (-150..=150).step_by(30) {
        let x = proj.geo_para_mundo(lon as f32, 0.0).x;
        commands.spawn((
            Mesh2d(meshes.add(Rectangle::new(1.5, a))),
            MeshMaterial2d(grade.clone()),
            Transform::from_xyz(x, 0.0, -9.0),
        ));
    }
}
