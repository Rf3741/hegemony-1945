//! Camada jogável: país do jogador, HUD, painel do país (1), ranking e notificações.
use super::enciclopedia::{dica_com, dica_conceito, texto_notificacao};
use super::economia_sim::{Comando, EconomiaSim, VistaBem, VistaEconomia};
use super::economy::RecursoMundo;
use super::mercado_ui::Icones;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use sim_economia::{
    CategoriaPredio, CatalogoEconomia, MetodoProducao, Predio, Provincia, SlotMetodo,
    TipoPredio, materiais_por_ponto,
};
use simulacao::mundo::EstadoPais;
use super::politica_sim::PoliticaSim;
use std::collections::{BTreeMap, BTreeSet};

/// Tag do país controlado pelo jogador.
#[derive(Resource, Debug, Clone)]
pub struct PaisJogador(pub String);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Aba {
    #[default]
    Economia,
    Construcao,
    Politica,
    Ranking,
}

#[derive(Resource, Default)]
pub struct PainelPais {
    pub aberto: bool,
    pub aba: Aba,
    /// Tipo de prédio escolhido para construir (índice no catálogo).
    pub construir_tipo: usize,
    /// Estado (id) escolhido para construir.
    pub construir_estado: String,
    /// Categoria selecionada no planejador de obras (None = Todas)
    pub filtro_cat_construir: Option<CategoriaPredio>,
    /// Filtro na lista de prédios: se true, exibe apenas os que dão prejuízo
    pub apenas_prejuizo: bool,
    /// Filtro de texto para pesquisar prédios/províncias
    pub filtro_texto_predios: String,
}

/// Mensagens recentes (texto, instante em segundos de relógio real, repetições).
#[derive(Resource, Default)]
pub struct Notificacoes(Vec<(String, f64, u32)>, Vec<String>);

impl Notificacoes {
    /// Textos novos desde a última chamada (usado pelo áudio).
    pub fn drenar_novas(&mut self) -> Vec<String> {
        std::mem::take(&mut self.1)
    }

    /// Converte as mensagens legadas dos simuladores em modelos da enciclopédia.
    pub fn mensagem_modelada(texto: &str, pais: &str) -> String {
        if texto == "Obra concluída!" || texto.starts_with("Obra concluída: ") {
            // Toast curto (1-2 linhas): "Obra concluída: Ateliê de Alta-Costura em Manus".
            return match texto.strip_prefix("Obra concluída: ") {
                Some(curto) => format!("Obra concluída: {curto}"),
                None => "Obra concluída: um novo empreendimento".to_string(),
            };
        }
        if let Some(lei) = texto.strip_prefix("Lei aprovada: ") {
            return texto_notificacao(
                "msg_lei_aprovada",
                &[
                    ("pais", pais.into()),
                    ("lei", lei.into()),
                    ("grupo_favorecido", "a coalizão governista".into()),
                    ("grupo_opositor", "a oposição parlamentar".into()),
                ],
                texto,
            );
        }
        if texto.starts_with("Eleição realizada") {
            let candidato = texto.split_once("vitória de ").map_or("-", |(_, v)| v);
            return texto_notificacao(
                "msg_eleicoes_presidenciais",
                &[
                    ("pais", pais.into()),
                    ("candidato_vencedor", candidato.into()),
                    ("partido_vencedor", candidato.into()),
                    ("percentual_votos", "maioria".into()),
                ],
                texto,
            );
        }
        if let Some(resto) = texto.strip_prefix("GUERRA: ").or_else(|| texto.strip_prefix("GUERRA CIVIL ")) {
            return texto_notificacao(
                "msg_guerra_deflagrada",
                &[
                    ("guerra", "Conflito internacional".into()),
                    ("agressor", pais.into()),
                    ("defensor", "nação adversária".into()),
                    ("contexto", resto.into()),
                ],
                texto,
            );
        }
        if let Some(resto) = texto.strip_prefix("PAZ: ") {
            return texto_notificacao(
                "msg_paz_assinada",
                &[
                    ("guerra", "conflito internacional".into()),
                    ("proponente", pais.into()),
                    ("termos", resto.into()),
                ],
                texto,
            );
        }
        texto.to_string()
    }
    pub fn push(&mut self, texto: impl Into<String>, agora: f64) {
        let texto = texto.into();
        self.1.push(texto.clone());
        self.0.retain(|(_, instante, _)| agora - *instante < 6.0);
        if let Some((_, instante, repeticoes)) = self.0.iter_mut().find(|(t, _, _)| *t == texto) {
            *instante = agora;
            *repeticoes += 1;
            return;
        }
        self.0.push((texto, agora, 1));
        if self.0.len() > 40 {
            self.0.remove(0);
        }
    }
}

/// Estado anterior do país do jogador, para detectar eventos.
#[derive(Resource, Default)]
struct Anterior {
    tag: String,
    negativo: bool,
    obras_na_fila: usize,
    calote: bool,
}

pub struct JogadorPlugin;

impl Plugin for JogadorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PainelPais>()
            .init_resource::<Notificacoes>()
            .init_resource::<Anterior>()
            .add_systems(
                Update,
                (alternar_painel, detectar_eventos).run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (painel_pais.run_if(resource_exists::<EconomiaSim>), toasts).run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida),
            );
    }
}

/// Formata valores monetários de forma compacta (mil, mi, bi, tri).
pub fn fmt_din(v: f64) -> String {
    let a = v.abs();
    let s = if a >= 1e12 {
        format!("{:.2} tri", a / 1e12)
    } else if a >= 1e9 {
        format!("{:.2} bi", a / 1e9)
    } else if a >= 1e6 {
        format!("{:.2} mi", a / 1e6)
    } else if a >= 1e3 {
        format!("{:.1} mil", a / 1e3)
    } else {
        format!("{a:.0}")
    };
    format!("{}${}", if v < 0.0 { "-" } else { "" }, s)
}

pub fn fmt_pop(v: u64) -> String {
    let v = v as f64;
    if v >= 1e9 {
        format!("{:.2} bi", v / 1e9)
    } else if v >= 1e6 {
        format!("{:.2} mi", v / 1e6)
    } else if v >= 1e3 {
        format!("{:.0} mil", v / 1e3)
    } else {
        format!("{v:.0}")
    }
}

pub fn cor_pais(p: &EstadoPais) -> egui::Color32 {
    egui::Color32::from_rgb(p.cor.0, p.cor.1, p.cor.2)
}

fn cor_saldo(v: f64) -> egui::Color32 {
    ui_refinada::cor_saldo(v)
}

/// Faixa do HUD com os indicadores do país do jogador (economia vinda do `sim_economia`).
pub fn hud_pais(ui: &mut egui::Ui, p: &EstadoPais, eco: &EconomiaSim, painel_aberto: &mut bool, mercado_aberto: &mut bool, comercio_aberto: &mut bool, construcao_aberta: &mut bool) {
    ui.horizontal_wrapped(|ui| {
        if !super::bandeiras_ui::bandeira_ui(ui, &p.tag, 18.0) {
            let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 14.0), egui::Sense::hover());
            ui.painter().rect_filled(r, 2.0, cor_pais(p));
        }
        ui.strong(&p.nome);
        ui.separator();
        match (&eco.vista.pais, eco.vista.pais_id.is_empty()) {
            (Some(pe), false) => {
                let o = &pe.orcamento;
                dica_com(ui.label(format!("Tesouro: {}", fmt_din(o.tesouro))), "tooltip_topbar_tesouro", |ui| {
                    ui.label(format!("Tesouro atual: {} | Dívida: {} | Saldo/dia: {}", fmt_din(o.tesouro), fmt_din(o.divida), fmt_din(o.saldo_diario)));
                });
                dica_com(ui.colored_label(cor_saldo(o.saldo_diario), format!("Saldo/dia: {}", fmt_din(o.saldo_diario))), "tooltip_topbar_balanco_diario", |ui| {
                    ui.label(format!("Saldo diário atual: {} (por ano: {})", fmt_din(o.saldo_diario), fmt_din(o.saldo_diario * 365.0)));
                });
                dica_conceito(ui.label(format!("Dívida: {}", fmt_din(o.divida))), "divida_publica");
                ui.separator();
                dica_conceito(ui.label(format!("PIB: {}", fmt_din(eco.vista.pib_anual))), "produtividade_do_trabalho");
                dica_conceito(ui.label(format!("Pop.: {}", fmt_pop(eco.vista.populacao as u64))), "pop");
                dica_conceito(ui.label(format!("Padrão de vida: {:.1}", eco.vista.padrao_vida)), "padrao_de_vida");
                dica_conceito(ui.label(format!("Desemprego: {:.1}%", eco.vista.desemprego * 100.0)), "estrato_social");
                if o.em_calote {
                    ui.colored_label(ui_refinada::menu::PERIGO, "CALOTE");
                }
            }
            _ => {
                ui.label(format!("PIB: {}", fmt_din(p.pib_anual_estimado)));
                ui.label(format!("Pop.: {}", fmt_pop(p.populacao_total())));
                ui.label("(economia detalhada indisponível para este país)");
            }
        }
        dica_com(ui.label(format!("Legitimidade: {:.0}%", p.politica.legitimidade)), "tooltip_topbar_legitimidade", |ui| {
            ui.label(format!("Legitimidade atual: {:.1}%", p.politica.legitimidade));
        });
        if eco.atraso_dias() > 10 || eco.dias_perdidos > 0 {
            ui.colored_label(ui_refinada::menu::AVISO, format!("Economia atrasada {} d", eco.atraso_dias()));
        }
        ui.separator();
        let b_constr = ui.add(
            egui::Button::new(egui::RichText::new("🏗 Construir (B)").strong().size(16.0).color(ui_refinada::menu::PAPEL))
                .fill(ui_refinada::menu::VERMELHO)
                .stroke(egui::Stroke::new(1.0, ui_refinada::menu::TINTA)),
        ).on_hover_text("Abre a tela de construção");
        super::tutorial_guiado::marcar(&b_constr, "btn_construir");
        if b_constr.clicked() {
            *construcao_aberta = !*construcao_aberta;
        }
        let b_pais = ui.button("País (1)");
        super::tutorial_guiado::marcar(&b_pais, "btn_pais");
        if b_pais.clicked() {
            *painel_aberto = !*painel_aberto;
        }
        let b_merc = ui.button("Mercado (2)");
        super::tutorial_guiado::marcar(&b_merc, "btn_mercado");
        if b_merc.clicked() {
            *mercado_aberto = !*mercado_aberto;
        }
        let b_com = ui.button("Comércio (3)");
        super::tutorial_guiado::marcar(&b_com, "btn_comercio");
        if b_com.clicked() {
            *comercio_aberto = !*comercio_aberto;
        }
        ui.label("Tecnologia (5)");
        ui.label("Enciclopédia (8)");
    });
}

