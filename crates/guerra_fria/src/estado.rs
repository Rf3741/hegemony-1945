//! Estado global do aplicativo e transições.
use bevy::prelude::*;
use crate::plugins::economy::RecursoMundo;
use crate::plugins::jogador::PaisJogador;
use crate::plugins::map::ProvinciaSelecionada;
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
            .add_systems(EguiPrimaryContextPass, (tela_menu.run_if(in_state(AppState::Menu)), tela_pausa.run_if(in_state(AppState::Pausado))));
    }
}

fn tecla_esc(teclas: Res<ButtonInput<KeyCode>>, atual: Res<State<AppState>>, mut prox: ResMut<NextState<AppState>>) {
    if teclas.just_pressed(KeyCode::Escape)
        && let Some(n) = alternar_pausa(*atual.get())
    {
        prox.set(n);
    }
}

fn tela_menu(
    mut contextos: EguiContexts,
    mut prox: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mundo: Res<RecursoMundo>,
    selecionada: Res<ProvinciaSelecionada>,
    mut busca: Local<String>,
) -> Result {
    let mut escolhido: Option<String> = None;
    egui::Window::new("Guerra Fria - 1945: escolha seu país")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
        .show(contextos.ctx_mut()?, |ui| {
            ui.label("Clique numa província do mapa ou escolha abaixo.");
            if selecionada.0 != 0
                && let Some(tag) = mundo.dono_por_provincia.get(&selecionada.0)
                && let Some(p) = mundo.paises.get(tag)
                && ui.button(format!("Jogar com: {} (clicado no mapa)", p.nome)).clicked()
            {
                escolhido = Some(tag.clone());
            }
            ui.separator();
            ui.strong("Potências");
            ui.horizontal_wrapped(|ui| {
                for tag in ["USA", "SOV", "GBR", "FRA", "CHI", "BRA", "GER", "JAP", "ITA", "IND", "CAN", "ARG", "MEX", "TUR"] {
                    if let Some(p) = mundo.paises.get(tag)
                        && ui.button(&p.nome).clicked()
                    {
                        escolhido = Some(tag.to_string());
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Buscar:");
                ui.text_edit_singleline(&mut *busca);
            });
            let b = busca.to_lowercase();
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                for p in mundo.paises.values() {
                    if p.populacao_total() == 0 || (!b.is_empty() && !p.nome.to_lowercase().contains(&b) && !p.tag.to_lowercase().contains(&b)) {
                        continue;
                    }
                    if ui.button(format!("{} ({}) - pop. {}", p.nome, p.tag, crate::plugins::jogador::fmt_pop(p.populacao_total()))).clicked() {
                        escolhido = Some(p.tag.clone());
                    }
                }
            });
        });
    if let Some(tag) = escolhido {
        commands.insert_resource(PaisJogador(tag));
        prox.set(AppState::Jogando);
    }
    Ok(())
}

fn tela_pausa(mut contextos: EguiContexts, mut prox: ResMut<NextState<AppState>>) -> Result {
    egui::Window::new("Pausado").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(contextos.ctx_mut()?, |ui| {
        if ui.button("Continuar (Esc)").clicked() {
            prox.set(AppState::Jogando);
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
