use bevy::prelude::*;
use simulacao::tempo::{DataSimulacao, RelogioSimulacao, VelocidadeJogo};

/// Evento disparado quando um dia se passa na simulação.
#[allow(dead_code)]
#[derive(Message, Debug, Clone, Copy)]
pub struct EventoTickDiario {
    pub nova_data: DataSimulacao,
    pub total_ticks: u64,
}

/// Recurso ECS que encapsula o relógio determinístico da simulação.
#[derive(Resource, Debug, Deref, DerefMut)]
pub struct RecursoRelogio(pub RelogioSimulacao);

pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RecursoRelogio(RelogioSimulacao::novo()))
            .add_message::<EventoTickDiario>()
            .add_systems(Update, (controlar_relogio_teclado, atualizar_tempo_sistema));
    }
}

fn controlar_relogio_teclado(
    teclas: Res<ButtonInput<KeyCode>>,
    mut relogio: ResMut<RecursoRelogio>,
) {
    if teclas.just_pressed(KeyCode::Space) {
        relogio.alternar_pausa();
    }
    if teclas.just_pressed(KeyCode::Digit1) || teclas.just_pressed(KeyCode::Numpad1) {
        relogio.definir_velocidade(VelocidadeJogo::V1);
    }
    if teclas.just_pressed(KeyCode::Digit2) || teclas.just_pressed(KeyCode::Numpad2) {
        relogio.definir_velocidade(VelocidadeJogo::V2);
    }
    if teclas.just_pressed(KeyCode::Digit3) || teclas.just_pressed(KeyCode::Numpad3) {
        relogio.definir_velocidade(VelocidadeJogo::V3);
    }
    if teclas.just_pressed(KeyCode::Digit4) || teclas.just_pressed(KeyCode::Numpad4) {
        relogio.definir_velocidade(VelocidadeJogo::V4);
    }
    if teclas.just_pressed(KeyCode::Digit5) || teclas.just_pressed(KeyCode::Numpad5) {
        relogio.definir_velocidade(VelocidadeJogo::V5);
    }
    if teclas.just_pressed(KeyCode::Equal) || teclas.just_pressed(KeyCode::NumpadAdd) {
        let v = relogio.velocidade.aumentar();
        relogio.definir_velocidade(v);
    }
    if teclas.just_pressed(KeyCode::Minus) || teclas.just_pressed(KeyCode::NumpadSubtract) {
        let v = relogio.velocidade.diminuir();
        relogio.definir_velocidade(v);
    }
}

fn atualizar_tempo_sistema(
    time: Res<Time>,
    mut relogio: ResMut<RecursoRelogio>,
    mut escritor_eventos: MessageWriter<EventoTickDiario>,
) {
    let dt = time.delta_secs_f64();
    let disparados = relogio.atualizar(dt);
    for _ in 0..disparados {
        escritor_eventos.write(EventoTickDiario {
            nova_data: relogio.data,
            total_ticks: relogio.total_ticks,
        });
    }
}
