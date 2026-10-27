//! Camada jogável: país do jogador, HUD, painel do país (F1), ranking e notificações.
use super::economia_sim::{Comando, EconomiaSim, VistaEconomia};
use super::economy::RecursoMundo;
use super::mercado_ui::{Icones, fmt_qtd};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use sim_economia::SlotMetodo;
use simulacao::mundo::EstadoPais;
use simulacao::politica::CategoriaLei;
use std::collections::BTreeMap;

/// Tag do país controlado pelo jogador.
#[derive(Resource, Debug, Clone)]
pub struct PaisJogador(pub String);

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Aba {
    #[default]
    Economia,
    Politica,
    Ranking,
}

#[derive(Resource, Default)]
pub struct PainelPais {
    pub aberto: bool,
    aba: Aba,
    /// Tipo de prédio escolhido para construir (índice no catálogo).
    construir_tipo: usize,
    /// Estado (id) escolhido para construir.
    construir_estado: String,
}

/// Mensagens recentes (texto, instante em segundos de relógio real).
#[derive(Resource, Default)]
pub struct Notificacoes(Vec<(String, f64)>);

impl Notificacoes {
    pub fn push(&mut self, texto: impl Into<String>, agora: f64) {
        self.0.push((texto.into(), agora));
        if self.0.len() > 40 {
            self.0.remove(0);
        }
    }
}

/// Estado anterior do país do jogador, para detectar eventos.
#[derive(Resource, Default)]
struct Anterior {
    tag: String,
    leis: BTreeMap<CategoriaLei, String>,
    dias_eleicao: Option<u32>,
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
    if v >= 0.0 { egui::Color32::LIGHT_GREEN } else { egui::Color32::LIGHT_RED }
}

