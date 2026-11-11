//! Tela de título, escolha de país (card estilo mockup) e dicas históricas da tela de carregamento.
use super::economy::RecursoMundo;
use super::jogador::{PaisJogador, fmt_pop};
use super::map::{ProvinciaSelecionada, RaizProjeto};
use crate::estado::AppState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use serde::Deserialize;
use simulacao::mundo::EstadoPais;

/// Sub-tela do menu principal.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum TelaMenu {
    #[default]
    Titulo,
    Paises,
    Opcoes,
}

/// Dica histórica (assets/menu/dicas.ron).
#[derive(Debug, Clone, Deserialize)]
pub struct Dica {
    #[allow(dead_code)]
    pub categoria: String,
    #[allow(dead_code)]
    pub ano: i32,
    #[allow(dead_code)]
    pub titulo: String,
    #[allow(dead_code)]
    pub texto: String,
    #[allow(dead_code)]
    pub dica_mecanica: String,
}

pub fn carregar_dicas(texto: &str) -> Vec<Dica> {
    ron::from_str(texto).unwrap_or_default()
}

#[derive(Resource, Default)]
pub struct Dicas(pub Vec<Dica>);

/// Fundo (mapa_1945.png) como textura egui.
#[derive(Resource, Default)]
struct FundoMenu(Option<egui::TextureHandle>);

/// Dificuldade pelo ranking de PIB entre os países com população.
pub fn dificuldade(posicao_pib: usize) -> &'static str {
    match posicao_pib {
        0..=2 => "Fácil",
        3..=9 => "Normal",
        10..=39 => "Difícil",
        _ => "Extrema",
    }
}

/// Descrição curta das potências (as demais recebem um texto genérico).
pub fn descricao_curta(tag: &str) -> &'static str {
    match tag {
        "USA" => "Única potência com a bomba atômica e a maior economia do planeta. Lidera o bloco ocidental e o dólar de Bretton Woods.",
        "SOV" => "Exército Vermelho vitorioso, mas país devastado. Planejamento central e uma cortina de ferro a erguer na Europa Oriental.",
        "GBR" => "Império exausto e endividado, ainda dono de colônias pelo mundo. Precisa escolher entre o império e a reconstrução.",
        "FRA" => "Governo provisório de De Gaulle, país libertado e arruinado. Busca recuperar o prestígio e o império colonial.",
        "CHI" => "Guerra civil iminente entre nacionalistas e comunistas. Gigante populoso, pobre e fragmentado.",
        "BRA" => "Maior país da América Latina, recém-saído do Estado Novo e aliado dos EUA. Industrialização em curso.",
        "GER" => "Derrotada, ocupada e dividida em zonas. Começa quase do zero, sob controle aliado.",
        "JAP" => "Em colapso e prestes à rendição. Ocupação americana e reconstrução à frente.",
        "ITA" => "Derrotada e dividida entre monarquia e república, com forte partido comunista.",
        "IND" => "Colônia britânica rumo à independência. Enorme população e economia agrária.",
        _ => "Nação em transição no mundo do pós-guerra. Escolha com cuidado entre os blocos que se formam.",
    }
}

pub fn nome_governo(ideologia: &str) -> String {
    let t = ideologia.replace('_', " ");
    let mut c = t.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

fn cor_egui(p: &EstadoPais) -> egui::Color32 {
    egui::Color32::from_rgb(p.cor.0, p.cor.1, p.cor.2)
}

pub struct MenuUiPlugin;

impl Plugin for MenuUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TelaMenu>()
            .init_resource::<Dicas>()
            .init_resource::<FundoMenu>()
            .add_systems(Startup, carregar_dicas_sistema)
            .add_systems(OnEnter(AppState::Menu), |mut t: ResMut<TelaMenu>| *t = TelaMenu::Titulo)
            .add_systems(EguiPrimaryContextPass, tela_menu_principal.run_if(in_state(AppState::Menu)));
    }
}

fn carregar_dicas_sistema(raiz: Res<RaizProjeto>, mut dicas: ResMut<Dicas>) {
    if let Ok(t) = std::fs::read_to_string(raiz.0.join("assets/menu/dicas.ron")) {
        dicas.0 = carregar_dicas(&t);
    }
}

