//! Carimbos militares e burocráticos de gabinete de guerra (1945).
//! Ex: "CONFIDENCIAL", "ULTRASSECRETO", "URGENTE".

use bevy_egui::egui::{Color32, RichText, Stroke, Ui, Vec2};

pub fn carimbo_vermelho(ui: &mut Ui, texto: &str) {
    let _padding = Vec2::new(8.0, 3.0);
    let cor_carimbo = Color32::from_rgb(163, 39, 34);

    ui.horizontal(|ui| {
        egui::Frame {
            inner_margin: egui::Margin::symmetric(8, 2),
            fill: Color32::from_rgba_unmultiplied(163, 39, 34, 30),
            stroke: Stroke::new(2.0, cor_carimbo),
            corner_radius: egui::CornerRadius::same(1),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.label(
                RichText::new(texto)
                    .monospace()
                    .strong()
                    .color(cor_carimbo),
            );
        });
    });
}

pub fn carimbo_dourado(ui: &mut Ui, texto: &str) {
    let cor_ouro = Color32::from_rgb(223, 177, 91);

    ui.horizontal(|ui| {
        egui::Frame {
            inner_margin: egui::Margin::symmetric(8, 2),
            fill: Color32::from_rgba_unmultiplied(223, 177, 91, 25),
            stroke: Stroke::new(1.5, cor_ouro),
            corner_radius: egui::CornerRadius::same(1),
            ..Default::default()
        }
        .show(ui, |ui| {
            ui.label(
                RichText::new(texto)
                    .monospace()
                    .strong()
                    .color(cor_ouro),
            );
        });
    });
}
