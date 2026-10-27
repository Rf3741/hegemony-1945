//! Tela do Mercado Global (F2): cotações por bem, variação de 30 dias, oferta, demanda e saldo;
//! ao clicar num bem, gráfico do histórico (mundial e nacional), maiores produtores e consumidores
//! e os prédios que produzem e consomem o bem. Também carrega os ícones PNG para o egui.

use super::economia_sim::{EconomiaSim, VistaBem};
use super::jogador::PaisJogador;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use egui_plot::{Legend, Line, Plot, PlotPoints};
use sim_economia::CategoriaBem;
use std::collections::HashMap;
use std::path::PathBuf;

/// Cache de texturas dos ícones (`assets/icones/<id>_32.png`).
#[derive(Resource, Default)]
pub struct Icones {
    pub dir: PathBuf,
    cache: HashMap<String, Option<egui::TextureHandle>>,
}

impl Icones {
    /// Mostra o ícone do id (bem, prédio ou tipo de pop); reserva o espaço mesmo sem arquivo.
    pub fn mostrar(&mut self, ui: &mut egui::Ui, id: &str, tam: f32) {
        let ctx = ui.ctx().clone();
        let dir = &self.dir;
        let tex = self.cache.entry(id.to_string()).or_insert_with(|| {
            let img = image::open(dir.join(format!("{id}_32.png"))).ok()?.to_rgba8();
            let (w, h) = img.dimensions();
            let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
            Some(ctx.load_texture(format!("icone_{id}"), ci, egui::TextureOptions::LINEAR))
        });
        match tex {
            Some(t) => {
                ui.add(egui::Image::new(egui::load::SizedTexture::new(t.id(), egui::vec2(tam, tam))));
            }
            None => {
                ui.add_space(tam);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Filtro {
    #[default]
    Todos,
    Alimentos,
    Materias,
    Energia,
    Industrial,
    Consumo,
    Luxo,
    Militar,
    Servicos,
}

impl Filtro {
    const TODOS: [Filtro; 9] = [Self::Todos, Self::Alimentos, Self::Materias, Self::Energia, Self::Industrial, Self::Consumo, Self::Luxo, Self::Militar, Self::Servicos];
    fn nome(self) -> &'static str {
        match self {
            Self::Todos => "Todos",
            Self::Alimentos => "Alimentos",
            Self::Materias => "Matérias-primas",
            Self::Energia => "Energia",
            Self::Industrial => "Industriais",
            Self::Consumo => "Consumo",
            Self::Luxo => "Luxo",
            Self::Militar => "Militares",
            Self::Servicos => "Serviços",
        }
    }
    fn aceita(self, c: CategoriaBem) -> bool {
        matches!(
            (self, c),
            (Self::Todos, _)
                | (Self::Alimentos, CategoriaBem::AlimentoBasico)
                | (Self::Materias, CategoriaBem::MateriaPrima)
                | (Self::Energia, CategoriaBem::Energia)
                | (Self::Industrial, CategoriaBem::Industrial)
                | (Self::Consumo, CategoriaBem::Consumo)
                | (Self::Luxo, CategoriaBem::Luxo)
                | (Self::Militar, CategoriaBem::Militar)
                | (Self::Servicos, CategoriaBem::LocalNaoComercializavel)
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Ordem {
    #[default]
    Nome,
    PrecoNacional,
    Variacao,
    Saldo,
}

#[derive(Resource, Default)]
pub struct PainelMercado {
    pub aberto: bool,
    filtro: Filtro,
    ordem: Ordem,
    selecionado: Option<String>,
}

pub struct MercadoUiPlugin;

impl Plugin for MercadoUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PainelMercado>()
            .add_systems(Startup, |mut commands: Commands, raiz: Res<super::map::RaizProjeto>| {
                commands.insert_resource(Icones { dir: raiz.0.join("assets/icones"), ..default() });
            })
            .add_systems(Update, alternar_mercado.run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida))
            .add_systems(
                EguiPrimaryContextPass,
                tela_mercado.run_if(resource_exists::<PaisJogador>).run_if(resource_exists::<EconomiaSim>).run_if(crate::estado::em_partida),
            );
    }
}

fn alternar_mercado(teclas: Res<ButtonInput<KeyCode>>, mut p: ResMut<PainelMercado>) {
    if teclas.just_pressed(KeyCode::F2) {
        p.aberto = !p.aberto;
    }
}

pub fn cor_variacao(v: f64) -> egui::Color32 {
    if v > 0.005 {
        egui::Color32::from_rgb(235, 120, 90)
    } else if v < -0.005 {
        egui::Color32::from_rgb(110, 200, 120)
    } else {
        egui::Color32::GRAY
    }
}

pub fn fmt_var(v: f64) -> String {
    let seta = if v > 0.005 { "↑" } else if v < -0.005 { "↓" } else { "→" };
    format!("{seta} {:+.1}%", v * 100.0)
}

pub fn fmt_qtd(v: f64) -> String {
    let a = v.abs();
    if a >= 1e9 {
        format!("{:.2} bi", v / 1e9)
    } else if a >= 1e6 {
        format!("{:.2} mi", v / 1e6)
    } else if a >= 1e3 {
        format!("{:.1} mil", v / 1e3)
    } else {
        format!("{v:.0}")
    }
}

fn tela_mercado(
    mut contextos: EguiContexts,
    mut painel: ResMut<PainelMercado>,
    eco: Res<EconomiaSim>,
    mut icones: ResMut<Icones>,
) -> Result {
    if !painel.aberto {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?;
    let mut aberto = true;
    let v = &eco.vista;
    egui::Window::new("Mercado Global (F2)").open(&mut aberto).default_size([980.0, 620.0]).show(ctx, |ui| {
        if v.bens.is_empty() {
            ui.label("Aguardando os primeiros dados do mercado...");
            return;
        }
        ui.horizontal_wrapped(|ui| {
            for f in Filtro::TODOS {
                if ui.selectable_label(painel.filtro == f, f.nome()).clicked() {
                    painel.filtro = f;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Ordenar por:");
            for (o, n) in [(Ordem::Nome, "Nome"), (Ordem::PrecoNacional, "Preço"), (Ordem::Variacao, "Variação 30d"), (Ordem::Saldo, "Saldo")] {
                if ui.selectable_label(painel.ordem == o, n).clicked() {
                    painel.ordem = o;
                }
            }
            ui.label("  Preços em $ por unidade. Mundial = oferta e demanda de todos os mercados; nacional = seu mercado.");
        });
        let mut lista: Vec<&VistaBem> = v.bens.iter().filter(|b| painel.filtro.aceita(b.categoria)).collect();
        match painel.ordem {
            Ordem::Nome => lista.sort_by(|a, b| a.nome.cmp(&b.nome)),
            Ordem::PrecoNacional => lista.sort_by(|a, b| b.preco_nacional.total_cmp(&a.preco_nacional)),
            Ordem::Variacao => lista.sort_by(|a, b| b.var30_mundial.abs().total_cmp(&a.var30_mundial.abs())),
            Ordem::Saldo => lista.sort_by(|a, b| a.saldo_comercial.total_cmp(&b.saldo_comercial)),
        }
        ui.separator();
        ui.columns(2, |cols| {
            // Coluna esquerda: tabela de bens
            egui::ScrollArea::vertical().id_salt("tab_bens").show(&mut cols[0], |ui| {
                egui::Grid::new("grade_bens").striped(true).num_columns(7).spacing([10.0, 3.0]).show(ui, |ui| {
                    for h in ["Bem", "Mundial", "Nacional", "Var. 30d", "Oferta", "Demanda", "Saldo"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for b in &lista {
                        ui.horizontal(|ui| {
                            icones.mostrar(ui, &b.id, 18.0);
                            let sel = painel.selecionado.as_deref() == Some(b.id.as_str());
                            if ui.selectable_label(sel, &b.nome).clicked() {
                                painel.selecionado = Some(b.id.clone());
                            }
                        });
                        ui.label(format!("{:.1}", b.preco_mundial));
                        ui.label(format!("{:.1}", b.preco_nacional));
                        ui.colored_label(cor_variacao(b.var30_mundial), fmt_var(b.var30_mundial));
                        ui.label(fmt_qtd(b.oferta_nacional));
                        ui.label(fmt_qtd(b.demanda_nacional));
                        let s = b.saldo_comercial;
                        let (txt, cor) = if s > 1.0 { (format!("+{} (importa)", fmt_qtd(s)), egui::Color32::LIGHT_BLUE) } else if s < -1.0 { (format!("{} (exporta)", fmt_qtd(s)), egui::Color32::GOLD) } else { ("-".into(), egui::Color32::GRAY) };
                        ui.colored_label(cor, txt);
                        ui.end_row();
                    }
                });
            });
            // Coluna direita: detalhe do bem
            let ui = &mut cols[1];
            let escolhido = painel.selecionado.as_deref().and_then(|id| v.bens.iter().find(|b| b.id == id));
            match escolhido {
                None => {
                    ui.label("Clique num bem para ver o histórico de preço, produtores e consumidores.");
                }
                Some(b) => detalhe_bem(ui, b, v, &mut icones),
            }
        });
    });
    painel.aberto = aberto;
    Ok(())
}

fn detalhe_bem(ui: &mut egui::Ui, b: &VistaBem, v: &super::economia_sim::VistaEconomia, icones: &mut Icones) {
    ui.horizontal(|ui| {
        icones.mostrar(ui, &b.id, 28.0);
        ui.heading(&b.nome);
    });
    ui.label(format!(
        "Base {:.1} | mundial {:.1} ({}) | nacional {:.1} ({})",
        b.preco_base,
        b.preco_mundial,
        fmt_var(b.var30_mundial),
        b.preco_nacional,
        fmt_var(b.var30_nacional)
    ));
    ui.label(format!(
        "Mundo: oferta {} / demanda {}{}",
        fmt_qtd(b.oferta_mundial),
        fmt_qtd(b.demanda_mundial),
        if b.comercializavel { "" } else { " (bem local, não comercializável)" }
    ));
    let serie = |h: &[f32]| -> PlotPoints<'static> { h.iter().enumerate().map(|(i, p)| [i as f64, *p as f64]).collect() };
    Plot::new("plot_preco").height(190.0).legend(Legend::default()).x_axis_label("dias de jogo").allow_scroll(false).show(ui, |pu| {
        pu.line(Line::new("Preço mundial", serie(&b.hist_mundial)).color(egui::Color32::from_rgb(240, 200, 90)).width(2.0));
        pu.line(Line::new("Seu mercado", serie(&b.hist_nacional)).color(egui::Color32::from_rgb(110, 170, 240)).width(2.0));
        pu.line(Line::new("Preço base", PlotPoints::from(vec![[0.0, b.preco_base], [b.hist_mundial.len().max(1) as f64, b.preco_base]])).color(egui::Color32::GRAY));
    });
    ui.columns(2, |cols| {
        cols[0].strong("Maiores produtores (mercados)");
        for (n, q) in &b.produtores {
            cols[0].label(format!("{n}: {}", fmt_qtd(*q)));
        }
        cols[1].strong("Maiores consumidores (mercados)");
        for (n, q) in &b.consumidores {
            cols[1].label(format!("{n}: {}", fmt_qtd(*q)));
        }
    });
    if let Some(cat) = &v.catalogo {
        let (mut produzem, mut consomem) = (Vec::new(), Vec::new());
        for t in &cat.tipos_predio {
            let (mut p, mut c) = (false, false);
            for mid in &t.metodos_permitidos {
                if let Some(m) = cat.metodos_producao.iter().find(|m| &m.id == mid) {
                    p |= m.produtos.contains_key(&b.id);
                    c |= m.insumos.contains_key(&b.id);
                }
            }
            if p {
                produzem.push(t);
            }
            if c {
                consomem.push(t);
            }
        }
        ui.separator();
        ui.columns(2, |cols| {
            cols[0].strong("Prédios que produzem");
            for t in &produzem {
                cols[0].horizontal(|ui| {
                    icones.mostrar(ui, &t.id, 16.0);
                    ui.label(&t.nome);
                });
            }
            cols[1].strong("Prédios que consomem");
            for t in &consomem {
                cols[1].horizontal(|ui| {
                    icones.mostrar(ui, &t.id, 16.0);
                    ui.label(&t.nome);
                });
            }
        });
    }
}
