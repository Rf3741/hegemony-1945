//! Medidores industriais e barras de status para Guerra Fria 1945.

use bevy_egui::egui::{Color32, Rect, Response, Sense, Stroke, Ui, Vec2};

pub fn render_defcon_badge(ui: &mut Ui, nivel: u8) -> Response {
    let (cor_fundo, cor_texto, texto) = match nivel {
        1 => (Color32::from_rgb(239, 68, 68), Color32::WHITE, "DEFCON 1 (GUERRA TOTAL)"),
        2 => (Color32::from_rgb(220, 38, 38), Color32::WHITE, "DEFCON 2 (IMINENTE)"),
        3 => (Color32::from_rgb(249, 115, 22), Color32::BLACK, "DEFCON 3 (PRONTIDÃO)"),
        4 => (Color32::from_rgb(234, 179, 8), Color32::BLACK, "DEFCON 4 (VIGILÂNCIA)"),
        _ => (Color32::from_rgb(59, 130, 246), Color32::WHITE, "DEFCON 5 (NORMALIDADE)"),
    };

    let (rect, response) = ui.allocate_exact_size(Vec2::new(140.0, 22.0), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 1.0, cor_fundo);
    painter.rect_stroke(rect, 1.0, Stroke::new(1.0, Color32::from_rgb(24, 27, 29)), bevy_egui::egui::StrokeKind::Inside);

    let pos_texto = rect.center() - Vec2::new(55.0, 6.0);
    painter.text(
        pos_texto,
        egui::Align2::LEFT_TOP,
        texto,
        egui::FontId::monospace(10.0),
        cor_texto,
    );

    response.on_hover_text(format!("Nível DEFCON {}: Mede o estado de alerta das forças armadas globais.", nivel))
}

pub fn render_barra_progresso_industrial(ui: &mut Ui, valor: f32, maximo: f32, cor: Color32, largura: f32) {
    let frac = (valor / maximo).clamp(0.0, 1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(largura, 10.0), Sense::hover());
    let painter = ui.painter_at(rect);

    // Fundo escuro da calha
    painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 22, 24));
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_rgb(46, 53, 59)), bevy_egui::egui::StrokeKind::Inside);

    // Barra preenchida
    let rect_preenchido = Rect::from_min_max(
        rect.min,
        egui::pos2(rect.min.x + rect.width() * frac, rect.max.y),
    );
    painter.rect_filled(rect_preenchido, 0.0, cor);
}
