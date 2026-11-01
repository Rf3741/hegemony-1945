//! Retratos e painel de líderes: cache de texturas dos PNGs gerados por `scripts/gerar_retratos.py`
//! e widgets egui reutilizados no painel de política, nos eventos e na diplomacia.

use super::map::RaizProjeto;
use super::personagens_sim::ElencoRes;
use bevy::prelude::*;
use bevy_egui::egui;
use personagens::{Elenco, Papel, Personagem};
use std::collections::HashMap;
use std::path::PathBuf;

/// Cache de texturas de retratos.
#[derive(Resource, Default)]
pub struct Retratos {
    pub dir: PathBuf,
    cache: HashMap<String, Option<egui::TextureHandle>>,
}

impl Retratos {
    fn textura(&mut self, ctx: &egui::Context, id: &str) -> Option<egui::TextureHandle> {
        let dir = self.dir.clone();
        self.cache
            .entry(id.to_string())
            .or_insert_with(|| {
                let abrir = |n: &str| image::open(dir.join(format!("{n}.png"))).ok();
                let img = abrir(id).or_else(|| abrir("retrato_generico"))?.to_rgba8();
                let tam = [img.width() as usize, img.height() as usize];
                Some(ctx.load_texture(format!("retrato_{id}"), egui::ColorImage::from_rgba_unmultiplied(tam, img.as_raw()), egui::TextureOptions::LINEAR))
            })
            .clone()
    }

    /// Mostra o retrato do personagem com `altura` pixels (proporção 4:5).
    pub fn mostrar(&mut self, ui: &mut egui::Ui, id: &str, altura: f32) -> egui::Response {
        let tam = egui::vec2(altura * 0.8, altura);
        match self.textura(ui.ctx(), id) {
            Some(t) => ui.add(egui::Image::new(&t).fit_to_exact_size(tam).corner_radius(4.0)),
            None => ui.allocate_response(tam, egui::Sense::hover()),
        }
    }
}

pub fn registrar(app: &mut App) {
    app.add_systems(Startup, |mut c: Commands, raiz: Res<RaizProjeto>| c.insert_resource(Retratos { dir: raiz.0.join("assets/retratos"), ..default() }));
}

fn nome_traco(e: &Elenco, id: &str) -> String {
    e.traco(id).map_or(id.to_string(), |t| t.nome.clone())
}

/// Cartão de um líder: retrato, nome, idade, ideologia e traços (com efeitos ao passar o mouse).
pub fn cartao_lider(ui: &mut egui::Ui, r: &mut Retratos, e: &Elenco, p: &Personagem, cargo: &str, hoje: personagens::Data, altura: f32) {
    ui.horizontal(|ui| {
        r.mostrar(ui, &p.id, altura);
        ui.vertical(|ui| {
            ui.small(cargo);
            ui.strong(&p.nome);
            ui.label(format!("{} anos | {}", p.idade(hoje), p.ideologia.replace('_', " ")));
            ui.horizontal_wrapped(|ui| {
                for t in &p.tracos {
                    let resp = ui.label(egui::RichText::new(nome_traco(e, t)).small().background_color(egui::Color32::from_rgb(0x2a, 0x33, 0x44)));
                    if let Some(tr) = e.traco(t) {
                        resp.on_hover_ui(|ui| {
                            ui.set_max_width(320.0);
                            ui.label(&tr.descricao);
                            for (k, v) in &tr.efeitos {
                                let cor = if *v >= 0.0 { egui::Color32::LIGHT_GREEN } else { egui::Color32::LIGHT_RED };
                                ui.colored_label(cor, format!("{k}: {v:+.2}"));
                            }
                        });
                    }
                }
            });
        });
    });
}

/// Painel de líderes de um país (tag do cenário político): Estado, Governo e líderes de GI.
pub fn painel_lideres(ui: &mut egui::Ui, el: &ElencoRes, r: &mut Retratos, tag: &str, hoje: personagens::Data) {
    let e = &el.elenco;
    let estado = e.lider(tag, Papel::Estado);
    let governo = e.lider(tag, Papel::Governo);
    if estado.is_none() && governo.is_none() {
        return;
    }
    ui.strong("Liderança");
    if let Some(p) = estado {
        cartao_lider(ui, r, e, p, Papel::Estado.nome(), hoje, 84.0);
    }
    if let Some(g) = governo.filter(|g| Some(g.id.as_str()) != estado.map(|p| p.id.as_str())) {
        ui.add_space(4.0);
        cartao_lider(ui, r, e, g, Papel::Governo.nome(), hoje, 84.0);
    }
    let gis = e.lideres_gi(tag);
    if !gis.is_empty() {
        ui.add_space(4.0);
        egui::CollapsingHeader::new(format!("Líderes de grupos e oposição ({})", gis.len())).id_salt(("lid_gi", tag)).show(ui, |ui| {
            for p in gis {
                cartao_lider(ui, r, e, p, &p.cargo_inicial.replace('_', " "), hoje, 56.0);
            }
        });
    }
    if !el.historico.is_empty() {
        egui::CollapsingHeader::new("Histórico de sucessões").id_salt("lid_hist").show(ui, |ui| {
            for h in el.historico.iter().rev().take(12) {
                ui.small(h);
            }
        });
    }
}

/// Retrato compacto do chefe de Estado de um país, com nome (para eventos e diplomacia).
pub fn retrato_chefe(ui: &mut egui::Ui, el: &ElencoRes, r: &mut Retratos, tag: &str, altura: f32) {
    if let Some(p) = el.elenco.lider(tag, Papel::Estado) {
        ui.horizontal(|ui| {
            r.mostrar(ui, &p.id, altura);
            ui.vertical(|ui| {
                ui.small(Papel::Estado.nome());
                ui.strong(&p.nome);
            });
        });
    }
}
