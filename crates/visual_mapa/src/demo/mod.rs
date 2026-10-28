//! Demonstração (feature `demo`): dados falsos, tempo acelerado 1945-1991, câmera, fundo e HUD.
//!
//! `DemoPlugin` NÃO adiciona o `VisualMapaPlugin` (adicione-o à parte) nem `DefaultPlugins`.
//! Opções: [`DemoPlugin::com_fundo`] (graticula de teste; desligue para pôr sobre o mapa real)
//! e [`DemoPlugin::com_camera`].

pub mod camera;
pub mod dados_demo;
pub mod fundo;
pub mod tempo;
pub mod ui;

use crate::lod::EstadoLod;
use crate::mapa::EstadoModoMapa;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

pub struct DemoPlugin {
    pub com_fundo: bool,
    pub com_camera: bool,
}

impl Default for DemoPlugin {
    fn default() -> Self {
        Self { com_fundo: true, com_camera: true }
    }
}

impl DemoPlugin {
    pub fn com_fundo(mut self, v: bool) -> Self {
        self.com_fundo = v;
        self
    }
    pub fn com_camera(mut self, v: bool) -> Self {
        self.com_camera = v;
        self
    }
}

impl Plugin for DemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_plugins(EguiPlugin::default())
            .init_resource::<tempo::TempoSimulacao>()
            .init_resource::<ui::EstadoUi>()
            .init_resource::<EstadoModoMapa>()
            .init_resource::<EstadoLod>()
            .add_systems(Startup, dados_demo::iniciar_carga_sistema)
            .add_systems(
                Update,
                (
                    tempo::atualizar_tempo_sistema,
                    tempo::publicar_ano_sistema,
                    dados_demo::aguardar_carga_sistema,
                    dados_demo::alimentar_entrada_sistema,
                    ui::atalhos_teclado_ui_sistema,
                )
                    .chain()
                    .before(crate::VisualMapaSet::Sincronizar),
            )
            .add_systems(EguiPrimaryContextPass, ui::renderizar_ui_sistema);
        if self.com_fundo {
            app.add_systems(Startup, fundo::criar_fundo_demo);
        }
        if self.com_camera {
            app.add_systems(Startup, criar_camera).add_systems(Update, camera::controlar_camera_sistema);
        }
    }
}

fn criar_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection { scale: 3.0, ..OrthographicProjection::default_2d() }),
        Transform::from_xyz(0.0, 150.0, 1000.0),
        camera::CameraMapa { velocidade_pan: 450.0, velocidade_zoom: 1.5, limite: Vec2::new(3200.0, 1200.0) },
    ));
}
