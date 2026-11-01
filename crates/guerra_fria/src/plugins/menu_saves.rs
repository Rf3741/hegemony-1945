//! Tela de salvar/carregar no estilo do card de escolha de país (papel, borda vermelha, texto escuro),
//! e aviso curto ("Jogo salvo", "Carregado").
use super::economy::RecursoMundo;
use super::saveload::{InfoSlot, PedirCarregar, PedirSalvar, Slot, Slots, SLOTS_MANUAIS, dir_saves, varrer};
use super::map::RaizProjeto;
use crate::estado::AppState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

const PAPEL: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xDE, 0xC8);
const VERMELHO: egui::Color32 = egui::Color32::from_rgb(0x8C, 0x25, 0x1E);
const TINTA: egui::Color32 = egui::Color32::from_rgb(0x17, 0x16, 0x14);
const TINTA_SUAVE: egui::Color32 = egui::Color32::from_rgb(0x5C, 0x58, 0x50);
const CINZA: egui::Color32 = egui::Color32::from_rgb(0x3A, 0x37, 0x32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Acao {
    Salvar,
    Apagar,
}

#[derive(Resource, Default)]
pub struct JanelaSaves {
    pub aberta: bool,
    /// Menu principal: só carrega.
    pub so_carregar: bool,
    confirmar: Option<(Acao, Slot)>,
}

impl JanelaSaves {
    pub fn abrir(&mut self, so_carregar: bool) {
        self.aberta = true;
        self.so_carregar = so_carregar;
        self.confirmar = None;
    }
}

pub struct MenuSavesPlugin;

impl Plugin for MenuSavesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JanelaSaves>().add_systems(
            EguiPrimaryContextPass,
            (janela_saves, aviso).run_if(|e: Option<Res<State<AppState>>>| matches!(e.as_deref().map(|s| *s.get()), Some(AppState::Menu | AppState::Jogando | AppState::Pausado))),
        );
    }
}

/// Data/hora local (UTC-3) a partir do relógio real.
fn data_real(t: std::time::SystemTime) -> String {
    let Ok(d) = t.duration_since(std::time::UNIX_EPOCH) else { return "—".into() };
    let seg = d.as_secs() as i64 - 3 * 3600;
    let dias = seg.div_euclid(86400);
    let resto = seg.rem_euclid(86400);
    let z = dias + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let ano = yoe + era * 400 + i64::from(mes <= 2);
    format!("{dia:02}/{mes:02}/{ano} {:02}:{:02}", resto / 3600, resto % 3600 / 60)
}

fn botao(ui: &mut egui::Ui, texto: &str, fill: egui::Color32, habilitado: bool) -> bool {
    ui.add_enabled(habilitado, egui::Button::new(egui::RichText::new(texto).size(16.0).color(PAPEL)).fill(fill).min_size(egui::vec2(110.0, 32.0))).clicked()
}

