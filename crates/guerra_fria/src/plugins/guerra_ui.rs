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

    egui::Window::new("Ministério da Guerra e Forças Armadas (6)")
        .open(&mut aberto)
        .default_size([940.0, 620.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut painel.aba, 0, "Guerras Ativas");
                ui.selectable_value(&mut painel.aba, 1, "Frentes Operacionais");
                ui.selectable_value(&mut painel.aba, 2, "Exércitos e Divisões");
                ui.selectable_value(&mut painel.aba, 3, "Histórico de Paz");
            });
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
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.heading(&g.nome_pt);
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(format!("Início: {:02}/{:02}/{}", g.data_inicio.dia, g.data_inicio.mes, g.data_inicio.ano));
                                        });
                                    });

                                    ui.horizontal(|ui| {
                                        ui.strong("Atacantes:");
                                        for a in &g.atacantes {
                                            ui.colored_label(egui::Color32::from_rgb(220, 80, 80), format!("{} ({a})", nome(a)));
                                        }
                                        ui.strong(" vs Defensores:");
                                        for d in &g.defensores {
                                            ui.colored_label(egui::Color32::from_rgb(80, 140, 240), format!("{} ({d})", nome(d)));
                                        }
                                    });

                                    // Barra e valor da pontuação de guerra
                                    let score = g.pontuacao_guerra;
                                    let cor_score = if score > 0.0 { egui::Color32::LIGHT_GREEN } else if score < 0.0 { egui::Color32::LIGHT_RED } else { egui::Color32::GRAY };
                                    ui.horizontal(|ui| {
                                        ui.label("Pontuação de Guerra:");
                                        ui.colored_label(cor_score, format!("{score:+.1}%"));
                                        let progresso = ((score + 100.0) / 200.0).clamp(0.0, 1.0) as f32;
                                        ui.add(egui::ProgressBar::new(progresso).desired_width(200.0).text(format!("{score:+.1}%")));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(format!("Baixas: {} (Atacantes) vs {} (Defensores)", g.baixas_atacantes, g.baixas_defensores));
                                        ui.separator();
                                        ui.label(format!("Vitórias em Batalha: {} vs {}", g.batalhas_vencidas_atacantes, g.batalhas_vencidas_defensores));
                                    });

                                    if g.capitulou_defensores {
                                        ui.colored_label(egui::Color32::YELLOW, "⚠ Os defensores capitularam incondicionalmente!");
                                    } else if g.capitulou_atacantes {
                                        ui.colored_label(egui::Color32::YELLOW, "⚠ Os atacantes capitularam incondicionalmente!");
                                    }

                                    // Ações de paz para o jogador beligerante
                                    if g.participa(&eu) {
                                        ui.separator();
                                        ui.strong("Tratado de Paz:");
                                        let vencedor = if g.pontuacao_guerra >= 35.0 {
                                            g.atacantes.contains(&eu)
                                        } else if g.pontuacao_guerra <= -35.0 {
                                            g.defensores.contains(&eu)
                                        } else {
                                            false
                                        };
                                        if vencedor {
                                            ui.label("Você venceu: a proposta padrão anexa as regiões ocupadas e exige reparações.");
                                            if ui.button("Oferecer paz automática (proposta padrão)").clicked() {
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

                                            if ui.button("Propor e Assinar Tratado de Paz").clicked() {
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
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.heading(&f.nome_pt);
                                        let cor_status = if f.ativa { egui::Color32::LIGHT_GREEN } else { egui::Color32::GRAY };
                                        ui.colored_label(cor_status, if f.ativa { "● Ativa" } else { "○ Inativa" });
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Beligerantes: {} vs {}", nome(&f.lado_a_tag), nome(&f.lado_b_tag)));
                                        ui.separator();
                                        ui.label(format!("Pares de Fronteira: {}", f.pares_fronteira.len()));
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(format!("Regiões Lado A: {}", f.regioes_lado_a.len()));
                                        ui.label(format!("Regiões Lado B: {}", f.regioes_lado_b.len()));
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

                    ui.heading(format!("Forças Armadas de {}", nome(&eu)));
                    ui.label(format!("Total de Exércitos: {}", exercitos.len()));
                    ui.separator();

                    egui::ScrollArea::vertical().id_salt("scroll_exercitos").show(ui, |ui| {
                        for (e_idx, exe) in exercitos.iter().enumerate() {
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.strong(&exe.nome_pt);
                                    if let Some(ref c_id) = exe.comandante_id {
                                        if let Some(c) = comandantes.iter().find(|cmd| &cmd.id == c_id) {
                                            ui.label(format!("General: {} (Atk {}, Def {}, Log {})", c.nome_pt, c.habilidade_ataque, c.habilidade_defesa, c.habilidade_logistica));
                                        }
                                    } else {
                                        ui.label("General: (Comando Geral)");
                                    }
                                });

                                // Postura operacional do exército
                                ui.horizontal(|ui| {
                                    ui.label("Postura Operacional:");
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
                                    for d_id in &exe.divisoes_ids {
                                        if let Some(div) = divisoes.iter().find(|d| &d.id == d_id) {
                                            ui.horizontal(|ui| {
                                                ui.label(&div.nome_pt);
                                                ui.label(format!("Efetivos: {}", div.efetivos_atuais));
                                                ui.label(format!("Org: {:.1}", div.organizacao));
                                                ui.label(format!("Suprimento: {:.0}%", div.suprimento_atual * 100.0));
                                                ui.label(format!("Local: {}", div.regiao_adm1));
                                            });
                                        }
                                    }
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
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.strong(format!("Tratado #{}: {}", t_idx + 1, t.guerra_id));
                                        ui.label(format!("Data: {:02}/{:02}/{}", t.data_assinatura.dia, t.data_assinatura.mes, t.data_assinatura.ano));
                                    });
                                    ui.label(format!("Vencedores: {:?}", t.vencedores));
                                    ui.label(format!("Derrotados: {:?}", t.derrotados));
                                    ui.label(format!("Cláusulas Executadas: {:?}", t.clausulas_executadas));
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
                ui.colored_label(egui::Color32::LIGHT_GREEN, &painel.mensagem);
            }
        });

    if !aberto {
        painel.aberto = false;
    }

    Ok(())
}