/// Resumo de um país qualquer (usado no painel da província).
pub fn resumo_pais(ui: &mut egui::Ui, p: &EstadoPais) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 12.0), egui::Sense::hover());
        ui.painter().rect_filled(r, 2.0, cor_pais(p));
        ui.strong(format!("{} ({})", p.nome, p.tag));
    });
    ui.label(format!("PIB: {}", fmt_din(p.pib_anual_estimado)));
    ui.label(format!("População: {}", fmt_pop(p.populacao_total())));
    ui.label(format!("Ideologia: {}", p.ideologia.replace('_', " ")));
    ui.label(format!("Legitimidade: {:.0}%", p.politica.legitimidade));
    ui.label(format!("Padrão de vida: {:.1}", p.padrao_vida_medio()));
    if let Some(s) = &p.suserano {
        ui.label(format!("Suserano: {s}"));
    }
}

fn alternar_painel(teclas: Res<ButtonInput<KeyCode>>, mut painel: ResMut<PainelPais>) {
    if teclas.just_pressed(KeyCode::Digit1) || teclas.just_pressed(KeyCode::Numpad1) {
        painel.aberto = !painel.aberto;
    }
}

fn detectar_eventos(
    jogador: Res<PaisJogador>,
    mundo: Res<RecursoMundo>,
    mut eco: Option<ResMut<EconomiaSim>>,
    mut pol: Option<ResMut<PoliticaSim>>,
    tempo: Res<Time<Real>>,
    mut ant: ResMut<Anterior>,
    mut notas: ResMut<Notificacoes>,
) {
    let Some(p) = mundo.paises.get(&jogador.0) else { return };
    let agora = tempo.elapsed_secs_f64();
    if let Some(pol) = pol.as_mut() {
        for m in pol.mensagens.drain(..) {
            notas.push(Notificacoes::mensagem_modelada(&m, &p.nome), agora);
        }
    }
    let mut calote = ant.calote;
    if let Some(eco) = eco.as_mut() {
        for m in eco.mensagens.drain(..) {
            notas.push(Notificacoes::mensagem_modelada(&m, &p.nome), agora);
        }
        if let Some(pe) = &eco.vista.pais {
            ant.obras_na_fila = pe.fila_construcao.len();
            calote = pe.orcamento.em_calote;
            if ant.tag == jogador.0 && calote && !ant.calote {
                let juros = format!("{:.1}", pe.orcamento.taxa_juros_efetiva_anual * 100.0);
                let txt = super::enciclopedia::texto_notificacao("msg_tesouro_insolvente", &[("pais", p.nome.clone()), ("taxa_juros", juros)], "Calote soberano: dívida acima do limite de crédito!");
                notas.push(txt, agora);
            }
        }
    }

    ant.calote = calote;
    let negativo = calote;
    if ant.tag == jogador.0 {
        if negativo && !ant.negativo {
            notas.push("O tesouro está zerado e o país está se endividando!", agora);
        }
    } else {
        notas.push(format!("Você governa: {}", p.nome), agora);
    }
    ant.tag = jogador.0.clone();
    ant.negativo = negativo;
}

fn toasts(mut contextos: EguiContexts, tempo: Res<Time<Real>>, mut notas: ResMut<Notificacoes>) -> Result {
    let agora = tempo.elapsed_secs_f64();
    notas.0.retain(|(_, t, _)| agora - t < 6.0);
    if notas.0.is_empty() {
        return Ok(());
    }
    // Acima dos botões fixos "Rankings (7)" e "Configurações" (canto inferior esquerdo).
    egui::Area::new("toasts".into()).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -64.0]).show(contextos.ctx_mut()?, |ui| {
        for (txt, _, repeticoes) in notas.0.iter().rev().take(3) {
            egui::Frame::NONE
                .fill(ui_refinada::menu::PAPEL)
                .stroke(egui::Stroke::new(1.5, ui_refinada::menu::VERMELHO))
                .inner_margin(8.0)
                .shadow(egui::Shadow {
                    offset: [2, 3],
                    blur: 6,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(70),
                })
                .show(ui, |ui| {
                    let msg = if *repeticoes > 1 {
                        format!("{txt} ×{repeticoes}")
                    } else {
                        txt.clone()
                    };
                    ui.label(egui::RichText::new(msg).color(ui_refinada::menu::TINTA).strong());
                });
        }
    });
    Ok(())
}

