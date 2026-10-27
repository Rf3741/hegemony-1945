//! Tela de Orçamento e Tesouro (F5).
//! Gestão fiscal com sliders interativos e projeção em tempo real de receitas, despesas e dívida.

use bevy_egui::egui::{RichText, ScrollArea, Slider, Ui};

use crate::theme::estilo::{moldura_cartao, moldura_painel};
use crate::theme::paleta::{status, texto};
use crate::view_model::orcamento::OrcamentoViewModel;

pub fn render_tela_orcamento(ui: &mut Ui, vm: &mut OrcamentoViewModel) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.heading("Gabinete Fiscal e Política Tributária de Guerra");
            ui.label(
                RichText::new("Ajuste as alíquotas tributárias e prioridades de custeio público. O saldo semanal reflete o impacto imediato na estabilidade financeira.")
                    .color(texto::SECUNDARIO),
            );

            ui.add_space(8.0);

            // -----------------------------------------------------------------
            // Painel Sintético de Balanço Semanal em Tempo Real
            // -----------------------------------------------------------------
            let receita = vm.receita_semanal_estimada();
            let despesa = vm.despesa_semanal_estimada();
            let saldo = vm.saldo_liquido_semanal();
            let cor_saldo = if saldo >= 0.0 {
                status::SUCESSO
            } else {
                status::PERIGO
            };

            moldura_painel().show(ui, |ui| {
                ui.columns(3, |cols| {
                    cols[0].group(|ui| {
                        ui.label(RichText::new("Receita Semanal Bruta").strong());
                        ui.label(
                            RichText::new(format!("+US$ {:.1}M", receita))
                                .strong()
                                .color(status::SUCESSO),
                        );
                    });

                    cols[1].group(|ui| {
                        ui.label(RichText::new("Despesa Semanal Total").strong());
                        ui.label(
                            RichText::new(format!("-US$ {:.1}M", despesa))
                                .strong()
                                .color(status::PERIGO),
                        );
                    });

                    cols[2].group(|ui| {
                        ui.label(RichText::new("Saldo Líquido Semanal").strong());
                        ui.label(
                            RichText::new(format!("{:+.1}M $/sem", saldo))
                                .strong()
                                .color(cor_saldo),
                        );
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Dívida de Guerra Acumulada: US$ {:.1}B", vm.divida_publica_bilhoes));
                    ui.separator();
                    ui.label(format!("Teto Constitucional: US$ {:.1}B", vm.teto_endividamento_bilhoes));
                    ui.separator();
                    ui.label(format!("Taxa Efetiva de Juros: {:.2}% a.a.", vm.taxa_juros_efetiva_pct()));
                    ui.separator();
                    ui.label(format!("Títulos de Guerra Subscritos: US$ {:.1}B", vm.titulos_guerra_inscritos_bilhoes));
                });
            });

            ui.add_space(12.0);

            // -----------------------------------------------------------------
            // Sliders de Tributação e Despesas
            // -----------------------------------------------------------------
            ui.columns(2, |cols| {
                // Coluna 1: Alíquotas de Impostos
                moldura_cartao().show(&mut cols[0], |ui| {
                    ui.heading("Alíquotas Tributárias");
                    ui.label(RichText::new("Receitas sobre o capital e a população:").small().color(texto::SECUNDARIO));
                    ui.add_space(6.0);

                    ui.label("Imposto sobre Renda Alta & Lucros Corporativos:");
                    ui.add(Slider::new(&mut vm.imposto_renda_alta_pct, 40.0..=95.0).suffix("%"));

                    ui.add_space(4.0);
                    ui.label("Imposto sobre Classe Média & Especialistas:");
                    ui.add(Slider::new(&mut vm.imposto_classe_media_pct, 10.0..=45.0).suffix("%"));

                    ui.add_space(4.0);
                    ui.label("Imposto sobre Classe Trabalhadora & Pops:");
                    ui.add(Slider::new(&mut vm.imposto_trabalhadores_pct, 5.0..=30.0).suffix("%"));

                    ui.add_space(4.0);
                    ui.label("Tarifas Alfandegárias de Importação:");
                    ui.add(Slider::new(&mut vm.tarifa_importacao_pct, 0.0..=50.0).suffix("%"));

                    ui.add_space(4.0);
                    ui.label("Impostos sobre Consumo & Vendas Gerais:");
                    ui.add(Slider::new(&mut vm.imposto_consumo_pct, 0.0..=25.0).suffix("%"));
                });

                // Coluna 2: Despesas Públicas
                moldura_cartao().show(&mut cols[1], |ui| {
                    ui.heading("Orçamento de Despesas Públicas");
                    ui.label(RichText::new("Custeio militar, ciência e seguridade:").small().color(texto::SECUNDARIO));
                    ui.add_space(6.0);

                    ui.label("Forças Armadas (Exército e Marinha):");
                    ui.add(Slider::new(&mut vm.despesa_forcas_armadas_milhoes, 40.0..=150.0).prefix("$").suffix("M/sem"));

                    ui.add_space(4.0);
                    ui.label("Pesquisa Estratégica (Projeto Manhattan):");
                    ui.add(Slider::new(&mut vm.despesa_pesquisa_manhattan_milhoes, 5.0..=30.0).prefix("$").suffix("M/sem"));

                    ui.add_space(4.0);
                    ui.label("Burocracia Federal & Eficiência Fiscal:");
                    ui.add(Slider::new(&mut vm.despesa_burocracia_federal_milhoes, 10.0..=40.0).prefix("$").suffix("M/sem"));

                    ui.add_space(4.0);
                    ui.label("Subsídios à Indústria & Construção:");
                    ui.add(Slider::new(&mut vm.despesa_subsidios_industriais_milhoes, 5.0..=40.0).prefix("$").suffix("M/sem"));

                    ui.add_space(4.0);
                    ui.label("Benefícios a Veteranos (GI Bill):");
                    ui.add(Slider::new(&mut vm.despesa_veteranos_gi_bill_milhoes, 0.0..=30.0).prefix("$").suffix("M/sem"));
                });
            });
        });
    });
}
