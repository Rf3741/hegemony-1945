//! Menu de pausa (Esc), janela de Configurações e botão "Rankings", no estilo do card de país:
//! papel #E8DEC8, borda vermelha #8C251E, tinta escura.
use super::config::{
    CANAIS, ConfigJogo, ModoTela, QualidadeGrafica, RESOLUCOES,
};
use super::estatisticas::{AbaLedger, RecursoLedger};
use super::menu_saves::JanelaSaves;
use super::menu_ui::TelaMenu;
use crate::estado::AppState;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use ui_refinada::menu::{CINZA, PAPEL, TINTA, TINTA_SUAVE, VERMELHO};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AbaConfig {
    #[default]
    Video,
    Audio,
    Graficos,
    Jogo,
    Interface,
    Controles,
}

impl AbaConfig {
    const TODAS: [(Self, &'static str); 6] = [
        (Self::Video, "Vídeo"),
        (Self::Audio, "Áudio"),
        (Self::Graficos, "Gráficos"),
        (Self::Jogo, "Jogo"),
        (Self::Interface, "Interface"),
        (Self::Controles, "Controles"),
    ];
}

#[derive(Resource, Default)]
pub struct JanelaConfig {
    pub aberta: bool,
    pub aba: AbaConfig,
}

impl JanelaConfig {
    pub fn abrir(&mut self) {
        self.aberta = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirmar {
    MenuPrincipal,
    Sair,
}

pub struct MenuPausaPlugin;

impl Plugin for MenuPausaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JanelaConfig>()
            .add_systems(Update, tecla_audio)
            .add_systems(
                EguiPrimaryContextPass,
                (
                    tela_pausa.run_if(in_state(AppState::Pausado)),
                    botao_rankings.run_if(in_state(AppState::Jogando)),
                    janela_config.run_if(|e: Option<Res<State<AppState>>>| {
                        matches!(
                            e.as_deref().map(|s| *s.get()),
                            Some(AppState::Menu | AppState::Jogando | AppState::Pausado)
                        )
                    }),
                ),
            );
    }
}

/// Tecla 9: abre as Configurações direto na aba Áudio (antiga janela de áudio).
fn tecla_audio(
    teclas: Res<ButtonInput<KeyCode>>,
    estado: Option<Res<State<AppState>>>,
    mut j: ResMut<JanelaConfig>,
) {
    if (teclas.just_pressed(KeyCode::Digit9) || teclas.just_pressed(KeyCode::Numpad9))
        && matches!(
            estado.as_deref().map(|s| *s.get()),
            Some(AppState::Menu | AppState::Jogando | AppState::Pausado)
        )
    {
        if j.aberta && j.aba == AbaConfig::Audio {
            j.aberta = false;
        } else {
            j.aberta = true;
            j.aba = AbaConfig::Audio;
        }
    }
}

fn botao(ui: &mut egui::Ui, texto: &str, fill: egui::Color32, tam: egui::Vec2) -> bool {
    ui.add(
        egui::Button::new(egui::RichText::new(texto).size(20.0).color(PAPEL))
            .fill(fill)
            .min_size(tam),
    )
    .clicked()
}

fn botao_menor(ui: &mut egui::Ui, texto: &str, ativo: bool) -> bool {
    let fill = if ativo { VERMELHO } else { CINZA };
    ui.add(
        egui::Button::new(egui::RichText::new(texto).size(15.0).color(PAPEL))
            .fill(fill)
            .min_size(egui::vec2(60.0, 28.0)),
    )
    .clicked()
}

/// Estilo escuro-sobre-papel para widgets dentro do card.
fn estilo_papel(ui: &mut egui::Ui) {
    let v = ui.visuals_mut();
    v.override_text_color = Some(TINTA);
    let fundo = egui::Color32::from_rgb(0xD2, 0xC5, 0xA6);
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        w.fg_stroke.color = TINTA;
        w.bg_stroke.color = TINTA_SUAVE;
        w.bg_fill = fundo;
        w.weak_bg_fill = fundo;
    }
    v.widgets.noninteractive.fg_stroke.color = TINTA;
    v.selection.bg_fill = VERMELHO;
    v.slider_trailing_fill = true;
}