pub fn desenhar_fundo(ctx: &egui::Context, fundo: &mut Option<egui::TextureHandle>, raiz: &std::path::Path) {
    if fundo.is_none()
        && let Ok(img) = image::open(raiz.join("assets/menu/mapa_1945.png"))
    {
        let rgba = img.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
        *fundo = Some(ctx.load_texture("fundo_menu", ci, egui::TextureOptions::LINEAR));
    }
    let tela = ctx.viewport_rect();
    let pintor = ctx.layer_painter(egui::LayerId::background());
    pintor.rect_filled(tela, 0.0, egui::Color32::from_rgb(0x1E, 0x1A, 0x16));
    if let Some(t) = fundo {
        pintor.image(t.id(), tela, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::from_white(0xB0));
    }
    pintor.rect_filled(tela, 0.0, egui::Color32::from_black_alpha(90));
}

/// Vinheta vetorial da Guerra Fria usada enquanto os dados do mapa são carregados.
fn _desenhar_animacao_carregamento(ctx: &egui::Context, segundos: f32) {
    let tela = ctx.viewport_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("animacao_carregamento")));
    let centro = tela.center();
    let escala = (tela.width() / 1200.0).clamp(0.65, 1.25);
    let tremor = (segundos * 2.8).sin() * 1.4 * escala;
    let bandeira = egui::vec2(150.0 * escala, 92.0 * escala);
    let y_bandeiras = centro.y - 72.0 * escala;
    let esquerda = egui::Rect::from_center_size(egui::pos2(centro.x - 250.0 * escala + tremor, y_bandeiras), bandeira);
    let direita = egui::Rect::from_center_size(egui::pos2(centro.x + 250.0 * escala - tremor, y_bandeiras), bandeira);

    painter.rect_filled(
        egui::Rect::from_center_size(egui::pos2(centro.x, centro.y - 20.0 * escala), egui::vec2(760.0 * escala, 300.0 * escala)),
        8.0,
        egui::Color32::from_rgba_unmultiplied(35, 29, 23, 105),
    );
    _desenhar_bandeira_usa(&painter, esquerda);
    _desenhar_bandeira_urss(&painter, direita);
    _desenhar_missil(&painter, tela, segundos, false, escala);
    _desenhar_missil(&painter, tela, segundos + 2.7, true, escala);
    ctx.request_repaint();
}

fn _desenhar_bandeira_usa(painter: &egui::Painter, rect: egui::Rect) {
    let azul = egui::Color32::from_rgb(25, 55, 96);
    let vermelho = egui::Color32::from_rgb(151, 42, 34);
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(235, 222, 194));
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(2.0, vermelho), egui::StrokeKind::Inside);
    for i in 0..7 {
        let y = rect.top() + rect.height() * (i as f32 + 0.5) / 7.0;
        painter.line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], egui::Stroke::new(rect.height() / 14.0, vermelho));
    }
    let canton = egui::Rect::from_min_max(rect.left_top(), egui::pos2(rect.left() + rect.width() * 0.43, rect.top() + rect.height() * 0.55));
    painter.rect_filled(canton, 0.0, azul);
    for row in 0..3 {
        for col in 0..4 {
            painter.circle_filled(
                egui::pos2(canton.left() + 7.0 + col as f32 * canton.width() / 4.0, canton.top() + 7.0 + row as f32 * canton.height() / 3.0),
                1.6,
                egui::Color32::from_rgb(239, 226, 190),
            );
        }
    }
    painter.text(rect.center_top() + egui::vec2(0.0, -22.0), egui::Align2::CENTER_CENTER, "EUA", egui::FontId::proportional(14.0), egui::Color32::from_rgb(235, 222, 194));
}