fn janela_saves(
    mut contextos: EguiContexts,
    mut j: ResMut<JanelaSaves>,
    mut slots: ResMut<Slots>,
    mundo: Res<RecursoMundo>,
    raiz: Res<RaizProjeto>,
    mut salvar: MessageWriter<PedirSalvar>,
    mut carregar: MessageWriter<PedirCarregar>,
    estado: Res<State<AppState>>,
) -> Result {
    if !j.aberta {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?.clone();
    if *estado.get() == AppState::Jogando && !j.so_carregar {
        // continua jogável por trás; nada a fazer
    }
    let so_carregar = j.so_carregar;
    let mut fechar = false;
    let mut nova_conf = j.confirmar;
    let mut acao_final: Option<(Acao, Slot)> = None;
    let mut carregar_slot: Option<Slot> = None;
    let ordem: Vec<Slot> = (1..=SLOTS_MANUAIS).map(Slot::Manual).chain([Slot::Auto, Slot::Rapido]).collect();
    egui::Window::new("saves")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(egui::Frame::NONE.fill(PAPEL).stroke(egui::Stroke::new(2.0, VERMELHO)).inner_margin(20.0))
        .show(&ctx, |ui| {
            ui.set_width(640.0);
            ui.label(egui::RichText::new(if so_carregar { "CARREGAR JOGO" } else { "SALVAR / CARREGAR" }).size(28.0).strong().color(VERMELHO));
            ui.add_space(8.0);
            for slot in ordem {
                let info: InfoSlot = slots.lista.iter().find(|(s, _)| *s == slot).map(|(_, i)| i.clone()).unwrap_or_default();
                egui::Frame::NONE.stroke(egui::Stroke::new(1.0, TINTA_SUAVE)).inner_margin(8.0).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(300.0);
                            ui.label(egui::RichText::new(slot.rotulo()).size(17.0).strong().color(TINTA));
                            match &info.meta {
                                Some(m) => {
                                    ui.horizontal(|ui| {
                                        if let Some(p) = mundo.paises.get(&m.pais) {
                                            let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                                            ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(p.cor.0, p.cor.1, p.cor.2));
                                            ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.0, TINTA), egui::StrokeKind::Inside);
                                            ui.label(egui::RichText::new(format!("{} ({})", p.nome, m.pais)).color(TINTA));
                                        } else {
                                            ui.label(egui::RichText::new(&m.pais).color(TINTA));
                                        }
                                    });
                                    ui.label(egui::RichText::new(format!("Data no jogo: {}", m.data)).color(CINZA));
                                    let real = info.modificado.map(data_real).unwrap_or_else(|| "—".into());
                                    ui.label(egui::RichText::new(format!("Salvo em: {real}  ({:.1} MB)", info.bytes as f64 / 1e6)).size(13.0).color(TINTA_SUAVE));
                                }
                                None => {
                                    ui.label(egui::RichText::new("vazio").italics().color(TINTA_SUAVE));
                                }
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let tem = info.meta.is_some();
                            if tem && botao(ui, "Apagar", CINZA, true) {
                                nova_conf = Some((Acao::Apagar, slot));
                            }
                            if tem && botao(ui, "Carregar", CINZA, true) {
                                carregar_slot = Some(slot);
                            }
                            if !so_carregar && !matches!(slot, Slot::Auto) && botao(ui, "Salvar aqui", VERMELHO, true) {
                                if tem {
                                    nova_conf = Some((Acao::Salvar, slot));
                                } else {
                                    acao_final = Some((Acao::Salvar, slot));
                                }
                            }
                        });
                    });
                    if let Some((a, s)) = nova_conf
                        && s == slot
                    {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            let q = if a == Acao::Salvar { "Sobrescrever este save?" } else { "Apagar este save? Não dá para desfazer." };
                            ui.label(egui::RichText::new(q).strong().color(VERMELHO));
                            if botao(ui, "Confirmar", VERMELHO, true) {
                                acao_final = Some((a, s));
                                nova_conf = None;
                            }
                            if botao(ui, "Cancelar", CINZA, true) {
                                nova_conf = None;
                            }
                        });
                    }
                });
                ui.add_space(4.0);
            }
            if !slots.mensagem.is_empty() {
                ui.label(egui::RichText::new(&slots.mensagem).size(13.0).color(TINTA_SUAVE));
            }
            ui.add_space(6.0);
            if botao(ui, "Voltar", CINZA, true) {
                fechar = true;
            }
        });
    j.confirmar = nova_conf;
    match acao_final {
        Some((Acao::Salvar, s)) => {
            salvar.write(PedirSalvar(s));
        }
        Some((Acao::Apagar, s)) => {
            let _ = std::fs::remove_file(dir_saves(&raiz.0).join(s.arquivo()));
            slots.lista = varrer(&raiz.0);
            slots.mensagem = format!("{} apagado.", s.rotulo());
        }
        None => {}
    }
    if let Some(s) = carregar_slot {
        carregar.write(PedirCarregar(s));
        fechar = true;
    }
    if fechar {
        j.aberta = false;
        j.confirmar = None;
    }
    Ok(())
}

/// Aviso curto no topo da tela, por ~2,5 s.
fn aviso(mut contextos: EguiContexts, slots: Res<Slots>, tempo: Res<Time>, mut visto: Local<(u32, f32)>) -> Result {
    if slots.aviso_id != visto.0 {
        *visto = (slots.aviso_id, 2.5);
    }
    if visto.1 <= 0.0 {
        return Ok(());
    }
    visto.1 -= tempo.delta_secs();
    let alfa = (visto.1.min(0.5) / 0.5).clamp(0.0, 1.0);
    let a = |c: egui::Color32| c.gamma_multiply(alfa);
    egui::Area::new(egui::Id::new("aviso_saves")).order(egui::Order::Tooltip).anchor(egui::Align2::CENTER_TOP, [0.0, 70.0]).interactable(false).show(contextos.ctx_mut()?, |ui| {
        egui::Frame::NONE.fill(a(PAPEL)).stroke(egui::Stroke::new(2.0, a(VERMELHO))).inner_margin(egui::Margin::symmetric(24, 10)).show(ui, |ui| {
            ui.label(egui::RichText::new(&slots.aviso).size(22.0).strong().color(a(VERMELHO)));
        });
    });
    Ok(())
}
