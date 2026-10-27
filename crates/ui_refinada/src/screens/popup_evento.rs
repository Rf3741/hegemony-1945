//! Modal de Decisão de Evento Histórico (ex: Conferência de Yalta de 1945).
//! Estilo documento confidencial em papel manila com carimbos e opções com consequências.

use bevy_egui::egui::{RichText, Window};

use crate::theme::estilo::moldura_papel_dossie;
use crate::theme::paleta::dossie_papel;
use crate::view_model::eventos::EventoModalViewModel;
use crate::widgets::stamps::carimbo_vermelho;

pub fn render_popup_evento(ctx: &egui::Context, vm: &mut EventoModalViewModel) {
    if !vm.aberto {
        return;
    }

    Window::new("Relatório de Estado-Maior")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(moldura_papel_dossie())
        .show(ctx, |ui| {
            ui.set_max_width(580.0);

            ui.horizontal(|ui| {
                carimbo_vermelho(ui, &vm.carimbo);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(&vm.data_evento)
                            .monospace()
                            .strong()
                            .color(dossie_papel::TINTA_MAQUINA),
                    );
                });
            });

            ui.add_space(8.0);
            ui.heading(
                RichText::new(&vm.titulo)
                    .strong()
                    .color(dossie_papel::TINTA_MAQUINA),
            );
            ui.label(
                RichText::new(&vm.subtitulo)
                    .italics()
                    .color(dossie_papel::TINTA_DESGASTADA),
            );

            ui.separator();
            ui.add_space(4.0);

            ui.label(
                RichText::new(&vm.texto_narrativo)
                    .color(dossie_papel::TINTA_MAQUINA),
            );

            ui.add_space(14.0);
            ui.separator();
            ui.label(RichText::new("Decisões de Gabinete:").strong().color(dossie_papel::TINTA_MAQUINA));
            ui.add_space(4.0);

            for opcao in vm.opcoes.iter() {
                ui.group(|ui| {
                    ui.label(RichText::new(&opcao.titulo).strong().color(dossie_papel::TINTA_MAQUINA));
                    ui.label(RichText::new(&opcao.texto_efeito).small().color(dossie_papel::TINTA_DESGASTADA));
                    ui.add_space(4.0);

                    if ui.button(RichText::new("Confirmar Diretriz").color(dossie_papel::TINTA_MAQUINA)).clicked() {
                        // Fecha o modal ao selecionar uma opção
                        vm.aberto = false;
                    }
                });
                ui.add_space(4.0);
            }
        });
}
