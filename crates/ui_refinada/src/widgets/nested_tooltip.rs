//! Tooltips detalhados e aninhados no estilo analítico de Victoria 3.
//! Decomposição transparente de fórmulas, receitas, despesas, legitimidade e forças sociais.

use bevy_egui::egui::{Color32, RichText, Ui};

pub struct ItemDecomposicao {
    pub descricao: String,
    pub valor: f64,
    pub eh_positivo: bool,
    pub detalhe_extra: Option<String>,
}

/// Renderiza o conteúdo interno de um tooltip de decomposição no estilo Victoria 3.
pub fn render_decomposicao_tooltip(
    ui: &mut Ui,
    titulo: &str,
    valor_total: &str,
    subtitulo: Option<&str>,
    itens: &[ItemDecomposicao],
    dica_estrategica: Option<&str>,
) {
    ui.set_max_width(340.0);

    // Cabeçalho Principal
    ui.heading(titulo);
    ui.horizontal(|ui| {
        ui.label("Valor Atual:");
        ui.label(RichText::new(valor_total).strong().color(Color32::from_rgb(223, 177, 91)));
    });

    if let Some(sub) = subtitulo {
        ui.label(RichText::new(sub).italics().color(Color32::from_rgb(155, 167, 176)));
    }

    ui.separator();
    ui.label(RichText::new("Decomposição do Cálculo:").strong());

    for item in itens {
        ui.horizontal(|ui| {
            let sinal = if item.eh_positivo { "+" } else { "-" };
            let cor = if item.eh_positivo {
                Color32::from_rgb(76, 138, 77)
            } else {
                Color32::from_rgb(184, 50, 40)
            };

            ui.label(RichText::new(format!("• {}:", item.descricao)).color(Color32::from_rgb(230, 236, 239)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("{}{:.1}", sinal, item.valor.abs())).strong().color(cor));
            });
        });

        if let Some(ref extra) = item.detalhe_extra {
            ui.indent("sub_item", |ui| {
                ui.label(RichText::new(extra).small().color(Color32::from_rgb(155, 167, 176)));
            });
        }
    }

    if let Some(dica) = dica_estrategica {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("★ Dica de Gabinete:").strong().color(Color32::from_rgb(56, 115, 158)));
        });
        ui.label(RichText::new(dica).small().color(Color32::from_rgb(200, 190, 175)));
    }
}
