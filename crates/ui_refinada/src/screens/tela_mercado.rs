//! Tela de Mercado e Comércio (F3).
//! Cotações de bens com desvio percentual, ações comerciais e gráfico histórico com `egui_plot`.

use bevy_egui::egui::{RichText, ScrollArea, Ui};
use egui_plot::{Line, Plot, PlotPoints};

use crate::theme::estilo::{moldura_cartao, moldura_painel};
use crate::theme::paleta::{status, texto};
use crate::view_model::mercado::{MercadoViewModel, StatusComercio};

pub fn render_tela_mercado(ui: &mut Ui, vm: &mut MercadoViewModel) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.heading("Terminal de Cotações de Mercado & Balança Comercial");
            ui.label(
                RichText::new("Preços regulados por oferta, demanda e tabelamento da OPA (Office of Price Administration).")
                    .color(texto::SECUNDARIO),
            );

            ui.add_space(8.0);

            // -----------------------------------------------------------------
            // Tabela de Cotações
            // -----------------------------------------------------------------
            moldura_painel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Mercadoria / Bem").strong().color(texto::PRIMARIO));
                    ui.add_space(90.0);
                    ui.label(RichText::new("Preço Atual").strong());
                    ui.add_space(20.0);
                    ui.label(RichText::new("Preço Base").strong());
                    ui.add_space(20.0);
                    ui.label(RichText::new("Desvio %").strong());
                    ui.add_space(20.0);
                    ui.label(RichText::new("Oferta Nac.").strong());
                    ui.add_space(20.0);
                    ui.label(RichText::new("Demanda Nac.").strong());
                    ui.add_space(20.0);
                    ui.label(RichText::new("Saldo Líquido").strong());
                    ui.add_space(20.0);
                    ui.label(RichText::new("Ações Comerciais").strong());
                });

                ui.separator();

                for bem in &mut vm.bens {
                    let eh_selecionado = bem.id == vm.bem_selecionado_id;
                    ui.horizontal(|ui| {
                        let prefixo = if eh_selecionado { "▶ " } else { "  " };
                        let btn_txt = format!("{}{}", prefixo, bem.nome);
                        if ui.selectable_label(eh_selecionado, btn_txt).clicked() {
                            vm.bem_selecionado_id = bem.id.clone();
                        }

                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("${:.2}", bem.preco_atual)).monospace().strong());
                        ui.add_space(35.0);
                        ui.label(RichText::new(format!("${:.2}", bem.preco_base)).monospace().color(texto::SECUNDARIO));
                        ui.add_space(35.0);

                        let cor_desvio = if bem.desvio_pct > 5.0 {
                            status::PERIGO // Inflação / escassez
                        } else if bem.desvio_pct < -5.0 {
                            status::SUCESSO // Barato / abundante
                        } else {
                            texto::SECUNDARIO
                        };
                        ui.label(RichText::new(format!("{:+.1}%", bem.desvio_pct)).monospace().color(cor_desvio));

                        ui.add_space(35.0);
                        ui.label(RichText::new(format!("{:.0}", bem.oferta_nacional)).monospace());
                        ui.add_space(35.0);
                        ui.label(RichText::new(format!("{:.0}", bem.demanda_nacional)).monospace());
                        ui.add_space(35.0);

                        let cor_saldo = if bem.saldo_liquido >= 0.0 {
                            status::SUCESSO
                        } else {
                            status::PERIGO
                        };
                        ui.label(RichText::new(format!("{:+.0}", bem.saldo_liquido)).monospace().color(cor_saldo));

                        ui.add_space(25.0);
                        // Ações interativas de comércio
                        let tag_status = match bem.status {
                            StatusComercio::Neutro => "Livre",
                            StatusComercio::Exportando => "Exportando",
                            StatusComercio::Importando => "Importando",
                            StatusComercio::Subsidiado => "Subsidiado",
                            StatusComercio::Tarifado => "Tarifado",
                        };

                        if ui.button(tag_status).clicked() {
                            bem.status = match bem.status {
                                StatusComercio::Neutro => StatusComercio::Exportando,
                                StatusComercio::Exportando => StatusComercio::Importando,
                                StatusComercio::Importando => StatusComercio::Subsidiado,
                                StatusComercio::Subsidiado => StatusComercio::Tarifado,
                                StatusComercio::Tarifado => StatusComercio::Neutro,
                            };
                        }
                    });
                }
            });

            ui.add_space(12.0);

            // -----------------------------------------------------------------
            // Gráfico Histórico de Preços (egui_plot) e Detalhamento do Bem
            // -----------------------------------------------------------------
            ui.heading(format!(
                "Análise Estrutural & Curva de Preços: {}",
                vm.bem_selecionado_id.to_uppercase()
            ));

            ui.columns(2, |cols| {
                // Coluna 1: Gráfico egui_plot
                moldura_cartao().show(&mut cols[0], |ui| {
                    ui.label(RichText::new("Evolução Histórica da Cotação (1940 - 1945)").strong());

                    let pontos: PlotPoints = vm
                        .historico_precos
                        .iter()
                        .map(|p| [p.tempo, p.preco])
                        .collect();

                    let linha = Line::new("Cotação US$", pontos)
                        .color(texto::OURO_DESTAQUE)
                        .width(2.5);

                    Plot::new("plot_mercado")
                        .view_aspect(2.2)
                        .allow_zoom(false)
                        .allow_drag(false)
                        .show(ui, |plot_ui| {
                            plot_ui.line(linha);
                        });
                });

                // Coluna 2: Maiores Produtores e Consumidores
                moldura_cartao().show(&mut cols[1], |ui| {
                    ui.label(RichText::new("Principais Províncias Produtoras:").strong());
                    for (prov, vol) in &vm.maiores_produtores {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {}", prov));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("{:.0} un/sem", vol));
                            });
                        });
                    }

                    ui.separator();
                    ui.label(RichText::new("Principais Setores Consumidores:").strong());
                    for (setor, vol) in &vm.maiores_consumidores {
                        ui.horizontal(|ui| {
                            ui.label(format!("• {}", setor));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(format!("{:.0} un/sem", vol));
                            });
                        });
                    }
                });
            });
        });
    });
}
