//! Eventos históricos: avaliação diária, pop-up estilo dossiê (pausa o jogo) e jornal.
use super::economy::RecursoMundo;
use super::jogador::PaisJogador;
use super::time::{EventoTickDiario, RecursoRelogio};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use eventos::{CatalogoEventos, EstadoEventos, aplicar_opcao, avaliar_dia, descrever_efeito};

#[derive(Resource)]
pub struct CatalogoEv(pub CatalogoEventos);

#[derive(Resource, Default)]
pub struct EstadoEv {
    pub estado: EstadoEventos,
    /// O relógio estava rodando quando o pop-up abriu (retoma ao decidir).
    retomar: bool,
    pub jornal_aberto: bool,
}

pub struct EventosPlugin;

impl Plugin for EventosPlugin {
    fn build(&self, app: &mut App) {
        let raiz = app
            .world()
            .get_resource::<super::map::RaizProjeto>()
            .map(|r| r.0.clone())
            .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        match CatalogoEventos::carregar(&raiz.join("assets/data/eventos")) {
            Ok(c) => {
                info!("eventos: {} carregados", c.eventos.len());
                app.insert_resource(CatalogoEv(c));
            }
            Err(e) => error!("eventos: falha ao carregar: {e}"),
        }
        app.insert_resource(EstadoEv { jornal_aberto: true, ..default() })
            .add_systems(Update, (tick_eventos, atalho_jornal).run_if(resource_exists::<CatalogoEv>).run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida))
            .add_systems(
                EguiPrimaryContextPass,
                (popup_evento, jornal).run_if(resource_exists::<CatalogoEv>).run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida),
            );
    }
}

fn tick_eventos(
    mut leitor: MessageReader<EventoTickDiario>,
    cat: Res<CatalogoEv>,
    pais: Res<PaisJogador>,
    mut mundo: ResMut<RecursoMundo>,
    mut ev: ResMut<EstadoEv>,
    mut relogio: ResMut<RecursoRelogio>,
    cfg: Res<super::config::ConfigJogo>,
) {
    if leitor.read().count() > 0 {
        let est = &mut ev.estado;
        avaliar_dia(&cat.0, est, &mut mundo.0, &pais.0);
    }
    // Enquanto houver decisão pendente o tempo fica parado.
    if !ev.estado.pendentes.is_empty() && cfg.partida.pausar_em_eventos {
        if !relogio.pausado {
            ev.retomar = true;
            relogio.pausado = true;
        }
    }
}

fn atalho_jornal(teclas: Res<ButtonInput<KeyCode>>, mut ev: ResMut<EstadoEv>) {
    if teclas.just_pressed(KeyCode::KeyJ) {
        ev.jornal_aberto = !ev.jornal_aberto;
    }
}

const PAPEL: egui::Color32 = ui_refinada::menu::PAPEL;
const TINTA: egui::Color32 = ui_refinada::menu::TINTA;
const TINTA_SUAVE: egui::Color32 = ui_refinada::menu::TINTA_SUAVE;
const CARIMBO: egui::Color32 = ui_refinada::menu::VERMELHO;

