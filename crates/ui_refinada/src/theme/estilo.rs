//! Estilo visual de gabinete de guerra e menu (1945) para egui.
//! Paleta em papel creme envelhecido, borda vermelha dupla, cartões e fontes de época.

use bevy_egui::egui::{
    style::{Spacing, WidgetVisuals, Widgets},
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, Style,
    TextStyle, Vec2, Visuals,
};

use super::paleta::menu;

/// Configura o tema unificado de Cartaz/Gabinete 1945 para o `egui::Visuals`:
/// papel envelhecido creme (#E8DEC8), borda vermelha (#8C251E), tinta carvão (#171614)
/// e widgets em cartões com alto contraste e linhas de tabela alternadas suaves.
pub fn cold_war_visuals() -> Visuals {
    let mut visuals = Visuals::light();

    // 1. Cores de Superfície e Fundo (Papel creme envelhecido)
    visuals.panel_fill = menu::PAPEL;
    visuals.window_fill = menu::PAPEL;
    visuals.extreme_bg_color = menu::PAPEL_CLARO;
    visuals.faint_bg_color = menu::PAPEL_ALT; // Usado por egui::Grid::striped(true) e tabelas
    visuals.code_bg_color = menu::PAPEL_CARD;

    // 2. Geometria Seca e Reta com Borda Vermelha (Estética 1945)
    visuals.window_corner_radius = CornerRadius::ZERO;
    visuals.menu_corner_radius = CornerRadius::ZERO;
    visuals.window_stroke = Stroke::new(2.0, menu::VERMELHO);
    visuals.window_shadow = egui::Shadow {
        offset: [4, 6],
        blur: 14,
        spread: 0,
        color: Color32::from_black_alpha(85),
    };

    // 3. Tipografia: tinta escura legível sobre creme
    visuals.override_text_color = Some(menu::TINTA);

    // 4. Estilo de Widgets (Botões, Checkboxes, Sliders, Caixas)
    visuals.widgets = Widgets {
        noninteractive: WidgetVisuals {
            bg_fill: menu::PAPEL_CARD,
            weak_bg_fill: menu::PAPEL,
            bg_stroke: Stroke::new(1.0, menu::PAPEL_BORDA),
            corner_radius: CornerRadius::ZERO,
            fg_stroke: Stroke::new(1.0, menu::TINTA),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            bg_fill: menu::PAPEL_CARD,
            weak_bg_fill: menu::PAPEL_CARD,
            bg_stroke: Stroke::new(1.2, menu::VERMELHO),
            corner_radius: CornerRadius::ZERO,
            fg_stroke: Stroke::new(1.0, menu::TINTA),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            bg_fill: menu::VERMELHO_HOVER,
            weak_bg_fill: menu::VERMELHO_HOVER,
            bg_stroke: Stroke::new(1.5, menu::TINTA),
            corner_radius: CornerRadius::ZERO,
            fg_stroke: Stroke::new(1.5, menu::PAPEL),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            bg_fill: menu::VERMELHO_ATIVO,
            weak_bg_fill: menu::VERMELHO_ATIVO,
            bg_stroke: Stroke::new(2.0, menu::TINTA),
            corner_radius: CornerRadius::ZERO,
            fg_stroke: Stroke::new(1.5, menu::PAPEL),
            expansion: -0.5,
        },
        open: WidgetVisuals {
            bg_fill: menu::PAPEL_ALT,
            weak_bg_fill: menu::PAPEL_ALT,
            bg_stroke: Stroke::new(1.5, menu::VERMELHO),
            corner_radius: CornerRadius::ZERO,
            fg_stroke: Stroke::new(1.0, menu::TINTA),
            expansion: 0.0,
        },
    };

    // 5. Seleção e Destaques
    visuals.selection.bg_fill = menu::VERMELHO;
    visuals.selection.stroke = Stroke::new(1.0, menu::TINTA);
    visuals.slider_trailing_fill = true;

    // 6. Tooltips e Janelas Flutuantes
    visuals.popup_shadow = egui::Shadow {
        offset: [2, 4],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(75),
    };

    visuals
}

