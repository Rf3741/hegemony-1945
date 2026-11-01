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
pub enum TelaMenu {
    #[default]
    Titulo,
    Paises,
    Opcoes,
}

/// Dica histórica (assets/menu/dicas.ron).
#[derive(Debug, Clone, Deserialize)]
pub struct Dica {
    pub categoria: String,
    pub ano: i32,
    pub titulo: String,
    pub texto: String,
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
    mut carregar: MessageWriter<super::saveload::PedirCarregar>,
    slots: Res<super::saveload::Slots>,
    mut menu_slots: Local<bool>,
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
                    if ui.add_sized(tam, egui::Button::new(egui::RichText::new("Carregar").size(22.0))).clicked() {
                        *menu_slots = !*menu_slots;
                    }
                    if ui.add_sized(tam, egui::Button::new(egui::RichText::new("Opções").size(22.0))).clicked() {
                        *tela = TelaMenu::Opcoes;
                    }
                    if ui.add_sized(tam, egui::Button::new(egui::RichText::new("Sair").size(22.0))).clicked() {
                        sair.write(AppExit::Success);
                    }
                    if *menu_slots {
                        ui.add_space(16.0);
                        egui::Frame::NONE.fill(egui::Color32::from_black_alpha(170)).inner_margin(10.0).show(ui, |ui| {
                            super::saveload::lista_slots(ui, &slots, |s| {
                                carregar.write(super::saveload::PedirCarregar(s));
                            }, None);
                        });
                    }
                });
            });
        }
        TelaMenu::Opcoes => {
            egui::Window::new("Opções").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(&ctx, |ui| {
                ui.label("Atalhos: F1 país, F2 mercado, F3 comércio, F4 diplomacia, F5 tecnologia, B construção, J jornal.");
                ui.label("Câmera: WASD/setas, roda do mouse e arrastar. Espaço pausa; 1-5 velocidade.");
                ui.label("Salvar: F9 (rápido) ou menu de pausa; carregar pelo menu.");
                if ui.button("Voltar").clicked() {
                    *tela = TelaMenu::Titulo;
                }
            });
        }
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
                ui.centered_and_justified(|ui| {
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
