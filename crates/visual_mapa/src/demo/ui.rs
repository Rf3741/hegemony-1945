//! Interface do usuário (UI) com bevy_egui: HUD temático vintage, controles de tempo, modos de mapa e diagnóstico FPS.

use crate::cidade::CidadeVisual;
use crate::era::GerenciadorEras;
use crate::industria::{InstalacaoIndustrial, ParticulaFumaca};
use crate::lod::EstadoLod;
use crate::mapa::{EstadoModoMapa, ModoMapa};
use super::tempo::TempoSimulacao;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

/// Estado da interface gráfica.
#[derive(Resource, Debug, Clone)]
pub struct EstadoUi {
    pub exibir_fps: bool,
    pub cidade_selecionada_id: Option<String>,
}

impl Default for EstadoUi {
    fn default() -> Self {
        Self {
            exibir_fps: true,
            cidade_selecionada_id: None,
        }
    }
}

/// Formata inteiros com separadores de milhar no padrão brasileiro (pontos).
fn formatar_numero(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    let bytes = s.as_bytes();
    let len = bytes.len();
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(b as char);
    }
    out
}

/// Sistema de controle de atalhos de teclado para a interface (F1 = toggle FPS, N = noite, Espaço = pausa).
pub fn atalhos_teclado_ui_sistema(
    keys: Res<ButtonInput<KeyCode>>,
    mut estado_ui: ResMut<EstadoUi>,
    mut tempo: ResMut<TempoSimulacao>,
    mut eras: ResMut<GerenciadorEras>,
    mut estado_mapa: ResMut<EstadoModoMapa>,
) {
    if keys.just_pressed(KeyCode::F1) {
        estado_ui.exibir_fps = !estado_ui.exibir_fps;
    }
    if keys.just_pressed(KeyCode::KeyN) {
        eras.modo_noturno = !eras.modo_noturno;
    }
    if keys.just_pressed(KeyCode::Space) {
        tempo.alternar_pausa();
    }
    if keys.just_pressed(KeyCode::Digit1) {
        estado_mapa.transicionar_para(ModoMapa::Politico);
    }
    if keys.just_pressed(KeyCode::Digit2) {
        estado_mapa.transicionar_para(ModoMapa::Industrial);
    }
    if keys.just_pressed(KeyCode::Digit3) {
        estado_mapa.transicionar_para(ModoMapa::Urbanizacao);
    }
}

