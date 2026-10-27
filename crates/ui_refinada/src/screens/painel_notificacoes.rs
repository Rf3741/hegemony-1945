//! Painel de Notificações e Telegramas de Gabinete.
//! Estilo papel datilografado com carimbos e fita de teletipo.

use bevy_egui::egui::{RichText, ScrollArea, Ui, Vec2};

use crate::theme::estilo::moldura_papel_dossie;
use crate::theme::paleta::{dossie_papel, status};
use crate::view_model::notificacoes::{NotificacoesViewModel, TipoTelegrama};
use crate::widgets::stamps::carimbo_vermelho;

pub fn render_painel_notificacoes(ui: &mut Ui, vm: &mut NotificacoesViewModel) {
    moldura_papel_dossie().show(ui, |ui| {
        ui.horizontal(|ui| {
            carimbo_vermelho(ui, "DESPACHOS DIPLOMÁTICOS");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn_txt = if vm.gaveta_expandida { "▲ Minimizar" } else { "▼ Expandir" };
                if ui.button(RichText::new(btn_txt).color(dossie_papel::TINTA_MAQUINA)).clicked() {
                    vm.gaveta_expandida = !vm.gaveta_expandida;
                }
            });
        });

        if vm.gaveta_expandida {
            ui.add_space(4.0);
            ui.separator();

            ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for telex in &mut vm.telegramas {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let cor_tipo = match telex.tipo {
                                TipoTelegrama::Critico => status::PERIGO,
                                TipoTelegrama::Importante => status::AVISO,
                                TipoTelegrama::Oportunidade => status::SUCESSO,
                                TipoTelegrama::Informativo => status::INFO,
                            };

                            let (indicador_rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
                            ui.painter().circle_filled(indicador_rect.center(), 4.0, cor_tipo);

                            ui.label(
                                RichText::new(format!("[{}] {}", telex.tag_origem, telex.assunto))
                                    .strong()
                                    .color(dossie_papel::TINTA_MAQUINA),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    RichText::new(&telex.data_hora)
                                        .monospace()
                                        .small()
                                        .color(dossie_papel::TINTA_DESGASTADA),
                                );
                            });
                        });

                        ui.label(
                            RichText::new(&telex.mensagem_completa)
                                .small()
                                .color(dossie_papel::TINTA_DESGASTADA),
                        );
                    });
                    ui.add_space(2.0);
                }
            });
        }
    });
}