fn popup_evento(
    mut contextos: EguiContexts,
    cat: Res<CatalogoEv>,
    mut mundo: ResMut<RecursoMundo>,
    mut ev: ResMut<EstadoEv>,
    mut relogio: ResMut<RecursoRelogio>,
    cfg: Res<super::config::ConfigJogo>,
    elenco: Option<Res<super::personagens_sim::ElencoRes>>,
    mut retratos: Option<ResMut<super::personagens_ui::Retratos>>,
) -> Result {
    let Some(pend) = ev.estado.pendentes.first().cloned() else { return Ok(()) };
    let Some(evento) = cat.0.por_id(&pend.evento_id) else {
        ev.estado.pendentes.remove(0);
        return Ok(());
    };
    if cfg.partida.pausar_em_eventos {
        relogio.pausado = true;
    }
    let d = mundo.0.data;
    let data = format!("{:02}/{:02}/{}", d.dia, d.mes, d.ano);
    let cor_pais = mundo.0.paises.get(&pend.pais).map(|p| egui::Color32::from_rgb(p.cor.0, p.cor.1, p.cor.2)).unwrap_or(CARIMBO);
    let nome_pais = mundo.0.paises.get(&pend.pais).map(|p| p.nome.clone()).unwrap_or_default();
    let mut escolhida: Option<usize> = None;
    let faltam = ev.estado.pendentes.len() - 1;

    // Véu escurecido acima dos painéis abertos: bloqueia cliques enquanto o dossiê urgente está na tela.
    let ctx = contextos.ctx_mut()?.clone();
    egui::Area::new(egui::Id::new("veu_evento_urgente"))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(0.0, 0.0))
        .show(&ctx, |ui| {
            let tela = ui.ctx().viewport_rect();
            ui.allocate_rect(tela, egui::Sense::click_and_drag());
            ui.painter().rect_filled(tela, 0.0, egui::Color32::from_black_alpha(140));
        });

    egui::Window::new("evento_historico")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([620.0, 0.0])
        .frame(ui_refinada::moldura_janela_papel())
        .show(contextos.ctx_mut()?, |ui| {
            ui_refinada::decorar_janela(ui);
            // Cabeçalho (faixa com a cor do país e carimbo).
            egui::Frame::new().fill(cor_pais.gamma_multiply(0.55)).inner_margin(egui::Margin::symmetric(14, 10)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(nome_pais.to_uppercase()).color(egui::Color32::WHITE).strong());
                    ui.label(egui::RichText::new(format!("• {data}")).color(egui::Color32::WHITE));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("URGENTE • JOGO PAUSADO").color(CARIMBO).background_color(PAPEL).strong());
                    });
                });
            });
            egui::Frame::new().inner_margin(egui::Margin::symmetric(18, 12)).show(ui, |ui| {
                if let (Some(el), Some(r)) = (elenco.as_deref(), retratos.as_deref_mut()) {
                    // Retrato do chefe de Estado do país afetado (tag do legado -> política).
                    super::personagens_ui::retrato_chefe(ui, el, r, &super::politica_sim::tag_politica(&pend.pais), 64.0);
                    ui.add_space(4.0);
                }
                ui.label(
                    egui::RichText::new(evento.titulo.to_uppercase())
                        .family(ui_refinada::familia_oswald(ui.ctx()))
                        .color(TINTA)
                        .size(22.0)
                        .strong(),
                );
                ui.add_space(6.0);
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    ui.label(egui::RichText::new(&evento.descricao).color(TINTA));
                });
                ui.add_space(10.0);
                ui.separator();
                for (i, op) in evento.opcoes.iter().enumerate() {
                    let letra = (b'A' + (i as u8 % 26)) as char;
                    let r = ui.add_sized(
                        [ui.available_width(), 34.0],
                        egui::Button::new(egui::RichText::new(format!("{letra}. {}", op.texto)).color(TINTA))
                            .fill(ui_refinada::menu::PAPEL_CARD)
                            .stroke(egui::Stroke::new(1.0, ui_refinada::menu::VERMELHO))
                            .wrap(),
                    );
                    let r = r.on_hover_ui(|ui| {
                        ui.set_max_width(380.0);
                        if !op.descricao_resultado.is_empty() {
                            ui.label(&op.descricao_resultado);
                            ui.separator();
                        }
                        for m in &op.efeitos {
                            let bom = m.valor >= 0.0;
                            ui.colored_label(if bom { ui_refinada::menu::SUCESSO } else { ui_refinada::menu::PERIGO }, descrever_efeito(m));
                        }
                    });
                    if r.clicked() {
                        escolhida = Some(i);
                    }
                    ui.add_space(2.0);
                }
                ui.add_space(4.0);
                let rodape = if faltam > 0 { format!("Passe o mouse sobre uma opção para ver os efeitos. (+{faltam} na fila)") } else { "Passe o mouse sobre uma opção para ver os efeitos.".to_string() };
                ui.label(egui::RichText::new(rodape).color(TINTA_SUAVE).small());
            });
        });

    if let Some(i) = escolhida {
        let evento = evento.clone();
        let est = &mut ev.estado;
        aplicar_opcao(&mut mundo.0, est, &evento, i, &pend.pais);
        if ev.estado.pendentes.is_empty() && ev.retomar {
            relogio.pausado = false;
            ev.retomar = false;
        }
    }
    Ok(())
}

fn jornal(mut contextos: EguiContexts, ev: Res<EstadoEv>) -> Result {
    if !ev.jornal_aberto || ev.estado.jornal.is_empty() {
        return Ok(());
    }
    ui_refinada::janela_estilo_menu("Jornal (J)")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .default_width(340.0)
        .resizable(false)
        .show(contextos.ctx_mut()?, |ui| {
            ui_refinada::decorar_janela(ui);
            ui_refinada::cabecalho_painel(ui, "JORNAL E TELEGRAMAS (J)");
            for n in ev.estado.jornal.iter().rev().take(6) {
                ui.label(egui::RichText::new(format!("{:02}/{:02}/{}", n.data.2, n.data.1, n.data.0)).small().color(TINTA_SUAVE));
                ui.label(egui::RichText::new(&n.texto).color(TINTA));
                ui.separator();
            }
        });
    Ok(())
}
