//! Polimento visual: tema egui de gabinete de guerra, vinheta/moldura, ondas do oceano e zoom suave.
//! Tudo configurável em `visual_mapa::EstiloMapa`.
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use visual_mapa::EstiloMapa;

pub struct PolimentoVisualPlugin;

impl Plugin for PolimentoVisualPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(EguiPrimaryContextPass, (aplicar_tema, moldura_vinheta).chain());
    }
}

/// Aplica fontes e estilo uma única vez (e de novo se `tema_ui` for religado).
fn aplicar_tema(mut contextos: EguiContexts, estilo: Res<EstiloMapa>, mut aplicado: Local<bool>) -> Result {
    if !estilo.tema_ui || *aplicado {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?;
    ui_refinada::setup_cold_war_fonts(ctx);
    let mut style = ui_refinada::cold_war_style();
    // Títulos de seção em Oswald (uma vez, junto com o tema).
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(19.0, ui_refinada::familia_oswald(ctx)),
    );
    ctx.set_style_of(egui::Theme::Light, style.clone());
    ctx.set_style_of(egui::Theme::Dark, style.clone());
    ctx.set_theme(egui::ThemePreference::Light);
    ctx.set_visuals(ui_refinada::cold_war_visuals());
    ctx.set_global_style(style);
    *aplicado = true;
    Ok(())
}

/// Vinheta escura nas bordas e moldura fina de mapa antigo, desenhadas acima do mapa.
fn moldura_vinheta(mut contextos: EguiContexts, estilo: Res<EstiloMapa>) -> Result {
    if estilo.vinheta_alfa <= 0.0 && !estilo.moldura_antiga {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?;
    let r = ctx.viewport_rect();
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("vinheta")));
    if estilo.vinheta_alfa > 0.0 {
        let a = (estilo.vinheta_alfa.clamp(0.0, 1.0) * 255.0) as u8;
        let escuro = egui::Color32::from_rgba_unmultiplied(8, 10, 12, a);
        let claro = egui::Color32::TRANSPARENT;
        let t = (r.width().min(r.height()) * 0.18).max(40.0);
        let faixa = |rect: egui::Rect, c: [egui::Color32; 4]| {
            let mut m = egui::Mesh::default();
            m.colored_vertex(rect.left_top(), c[0]);
            m.colored_vertex(rect.right_top(), c[1]);
            m.colored_vertex(rect.right_bottom(), c[2]);
            m.colored_vertex(rect.left_bottom(), c[3]);
            m.add_triangle(0, 1, 2);
            m.add_triangle(0, 2, 3);
            p.add(egui::Shape::mesh(m));
        };
        faixa(egui::Rect::from_min_size(r.min, egui::vec2(r.width(), t)), [escuro, escuro, claro, claro]);
        faixa(egui::Rect::from_min_max(egui::pos2(r.min.x, r.max.y - t), r.max), [claro, claro, escuro, escuro]);
        faixa(egui::Rect::from_min_size(r.min, egui::vec2(t, r.height())), [escuro, claro, claro, escuro]);
        faixa(egui::Rect::from_min_max(egui::pos2(r.max.x - t, r.min.y), r.max), [claro, escuro, escuro, claro]);
    }
    if estilo.moldura_antiga {
        let ouro = egui::Color32::from_rgba_unmultiplied(158, 145, 118, 150);
        p.rect_stroke(r.shrink(6.0), 0.0, egui::Stroke::new(2.0, ouro), egui::StrokeKind::Inside);
        p.rect_stroke(r.shrink(11.0), 0.0, egui::Stroke::new(1.0, ouro), egui::StrokeKind::Inside);
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use ui_refinada::menu;
    use ui_refinada::paleta::razao_contraste;

    #[test]
    fn tema_contraste_minimo_papel_e_tinta() {
        let papel = menu::PAPEL;
        let tinta = menu::TINTA;
        let c = razao_contraste(tinta, papel);
        assert!(c >= 7.0, "Tinta sobre papel deve ter contraste AAA >= 7.0 (deu {c:.2})");

        let c_vermelho = razao_contraste(menu::VERMELHO, papel);
        assert!(c_vermelho >= 4.5, "Vermelho de destaque deve ter contraste >= 4.5 (deu {c_vermelho:.2})");

        let c_sucesso = razao_contraste(menu::SUCESSO, papel);
        assert!(c_sucesso >= 4.0, "Sucesso sobre papel deve ter contraste >= 4.0 (deu {c_sucesso:.2})");

        let c_perigo = razao_contraste(menu::PERIGO, papel);
        assert!(c_perigo >= 4.5, "Perigo sobre papel deve ter contraste >= 4.5 (deu {c_perigo:.2})");
    }

    #[test]
    fn estilo_visual_contem_fontes_e_molduras() {
        let visuals = ui_refinada::cold_war_visuals();
        assert_eq!(visuals.panel_fill, menu::PAPEL);
        assert_eq!(visuals.window_fill, menu::PAPEL);
        assert_eq!(visuals.override_text_color, Some(menu::TINTA));

        let moldura = ui_refinada::moldura_janela_papel();
        assert_eq!(moldura.fill, menu::PAPEL);
        assert_eq!(moldura.stroke.color, menu::VERMELHO);
    }
}