fn painel_pais(
    mut contextos: EguiContexts,
    mut painel: ResMut<PainelPais>,
    mut eco: ResMut<EconomiaSim>,
    mut pol: Option<ResMut<PoliticaSim>>,
    mut icones: ResMut<Icones>,
    elenco: Option<Res<super::personagens_sim::ElencoRes>>,
    mut retratos: Option<ResMut<super::personagens_ui::Retratos>>,
    relogio: Res<super::time::RecursoRelogio>,
) -> Result {
    if !painel.aberto {
        return Ok(());
    }
    let hoje = (relogio.data.ano, relogio.data.mes, relogio.data.dia);
    let mut aberto = true;
    let mut aba = painel.aba;
    let ctx = contextos.ctx_mut()?;
    let (tag_p, nome_p) = pol.as_ref().and_then(|p| p.vista.pais.as_ref().map(|x| (x.tag.clone(), x.nome.clone()))).unwrap_or_default();
    ui_refinada::janela_estilo_menu("Painel do país (1)")
        .open(&mut aberto)
        .default_size([720.0, 600.0])
        .show(ctx, |ui| {
            ui_refinada::decorar_janela(ui);
            if !tag_p.is_empty() {
                ui.horizontal(|ui| {
                    let tl = super::politica_sim::tag_legado_pub(&tag_p);
                    super::bandeiras_ui::bandeira_ui(ui, &tl, 40.0);
                    let titulo_txt = format!("PAÍS (1) — {}", super::bandeiras_ui::nome_dinamico(ui, &tl).unwrap_or(nome_p.clone()));
                    ui_refinada::cabecalho_painel(ui, &titulo_txt);
                });
            } else {
                ui_refinada::cabecalho_painel(ui, "PAÍS (1)");
            }
            ui.horizontal(|ui| {
                if ui_refinada::aba_estilo_menu(ui, aba == Aba::Economia, "Economia") { aba = Aba::Economia; }
                if ui_refinada::aba_estilo_menu(ui, aba == Aba::Construcao, "Construção") { aba = Aba::Construcao; }
                if ui_refinada::aba_estilo_menu(ui, aba == Aba::Politica, "Política") { aba = Aba::Politica; }
                if ui_refinada::aba_estilo_menu(ui, aba == Aba::Ranking, "Ranking") { aba = Aba::Ranking; }
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| match aba {
                Aba::Economia => aba_economia(ui, &mut eco, &mut painel, &mut icones),
                Aba::Construcao => aba_construcao(ui, &mut eco, &mut painel, &mut icones),
                Aba::Politica => match pol.as_mut() {
                    Some(p) => {
                        if let (Some(el), Some(r), Some(tag)) = (elenco.as_deref(), retratos.as_deref_mut(), p.tag.clone()) {
                            super::personagens_ui::painel_lideres(ui, el, r, &tag, hoje);
                            ui.separator();
                        }
                        super::politica_ui::aba_politica(ui, p)
                    }
                    None => {
                        ui.label("Política ainda carregando...");
                    }
                },
                Aba::Ranking => aba_ranking(ui, &eco.vista),
            });
        });
    painel.aba = aba;
    painel.aberto = aberto;
    Ok(())
}

fn linha_fluxo(ui: &mut egui::Ui, nome: &str, v: f64) {
    ui.label(nome);
    ui.colored_label(cor_saldo(v), fmt_din(v));
    ui.end_row();
}

fn aba_economia(ui: &mut egui::Ui, eco: &mut EconomiaSim, painel: &mut PainelPais, icones: &mut Icones) {
    let mut cmds: Vec<Comando> = Vec::new();
    {
        let v = &eco.vista;
        let (Some(pais), Some(_cat)) = (&v.pais, &v.catalogo) else {
            ui.label("Sem dados econômicos detalhados para este país (ou a economia ainda está calculando).");
            return;
        };
        let o = &pais.orcamento;

        // ---------------- Orçamento ----------------
        ui.heading("Orçamento (por dia)");
        ui.label(format!("Tesouro {} | Dívida {} | Limite de crédito {} | Juros {:.1}% a.a.", fmt_din(o.tesouro), fmt_din(o.divida), fmt_din(o.limite_credito), o.taxa_juros_efetiva_anual * 100.0));
        ui.columns(2, |cols| {
            egui::Grid::new("receitas").num_columns(2).show(&mut cols[0], |ui| {
                ui.strong("Receitas");
                ui.end_row();
                linha_fluxo(ui, "Imposto de renda e dividendos", o.receita_imposto_renda);
                linha_fluxo(ui, "Imposto de consumo", o.receita_imposto_consumo);
                linha_fluxo(ui, "Imposto sobre a terra", o.receita_imposto_terra);
                linha_fluxo(ui, "Empresas estatais", o.receita_dividendos_estatais);
                linha_fluxo(ui, "Tarifas de importação", o.receita_tarifas);
                linha_fluxo(ui, "Turismo e hotelaria", o.receita_turismo);
            });
            egui::Grid::new("despesas").num_columns(2).show(&mut cols[1], |ui| {
                ui.strong("Despesas");
                ui.end_row();
                linha_fluxo(ui, "Burocracia", -o.gasto_burocracia);
                linha_fluxo(ui, "Forças armadas", -o.gasto_militar);
                linha_fluxo(ui, "Setor de construção", -o.gasto_construcao);
                linha_fluxo(ui, "Materiais das obras", -o.gasto_obras_materiais);
                linha_fluxo(ui, "Subsídios", -o.gasto_subsidios);
                linha_fluxo(ui, "Juros da dívida", -o.gasto_juros);
            });
        });
        ui.horizontal(|ui| {
            ui.strong("Saldo diário:");
            ui.colored_label(cor_saldo(o.saldo_diario), fmt_din(o.saldo_diario));
            if o.em_calote {
                ui.colored_label(egui::Color32::LIGHT_RED, "País em calote (dívida no limite de crédito)!");
            }
        });
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(120, 200, 255), format!("💼 Fundo de Investimento Privado: {}", fmt_din(pais.fundo_investimento_privado)));
            ui.label(format!("(+{}/d aportes, -{}/d obras)", fmt_din(pais.aporte_fundo_privado_ultimo_tick), fmt_din(pais.gasto_fundo_privado_ultimo_tick)));
        });
        let pf = &pais.politica_fiscal;
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Imposto de renda: {:.0}% / {:.0}% / {:.0}% (baixo/médio/alto), dividendos {:.0}%", pf.aliquota_imposto_renda_baixo * 100.0, pf.aliquota_imposto_renda_medio * 100.0, pf.aliquota_imposto_renda_alto * 100.0, pf.aliquota_imposto_dividendos * 100.0));
            if ui.button("Impostos -10%").clicked() {
                cmds.push(Comando::EscalarImposto(0.9));
            }
            if ui.button("Impostos +10%").clicked() {
                cmds.push(Comando::EscalarImposto(1.1));
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Tarifa de importação: {:.0}% (afeta a abertura ao mercado mundial e a arrecadação)", pf.tarifa_importacao * 100.0));
            if ui.button("-2 pp").clicked() {
                cmds.push(Comando::AjustarTarifa(-0.02));
            }
            if ui.button("+2 pp").clicked() {
                cmds.push(Comando::AjustarTarifa(0.02));
            }
        });
        ui.label(format!("Lei de comércio: {} | Eficiência de arrecadação {:.0}%", pf.lei_comercial_id.replace('_', " "), pais.eficiencia_arrecadacao * 100.0));
        ui.separator();

        // ---------------- População ----------------
        ui.heading("População");
        ui.label(format!("PIB {} | Padrão de vida médio {:.1} | Desemprego {:.1}%", fmt_din(v.pib_anual), v.padrao_vida, v.desemprego * 100.0));
        egui::Grid::new("pops").striped(true).show(ui, |ui| {
            for h in ["Tipo", "Habitantes", "Empregados", "Renda/pessoa/dia", "Padrão de vida"] {
                ui.strong(h);
            }
            ui.end_row();
            for p in &v.pops {
                ui.horizontal(|ui| {
                    icones.mostrar(ui, &p.id, 16.0);
                    ui.label(&p.nome);
                });
                ui.label(fmt_pop(p.populacao as u64));
                ui.label(fmt_pop(p.empregados as u64));
                ui.label(format!("{:.2}", p.renda_per_capita()));
                ui.label(format!("{:.1}", p.padrao_vida()));
                ui.end_row();
            }
        });
        ui.separator();

        // ---------------- Construção & Prédios (Resumo) ----------------
        ui.heading("Construção & Prédios");
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Capacidade: {:.0} pts/dia | Obras na fila: {} | Gasto diário em materiais: {}", v.pontos_construcao_dia, pais.fila_construcao.len(), fmt_din(o.gasto_obras_materiais)));
            if ui.button("🏗 Ir para a aba Construção e Prédios (Gestão Completa)").clicked() {
                painel.aba = Aba::Construcao;
            }
        });
    }
    for c in cmds {
        eco.enviar(c);
    }
}

fn aba_ranking(ui: &mut egui::Ui, v: &VistaEconomia) {
    let mut paises: Vec<&super::economia_sim::ResumoPaisEco> = v.ranking.iter().collect();
    ui.columns(3, |cols| {
        paises.sort_by(|a, b| b.pib_anual.total_cmp(&a.pib_anual));
        cols[0].heading("Maiores PIBs");
        for (i, p) in paises.iter().take(10).enumerate() {
            let t = format!("{}. {} - {}", i + 1, p.nome, fmt_din(p.pib_anual));
            if p.id == v.pais_id { cols[0].strong(t); } else { cols[0].label(t); }
        }
        paises.sort_by(|a, b| b.populacao.total_cmp(&a.populacao));
        cols[1].heading("Maiores populações");
        for (i, p) in paises.iter().take(10).enumerate() {
            let t = format!("{}. {} - {}", i + 1, p.nome, fmt_pop(p.populacao as u64));
            if p.id == v.pais_id { cols[1].strong(t); } else { cols[1].label(t); }
        }
        paises.retain(|p| p.populacao > 1.0e6);
        paises.sort_by(|a, b| b.padrao_vida.total_cmp(&a.padrao_vida));
        cols[2].heading("Maior padrão de vida");
        for (i, p) in paises.iter().take(10).enumerate() {
            let t = format!("{}. {} - {:.1}", i + 1, p.nome, p.padrao_vida);
            if p.id == v.pais_id { cols[2].strong(t); } else { cols[2].label(t); }
        }
    });
}

fn rotulo_slot(slot: SlotMetodo) -> &'static str {
    match slot {
        SlotMetodo::ProducaoPrincipal => "Produção Principal",
        SlotMetodo::Automacao => "Automação",
        SlotMetodo::Propriedade => "Regime de Propriedade",
        SlotMetodo::Secundario => "Processo Secundário",
    }
}

pub(crate) struct LucratividadeEsperada {
    pub(crate) receita: f64,
    pub(crate) custo_insumos: f64,
    pub(crate) custo_folha: f64,
    pub(crate) lucro_liquido: f64,
    pub(crate) margem: f64,
    pub(crate) vagas_totais: f64,
    pub(crate) produtividade: f64,
}