/// Faixa do HUD com os indicadores do país do jogador (economia vinda do `sim_economia`).
pub fn hud_pais(ui: &mut egui::Ui, p: &EstadoPais, eco: &EconomiaSim, painel_aberto: &mut bool, mercado_aberto: &mut bool) {
    ui.horizontal_wrapped(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(r, 2.0, cor_pais(p));
        ui.strong(&p.nome);
        ui.separator();
        match (&eco.vista.pais, eco.vista.pais_id.is_empty()) {
            (Some(pe), false) => {
                let o = &pe.orcamento;
                ui.label(format!("Tesouro: {}", fmt_din(o.tesouro)));
                ui.colored_label(cor_saldo(o.saldo_diario), format!("Saldo/dia: {}", fmt_din(o.saldo_diario)));
                ui.label(format!("Dívida: {}", fmt_din(o.divida)));
                ui.separator();
                ui.label(format!("PIB: {}", fmt_din(eco.vista.pib_anual)));
                ui.label(format!("Pop.: {}", fmt_pop(eco.vista.populacao as u64)));
                ui.label(format!("Padrão de vida: {:.1}", eco.vista.padrao_vida));
                ui.label(format!("Desemprego: {:.1}%", eco.vista.desemprego * 100.0));
                if o.em_calote {
                    ui.colored_label(egui::Color32::LIGHT_RED, "CALOTE");
                }
            }
            _ => {
                ui.label(format!("PIB: {}", fmt_din(p.pib_anual_estimado)));
                ui.label(format!("Pop.: {}", fmt_pop(p.populacao_total())));
                ui.label("(economia detalhada indisponível para este país)");
            }
        }
        ui.label(format!("Legitimidade: {:.0}%", p.politica.legitimidade));
        if eco.atraso_dias() > 10 || eco.dias_perdidos > 0 {
            ui.colored_label(egui::Color32::YELLOW, format!("Economia atrasada {} d", eco.atraso_dias()));
        }
        ui.separator();
        if ui.button("País (F1)").clicked() {
            *painel_aberto = !*painel_aberto;
        }
        if ui.button("Mercado (F2)").clicked() {
            *mercado_aberto = !*mercado_aberto;
        }
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
    if teclas.just_pressed(KeyCode::F1) {
        painel.aberto = !painel.aberto;
    }
}

fn detectar_eventos(
    jogador: Res<PaisJogador>,
    mundo: Res<RecursoMundo>,
    mut eco: Option<ResMut<EconomiaSim>>,
    tempo: Res<Time<Real>>,
    mut ant: ResMut<Anterior>,
    mut notas: ResMut<Notificacoes>,
) {
    let Some(p) = mundo.paises.get(&jogador.0) else { return };
    let agora = tempo.elapsed_secs_f64();
    let (mut obras, mut calote) = (ant.obras_na_fila, ant.calote);
    if let Some(eco) = eco.as_mut() {
        for m in eco.mensagens.drain(..) {
            notas.push(m, agora);
        }
        if let Some(pe) = &eco.vista.pais {
            obras = pe.fila_construcao.len();
            calote = pe.orcamento.em_calote;
            if ant.tag == jogador.0 && obras < ant.obras_na_fila {
                notas.push("Obra concluída!", agora);
            }
            if ant.tag == jogador.0 && calote && !ant.calote {
                notas.push("Calote soberano: dívida acima do limite de crédito!", agora);
            }
        }
    }
    ant.obras_na_fila = obras;
    ant.calote = calote;
    let negativo = calote;
    if ant.tag == jogador.0 {
        if p.politica.dias_ate_eleicao.unwrap_or(0) > ant.dias_eleicao.unwrap_or(0) && ant.dias_eleicao.is_some() {
            notas.push("Eleição realizada: a correlação de forças mudou.", agora);
        }
        for (cat, lei) in &p.politica.leis_ativas {
            if ant.leis.get(cat) != Some(lei) {
                notas.push(format!("Lei aprovada: {} -> {}", cat.nome_pt(), nome_lei(lei)), agora);
            }
        }
        if negativo && !ant.negativo {
            notas.push("O tesouro está zerado e o país está se endividando!", agora);
        }
    } else {
        notas.push(format!("Você governa: {}", p.nome), agora);
    }
    ant.tag = jogador.0.clone();
    ant.leis = p.politica.leis_ativas.clone();
    ant.dias_eleicao = p.politica.dias_ate_eleicao;
    ant.negativo = negativo;
}

fn toasts(mut contextos: EguiContexts, tempo: Res<Time<Real>>, mut notas: ResMut<Notificacoes>) -> Result {
    let agora = tempo.elapsed_secs_f64();
    notas.0.retain(|(_, t)| agora - t < 8.0);
    if notas.0.is_empty() {
        return Ok(());
    }
    egui::Area::new("toasts".into()).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0]).show(contextos.ctx_mut()?, |ui| {
        for (txt, _) in notas.0.iter().rev().take(5) {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(txt);
            });
        }
    });
    Ok(())
}

pub fn nome_lei(chave: &str) -> String {
    let mut s = chave.replace('_', " ");
    if let Some(c) = s.get(0..1) {
        s = c.to_uppercase() + &s[1..];
    }
    s
}

/// Leis selecionáveis em cada categoria.
fn opcoes_lei(cat: CategoriaLei) -> &'static [&'static str] {
    match cat {
        CategoriaLei::Governo => &["oligarquia", "voto_censitario", "voto_capacitario", "sufragio_universal", "partido_unico", "autocracia"],
        CategoriaLei::Economia => &["livre_mercado", "capitalismo_de_intervencao", "economia_planificada"],
        CategoriaLei::Trabalho => &["regulamentacao_basica", "sem_regulamentacao", "sindicatos_fortes"],
        CategoriaLei::DireitosCivis => &["liberdades_restringidas", "liberdades_amplas", "estado_policial"],
        CategoriaLei::SistemaTributario => &["tributacao_regressiva", "tributacao_proporcional", "tributacao_progressiva", "requisicao_estatal"],
        CategoriaLei::SaudeEEducacao => &["escolas_religiosas", "sistema_publico_basico", "sistema_publico_universal"],
        CategoriaLei::Exercito => &["alistamento_obrigatorio", "exercito_profissional", "milicia_popular"],
    }
}

