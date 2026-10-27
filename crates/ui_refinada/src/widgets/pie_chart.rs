//! Widget de Gráfico de Pizza (Pie Chart) personalizado para egui.
//! Usado para exibir a distribuição demográfica de Pops (Profissão, Cultura, Religião).

use bevy_egui::egui::{
    epaint::Shape, Color32, Pos2, Response, Sense, Stroke, Ui, Vec2,
};
use std::f32::consts::PI;

use crate::view_model::estado::PopFatiaViewModel;

/// Desenha um gráfico de pizza com fatias proporcionais e legenda interativa.
pub fn render_pie_chart(
    ui: &mut Ui,
    fatias: &[PopFatiaViewModel],
    raio: f32,
    titulo: &str,
) -> Response {
    ui.vertical(|ui| {
        ui.heading(titulo);
        ui.add_space(4.0);

        let diametro = raio * 2.0;
        let padding = 10.0;
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(diametro + padding * 2.0, diametro + padding * 2.0),
            Sense::hover(),
        );

        let painter = ui.painter_at(rect);
        let centro = rect.center();

        let mut angulo_atual = -PI / 2.0; // Inicia no topo (12 horas)
        let total_percentual: f32 = fatias.iter().map(|f| f.percentual).sum();
        let fator_normalizacao = if total_percentual > 0.0 {
            100.0 / total_percentual
        } else {
            1.0
        };

        // Verifica posição do mouse para realce de fatia (hover)
        let pos_mouse = ui.input(|i| i.pointer.hover_pos());

        for fatia in fatias {
            let pct_normalizado = fatia.percentual * fator_normalizacao;
            let angulo_fatia = (pct_normalizado / 100.0) * (2.0 * PI);
            let proximo_angulo = angulo_atual + angulo_fatia;

            // Determina se o mouse está sobre esta fatia angular
            let mut fatia_sob_cursor = false;
            if let Some(mouse) = pos_mouse {
                let dx = mouse.x - centro.x;
                let dy = mouse.y - centro.y;
                let distancia = (dx * dx + dy * dy).sqrt();
                if distancia <= raio + 4.0 {
                    let mut ang_mouse = dy.atan2(dx);
                    // Normaliza para o mesmo ciclo
                    while ang_mouse < angulo_atual {
                        ang_mouse += 2.0 * PI;
                    }
                    if ang_mouse >= angulo_atual && ang_mouse <= proximo_angulo {
                        fatia_sob_cursor = true;
                    }
                }
            }

            let r_efetivo = if fatia_sob_cursor {
                raio + 4.0
            } else {
                raio
            };

            // Gera polígono para a fatia
            let passos = ((angulo_fatia / (2.0 * PI)) * 36.0).max(6.0) as usize;
            let mut pontos = Vec::with_capacity(passos + 2);
            pontos.push(centro);

            for i in 0..=passos {
                let frac = i as f32 / passos as f32;
                let a = angulo_atual + frac * angulo_fatia;
                let x = centro.x + r_efetivo * a.cos();
                let y = centro.y + r_efetivo * a.sin();
                pontos.push(Pos2::new(x, y));
            }

            let cor_preenchimento = fatia.cor32();
            let borda = if fatia_sob_cursor {
                Stroke::new(1.5, Color32::WHITE)
            } else {
                Stroke::new(1.0, Color32::from_rgb(24, 27, 29))
            };

            painter.add(Shape::convex_polygon(pontos, cor_preenchimento, borda));

            angulo_atual = proximo_angulo;
        }

        // Legenda detalhada abaixo do gráfico
        ui.add_space(8.0);
        for fatia in fatias {
            ui.horizontal(|ui| {
                // Quadrado colorido indicador
                let (quad_rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), Sense::hover());
                ui.painter().rect_filled(quad_rect, 1.0, fatia.cor32());
                ui.painter().rect_stroke(
                    quad_rect,
                    1.0,
                    Stroke::new(1.0, Color32::from_rgb(65, 74, 82)),
                    bevy_egui::egui::StrokeKind::Inside,
                );

                let label_text = format!(
                    "{}: {:.1}% ({})",
                    fatia.rotulo,
                    fatia.percentual,
                    formatar_numero_habitantes(fatia.quantidade)
                );

                let label_response = ui.label(label_text);
                if !fatia.detalhe_estrato_ou_origem.is_empty() {
                    label_response.on_hover_text(&fatia.detalhe_estrato_ou_origem);
                }
            });
        }

        response
    })
    .inner
}

fn formatar_numero_habitantes(qtd: u64) -> String {
    if qtd >= 1_000_000 {
        format!("{:.2}M", qtd as f64 / 1_000_000.0)
    } else if qtd >= 1_000 {
        format!("{:.1}k", qtd as f64 / 1_000.0)
    } else {
        qtd.to_string()
    }
}
