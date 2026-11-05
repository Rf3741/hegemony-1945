//! Tela de Guerra (6): frentes de combate, exércitos nacionais, pontuação de guerra e negociação de paz.

use super::economy::RecursoMundo;
use super::guerra_sim::{GuerraSim, PainelGuerra, acao_propor_paz, normalizar_tag};
use super::jogador::{Notificacoes, PaisJogador};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use sim_guerra::PosturaOperacional;

pub struct GuerraUiPlugin;

impl Plugin for GuerraUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            EguiPrimaryContextPass,
            tela_guerra
                .run_if(resource_exists::<PaisJogador>)
                .run_if(resource_exists::<GuerraSim>)
                .run_if(crate::estado::em_partida),
        );
    }
}

/// Barrazo de tabela: calha creme com enchimento sólido na cor dada.
fn barra_mini(ui: &mut egui::Ui, frac: f32, cor: egui::Color32) {
    ui_refinada::barra_trilho(ui, frac, cor, 84.0, 10.0);
}

fn tela_guerra(
    mut contextos: EguiContexts,
    mut painel: ResMut<PainelGuerra>,
    mut guerra_sim: ResMut<GuerraSim>,
    mundo: Res<RecursoMundo>,
    jogador: Res<PaisJogador>,
    mut notificacoes: ResMut<Notificacoes>,
    tempo: Res<Time<Real>>,
) -> Result {
    if !painel.aberto {
        return Ok(());
    }

    let ctx = contextos.ctx_mut()?;
    let eu = normalizar_tag(&jogador.0);

    let nomes: std::collections::HashMap<String, String> = mundo
        .0
        .paises
        .values()
        .map(|p| (normalizar_tag(&p.tag), p.nome.clone()))
        .collect();
    let nome = |t: &str| nomes.get(t).cloned().unwrap_or_else(|| t.to_string());

    let mut aberto = true;

    ui_refinada::janela_dossie("MINISTÉRIO DA GUERRA E FORÇAS ARMADAS (6)", None)
        .default_pos([20.0, 96.0])
        .default_size([980.0, 640.0])
        .show(ctx, |ui| {
            ui_refinada::faixa_titulo(ui, "Ministério da Guerra", Some("6"), Some(&mut aberto));
            ui.horizontal(|ui| {
                if ui_refinada::aba_glifo(ui, painel.aba == 0, Some("canhao"), "Guerras Ativas") { painel.aba = 0; }
                if ui_refinada::aba_glifo(ui, painel.aba == 1, Some("escudo"), "Frentes") { painel.aba = 1; }
                if ui_refinada::aba_glifo(ui, painel.aba == 2, Some("pessoa"), "Exércitos") { painel.aba = 2; }
                if ui_refinada::aba_glifo(ui, painel.aba == 3, Some("livro"), "Histórico") { painel.aba = 3; }
            });
            ui.add_space(4.0);
            ui.separator();

            match painel.aba {
                0 => {
                    // ---- ABA 0: GUERRAS ATIVAS ----
                    let guerras_ativas: Vec<_> = guerra_sim.0.guerras().iter().filter(|g| g.ativa).cloned().collect();

                    if guerras_ativas.is_empty() {
                        ui.label("O mundo encontra-se em estado de paz. Nenhuma guerra ativa.");
                        ui.label("Você pode justificar e declarar guerra a outros países através do painel de Diplomacia (4).");
                    } else {
                        egui::ScrollArea::vertical().id_salt("scroll_guerras").show(ui, |ui| {
                            for g in &guerras_ativas {
                                ui_refinada::moldura_cartao().show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&g.nome_pt)
                                                .family(ui_refinada::familia_oswald(ui.ctx()))
                                                .size(18.0)
                                                .strong()
                                                .color(ui_refinada::menu::TINTA),
                                        );
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.weak(format!("Início em {:02}/{:02}/{}", g.data_inicio.dia, g.data_inicio.mes, g.data_inicio.ano));
                                        });
                                    });
                                    ui_refinada::linha_decorativa(ui);

                                    ui.horizontal_wrapped(|ui| {
                                        ui.colored_label(ui_refinada::menu::PERIGO, egui::RichText::new("ATACANTES").small().strong());
                                        for a in &g.atacantes {
                                            ui.colored_label(ui_refinada::menu::PERIGO, format!("{} ({a})", nome(a)));
                                        }
                                        ui.colored_label(ui_refinada::menu::TINTA_SUAVE, egui::RichText::new("vs").small());
                                        ui.colored_label(ui_refinada::menu::AZUL_MARINHO, egui::RichText::new("DEFENSORES").small().strong());
                                        for d in &g.defensores {
                                            ui.colored_label(ui_refinada::menu::AZUL_MARINHO, format!("{} ({d})", nome(d)));
                                        }
                                    });

                                    // Barra e valor da pontuação de guerra
                                    let score = g.pontuacao_guerra;
                                    let cor_score = if score > 0.0 { ui_refinada::menu::SUCESSO } else if score < 0.0 { ui_refinada::menu::PERIGO } else { ui_refinada::menu::TINTA_SUAVE };
                                    ui_refinada::widgets::barra_status(
                                        ui,
                                        Some("Pontuação de guerra"),
                                        (score + 100.0) as f32,
                                        200.0,
                                        cor_score,
                                        220.0,
                                        Some(&format!("{score:+.1}%")),
                                    );

                                    ui.horizontal_wrapped(|ui| {
                                        ui.weak(format!("Baixas: {} × atacantes · {} × defensores", g.baixas_atacantes, g.baixas_defensores));
                                        ui.separator();
                                        ui.weak(format!("Batalhas vencidas: {} × {}", g.batalhas_vencidas_atacantes, g.batalhas_vencidas_defensores));
                                    });

                                    if g.capitulou_defensores {
                                        ui.colored_label(ui_refinada::menu::AVISO, egui::RichText::new("DEFENSORES CAPITULARAM — rendição incondicional").strong());
                                    } else if g.capitulou_atacantes {
                                        ui.colored_label(ui_refinada::menu::AVISO, egui::RichText::new("ATACANTES CAPITULARAM — rendição incondicional").strong());
                                    }

                                    // Ações de paz para o jogador beligerante
                                    if g.participa(&eu) {
                                        ui_refinada::linha_decorativa(ui);
                                        ui.label(
                                            egui::RichText::new("TRATADO DE PAZ")
                                                .family(ui_refinada::familia_oswald(ui.ctx()))
                                                .size(15.0)
                                                .strong()
                                                .color(ui_refinada::menu::AZUL_MARINHO),
                                        );
                                        let vencedor = if g.pontuacao_guerra >= 35.0 {
                                            g.atacantes.contains(&eu)
                                        } else if g.pontuacao_guerra <= -35.0 {
                                            g.defensores.contains(&eu)
                                        } else {
                                            false
                                        };
                                        if vencedor {
                                            ui.label("Você venceu: a proposta padrão anexa as regiões ocupadas e exige reparações.");
                                            if ui_refinada::botao_estilo_menu(ui, "Oferecer paz automática (proposta padrão)").clicked() {
                                                match acao_propor_paz(&mut guerra_sim, &g.id, &eu, true, true) {
                                                    Ok(tratado) => {
                                                        painel.mensagem = format!("Paz automática assinada: {} cláusulas.", tratado.clausulas_executadas.len());
                                                        notificacoes.push(
                                                            Notificacoes::mensagem_modelada(
                                                                &format!("PAZ: {} termina.", g.nome_pt),
                                                                &nome(&eu),
                                                            ),
                                                            tempo.elapsed_secs_f64(),
                                                        );
                                                        painel.repintar = true;
                                                    }
                                                    Err(e) => painel.mensagem = e,
                                                }
                                            }
                                        }
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut painel.proposta_anexar, "Anexar províncias ocupadas");
                                            ui.checkbox(&mut painel.proposta_reparacoes, "Exigir reparações de guerra");

                                            if ui_refinada::botao_estilo_menu(ui, "Propor e Assinar Tratado de Paz").clicked() {
                                                match acao_propor_paz(
                                                    &mut guerra_sim,
                                                    &g.id,
                                                    &eu,
                                                    painel.proposta_anexar,
                                                    painel.proposta_reparacoes,
                                                ) {
                                                    Ok(tratado) => {
                                                        painel.mensagem = format!(
                                                            "Tratado de paz assinado com sucesso! Cláusulas executadas: {}",
                                                            tratado.clausulas_executadas.len()
                                                        );
                                                        notificacoes.push(
                                                            Notificacoes::mensagem_modelada(
                                                                &format!("PAZ: {} termina.", g.nome_pt),
                                                                &nome(&eu),
                                                            ),
                                                            tempo.elapsed_secs_f64(),
                                                        );
                                                        painel.repintar = true;
                                                    }
                                                    Err(e) => {
                                                        painel.mensagem = e;
                                                    }
                                                }
                                            }
                                        });
                                    }
                                });
                                ui.add_space(8.0);
                            }
                        });
                    }
                }
                1 => {
                    // ---- ABA 1: FRENTES OPERACIONAIS ----
                    let frentes = guerra_sim.0.frentes().to_vec();
                    if frentes.is_empty() {
                        ui.label("Nenhuma frente de combate ativa no momento.");
                    } else {
                        egui::ScrollArea::vertical().id_salt("scroll_frentes").show(ui, |ui| {
                            for f in &frentes {
                                ui_refinada::moldura_cartao().show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(&f.nome_pt)
                                                .family(ui_refinada::familia_oswald(ui.ctx()))
                                                .size(16.0)
                                                .strong()
                                                .color(ui_refinada::menu::TINTA),
                                        );
                                        let cor_status = if f.ativa { ui_refinada::menu::SUCESSO } else { ui_refinada::menu::CINZA_DESATIVADO };
                                        ui.colored_label(cor_status, egui::RichText::new(if f.ativa { "EM MOVIMENTO" } else { "PARADA" }).small().strong());
                                    });
                                    ui_refinada::linha_decorativa(ui);
                                    ui.horizontal_wrapped(|ui| {
                                        ui_refinada::chapa(ui, "canhao", "Lado A", &nome(&f.lado_a_tag), Some(ui_refinada::menu::PERIGO));
                                        ui_refinada::chapa(ui, "escudo", "Lado B", &nome(&f.lado_b_tag), Some(ui_refinada::menu::AZUL_MARINHO));
                                        ui_refinada::chapa(ui, "torre", "Pares de fronteira", &format!("{}", f.pares_fronteira.len()), None);
                                        ui_refinada::chapa(ui, "caixa", "Regiões", &format!("{} × {}", f.regioes_lado_a.len(), f.regioes_lado_b.len()), None);
                                    });
                                });
                                ui.add_space(6.0);
                            }
                        });
                    }
                }
                2 => {
                    // ---- ABA 2: EXÉRCITOS E DIVISÕES ----
                    let exercitos: Vec<_> = guerra_sim.0.exercitos().iter().filter(|e| e.pais_tag == eu).cloned().collect();
                    let divisoes: Vec<_> = guerra_sim.0.divisoes().to_vec();
                    let comandantes: Vec<_> = guerra_sim.0.cenario.comandantes.clone();

                    ui.label(
                        egui::RichText::new(format!("FORÇAS ARMADAS DE {}", nome(&eu).to_uppercase()))
                            .family(ui_refinada::familia_oswald(ui.ctx()))
                            .size(17.0)
                            .strong()
                            .color(ui_refinada::menu::AZUL_MARINHO),
                    );
                    ui_refinada::chapa(ui, "pessoa", "Exércitos em campo", &format!("{}", exercitos.len()), None);
                    ui.separator();

                    egui::ScrollArea::vertical().id_salt("scroll_exercitos").show(ui, |ui| {
                        for (e_idx, exe) in exercitos.iter().enumerate() {
                            ui_refinada::moldura_cartao().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui_refinada::glifo_ui(ui, "pessoa", 16.0, Some(ui_refinada::menu::AZUL_MARINHO));
                                    ui.label(
                                        egui::RichText::new(&exe.nome_pt)
                                            .family(ui_refinada::familia_oswald(ui.ctx()))
                                            .size(16.0)
                                            .strong()
                                            .color(ui_refinada::menu::TINTA),
                                    );
                                    if let Some(ref c_id) = exe.comandante_id {
                                        if let Some(c) = comandantes.iter().find(|cmd| &cmd.id == c_id) {
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.weak(format!("Log {} · Def {} · Atk {}", c.habilidade_logistica, c.habilidade_defesa, c.habilidade_ataque));
                                                ui.label(egui::RichText::new(format!("Gen. {}", c.nome_pt)).strong());
                                            });
                                        }
                                    } else {
                                        ui.weak("Comando Geral");
                                    }
                                });
                                ui_refinada::linha_decorativa(ui);

                                // Postura operacional do exército
                                ui.horizontal(|ui| {
                                    ui.label("Postura:");
                                    let mut postura = exe.postura;
                                    if ui.selectable_value(&mut postura, PosturaOperacional::AssaltoAgressivo, "Ofensiva").clicked() {
                                        if let Some(ex_mut) = guerra_sim.0.exercitos_mut().iter_mut().find(|e| e.id == exe.id) {
                                            ex_mut.postura = PosturaOperacional::AssaltoAgressivo;
                                        }
                                    }
                                    if ui.selectable_value(&mut postura, PosturaOperacional::OfensivaEquilibrada, "Equilibrada").clicked() {
                                        if let Some(ex_mut) = guerra_sim.0.exercitos_mut().iter_mut().find(|e| e.id == exe.id) {
                                            ex_mut.postura = PosturaOperacional::OfensivaEquilibrada;
                                        }
                                    }
                                    if ui.selectable_value(&mut postura, PosturaOperacional::DefesaEmProfundidade, "Defensiva").clicked() {
                                        if let Some(ex_mut) = guerra_sim.0.exercitos_mut().iter_mut().find(|e| e.id == exe.id) {
                                            ex_mut.postura = PosturaOperacional::DefesaEmProfundidade;
                                        }
                                    }
                                    if ui.selectable_value(&mut postura, PosturaOperacional::ContencaoGuarnicao, "Retirada").clicked() {
                                        if let Some(ex_mut) = guerra_sim.0.exercitos_mut().iter_mut().find(|e| e.id == exe.id) {
                                            ex_mut.postura = PosturaOperacional::ContencaoGuarnicao;
                                        }
                                    }
                                });

                                // Divisões subordinadas
                                ui.collapsing(format!("Divisões ({})", exe.divisoes_ids.len()), |ui| {
                                    let cab = |ui: &mut egui::Ui, t: &str| {
                                        ui.label(egui::RichText::new(t.to_uppercase()).small().strong().color(ui_refinada::menu::TINTA_SUAVE));
                                    };
                                    egui::Grid::new(ui.id().with(("grade_div", &exe.id)))
                                        .striped(true)
                                        .spacing([10.0, 4.0])
                                        .show(ui, |ui| {
                                            cab(ui, "Divisão");
                                            cab(ui, "Efetivos");
                                            cab(ui, "Organização");
                                            cab(ui, "Suprimento");
                                            cab(ui, "Região");
                                            ui.end_row();
                                            for d_id in &exe.divisoes_ids {
                                                if let Some(div) = divisoes.iter().find(|d| &d.id == d_id) {
                                                    ui.add_sized([210.0, 16.0], egui::Label::new(&div.nome_pt));
                                                    ui.add_sized([70.0, 16.0], egui::Label::new(egui::RichText::new(format!("{}", div.efetivos_atuais)).strong()));
                                                    ui.horizontal(|ui| {
                                                        barra_mini(ui, div.organizacao as f32, ui_refinada::menu::AZUL_MARINHO);
                                                        ui.weak(format!("{:.2}", div.organizacao));
                                                    });
                                                    let cor_sup = if div.suprimento_atual >= 0.6 {
                                                        ui_refinada::menu::SUCESSO
                                                    } else if div.suprimento_atual >= 0.3 {
                                                        ui_refinada::menu::AVISO
                                                    } else {
                                                        ui_refinada::menu::PERIGO
                                                    };
                                                    ui.horizontal(|ui| {
                                                        barra_mini(ui, div.suprimento_atual as f32, cor_sup);
                                                        ui.weak(format!("{:.0}%", div.suprimento_atual * 100.0));
                                                    });
                                                    ui.add_sized([110.0, 16.0], egui::Label::new(egui::RichText::new(&div.regiao_adm1).small()));
                                                    ui.end_row();
                                                }
                                            }
                                        });
                                });
                            });
                            ui.add_space(6.0);
                        }
                    });
                }
                3 => {
                    // ---- ABA 3: HISTÓRICO DE PAZ ----
                    let tratados = &guerra_sim.0.historico_tratados;
                    if tratados.is_empty() {
                        ui.label("Nenhum tratado de paz assinado até o momento.");
                    } else {
                        egui::ScrollArea::vertical().id_salt("scroll_tratados").show(ui, |ui| {
                            for (t_idx, t) in tratados.iter().enumerate() {
                                ui_refinada::moldura_cartao().show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui_refinada::glifo_ui(ui, "balanca", 15.0, None);
                                        ui.label(
                                            egui::RichText::new(format!("TRATADO Nº {} · {}", t_idx + 1, t.guerra_id.to_uppercase()))
                                                .family(ui_refinada::familia_oswald(ui.ctx()))
                                                .size(15.0)
                                                .strong()
                                                .color(ui_refinada::menu::TINTA),
                                        );
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.weak(format!("{:02}/{:02}/{}", t.data_assinatura.dia, t.data_assinatura.mes, t.data_assinatura.ano));
                                        });
                                    });
                                    ui_refinada::linha_decorativa(ui);
                                    ui.horizontal_wrapped(|ui| {
                                        ui.colored_label(ui_refinada::menu::SUCESSO, egui::RichText::new("Vencedores:").small().strong());
                                        ui.label(t.vencedores.iter().map(|s| nome(s)).collect::<Vec<_>>().join(", "));
                                    });
                                    ui.horizontal_wrapped(|ui| {
                                        ui.colored_label(ui_refinada::menu::PERIGO, egui::RichText::new("Vencidos:").small().strong());
                                        ui.label(t.derrotados.iter().map(|s| nome(s)).collect::<Vec<_>>().join(", "));
                                    });
                                    if !t.clausulas_executadas.is_empty() {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.colored_label(ui_refinada::menu::TINTA_SUAVE, egui::RichText::new("Cláusulas:").small().strong());
                                            ui.small(t.clausulas_executadas.iter().map(|c| format!("{:?}", c)).collect::<Vec<_>>().join(" · "));
                                        });
                                    }
                                });
                                ui.add_space(6.0);
                            }
                        });
                    }
                }
                _ => {}
            }

            if !painel.mensagem.is_empty() {
                ui.separator();
                ui.colored_label(ui_refinada::menu::SUCESSO, &painel.mensagem);
            }
        });

    if !aberto {
        painel.aberto = false;
    }

    Ok(())
}
