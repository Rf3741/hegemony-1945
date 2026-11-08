//! Interface egui: barra superior (data e controles de tempo) e painel de província.
use super::comercio_ui::PainelComercio;
use super::construcao_ui::{ConstrucaoUi, secao_construir_aqui};
use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::enciclopedia::dica;
use super::jogador::{PainelPais, PaisJogador, hud_pais, resumo_pais};
use super::map::{ProvinciaSelecionada, RecursoCatalogo};
use super::mercado_ui::PainelMercado;
use super::time::RecursoRelogio;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use gf_mapa::tempo::VelocidadeJogo;

/// Estado por quadro do retângulo livre da janela principal: `(quadro, rect_atual)`.
type EstadoLayoutRaiz = (u64, egui::Rect);

fn id_layout_raiz() -> egui::Id {
    egui::Id::new("layout_raiz_partida")
}

/// Cria um `egui::Ui` para o próximo painel da cadeia (`barra_superior` -> `barra_hud` -> `barra_extra` -> `painel_provincia`),
/// usando apenas o retângulo restante deixado pelos painéis já desenhados neste quadro.
pub(crate) fn ui_raiz(ctx: &egui::Context) -> egui::Ui {
    let quadro = ctx.cumulative_frame_nr();
    let tela = ctx.viewport_rect();
    let rect = ctx.data_mut(|d| {
        let est = d.get_temp_mut_or_insert_with::<EstadoLayoutRaiz>(id_layout_raiz(), || (quadro, tela));
        if est.0 != quadro {
            *est = (quadro, tela);
        }
        est.1
    });
    egui::Ui::new(
        ctx.clone(),
        "viewport_raiz".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(rect),
    )
}

/// Desconta a altura de um painel de topo recém-desenhado (`max_y`) do retângulo livre do quadro.
pub(crate) fn consumir_topo(ctx: &egui::Context, max_y: f32) {
    let quadro = ctx.cumulative_frame_nr();
    let tela = ctx.viewport_rect();
    ctx.data_mut(|d| {
        let est = d.get_temp_mut_or_insert_with::<EstadoLayoutRaiz>(id_layout_raiz(), || (quadro, tela));
        if est.0 != quadro {
            *est = (quadro, tela);
        }
        est.1.min.y = est.1.min.y.max(max_y).min(est.1.max.y);
    });
}

/// Desconta a largura de um painel lateral direito recém-desenhado (`min_x`) do retângulo livre do quadro.
pub(crate) fn consumir_direita(ctx: &egui::Context, min_x: f32) {
    let quadro = ctx.cumulative_frame_nr();
    let tela = ctx.viewport_rect();
    ctx.data_mut(|d| {
        let est = d.get_temp_mut_or_insert_with::<EstadoLayoutRaiz>(id_layout_raiz(), || (quadro, tela));
        if est.0 != quadro {
            *est = (quadro, tela);
        }
        est.1.max.x = est.1.max.x.min(min_x).max(est.1.min.x);
    });
}

/// Retângulo central livre do mapa (abaixo das barras superiores e à esquerda do painel de província).
pub(crate) fn area_livre_mapa(ctx: &egui::Context) -> egui::Rect {
    let tela = ctx.viewport_rect();
    ctx.data(|d| d.get_temp::<EstadoLayoutRaiz>(id_layout_raiz()))
        .map_or(tela, |(_, r)| r.intersect(tela))
}

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
    mut relogio: ResMut<RecursoRelogio>,
) -> Result {
    let ctx = contextos.ctx_mut()?.clone();
    let mut raiz = ui_raiz(&ctx);
    let resp = egui::Panel::top("barra_superior")
        .frame(
            egui::Frame::NONE
                .fill(ui_refinada::menu::PAPEL)
                .stroke(egui::Stroke::new(1.0, ui_refinada::menu::VERMELHO))
                .inner_margin(6.0),
        )
        .show(&mut raiz, |ui| {
            ui.horizontal(|ui| {
                let b_salvar = ui_refinada::botao_estilo_secundario(ui, "Salvar / Carregar")
                    .on_hover_text("Abre o menu de jogos salvos (Ctrl+S salva rápido, Ctrl+L carrega)");
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
                ui.label(egui::RichText::new("?").color(ui_refinada::menu::TINTA_SUAVE)).on_hover_text(
                    "Espaço: pausa | +/−: velocidade | WASD: mover | scroll: zoom | V: construções | M: modos de mapa | 1-9: painéis | Ctrl+S/L: salvar/carregar",
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let r_rel = widget_relogio(ui, &mut relogio);
                    super::tutorial_guiado::alvo(ui.ctx(), "relogio", r_rel);
                });
            });
        });
    consumir_topo(&ctx, resp.response.rect.max.y);
    Ok(())
}

