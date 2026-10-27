//! Estilo visual de gabinete de guerra (1945) para egui.
//! Baseado nas especificações de `design/1_identidade/ESTILO.md`.

use bevy_egui::egui::{
    style::{Spacing, WidgetVisuals, Widgets},
    Margin,
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Style, Vec2, Visuals,
};

use super::paleta::{blocos, dossie_papel, superficies, texto};

/// Configura o tema de Gabinete de Guerra 1945 para o `egui::Visuals`.
pub fn cold_war_visuals() -> Visuals {
    let mut visuals = Visuals::dark();

    // 1. Cores de Superfície e Fundo
    visuals.panel_fill = superficies::BG_BASE;
    visuals.window_fill = superficies::BG_BASE;
    visuals.extreme_bg_color = superficies::BG_SURFACE_SUBTLE;
    visuals.faint_bg_color = superficies::BG_SURFACE;
    visuals.code_bg_color = superficies::BG_VOID;

    // 2. Geometria Seca e Reta (Estética 1945)
    visuals.window_corner_radius = CornerRadius::ZERO;
    visuals.menu_corner_radius = CornerRadius::ZERO;
    visuals.window_stroke = Stroke::new(1.0, superficies::BORDER_MEDIUM);

    // 3. Estilo de Widgets (Botões, Checkboxes, Sliders)
    visuals.widgets = Widgets {
        noninteractive: WidgetVisuals {
            bg_fill: superficies::BG_SURFACE,
            weak_bg_fill: superficies::BG_SURFACE,
            bg_stroke: Stroke::new(1.0, superficies::BORDER_SUBTLE),
            corner_radius: CornerRadius::ZERO,
            fg_stroke: Stroke::new(1.0, texto::SECUNDARIO),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            bg_fill: superficies::BG_SURFACE,
            weak_bg_fill: superficies::BG_BASE,
            bg_stroke: Stroke::new(1.0, superficies::BORDER_MEDIUM),
            corner_radius: CornerRadius::same(1),
            fg_stroke: Stroke::new(1.0, texto::PRIMARIO),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            bg_fill: superficies::BG_SURFACE_ELEVATED,
            weak_bg_fill: superficies::BG_SURFACE_ELEVATED,
            bg_stroke: Stroke::new(1.0, superficies::BORDER_STRONG),
            corner_radius: CornerRadius::same(1),
            fg_stroke: Stroke::new(1.0, Color32::WHITE),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            bg_fill: superficies::BG_SURFACE_SUBTLE,
            weak_bg_fill: superficies::BG_SURFACE_SUBTLE,
            bg_stroke: Stroke::new(1.5, texto::OURO_DESTAQUE),
            corner_radius: CornerRadius::ZERO,
            fg_stroke: Stroke::new(1.5, texto::OURO_DESTAQUE),
            expansion: -0.5,
        },
        open: WidgetVisuals {
            bg_fill: superficies::BG_SURFACE_ELEVATED,
            weak_bg_fill: superficies::BG_SURFACE_ELEVATED,
            bg_stroke: Stroke::new(1.0, superficies::BORDER_STRONG),
            corner_radius: CornerRadius::ZERO,
            fg_stroke: Stroke::new(1.0, texto::PRIMARIO),
            expansion: 0.0,
        },
    };

    // 4. Seleção e Destaques
    visuals.selection.bg_fill = blocos::OCIDENTAL_PRIMARIA;
    visuals.selection.stroke = Stroke::new(1.0, blocos::OCIDENTAL_REALCE);

    visuals
}

/// Configuração de espaçamento industrial rígido para egui.
#[allow(clippy::field_reassign_with_default)]
pub fn cold_war_style() -> Style {
    let mut style = Style::default();
    style.visuals = cold_war_visuals();

    style.spacing = Spacing {
        item_spacing: Vec2::new(8.0, 6.0),
        window_margin: Margin::same(10),
        button_padding: Vec2::new(10.0, 5.0),
        indent: 14.0,
        interact_size: Vec2::new(40.0, 24.0),
        slider_width: 140.0,
        combo_width: 120.0,
        text_edit_width: 200.0,
        icon_width: 16.0,
        icon_spacing: 6.0,
        tooltip_width: 360.0,
        ..Default::default()
    };

    style
}

/// Carrega e registra as fontes de época de 1945 no `egui::Context`.
pub fn setup_cold_war_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Fontes embutidas diretamente do pacote local
    fonts.font_data.insert(
        "Barlow".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/Barlow-Regular.ttf"))),
    );
    fonts.font_data.insert(
        "Barlow-SemiBold".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/Barlow-SemiBold.ttf"))),
    );
    fonts.font_data.insert(
        "CourierPrime".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/CourierPrime-Regular.ttf"))),
    );
    fonts.font_data.insert(
        "CourierPrime-Bold".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/CourierPrime-Bold.ttf"))),
    );
    fonts.font_data.insert(
        "Oswald".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/Oswald-Bold.ttf"))),
    );
    fonts.font_data.insert(
        "SpecialElite".to_owned(),
        std::sync::Arc::new(FontData::from_static(include_bytes!("../../assets/fonts/SpecialElite-Regular.ttf"))),
    );

    // Associação de famílias proporcionais e monoespaçadas
    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(0, "Barlow".to_owned());
    }
    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.insert(0, "CourierPrime".to_owned());
    }

    ctx.set_fonts(fonts);
}

/// Moldura estilo gabinete de guerra para painéis e janelas
pub fn moldura_painel() -> egui::Frame {
    egui::Frame {
        inner_margin: Margin::same(12),
        fill: superficies::BG_BASE,
        stroke: Stroke::new(1.0, superficies::BORDER_MEDIUM),
        corner_radius: CornerRadius::ZERO,
        ..Default::default()
    }
}

/// Moldura estilo cartão de ministério / setor
pub fn moldura_cartao() -> egui::Frame {
    egui::Frame {
        inner_margin: Margin::same(8),
        fill: superficies::BG_SURFACE,
        stroke: Stroke::new(1.0, superficies::BORDER_SUBTLE),
        corner_radius: CornerRadius::same(1),
        ..Default::default()
    }
}

/// Moldura estilo documento em papel de gabinete envelhecido
pub fn moldura_papel_dossie() -> egui::Frame {
    egui::Frame {
        inner_margin: Margin::same(14),
        fill: dossie_papel::PAPEL_BASE,
        stroke: Stroke::new(1.5, dossie_papel::PAPEL_BORDA),
        corner_radius: CornerRadius::ZERO,
        ..Default::default()
    }
}
