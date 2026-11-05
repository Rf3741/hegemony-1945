//! Interface egui: barra superior (data e controles de tempo) e painel de província.
use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::comercio_ui::PainelComercio;
use super::construcao_ui::{ConstrucaoUi, secao_construir_aqui};
use super::mercado_ui::PainelMercado;
use super::jogador::{PainelPais, PaisJogador, hud_pais, resumo_pais};
use super::map::{ProvinciaSelecionada, RecursoCatalogo};
use super::time::RecursoRelogio;
use bevy::prelude::*;
use super::enciclopedia::dica;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

pub(crate) fn ui_raiz(ctx: &egui::Context) -> egui::Ui {
    egui::Ui::new(
        ctx.clone(),
        "viewport_raiz".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    )
}
use gf_mapa::tempo::VelocidadeJogo;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            EguiPrimaryContextPass,
            (barra_superior, barra_hud, super::hud_mapas::barra_extra, painel_provincia)
                .chain()
                .run_if(crate::estado::em_partida),
        );
    }
}

fn barra_superior(
    mut contextos: EguiContexts,
    camada_visivel: Option<Res<visual_mapa::CamadaConstrucoesVisivel>>,
    mut janela_saves: ResMut<super::menu_saves::JanelaSaves>,
) -> Result {
    let mut raiz = ui_raiz(contextos.ctx_mut()?);
    egui::Panel::top("barra_superior")
        .frame(egui::Frame::NONE.fill(ui_refinada::menu::PAPEL).stroke(egui::Stroke::new(1.0, ui_refinada::menu::VERMELHO)).inner_margin(6.0))
        .show(&mut raiz, |ui| {
            ui.horizontal(|ui| {
                let b_salvar = ui_refinada::botao_estilo_secundario(ui, "Salvar / Carregar").on_hover_text("Abre o menu de jogos salvos (Ctrl+S salva rápido, Ctrl+L carrega)");
                super::tutorial_guiado::marcar(&b_salvar, "salvar");
                if b_salvar.clicked() {
                    janela_saves.abrir(false);
                }
                ui.separator();
                if let Some(c) = camada_visivel {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                    ui_refinada::theme::icone::desenhar(
                        ui.painter(),
                        rect,
                        "torre",
                        ui_refinada::menu::TINTA,
                        ui_refinada::menu::VERMELHO,
                    );
                    let texto = if c.0 { "Construções: [V] Ativas" } else { "Construções: [V] Ocultas" };
                    ui.label(egui::RichText::new(texto).color(ui_refinada::menu::TINTA));
                    ui.separator();
                }
                ui.label(egui::RichText::new("?").color(ui_refinada::menu::TINTA_SUAVE)).on_hover_text("Espaço: pausa | +/−: velocidade | WASD: mover | scroll: zoom | V: construções | M: modos de mapa | 1-9: painéis | Ctrl+S/L: salvar/carregar");
            });
        });
    Ok(())
}

/// Relógio estilo Victoria 3/HOI4: placa de data, pausa, − / + e barrinhas de velocidade (1–5).
fn widget_relogio(ui: &mut egui::Ui, relogio: &mut RecursoRelogio) {
    // Em layout direita-para-esquerda: o último desenhado fica mais à esquerda.
    if ui.small_button("+").on_hover_text("Aumentar velocidade (+)").clicked() {
        let v = relogio.velocidade.aumentar();
        relogio.definir_velocidade(v);
    }
    for v in VelocidadeJogo::TODAS.iter().rev() {
        let n = v.nivel();
        let (r, resp) = ui.allocate_exact_size(egui::vec2(8.0, 18.0), egui::Sense::click());
        let acesa = !relogio.pausado && n <= relogio.velocidade.nivel();
        let cor = if acesa {
            ui_refinada::menu::VERMELHO
        } else if resp.hovered() {
            ui_refinada::menu::TINTA_SUAVE
        } else {
            ui_refinada::menu::PAPEL_CARD
        };
        let alto = 6.0 + 3.0 * n as f32;
        let barra = egui::Rect::from_min_max(egui::pos2(r.left() + 1.0, r.bottom() - alto), egui::pos2(r.right() - 1.0, r.bottom()));
        ui.painter().rect_filled(barra, 1.0, cor);
        if resp.on_hover_text(v.rotulo_pt()).clicked() {
            relogio.definir_velocidade(*v);
        }
    }
    if ui.small_button("-").on_hover_text("Diminuir velocidade (−)").clicked() {
        let v = relogio.velocidade.diminuir();
        relogio.definir_velocidade(v);
    }
    let rotulo = if relogio.pausado { "Continuar" } else { "Pausar" };
    let b_pausa = dica(ui_refinada::botao_estilo_menu(ui, rotulo), "tooltip_topbar_botao_pausar");
    super::tutorial_guiado::marcar(&b_pausa, "pausa");
    if b_pausa.clicked() {
        relogio.alternar_pausa();
    }
    let placa = egui::Frame::new().show(ui, |ui| {
        ui_refinada::placa_data(ui, &relogio.data.formatar_pt_br());
    });
    let resp = dica(placa.response, "tooltip_topbar_data");
    if relogio.pausado {
        let rect = resp.rect;
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            0.0,
            egui::Stroke::new(1.5, ui_refinada::menu::VERMELHO_HOVER),
            egui::StrokeKind::Inside,
        );
    }
}

