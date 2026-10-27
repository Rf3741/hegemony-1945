//! Tela de Política, Leis e Grupos de Interesse (F6).
//! 7 categorias institucionais de leis vigentes, projeto em tramitação parlamentar e forças sociais (IIGG).

use bevy_egui::egui::{RichText, ScrollArea, Ui};

use crate::theme::estilo::{moldura_cartao, moldura_painel};
use crate::theme::paleta::{status, texto};
use crate::view_model::politica::PoliticaViewModel;
use crate::widgets::meter::render_barra_progresso_industrial;

pub fn render_tela_politica(ui: &mut Ui, vm: &PoliticaViewModel) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.heading("Gabinete Político, Legislação e Grupos de Pressão (IIGG)");
            ui.label(
                RichText::new("A sustentabilidade do governo depende da aprovação dos Grupos de Interesse e da legitimidade institucional.")
                    .color(texto::SECUNDARIO),
            );

            ui.add_space(8.0);

            // -----------------------------------------------------------------
            // Projeto de Lei em Tramitação Parlamentar
            // -----------------------------------------------------------------
            if let Some(ref tramitacao) = vm.lei_em_tramitacao {
                moldura_painel().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("★ PROJETO DE LEI EM TRAMITAÇÃO").strong().color(texto::OURO_DESTAQUE));
                            ui.heading(&tramitacao.nome_projeto);
                            ui.label(RichText::new(format!("Fase Legislativa: {}", tramitacao.fase_atual)).color(texto::SECUNDARIO));
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("Próxima checagem em {} dias", tramitacao.dias_para_proxima_votacao));
                        });
                    });

                    ui.add_space(6.0);
                    render_barra_progresso_industrial(ui, tramitacao.progresso_pct, 100.0, status::INFO, 480.0);
                    ui.label(format!("{:.0}% de Progresso no Plenário", tramitacao.progresso_pct));

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Aprovação: {:.0}%", tramitacao.probabilidade_aprovacao_pct)).color(status::SUCESSO).strong());
                        ui.separator();
                        ui.label(RichText::new(format!("Emendas: {:.0}%", tramitacao.probabilidade_emendas_pct)).color(status::AVISO).strong());
                        ui.separator();
                        ui.label(RichText::new(format!("Bloqueio: {:.0}%", tramitacao.probabilidade_bloqueio_pct)).color(status::PERIGO).strong());
                    });
                });
            }

            ui.add_space(12.0);

            // -----------------------------------------------------------------
            // Leis Vigentes (7 Categorias Institucionais)
            // -----------------------------------------------------------------
            ui.heading("Legislação Institucional Vigente");
            for lei in &vm.leis_ativas {
                moldura_cartao().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&lei.categoria_nome).small().color(texto::OURO_DESTAQUE));
                            ui.label(RichText::new(&lei.lei_nome).strong().color(texto::PRIMARIO));
                            ui.label(RichText::new(&lei.descricao_efeito).small().color(texto::SECUNDARIO));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Propor Reforma").clicked() {
                                // Ação de reforma de lei
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }

            ui.add_space(12.0);

            // -----------------------------------------------------------------
            // Grupos de Interesse (IIGG)
            // -----------------------------------------------------------------
            ui.heading("Grupos de Interesse Sociopolíticos (IIGG)");
            for ig in &vm.grupos_interesse {
                moldura_cartao().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let titulo = if ig.esta_no_governo {
                                format!("{} [No Governo]", ig.nome)
                            } else {
                                ig.nome.clone()
                            };

                            ui.label(RichText::new(titulo).strong().color(texto::PRIMARIO));
                            ui.label(RichText::new(&ig.base_social_descricao).small().color(texto::SECUNDARIO));
                            ui.label(RichText::new(&ig.pautas_principais).small().color(texto::OURO_DESTAQUE));
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let cor_aprovacao = ig.cor_aprovacao();
                            ui.label(
                                RichText::new(format!("Aprovação: {:+} ({})", ig.aprovacao, ig.estado_lealdade))
                                    .strong()
                                    .color(cor_aprovacao),
                            );
                            ui.label(format!("Força Política (Clout): {:.1}%", ig.forca_politica_clout_pct));
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });
    });
}