fn card(ctx: &egui::Context, id: &str, largura: f32, conteudo: impl FnOnce(&mut egui::Ui)) {
    egui::Window::new(id)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::NONE
                .fill(PAPEL)
                .stroke(egui::Stroke::new(2.0, VERMELHO))
                .inner_margin(24.0),
        )
        .show(ctx, |ui| {
            ui.set_width(largura);
            ui_refinada::decorar_janela(ui);
            estilo_papel(ui);
            conteudo(ui);
        });
}

/// Escurece tudo atrás e bloqueia cliques no jogo.
fn escurecer(ctx: &egui::Context) {
    egui::Area::new(egui::Id::new("escurecer_pausa"))
        .order(egui::Order::Background)
        .fixed_pos(egui::pos2(0.0, 0.0))
        .show(ctx, |ui| {
            let r = ctx.viewport_rect();
            ui.allocate_rect(r, egui::Sense::click_and_drag());
            ui.painter()
                .rect_filled(r, 0.0, egui::Color32::from_black_alpha(170));
        });
}

fn tela_pausa(
    mut contextos: EguiContexts,
    mut prox: ResMut<NextState<AppState>>,
    mut saves: ResMut<JanelaSaves>,
    mut config: ResMut<JanelaConfig>,
    mut ledger: ResMut<RecursoLedger>,
    mut menu: ResMut<TelaMenu>,
    mut sair: MessageWriter<AppExit>,
    mut confirmar: Local<Option<Confirmar>>,
) -> Result {
    let ctx = contextos.ctx_mut()?.clone();
    escurecer(&ctx);
    if config.aberta || saves.aberta || ledger.aberto {
        *confirmar = None;
        return Ok(());
    }
    card(&ctx, "menu_pausa", 340.0, |ui| {
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("PAUSADO")
                    .size(34.0)
                    .strong()
                    .color(VERMELHO),
            );
            ui.label(
                egui::RichText::new("Hegemony: 1945")
                    .size(15.0)
                    .color(TINTA_SUAVE),
            );
        });
        ui.add_space(12.0);
        let tam = egui::vec2(ui.available_width(), 40.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        if botao(ui, "Continuar (Esc)", VERMELHO, tam) {
            prox.set(AppState::Jogando);
        }
        if botao(ui, "Salvar / Carregar", CINZA, tam) {
            saves.abrir(false);
        }
        if botao(ui, "Estatísticas e Rankings (7)", CINZA, tam) {
            ledger.aberto = true;
            ledger.aba_ativa = AbaLedger::RankingsCategoria;
        }
        if botao(ui, "Configurações", CINZA, tam) {
            config.abrir();
        }
        if botao(ui, "Menu principal", CINZA, tam) {
            *confirmar = Some(Confirmar::MenuPrincipal);
        }
        if botao(ui, "Sair do jogo", CINZA, tam) {
            *confirmar = Some(Confirmar::Sair);
        }
        if let Some(c) = *confirmar {
            ui.add_space(4.0);
            let q = match c {
                Confirmar::MenuPrincipal => "Voltar ao menu? O progresso não salvo se perde.",
                Confirmar::Sair => "Sair do jogo? O progresso não salvo se perde.",
            };
            ui.label(egui::RichText::new(q).strong().color(VERMELHO));
            ui.horizontal(|ui| {
                if botao_menor(ui, "Confirmar", true) {
                    match c {
                        Confirmar::MenuPrincipal => {
                            *menu = TelaMenu::Titulo;
                            prox.set(AppState::Menu);
                        }
                        Confirmar::Sair => {
                            sair.write(AppExit::Success);
                        }
                    }
                    *confirmar = None;
                }
                if botao_menor(ui, "Cancelar", false) {
                    *confirmar = None;
                }
            });
        }
    });
    Ok(())
}

/// Botão fixo no canto inferior esquerdo durante o jogo.
fn botao_rankings(
    mut contextos: EguiContexts,
    mut ledger: ResMut<RecursoLedger>,
    mut config: ResMut<JanelaConfig>,
) -> Result {
    egui::Area::new(egui::Id::new("botao_rankings")).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0]).show(contextos.ctx_mut()?, |ui| {
        ui.horizontal(|ui| {
            let fam = ui_refinada::familia_oswald(ui.ctx());
            let b = egui::Button::new(
                egui::RichText::new("Rankings (7)")
                    .family(fam.clone())
                    .strong()
                    .size(16.0)
                    .color(PAPEL),
            )
            .fill(VERMELHO)
            .stroke(egui::Stroke::new(1.5, TINTA));
            if ui.add(b).on_hover_text("Estatísticas e rankings globais: comércio, PIB, população, força militar...").clicked() {
                ledger.aberto = !ledger.aberto;
                if ledger.aberto {
                    ledger.aba_ativa = AbaLedger::RankingsCategoria;
                }
            }
            let b_cfg = egui::Button::new(
                egui::RichText::new("Configurações")
                    .family(fam)
                    .size(15.0)
                    .color(TINTA),
            )
            .fill(PAPEL)
            .stroke(egui::Stroke::new(1.0, VERMELHO));
            if ui.add(b_cfg).clicked() {
                config.abrir();
            }
        });
    });
    Ok(())
}

