//! Estado global do aplicativo e transições.
use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    /// Gerando a textura do mapa em segundo plano.
    #[default]
    Carregando,
    Menu,
    Jogando,
    Pausado,
}

/// Próximo estado ao apertar Esc; `None` se Esc não faz nada.
pub fn alternar_pausa(atual: AppState) -> Option<AppState> {
    match atual {
        AppState::Jogando => Some(AppState::Pausado),
        AppState::Pausado => Some(AppState::Jogando),
        _ => None,
    }
}

/// Condição: partida em andamento (Jogando ou Pausado).
pub fn em_partida(estado: Option<Res<State<AppState>>>) -> bool {
    matches!(estado.as_deref().map(|s| *s.get()), Some(AppState::Jogando | AppState::Pausado))
}

/// Condição: partida em andamento ou menu (o mapa fica interativo para escolher o país).
pub fn em_partida_ou_menu(estado: Option<Res<State<AppState>>>) -> bool {
    matches!(estado.as_deref().map(|s| *s.get()), Some(AppState::Jogando | AppState::Pausado | AppState::Menu))
}

pub struct EstadoPlugin;

impl Plugin for EstadoPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(Update, tecla_esc)
            ;
    }
}

fn tecla_esc(
    teclas: Res<ButtonInput<KeyCode>>,
    atual: Res<State<AppState>>,
    mut prox: ResMut<NextState<AppState>>,
    mut config: ResMut<crate::plugins::menu_pausa::JanelaConfig>,
    mut saves: ResMut<crate::plugins::menu_saves::JanelaSaves>,
    mut ledger: ResMut<crate::plugins::estatisticas::RecursoLedger>,
) {
    if !teclas.just_pressed(KeyCode::Escape) {
        return;
    }
    // Esc fecha primeiro a janela que estiver aberta por cima.
    if config.aberta {
        config.aberta = false;
    } else if saves.aberta {
        saves.aberta = false;
    } else if ledger.aberto {
        ledger.aberto = false;
    } else if let Some(n) = alternar_pausa(*atual.get()) {
        prox.set(n);
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn esc_alterna_somente_em_jogo() {
        assert_eq!(alternar_pausa(AppState::Jogando), Some(AppState::Pausado));
        assert_eq!(alternar_pausa(AppState::Pausado), Some(AppState::Jogando));
        assert_eq!(alternar_pausa(AppState::Menu), None);
        assert_eq!(alternar_pausa(AppState::Carregando), None);
    }
}