/// Configuração de estilo e espaçamento para egui.
#[allow(clippy::field_reassign_with_default)]
pub fn cold_war_style() -> Style {
    let mut style = Style::default();
    style.visuals = cold_war_visuals();

    // Fonte para títulos principais (segura contra contextos sem Oswald carregado)
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(20.0, FontFamily::Proportional),
    );

    style.spacing = Spacing {
        item_spacing: Vec2::new(8.0, 6.0),
        window_margin: Margin::same(14),
        button_padding: Vec2::new(10.0, 5.0),
        indent: 14.0,
        interact_size: Vec2::new(40.0, 24.0),
        slider_width: 140.0,
        combo_width: 120.0,
        text_edit_width: 200.0,
        icon_width: 16.0,
        icon_spacing: 6.0,
        tooltip_width: 380.0,
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
    fonts.families.insert(FontFamily::Name("Oswald".into()), vec!["Oswald".to_owned()]);
    fonts.families.insert(FontFamily::Name("SpecialElite".into()), vec!["SpecialElite".to_owned()]);

    ctx.set_fonts(fonts);
}

/// Retorna a família de fontes para títulos/botões: Oswald se disponível no contexto, senão Proportional.
pub fn familia_oswald(ctx: &egui::Context) -> FontFamily {
    let oswald = FontFamily::Name("Oswald".into());
    let disponivel = ctx.fonts(|f| f.definitions().families.contains_key(&oswald));
    if disponivel {
        oswald
    } else {
        FontFamily::Proportional
    }
}

/// Moldura estilo gabinete de guerra para painéis e janelas (papel creme com borda vermelha)
pub fn moldura_painel() -> egui::Frame {
    egui::Frame {
        inner_margin: Margin::same(14),
        fill: menu::PAPEL,
        stroke: Stroke::new(2.0, menu::VERMELHO),
        corner_radius: CornerRadius::ZERO,
        shadow: egui::Shadow {
            offset: [4, 6],
            blur: 14,
            spread: 0,
            color: Color32::from_black_alpha(85),
        },
        ..Default::default()
    }
}

/// Moldura padrão para janelas no estilo do menu: papel envelhecido, borda vermelha 2px e cantos secos.
pub fn moldura_janela_papel() -> egui::Frame {
    moldura_painel()
}

/// Moldura estilo cartão de dossiê / setor secundário
pub fn moldura_cartao() -> egui::Frame {
    egui::Frame {
        inner_margin: Margin::same(10),
        fill: menu::PAPEL_CARD,
        stroke: Stroke::new(1.0, menu::PAPEL_BORDA),
        corner_radius: CornerRadius::ZERO,
        ..Default::default()
    }
}

/// Moldura estilo cartão em papel
pub fn moldura_cartao_papel() -> egui::Frame {
    moldura_cartao()
}

/// Moldura estilo documento em papel de gabinete envelhecido
pub fn moldura_papel_dossie() -> egui::Frame {
    egui::Frame {
        inner_margin: Margin::same(14),
        fill: menu::PAPEL,
        stroke: Stroke::new(1.5, menu::VERMELHO),
        corner_radius: CornerRadius::ZERO,
        ..Default::default()
    }
}

