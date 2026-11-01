//! Tela de Comércio Internacional (3): controle de importações e exportações por bem e parceiro,
//! configuração de tarifas gerais e por bem, acordos bilaterais de livre comércio e blocos econômicos (URSS/COMECON).

use super::economia_sim::{Comando, EconomiaSim, VistaEconomia};
use super::jogador::{PaisJogador, fmt_din};
use super::mercado_ui::Icones;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum AbaComercio {
    #[default]
    RotasAtivas,
    NovaRota,
    TarifasEAbertura,
    AcordosEBlocos,
}

#[derive(Resource)]
pub struct PainelComercio {
    pub aberto: bool,
    aba: AbaComercio,
    // Estado para Nova Rota
    pub nova_rota_bem: String,
    pub nova_rota_parceiro: String,
    pub nova_rota_exportacao: bool,
    pub nova_rota_volume: f64,
    // Estado para Acordo Comercial
    pub novo_acordo_parceiro: String,
}

impl Default for PainelComercio {
    fn default() -> Self {
        Self {
            aberto: false,
            aba: AbaComercio::RotasAtivas,
            nova_rota_bem: "petroleo".to_string(),
            nova_rota_parceiro: String::new(),
            nova_rota_exportacao: false,
            nova_rota_volume: 50.0,
            novo_acordo_parceiro: String::new(),
        }
    }
}

pub struct ComercioUiPlugin;

impl Plugin for ComercioUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PainelComercio>()
            .add_systems(
                Update,
                alternar_comercio
                    .run_if(resource_exists::<PaisJogador>)
                    .run_if(crate::estado::em_partida),
            )
            .add_systems(
                EguiPrimaryContextPass,
                tela_comercio
                    .run_if(resource_exists::<PaisJogador>)
                    .run_if(resource_exists::<EconomiaSim>)
                    .run_if(crate::estado::em_partida),
            );
    }
}

fn alternar_comercio(teclas: Res<ButtonInput<KeyCode>>, mut p: ResMut<PainelComercio>) {
    if teclas.just_pressed(KeyCode::Digit3) || teclas.just_pressed(KeyCode::Numpad3) {
        p.aberto = !p.aberto;
    }
}