fn _desenhar_bandeira_urss(painter: &egui::Painter, rect: egui::Rect) {
    let vermelho = egui::Color32::from_rgb(157, 38, 31);
    let dourado = egui::Color32::from_rgb(239, 196, 74);
    painter.rect_filled(rect, 2.0, vermelho);
    painter.rect_stroke(rect, 2.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(235, 222, 194)), egui::StrokeKind::Inside);
    let c = rect.center();
    painter.circle_filled(c, rect.height() * 0.13, dourado);
    painter.line_segment([c - egui::vec2(0.0, 25.0), c + egui::vec2(0.0, 18.0)], egui::Stroke::new(4.0, dourado));
    painter.line_segment([c - egui::vec2(18.0, 8.0), c + egui::vec2(18.0, 8.0)], egui::Stroke::new(4.0, dourado));
    let mut foice = Vec::with_capacity(13);
    for i in 0..13 {
        let a = std::f32::consts::PI * 0.25 + std::f32::consts::PI * 1.5 * i as f32 / 12.0;
        foice.push(egui::pos2(c.x + 22.0 * a.cos(), c.y - 3.0 + 22.0 * a.sin()));
    }
    painter.add(egui::Shape::line(foice, egui::Stroke::new(3.0, dourado)));
    painter.text(rect.center_top() + egui::vec2(0.0, -22.0), egui::Align2::CENTER_CENTER, "URSS", egui::FontId::proportional(14.0), egui::Color32::from_rgb(239, 196, 74));
}

fn _desenhar_missil(painter: &egui::Painter, tela: egui::Rect, segundos: f32, direita: bool, escala: f32) {
    let progresso = (segundos * 0.16).rem_euclid(1.0);
    let (inicio_x, fim_x) = if direita { (tela.right() * 0.88, tela.right() * 0.58) } else { (tela.left() + tela.width() * 0.12, tela.left() + tela.width() * 0.42) };
    let inicio = egui::pos2(inicio_x, tela.bottom() + 24.0);
    let cor = if direita { egui::Color32::from_rgb(218, 83, 61) } else { egui::Color32::from_rgb(92, 130, 170) };
    let ponto = |p: f32| {
        egui::pos2(
            inicio.x + (fim_x - inicio.x) * p,
            inicio.y - tela.height() * 0.52 * p - tela.height() * 0.26 * (std::f32::consts::PI * p).sin(),
        )
    };
    let mut anterior = ponto(progresso);
    for i in 1..=12 {
        let p = (progresso - i as f32 * 0.018).rem_euclid(1.0);
        let atual = ponto(p);
        let alpha = (150.0 * (1.0 - i as f32 / 13.0)) as u8;
        painter.line_segment([anterior, atual], egui::Stroke::new(3.0 * escala, egui::Color32::from_rgba_unmultiplied(cor.r(), cor.g(), cor.b(), alpha)));
        anterior = atual;
    }
    let p = ponto(progresso);
    let direcao = egui::vec2(if direita { 0.7 } else { -0.7 }, -1.0).normalized();
    let lateral = egui::vec2(-direcao.y, direcao.x);
    let base = p - direcao * 18.0 * escala;
    painter.line_segment([base, p + direcao * 10.0 * escala], egui::Stroke::new(6.0 * escala, cor));
    painter.add(egui::Shape::convex_polygon(vec![p + direcao * 17.0 * escala, base + lateral * 7.0 * escala, base - lateral * 7.0 * escala], cor, egui::Stroke::NONE));
    painter.line_segment([base - direcao * 15.0 * escala, base - direcao * 26.0 * escala], egui::Stroke::new(5.0 * escala, egui::Color32::from_rgb(237, 177, 64)));
}

trait CorBranca {
    fn from_white(a: u8) -> egui::Color32;
}
impl CorBranca for egui::Color32 {
    fn from_white(a: u8) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, a)
    }
}