pub(crate) fn calcular_lucratividade_esperada(
    cat: &CatalogoEconomia,
    tipo: &TipoPredio,
    metodos: &[String],
    bens: &[VistaBem],
    salario_base: f64,
) -> LucratividadeEsperada {
    let preco_de = |bid: &str| -> f64 {
        bens.iter()
            .find(|b| b.id == bid)
            .map(|b| b.preco_nacional)
            .unwrap_or_else(|| cat.bens.iter().find(|b| b.id == bid).map_or(20.0, |b| b.preco_base))
    };

    let mut map_produtos: BTreeMap<String, f64> = BTreeMap::new();
    let mut map_insumos: BTreeMap<String, f64> = BTreeMap::new();
    let mut map_empregos: BTreeMap<String, i32> = BTreeMap::new();

    for mid in metodos {
        if let Some(m) = cat.metodos_producao.iter().find(|m| &m.id == mid) {
            for (bid, qtd) in &m.produtos {
                *map_produtos.entry(bid.clone()).or_insert(0.0) += *qtd;
            }
            for (bid, qtd) in &m.insumos {
                *map_insumos.entry(bid.clone()).or_insert(0.0) += *qtd;
            }
            for (pid, delta) in &m.empregos {
                *map_empregos.entry(pid.clone()).or_insert(0) += *delta;
            }
        }
    }

    let mut receita = 0.0;
    for (bid, qtd) in map_produtos {
        receita += qtd * preco_de(&bid);
    }

    let mut custo_insumos = 0.0;
    for (bid, qtd) in map_insumos {
        custo_insumos += qtd * preco_de(&bid);
    }

    let mut custo_folha = 0.0;
    let mut vagas_totais = 0.0;
    for (pid, delta) in map_empregos {
        let mult = cat.tipos_pop.iter().find(|tp| tp.id == pid).map_or(1.0, |tp| tp.multiplicador_salario);
        if delta > 0 {
            let v = delta as f64;
            vagas_totais += v;
            custo_folha += v * salario_base * mult;
        }
    }

    let lucro_liquido = if tipo.financiado_pelo_governo {
        -(custo_insumos + custo_folha)
    } else {
        receita - custo_insumos - custo_folha
    };

    let margem = if receita > 1e-6 { (lucro_liquido / receita) * 100.0 } else { 0.0 };
    let produtividade = if vagas_totais > 0.0 { (receita - custo_insumos) / vagas_totais } else { 0.0 };

    LucratividadeEsperada {
        receita,
        custo_insumos,
        custo_folha,
        lucro_liquido,
        margem,
        vagas_totais,
        produtividade,
    }
}

pub(crate) struct CustoMateriaisObra {
    pub(crate) itens: Vec<(String, f64, f64, f64)>, // (nome_bem, qtd_total, preco_un, custo_total)
    pub(crate) total_reais: f64,
}

pub(crate) fn calcular_custo_materiais_obra(
    cat: &CatalogoEconomia,
    tipo: &TipoPredio,
    bens: &[VistaBem],
) -> CustoMateriaisObra {
    let preco_de = |bid: &str| -> f64 {
        bens.iter()
            .find(|b| b.id == bid)
            .map(|b| b.preco_nacional)
            .unwrap_or_else(|| cat.bens.iter().find(|b| b.id == bid).map_or(20.0, |b| b.preco_base))
    };
    let nome_bem = |bid: &str| -> String {
        cat.bens.iter().find(|b| b.id == bid).map_or_else(|| bid.to_string(), |b| b.nome.clone())
    };

    let mut itens = Vec::new();
    let mut total_reais = 0.0;
    for &(bem_id, qtd_ponto) in materiais_por_ponto(tipo.categoria) {
        let qtd_total = qtd_ponto * tipo.custo_pontos_construcao;
        let pu = preco_de(bem_id);
        let ct = qtd_total * pu;
        total_reais += ct;
        itens.push((nome_bem(bem_id), qtd_total, pu, ct));
    }

    CustoMateriaisObra { itens, total_reais }
}

fn formatar_efeito_troca_metodo(
    cat: &CatalogoEconomia,
    bens: &[VistaBem],
    metodo_novo_id: &str,
    metodo_antigo_id: &str,
    niveis: u32,
    escopo_label: &str,
) -> String {
    let Some(novo) = cat.metodos_producao.iter().find(|m| m.id == metodo_novo_id) else {
        return format!("Trocar para {metodo_novo_id}");
    };
    let antigo = cat.metodos_producao.iter().find(|m| m.id == metodo_antigo_id);

    let nome_bem = |bid: &str| -> String {
        cat.bens.iter().find(|b| b.id == bid).map_or_else(|| bid.to_string(), |b| b.nome.clone())
    };
    let nome_pop = |pid: &str| -> String {
        cat.tipos_pop.iter().find(|p| p.id == pid).map_or_else(|| pid.to_string(), |p| p.nome.clone())
    };
    let preco_de = |bid: &str| -> f64 {
        bens.iter().find(|b| b.id == bid).map(|b| b.preco_nacional).unwrap_or(20.0)
    };

    let mut lines = Vec::new();
    lines.push(format!("Trocar para: {} ({})", novo.nome, rotulo_slot(novo.slot)));
    lines.push(format!("Alvo: {} ({} níveis)\n", escopo_label, niveis.max(1)));

    // Insumos
    let mut insumos_deltas = Vec::new();
    let mut todos_insumos: BTreeSet<&String> = novo.insumos.keys().collect();
    if let Some(a) = antigo {
        todos_insumos.extend(a.insumos.keys());
    }
    for &bid in &todos_insumos {
        let q_novo = novo.insumos.get(bid).copied().unwrap_or(0.0);
        let q_antigo = antigo.and_then(|a| a.insumos.get(bid)).copied().unwrap_or(0.0);
        let d = q_novo - q_antigo;
        if d.abs() > 1e-5 {
            let val = d * preco_de(bid);
            let sinal = if d > 0.0 { "+" } else { "" };
            insumos_deltas.push(format!("  • {}{:.1} {}/dia ({}{})", sinal, d, nome_bem(bid), sinal, fmt_din(val)));
        }
    }
    if !insumos_deltas.is_empty() {
        lines.push("Efeito em Insumos (por nível):".into());
        lines.extend(insumos_deltas);
    } else {
        lines.push("Insumos: Sem alteração".into());
    }

    // Saídas / Produtos
    let mut produtos_deltas = Vec::new();
    let mut todos_produtos: BTreeSet<&String> = novo.produtos.keys().collect();
    if let Some(a) = antigo {
        todos_produtos.extend(a.produtos.keys());
    }
    for &bid in &todos_produtos {
        let q_novo = novo.produtos.get(bid).copied().unwrap_or(0.0);
        let q_antigo = antigo.and_then(|a| a.produtos.get(bid)).copied().unwrap_or(0.0);
        let d = q_novo - q_antigo;
        if d.abs() > 1e-5 {
            let val = d * preco_de(bid);
            let sinal = if d > 0.0 { "+" } else { "" };
            produtos_deltas.push(format!("  • {}{:.1} {}/dia ({}{})", sinal, d, nome_bem(bid), sinal, fmt_din(val)));
        }
    }
    if !produtos_deltas.is_empty() {
        lines.push("\nEfeito em Saídas (por nível):".into());
        lines.extend(produtos_deltas);
    } else {
        lines.push("\nSaídas: Sem alteração".into());
    }

    // Empregos
    let mut empregos_deltas = Vec::new();
    let mut todos_empregos: BTreeSet<&String> = novo.empregos.keys().collect();
    if let Some(a) = antigo {
        todos_empregos.extend(a.empregos.keys());
    }
    for &pid in &todos_empregos {
        let e_novo = novo.empregos.get(pid).copied().unwrap_or(0);
        let e_antigo = antigo.and_then(|a| a.empregos.get(pid)).copied().unwrap_or(0);
        let d = e_novo - e_antigo;
        if d != 0 {
            let sinal = if d > 0 { "+" } else { "" };
            empregos_deltas.push(format!("  • {}{} vagas de {}", sinal, d, nome_pop(pid)));
        }
    }
    if !empregos_deltas.is_empty() {
        lines.push("\nEfeito em Empregos (por nível):".into());
        lines.extend(empregos_deltas);
    } else {
        lines.push("\nEmpregos: Sem alteração".into());
    }

    lines.push("\n👉 Clique para trocar o método de produção com 1 clique!".into());
    lines.join("\n")
}

