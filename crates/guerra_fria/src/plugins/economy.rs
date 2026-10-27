use super::time::EventoTickDiario;
use bevy::prelude::*;
use simulacao::mundo::MundoJogo;

/// Recurso ECS que armazena a simulação geopolítica e macroeconômica global.
#[derive(Resource, Deref, DerefMut)]
pub struct RecursoMundo(pub MundoJogo);

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, processar_tick_economia.run_if(in_state(crate::estado::AppState::Jogando)));
    }
}

fn processar_tick_economia(
    mut leitor_eventos: MessageReader<EventoTickDiario>,
    mut recurso_mundo: Option<ResMut<RecursoMundo>>,
) {
    let Some(ref mut mundo) = recurso_mundo else {
        return;
    };
    for _ in leitor_eventos.read() {
        // O motor não avança o calendário sozinho nesta chamada: sem isso o crescimento mensal rodaria todo dia.
        mundo.data.avancar_dia();
        mundo.ticks_executados += 1;
        mundo.executar_tick_economia();
    }
}
