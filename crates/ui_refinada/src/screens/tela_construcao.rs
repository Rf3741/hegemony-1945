//! Tela de Construção e Obras Públicas (F4).
//! Gerenciamento da fila prioritária nacional com ações de reordenação e catálogo de edifícios.

use bevy_egui::egui::{Color32, RichText, ScrollArea, Ui};

use crate::theme::estilo::{moldura_cartao, moldura_painel};
use crate::theme::paleta::{status, texto};
use crate::view_model::construcao::{ConstrucaoViewModel, ItemFilaConstrucaoViewModel};
use crate::widgets::meter::render_barra_progresso_industrial;

pub fn render_tela_construcao(ui: &mut Ui, vm: &mut ConstrucaoViewModel) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.heading("Diretoria de Obras e Mobilização Industrial");

            // Indicador de Capacidade de Construção (IC)
            moldura_painel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Capacidade Civil Disponível:").strong());
                    ui.label(
                        RichText::new(format!("{} / {} Pontos IC", vm.ic_em_uso, vm.ic_total_disponivel))
                            .color(texto::OURO_DESTAQUE)
                            .strong(),
                    );
                    ui.separator();
                    ui.label(format!("Eficiência Construtiva: {:.0}%", vm.eficiencia_construcao_pct));
                });
            });

            ui.add_space(10.0);

            // -----------------------------------------------------------------
            // Fila Nacional de Construção (Projetos em Andamento)
            // -----------------------------------------------------------------
            ui.heading("Fila Prioritária de Projetos Nacionais");
            moldura_painel().show(ui, |ui| {
                let mut indice_para_subir = None;
                let mut indice_para_descer = None;
                let mut indice_para_remover = None;

                let total_fila = vm.fila.len();
                for (idx, item) in vm.fila.iter_mut().enumerate() {
                    moldura_cartao().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("#{}.", idx + 1)).strong());
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&item.nome_projeto).strong().color(texto::PRIMARIO));
                                ui.label(
                                    RichText::new(format!(
                                        "Local: {} • Nível: {} ➔ {} • {} semanas restantes",
                                        item.nome_estado, item.nivel_atual, item.nivel_alvo, item.semanas_restantes
                                    ))
                                    .small()
                                    .color(texto::SECUNDARIO),
                                );
                            });

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                // Botão Cancelar
                                if ui.button(RichText::new("✕ Cancelar").color(status::PERIGO)).clicked() {
                                    indice_para_remover = Some(idx);
                                }

                                // Botão Pausar / Retomar
                                let txt_pausa = if item.pausado { "▶ Retomar" } else { "❚❚ Pausar" };
                                if ui.button(txt_pausa).clicked() {
                                    item.pausado = !item.pausado;
                                }

                                // Botões de Reordenação
                                if idx + 1 < total_fila && ui.button("▼").clicked() {
                                    indice_para_descer = Some(idx);
                                }
                                if idx > 0 && ui.button("▲").clicked() {
                                    indice_para_subir = Some(idx);
                                }

                                ui.label(format!("IC: {}", item.ic_alocada));
                            });
                        });

                        // Barra de Progresso do Projeto
                        ui.add_space(4.0);
                        let cor_barra = if item.pausado {
                            Color32::from_rgb(104, 116, 126)
                        } else {
                            status::SUCESSO
                        };
                        render_barra_progresso_industrial(ui, item.progresso_pct, 100.0, cor_barra, 400.0);
                        ui.label(RichText::new(format!("{:.0}% Concluído", item.progresso_pct)).small());
                    });
                    ui.add_space(4.0);
                }

                // Processa reordenações e remoções
                if let Some(idx) = indice_para_subir {
                    vm.fila.swap(idx, idx - 1);
                } else if let Some(idx) = indice_para_descer {
                    vm.fila.swap(idx, idx + 1);
                } else if let Some(idx) = indice_para_remover {
                    vm.fila.remove(idx);
                }
            });

            ui.add_space(12.0);

            // -----------------------------------------------------------------
            // Catálogo de Edifícios Disponíveis para Construção
            // -----------------------------------------------------------------
            ui.heading("Catálogo de Instalações e Complexos Fabris");
            let mut novo_projeto = None;

            for predio in &vm.catalogo {
                moldura_cartao().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&predio.nome).strong().color(texto::PRIMARIO));
                            ui.label(RichText::new(&predio.descricao).small().color(texto::SECUNDARIO));
                            ui.label(format!(
                                "Categoria: {} • Custo: {} Pontos IC • Postos: {} operários",
                                predio.categoria, predio.custo_pontos_ic, predio.empregos_por_nivel
                            ));
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new("+ Construir na Pensilvânia").strong().color(texto::OURO_DESTAQUE)).clicked() {
                                novo_projeto = Some(ItemFilaConstrucaoViewModel {
                                    id: format!("proj_{}", vm.fila.len() + 1),
                                    nome_projeto: predio.nome.clone(),
                                    nome_estado: "Pensilvânia".to_string(),
                                    nivel_atual: 1,
                                    nivel_alvo: 2,
                                    progresso_pct: 0.0,
                                    ic_alocada: 15,
                                    semanas_restantes: 24,
                                    pausado: false,
                                });
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }

            if let Some(item) = novo_projeto {
                vm.fila.push(item);
            }
        });
    });
}