fn tela_menu_principal(
    mut contextos: EguiContexts,
    mut tela: ResMut<TelaMenu>,
    mut fundo: ResMut<FundoMenu>,
    raiz: Res<RaizProjeto>,
    mut prox: ResMut<NextState<AppState>>,
    mut commands: Commands,
    mundo: Res<RecursoMundo>,
    selecionada: Res<ProvinciaSelecionada>,
    mut busca: Local<String>,
    mut escolha: Local<Option<String>>,
    mut sair: MessageWriter<AppExit>,
    mut janela_saves: ResMut<super::menu_saves::JanelaSaves>,
    mut config_janela: ResMut<super::menu_pausa::JanelaConfig>,
    mut tutorial: ResMut<super::tutorial_guiado::TutorialGuiado>,
    config: Res<super::config::ConfigJogo>,
) -> Result {
    let ctx = contextos.ctx_mut()?.clone();
    // Cliques no mapa escolhem o país, mas o fundo só aparece em Titulo/Paises/Opcoes.
    desenhar_fundo(&ctx, &mut fundo.0, &raiz.0);
    let mut iniciar: Option<String> = None;
    let mut raiz_ui = super::ui::ui_raiz(&ctx);
    match *tela {
        TelaMenu::Titulo => {
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(&mut raiz_ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.18);
                    ui.label(egui::RichText::new("HEGEMONY: 1945").size(64.0).strong().color(egui::Color32::from_rgb(0xE8, 0xDE, 0xC8)));
                    ui.label(egui::RichText::new("A divisão do mundo — 1º de janeiro de 1945").size(20.0).color(egui::Color32::from_rgb(0xDF, 0xC9, 0xA0)));
                    ui.add_space(40.0);
                    let tam = egui::vec2(260.0, 44.0);
                    if ui.add_sized(tam, egui::Button::new(egui::RichText::new("Novo Jogo").size(22.0))).clicked() {
                        *tela = TelaMenu::Paises;
                    }
                    let botao_tut = egui::Button::new(egui::RichText::new("Tutorial").size(22.0).color(egui::Color32::from_rgb(0x23, 0x20, 0x1B)))
                        .fill(egui::Color32::from_rgb(0xE8, 0xDE, 0xC8))
                        .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(0x8C, 0x25, 0x1E)));
                    if ui.add_sized(tam, botao_tut).on_hover_text("Partida guiada com os Estados Unidos").clicked() {
                        tutorial.iniciar();
                        iniciar = Some("USA".to_string());
                    }
                    if config.partida.mostrar_tutorial {
                        ui.label(egui::RichText::new("Primeira vez? Faça o Tutorial.").italics().color(egui::Color32::from_rgb(0xDF, 0xC9, 0xA0)));
                    }
                    if ui.add_sized(tam, egui::Button::new(egui::RichText::new("Carregar").size(22.0))).clicked() {
                        janela_saves.abrir(true);
                    }
                    if ui.add_sized(tam, egui::Button::new(egui::RichText::new("Opções").size(22.0))).clicked() {
                        config_janela.abrir();
                    }
                    if ui.add_sized(tam, egui::Button::new(egui::RichText::new("Sair").size(22.0))).clicked() {
                        sair.write(AppExit::Success);
                    }
                });
            });
        }
        // As opções agora são a janela de Configurações (menu_pausa::JanelaConfig).
        TelaMenu::Opcoes => *tela = TelaMenu::Titulo,
        TelaMenu::Paises => {
            // Ranking por PIB entre países habitados.
            let mut ranking: Vec<(&String, f64)> = mundo.paises.iter().filter(|(_, p)| p.populacao_total() > 0).map(|(t, p)| (t, p.pib_anual_estimado)).collect();
            ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            if let Some(tag) = selecionada_do_mapa(&selecionada, &mundo)
                && escolha.as_deref() != Some(tag.as_str())
                && selecionada.is_changed()
            {
                *escolha = Some(tag);
            }
            egui::Panel::left("lista_paises").exact_size(300.0).show(&mut raiz_ui, |ui| {
                ui.heading("Escolha sua nação");
                if ui.button("< Voltar").clicked() {
                    *tela = TelaMenu::Titulo;
                }
                ui.separator();
                ui.strong("Potências");
                ui.horizontal_wrapped(|ui| {
                    for tag in ["USA", "SOV", "GBR", "FRA", "CHI", "BRA", "GER", "JAP", "ITA", "IND", "CAN", "ARG", "MEX", "TUR"] {
                        if let Some(p) = mundo.paises.get(tag)
                            && ui.selectable_label(escolha.as_deref() == Some(tag), &p.nome).clicked()
                        {
                            *escolha = Some(tag.to_string());
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Buscar:");
                    ui.text_edit_singleline(&mut *busca);
                });
                let b = busca.to_lowercase();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (tag, _) in &ranking {
                        let p = &mundo.paises[*tag];
                        if !b.is_empty() && !p.nome.to_lowercase().contains(&b) && !p.tag.to_lowercase().contains(&b) {
                            continue;
                        }
                        if ui.selectable_label(escolha.as_deref() == Some(tag.as_str()), format!("{} ({})", p.nome, p.tag)).clicked() {
                            *escolha = Some((*tag).clone());
                        }
                    }
                });
            });
            egui::CentralPanel::default().frame(egui::Frame::NONE).show(&mut raiz_ui, |ui| {
                // Centralizado sem "justified": o justified esticava a 1ª linha do card até o fim da tela
                // e empurrava os dados e o botão "Governar" para fora da área visível.
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space((ui.available_height() * 0.12).max(20.0));
                    if let Some(tag) = escolha.clone()
                        && let Some(p) = mundo.paises.get(&tag)
                    {
                        let pos = ranking.iter().position(|(t, _)| **t == tag).unwrap_or(99);
                        egui::Frame::NONE
                            .fill(egui::Color32::from_rgb(0xE8, 0xDE, 0xC8))
                            .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(0x8C, 0x25, 0x1E)))
                            .inner_margin(20.0)
                            .show(ui, |ui| {
                                ui.set_width(420.0);
                                if card_pais(ui, p, pos) {
                                    tutorial.ativo = false;
                                    iniciar = Some(tag);
                                }
                            });
                    } else {
                        ui.label(egui::RichText::new("Escolha um país na lista ou clique numa província do mapa.").size(18.0).color(egui::Color32::WHITE));
                    }
                });
            });
        }
    }
    if let Some(tag) = iniciar {
        commands.insert_resource(PaisJogador(tag));
        prox.set(AppState::Jogando);
    }
    Ok(())
}

