//! UiPlugin: Plugin reutilizável para Bevy Engine.
//! Integra toda a interface de Guerra Fria 1945 ao loop ECS do Bevy.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::mock_data::CenariosUi1945;
use crate::screens::*;
use crate::theme::estilo::{cold_war_style, cold_war_visuals, moldura_painel, setup_cold_war_fonts};
use crate::view_model::*;

/// Recurso de controle local da interface (qual aba do estado está aberta).
#[derive(Resource, Debug, Default)]
pub struct ControleAbasUi {
    pub aba_estado: AbaEstado,
    pub fontes_inicializadas: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for AbaEstado {
    fn default() -> Self {
        AbaEstado::DemografiaPops
    }
}

/// Plugin principal de interface gráfica reutilizável.
#[derive(Default)]
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        // Inicializa cenários e recursos padrão de 1945
        let cenarios = CenariosUi1945::eua_1945();

        app.insert_resource(cenarios.top_bar)
            .insert_resource(cenarios.pais)
            .insert_resource(cenarios.estado)
            .insert_resource(cenarios.mercado)
            .insert_resource(cenarios.construcao)
            .insert_resource(cenarios.orcamento)
            .insert_resource(cenarios.politica)
            .insert_resource(cenarios.diplomacia)
            .insert_resource(cenarios.ranking)
            .insert_resource(cenarios.notificacoes)
            .insert_resource(cenarios.evento)
            .insert_resource(EstadoUiGlobal::default())
            .insert_resource(ControleAbasUi::default())
            .add_systems(Update, atalhos_teclado_system)
            .add_systems(EguiPrimaryContextPass, render_interface_system);
    }
}

/// Sistema de controle de atalhos globais de teclado (F1-F8, ESC, ESPAÇO, 1-5).
pub fn atalhos_teclado_system(
    teclado: Res<ButtonInput<KeyCode>>,
    mut estado_global: ResMut<EstadoUiGlobal>,
    mut top_bar: ResMut<TopBarViewModel>,
    mut evento_modal: ResMut<EventoModalViewModel>,
) {
    // Tecla ESC: Volta ao Mapa Geral com HUD
    if teclado.just_pressed(KeyCode::Escape) {
        estado_global.tela_ativa = TelaAtiva::MapaGeralHud;
    }

    // Tecla ESPAÇO: Alterna Pausa
    if teclado.just_pressed(KeyCode::Space) {
        top_bar.esta_pausado = !top_bar.esta_pausado;
    }

    // Teclas F1 a F8: Navegação direta entre telas
    if teclado.just_pressed(KeyCode::F1) {
        estado_global.tela_ativa = TelaAtiva::PainelPais;
    }
    if teclado.just_pressed(KeyCode::F2) {
        estado_global.tela_ativa = TelaAtiva::PainelEstado;
    }
    if teclado.just_pressed(KeyCode::F3) {
        estado_global.tela_ativa = TelaAtiva::Mercado;
    }
    if teclado.just_pressed(KeyCode::F4) {
        estado_global.tela_ativa = TelaAtiva::Construcao;
    }
    if teclado.just_pressed(KeyCode::F5) {
        estado_global.tela_ativa = TelaAtiva::Orcamento;
    }
    if teclado.just_pressed(KeyCode::F6) {
        estado_global.tela_ativa = TelaAtiva::Politica;
    }
    if teclado.just_pressed(KeyCode::F7) {
        estado_global.tela_ativa = TelaAtiva::Diplomacia;
    }
    if teclado.just_pressed(KeyCode::F8) {
        estado_global.tela_ativa = TelaAtiva::RankingPotencias;
    }

    // Teclas 1 a 5: Velocidade da simulação
    if teclado.just_pressed(KeyCode::Digit1) {
        top_bar.velocidade = VelocidadeJogo::Velocidade1;
        top_bar.esta_pausado = false;
    }
    if teclado.just_pressed(KeyCode::Digit2) {
        top_bar.velocidade = VelocidadeJogo::Velocidade2;
        top_bar.esta_pausado = false;
    }
    if teclado.just_pressed(KeyCode::Digit3) {
        top_bar.velocidade = VelocidadeJogo::Velocidade3;
        top_bar.esta_pausado = false;
    }
    if teclado.just_pressed(KeyCode::Digit4) {
        top_bar.velocidade = VelocidadeJogo::Velocidade4;
        top_bar.esta_pausado = false;
    }
    if teclado.just_pressed(KeyCode::Digit5) {
        top_bar.velocidade = VelocidadeJogo::Velocidade5;
        top_bar.esta_pausado = false;
    }

    // Tecla E: Dispara Modal de Evento de Demonstração (Yalta)
    if teclado.just_pressed(KeyCode::KeyE) {
        evento_modal.aberto = !evento_modal.aberto;
    }
}

