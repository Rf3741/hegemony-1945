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

/// Nome de célula de tabela: corta em `n` caracteres e sinaliza o corte com reticências ASCII.
fn encurtar(texto: &str, n: usize) -> String {
    if texto.chars().count() > n {
        format!("{}...", texto.chars().take(n).collect::<String>())
    } else {
        texto.to_string()
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

    ui_refinada::janela_dossie("COMÉRCIO INTERNACIONAL (3)", None)
        .default_pos([20.0, 96.0])
        .default_size([980.0, 660.0])
        .show(ctx, |ui| {
            ui_refinada::faixa_titulo(ui, "Comércio Internacional", Some("3"), Some(&mut aberto));
            // =========================================================================
            // PAINEL SUPERIOR: INDICADORES MACROECONÔMICOS DE COMÉRCIO
            // =========================================================================
            ui.horizontal_wrapped(|ui| {
                ui_refinada::chapa(ui, "navio", "Exportações", &fmt_din(r.total_exportacoes_valor), Some(ui_refinada::menu::SUCESSO));
                ui_refinada::chapa(ui, "trem", "Importações", &fmt_din(r.total_importacoes_valor), Some(ui_refinada::menu::AVISO));
                let cor_saldo = if r.saldo_comercial >= 0.0 {
                    ui_refinada::menu::SUCESSO
                } else {
                    ui_refinada::menu::PERIGO
                };
                ui_refinada::chapa(ui, "balanca", "Saldo comercial", &fmt_din(r.saldo_comercial), Some(cor_saldo));
                ui_refinada::chapa(ui, "moeda", "Receita de tarifas", &fmt_din(r.receita_tarifas_total), Some(ui_refinada::menu::INFO));
                ui_refinada::chapa(ui, "engrenagem", "Abertura", &format!("{:.0}%", r.nivel_abertura * 100.0), None);
            });

            ui.add_space(4.0);

            // =========================================================================
            // LINHA DE STATUS DO BLOCO COMERCIAL E COMECON
            // =========================================================================
            ui.horizontal_wrapped(|ui| {
                if let Some(bloco_nome) = &r.bloco_nome {
                    let (glifo, status_txt, cor_bloco) = if r.bloco_id.as_deref() == Some("bloco_sovietico") {
                        if r.comecon_ativo {
                            ("estrela", "COMECON ATIVO · MERCADO INTEGRADO SOCIALISTA", ui_refinada::menu::SUCESSO)
                        } else {
                            ("caixa", "MERCADOS FECHADOS ATÉ O COMECON", ui_refinada::menu::PERIGO)
                        }
                    } else if r.bloco_fechado {
                        ("caixa", "MERCADO FECHADO", ui_refinada::menu::PERIGO)
                    } else {
                        ("navio", "MERCADO ABERTO · GATT", ui_refinada::menu::INFO)
                    };

                    ui.label(format!("Bloco Econômico: {bloco_nome}"));
                    ui_refinada::glifo_ui(ui, glifo, 14.0, Some(cor_bloco));
                    ui.colored_label(cor_bloco, egui::RichText::new(status_txt).small().strong());

                    // Se for a URSS ou satélite do bloco soviético, permite alternar/ativar o COMECON
                    if r.bloco_id.as_deref() == Some("bloco_sovietico") {
                        let btn_txt = if r.comecon_ativo {
                            "Desativar COMECON"
                        } else {
                            "Ativar Integração COMECON (1949)"
                        };
                        if ui_refinada::botao_estilo_secundario(ui, btn_txt).clicked() {
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
                if ui_refinada::aba_glifo(ui, painel.aba == AbaComercio::RotasAtivas, Some("navio"), "Rotas Ativas") {
                    painel.aba = AbaComercio::RotasAtivas;
                }
                if ui_refinada::aba_glifo(ui, painel.aba == AbaComercio::NovaRota, Some("mais"), "Nova Rota") {
                    painel.aba = AbaComercio::NovaRota;
                }
                if ui_refinada::aba_glifo(ui, painel.aba == AbaComercio::TarifasEAbertura, Some("balanca"), "Tarifas") {
                    painel.aba = AbaComercio::TarifasEAbertura;
                }
                if ui_refinada::aba_glifo(ui, painel.aba == AbaComercio::AcordosEBlocos, Some("coroa"), "Acordos e Blocos") {
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
            ui.label(
                egui::RichText::new("NENHUMA ROTA COMERCIAL DIRETA ATIVA")
                    .family(ui_refinada::familia_oswald(ui.ctx()))
                    .size(18.0)
                    .strong()
                    .color(ui_refinada::menu::AZUL_MARINHO),
            );
            ui.label("Use a aba Nova Rota para abrir linhas de importação ou exportação com outros países.");
            ui.add_space(20.0);
        });
        return;
    }

    ui.label(format!("Total de rotas ativas: {}", v.rotas_comerciais.len()));

    egui::ScrollArea::both().id_salt("scroll_rotas_ativas").max_height(300.0).show(ui, |ui| {
        egui::Grid::new("tabela_rotas_comerciais")
            .striped(true)
            .min_col_width(0.0)
            .spacing([8.0, 5.0])
            .show(ui, |ui| {
                let cab = |ui: &mut egui::Ui, t: &str| {
                    ui.label(
                        egui::RichText::new(t.to_uppercase())
                            .family(ui_refinada::familia_oswald(ui.ctx()))
                            .size(11.0)
                            .strong()
                            .color(ui_refinada::menu::TINTA_SUAVE),
                    );
                };
                cab(ui, "Tipo");
                cab(ui, "Bem");
                cab(ui, "Parceiro");
                cab(ui, "Volume");
                cab(ui, "Preços");
                cab(ui, "Margem");
                cab(ui, "Lucro/dia");
                cab(ui, "Tarifas");
                cab(ui, "Ações");
                ui.end_row();

                for rota in &v.rotas_comerciais {
                    let tipo_rotulo = if rota.eh_exportacao { "Exportação" } else { "Importação" };
                    let tipo_cor = if rota.eh_exportacao {
                        ui_refinada::menu::SUCESSO
                    } else {
                        ui_refinada::menu::AVISO
                    };
                    ui.add_sized(
                        [66.0, 16.0],
                        egui::Label::new(egui::RichText::new(tipo_rotulo).size(12.0).color(tipo_cor)).selectable(false),
                    );

                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(118.0, 18.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            icones.mostrar(ui, &rota.bem_id, 18.0);
                            let nome: String = encurtar(&rota.bem_nome, 12);
                            ui.label(egui::RichText::new(nome).size(12.0).color(ui_refinada::menu::TINTA)).on_hover_text(&rota.bem_nome);
                        },
                    );

                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(130.0, 18.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let nome: String = encurtar(&rota.parceiro_pais_nome, 18);
                            ui.label(egui::RichText::new(nome).size(12.0).color(ui_refinada::menu::TINTA)).on_hover_text(&rota.parceiro_pais_nome);
                            if rota.tem_acordo {
                                ui.colored_label(ui_refinada::menu::INFO, egui::RichText::new("acordo").size(10.0).color(ui_refinada::menu::INFO));
                            }
                        },
                    );

                    ui.allocate_ui_with_layout(
                        egui::Vec2::new(88.0, 18.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let ocup = if rota.volume_maximo > 0.0 { (rota.volume_atual / rota.volume_maximo) as f32 } else { 0.0 };
                            ui_refinada::barra_trilho(ui, ocup, ui_refinada::menu::AZUL_MARINHO, 34.0, 10.0);
                            ui.label(egui::RichText::new(format!("{:.0}/{:.0}", rota.volume_atual, rota.volume_maximo)).size(11.0));
                        },
                    );
                    ui.add_sized(
                        [84.0, 16.0],
                        egui::Label::new(egui::RichText::new(format!("{} / {}", fmt_din(rota.preco_origem), fmt_din(rota.preco_destino))).size(12.0)).selectable(false),
                    );

                    let margem = rota.preco_destino / (1.0 + rota.tarifa_destino)
                        - (rota.preco_origem * (1.0 + rota.tarifa_origem));
                    let cor_margem = if margem >= 0.0 {
                        ui_refinada::menu::SUCESSO
                    } else {
                        ui_refinada::menu::PERIGO
                    };
                    ui.add_sized(
                        [56.0, 16.0],
                        egui::Label::new(egui::RichText::new(fmt_din(margem)).size(12.0).color(cor_margem)).selectable(false),
                    );

                    let cor_lucro = if rota.lucro_ultimo_tick >= 0.0 {
                        ui_refinada::menu::SUCESSO
                    } else {
                        ui_refinada::menu::PERIGO
                    };
                    ui.add_sized(
                        [60.0, 16.0],
                        egui::Label::new(egui::RichText::new(fmt_din(rota.lucro_ultimo_tick)).size(12.0).color(cor_lucro)).selectable(false),
                    );

                    ui.add_sized(
                        [70.0, 16.0],
                        egui::Label::new(
                            egui::RichText::new(format!("{:.0}% / {:.0}%", rota.tarifa_origem * 100.0, rota.tarifa_destino * 100.0))
                                .size(12.0)
                                .color(ui_refinada::menu::TINTA),
                        )
                        .selectable(false),
                    );

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let btn_auto = if rota.automatica { "Auto" } else { "Fixo" };
                        if ui.small_button(btn_auto).on_hover_text("Alternar entre ajuste automático de volume e volume manual fixo").clicked() {
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
                        if ui.small_button("Encerrar").clicked() {
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
                        extra.push_str(" [Bloqueado por Bloco]");
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
        ui_refinada::moldura_cartao().show(ui, |ui| {
            ui.horizontal(|ui| {
                icones.mostrar(ui, &bem.id, 24.0);
                ui.heading(format!("Análise da Rota: {} com {}", bem.nome, parc.nome));
            });
            ui_refinada::linha_decorativa(ui);

            egui::Grid::new("gradeAnaliseRota").striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
                ui.weak("Preço base internacional:");
                ui.label(fmt_din(bem.preco_base));
                ui.end_row();
                ui.weak("Preço no seu mercado:");
                ui.label(fmt_din(bem.preco_nacional));
                ui.end_row();
                ui.weak("Preço médio mundial:");
                ui.label(fmt_din(bem.preco_mundial));
                ui.end_row();
            });

            if !parc.permitido_comercio {
                if let Some(razao) = &parc.razao_bloqueio {
                    ui.colored_label(ui_refinada::menu::PERIGO, egui::RichText::new(format!("COMÉRCIO BLOQUEADO · {razao}")).strong());
                    ui.label("Assine um acordo comercial prévio na aba Acordos e Blocos para liberar o comércio com este país.");
                }
            } else {
                if parc.tem_acordo {
                    ui.colored_label(ui_refinada::menu::INFO, "ACORDO COMERCIAL ATIVO · 85% de desconto nas tarifas alfandegárias.");
                }
                if ui_refinada::botao_estilo_menu(ui, "Confirmar e Abrir Rota Comercial").clicked() {
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

    ui_refinada::moldura_cartao().show(ui, |ui| {
        ui.strong("Presets de Política Comercial:");
        ui.horizontal_wrapped(|ui| {
            if ui_refinada::botao_estilo_secundario(ui, "Livre-Câmbio (GATT) · tarifas mínimas").clicked() {
                comandos.push(Comando::DefinirAbertura(1.0));
            }
            if ui_refinada::botao_estilo_secundario(ui, "Protecionismo Moderado · subs. importações").clicked() {
                comandos.push(Comando::DefinirAbertura(0.60));
            }
            if ui_refinada::botao_estilo_secundario(ui, "Protecionismo Severo · altas tarifas").clicked() {
                comandos.push(Comando::DefinirAbertura(0.30));
            }
            if ui_refinada::botao_estilo_secundario(ui, "Autarquia Fechada · isolacionismo").clicked() {
                comandos.push(Comando::DefinirAbertura(0.0));
            }
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Nível Geral de Abertura:");
            ui_refinada::barra_trilho(ui, abertura as f32, ui_refinada::menu::AZUL_MARINHO, 120.0, 12.0);
            ui.strong(format!("{:.0}%", abertura * 100.0));
            if ui.button("Mais Aberto (+5%)").clicked() {
                comandos.push(Comando::DefinirAbertura((abertura + 0.05).min(1.0)));
            }
            if ui.button("Mais Fechado (-5%)").clicked() {
                comandos.push(Comando::DefinirAbertura((abertura - 0.05).max(0.0)));
            }
        });
    });

    ui.add_space(8.0);

    ui_refinada::moldura_cartao().show(ui, |ui| {
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
    ui_refinada::moldura_cartao().show(ui, |ui| {
        ui_refinada::titulo_secao(ui, "Acordos Bilaterais Vigentes");
        let acordos = &v.resumo_comercio.acordos_comerciais;
        if acordos.is_empty() {
            ui.weak("(Nenhum acordo comercial bilateral firmado no momento)");
        } else {
            let mut cancelar = None;
            egui::Grid::new("grade_acordos").striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
                for pid in acordos {
                    let nome_parceiro = v.ranking.iter().find(|p| &p.id == pid).map(|p| p.nome.as_str()).unwrap_or(pid);
                    ui_refinada::glifo_ui(ui, "balanca", 14.0, Some(ui_refinada::menu::AZUL_MARINHO));
                    ui.label(format!("Acordo de Livre Comércio com {nome_parceiro} ({pid})"));
                    if ui.button("Revogar Acordo").clicked() {
                        cancelar = Some(pid.clone());
                    }
                    ui.end_row();
                }
            });
            if let Some(pid) = cancelar {
                comandos.push(Comando::CancelarAcordo(pid));
            }
        }
    });

    ui.add_space(8.0);

    // =========================================================================
    // SEÇÃO 2: PROPOR NOVO ACORDO
    // =========================================================================
    ui_refinada::moldura_cartao().show(ui, |ui| {
        ui_refinada::titulo_secao(ui, "Firmar Novo Acordo Comercial Bilateral");
        let parceiros_sem_acordo: Vec<&super::economia_sim::ResumoPaisEco> = v
            .ranking
            .iter()
            .filter(|p| p.id != v.pais_id && !v.resumo_comercio.acordos_comerciais.contains(&p.id))
            .collect();

        if parceiros_sem_acordo.is_empty() {
            ui.weak("Todos os países conhecidos já possuem acordos comerciais com a sua nação.");
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

                if ui_refinada::botao_estilo_menu(ui, "Firmar Acordo Comercial").clicked() {
                    comandos.push(Comando::AssinarAcordo(painel.novo_acordo_parceiro.clone()));
                }
            });
        }
    });

    ui.add_space(8.0);

    // =========================================================================
    // SEÇÃO 3: BLOCOS ECONÔMICOS DA GUERRA FRIA
    // =========================================================================
    ui_refinada::moldura_cartao().show(ui, |ui| {
        ui_refinada::titulo_secao(ui, "Blocos Econômicos da Guerra Fria");
        ui.label("Na ordem econômica pós-1945, o mundo se divide entre a integração multilateral aberta (Bretton Woods / GATT) e a economia planificada socialista.");
        ui.add_space(4.0);

        ui_refinada::chapa(
            ui,
            "caixa",
            "Seu bloco atual",
            v.resumo_comercio.bloco_nome.as_deref().unwrap_or("Não Alinhado"),
            Some(ui_refinada::menu::AZUL_MARINHO),
        );
        let glifo_status = if v.resumo_comercio.bloco_fechado { "canhao" } else { "navio" };
        let (texto_status, cor_status) = if v.resumo_comercio.bloco_fechado {
            ("Fechado para o comércio exterior capitalista sem acordo bilateral.", ui_refinada::menu::PERIGO)
        } else {
            ("Aberto ao comércio internacional.", ui_refinada::menu::SUCESSO)
        };
        ui.horizontal(|ui| {
            ui_refinada::glifo_ui(ui, glifo_status, 15.0, Some(cor_status));
            ui.colored_label(cor_status, texto_status);
        });

        if v.resumo_comercio.bloco_id.as_deref() == Some("bloco_sovietico") {
            ui.add_space(6.0);
            ui_refinada::linha_decorativa(ui);
            ui.strong("Conselho para Assistência Econômica Mútua (COMECON)");
            ui.label("Em 1945, os mercados da URSS e seus satélites no Leste Europeu operavam fechados sob controle central e acordos bilaterais rígidos.");
            ui.label("Com a ativação do COMECON (historicamente em janeiro de 1949), todos os membros do bloco socialista desfrutam de livre cooperação econômica e comércio mútuo com tarifas zeradas.");
            ui.add_space(4.0);

            let (status_comecon, cor_comecon) = if v.resumo_comercio.comecon_ativo {
                ("Ativo: livre comércio socialista unificado (0% de tarifas entre membros)", ui_refinada::menu::SUCESSO)
            } else {
                ("Inativo: mercados da URSS e satélites fechados até 1949", ui_refinada::menu::AVISO)
            };
            ui.horizontal(|ui| {
                ui_refinada::glifo_ui(ui, "balanca", 15.0, Some(cor_comecon));
                ui.strong("Situação:");
                ui.colored_label(cor_comecon, status_comecon);
            });

            let btn_txt = if v.resumo_comercio.comecon_ativo {
                "Desativar Integração COMECON"
            } else {
                "Fundar COMECON: Mercado Socialista Integrado"
            };
            if ui_refinada::botao_estilo_menu(ui, btn_txt).clicked() {
                comandos.push(Comando::AlternarComecon);
            }
        }
    });
}
