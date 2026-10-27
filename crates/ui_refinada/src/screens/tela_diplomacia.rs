//! Tela de Diplomacia e Esferas de Influência (F7).
//! Comparativo bipolar (EUA x URSS), termômetro DEFCON e ações diplomáticas.

use bevy_egui::egui::{RichText, ScrollArea, Ui};

use crate::theme::estilo::{moldura_cartao, moldura_painel};
use crate::theme::paleta::{blocos, status, texto};
use crate::view_model::diplomacia::DiplomaciaViewModel;

pub fn render_tela_diplomacia(ui: &mut Ui, vm: &DiplomaciaViewModel) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.heading("Teatro Diplomático Global & Polarização Bipolar");

            // -----------------------------------------------------------------
            // Comparativo Bipolar: Bloco Ocidental x Bloco Soviético
            // -----------------------------------------------------------------
            ui.columns(2, |cols| {
                // Bloco Ocidental
                moldura_painel().show(&mut cols[0], |ui| {
                    ui.label(RichText::new("★ BLOCO OCIDENTAL (Capitalismo / Aliados)").strong().color(blocos::OCIDENTAL_CLARA));
                    ui.separator();
                    ui.label(format!("PIB Combinado: US$ {:.1} Bilhões", vm.pib_bloco_ocidental_bilhoes));
                    ui.label(format!("Divisões Militares Ativas: {} Divisões", vm.divisoes_bloco_ocidental));
                    ui.label("Doutrina Externa: Livre Comércio e Padrão-Ouro Bretton Woods");
                });

                // Bloco Soviético
                moldura_painel().show(&mut cols[1], |ui| {
                    ui.label(RichText::new("★ BLOCO SOVIÉTICO (Socialismo / Pacto)").strong().color(blocos::SOVIETICO_CLARA));
                    ui.separator();
                    ui.label(format!("PIB Combinado: US$ {:.1} Bilhões", vm.pib_bloco_sovietico_bilhoes));
                    ui.label(format!("Divisões Militares Ativas: {} Divisões", vm.divisoes_bloco_sovietico));
                    ui.label("Doutrina Externa: Economia Planificada e Vanguarda Revolucionária");
                });
            });

            ui.add_space(10.0);

            // -----------------------------------------------------------------
            // Dossiê de Relações Bilaterais com Moscou
            // -----------------------------------------------------------------
            moldura_cartao().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("Dossiê Bilateral: Washington ➔ Moscou");
                        ui.label(RichText::new(&vm.status_relacao_urss).strong().color(status::AVISO));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("Relação: {:+}", vm.relacao_bilateral_urss))
                                .strong()
                                .color(status::AVISO),
                        );
                    });
                });

                ui.separator();
                ui.label(RichText::new("Pontos Críticos de Fricção Territorial em 1945:").strong());
                for cap in &vm.capitais_em_disputa {
                    ui.label(format!("• {}", cap));
                }
            });

            ui.add_space(12.0);

            // -----------------------------------------------------------------
            // Ações Diplomáticas e Clandestinas Disponíveis
            // -----------------------------------------------------------------
            ui.heading("Iniciativas Diplomáticas Estratégicas");
            for acao in &vm.acoes_disponiveis {
                moldura_cartao().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&acao.nome).strong().color(texto::PRIMARIO));
                            ui.label(RichText::new(&acao.descricao).small().color(texto::SECUNDARIO));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let btn_txt = format!("Executar (-{} Prestígio)", acao.custo_prestigio);
                            if ui.button(RichText::new(btn_txt).color(texto::OURO_DESTAQUE)).clicked() {
                                // Executa ação diplomática
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });
    });
}