/// Relógio estilo Victoria 3/HOI4: data, pausa, − / + e barrinhas de velocidade (1–5).
/// Retorna o retângulo exato ocupado pelos controles do relógio.
fn widget_relogio(ui: &mut egui::Ui, relogio: &mut RecursoRelogio) -> egui::Rect {
    // Em layout direita-para-esquerda: o último desenhado fica mais à esquerda.
    let r_mais = ui.small_button("+").on_hover_text("Aumentar velocidade (+)");
    if r_mais.clicked() {
        let v = relogio.velocidade.aumentar();
        relogio.definir_velocidade(v);
    }
    let mut rect = r_mais.rect;
    for v in VelocidadeJogo::TODAS.iter().rev() {
        let n = v.nivel();
        let (r, resp) = ui.allocate_exact_size(egui::vec2(8.0, 18.0), egui::Sense::click());
        rect = rect.union(r);
        let acesa = !relogio.pausado && n <= relogio.velocidade.nivel();
        let cor = if acesa {
            ui_refinada::menu::VERMELHO
        } else if resp.hovered() {
            ui_refinada::menu::TINTA_SUAVE
        } else {
            ui_refinada::menu::PAPEL_CARD
        };
        let alto = 6.0 + 3.0 * n as f32;
        let barra = egui::Rect::from_min_max(
            egui::pos2(r.left() + 1.0, r.bottom() - alto),
            egui::pos2(r.right() - 1.0, r.bottom()),
        );
        ui.painter().rect_filled(barra, 1.0, cor);
        if resp.on_hover_text(v.rotulo_pt()).clicked() {
            relogio.definir_velocidade(*v);
        }
    }
    let r_menos = ui.small_button("-").on_hover_text("Diminuir velocidade (−)");
    rect = rect.union(r_menos.rect);
    if r_menos.clicked() {
        let v = relogio.velocidade.diminuir();
        relogio.definir_velocidade(v);
    }
    let rotulo = if relogio.pausado { "Continuar" } else { "Pausar" };
    let b_pausa = dica(ui_refinada::botao_estilo_menu(ui, rotulo), "tooltip_topbar_botao_pausar");
    super::tutorial_guiado::marcar(&b_pausa, "pausa");
    rect = rect.union(b_pausa.rect);
    if b_pausa.clicked() {
        relogio.alternar_pausa();
    }
    let placa = egui::Frame::new().show(ui, |ui| {
        ui_refinada::placa_data(ui, &relogio.data.formatar_pt_br());
    });
    let resp = dica(placa.response, "tooltip_topbar_data");
    if relogio.pausado {
        let r_placa = resp.rect;
        ui.painter().rect_stroke(
            r_placa.shrink(1.0),
            0.0,
            egui::Stroke::new(1.5, ui_refinada::menu::VERMELHO_HOVER),
            egui::StrokeKind::Inside,
        );
    }
    rect.union(resp.rect)
}

fn barra_hud(
    mut contextos: EguiContexts,
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
    let ctx = contextos.ctx_mut()?.clone();
    let mut raiz = ui_raiz(&ctx);
    let resp = egui::Panel::top("barra_hud")
        .frame(
            egui::Frame::NONE
                .fill(ui_refinada::menu::PAPEL)
                .stroke(egui::Stroke::new(1.5, ui_refinada::menu::VERMELHO))
                .inner_margin(6.0),
        )
        .show(&mut raiz, |ui| {
            hud_pais(ui, p, &eco, &mut painel.aberto, &mut mercado.aberto, &mut comercio.aberto, &mut constr.aberta);
            super::tutorial_guiado::alvo(ui.ctx(), "hud", ui.min_rect());
        });
    consumir_topo(&ctx, resp.response.rect.max.y);
    Ok(())
}

pub(crate) fn painel_provincia(
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
    let ctx = contextos.ctx_mut()?.clone();
    let mut raiz = ui_raiz(&ctx);
    let resp = egui::Panel::right("painel_provincia")
        .frame(ui_refinada::moldura_janela_papel())
        .min_size(260.0)
        .show(&mut raiz, |ui| {
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
                egui::ScrollArea::vertical()
                    .id_salt("scroll_painel_prov")
                    .show(ui, |ui| secao_construir_aqui(ui, &eco.vista, &prov.adm1_code, &mut constr));
            }
        });
    super::tutorial_guiado::alvo(&ctx, "painel_provincia", resp.response.rect);
    consumir_direita(&ctx, resp.response.rect.min.x);
    Ok(())
}