fn tela_comercio(
    mut contextos: EguiContexts,
    mut painel: ResMut<PainelComercio>,
    mut eco: ResMut<EconomiaSim>,
    mut icones: ResMut<Icones>,
) -> Result {
    if !painel.aberto {
        return Ok(());
    }

    let ctx = contextos.ctx_mut()?;
    let mut aberto = true;
    let v = &eco.vista;
    let r = &v.resumo_comercio;
    let mut comandos = Vec::new();

    egui::Window::new("Comércio Internacional (3)")
        .open(&mut aberto)
        .default_size([960.0, 640.0])
        .show(ctx, |ui| {
            // =========================================================================
            // PAINEL SUPERIOR: INDICADORES MACROECONÔMICOS DE COMÉRCIO
            // =========================================================================
            ui.horizontal_wrapped(|ui| {
                ui.group(|ui| {
                    ui.label("Exportações:");
                    ui.colored_label(egui::Color32::LIGHT_GREEN, fmt_din(r.total_exportacoes_valor));
                });
                ui.group(|ui| {
                    ui.label("Importações:");
                    ui.colored_label(egui::Color32::from_rgb(230, 160, 90), fmt_din(r.total_importacoes_valor));
                });
                ui.group(|ui| {
                    ui.label("Saldo Comercial:");
                    let cor_saldo = if r.saldo_comercial >= 0.0 {
                        egui::Color32::LIGHT_GREEN
                    } else {
                        egui::Color32::LIGHT_RED
                    };
                    ui.colored_label(cor_saldo, fmt_din(r.saldo_comercial));
                });
                ui.group(|ui| {
                    ui.label("Receita de Tarifas:");
                    ui.colored_label(egui::Color32::LIGHT_BLUE, fmt_din(r.receita_tarifas_total));
                });
                ui.group(|ui| {
                    ui.label("Abertura Comercial:");
                    ui.strong(format!("{:.0}%", r.nivel_abertura * 100.0));
                });
            });

            ui.add_space(4.0);

            // =========================================================================
            // LINHA DE STATUS DO BLOCO COMERCIAL E COMECON
            // =========================================================================
            ui.horizontal(|ui| {
                if let Some(bloco_nome) = &r.bloco_nome {
                    let (status_txt, cor_bloco) = if r.bloco_id.as_deref() == Some("bloco_sovietico") {
                        if r.comecon_ativo {
                            ("★ COMECON Ativo (Mercado Integrado Socialista)", egui::Color32::LIGHT_GREEN)
                        } else {
                            ("🔒 Mercados Fechados até o COMECON", egui::Color32::LIGHT_RED)
                        }
                    } else if r.bloco_fechado {
                        ("🔒 Mercado Fechado", egui::Color32::LIGHT_RED)
                    } else {
                        ("🌐 Mercado Aberto / GATT", egui::Color32::LIGHT_BLUE)
                    };

                    ui.label(format!("Bloco Econômico: {bloco_nome}"));
                    ui.colored_label(cor_bloco, format!("[{status_txt}]"));

                    // Se for a URSS ou satélite do bloco soviético, permite alternar/ativar o COMECON
                    if r.bloco_id.as_deref() == Some("bloco_sovietico") {
                        let btn_txt = if r.comecon_ativo {
                            "Desativar COMECON"
                        } else {
                            "★ Ativar Integração COMECON (1949)"
                        };
                        if ui.button(btn_txt).clicked() {
                            comandos.push(Comando::AlternarComecon);
                        }
                    }
                } else {
                    ui.label("Bloco Econômico: Não Alinhado / Mercado Autônomo");
                }
            });

            ui.separator();

            // =========================================================================
            // ABAS DE NAVEGAÇÃO
            // =========================================================================
            ui.horizontal(|ui| {
                if ui.selectable_label(painel.aba == AbaComercio::RotasAtivas, "🛳 Rotas Comerciais Ativas").clicked() {
                    painel.aba = AbaComercio::RotasAtivas;
                }
                if ui.selectable_label(painel.aba == AbaComercio::NovaRota, "➕ Estabelecer Nova Rota").clicked() {
                    painel.aba = AbaComercio::NovaRota;
                }
                if ui.selectable_label(painel.aba == AbaComercio::TarifasEAbertura, "⚖ Tarifas e Abertura").clicked() {
                    painel.aba = AbaComercio::TarifasEAbertura;
                }
                if ui.selectable_label(painel.aba == AbaComercio::AcordosEBlocos, "🤝 Acordos Bilaterais e Blocos").clicked() {
                    painel.aba = AbaComercio::AcordosEBlocos;
                }
            });

            ui.separator();

            // =========================================================================
            // CONTEÚDO DA ABA SELECIONADA
            // =========================================================================
            match painel.aba {
                AbaComercio::RotasAtivas => {
                    render_rotas_ativas(ui, v, &mut comandos, &mut icones);
                }
                AbaComercio::NovaRota => {
                    render_nova_rota(ui, &mut painel, v, &mut comandos, &mut icones);
                }
                AbaComercio::TarifasEAbertura => {
                    render_tarifas_e_abertura(ui, v, &mut comandos, &mut icones);
                }
                AbaComercio::AcordosEBlocos => {
                    render_acordos_e_blocos(ui, &mut painel, v, &mut comandos);
                }
            }
        });

    for cmd in comandos {
        eco.enviar(cmd);
    }

    if !aberto {
        painel.aberto = false;
    }
    Ok(())
}

