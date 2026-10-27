//! Camada jogável: país do jogador, HUD, painel do país (F1), ranking e notificações.
use super::economy::RecursoMundo;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use simulacao::economia::{NivelImposto, Predio, Profissao, TipoPredio};
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
                (painel_pais, toasts).run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida),
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

/// Faixa do HUD com os indicadores do país do jogador.
pub fn hud_pais(ui: &mut egui::Ui, p: &EstadoPais, painel_aberto: &mut bool) {
    ui.horizontal_wrapped(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(r, 2.0, cor_pais(p));
        ui.strong(&p.nome);
        ui.separator();
        let o = &p.orcamento;
        ui.label(format!("Tesouro: {}", fmt_din(o.tesouro)));
        let saldo = o.saldo_diario();
        ui.colored_label(cor_saldo(saldo), format!("Saldo/dia: {}", fmt_din(saldo)));
        ui.label(format!("Dívida: {}", fmt_din(o.divida_soberana)));
        ui.separator();
        ui.label(format!("PIB: {}", fmt_din(p.pib_anual_estimado)));
        ui.label(format!("Pop.: {}", fmt_pop(p.populacao_total())));
        ui.label(format!("Legitimidade: {:.0}%", p.politica.legitimidade));
        ui.label(format!("Padrão de vida: {:.1}", p.padrao_vida_medio()));
        ui.separator();
        if ui.button("País (F1)").clicked() {
            *painel_aberto = !*painel_aberto;
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
    tempo: Res<Time<Real>>,
    mut ant: ResMut<Anterior>,
    mut notas: ResMut<Notificacoes>,
) {
    let Some(p) = mundo.paises.get(&jogador.0) else { return };
    let agora = tempo.elapsed_secs_f64();
    let negativo = p.orcamento.divida_soberana > 0.0 && p.orcamento.tesouro <= 0.0;
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

fn nome_imposto(n: NivelImposto) -> &'static str {
    match n {
        NivelImposto::MuitoBaixo => "Muito baixo",
        NivelImposto::Baixo => "Baixo",
        NivelImposto::Medio => "Médio",
        NivelImposto::Alto => "Alto",
        NivelImposto::MuitoAlto => "Muito alto",
    }
}

fn custo_construcao(p: &EstadoPais, pr: &Predio) -> f64 {
    (p.pib_anual_estimado * 0.0005).max(500.0) * (1.0 + pr.nivel as f64 / 10.0)
}

fn painel_pais(
    mut contextos: EguiContexts,
    mut painel: ResMut<PainelPais>,
    jogador: Res<PaisJogador>,
    mut mundo: ResMut<RecursoMundo>,
) -> Result {
    if !painel.aberto {
        return Ok(());
    }
    let mut aberto = true;
    let mut aba = painel.aba;
    let ctx = contextos.ctx_mut()?;
    egui::Window::new("Painel do país (F1)").open(&mut aberto).default_size([620.0, 520.0]).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut aba, Aba::Economia, "Economia");
            ui.selectable_value(&mut aba, Aba::Politica, "Política");
            ui.selectable_value(&mut aba, Aba::Ranking, "Ranking");
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| match aba {
            Aba::Economia => aba_economia(ui, &mut mundo, &jogador.0),
            Aba::Politica => aba_politica(ui, &mut mundo, &jogador.0),
            Aba::Ranking => aba_ranking(ui, &mundo, &jogador.0),
        });
    });
    painel.aba = aba;
    painel.aberto = aberto;
    Ok(())
}