/// Sistema principal de renderização imediata com `bevy_egui`.
#[allow(clippy::too_many_arguments)]
pub fn render_interface_system(
    mut contexts: EguiContexts,
    mut estado_global: ResMut<EstadoUiGlobal>,
    mut controle_abas: ResMut<ControleAbasUi>,
    mut top_bar: ResMut<TopBarViewModel>,
    pais: Res<PaisViewModel>,
    estado: Res<EstadoViewModel>,
    mut mercado: ResMut<MercadoViewModel>,
    mut construcao: ResMut<ConstrucaoViewModel>,
    mut orcamento: ResMut<OrcamentoViewModel>,
    politica: Res<PoliticaViewModel>,
    diplomacia: Res<DiplomaciaViewModel>,
    ranking: Res<RankingViewModel>,
    mut notificacoes: ResMut<NotificacoesViewModel>,
    mut evento_modal: ResMut<EventoModalViewModel>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // Configura fontes no primeiro frame
    if !controle_abas.fontes_inicializadas {
        setup_cold_war_fonts(ctx);
        ctx.set_visuals(cold_war_visuals());
        ctx.set_global_style(cold_war_style());
        controle_abas.fontes_inicializadas = true;
    }

    // 1. Barra Superior Fixa (Top Bar - 48px)
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    egui::Panel::top("top_bar_hud")
        .exact_size(48.0)
        .frame(moldura_painel())
        .show(&mut viewport_ui, |ui| {
            render_top_bar(ui, &mut top_bar, &mut estado_global);
        });

    // 2. Painel Central da Tela Ativa
    egui::CentralPanel::default()
        .frame(moldura_painel())
        .show(&mut viewport_ui, |ui| match estado_global.tela_ativa {
            TelaAtiva::MapaGeralHud => {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.heading(
                        egui::RichText::new("GUERRA FRIA: 1945 — GABINETE DE SITUAÇÃO")
                            .strong()
                            .size(24.0)
                            .color(crate::theme::paleta::texto::OURO_DESTAQUE),
                    );
                    ui.label(
                        egui::RichText::new("Área Cartográfica Principal • Pressione F1 a F8 para alternar entre os ministérios e painéis.")
                            .color(crate::theme::paleta::texto::SECUNDARIO),
                    );

                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.label("Dica: Pressione [E] para simular a abertura da Conferência de Yalta (Modal Histórico).");
                    });
                });

                // Gaveta de Notificações ancorada no canto inferior esquerdo
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.set_max_width(420.0);
                    render_painel_notificacoes(ui, &mut notificacoes);
                });
            }
            TelaAtiva::PainelPais => {
                render_tela_pais(ui, &pais);
            }
            TelaAtiva::PainelEstado => {
                render_tela_estado(ui, &estado, &mut controle_abas.aba_estado);
            }
            TelaAtiva::Mercado => {
                render_tela_mercado(ui, &mut mercado);
            }
            TelaAtiva::Construcao => {
                render_tela_construcao(ui, &mut construcao);
            }
            TelaAtiva::Orcamento => {
                render_tela_orcamento(ui, &mut orcamento);
            }
            TelaAtiva::Politica => {
                render_tela_politica(ui, &politica);
            }
            TelaAtiva::Diplomacia => {
                render_tela_diplomacia(ui, &diplomacia);
            }
            TelaAtiva::RankingPotencias => {
                render_tela_ranking(ui, &ranking);
            }
        });

    // 3. Modal de Evento Histórico (se ativo)
    render_popup_evento(ctx, &mut evento_modal);
}