fn linha(ui: &mut egui::Ui, rotulo: &str, conteudo: impl FnOnce(&mut egui::Ui)) {
    ui.label(egui::RichText::new(rotulo).color(TINTA_SUAVE));
    ui.horizontal(|ui| conteudo(ui));
    ui.end_row();
}

fn janela_config(
    mut contextos: EguiContexts,
    mut j: ResMut<JanelaConfig>,
    mut cfg: ResMut<ConfigJogo>,
) -> Result {
    if !j.aberta {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?.clone();
    let mut c = cfg.clone();
    let mut fechar = false;
    let mut aba = j.aba;
    card(&ctx, "janela_config", 700.0, |ui| {
        ui.label(
            egui::RichText::new("CONFIGURAÇÕES")
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(28.0)
                .strong()
                .color(VERMELHO),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (a, nome) in AbaConfig::TODAS {
                if botao_menor(ui, nome, aba == a) {
                    aba = a;
                }
            }
        });
        ui.separator();
        ui.set_min_height(330.0);
        egui::Grid::new("grade_config").num_columns(2).spacing([24.0, 12.0]).show(ui, |ui| match aba {
            AbaConfig::Video => {
                linha(ui, "Tela", |ui| {
                    for (m, nome) in [(ModoTela::Janela, "Janela"), (ModoTela::SemBordas, "Tela cheia (sem bordas)"), (ModoTela::Exclusiva, "Tela cheia exclusiva")] {
                        if botao_menor(ui, nome, c.video.modo == m) {
                            c.video.modo = m;
                        }
                    }
                });
                linha(ui, "Resolução (modo janela)", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (l, a) in RESOLUCOES {
                            if botao_menor(ui, &format!("{l}x{a}"), c.video.resolucao == (l, a)) {
                                c.video.resolucao = (l, a);
                            }
                        }
                    });
                });
                linha(ui, "VSync", |ui| {
                    ui.checkbox(&mut c.video.vsync, "sincronizar com o monitor");
                });
                linha(ui, "Escala da interface", |ui| {
                    ui.add(egui::Slider::new(&mut c.video.escala_ui, 0.75..=2.0).fixed_decimals(2));
                });
                linha(ui, "Limite de FPS", |ui| {
                    for (v, nome) in [(0, "Sem limite"), (30, "30"), (60, "60"), (120, "120"), (144, "144")] {
                        if botao_menor(ui, nome, c.video.limite_fps == v) {
                            c.video.limite_fps = v;
                        }
                    }
                });
                linha(ui, "", |ui| {
                    ui.label(egui::RichText::new("F11 ou Alt+Enter alternam a tela cheia.").size(13.0).color(TINTA_SUAVE));
                });
            }
            AbaConfig::Audio => {
                for (i, (_, nome)) in CANAIS.iter().enumerate() {
                    linha(ui, nome, |ui| {
                        ui.add(egui::Slider::new(&mut c.audio.volumes[i], 0.0..=1.0).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)));
                        ui.checkbox(&mut c.audio.mudos[i], "mudo");
                    });
                }
            }
            AbaConfig::Graficos => {
                linha(ui, "Qualidade geral", |ui| {
                    for (q, nome) in [
                        (QualidadeGrafica::Baixo, "Baixo"),
                        (QualidadeGrafica::Medio, "Médio"),
                        (QualidadeGrafica::Alto, "Alto"),
                        (QualidadeGrafica::Ultra, "Ultra"),
                    ] {
                        if botao_menor(ui, nome, c.graficos.qualidade == q) {
                            c.graficos.qualidade = q;
                        }
                    }
                });
                linha(ui, "Hillshade", |ui| {
                    ui.add(egui::Slider::new(&mut c.graficos.hillshade, 0.0..=1.0).fixed_decimals(2));
                });
                linha(ui, "Ondas e brilho da água", |ui| {
                    ui.add(egui::Slider::new(&mut c.graficos.ondas, 0.0..=0.25).fixed_decimals(2));
                });
                linha(ui, "Sombra das costas", |ui| {
                    ui.add(egui::Slider::new(&mut c.graficos.sombra_costas, 0.0..=1.0).fixed_decimals(2));
                });
                linha(ui, "Recursos opcionais", |ui| {
                    ui.checkbox(&mut c.graficos.nuvens, "nuvens");
                    ui.checkbox(&mut c.graficos.dia_noite, "dia/noite");
                    ui.checkbox(&mut c.graficos.sombras, "sombras");
                    ui.checkbox(&mut c.graficos.fumaca, "fumaça");
                    ui.checkbox(&mut c.graficos.agua_animada, "água animada");
                });
                linha(ui, "Intensidade da noite", |ui| {
                    ui.add(egui::Slider::new(&mut c.video.intensidade_noite, 0.0..=1.0).fixed_decimals(2));
                });
                linha(ui, "", |ui| {
                    ui.label(egui::RichText::new("Baixo mantém o mapa atual; Ultra ativa normais e sombras em resolução cheia.").size(13.0).color(TINTA_SUAVE));
                });
            }
            AbaConfig::Jogo => {
                linha(ui, "Velocidade inicial", |ui| {
                    for n in 1..=5u8 {
                        if botao_menor(ui, &n.to_string(), c.partida.velocidade_inicial == n) {
                            c.partida.velocidade_inicial = n;
                        }
                    }
                });
                linha(ui, "Autosave a cada", |ui| {
                    for (m, nome) in [(0, "Desligado"), (1, "1 mês"), (3, "3 meses"), (6, "6 meses"), (12, "1 ano")] {
                        if botao_menor(ui, nome, c.partida.autosave_meses == m) {
                            c.partida.autosave_meses = m;
                        }
                    }
                });
                linha(ui, "Eventos", |ui| {
                    ui.checkbox(&mut c.partida.pausar_em_eventos, "pausar o tempo em eventos com decisão");
                });
                linha(ui, "Tutorial", |ui| {
                    ui.checkbox(&mut c.partida.mostrar_tutorial, "mostrar a dica \"faça o Tutorial\" no menu principal");
                });
            }
            AbaConfig::Interface => {
                linha(ui, "Tamanho da fonte", |ui| {
                    ui.add(egui::Slider::new(&mut c.interface.fonte, 0.8..=1.6).fixed_decimals(2));
                });
                linha(ui, "Tooltips", |ui| {
                    ui.checkbox(&mut c.interface.tooltips, "mostrar dicas ao passar o mouse");
                });
            }
            AbaConfig::Controles => {
                for (k, v) in [
                    ("Esc", "Pausa / fecha a janela aberta"),
                    ("Espaço", "Pausar ou continuar o tempo"),
                    ("+  /  −", "Aumentar / diminuir a velocidade"),
                    ("W A S D / setas", "Mover a câmera"),
                    ("Roda do mouse", "Zoom"),
                    ("Arrastar", "Mover o mapa"),
                    ("1 a 8", "País, mercado, comércio, diplomacia, tecnologia, guerra, estatísticas, enciclopédia"),
                    ("9", "Configurações de áudio"),
                    ("B  /  J  /  V  /  M", "Construção, jornal, construções no mapa, modos de mapa"),
                    ("Ctrl+S  /  Ctrl+L", "Salvar rápido / carregar"),
                    ("F11  /  Alt+Enter", "Alternar tela cheia"),
                ] {
                    linha(ui, k, |ui| {
                        ui.label(egui::RichText::new(v).color(TINTA));
                    });
                }
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if botao_menor(ui, "Voltar", false) {
                fechar = true;
            }
            if botao_menor(ui, "Restaurar padrão", false) {
                c = ConfigJogo::default();
            }
            ui.label(
                egui::RichText::new(
                    "As mudanças valem na hora e ficam em ~/.config/hegemony1945/config.ron",
                )
                .size(12.0)
                .color(TINTA_SUAVE),
            );
        });
    });
    j.aba = aba;
    if fechar {
        j.aberta = false;
    }
    let c = c.normalizada();
    if c != *cfg {
        *cfg = c;
    }
    Ok(())
}