fn painel_pais(
    mut contextos: EguiContexts,
    mut painel: ResMut<PainelPais>,
    jogador: Res<PaisJogador>,
    mut mundo: ResMut<RecursoMundo>,
    mut eco: ResMut<EconomiaSim>,
    mut icones: ResMut<Icones>,
) -> Result {
    if !painel.aberto {
        return Ok(());
    }
    let mut aberto = true;
    let mut aba = painel.aba;
    let ctx = contextos.ctx_mut()?;
    egui::Window::new("Painel do país (F1)").open(&mut aberto).default_size([720.0, 600.0]).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut aba, Aba::Economia, "Economia");
            ui.selectable_value(&mut aba, Aba::Politica, "Política");
            ui.selectable_value(&mut aba, Aba::Ranking, "Ranking");
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| match aba {
            Aba::Economia => aba_economia(ui, &mut eco, &mut painel, &mut icones),
            Aba::Politica => aba_politica(ui, &mut mundo, &jogador.0),
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
        let (Some(pais), Some(cat)) = (&v.pais, &v.catalogo) else {
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

        // ---------------- Construção ----------------
        ui.heading("Construção");
        ui.label(format!("Capacidade: {:.0} pontos de construção por dia (máx. 25 por obra). Obras públicas consomem material de construção, aço e máquinas do mercado e custam ao orçamento.", v.pontos_construcao_dia));
        if pais.fila_construcao.is_empty() {
            ui.label("Fila de obras vazia.");
        }
        for (i, item) in pais.fila_construcao.iter().enumerate() {
            let nome_t = cat.tipos_predio.iter().find(|t| t.id == item.tipo_predio_id).map_or(item.tipo_predio_id.as_str(), |t| t.nome.as_str());
            let nome_p = v.provincias.iter().find(|p| p.id == item.provincia_id).map_or(item.provincia_id.as_str(), |p| p.nome.as_str());
            let frac = (item.progresso_pontos / item.custo_total_pontos.max(1.0)).clamp(0.0, 1.0) as f32;
            let falta = (item.custo_total_pontos - item.progresso_pontos).max(0.0);
            let eta = falta / v.pontos_construcao_dia.clamp(1.0, 25.0);
            ui.horizontal(|ui| {
                icones.mostrar(ui, &item.tipo_predio_id, 16.0);
                ui.add(egui::ProgressBar::new(frac).desired_width(180.0).text(format!("{nome_t} - {nome_p}: {:.0}%", frac * 100.0)));
                ui.label(format!("~{eta:.0} dias"));
                if ui.small_button("Cancelar").clicked() {
                    cmds.push(Comando::CancelarObra(i));
                }
            });
        }
        egui::CollapsingHeader::new("Construir").default_open(false).show(ui, |ui| {
            if painel.construir_tipo >= cat.tipos_predio.len() {
                painel.construir_tipo = 0;
            }
            if painel.construir_estado.is_empty() || !v.estados.iter().any(|e| e.id == painel.construir_estado) {
                painel.construir_estado = v.estados.first().map(|e| e.id.clone()).unwrap_or_default();
            }
            ui.horizontal(|ui| {
                ui.label("Prédio:");
                egui::ComboBox::from_id_salt("cb_tipo").width(220.0).selected_text(&cat.tipos_predio[painel.construir_tipo].nome).show_ui(ui, |ui| {
                    for (i, t) in cat.tipos_predio.iter().enumerate() {
                        ui.selectable_value(&mut painel.construir_tipo, i, &t.nome);
                    }
                });
                ui.label("Estado:");
                let nome_est = v.estados.iter().find(|e| e.id == painel.construir_estado).map_or("-", |e| e.nome.as_str()).to_string();
                egui::ComboBox::from_id_salt("cb_estado").width(220.0).selected_text(nome_est).show_ui(ui, |ui| {
                    for e in &v.estados {
                        ui.selectable_value(&mut painel.construir_estado, e.id.clone(), &e.nome);
                    }
                });
            });
            let t = &cat.tipos_predio[painel.construir_tipo];
            let mat = v.custo_materiais.get(painel.construir_tipo).copied().unwrap_or(0.0);
            ui.label(format!("{}: {:.0} pontos (~{:.0} dias), materiais ~{} a preços de hoje, consumidos ao longo da obra.", t.nome, t.custo_pontos_construcao, t.custo_pontos_construcao / v.pontos_construcao_dia.clamp(1.0, 25.0), fmt_din(mat)));
            let mut n = 0;
            for pr in v.provincias.iter().filter(|p| p.estado_id == painel.construir_estado) {
                if n >= 14 {
                    ui.label("...");
                    break;
                }
                let ok = v.comporta(pr, painel.construir_tipo);
                let nivel_atual = pr.predios.iter().find(|b| b.tipo_predio_id == t.id).map_or(0, |b| b.nivel);
                ui.horizontal(|ui| {
                    ui.label(format!("{} (nível atual {nivel_atual})", pr.nome));
                    if ui.add_enabled(ok, egui::Button::new("Construir +1")).on_disabled_hover_text("Sem terra arável livre ou recurso natural esgotado.").clicked() {
                        cmds.push(Comando::Construir { provincia_id: pr.id.clone(), tipo_id: t.id.clone() });
                    }
                });
                n += 1;
            }
        });
        ui.separator();

        // ---------------- Prédios ----------------
        ui.heading("Prédios");
        ui.label("Agrupados por tipo. Abra um tipo para trocar o método de produção (vale para todos os prédios do tipo) e ver os maiores.");
        for (ti, t) in cat.tipos_predio.iter().enumerate() {
            let linhas: Vec<(&sim_economia::Provincia, &sim_economia::Predio)> = v.provincias.iter().flat_map(|p| p.predios.iter().filter(|b| b.tipo_predio_id == t.id).map(move |b| (p, b))).collect();
            if linhas.is_empty() {
                continue;
            }
            let niveis: u32 = linhas.iter().map(|(_, b)| b.nivel).sum();
            let emp: f64 = linhas.iter().map(|(_, b)| b.total_empregados()).sum();
            let vagas: f64 = linhas.iter().map(|(_, b)| b.total_vagas()).sum();
            let lucro: f64 = linhas.iter().map(|(_, b)| b.lucro_ultimo_tick).sum();
            let cab = format!("{} - nível {} | empregos {}/{} | lucro {}", t.nome, niveis, fmt_pop(emp as u64), fmt_pop(vagas as u64), fmt_din(lucro));
            egui::CollapsingHeader::new(cab).id_salt(("pred", ti)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    icones.mostrar(ui, &t.id, 20.0);
                    if let Some(m) = seletor_metodos(ui, ("sel_todos", ti), cat, t, &linhas[0].1.metodos_ativos, "Aplicar a todos:") {
                        cmds.push(Comando::TrocarMetodoTodos { tipo_id: t.id.clone(), metodo_id: m });
                    }
                });
                let mut ord = linhas.clone();
                ord.sort_by(|a, b| b.1.nivel.cmp(&a.1.nivel));
                egui::Grid::new(("grade_pred", ti)).striped(true).show(ui, |ui| {
                    for h in ["Província", "Nível", "Empregos", "Salário", "Lucro/dia", "Produção/dia", "Insumos/dia"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for (pr, b) in ord.iter().take(12) {
                        ui.label(&pr.nome);
                        ui.label(b.nivel.to_string());
                        ui.label(format!("{}/{}", fmt_pop(b.total_empregados() as u64), fmt_pop(b.total_vagas() as u64)));
                        ui.label(format!("{:.2}", b.salario_base));
                        ui.colored_label(cor_saldo(b.lucro_ultimo_tick), fmt_din(b.lucro_ultimo_tick));
                        let fluxo = |l: &[(u16, f64)]| -> String {
                            let s: Vec<String> = l.iter().map(|(g, q)| format!("{} {}", fmt_qtd(q * b.escala_efetiva()), cat.bens[*g as usize].nome)).collect();
                            if s.is_empty() { "-".to_string() } else { s.join(", ") }
                        };
                        ui.label(fluxo(b.produtos_por_nivel()));
                        ui.label(format!("{} (suprim. {:.0}%)", fluxo(b.insumos_por_nivel()), b.fator_insumos() * 100.0));
                        ui.end_row();
                    }
                });
                if ord.len() > 12 {
                    ui.label(format!("... e mais {} prédios deste tipo.", ord.len() - 12));
                }
            });
        }
    }
    for c in cmds {
        eco.enviar(c);
    }
}