/// Desenha a borda interna e os cantos decorativos estilo dossiê 1945 sobre o retângulo fornecido.
pub fn desenhar_moldura_dupla_com_cantos(painter: &egui::Painter, rect: egui::Rect) {
    let vermelha = menu::VERMELHO;
    // Borda interna recuada 3.5 px da borda da janela
    let interna = rect.shrink(3.5);
    painter.rect_stroke(interna, 0.0, Stroke::new(1.0, vermelha), egui::StrokeKind::Inside);

    // Cantos decorativos (linhas reforçadas em L nos 4 vértices)
    let tam = 9.0;
    let s = Stroke::new(2.0, vermelha);

    // Superior Esquerdo
    let tl = interna.left_top();
    painter.line_segment([tl, tl + egui::vec2(tam, 0.0)], s);
    painter.line_segment([tl, tl + egui::vec2(0.0, tam)], s);

    // Superior Direito
    let tr = interna.right_top();
    painter.line_segment([tr, tr - egui::vec2(tam, 0.0)], s);
    painter.line_segment([tr, tr + egui::vec2(0.0, tam)], s);

    // Inferior Esquerdo
    let bl = interna.left_bottom();
    painter.line_segment([bl, bl + egui::vec2(tam, 0.0)], s);
    painter.line_segment([bl, bl - egui::vec2(0.0, tam)], s);

    // Inferior Direito
    let br = interna.right_bottom();
    painter.line_segment([br, br - egui::vec2(tam, 0.0)], s);
    painter.line_segment([br, br - egui::vec2(0.0, tam)], s);
}

/// Decora a janela atual com borda dupla vermelha e cantos de cartaz.
pub fn decorar_janela(ui: &egui::Ui) {
    desenhar_moldura_dupla_com_cantos(ui.painter(), ui.max_rect());
}

/// Cria uma janela egui pré-configurada no estilo visual do menu.
pub fn janela_estilo_menu<'a>(titulo: impl Into<egui::WidgetText>) -> egui::Window<'a> {
    egui::Window::new(titulo)
        .frame(moldura_janela_papel())
        .collapsible(false)
}

/// Desenha linha divisória decorativa dupla (vermelha) no estilo do cartaz da Guerra Fria.
pub fn linha_decorativa(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), egui::Sense::hover());
    let p = ui.painter();
    let y1 = rect.top() + 1.0;
    let y2 = rect.top() + 4.0;
    let cor = menu::VERMELHO;
    p.line_segment([egui::pos2(rect.left(), y1), egui::pos2(rect.right(), y1)], Stroke::new(1.5, cor));
    p.line_segment([egui::pos2(rect.left(), y2), egui::pos2(rect.right(), y2)], Stroke::new(0.8, cor));
    ui.add_space(4.0);
}

/// Cabeçalho estilizado para painéis: título com fonte Oswald em CAIXA ALTA e linha decorativa dupla vermelha.
pub fn cabecalho_painel(ui: &mut egui::Ui, titulo: &str) {
    let fam = familia_oswald(ui.ctx());
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(titulo.to_uppercase())
                .family(fam)
                .size(22.0)
                .strong()
                .color(menu::TINTA),
        );
        linha_decorativa(ui);
    });
}

/// Botão no estilo do menu: fundo vermelho com texto creme na fonte Oswald.
pub fn botao_estilo_menu(ui: &mut egui::Ui, texto: &str) -> egui::Response {
    let fam = familia_oswald(ui.ctx());
    ui.add(
        egui::Button::new(
            egui::RichText::new(texto)
                .family(fam)
                .size(16.0)
                .color(menu::PAPEL),
        )
        .fill(menu::VERMELHO)
        .stroke(Stroke::new(1.0, menu::TINTA))
        .min_size(Vec2::new(80.0, 30.0)),
    )
}

/// Botão secundário no estilo do menu: cartão creme com borda vermelha e texto escuro.
pub fn botao_estilo_secundario(ui: &mut egui::Ui, texto: &str) -> egui::Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(texto)
                .size(15.0)
                .color(menu::TINTA),
        )
        .fill(menu::PAPEL_CARD)
        .stroke(Stroke::new(1.0, menu::VERMELHO))
        .min_size(Vec2::new(70.0, 28.0)),
    )
}