fn render_rotas_ativas(
    ui: &mut egui::Ui,
    v: &VistaEconomia,
    comandos: &mut Vec<Comando>,
    icones: &mut Icones,
) {
    if v.rotas_comerciais.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.heading("Nenhuma rota comercial direta ativa.");
            ui.label("Use a aba '➕ Estabelecer Nova Rota' para abrir linhas de importação ou exportação com outros países.");
            ui.add_space(20.0);
        });
        return;
    }

    ui.label(format!("Total de rotas ativas: {}", v.rotas_comerciais.len()));

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("tabela_rotas_comerciais")
            .striped(true)
            .min_col_width(70.0)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.strong("Tipo");
                ui.strong("Bem");
                ui.strong("Parceiro");
                ui.strong("Volume");
                ui.strong("Preço Origem");
                ui.strong("Preço Destino");
                ui.strong("Margem Unit.");
                ui.strong("Lucro Diário");
                ui.strong("Tarifas");
                ui.strong("Ações");
                ui.end_row();

                for rota in &v.rotas_comerciais {
                    let tipo_rotulo = if rota.eh_exportacao { "Exportação" } else { "Importação" };
                    let tipo_cor = if rota.eh_exportacao {
                        egui::Color32::LIGHT_GREEN
                    } else {
                        egui::Color32::from_rgb(230, 160, 90)
                    };
                    ui.colored_label(tipo_cor, tipo_rotulo);

                    ui.horizontal(|ui| {
                        icones.mostrar(ui, &rota.bem_id, 20.0);
                        ui.label(&rota.bem_nome);
                    });

                    ui.horizontal(|ui| {
                        ui.label(&rota.parceiro_pais_nome);
                        if rota.tem_acordo {
                            ui.colored_label(egui::Color32::LIGHT_BLUE, "(Acordo)");
                        }
                    });

                    ui.label(format!("{:.1} / {:.0}", rota.volume_atual, rota.volume_maximo));
                    ui.label(fmt_din(rota.preco_origem));
                    ui.label(fmt_din(rota.preco_destino));

                    let margem = rota.preco_destino / (1.0 + rota.tarifa_destino)
                        - (rota.preco_origem * (1.0 + rota.tarifa_origem));
                    let cor_margem = if margem >= 0.0 {
                        egui::Color32::LIGHT_GREEN
                    } else {
                        egui::Color32::LIGHT_RED
                    };
                    ui.colored_label(cor_margem, fmt_din(margem));

                    let cor_lucro = if rota.lucro_ultimo_tick >= 0.0 {
                        egui::Color32::LIGHT_GREEN
                    } else {
                        egui::Color32::LIGHT_RED
                    };
                    ui.colored_label(cor_lucro, fmt_din(rota.lucro_ultimo_tick));

                    ui.label(format!("Exp {:.0}% / Imp {:.0}%", rota.tarifa_origem * 100.0, rota.tarifa_destino * 100.0));

                    ui.horizontal(|ui| {
                        let btn_auto = if rota.automatica { "Auto" } else { "Fixo" };
                        if ui.button(btn_auto).on_hover_text("Alternar entre ajuste automático de volume e volume manual fixo").clicked() {
                            comandos.push(Comando::AlternarRotaAuto(rota.id.clone()));
                        }
                        if ui.small_button("+10").clicked() {
                            comandos.push(Comando::AjustarVolumeRota {
                                rota_id: rota.id.clone(),
                                volume: rota.volume_maximo + 10.0,
                            });
                        }
                        if ui.small_button("-10").clicked() && rota.volume_maximo > 10.0 {
                            comandos.push(Comando::AjustarVolumeRota {
                                rota_id: rota.id.clone(),
                                volume: (rota.volume_maximo - 10.0).max(5.0),
                            });
                        }
                        if ui.button("❌").on_hover_text("Cancelar e encerrar rota comercial").clicked() {
                            comandos.push(Comando::RemoverRota(rota.id.clone()));
                        }
                    });

                    ui.end_row();
                }
            });
    });
}

