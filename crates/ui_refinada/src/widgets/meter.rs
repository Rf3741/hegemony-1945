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

/// Barra de status em tema de papel: calha creme com moldura de tinta e preenchimento sólido.
/// `rotulo` opcional aparece à esquerda em caixa alta; `texto_direita` (ex.: "72%") à direita.
pub fn barra_status(
    ui: &mut Ui,
    rotulo: Option<&str>,
    valor: f32,
    maximo: f32,
    cor: Color32,
    largura: f32,
    texto_direita: Option<&str>,
) {
    use crate::theme::paleta::menu;
    let frac = (valor / maximo).clamp(0.0, 1.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if let Some(r) = rotulo {
            ui.add_sized(
                [92.0, 14.0],
                egui::Label::new(egui::RichText::new(r.to_uppercase()).small().color(menu::TINTA_SUAVE)),
            );
        }
        let (rect, _) = ui.allocate_exact_size(Vec2::new(largura, 12.0), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, menu::PAPEL_CLARO);
        let cheio = Rect::from_min_max(rect.min, egui::pos2(rect.min.x + rect.width() * frac, rect.max.y));
        painter.rect_filled(cheio, 0.0, cor);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, menu::TINTA), bevy_egui::egui::StrokeKind::Inside);
        // Marcação de 50% (referência visual de cartaz)
        let x50 = rect.min.x + rect.width() * 0.5;
        painter.line_segment(
            [egui::pos2(x50, rect.max.y - 3.0), egui::pos2(x50, rect.max.y)],
            Stroke::new(1.0, menu::TINTA_SUAVE),
        );
        if let Some(t) = texto_direita {
            ui.label(egui::RichText::new(t).size(12.0).strong().color(menu::TINTA));
        }
    });
}

/// Trilho fino para células de tabela: calha creme com enchimento sólido e moldura de tinta.
pub fn barra_trilho(ui: &mut Ui, frac: f32, cor: Color32, largura: f32, altura: f32) {
    use crate::theme::paleta::menu;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(largura, altura), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, menu::PAPEL_CLARO);
    let cheio = Rect::from_min_max(
        rect.min,
        egui::pos2(rect.min.x + rect.width() * frac.clamp(0.0, 1.0), rect.max.y),
    );
    p.rect_filled(cheio, 0.0, cor);
    p.rect_stroke(rect, 0.0, Stroke::new(1.0, menu::TINTA_SUAVE), bevy_egui::egui::StrokeKind::Inside);
}

/// Barra de relação diplomática: eixo central (neutro), preenchimento para a direita
/// (amizade) ou para a esquerda (hostilidade), com cores do cartaz.
pub fn barra_relacao(ui: &mut Ui, valor: f32, largura: f32) {
    use crate::theme::paleta::menu;
    let v = valor.clamp(-1.0, 1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(largura, 12.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, menu::PAPEL_CLARO);
    let meio = rect.center().x;
    let cor = if v >= 0.0 { menu::AZUL_MARINHO } else { menu::VERMELHO };
    // valores pequenos ainda precisam mostrar uma lasca visível da barra
    let m = if v == 0.0 { 0.0 } else { v.signum() * v.abs().max(0.06) };
    let cheio = if m >= 0.0 {
        Rect::from_min_max(egui::pos2(meio, rect.min.y), egui::pos2(meio + rect.width() * 0.5 * m, rect.max.y))
    } else {
        Rect::from_min_max(egui::pos2(meio + rect.width() * 0.5 * m, rect.min.y), egui::pos2(meio, rect.max.y))
    };
    painter.rect_filled(cheio, 0.0, cor);
    painter.line_segment(
        [egui::pos2(meio, rect.min.y), egui::pos2(meio, rect.max.y)],
        Stroke::new(1.5, menu::TINTA),
    );
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, menu::TINTA), bevy_egui::egui::StrokeKind::Inside);
}