pub fn aba_construcao(
    ui: &mut egui::Ui,
    eco: &mut EconomiaSim,
    painel: &mut PainelPais,
    icones: &mut Icones,
) {
    let mut cmds: Vec<Comando> = Vec::new();
    {
        let v = &eco.vista;
        let (Some(pais), Some(cat)) = (&v.pais, &v.catalogo) else {
            ui.label("Sem dados econômicos detalhados para este país (ou a economia ainda está calculando).");
            return;
        };

        // Identifica prédios com prejuízo no país inteiro
        let mut predios_com_prejuizo = 0usize;
        let mut prejuizo_total = 0.0f64;
        for pr in &v.provincias {
            for b in &pr.predios {
                let financiado = cat.tipos_predio.iter().find(|t| t.id == b.tipo_predio_id).map_or(false, |t| t.financiado_pelo_governo);
                if !financiado && b.lucro_ultimo_tick < -0.01 {
                    predios_com_prejuizo += 1;
                    prejuizo_total += b.lucro_ultimo_tick.abs();
                }
            }
        }

        // =========================================================================
        // 1. BANNER DE RESUMO NACIONAL & ALERTA DE PREJUÍZO
        // =========================================================================
        let cota_priv = pais.alocacao_privada_lei();
        let cota_est = pais.alocacao_estatal_lei();
        let lei_nome = pais.politica_fiscal.lei_sistema_economico_id.replace('_', " ");

        ui.heading("Construção & Prédios (Estilo Victoria 3)");
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Capacidade de Construção: {:.0} pts/dia (máx. 25 pts/obra)", v.pontos_construcao_dia));
            ui.separator();
            ui.label(format!(
                "Alocação da Lei: {:.0}% Estatal ({:.1} pts/d) | {:.0}% Privado ({:.1} pts/d) [{}]",
                cota_est * 100.0,
                v.pontos_construcao_dia * cota_est,
                cota_priv * 100.0,
                v.pontos_construcao_dia * cota_priv,
                lei_nome
            ));
            ui.separator();
            ui.label(format!("Gasto estatal com materiais: {}", fmt_din(pais.orcamento.gasto_obras_materiais)));
        });

        if predios_com_prejuizo > 0 {
            ui.add_space(4.0);
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(120, 20, 20, 60))
                .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_RED))
                .inner_margin(8.0)
                .corner_radius(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::LIGHT_RED, "⚠ ALERTA ECONÔMICO:");
                        ui.label(format!(
                            "{} instalações estão operando com prejuízo no país (perda agregada de -{}/dia).",
                            predios_com_prejuizo,
                            fmt_din(prejuizo_total)
                        ));
                        ui.separator();
                        ui.checkbox(&mut painel.apenas_prejuizo, "Exibir apenas prédios dando prejuízo");
                    });
                });
        }
        ui.separator();

        // =========================================================================
        // 2A. FILA DE OBRAS PÚBLICAS / ESTATAIS (DECISÃO DO GOVERNO)
        // =========================================================================
        let obras_estatais: Vec<(usize, &sim_economia::ItemConstrucao)> = pais
            .fila_construcao
            .iter()
            .enumerate()
            .filter(|(_, item)| !item.privado)
            .collect();
        let n_estatais = obras_estatais.len();

        ui.horizontal(|ui| {
            ui.heading(format!("🏛 Obras Públicas e Estatais ({n_estatais})"));
            ui.label(format!("Cota reservada: {:.1} pts/dia | Financiamento: Orçamento Público", v.pontos_construcao_dia * cota_est));
        });

        if n_estatais == 0 {
            ui.label("A fila estatal está vazia. Escolha um prédio e um estado abaixo no planejador para encomendar uma obra pública.");
        } else {
            let pts_est_disponiveis = (v.pontos_construcao_dia * cota_est).max(1.0);
            let max_por_obra = 25.0f64;
            let mut pts_restantes_hoje = pts_est_disponiveis;
            let mut dias_acumulados = 0.0f64;

            egui::Grid::new("grid_fila_estatal")
                .striped(true)
                .min_col_width(50.0)
                .show(ui, |ui| {
                    ui.strong("#");
                    ui.strong("Prédio");
                    ui.strong("Localização");
                    ui.strong("Progresso");
                    ui.strong("Ritmo");
                    ui.strong("Tempo Estimado");
                    ui.strong("Reordenar");
                    ui.strong("Ação");
                    ui.end_row();

                    for (pos_ordem, &(idx_original, item)) in obras_estatais.iter().enumerate() {
                        let nome_t = cat.tipos_predio.iter().find(|t| t.id == item.tipo_predio_id)
                            .map_or(item.tipo_predio_id.as_str(), |t| t.nome.as_str());
                        let prov = v.provincias.iter().find(|p| p.id == item.provincia_id);
                        let nome_p = prov.map_or(item.provincia_id.as_str(), |p| p.nome.as_str());
                        let nome_e = prov.and_then(|pr| v.estados.iter().find(|e| e.id == pr.estado_id))
                            .map_or("", |e| e.nome.as_str());

                        let frac = (item.progresso_pontos / item.custo_total_pontos.max(1.0)).clamp(0.0, 1.0) as f32;
                        let falta = (item.custo_total_pontos - item.progresso_pontos).max(0.0);

                        let alocacao_hoje = (pts_restantes_hoje.min(max_por_obra)).min(falta);
                        pts_restantes_hoje = (pts_restantes_hoje - alocacao_hoje).max(0.0);

                        let dias_desta_obra = falta / max_por_obra.min(pts_est_disponiveis);
                        let dias_conclusao = if alocacao_hoje > 0.0 {
                            dias_desta_obra
                        } else {
                            dias_acumulados + dias_desta_obra
                        };
                        dias_acumulados += dias_desta_obra;

                        // #
                        ui.label(format!("{}", pos_ordem + 1));

                        // Prédio
                        ui.horizontal(|ui| {
                            icones.mostrar(ui, &item.tipo_predio_id, 16.0);
                            ui.label(nome_t);
                        });

                        // Localização
                        ui.label(if nome_e.is_empty() { nome_p.to_string() } else { format!("{nome_p} ({nome_e})") });

                        // Progresso
                        ui.add(
                            egui::ProgressBar::new(frac)
                                .desired_width(130.0)
                                .text(format!("{:.0}% ({:.0}/{:.0})", frac * 100.0, item.progresso_pontos, item.custo_total_pontos))
                        );

                        // Ritmo
                        if pais.orcamento.em_calote {
                            ui.colored_label(egui::Color32::LIGHT_RED, "Paralisado (Calote)");
                        } else if alocacao_hoje > 0.0 {
                            ui.colored_label(egui::Color32::LIGHT_GREEN, format!("{alocacao_hoje:.1} pts/d"));
                        } else {
                            ui.colored_label(egui::Color32::GRAY, "Aguardando");
                        }

                        // Tempo Estimado
                        if alocacao_hoje > 0.0 {
                            ui.label(format!("~{dias_conclusao:.0} dias"));
                        } else {
                            ui.label(format!("~{dias_conclusao:.0} d (em fila)"));
                        }

                        // Reordenação na fila do país
                        ui.horizontal(|ui| {
                            if ui.add_enabled(pos_ordem > 0, egui::Button::new("⤒")).on_hover_text("Mover para o topo").clicked() {
                                cmds.push(Comando::MoverObra { de: idx_original, para: 0 });
                            }
                            if ui.add_enabled(pos_ordem > 0, egui::Button::new("⬆")).on_hover_text("Subir 1 posição").clicked() {
                                cmds.push(Comando::MoverObra { de: idx_original, para: idx_original.saturating_sub(1) });
                            }
                            if ui.add_enabled(pos_ordem + 1 < n_estatais, egui::Button::new("⬇")).on_hover_text("Descer 1 posição").clicked() {
                                cmds.push(Comando::MoverObra { de: idx_original, para: (idx_original + 1).min(pais.fila_construcao.len().saturating_sub(1)) });
                            }
                        });

                        // Ação (Cancelar obra pública)
                        if ui.button("❌").on_hover_text("Cancelar esta obra pública").clicked() {
                            cmds.push(Comando::CancelarObra(idx_original));
                        }

                        ui.end_row();
                    }
                });
        }
        ui.add_space(8.0);

        // =========================================================================
        // 2B. SETOR PRIVADO: FUNDO DE INVESTIMENTO E OBRAS AUTÔNOMAS (VICTORIA 3)
        // =========================================================================
        egui::Frame::new()
            .fill(egui::Color32::from_rgba_unmultiplied(22, 32, 48, 220))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 130, 200)))
            .inner_margin(10.0)
            .corner_radius(6.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("💼 Setor Privado: Fundo de Investimento & Obras Autônomas");
                    ui.label(format!("Cota por Lei: {:.0}% ({:.1} pts/dia)", cota_priv * 100.0, v.pontos_construcao_dia * cota_priv));
                });

                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(100, 205, 255), format!("💰 Fundo Acumulado: {}", fmt_din(pais.fundo_investimento_privado)));
                    ui.separator();
                    ui.colored_label(egui::Color32::LIGHT_GREEN, format!("📥 Aportes de Dividendos: +{}/dia", fmt_din(pais.aporte_fundo_privado_ultimo_tick)));
                    ui.separator();
                    ui.colored_label(egui::Color32::from_rgb(255, 185, 110), format!("📤 Custo Materiais Privados: -{}/dia", fmt_din(pais.gasto_fundo_privado_ultimo_tick)));
                    ui.separator();
                    ui.colored_label(egui::Color32::WHITE, format!("⚡ Ritmo: {:.1} pts/dia alocados", pais.pontos_privados_ultimo_tick));
                });
                ui.add_space(6.0);

                let obras_privadas: Vec<&sim_economia::ItemConstrucao> = pais
                    .fila_construcao
                    .iter()
                    .filter(|item| item.privado)
                    .collect();

                if obras_privadas.is_empty() {
                    if pais.exige_propriedade_estatal() {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 150, 150),
                            "🏛 Economia Centralmente Planificada (Gosplan): O setor privado está desativado. 100% dos lucros e investimentos são estatais.",
                        );
                    } else {
                        ui.label("Nenhuma obra privada em andamento no momento. Capitalistas e aristocratas acumulam reservas no Fundo e iniciam novas obras pelo retorno esperado, respeitando o potencial regional e a infraestrutura.");
                    }
                } else {
                    egui::Grid::new("grid_fila_privada")
                        .striped(true)
                        .min_col_width(50.0)
                        .show(ui, |ui| {
                            ui.strong("#");
                            ui.strong("Prédio");
                            ui.strong("Localização");
                            ui.strong("Progresso");
                            ui.strong("Ritmo");
                            ui.strong("Retorno Esperado");
                            ui.strong("Financiamento");
                            ui.end_row();

                            for (i, item) in obras_privadas.iter().enumerate() {
                                let nome_t = cat.tipos_predio.iter().find(|t| t.id == item.tipo_predio_id)
                                    .map_or(item.tipo_predio_id.as_str(), |t| t.nome.as_str());
                                let prov = v.provincias.iter().find(|p| p.id == item.provincia_id);
                                let nome_p = prov.map_or(item.provincia_id.as_str(), |p| p.nome.as_str());
                                let nome_e = prov.and_then(|pr| v.estados.iter().find(|e| e.id == pr.estado_id))
                                    .map_or("", |e| e.nome.as_str());

                                let frac = (item.progresso_pontos / item.custo_total_pontos.max(1.0)).clamp(0.0, 1.0) as f32;

                                ui.label(format!("{}", i + 1));
                                ui.horizontal(|ui| {
                                    icones.mostrar(ui, &item.tipo_predio_id, 16.0);
                                    ui.label(nome_t);
                                });
                                ui.label(if nome_e.is_empty() { nome_p.to_string() } else { format!("{nome_p} ({nome_e})") });
                                ui.add(
                                    egui::ProgressBar::new(frac)
                                        .desired_width(120.0)
                                        .text(format!("{:.0}% ({:.0}/{:.0})", frac * 100.0, item.progresso_pontos, item.custo_total_pontos))
                                );
                                if pais.pontos_privados_ultimo_tick > 0.0 {
                                    ui.colored_label(egui::Color32::LIGHT_GREEN, "Ativo");
                                } else {
                                    ui.colored_label(egui::Color32::GRAY, "Aguardando");
                                }
                                if !item.justificativa.is_empty() {
                                    ui.label(&item.justificativa);
                                } else if item.retorno_esperado > 0.0 {
                                    ui.label(format!("+£{:.0}/dia", item.retorno_esperado));
                                } else {
                                    ui.label("Lucrativo");
                                }
                                ui.colored_label(egui::Color32::from_rgb(120, 200, 255), "💼 100% Fundo Privado");
                                ui.end_row();
                            }
                        });
                }
            });
        ui.separator();

        // =========================================================================
        // 3. PLANEJADOR DE OBRAS: ESCOLHER PRÉDIO E ESTADO
        //    (Custo em bens, pontos de construção e lucratividade esperada)
        // =========================================================================
        egui::CollapsingHeader::new("🏗 Planejar Nova Construção")
            .default_open(true)
            .show(ui, |ui| {
                if painel.construir_tipo >= cat.tipos_predio.len() {
                    painel.construir_tipo = 0;
                }
                if painel.construir_estado.is_empty() || !v.estados.iter().any(|e| e.id == painel.construir_estado) {
                    painel.construir_estado = v.estados.first().map(|e| e.id.clone()).unwrap_or_default();
                }

                // Filtros de Categoria
                ui.horizontal_wrapped(|ui| {
                    ui.strong("Categoria:");
                    if ui.selectable_label(painel.filtro_cat_construir.is_none(), "Todas").clicked() {
                        painel.filtro_cat_construir = None;
                    }
                    for &(cat_predio, label) in &[
                        (CategoriaPredio::RuralAgricola, "Rural / Agrícola"),
                        (CategoriaPredio::Extrativo, "Extrativo / Minas"),
                        (CategoriaPredio::Industrial, "Industrial / Fábricas"),
                        (CategoriaPredio::Infraestrutura, "Infraestrutura"),
                        (CategoriaPredio::Governamental, "Governamental"),
                        (CategoriaPredio::Militar, "Militar"),
                    ] {
                        let sel = painel.filtro_cat_construir == Some(cat_predio);
                        if ui.selectable_label(sel, label).clicked() {
                            painel.filtro_cat_construir = Some(cat_predio);
                            if cat.tipos_predio[painel.construir_tipo].categoria != cat_predio {
                                if let Some(novo_idx) = cat.tipos_predio.iter().position(|t| t.categoria == cat_predio) {
                                    painel.construir_tipo = novo_idx;
                                }
                            }
                        }
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.strong("Prédio:");
                    let t_atual = &cat.tipos_predio[painel.construir_tipo];
                    egui::ComboBox::from_id_salt("cb_planejar_tipo")
                        .width(260.0)
                        .selected_text(&t_atual.nome)
                        .show_ui(ui, |ui| {
                            for (i, t) in cat.tipos_predio.iter().enumerate() {
                                if let Some(filtro) = painel.filtro_cat_construir {
                                    if t.categoria != filtro {
                                        continue;
                                    }
                                }
                                ui.selectable_value(&mut painel.construir_tipo, i, &t.nome);
                            }
                        });

                    ui.strong("Estado:");
                    let nome_est = v.estados.iter().find(|e| e.id == painel.construir_estado)
                        .map_or("-", |e| e.nome.as_str()).to_string();
                    egui::ComboBox::from_id_salt("cb_planejar_estado")
                        .width(240.0)
                        .selected_text(nome_est)
                        .show_ui(ui, |ui| {
                            for e in &v.estados {
                                ui.selectable_value(&mut painel.construir_estado, e.id.clone(), &e.nome);
                            }
                        });
                });

                let t = &cat.tipos_predio[painel.construir_tipo];
                let custo_materiais = calcular_custo_materiais_obra(cat, t, &v.bens);
                let luc_esperada = calcular_lucratividade_esperada(cat, t, &t.metodos_padrao, &v.bens, 4.0);

                ui.add_space(6.0);
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(35, 45, 55, 120))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 90, 110)))
                    .inner_margin(10.0)
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.columns(2, |cols| {
                            // Coluna 1: Custo em Bens e Pontos de Construção
                            cols[0].strong("📦 Custo da Obra (1 Nível)");
                            cols[0].label(format!(
                                "Pontos de Construção: {:.0} pts (~{:.0} dias de obra)",
                                t.custo_pontos_construcao,
                                t.custo_pontos_construcao / v.pontos_construcao_dia.clamp(1.0, 25.0)
                            ));
                            cols[0].label(format!("Custo estimado em materiais: {}", fmt_din(custo_materiais.total_reais)));
                            cols[0].add_space(2.0);
                            cols[0].label("Materiais consumidos pelo setor:");
                            for (nome_bem, qtd, pu, ct) in &custo_materiais.itens {
                                cols[0].label(format!("  • {:.0}x {} (a {} = {})", qtd, nome_bem, fmt_din(*pu), fmt_din(*ct)));
                            }

                            // Coluna 2: Lucratividade Esperada
                            cols[1].strong("📈 Lucratividade Esperada (Por Nível Operando)");
                            if t.financiado_pelo_governo {
                                cols[1].colored_label(egui::Color32::LIGHT_BLUE, "🏛 Prédio Governamental / Serviço Público");
                                cols[1].label(format!("Custo Operacional ao Tesouro: -{}/dia", fmt_din(luc_esperada.custo_insumos + luc_esperada.custo_folha)));
                                cols[1].label(format!("  • Custo de Insumos: -{}/dia", fmt_din(luc_esperada.custo_insumos)));
                                cols[1].label(format!("  • Folha Salarial: -{}/dia ({} vagas)", fmt_din(luc_esperada.custo_folha), fmt_pop(luc_esperada.vagas_totais as u64)));
                            } else {
                                cols[1].horizontal(|ui| {
                                    ui.label("Lucro Líquido Estimado:");
                                    ui.colored_label(cor_saldo(luc_esperada.lucro_liquido), format!("{}/dia (margem {:.1}%)", fmt_din(luc_esperada.lucro_liquido), luc_esperada.margem));
                                });
                                cols[1].label(format!("Lucro Anual Estimado: {}", fmt_din(luc_esperada.lucro_liquido * 365.0)));
                                cols[1].label(format!("Produtividade Esperada: {}/trabalhador/dia", fmt_din(luc_esperada.produtividade)));
                                cols[1].label(format!("Receita de Vendas: +{}/dia", fmt_din(luc_esperada.receita)));
                                cols[1].label(format!("Custo de Insumos: -{}/dia", fmt_din(luc_esperada.custo_insumos)));
                                cols[1].label(format!("Folha Salarial: -{}/dia", fmt_din(luc_esperada.custo_folha)));
                            }
                        });
                    });

                // Províncias do Estado Selecionado & Potencial Regional
                ui.add_space(6.0);
                let estado_sel = v.estados.iter().find(|e| e.id == painel.construir_estado);
                let provs_estado: Vec<&Provincia> = v.provincias.iter()
                    .filter(|p| p.estado_id == painel.construir_estado)
                    .collect();

                let nome_est_label = estado_sel.map_or("-", |e| e.nome.as_str());

                // Cartão de Potencial Regional (Victoria 3)
                if let Some(est) = estado_sel {
                    egui::Frame::new()
                        .fill(egui::Color32::from_rgba_unmultiplied(25, 35, 45, 90))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 80, 100)))
                        .inner_margin(8.0)
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.strong(format!("🌍 Potencial de {nome_est_label}:"));
                                if est.terra_aravel > 0 {
                                    ui.label(format!("Terra Arável: {}/{} níveis", est.terra_aravel_ocupada(), est.terra_aravel));
                                }
                                if !est.culturas_permitidas.is_empty() {
                                    ui.label(format!("Culturas: {}", est.culturas_permitidas.join(", ")));
                                }
                            });
                            if !est.limites_recursos.is_empty() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.strong("Jazidas Naturais:");
                                    for (rk, max_v) in &est.limites_recursos {
                                        let b_str = est.qualidade_recursos.get(rk)
                                            .map_or("".to_string(), |q| format!(" (+{:.0}%)", (q - 1.0) * 100.0));
                                        ui.label(format!("• {}: máx. {}{}", rk.replace('_', " "), max_v, b_str));
                                    }
                                });
                            }
                        });
                    ui.add_space(4.0);
                }

                ui.strong(format!("Províncias em {nome_est_label} ({} províncias):", provs_estado.len()));
                egui::Grid::new("grid_construir_provincias")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Província");
                        ui.strong("Nível Atual");
                        ui.strong("Capacidade / Potencial");
                        ui.strong("Ação");
                        ui.end_row();

                        for pr in provs_estado {
                            let ok = v.comporta(pr, painel.construir_tipo);
                            let nivel_atual = pr.predios.iter()
                                .find(|b| b.tipo_predio_id == t.id)
                                .map_or(0, |b| b.nivel);

                            ui.label(&pr.nome);
                            ui.label(format!("Nível {nivel_atual}"));

                            // Info detalhada de potencial e capacidade
                            let mut msg_bloqueio = "Capacidade máxima esgotada ou recurso ausente.";
                            if let Some(est) = estado_sel {
                                if t.usa_terra_aravel {
                                    if !est.cultura_permitida(&t.id) {
                                        ui.colored_label(egui::Color32::LIGHT_RED, "❌ Cultura não permitida no estado");
                                        msg_bloqueio = "Cultura agrícola incompatível com o clima e solo deste estado.";
                                    } else {
                                        let niveis_agricolas: u32 = pr.predios.iter()
                                            .filter(|b| cat.tipos_predio.iter().find(|tp| tp.id == b.tipo_predio_id).map_or(false, |tp| tp.usa_terra_aravel))
                                            .map(|b| b.nivel)
                                            .sum();
                                        let livre = pr.terra_aravel_total.saturating_sub(niveis_agricolas);
                                        let b_q = est.fator_qualidade_predio(t);
                                        let q_tag = if b_q > 1.0 { format!(" (+{:.0}% bônus)", (b_q - 1.0) * 100.0) } else { String::new() };
                                        ui.label(format!("Terra livre: {}/{} prov. (estado: {}/livre){}", livre, pr.terra_aravel_total, est.terra_aravel_livre(), q_tag));
                                        msg_bloqueio = "Terra arável insuficiente na província ou no estado.";
                                    }
                                } else if let Some(rec) = &t.recurso_natural_requerido {
                                    let lim_est = est.limite_recurso(rec);
                                    if lim_est == 0 {
                                        ui.colored_label(egui::Color32::LIGHT_RED, format!("❌ Sem jazida de {rec} no estado"));
                                        msg_bloqueio = "Não há jazida deste recurso natural no estado.";
                                    } else {
                                        let max_rec = pr.limites_recursos.get(rec).copied().unwrap_or(lim_est);
                                        let b_q = est.fator_qualidade_predio(t);
                                        let q_tag = if b_q > 1.0 { format!(" (+{:.0}% bônus)", (b_q - 1.0) * 100.0) } else { String::new() };
                                        ui.label(format!("Jazida {rec}: {nivel_atual}/{max_rec} (estado máx: {lim_est}){}", q_tag));
                                        msg_bloqueio = "Capacidade de extração da jazida no estado foi atingida.";
                                    }
                                } else {
                                    ui.label("Espaço urbano disponível");
                                }
                            } else {
                                ui.label("Espaço urbano disponível");
                            }

                            if ui.add_enabled(ok, egui::Button::new("➕ Construir +1"))
                                .on_disabled_hover_text(msg_bloqueio)
                                .on_hover_text(format!("Enfileirar a construção de +1 nível de {} em {}", t.nome, pr.nome))
                                .clicked()
                            {
                                cmds.push(Comando::Construir {
                                    provincia_id: pr.id.clone(),
                                    tipo_id: t.id.clone(),
                                });
                            }

                            ui.end_row();
                        }
                    });
            });
        ui.separator();

        // =========================================================================
        // 4. PARQUE PRODUTIVO NACIONAL (LISTA DE PRÉDIOS AGRUPADA POR TIPO)
        //    (Com nível, método de produção de 1 clique, empregados/vagas, salários,
        //     lucro e produtividade. Destaque aos prédios dando prejuízo).
        // =========================================================================
        ui.horizontal(|ui| {
            ui.heading("Parque Produtivo Nacional");
            ui.separator();
            ui.label("Buscar:");
            ui.text_edit_singleline(&mut painel.filtro_texto_predios);
            if !painel.filtro_texto_predios.is_empty() && ui.small_button("Limpar").clicked() {
                painel.filtro_texto_predios.clear();
            }
            ui.separator();
            ui.checkbox(&mut painel.apenas_prejuizo, "⚠ Apenas Prédios com Prejuízo");
        });
        ui.label("Agrupado por tipo. Troque métodos de produção com 1 clique (para o país todo no cabeçalho ou individualmente na província).");

        for (ti, t) in cat.tipos_predio.iter().enumerate() {
            let linhas: Vec<(&Provincia, &Predio)> = v.provincias.iter()
                .flat_map(|p| p.predios.iter().filter(|b| b.tipo_predio_id == t.id).map(move |b| (p, b)))
                .collect();

            if linhas.is_empty() {
                continue;
            }

            // Filtro por texto
            let busca = painel.filtro_texto_predios.to_lowercase();
            if !busca.is_empty() {
                let nome_bate = t.nome.to_lowercase().contains(&busca) || t.id.to_lowercase().contains(&busca);
                let prov_bate = linhas.iter().any(|(p, _)| p.nome.to_lowercase().contains(&busca));
                if !nome_bate && !prov_bate {
                    continue;
                }
            }

            let niveis: u32 = linhas.iter().map(|(_, b)| b.nivel).sum();
            let emp: f64 = linhas.iter().map(|(_, b)| b.total_empregados()).sum();
            let vagas: f64 = linhas.iter().map(|(_, b)| b.total_vagas()).sum();
            let lucro: f64 = linhas.iter().map(|(_, b)| b.lucro_ultimo_tick).sum();
            let rec: f64 = linhas.iter().map(|(_, b)| b.receita_ultimo_tick).sum();
            let ins: f64 = linhas.iter().map(|(_, b)| b.custo_insumos_ultimo_tick).sum();
            let produtividade: f64 = if emp > 0.0 { (rec - ins) / emp } else { 0.0 };

            // Quantos prédios deste tipo dão prejuízo
            let perdas: Vec<(&Provincia, &Predio)> = linhas.iter()
                .filter(|(_, b)| !t.financiado_pelo_governo && b.lucro_ultimo_tick < -0.01)
                .copied()
                .collect();
            let tem_prejuizo = !perdas.is_empty();

            if painel.apenas_prejuizo && !tem_prejuizo {
                continue;
            }

            // Cabeçalho do Grupo
            let alerta_prej = if tem_prejuizo {
                format!(" ⚠ [{} COM PREJUÍZO]", perdas.len())
            } else {
                String::new()
            };

            let cabecalho = format!(
                "{} (Nível {}) | Empregos: {}/{} ({:.0}%) | Lucro: {} | Produtividade: {}/trab{}",
                t.nome,
                niveis,
                fmt_pop(emp as u64),
                fmt_pop(vagas as u64),
                if vagas > 0.0 { (emp / vagas) * 100.0 } else { 0.0 },
                fmt_din(lucro),
                fmt_din(produtividade),
                alerta_prej
            );

            let header_resp = egui::CollapsingHeader::new(cabecalho)
                .id_salt(("pred_novo", ti))
                .default_open(tem_prejuizo || !painel.filtro_texto_predios.is_empty());

            header_resp.show(ui, |ui| {
                // Barra de Métodos de Produção Nacional (1 CLIQUE PARA O PAÍS INTEIRO)
                ui.horizontal_wrapped(|ui| {
                    icones.mostrar(ui, &t.id, 24.0);
                    ui.strong("Métodos Nacionais (1 clique):");
                });

                for slot in [SlotMetodo::ProducaoPrincipal, SlotMetodo::Automacao, SlotMetodo::Propriedade, SlotMetodo::Secundario] {
                    let opcoes: Vec<&MetodoProducao> = t.metodos_permitidos.iter()
                        .filter_map(|mid| cat.metodos_producao.iter().find(|m| &m.id == mid))
                        .filter(|m| m.slot == slot)
                        .collect();
                    if opcoes.len() < 2 {
                        continue;
                    }

                    let ativos_primeiro = &linhas[0].1.metodos_ativos;
                    let ativo_id = ativos_primeiro.iter().find(|mid| opcoes.iter().any(|m| &m.id == *mid)).cloned();

                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{}:", rotulo_slot(slot)));
                        for m in &opcoes {
                            let esta_ativo = ativo_id.as_deref() == Some(m.id.as_str());
                            let tooltip = formatar_efeito_troca_metodo(
                                cat,
                                &v.bens,
                                &m.id,
                                ativo_id.as_deref().unwrap_or(""),
                                niveis,
                                &format!("Todo o país ({} prédios)", linhas.len())
                            );

                            let rotulo_btn = if esta_ativo {
                                format!("✓ {}", m.nome)
                            } else {
                                m.nome.clone()
                            };

                            let trava = v.bloqueio_metodos.get(&m.id);
                            let tooltip = match trava {
                                Some(tec) => format!("Requer tecnologia: {tec}"),
                                None => tooltip,
                            };
                            let resp = ui.add_enabled(!esta_ativo && trava.is_none(), egui::Button::new(rotulo_btn))
                                .on_hover_text(tooltip);

                            if resp.clicked() && !esta_ativo && trava.is_none() {
                                cmds.push(Comando::TrocarMetodoTodos {
                                    tipo_id: t.id.clone(),
                                    metodo_id: m.id.clone(),
                                });
                            }
                        }
                    });
                }

                ui.add_space(4.0);
                // Tabela Detalhada das Instâncias nas Províncias
                let mut ord = linhas.clone();
                // Ordena: primeiro os que dão prejuízo, depois por nível decrescente
                ord.sort_by(|a, b| {
                    let prej_a = !t.financiado_pelo_governo && a.1.lucro_ultimo_tick < -0.01;
                    let prej_b = !t.financiado_pelo_governo && b.1.lucro_ultimo_tick < -0.01;
                    match (prej_a, prej_b) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => b.1.nivel.cmp(&a.1.nivel),
                    }
                });

                egui::Grid::new(("grade_detalhe_predio", ti))
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Província");
                        ui.strong("Nível");
                        ui.strong("Métodos Ativos (Troca 1 clique)");
                        ui.strong("Empregados / Vagas");
                        ui.strong("Salário Base");
                        ui.strong("Lucro / Dia");
                        ui.strong("Produtividade");
                        ui.strong("Status / Insumos");
                        ui.strong("Ação");
                        ui.end_row();

                        for (pr, b) in ord {
                            let eh_prej = !t.financiado_pelo_governo && b.lucro_ultimo_tick < -0.01;
                            let emp_b = b.total_empregados();
                            let prod_b = if emp_b > 0.0 {
                                (b.receita_ultimo_tick - b.custo_insumos_ultimo_tick) / emp_b
                            } else {
                                0.0
                            };

                            // Província (com destaque vermelho se prejuízo)
                            if eh_prej {
                                ui.colored_label(egui::Color32::LIGHT_RED, format!("⚠ {}", pr.nome));
                            } else {
                                ui.label(&pr.nome);
                            }

                            // Nível
                            ui.label(b.nivel.to_string());

                            // Métodos Ativos com botão de troca rápida individual (1 clique)
                            ui.horizontal_wrapped(|ui| {
                                for mid in &b.metodos_ativos {
                                    if let Some(m) = cat.metodos_producao.iter().find(|m| &m.id == mid) {
                                        let alternativas: Vec<&MetodoProducao> = t.metodos_permitidos.iter()
                                            .filter_map(|alt_id| cat.metodos_producao.iter().find(|alt| &alt.id == alt_id))
                                            .filter(|alt| alt.slot == m.slot && alt.id != m.id && !v.bloqueio_metodos.contains_key(&alt.id))
                                            .collect();

                                        if alternativas.is_empty() {
                                            ui.label(&m.nome);
                                        } else {
                                            ui.menu_button(&m.nome, |ui| {
                                                ui.label(format!("Slot: {:?}", m.slot));
                                                ui.separator();
                                                for alt in alternativas {
                                                    let tt = formatar_efeito_troca_metodo(
                                                        cat,
                                                        &v.bens,
                                                        &alt.id,
                                                        &m.id,
                                                        b.nivel,
                                                        &format!("{} ({})", pr.nome, t.nome)
                                                    );
                                                    if ui.button(&alt.nome).on_hover_text(tt).clicked() {
                                                        cmds.push(Comando::TrocarMetodo {
                                                            provincia_id: pr.id.clone(),
                                                            tipo_id: t.id.clone(),
                                                            metodo_id: alt.id.clone(),
                                                        });
                                                        ui.close();
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                            });

                            // Empregados / Vagas
                            ui.label(format!("{}/{} ({:.0}%)", fmt_pop(emp_b as u64), fmt_pop(b.total_vagas() as u64), b.taxa_ocupacao() * 100.0));

                            // Salário Base
                            ui.label(format!("{:.2}", b.salario_base));

                            // Lucro Diário (com destaque explícito em vermelho se prejuízo)
                            if eh_prej {
                                ui.colored_label(egui::Color32::LIGHT_RED, format!("⚠ {}", fmt_din(b.lucro_ultimo_tick)));
                            } else {
                                ui.colored_label(cor_saldo(b.lucro_ultimo_tick), fmt_din(b.lucro_ultimo_tick));
                            }

                            // Produtividade
                            ui.label(format!("{}/trab", fmt_din(prod_b)));

                            // Status / Fluxo
                            if eh_prej {
                                ui.colored_label(
                                    egui::Color32::LIGHT_RED,
                                    format!("PREJUÍZO (suprim. {:.0}%)", b.fator_insumos() * 100.0)
                                );
                            } else if t.financiado_pelo_governo {
                                ui.colored_label(egui::Color32::LIGHT_BLUE, "🏛 Governamental");
                            } else {
                                ui.colored_label(
                                    egui::Color32::LIGHT_GREEN,
                                    format!("Lucrativo (suprim. {:.0}%)", b.fator_insumos() * 100.0)
                                );
                            }

                            // Ação rápida: Construir +1 se província comportar
                            let ok_mais_um = v.comporta(pr, ti);
                            if ui.add_enabled(ok_mais_um, egui::Button::new("➕ +1"))
                                .on_hover_text(format!("Construir +1 nível de {} em {}", t.nome, pr.nome))
                                .on_disabled_hover_text("Sem terra livre ou limite de jazida atingido nesta província.")
                                .clicked()
                            {
                                cmds.push(Comando::Construir {
                                    provincia_id: pr.id.clone(),
                                    tipo_id: t.id.clone(),
                                });
                            }

                            ui.end_row();
                        }
                    });
            });
        }
    }

    for c in cmds {
        eco.enviar(c);
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::path::Path;

    fn carregar_catalogo_teste() -> CatalogoEconomia {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
        CatalogoEconomia::carregar_diretorio(&dir).expect("catálogo econômico de 1945")
    }

    #[test]
    fn painel_pais_tem_aba_construcao_e_filtros() {
        let mut p = PainelPais::default();
        assert_eq!(p.aba, Aba::Economia);
        p.aba = Aba::Construcao;
        assert_eq!(p.aba, Aba::Construcao);
        assert!(!p.apenas_prejuizo);
        p.apenas_prejuizo = true;
        assert!(p.apenas_prejuizo);
    }

    #[test]
    fn calculo_materiais_obra_retorna_bens_e_custo() {
        let cat = carregar_catalogo_teste();
        let sid = cat.tipos_predio.iter().find(|t| t.id == "siderurgica").expect("siderurgica");
        let custo = calcular_custo_materiais_obra(&cat, sid, &[]);
        assert!(!custo.itens.is_empty(), "deve exigir bens para construir");
        assert!(custo.total_reais > 0.0, "custo em bens deve ser > 0");
    }

    #[test]
    fn calculo_lucratividade_esperada_funciona() {
        let cat = carregar_catalogo_teste();
        let sid = cat.tipos_predio.iter().find(|t| t.id == "siderurgica").expect("siderurgica");
        let luc = calcular_lucratividade_esperada(&cat, sid, &sid.metodos_padrao, &[], 4.0);
        assert!(luc.receita > 0.0, "siderurgica deve gerar receita");
        assert!(luc.custo_insumos > 0.0, "siderurgica deve consumir insumos");
        assert!(luc.vagas_totais > 0.0, "siderurgica deve ter postos de trabalho");
        assert!(luc.produtividade.is_finite());
    }

    #[test]
    fn formatacao_efeito_troca_metodo_mostra_deltas() {
        let cat = carregar_catalogo_teste();
        let sid = cat.tipos_predio.iter().find(|t| t.id == "siderurgica").expect("siderurgica");
        let metodo_alvo = &sid.metodos_permitidos[0];
        let texto = formatar_efeito_troca_metodo(
            &cat,
            &[],
            metodo_alvo,
            metodo_alvo,
            5,
            "Nacional",
        );
        assert!(texto.contains("Trocar para:"));
        assert!(texto.contains("Alvo: Nacional (5 níveis)"));
    }
}