fn render_nova_rota(
    ui: &mut egui::Ui,
    painel: &mut PainelComercio,
    v: &VistaEconomia,
    comandos: &mut Vec<Comando>,
    icones: &mut Icones,
) {
    ui.heading("Estabelecer Nova Rota Comercial Bilateral");
    ui.label("Conecte seu mercado ao de outro país para sanar déficits internos ou lucrar com a venda de excedentes.");
    ui.add_space(8.0);

    // Inicializa seleção de parceiro se vazia
    if painel.nova_rota_parceiro.is_empty() && !v.parceiros_comerciais.is_empty() {
        if let Some(p) = v.parceiros_comerciais.iter().find(|p| p.permitido_comercio) {
            painel.nova_rota_parceiro = p.pais_id.clone();
        } else {
            painel.nova_rota_parceiro = v.parceiros_comerciais[0].pais_id.clone();
        }
    }

    egui::Grid::new("grid_nova_rota").spacing([16.0, 10.0]).show(ui, |ui| {
        // Sentido da rota
        ui.strong("Sentido da Rota:");
        ui.horizontal(|ui| {
            ui.radio_value(&mut painel.nova_rota_exportacao, false, "Importar (comprar de parceiro)");
            ui.radio_value(&mut painel.nova_rota_exportacao, true, "Exportar (vender para parceiro)");
        });
        ui.end_row();

        // Bem a comercializar
        ui.strong("Mercadoria / Bem:");
        let mut nome_atual = painel.nova_rota_bem.clone();
        if let Some(b) = v.bens.iter().find(|b| b.id == painel.nova_rota_bem) {
            nome_atual = b.nome.clone();
        }
        egui::ComboBox::from_id_salt("combo_nova_rota_bem")
            .selected_text(nome_atual)
            .show_ui(ui, |ui| {
                for b in v.bens.iter().filter(|b| b.comercializavel) {
                    let rotulo = format!("{} (Preço Nacional: {})", b.nome, fmt_din(b.preco_nacional));
                    if ui.selectable_value(&mut painel.nova_rota_bem, b.id.clone(), rotulo).clicked() {}
                }
            });
        ui.end_row();

        // Parceiro comercial
        ui.strong("País Parceiro:");
        let mut nome_parceiro_atual = painel.nova_rota_parceiro.clone();
        if let Some(p) = v.parceiros_comerciais.iter().find(|p| p.pais_id == painel.nova_rota_parceiro) {
            nome_parceiro_atual = p.nome.clone();
        }
        egui::ComboBox::from_id_salt("combo_nova_rota_parceiro")
            .selected_text(nome_parceiro_atual)
            .show_ui(ui, |ui| {
                for p in &v.parceiros_comerciais {
                    let mut extra = String::new();
                    if p.tem_acordo {
                        extra.push_str(" [Acordo Bilateral]");
                    }
                    if !p.permitido_comercio {
                        extra.push_str(" [🔒 Bloqueado por Bloco]");
                    }
                    let rotulo = format!("{}{extra}", p.nome);
                    if ui.selectable_value(&mut painel.nova_rota_parceiro, p.pais_id.clone(), rotulo).clicked() {}
                }
            });
        ui.end_row();

        // Volume inicial desejado
        ui.strong("Volume Diário Máximo:");
        ui.add(egui::Slider::new(&mut painel.nova_rota_volume, 10.0..=500.0).suffix(" un/dia"));
        ui.end_row();
    });

    ui.add_space(12.0);

    // Validação e previsão da rota
    let parceiro_opt = v.parceiros_comerciais.iter().find(|p| p.pais_id == painel.nova_rota_parceiro);
    let bem_opt = v.bens.iter().find(|b| b.id == painel.nova_rota_bem);

    if let (Some(parc), Some(bem)) = (parceiro_opt, bem_opt) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                icones.mostrar(ui, &bem.id, 24.0);
                ui.heading(format!("Análise da Rota: {} com {}", bem.nome, parc.nome));
            });

            ui.label(format!("• Preço Base Internacional: {}", fmt_din(bem.preco_base)));
            ui.label(format!("• Preço no Seu Mercado: {}", fmt_din(bem.preco_nacional)));
            ui.label(format!("• Preço Médio Mundial: {}", fmt_din(bem.preco_mundial)));

            if !parc.permitido_comercio {
                if let Some(razao) = &parc.razao_bloqueio {
                    ui.colored_label(egui::Color32::LIGHT_RED, format!("⚠ Comércio Bloqueado: {razao}"));
                    ui.label("Assine um acordo comercial prévio na aba '🤝 Acordos Bilaterais' para liberar o comércio com este país.");
                }
            } else {
                if parc.tem_acordo {
                    ui.colored_label(egui::Color32::LIGHT_BLUE, "✓ Acordo Comercial Ativo: 85% de desconto nas tarifas alfandegárias.");
                }
                if ui.button("Confirmar e Abrir Rota Comercial").clicked() {
                    comandos.push(Comando::CriarRota {
                        bem_id: painel.nova_rota_bem.clone(),
                        parceiro_id: painel.nova_rota_parceiro.clone(),
                        exportacao: painel.nova_rota_exportacao,
                        volume: painel.nova_rota_volume,
                    });
                    painel.aba = AbaComercio::RotasAtivas;
                }
            }
        });
    }
}