fn barra_hud(
    mut contextos: EguiContexts,
    mut relogio: ResMut<RecursoRelogio>,
    jogador: Option<Res<PaisJogador>>,
    mundo: Res<RecursoMundo>,
    eco: Option<Res<EconomiaSim>>,
    mut painel: ResMut<PainelPais>,
    mut mercado: ResMut<PainelMercado>,
    mut comercio: ResMut<PainelComercio>,
    mut constr: ResMut<ConstrucaoUi>,
) -> Result {
    let Some(eco) = eco else { return Ok(()) };
    let Some(p) = jogador.and_then(|j| mundo.paises.get(&j.0)) else { return Ok(()) };
    let mut raiz = ui_raiz(contextos.ctx_mut()?);
    egui::Panel::top("barra_hud")
        .frame(egui::Frame::NONE.fill(ui_refinada::menu::PAPEL).stroke(egui::Stroke::new(1.5, ui_refinada::menu::VERMELHO)).inner_margin(6.0))
        .show(&mut raiz, |ui| {
            hud_pais(ui, p, &eco, &mut painel.aberto, &mut mercado.aberto, &mut comercio.aberto, &mut constr.aberta);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                widget_relogio(ui, &mut relogio);
                super::tutorial_guiado::alvo(ui.ctx(), "relogio", ui.min_rect());
            });
            super::tutorial_guiado::alvo(ui.ctx(), "hud", ui.min_rect());
        });
    Ok(())
}

fn painel_provincia(
    mut contextos: EguiContexts,
    selecionada: Res<ProvinciaSelecionada>,
    catalogo: Res<RecursoCatalogo>,
    mundo: Res<RecursoMundo>,
    eco: Option<Res<EconomiaSim>>,
    mut constr: ResMut<ConstrucaoUi>,
) -> Result {
    if selecionada.0 == 0 {
        return Ok(());
    }
    let Some(prov) = catalogo.0.obter_por_id(selecionada.0) else { return Ok(()) };
    let mut raiz = ui_raiz(contextos.ctx_mut()?);
    egui::Panel::right("painel_provincia")
        .frame(ui_refinada::moldura_janela_papel())
        .min_size(260.0)
        .show(&mut raiz, |ui| {
            super::tutorial_guiado::alvo(ui.ctx(), "painel_provincia", ui.max_rect());
            ui_refinada::cabecalho_painel(ui, &prov.nome);
            let estado = if prov.estado.is_empty() { prov.adm1_code.as_str() } else { prov.estado.as_str() };
            ui.label(format!("Estado: {estado}"));
            ui.label(format!("ID da província: {} | Código do estado (ADM1): {}", prov.id, prov.adm1_code));
            ui.label(format!("Área da província: {:.0} km²", prov.area));
            ui.label(format!("País moderno: {}", prov.pais_moderno));
            if let Some(tag) = mundo.dono_por_provincia.get(&prov.id) {
                let nome = mundo.paises.get(tag).map(|p| p.nome.as_str()).unwrap_or("?");
                ui.label(format!("Dono em 1945: {nome} ({tag})"));
                if let Some(pais) = mundo.paises.get(tag) {
                    ui.separator();
                    resumo_pais(ui, pais);
                    ui.separator();
                }
            }
            if let Some(pop) = mundo.populacao_por_provincia.get(&prov.id) {
                ui.label(format!("População da província: {pop}"));
            }
            let irmas = catalogo.0.provincias.iter().filter(|p| p.adm1_code == prov.adm1_code).count();
            ui.label(format!("Vizinhas: {} | Províncias do estado: {irmas}", prov.vizinhos.len()));
            ui.label("Construção, economia e política são por estado.");
            if let Some(eco) = &eco {
                egui::ScrollArea::vertical().id_salt("scroll_painel_prov").show(ui, |ui| secao_construir_aqui(ui, &eco.vista, &prov.adm1_code, &mut constr));
            }
        });
    Ok(())
}