/// Comboboxes dos slots de método de produção. Devolve o método escolhido se o jogador trocou algum.
fn seletor_metodos(ui: &mut egui::Ui, id: (&str, usize), cat: &sim_economia::CatalogoEconomia, t: &sim_economia::TipoPredio, ativos: &[String], rotulo: &str) -> Option<String> {
    let mut escolhido = None;
    ui.label(rotulo);
    for (si, slot) in [SlotMetodo::ProducaoPrincipal, SlotMetodo::Automacao, SlotMetodo::Propriedade, SlotMetodo::Secundario].into_iter().enumerate() {
        let opcoes: Vec<&sim_economia::MetodoProducao> = t.metodos_permitidos.iter().filter_map(|mid| cat.metodos_producao.iter().find(|m| &m.id == mid)).filter(|m| m.slot == slot).collect();
        if opcoes.len() < 2 {
            continue;
        }
        let atual = opcoes.iter().find(|m| ativos.contains(&m.id));
        egui::ComboBox::from_id_salt((id.0, id.1, si)).selected_text(atual.map_or("-", |m| m.nome.as_str())).width(190.0).show_ui(ui, |ui| {
            for m in &opcoes {
                let sel = ativos.contains(&m.id);
                if ui.selectable_label(sel, &m.nome).clicked() && !sel {
                    escolhido = Some(m.id.clone());
                }
            }
        });
    }
    escolhido
}