fn render_tarifas_e_abertura(
    ui: &mut egui::Ui,
    v: &VistaEconomia,
    comandos: &mut Vec<Comando>,
    icones: &mut Icones,
) {
    ui.heading("Política Aduaneira, Tarifas e Nível de Abertura");
    ui.label("Equilibre a proteção das empresas nacionais com o abastecimento de insumos e matérias-primas importadas.");
    ui.add_space(8.0);

    let Some(pais) = &v.pais else { return };
    let mut tarifa_imp = pais.politica_fiscal.tarifa_importacao;
    let mut tarifa_exp = pais.politica_fiscal.tarifa_exportacao;
    let abertura = pais.politica_fiscal.nivel_abertura();

    ui.group(|ui| {
        ui.strong("Presets de Política Comercial (Victoria 3):");
        ui.horizontal_wrapped(|ui| {
            if ui.button("🌐 Livre-Câmbio (GATT) [Tarifas Mínimas]").clicked() {
                comandos.push(Comando::DefinirAbertura(1.0));
            }
            if ui.button("🛡 Protecionismo Moderado [Subs. Importações]").clicked() {
                comandos.push(Comando::DefinirAbertura(0.60));
            }
            if ui.button("🧱 Protecionismo Severo [Altas Tarifas]").clicked() {
                comandos.push(Comando::DefinirAbertura(0.30));
            }
            if ui.button("🔒 Autarquia Fechada [Isolacionismo]").clicked() {
                comandos.push(Comando::DefinirAbertura(0.0));
            }
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(format!("Nível Geral de Abertura: {:.0}%", abertura * 100.0));
            if ui.button("➕ Mais Aberto (+5%)").clicked() {
                comandos.push(Comando::DefinirAbertura((abertura + 0.05).min(1.0)));
            }
            if ui.button("➖ Mais Fechado (-5%)").clicked() {
                comandos.push(Comando::DefinirAbertura((abertura - 0.05).max(0.0)));
            }
        });
    });

    ui.add_space(8.0);

    ui.group(|ui| {
        ui.strong("Tarifas Gerais Aduaneiras:");
        let mut alterou_geral = false;
        ui.horizontal(|ui| {
            ui.label("Tarifa Geral de Importação:");
            if ui.add(egui::Slider::new(&mut tarifa_imp, 0.0..=0.60).custom_formatter(|v, _| format!("{:.1}%", v * 100.0))).changed() {
                alterou_geral = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Tarifa Geral de Exportação:");
            if ui.add(egui::Slider::new(&mut tarifa_exp, 0.0..=0.40).custom_formatter(|v, _| format!("{:.1}%", v * 100.0))).changed() {
                alterou_geral = true;
            }
        });
        if alterou_geral {
            comandos.push(Comando::DefinirTarifaGeral { importacao: tarifa_imp, exportacao: tarifa_exp });
        }
    });

    ui.add_space(8.0);
    ui.strong("Tarifas Seletivas por Bem:");
    ui.label("Você pode proteger setores estratégicos da indústria nacional ou isentar insumos críticos.");

    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        egui::Grid::new("grid_tarifas_bens")
            .striped(true)
            .min_col_width(80.0)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.strong("Bem");
                ui.strong("Tarifa Importação");
                ui.strong("Tarifa Exportação");
                ui.strong("Políticas Rápidas");
                ui.end_row();

                for bem in v.bens.iter().filter(|b| b.comercializavel) {
                    let imp_efetiva = pais.politica_fiscal.tarifa_importacao_bem(&bem.id);
                    let exp_efetiva = pais.politica_fiscal.tarifa_exportacao_bem(&bem.id);

                    ui.horizontal(|ui| {
                        icones.mostrar(ui, &bem.id, 18.0);
                        ui.label(&bem.nome);
                    });

                    ui.label(format!("{:.1}%", imp_efetiva * 100.0));
                    ui.label(format!("{:.1}%", exp_efetiva * 100.0));

                    ui.horizontal(|ui| {
                        if ui.small_button("Isento (0%)").on_hover_text("Incentivar importação e suprimento").clicked() {
                            comandos.push(Comando::DefinirTarifaBem { bem_id: bem.id.clone(), tarifa_imp: 0.0, tarifa_exp: 0.0 });
                        }
                        if ui.small_button("Padrão").on_hover_text("Retornar à tarifa geral da nação").clicked() {
                            comandos.push(Comando::DefinirTarifaBem { bem_id: bem.id.clone(), tarifa_imp: pais.politica_fiscal.tarifa_importacao, tarifa_exp: pais.politica_fiscal.tarifa_exportacao });
                        }
                        if ui.small_button("Proteger (30%)").on_hover_text("Defender produtores nacionais contra concorrência estrangeira").clicked() {
                            comandos.push(Comando::DefinirTarifaBem { bem_id: bem.id.clone(), tarifa_imp: 0.30, tarifa_exp: 0.02 });
                        }
                        if ui.small_button("Barreira (60%)").on_hover_text("Barreira protecionista quase proibitiva").clicked() {
                            comandos.push(Comando::DefinirTarifaBem { bem_id: bem.id.clone(), tarifa_imp: 0.60, tarifa_exp: 0.0 });
                        }
                    });

                    ui.end_row();
                }
            });
    });
}

