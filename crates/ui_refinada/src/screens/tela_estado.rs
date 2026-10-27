//! Tela do Estado / Província (F2).
//! Gestão regional com Gráficos de Pizza de Pops (Profissão, Cultura, Religião),
//! prédios produtivos e recursos naturais.

use bevy_egui::egui::{RichText, ScrollArea, Ui};

use crate::theme::estilo::{moldura_cartao, moldura_painel};
use crate::theme::paleta::{status, texto};
use crate::view_model::estado::EstadoViewModel;
use crate::widgets::pie_chart::render_pie_chart;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbaEstado {
    DemografiaPops,
    PrediosEIndustrias,
    RecursosLocais,
}

pub fn render_tela_estado(
    ui: &mut Ui,
    vm: &EstadoViewModel,
    aba_atual: &mut AbaEstado,
) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.vertical(|ui| {
            // -----------------------------------------------------------------
            // Cabeçalho da Região Administrativa (Admin-1)
            // -----------------------------------------------------------------
            moldura_painel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(
                            RichText::new(format!("{} [{}]", vm.nome, vm.adm1_code))
                                .strong()
                                .color(texto::PRIMARIO),
                        );
                        ui.label(
                            RichText::new(format!("Centro de Comando: {} • País: {}", vm.capital_regional, vm.pais_tag))
                                .color(texto::SECUNDARIO),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.vertical(|ui| {
                            let sol_color = if vm.padrao_vida_sol >= 50.0 {
                                status::SUCESSO
                            } else if vm.padrao_vida_sol >= 25.0 {
                                status::AVISO
                            } else {
                                status::PERIGO
                            };

                            ui.label(
                                RichText::new(format!("Padrão de Vida (SoL): {:.1}", vm.padrao_vida_sol))
                                    .strong()
                                    .color(sol_color),
                            );
                            ui.label(format!("Agitação Operária: {:.1}%", vm.agitacao_social_pct));
                        });
                    });
                });

                ui.add_space(6.0);
                ui.separator();

                // Índices Rápidos
                ui.horizontal(|ui| {
                    ui.label(format!("População Total: {:.2}M", vm.populacao_total as f64 / 1_000_000.0));
                    ui.separator();
                    ui.label(format!("Mão de Obra Ativa: {:.2}M", vm.mao_de_obra_total as f64 / 1_000_000.0));
                    ui.separator();
                    ui.label(format!(
                        "Infraestrutura: {} / {} (Acesso ao Mercado: {:.0}%)",
                        vm.infraestrutura_usada, vm.infraestrutura_fornecida, vm.acesso_ao_mercado_pct
                    ));
                });
            });

            ui.add_space(8.0);

            // -----------------------------------------------------------------
            // Seletor de Abas Locais
            // -----------------------------------------------------------------
            ui.horizontal(|ui| {
                let abas = [
                    (AbaEstado::DemografiaPops, "Pops & Estratificação (Pizza)"),
                    (AbaEstado::PrediosEIndustrias, "Complexos Industriais & Prédios"),
                    (AbaEstado::RecursosLocais, "Recursos Naturais & Jazidas"),
                ];

                for (aba, titulo) in abas {
                    let ativo = *aba_atual == aba;
                    let btn = if ativo {
                        egui::Button::new(RichText::new(titulo).strong().color(texto::OURO_DESTAQUE))
                    } else {
                        egui::Button::new(RichText::new(titulo).color(texto::SECUNDARIO))
                    };

                    if ui.add(btn).clicked() {
                        *aba_atual = aba;
                    }
                }
            });

            ui.add_space(8.0);

            // -----------------------------------------------------------------
            // Conteúdo da Aba Ativa
            // -----------------------------------------------------------------
            match aba_atual {
                AbaEstado::DemografiaPops => {
                    ui.columns(3, |cols| {
                        cols[0].group(|ui| {
                            render_pie_chart(
                                ui,
                                &vm.pops_profissoes,
                                75.0,
                                "Por Profissão / Estrato",
                            );
                        });

                        cols[1].group(|ui| {
                            render_pie_chart(
                                ui,
                                &vm.pops_culturas,
                                75.0,
                                "Por Herança Cultural",
                            );
                        });

                        cols[2].group(|ui| {
                            render_pie_chart(
                                ui,
                                &vm.pops_religioes,
                                75.0,
                                "Por Religião / Confissão",
                            );
                        });
                    });
                }
                AbaEstado::PrediosEIndustrias => {
                    for predio in &vm.predios {
                        moldura_cartao().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.heading(
                                        RichText::new(format!("{} (Nível {})", predio.nome, predio.nivel))
                                            .strong()
                                            .color(texto::PRIMARIO),
                                    );
                                    ui.label(
                                        RichText::new(format!("Método de Produção: {}", predio.metodo_producao))
                                            .color(texto::SECUNDARIO),
                                    );
                                    ui.label(format!(
                                        "Empregos: {} / {} operários ({:.1}%)",
                                        predio.trabalhadores_empregados,
                                        predio.trabalhadores_capacidade,
                                        (predio.trabalhadores_empregados as f32 / predio.trabalhadores_capacidade as f32) * 100.0
                                    ));
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let cor_lucro = if predio.lucro_semanal_mil >= 0.0 {
                                        status::SUCESSO
                                    } else {
                                        status::PERIGO
                                    };
                                    ui.label(
                                        RichText::new(format!("{:+.1}k $/sem", predio.lucro_semanal_mil))
                                            .strong()
                                            .color(cor_lucro),
                                    );
                                });
                            });

                            ui.separator();
                            ui.columns(2, |cols| {
                                cols[0].label(RichText::new("Consumo Semanal de Insumos:").strong());
                                for (insumo, qtd) in &predio.consumo_insumos {
                                    cols[0].label(format!("- {:.0}x {}", qtd, insumo));
                                }

                                cols[1].label(RichText::new("Produção Semanal de Bens:").strong());
                                for (saida, qtd) in &predio.producao_saidas {
                                    cols[1].label(format!("+ {:.0}x {}", qtd, saida));
                                }
                            });
                        });
                        ui.add_space(6.0);
                    }
                }
                AbaEstado::RecursosLocais => {
                    for rec in &vm.recursos {
                        moldura_cartao().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.heading(&rec.nome);
                                    ui.label(format!("Bem Gerado: {}", rec.tipo_bem_associado));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Exploração: {} de {} níveis máximos",
                                            rec.explorado_niveis, rec.limite_maximo
                                        ))
                                        .strong()
                                        .color(texto::OURO_DESTAQUE),
                                    );
                                });
                            });
                        });
                        ui.add_space(4.0);
                    }
                }
            }
        });
    });
}