fn selecionada_do_mapa(sel: &ProvinciaSelecionada, mundo: &RecursoMundo) -> Option<String> {
    if sel.0 == 0 {
        return None;
    }
    mundo.dono_por_provincia.get(&sel.0).filter(|t| mundo.paises.get(*t).is_some_and(|p| p.populacao_total() > 0)).cloned()
}

/// Card do país; devolve true quando o jogador confirma.
fn card_pais(ui: &mut egui::Ui, p: &EstadoPais, posicao_pib: usize) -> bool {
    let tinta = egui::Color32::from_rgb(0x17, 0x16, 0x14);
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(72.0, 48.0), egui::Sense::hover());
        ui.painter().rect_filled(r, 2.0, cor_egui(p));
        ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.5, tinta), egui::StrokeKind::Outside);
        ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, &p.tag, egui::FontId::monospace(18.0), egui::Color32::WHITE);
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(&p.nome).size(28.0).strong().color(tinta));
            ui.label(egui::RichText::new(format!("Capital: {}", p.capital)).color(egui::Color32::from_rgb(0x3A, 0x37, 0x32)));
        });
    });
    ui.separator();
    egui::Grid::new("card_dados").num_columns(2).spacing([20.0, 6.0]).show(ui, |ui| {
        let mut linha = |k: &str, v: String| {
            ui.label(egui::RichText::new(k).color(egui::Color32::from_rgb(0x5C, 0x58, 0x50)));
            ui.label(egui::RichText::new(v).monospace().strong().color(tinta));
            ui.end_row();
        };
        linha("Governo", nome_governo(&p.ideologia));
        linha("PIB anual", format!("US$ {}", fmt_pop(p.pib_anual_estimado as u64)));
        linha("População", fmt_pop(p.populacao_total()));
        linha("Ranking PIB", format!("{}º", posicao_pib + 1));
        linha("Dificuldade", dificuldade(posicao_pib).to_string());
    });
    ui.separator();
    ui.label(egui::RichText::new(descricao_curta(&p.tag)).color(tinta));
    ui.add_space(12.0);
    ui.add_sized([ui.available_width(), 40.0], egui::Button::new(egui::RichText::new(format!("Governar {}", p.nome)).size(20.0))).clicked()
}

/// Dica da tela de carregamento (troca a cada 6 s).
#[allow(dead_code)]
pub fn dica_atual<'a>(dicas: &'a [Dica], segundos: f32) -> Option<&'a Dica> {
    if dicas.is_empty() {
        return None;
    }
    dicas.get((segundos / 6.0) as usize % dicas.len())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn dicas_do_arquivo_carregam() {
        let t = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/menu/dicas.ron")).unwrap();
        let d = carregar_dicas(&t);
        assert_eq!(d.len(), 30);
        assert!(dica_atual(&d, 7.0).is_some());
    }

    #[test]
    fn dificuldade_por_ranking() {
        assert_eq!(dificuldade(0), "Fácil");
        assert_eq!(dificuldade(50), "Extrema");
        assert_eq!(nome_governo("democracia_batllista"), "Democracia batllista");
    }
}