fn aba_economia(ui: &mut egui::Ui, mundo: &mut RecursoMundo, tag: &str) {
    let Some(p) = mundo.paises.get_mut(tag) else { return };
    ui.heading("Orçamento");
    let o = &p.orcamento;
    egui::Grid::new("orc").num_columns(2).show(ui, |ui| {
        for (n, v) in [
            ("Receita de impostos", o.receita_impostos_diaria),
            ("Receita estatal", o.receita_estatal_diaria),
            ("Despesa administrativa", -o.despesa_administrativa_diaria),
            ("Despesa militar", -o.despesa_militar_diaria),
            ("Despesa de construção", -o.despesa_construcao_diaria),
            ("Juros da dívida", -o.juros_divida_diaria),
            ("Transferências externas", o.transferencias_externas_diaria),
            ("Saldo diário", o.saldo_diario()),
        ] {
            ui.label(n);
            ui.colored_label(cor_saldo(v), fmt_din(v));
            ui.end_row();
        }
    });
    if o.em_moratoria {
        ui.colored_label(egui::Color32::LIGHT_RED, "País em moratória (dívida acima do limite de crédito)!");
    }
    ui.horizontal(|ui| {
        ui.label("Nível de imposto:");
        for n in [NivelImposto::MuitoBaixo, NivelImposto::Baixo, NivelImposto::Medio, NivelImposto::Alto, NivelImposto::MuitoAlto] {
            if ui.selectable_label(p.orcamento.nivel_imposto == n, nome_imposto(n)).clicked() {
                p.orcamento.nivel_imposto = n;
            }
        }
    });
    ui.label(format!(
        "Eficiência de arrecadação: {:.0}% | Limite de crédito: {}",
        p.orcamento.eficiencia_arrecadacao * 100.0,
        fmt_din(p.orcamento.limite_credito)
    ));
    ui.separator();
    ui.heading("População por estrato e profissão");
    let mut por_prof: BTreeMap<Profissao, (u64, f64)> = BTreeMap::new();
    for pop in &p.pops {
        let h = pop.total_habitantes();
        let e = por_prof.entry(pop.profissao).or_default();
        e.0 += h;
        e.1 += pop.padrao_vida as f64 * h as f64;
    }
    egui::Grid::new("pops").striped(true).show(ui, |ui| {
        ui.strong("Profissão");
        ui.strong("Estrato");
        ui.strong("Habitantes");
        ui.strong("Padrão de vida");
        ui.end_row();
        for (prof, (h, sol)) in &por_prof {
            ui.label(prof.nome_pt());
            ui.label(format!("{:?}", prof.estrato()));
            ui.label(fmt_pop(*h));
            ui.label(format!("{:.1}", sol / (*h).max(1) as f64));
            ui.end_row();
        }
    });
    ui.separator();
    ui.heading("Prédios");
    let mut construir: Option<usize> = None;
    let tesouro = p.orcamento.tesouro;
    egui::Grid::new("predios").striped(true).show(ui, |ui| {
        ui.strong("Tipo");
        ui.strong("Nível");
        ui.strong("Lucro/dia");
        ui.strong("");
        ui.end_row();
        for (i, pr) in p.predios.iter().enumerate() {
            ui.label(nome_predio(pr.tipo));
            ui.label(pr.nivel.to_string());
            ui.colored_label(cor_saldo(pr.ultimo_lucro_diario), fmt_din(pr.ultimo_lucro_diario));
            let custo = custo_construcao(p, pr);
            let ok = tesouro >= custo;
            if ui.add_enabled(ok, egui::Button::new(format!("Construir +1 ({})", fmt_din(custo)))).clicked() {
                construir = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = construir {
        let custo = custo_construcao(p, &p.predios[i]);
        if p.orcamento.tesouro >= custo {
            p.orcamento.tesouro -= custo;
            let pr = &mut p.predios[i];
            pr.nivel += 1;
            pr.empregados += TipoPredio::TRABALHADORES_POR_NIVEL;
        }
    }
}

fn nome_predio(t: TipoPredio) -> String {
    // Converte CamelCase em palavras separadas.
    let d = format!("{t:?}");
    let mut s = String::new();
    for (i, c) in d.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            s.push(' ');
        }
        s.push(c);
    }
    s
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

fn aba_ranking(ui: &mut egui::Ui, mundo: &RecursoMundo, tag: &str) {
    let mut paises: Vec<&EstadoPais> = mundo.paises.values().collect();
    ui.columns(2, |cols| {
        paises.sort_by(|a, b| b.pib_anual_estimado.total_cmp(&a.pib_anual_estimado));
        cols[0].heading("Maiores PIBs");
        for (i, p) in paises.iter().take(10).enumerate() {
            let t = format!("{}. {} - {}", i + 1, p.nome, fmt_din(p.pib_anual_estimado));
            if p.tag == tag { cols[0].strong(t); } else { cols[0].label(t); }
        }
        paises.sort_by_key(|p| std::cmp::Reverse(p.populacao_total()));
        cols[1].heading("Maiores populações");
        for (i, p) in paises.iter().take(10).enumerate() {
            let t = format!("{}. {} - {}", i + 1, p.nome, fmt_pop(p.populacao_total()));
            if p.tag == tag { cols[1].strong(t); } else { cols[1].label(t); }
        }
    });
}

