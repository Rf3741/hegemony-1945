use super::time::EventoTickDiario;
use bevy::prelude::*;
use simulacao::mundo::MundoJogo;

/// Recurso ECS que armazena a simulação geopolítica e macroeconômica global.
#[derive(Resource, Deref, DerefMut)]
pub struct RecursoMundo(pub MundoJogo);

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, processar_tick_economia);
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
        mundo.executar_tick_economia();
    }
}
