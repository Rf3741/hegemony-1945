//! Tela do País (Dossiê Nacional - F1).
//! Raio-X executivo do país: liderança, finanças semanais, IC civil/militar e esferas de aliança.

use bevy_egui::egui::{Color32, RichText, ScrollArea, Stroke, Ui, Vec2};

use crate::theme::estilo::{moldura_cartao, moldura_painel};
use crate::theme::paleta::{status, superficies, texto};
use crate::view_model::pais::PaisViewModel;
use crate::widgets::stamps::{carimbo_dourado, carimbo_vermelho};

pub fn render_tela_pais(ui: &mut Ui, vm: &PaisViewModel) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.vertical(|ui| {
            // -----------------------------------------------------------------
            // Cabeçalho de Dossiê Militar / Presidencial
            // -----------------------------------------------------------------
            ui.horizontal(|ui| {
                carimbo_vermelho(ui, "DOSSIÊ NACIONAL • CLASSIFICADO");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    carimbo_dourado(ui, "SUPERPOTÊNCIA #1");
                });
            });

            ui.add_space(8.0);

            // Bloco de Identificação e Líder
            moldura_painel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Silhueta / Retrato institucional
                    let (ret_rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 80.0), egui::Sense::hover());
                    ui.painter().rect_filled(ret_rect, 1.0, Color32::from_rgb(34, 38, 41));
                    ui.painter().rect_stroke(ret_rect, 1.0, Stroke::new(1.0, superficies::BORDER_MEDIUM), bevy_egui::egui::StrokeKind::Inside);
                    ui.painter().text(
                        ret_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "FDR",
                        egui::FontId::monospace(18.0),
                        texto::OURO_DESTAQUE,
                    );

                    ui.vertical(|ui| {
                        ui.heading(
                            RichText::new(&vm.nome_oficial)
                                .color(texto::PRIMARIO)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("Capital Política: {}", vm.capital))
                                .color(texto::SECUNDARIO),
                        );
                        ui.label(
                            RichText::new(format!("Regime: {}", vm.forma_governo))
                                .color(texto::PRIMARIO),
                        );
                        ui.label(
                            RichText::new(format!("Doutrina / Ideologia: {}", vm.ideologia_dominante))
                                .color(texto::SECUNDARIO),
                        );
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Chefe de Estado:").strong());
                            ui.label(&vm.lider_nome);
                            ui.label(RichText::new("• Vice-Presidente:").strong());
                            ui.label(&vm.vice_lider_nome);
                        });
                    });
                });
            });

            ui.add_space(12.0);

            // -----------------------------------------------------------------
            // Balanço Fiscal Semanal e Fluxo do Tesouro
            // -----------------------------------------------------------------
            ui.heading("Balanço do Tesouro e Finanças Públicas");
            ui.columns(2, |cols| {
                // Coluna 1: Receitas
                moldura_cartao().show(&mut cols[0], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Receitas Semanais:").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("+US$ {:.1}M", vm.receita_semanal_milhoes))
                                    .color(status::SUCESSO)
                                    .strong(),
                            );
                        });
                    });
                    ui.separator();
                    for (nome, val) in &vm.fontes_receita {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {}", nome));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("+US$ {:.1}M", val));
                            });
                        });
                    }
                });

                // Coluna 2: Despesas
                moldura_cartao().show(&mut cols[1], |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Despesas Semanais:").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(format!("-US$ {:.1}M", vm.despesa_semanal_milhoes))
                                    .color(status::PERIGO)
                                    .strong(),
                            );
                        });
                    });
                    ui.separator();
                    for (nome, val) in &vm.fontes_despesa {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {}", nome));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("-US$ {:.1}M", val));
                            });
                        });
                    }
                });
            });

            ui.add_space(8.0);

            // Resumo de Dívida e Ouro
            moldura_cartao().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Saldo Líquido Semanal:");
                    ui.label(
                        RichText::new(format!("+US$ {:.1}M/semana", vm.saldo_semanal_milhoes))
                            .color(status::SUCESSO)
                            .strong(),
                    );
                    ui.separator();
                    ui.label("Reservas de Ouro (Fort Knox):");
                    ui.label(
                        RichText::new(format!("US$ {:.1} Bilhões", vm.reservas_ouro_bilhoes))
                            .color(texto::OURO_DESTAQUE)
                            .strong(),
                    );
                    ui.separator();
                    ui.label("Dívida de Guerra:");
                    ui.label(format!("US$ {:.1}B (Teto: ${:.1}B)", vm.divida_publica_bilhoes, vm.teto_endividamento_bilhoes));
                });
            });

            ui.add_space(12.0);

            // -----------------------------------------------------------------
            // Capacidade Industrial (IC) Decomposta
            // -----------------------------------------------------------------
            ui.heading("Capacidade Industrial Nacional (IC)");
            moldura_cartao().show(ui, |ui| {
                let ic_total = vm.ic_fabricas_civis + vm.ic_fabricas_militares + vm.ic_estaleiros_navais;
                ui.horizontal(|ui| {
                    ui.label(format!("Total: {} Fábricas Ativas", ic_total));
                });

                ui.add_space(4.0);
                ui.columns(3, |cols| {
                    cols[0].group(|ui| {
                        ui.label(RichText::new("Fábricas Civis (Construção)").strong());
                        ui.label(format!("{} IC ({:.1}%)", vm.ic_fabricas_civis, (vm.ic_fabricas_civis as f32 / ic_total as f32) * 100.0));
                    });
                    cols[1].group(|ui| {
                        ui.label(RichText::new("Fábricas Militares (Arsenais)").strong());
                        ui.label(format!("{} IC ({:.1}%)", vm.ic_fabricas_militares, (vm.ic_fabricas_militares as f32 / ic_total as f32) * 100.0));
                    });
                    cols[2].group(|ui| {
                        ui.label(RichText::new("Estaleiros Navais").strong());
                        ui.label(format!("{} IC ({:.1}%)", vm.ic_estaleiros_navais, (vm.ic_estaleiros_navais as f32 / ic_total as f32) * 100.0));
                    });
                });
            });

            ui.add_space(12.0);

            // -----------------------------------------------------------------
            // Tratados e Alianças Geopolíticas
            // -----------------------------------------------------------------
            ui.heading("Tratados Globais e Alianças Estratégicas");
            moldura_cartao().show(ui, |ui| {
                ui.label(RichText::new(&vm.status_bretton_woods).strong());
                ui.label(RichText::new(&vm.status_lend_lease).color(texto::SECUNDARIO));
                ui.label(format!("• Rede de Nações Aliadas e Dependentes: {} governos alinhados", vm.num_aliados_esfera));
            });
        });
    });
}