fn render_acordos_e_blocos(
    ui: &mut egui::Ui,
    painel: &mut PainelComercio,
    v: &VistaEconomia,
    comandos: &mut Vec<Comando>,
) {
    ui.heading("Acordos Bilaterais de Comércio e Blocos Econômicos");
    ui.label("Acordos comerciais reduzem tarifas aduaneiras em 85% e autorizam o comércio com países em blocos fechados.");
    ui.add_space(8.0);

    // =========================================================================
    // SEÇÃO 1: ACORDOS VIGENTES
    // =========================================================================
    ui.group(|ui| {
        ui.strong("Acordos Comerciais Bilaterais Ativos:");
        let acordos = &v.resumo_comercio.acordos_comerciais;
        if acordos.is_empty() {
            ui.label("(Nenhum acordo comercial bilateral firmado no momento)");
        } else {
            let mut cancelar = None;
            for pid in acordos {
                let nome_parceiro = v.ranking.iter().find(|p| &p.id == pid).map(|p| p.nome.as_str()).unwrap_or(pid);
                ui.horizontal(|ui| {
                    ui.label(format!("🤝 Acordo de Livre Comércio com {nome_parceiro} ({pid})"));
                    if ui.button("Revogar Acordo").clicked() {
                        cancelar = Some(pid.clone());
                    }
                });
            }
            if let Some(pid) = cancelar {
                comandos.push(Comando::CancelarAcordo(pid));
            }
        }
    });

    ui.add_space(8.0);

    // =========================================================================
    // SEÇÃO 2: PROPOR NOVO ACORDO
    // =========================================================================
    ui.group(|ui| {
        ui.strong("Firmar Novo Acordo Comercial Bilateral:");
        let parceiros_sem_acordo: Vec<&super::economia_sim::ResumoPaisEco> = v
            .ranking
            .iter()
            .filter(|p| p.id != v.pais_id && !v.resumo_comercio.acordos_comerciais.contains(&p.id))
            .collect();

        if parceiros_sem_acordo.is_empty() {
            ui.label("Todos os países conhecidos já possuem acordos comerciais com a sua nação.");
        } else {
            if painel.novo_acordo_parceiro.is_empty() || !parceiros_sem_acordo.iter().any(|p| p.id == painel.novo_acordo_parceiro) {
                painel.novo_acordo_parceiro = parceiros_sem_acordo[0].id.clone();
            }

            ui.horizontal(|ui| {
                let nome_sel = parceiros_sem_acordo.iter().find(|p| p.id == painel.novo_acordo_parceiro).map(|p| p.nome.as_str()).unwrap_or(&painel.novo_acordo_parceiro);
                egui::ComboBox::from_id_salt("combo_propor_acordo")
                    .selected_text(nome_sel)
                    .show_ui(ui, |ui| {
                        for p in &parceiros_sem_acordo {
                            ui.selectable_value(&mut painel.novo_acordo_parceiro, p.id.clone(), &p.nome);
                        }
                    });

                if ui.button("Firmar Acordo Comercial").clicked() {
                    comandos.push(Comando::AssinarAcordo(painel.novo_acordo_parceiro.clone()));
                }
            });
        }
    });

    ui.add_space(8.0);

    // =========================================================================
    // SEÇÃO 3: BLOCOS ECONÔMICOS DA GUERRA FRIA
    // =========================================================================
    ui.group(|ui| {
        ui.strong("Blocos Econômicos da Guerra Fria:");
        ui.label("Na ordem econômica pós-1945, o mundo se divide entre a integração multilateral aberta (Bretton Woods / GATT) e a economia planificada socialista.");
        ui.add_space(4.0);

        ui.label(format!("• Seu Bloco Atual: {}", v.resumo_comercio.bloco_nome.as_deref().unwrap_or("Não Alinhado")));
        if v.resumo_comercio.bloco_fechado {
            ui.colored_label(egui::Color32::LIGHT_RED, "• Status do Bloco: FECHADO para o comércio exterior capitalista sem acordo bilateral.");
        } else {
            ui.colored_label(egui::Color32::LIGHT_GREEN, "• Status do Bloco: ABERTO ao comércio internacional.");
        }

        if v.resumo_comercio.bloco_id.as_deref() == Some("bloco_sovietico") {
            ui.separator();
            ui.strong("Conselho para Assistência Econômica Mútua (COMECON):");
            ui.label("Em 1945, os mercados da URSS e seus satélites no Leste Europeu operavam fechados sob controle central e acordos bilaterais rígidos.");
            ui.label("Com a ativação do COMECON (historicamente em janeiro de 1949), todos os membros do bloco socialista desfrutam de livre cooperação econômica e comércio mútuo com tarifas zeradas.");
            ui.add_space(4.0);

            let status_comecon = if v.resumo_comercio.comecon_ativo {
                "Ativo: Livre Comércio Socialista Unificado (0% Tarifas entre Membros)"
            } else {
                "Inativo (Mercados da URSS e Satélites Fechados até 1949)"
            };
            ui.horizontal(|ui| {
                ui.strong("Situação:");
                ui.colored_label(if v.resumo_comercio.comecon_ativo { egui::Color32::LIGHT_GREEN } else { egui::Color32::YELLOW }, status_comecon);
            });

            let btn_txt = if v.resumo_comercio.comecon_ativo {
                "Desativar Integração COMECON"
            } else {
                "★ Fundar / Ativar COMECON (Mercado Socialista Integrado)"
            };
            if ui.button(btn_txt).clicked() {
                comandos.push(Comando::AlternarComecon);
            }
        }
    });
}
