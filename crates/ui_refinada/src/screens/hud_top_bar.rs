//! Barra superior do HUD (Top Bar - 48px).
//! Identidade nacional, métricas do Tesouro com tooltips aninhados, DEFCON e controles de tempo.

use bevy_egui::egui::{Color32, RichText, Stroke, Ui, Vec2};

use crate::theme::paleta::{status, superficies, texto};
use crate::widgets::nested_tooltip::{render_decomposicao_tooltip, ItemDecomposicao};
use crate::view_model::{
    top_bar::{TopBarViewModel, VelocidadeJogo},
    EstadoUiGlobal, TelaAtiva,
};

/// Renderiza o HUD superior horizontal fixo.
pub fn render_top_bar(
    ui: &mut Ui,
    vm: &mut TopBarViewModel,
    estado_global: &mut EstadoUiGlobal,
) {
    ui.horizontal_centered(|ui| {
        // ---------------------------------------------------------------------
        // 1. Identidade Nacional & Tag
        // ---------------------------------------------------------------------
        ui.horizontal(|ui| {
            let tag_color = match vm.tag_pais.as_str() {
                "USA" => Color32::from_rgb(45, 95, 175),
                "SOV" => Color32::from_rgb(175, 25, 25),
                _ => Color32::from_rgb(108, 127, 95),
            };

            let (tag_rect, _) = ui.allocate_exact_size(Vec2::new(38.0, 24.0), egui::Sense::hover());
            ui.painter().rect_filled(tag_rect, 1.0, tag_color);
            ui.painter().rect_stroke(tag_rect, 1.0, Stroke::new(1.0, superficies::BORDER_STRONG), bevy_egui::egui::StrokeKind::Inside);
            ui.painter().text(
                tag_rect.center(),
                egui::Align2::CENTER_CENTER,
                &vm.tag_pais,
                egui::FontId::monospace(12.0),
                Color32::WHITE,
            );

            ui.vertical(|ui| {
                ui.label(
                    RichText::new(&vm.nome_pais)
                        .strong()
                        .color(texto::PRIMARIO),
                );
                ui.label(
                    RichText::new(format!("{} • {}", vm.classificacao_ranking, vm.nome_lider))
                        .small()
                        .color(texto::SECUNDARIO),
                );
            });
        });

        ui.separator();

        // ---------------------------------------------------------------------
        // 2. Finanças: Tesouro & Balanço com Tooltip Aninhado Victoria 3
        // ---------------------------------------------------------------------
        let tesouro_label = ui.label(
            RichText::new(format!("US$ {:.1}M", vm.tesouro_milhoes))
                .strong()
                .color(texto::OURO_DESTAQUE),
        );

        tesouro_label.on_hover_ui(|ui| {
            render_decomposicao_tooltip(
                ui,
                "Tesouro Nacional",
                &format!("US$ {:.1} Milhões", vm.tesouro_milhoes),
                Some("Saldo líquido de reservas fiscais do governo central"),
                &[
                    ItemDecomposicao {
                        descricao: "Receitas Fiscais Diárias".to_string(),
                        valor: 125.4,
                        eh_positivo: true,
                        detalhe_extra: Some("Impostos de renda corporativo, consumo e tarifas".to_string()),
                    },
                    ItemDecomposicao {
                        descricao: "Despesas Operacionais Diárias".to_string(),
                        valor: 82.5,
                        eh_positivo: false,
                        detalhe_extra: Some("Forças armadas, burocracia e subsídios".to_string()),
                    },
                    ItemDecomposicao {
                        descricao: "Rendimentos em Ouro (Fort Knox)".to_string(),
                        valor: 12.8,
                        eh_positivo: true,
                        detalhe_extra: Some("Paridade fixa de US$ 35/oz lastreada".to_string()),
                    },
                ],
                Some("Clique na tecla [F5] para acessar a Tela de Orçamento e ajustar alíquotas tributárias."),
            );
        });

        // Balanço diário colorido
        let cor_balanco = if vm.balanco_diario >= 0.0 {
            status::SUCESSO
        } else {
            status::PERIGO
        };

        let balanco_texto = format!("{:+.1}k $/d", vm.balanco_diario / 1000.0);
        let balanco_label = ui.label(RichText::new(balanco_texto).strong().color(cor_balanco));

        balanco_label.on_hover_ui(|ui| {
            render_decomposicao_tooltip(
                ui,
                "Balanço Fiscal Diário",
                &format!("{:+.2}k US$/dia", vm.balanco_diario / 1000.0),
                Some("Projeção semanal: +US$ 14.2 Milhões"),
                &[
                    ItemDecomposicao {
                        descricao: "Imposto Corporativo de Guerra".to_string(),
                        valor: 58.2,
                        eh_positivo: true,
                        detalhe_extra: None,
                    },
                    ItemDecomposicao {
                        descricao: "Manutenção do Exército e Frota".to_string(),
                        valor: 62.5,
                        eh_positivo: false,
                        detalhe_extra: None,
                    },
                    ItemDecomposicao {
                        descricao: "Tarifas de Exportação Lend-Lease".to_string(),
                        valor: 14.3,
                        eh_positivo: true,
                        detalhe_extra: None,
                    },
                ],
                Some("Manter superávit fiscal garante liquidez sem necessidade de emitir novos títulos da dívida."),
            );
        });

        ui.separator();

        // ---------------------------------------------------------------------
        // 3. Capacidade Industrial (IC) & PIB
        // ---------------------------------------------------------------------
        let ic_label = ui.label(
            RichText::new(format!("IC: {} / {}", vm.ic_alocada, vm.ic_total))
                .strong()
                .color(texto::PRIMARIO),
        );
        ic_label.on_hover_text(format!(
            "Capacidade Industrial Nacional: {} ativas de {} totais (Ocupação: {:.0}%)",
            vm.ic_alocada,
            vm.ic_total,
            (vm.ic_alocada as f32 / vm.ic_total as f32) * 100.0
        ));

        let pib_label = ui.label(
            RichText::new(format!("PIB: ${:.1}B", vm.pib_bilhoes))
                .color(texto::SECUNDARIO),
        );
        pib_label.on_hover_text(format!(
            "Produto Interno Bruto Anual: US$ {:.1} Bilhões (+{:.1}% a.a.)",
            vm.pib_bilhoes, vm.crescimento_pib_pct
        ));

        ui.separator();

        // ---------------------------------------------------------------------
        // 4. Legitimidade, Estabilidade & Tensão DEFCON
        // ---------------------------------------------------------------------
        let legit_label = ui.label(RichText::new(format!("Legit: {:.0}%", vm.legitimidade_pct)).strong().color(texto::PRIMARIO));
        legit_label.on_hover_ui(|ui| {
            render_decomposicao_tooltip(
                ui,
                "Legitimidade Governamental",
                &format!("{:.0}% (Governo Firme)", vm.legitimidade_pct),
                Some("Apoio parlamentar e coesão das bancadas no congresso"),
                &[
                    ItemDecomposicao {
                        descricao: "Bancada Democrata de Apoio a FDR".to_string(),
                        valor: 53.4,
                        eh_positivo: true,
                        detalhe_extra: None,
                    },
                    ItemDecomposicao {
                        descricao: "Aprovação do Estado-Maior Militar".to_string(),
                        valor: 20.0,
                        eh_positivo: true,
                        detalhe_extra: None,
                    },
                    ItemDecomposicao {
                        descricao: "Atrito Salarial com Sindicatos".to_string(),
                        valor: 8.0,
                        eh_positivo: false,
                        detalhe_extra: None,
                    },
                ],
                Some("Legitimidade acima de 75% acelera em +25% a aprovação de reformas e leis no Parlamento."),
            );
        });

        ui.label(format!("Estab: {:.0}%", vm.estabilidade_pct));

        // Termômetro DEFCON
        crate::widgets::meter::render_defcon_badge(ui, vm.nivel_defcon);

        ui.separator();

        // ---------------------------------------------------------------------
        // 5. Botões Rápidos de Telas (Navegação F1-F8)
        // ---------------------------------------------------------------------
        let abas = [
            (TelaAtiva::MapaGeralHud, "Mapa [ESC]"),
            (TelaAtiva::PainelPais, "País [F1]"),
            (TelaAtiva::PainelEstado, "Estado [F2]"),
            (TelaAtiva::Mercado, "Mercado [F3]"),
            (TelaAtiva::Construcao, "Obras [F4]"),
            (TelaAtiva::Orcamento, "Tesouro [F5]"),
            (TelaAtiva::Politica, "Política [F6]"),
            (TelaAtiva::Diplomacia, "Diplomacia [F7]"),
            (TelaAtiva::RankingPotencias, "Ranking [F8]"),
        ];

        for (tela, titulo) in abas {
            let eh_ativa = estado_global.tela_ativa == tela;
            let botao = if eh_ativa {
                egui::Button::new(RichText::new(titulo).strong().color(texto::OURO_DESTAQUE))
            } else {
                egui::Button::new(RichText::new(titulo).small().color(texto::SECUNDARIO))
            };

            if ui.add(botao).clicked() {
                estado_global.tela_ativa = tela;
            }
        }

        // ---------------------------------------------------------------------
        // 6. Calendário e Controles de Velocidade (Extrema Direita)
        // ---------------------------------------------------------------------
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Notificações / Alertas
            let badge_alerta = format!("✉ {}", vm.alertas_pendentes);
            if ui.button(RichText::new(badge_alerta).color(texto::OURO_DESTAQUE)).clicked() {
                estado_global.tela_ativa = TelaAtiva::MapaGeralHud;
            }

            // Data atual formatada estilo teletipo
            ui.label(
                RichText::new(&vm.data_formatada)
                    .monospace()
                    .strong()
                    .color(Color32::WHITE),
            );

            // Botões de Velocidade 1 a 5
            for v in [5, 4, 3, 2, 1] {
                let ativo = vm.velocidade.nivel() == v;
                let txt = RichText::new(format!("{}", v)).strong();
                let btn = if ativo {
                    egui::Button::new(txt.color(Color32::BLACK))
                        .fill(texto::OURO_DESTAQUE)
                } else {
                    egui::Button::new(txt.color(texto::SECUNDARIO))
                };

                if ui.add(btn).clicked() {
                    vm.velocidade = match v {
                        1 => VelocidadeJogo::Velocidade1,
                        2 => VelocidadeJogo::Velocidade2,
                        3 => VelocidadeJogo::Velocidade3,
                        4 => VelocidadeJogo::Velocidade4,
                        _ => VelocidadeJogo::Velocidade5,
                    };
                    vm.esta_pausado = false;
                }
            }

            // Botão Pausar / Continuar
            let icone_pausa = if vm.esta_pausado { "▶ Continuar" } else { "❚❚ Pausar" };
            if ui.button(icone_pausa).clicked() {
                vm.esta_pausado = !vm.esta_pausado;
            }
        });
    });
}
