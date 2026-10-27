use super::economy::RecursoMundo;
use super::time::EventoTickDiario;
use bevy::prelude::*;

pub struct PoliticsPlugin;

impl Plugin for PoliticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, processar_tick_politica.run_if(in_state(crate::estado::AppState::Jogando)));
    }
}

fn processar_tick_politica(
    mut leitor_eventos: MessageReader<EventoTickDiario>,
    mut recurso_mundo: Option<ResMut<RecursoMundo>>,
) {
    let Some(ref mut mundo) = recurso_mundo else {
        return;
    };
    for _ in leitor_eventos.read() {
        mundo.executar_tick_politica();
    }
}