/// Sistema principal de renderização da UI egui: barra superior, controles de tempo, modos de mapa e painel de diagnóstico.
pub fn renderizar_ui_sistema(
    mut contexts: EguiContexts,
    mut tempo: ResMut<TempoSimulacao>,
    mut eras: ResMut<GerenciadorEras>,
    mut estado_mapa: ResMut<EstadoModoMapa>,
    mut estado_ui: ResMut<EstadoUi>,
    estado_lod: Res<EstadoLod>,
    diagnostics: Res<DiagnosticsStore>,
    query_cidades: Query<&CidadeVisual>,
    query_industrias: Query<&InstalacaoIndustrial>,
    query_particulas: Query<&ParticulaFumaca>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    // 1. Barra Superior Principal (Estilo Gabinete Militar Vintage)
    egui::Panel::top("painel_superior_hud")
        .frame(
            egui::Frame::NONE
                .fill(egui::Color32::from_rgb(20, 23, 26))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 58, 65)))
                .inner_margin(8.0),
        )
        .show(&mut viewport_ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 4.0);

                // Título e Selo
                ui.colored_label(
                    egui::Color32::from_rgb(223, 177, 91),
                    "★ GUERRA FRIA: MAPA VIVO",
                );
                ui.separator();

                // Data e Era
                ui.label(
                    egui::RichText::new(format!("📅 {}", tempo.data_formatada()))
                        .monospace()
                        .color(egui::Color32::from_rgb(230, 236, 239)),
                );

                let era_info = eras.era_atual();
                let badge_cor = match eras.indice_atual {
                    0 => egui::Color32::from_rgb(180, 80, 60),
                    1 => egui::Color32::from_rgb(120, 130, 140),
                    2 => egui::Color32::from_rgb(140, 150, 160),
                    3 => egui::Color32::from_rgb(90, 130, 170),
                    _ => egui::Color32::from_rgb(108, 180, 238),
                };
                ui.colored_label(badge_cor, format!("[{}]", era_info.nome.to_uppercase()));

                ui.separator();

                // Controles de tempo
                let texto_pausa = if tempo.pausado { "▶ RETOMAR" } else { "⏸ PAUSAR" };
                if ui.button(texto_pausa).clicked() {
                    tempo.alternar_pausa();
                }

                if ui.selectable_label(tempo.velocidade == 1.0 && !tempo.modo_demonstracao, "1x").clicked() {
                    tempo.velocidade = 1.0;
                    tempo.modo_demonstracao = false;
                    tempo.pausado = false;
                }
                if ui.selectable_label(tempo.velocidade == 2.0 && !tempo.modo_demonstracao, "2x").clicked() {
                    tempo.velocidade = 2.0;
                    tempo.modo_demonstracao = false;
                    tempo.pausado = false;
                }
                if ui.selectable_label(tempo.velocidade == 5.0 && !tempo.modo_demonstracao, "5x").clicked() {
                    tempo.velocidade = 5.0;
                    tempo.modo_demonstracao = false;
                    tempo.pausado = false;
                }
                if ui.selectable_label(tempo.velocidade == 10.0 && !tempo.modo_demonstracao, "10x").clicked() {
                    tempo.velocidade = 10.0;
                    tempo.modo_demonstracao = false;
                    tempo.pausado = false;
                }

                let btn_demo = ui.selectable_label(
                    tempo.modo_demonstracao,
                    "🚀 ACELERAR DEMO (1945 → 1991)",
                );
                if btn_demo.clicked() {
                    tempo.modo_demonstracao = !tempo.modo_demonstracao;
                    if tempo.modo_demonstracao {
                        tempo.pausado = false;
                    }
                }

                ui.separator();

                // Modos de mapa
                ui.label("MODO:");
                if ui.selectable_label(estado_mapa.modo_alvo == ModoMapa::Politico, "1: POLÍTICO").clicked() {
                    estado_mapa.transicionar_para(ModoMapa::Politico);
                }
                if ui.selectable_label(estado_mapa.modo_alvo == ModoMapa::Industrial, "2: INDUSTRIAL").clicked() {
                    estado_mapa.transicionar_para(ModoMapa::Industrial);
                }
                if ui.selectable_label(estado_mapa.modo_alvo == ModoMapa::Urbanizacao, "3: URBANIZAÇÃO").clicked() {
                    estado_mapa.transicionar_para(ModoMapa::Urbanizacao);
                }

                ui.separator();

                // Botão de Noite
                let texto_noite = if eras.modo_noturno { "🌙 NOITE [LIGADA]" } else { "☀️ DIA [DESLIGADA]" };
                if ui.button(texto_noite).clicked() {
                    eras.modo_noturno = !eras.modo_noturno;
                }

                // Toggle F1
                if ui.selectable_label(estado_ui.exibir_fps, "F1: DIAGNÓSTICO").clicked() {
                    estado_ui.exibir_fps = !estado_ui.exibir_fps;
                }
            });

            // Barra de scrubber temporal (permite arrastar o ano diretamente)
            ui.horizontal(|ui| {
                ui.label("LINHA DO TEMPO:");
                let mut ano_slider = tempo.ano_continuo;
                let slider = ui.add(
                    egui::Slider::new(&mut ano_slider, 1945.0..=1991.0)
                        .show_value(false)
                        .text(format!("{:.1}", tempo.ano_continuo)),
                );
                if slider.changed() {
                    tempo.definir_ano(ano_slider);
                }

                ui.label(format!("Evolução Global: {:.0}%", tempo.fator_evolucao_global() * 100.0));
            });
        });

    // 2. Painel de FPS e Diagnóstico de Performance (Ativado por F1)
    if estado_ui.exibir_fps {
        egui::Window::new("📊 Desempenho & Estatísticas (F1)")
            .default_pos(egui::pos2(12.0, 75.0))
            .default_width(260.0)
            .resizable(false)
            .collapsible(true)
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_premultiplied(16, 19, 22, 235))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 72, 82)))
                    .inner_margin(10.0),
            )
            .show(ctx, |ui| {
                // Cálculo de FPS
                let fps_val = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FPS)
                    .and_then(|d| d.smoothed())
                    .unwrap_or(60.0);

                let cor_fps = if fps_val >= 55.0 {
                    egui::Color32::from_rgb(76, 175, 80) // Verde
                } else if fps_val >= 30.0 {
                    egui::Color32::from_rgb(255, 193, 7) // Amarelo
                } else {
                    egui::Color32::from_rgb(244, 67, 54) // Vermelho
                };

                ui.horizontal(|ui| {
                    ui.label("Taxa de Quadros:");
                    ui.colored_label(
                        cor_fps,
                        egui::RichText::new(format!("{:.1} FPS", fps_val)).strong().monospace(),
                    );
                });

                let frame_time_ms = if fps_val > 0.0 { 1000.0 / fps_val } else { 16.6 };
                ui.horizontal(|ui| {
                    ui.label("Tempo de Quadro:");
                    ui.monospace(format!("{:.2} ms", frame_time_ms));
                });

                ui.separator();

                // Contagens de entidades simuladas e renderizadas
                let total_cidades = query_cidades.iter().count();
                let total_industrias = query_industrias.iter().count();
                let industrias_ativas = query_industrias.iter().filter(|i| i.escala_animada > 0.1).count();
                let particulas_fumaca = query_particulas.iter().count();

                ui.horizontal(|ui| {
                    ui.label("Cidades no Mundo:");
                    ui.monospace(format!("{}", total_cidades));
                });
                ui.horizontal(|ui| {
                    ui.label("Complexos Industriais:");
                    ui.monospace(format!("{} ({} ativas)", total_industrias, industrias_ativas));
                });
                ui.horizontal(|ui| {
                    ui.label("Partículas de Fumaça:");
                    ui.monospace(format!("{}", particulas_fumaca));
                });

                ui.separator();

                // Nível de LOD
                let (lod_nome, lod_desc) = match estado_lod.nivel_atual {
                    crate::lod::NivelLod::Global => ("Global", "Marcadores esféricos (otimização máxima)"),
                    crate::lod::NivelLod::Regional => ("Regional", "Malhas viárias e centros industriais"),
                    crate::lod::NivelLod::Local => ("Local", "Aglomerados de prédios e chaminés 3D"),
                };
                ui.horizontal(|ui| {
                    ui.label("LOD de Zoom:");
                    ui.colored_label(egui::Color32::from_rgb(106, 165, 222), lod_nome);
                });
                ui.small(lod_desc);
                ui.small(format!("Escala da Câmera: {:.2}x", estado_lod.escala_camera));

                ui.separator();
                ui.small("Controles: WASD ou Setas para Mover | Scroll para Zoom");
            });
    }

    // 3. Painel de Inspeção da Cidade Selecionada
    if let Some(ref cid_id) = estado_ui.cidade_selecionada_id {
        if let Some(cidade) = query_cidades.iter().find(|c| c.id == *cid_id) {
            egui::Window::new(format!("🏙 Dossiê Urbano: {}", cidade.nome))
                .default_pos(egui::pos2(12.0, 320.0))
                .default_width(280.0)
                .resizable(false)
                .frame(
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgb(24, 27, 29))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(65, 74, 82)))
                        .inner_margin(10.0),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Nação Soberana:");
                        ui.strong(&cidade.pais_tag);
                        if cidade.eh_capital {
                            ui.colored_label(egui::Color32::from_rgb(223, 177, 91), "★ CAPITAL");
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Nível de Desenvolvimento:");
                        ui.strong(format!("{}/8", cidade.nivel_atual_inteiro));
                    });

                    // Barra de progresso do nível atual
                    let prog_nivel = cidade.nivel_atual_fracionario.fract();
                    ui.add(egui::ProgressBar::new(prog_nivel).text(format!("{:.1}", cidade.nivel_atual_fracionario)));

                    ui.horizontal(|ui| {
                        ui.label("População Estimada:");
                        ui.monospace(format!("{} hab.", formatar_numero(cidade.populacao)));
                    });

                    ui.separator();
                    if ui.button("Fechar Dossiê").clicked() {
                        estado_ui.cidade_selecionada_id = None;
                    }
                });
        }
    }
}
