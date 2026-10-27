//! Tela de Ranking das Grandes Potências Mundiais (F8).
//! Hierarquia geopolítica de 1945 baseada em PIB, força bélica, alianças e tecnologia.

use bevy_egui::egui::{RichText, ScrollArea, Ui};

use crate::theme::estilo::moldura_painel;
use crate::theme::paleta::{cor_pais, texto};
use crate::view_model::ranking::RankingViewModel;

pub fn render_tela_ranking(ui: &mut Ui, vm: &RankingViewModel) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.heading("Concerto Global: Ranking das Grandes Potências (01/01/1945)");
            ui.label(
                RichText::new("O prestígio é calculado pela ponderação: 40% PIB + 30% Força Militar + 20% Esfera Diplomática + 10% Tecnologia.")
                    .color(texto::SECUNDARIO),
            );

            ui.add_space(8.0);

            moldura_painel().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Pos").strong());
                    ui.add_space(15.0);
                    ui.label(RichText::new("País / Grande Potência").strong().color(texto::PRIMARIO));
                    ui.add_space(160.0);
                    ui.label(RichText::new("Prestígio").strong().color(texto::OURO_DESTAQUE));
                    ui.add_space(20.0);
                    ui.label(RichText::new("PIB").strong());
                    ui.add_space(20.0);
                    ui.label(RichText::new("Militar").strong());
                    ui.add_space(20.0);
                    ui.label(RichText::new("Diplomacia").strong());
                    ui.add_space(20.0);
                    ui.label(RichText::new("Tecnologia").strong());
                });

                ui.separator();

                for pot in &vm.potencias {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("#{}.", pot.posicao)).monospace().strong());
                        ui.add_space(10.0);

                        // Tag com cor nacional
                        let cor = cor_pais(&pot.tag);
                        ui.label(RichText::new(format!("[{}] {}", pot.tag, pot.nome)).strong().color(cor));

                        ui.add_space(30.0);
                        ui.label(RichText::new(format!("{}", pot.pontos_prestigio)).monospace().strong().color(texto::OURO_DESTAQUE));

                        ui.add_space(35.0);
                        ui.label(RichText::new(format!("{}", pot.pib_pontos)).monospace());
                        ui.add_space(35.0);
                        ui.label(RichText::new(format!("{}", pot.forca_militar_pontos)).monospace());
                        ui.add_space(35.0);
                        ui.label(RichText::new(format!("{}", pot.esfera_diplomatica_pontos)).monospace());
                        ui.add_space(35.0);
                        ui.label(RichText::new(format!("{}", pot.tecnologia_pontos)).monospace());
                    });
                }
            });
        });
    });
}
