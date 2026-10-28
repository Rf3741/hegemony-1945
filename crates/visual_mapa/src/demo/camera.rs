//! Câmera 2D com pan, zoom suave, limites geográficos e suporte a teclado e mouse.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

/// Componente que marca a câmera do mapa e seus limites.
#[derive(Component, Debug, Clone)]
pub struct CameraMapa {
    pub velocidade_pan: f32,
    pub velocidade_zoom: f32,
    /// Metade da extensão navegável (unidades de mundo).
    pub limite: Vec2,
}

/// Sistema de controle da câmera por teclado (WASD / setas) e roda do mouse.
pub fn controlar_camera_sistema(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: MessageReader<MouseWheel>,
    mut query_camera: Query<(&mut Transform, &mut Projection, &CameraMapa), With<Camera2d>>,
) {
    let Ok((mut transform, mut proj, cam)) = query_camera.single_mut() else {
        return;
    };

    let dt = time.delta_secs();

    // 1. Zoom por roda do mouse
    let mut zoom_delta = 0.0;
    for ev in mouse_wheel.read() {
        zoom_delta -= ev.y * 0.15;
    }

    // Zoom por teclas (+ / - ou Q / E)
    if keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        zoom_delta -= 1.2 * dt;
    }
    if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        zoom_delta += 1.2 * dt;
    }

    if let Projection::Orthographic(ref mut ortho) = *proj {
        if zoom_delta.abs() > 0.0001 {
            ortho.scale = (ortho.scale * (1.0 + zoom_delta)).clamp(0.25, 7.0);
        }

        // 2. Pan por teclado (velocidade escala com o zoom atual)
        let vel = cam.velocidade_pan * ortho.scale * dt;
        let mut mov = Vec2::ZERO;

        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            mov.x -= vel;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            mov.x += vel;
        }
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            mov.y += vel;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            mov.y -= vel;
        }

        transform.translation.x = (transform.translation.x + mov.x).clamp(-cam.limite.x, cam.limite.x);
        transform.translation.y = (transform.translation.y + mov.y).clamp(-cam.limite.y, cam.limite.y);
    }
}