/// Aba estilo dossiê: botão selecionável com realce vermelho quando ativo.
pub fn aba_estilo_menu(ui: &mut egui::Ui, ativa: bool, texto: &str) -> bool {
    let fam = familia_oswald(ui.ctx());
    let (fill, text_color, stroke) = if ativa {
        (
            menu::VERMELHO,
            menu::PAPEL,
            Stroke::new(1.5, menu::TINTA),
        )
    } else {
        (
            menu::PAPEL_CARD,
            menu::TINTA,
            Stroke::new(1.0, menu::PAPEL_BORDA),
        )
    };
    ui.add(
        egui::Button::new(
            egui::RichText::new(texto)
                .family(fam)
                .size(15.0)
                .color(text_color),
        )
        .fill(fill)
        .stroke(stroke)
        .min_size(Vec2::new(90.0, 28.0)),
    )
    .clicked()
}

/// Pinta o fundo de uma linha de tabela de forma suave e alternada (odd/even).
pub fn tabela_linha_alt(painter: &egui::Painter, indice: usize, rect: egui::Rect) {
    if indice % 2 == 1 {
        painter.rect_filled(rect, 0.0, menu::PAPEL_ALT);
    }
}

/// Garante que o Ui use as cores escuras sobre papel mesmo em áreas isoladas.
pub fn estilo_papel_ui(ui: &mut egui::Ui) {
    let v = ui.visuals_mut();
    v.override_text_color = Some(menu::TINTA);
    v.selection.bg_fill = menu::VERMELHO;
    v.slider_trailing_fill = true;
}

/// Ornamento ◆ (losango) desenhado pelo painter, usado nas faixas de título.
fn losango(ui: &mut egui::Ui, cor: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(9.0), egui::Sense::hover());
    let c = rect.center();
    let d = rect.width() * 0.46;
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            c + egui::vec2(0.0, -d),
            c + egui::vec2(d, 0.0),
            c + egui::vec2(0.0, d),
            c + egui::vec2(-d, 0.0),
        ],
        cor,
        Stroke::NONE,
    ));
}

/// Faixa de título estilo cartaz: barra azul-marinho com rótulo Oswald creme,
/// losangos vermelhos de ornamento, selo opcional de atalho e botão de fechar.
/// `aberto: Some(&mut bool)` mostra o "X" que fecha o painel.
pub fn faixa_titulo(ui: &mut egui::Ui, titulo: &str, atalho: Option<&str>, aberto: Option<&mut bool>) {
    let fam = familia_oswald(ui.ctx());
    egui::Frame::new()
        .fill(menu::AZUL_MARINHO)
        .inner_margin(Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 7.0;
                if let Some(t) = atalho {
                    ui.label(
                        egui::RichText::new(t)
                            .family(fam.clone())
                            .size(12.0)
                            .color(menu::AZUL_MARINHO)
                            .background_color(menu::PAPEL)
                            .strong(),
                    );
                }
                losango(ui, menu::VERMELHO_HOVER);
                ui.label(
                    egui::RichText::new(titulo.to_uppercase())
                        .family(fam)
                        .size(17.0)
                        .strong()
                        .color(menu::PAPEL_CLARO),
                );
                losango(ui, menu::VERMELHO_HOVER);
                if let Some(flag) = aberto {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("X")
                                        .family(FontFamily::Name("Oswald".into()))
                                        .size(13.0)
                                        .color(menu::PAPEL),
                                )
                                .fill(menu::VERMELHO)
                                .stroke(Stroke::new(1.0, menu::PAPEL))
                                .min_size(Vec2::new(22.0, 20.0)),
                            )
                            .clicked()
                        {
                            *flag = false;
                        }
                    });
                }
            });
        });
}

/// Título de seção dentro de um painel: Oswald azul-marinho sobre filete de cartaz.
pub fn titulo_secao(ui: &mut egui::Ui, texto: &str) {
    ui.label(
        egui::RichText::new(texto.to_uppercase())
            .family(familia_oswald(ui.ctx()))
            .size(17.0)
            .strong()
            .color(menu::AZUL_MARINHO),
    );
    linha_decorativa(ui);
}

