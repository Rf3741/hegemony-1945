//! Estado global do aplicativo e transições.
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

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
            .add_systems(EguiPrimaryContextPass, tela_pausa.run_if(in_state(AppState::Pausado)));
    }
}

fn tecla_esc(teclas: Res<ButtonInput<KeyCode>>, atual: Res<State<AppState>>, mut prox: ResMut<NextState<AppState>>) {
    if teclas.just_pressed(KeyCode::Escape)
        && let Some(n) = alternar_pausa(*atual.get())
    {
        prox.set(n);
    }
}

fn tela_pausa(
    mut contextos: EguiContexts,
    mut prox: ResMut<NextState<AppState>>,
    slots: Res<crate::plugins::saveload::Slots>,
    mut salvar: MessageWriter<crate::plugins::saveload::PedirSalvar>,
    mut carregar: MessageWriter<crate::plugins::saveload::PedirCarregar>,
    mut menu: ResMut<crate::plugins::menu_ui::TelaMenu>,
) -> Result {
    egui::Window::new("Pausado").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(contextos.ctx_mut()?, |ui| {
        if ui.button("Continuar (Esc)").clicked() {
            prox.set(AppState::Jogando);
        }
        ui.separator();
        ui.strong("Salvar / Carregar");
        let mut s = |slot| {
            salvar.write(crate::plugins::saveload::PedirSalvar(slot));
        };
        crate::plugins::saveload::lista_slots(ui, &slots, |slot| {
            carregar.write(crate::plugins::saveload::PedirCarregar(slot));
        }, Some(&mut s));
        ui.separator();
        if ui.button("Sair para o menu principal").clicked() {
            *menu = crate::plugins::menu_ui::TelaMenu::Titulo;
            prox.set(AppState::Menu);
        }
    });
    Ok(())
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