fn aba_politica(ui: &mut egui::Ui, mundo: &mut RecursoMundo, tag: &str) {
    let Some(p) = mundo.paises.get_mut(tag) else { return };
    ui.heading("Leis");
    if let Some(r) = &p.politica.reforma_ativa {
        ui.colored_label(
            egui::Color32::YELLOW,
            format!(
                "Reforma em andamento: {} -> {} (marco {}/3, {}/30 dias, chance {:.0}%)",
                r.categoria.nome_pt(),
                nome_lei(&r.nova_lei),
                r.marcos_concluidos,
                r.dias_no_marco,
                r.chance_avanco * 100.0
            ),
        );
    } else {
        ui.label("Nenhuma reforma em andamento (só uma por vez).");
    }
    let mut iniciar: Option<(CategoriaLei, &'static str)> = None;
    let reformando = p.politica.reforma_ativa.is_some();
    for cat in CategoriaLei::TODAS {
        let atual = p.politica.lei_ativa(cat).to_string();
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("{}:", cat.nome_pt()));
            ui.label(nome_lei(&atual));
        });
        ui.horizontal_wrapped(|ui| {
            ui.add_space(16.0);
            for &op in opcoes_lei(cat) {
                if op == atual {
                    continue;
                }
                if ui.add_enabled(!reformando, egui::Button::new(format!("Reformar para: {}", nome_lei(op)))).clicked() {
                    iniciar = Some((cat, op));
                }
            }
        });
    }
    if let Some((c, l)) = iniciar {
        p.politica.iniciar_reforma(c, l);
    }
    ui.separator();
    ui.heading("Grupos de interesse");
    egui::Grid::new("gis").striped(true).show(ui, |ui| {
        ui.strong("Grupo");
        ui.strong("Clout");
        ui.strong("Aprovação");
        ui.strong("Governo");
        ui.end_row();
        for g in p.politica.grupos.values() {
            ui.label(g.id.nome_pt());
            ui.label(format!("{:.0}%", g.clout * 100.0));
            ui.label(format!("{:.1}", g.aprovacao));
            ui.label(if g.em_governo { "sim" } else { "-" });
            ui.end_row();
        }
    });
    ui.separator();
    ui.label(format!("Legitimidade: {:.1}%", p.politica.legitimidade));
    ui.label(format!("Esfera de influência: {}", p.politica.esfera_atual().rotulo_pt()));
    match p.politica.dias_ate_eleicao {
        Some(d) => ui.label(format!("Próxima eleição em {} dias (~{:.1} anos)", d, d as f32 / 365.0)),
        None => ui.label("Este regime não realiza eleições."),
    };
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