/// Janela de dossiê: moldura de papel, sem barra de título do egui (use [`faixa_titulo`] dentro dela).
/// Posicionada abaixo da barra superior para não cobrir recursos/data.
pub fn janela_dossie<'a>(titulo_interno: &'static str, aberto: Option<&'a mut bool>) -> egui::Window<'a> {
    let mut win = egui::Window::new(titulo_interno)
        .title_bar(false)
        .frame(moldura_janela_papel())
        .collapsible(false)
        .resizable(true)
        .default_pos([24.0, 104.0]);
    if let Some(f) = aberto {
        win = win.open(f);
    }
    win
}

/// Aba com glifo vetorial desenhado pelo painter no lugar de emoji.
/// `chave_glifo = None` renderiza apenas o texto.
pub fn aba_glifo(ui: &mut egui::Ui, ativa: bool, chave_glifo: Option<&str>, texto: &str) -> bool {
    let fam = familia_oswald(ui.ctx());
    let (fill, text_color, stroke) = if ativa {
        (menu::VERMELHO, menu::PAPEL, Stroke::new(1.5, menu::TINTA))
    } else {
        (menu::PAPEL_CARD, menu::TINTA, Stroke::new(1.0, menu::PAPEL_BORDA))
    };
    let (g, s) = (menu::TINTA, menu::VERMELHO);
    let (pa, pac) = (menu::PAPEL, menu::PAPEL_CLARO);
    let resp = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .inner_margin(Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (rect, _) = ui.allocate_at_least(Vec2::splat(15.0), egui::Sense::hover());
                if let Some(k) = chave_glifo {
                    let cores = if ativa { (pa, pac) } else { (g, s) };
                    super::icone::desenhar(ui.painter(), rect, k, cores.0, cores.1);
                }
                if !texto.is_empty() {
                    ui.label(
                        egui::RichText::new(texto.to_uppercase())
                            .family(fam.clone())
                            .size(13.0)
                            .color(text_color),
                    );
                }
            });
            ui.min_rect()
        })
        .response
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let clicado = resp.clicked();
    if ativa {
        ui.painter().line_segment(
            [resp.rect.left_bottom(), resp.rect.right_bottom()],
            Stroke::new(2.0, menu::OURO),
        );
    }
    clicado
}

/// Chapa de recurso: cartão de papel com glifo, rótulo e valor alinhados.
pub fn chapa(ui: &mut egui::Ui, chave_glifo: &str, rotulo: &str, valor: &str, cor_valor: Option<Color32>) {
    let fam = familia_oswald(ui.ctx());
    egui::Frame::new()
        .fill(menu::PAPEL_CARD)
        .stroke(Stroke::new(1.0, menu::PAPEL_BORDA))
        .inner_margin(Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), egui::Sense::hover());
                super::icone::desenhar(ui.painter(), rect, chave_glifo, menu::TINTA, menu::VERMELHO);
                ui.label(egui::RichText::new(rotulo).small().color(menu::TINTA_SUAVE));
                ui.label(egui::RichText::new(valor).family(fam).size(15.0).strong().color(cor_valor.unwrap_or(menu::TINTA)));
            });
        });
}

/// Placa da data: bloco azul-marinho com a data em Oswald creme, destaque da barra superior.
pub fn placa_data(ui: &mut egui::Ui, data: &str) {
    let fam = familia_oswald(ui.ctx());
    egui::Frame::new()
        .fill(menu::AZUL_MARINHO)
        .stroke(Stroke::new(1.0, menu::TINTA))
        .inner_margin(Margin::symmetric(10, 2))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(data.to_uppercase())
                    .family(fam)
                    .size(17.0)
                    .strong()
                    .color(menu::PAPEL_CLARO),
            );
        });
}
