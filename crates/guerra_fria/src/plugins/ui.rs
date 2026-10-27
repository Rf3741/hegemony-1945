//! Interface egui: barra superior (data e controles de tempo) e painel de província.
use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::mercado_ui::PainelMercado;
use super::jogador::{PainelPais, PaisJogador, hud_pais, resumo_pais};
use super::map::{ProvinciaSelecionada, RecursoCatalogo};
use super::time::RecursoRelogio;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

fn ui_raiz(ctx: &egui::Context) -> egui::Ui {
    egui::Ui::new(
        ctx.clone(),
        "viewport_raiz".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    )
}
use gf_mapa::tempo::VelocidadeJogo;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            EguiPrimaryContextPass,
            (barra_superior, barra_hud, painel_provincia)
                .chain()
                .run_if(crate::estado::em_partida),
        );
    }
}

fn barra_superior(
    mut contextos: EguiContexts,
    mut relogio: ResMut<RecursoRelogio>,
) -> Result {
    let mut raiz = ui_raiz(contextos.ctx_mut()?);
    egui::Panel::top("barra_superior").show(&mut raiz, |ui| {
        ui.horizontal(|ui| {
            ui.heading(relogio.data.formatar_pt_br());
            ui.separator();
            let rotulo = if relogio.pausado { "Continuar" } else { "Pausar" };
            if ui.button(rotulo).clicked() {
                relogio.alternar_pausa();
            }
            for v in VelocidadeJogo::TODAS {
                let ativa = !relogio.pausado && relogio.velocidade == v;
                if ui.selectable_label(ativa, v.nivel().to_string()).clicked() {
                    relogio.definir_velocidade(v);
                }
            }
            ui.separator();
            if relogio.pausado {
                ui.label("PAUSADO");
            }
            ui.label("Espaço: pausa | 1-5: velocidade | WASD/arrastar: mover | scroll: zoom");
        });
    });
    Ok(())
}

fn barra_hud(
    mut contextos: EguiContexts,
    jogador: Option<Res<PaisJogador>>,
    mundo: Res<RecursoMundo>,
    eco: Option<Res<EconomiaSim>>,
    mut painel: ResMut<PainelPais>,
    mut mercado: ResMut<PainelMercado>,
) -> Result {
    let Some(eco) = eco else { return Ok(()) };
    let Some(p) = jogador.and_then(|j| mundo.paises.get(&j.0)) else { return Ok(()) };
    let mut raiz = ui_raiz(contextos.ctx_mut()?);
    egui::Panel::top("barra_hud").show(&mut raiz, |ui| hud_pais(ui, p, &eco, &mut painel.aberto, &mut mercado.aberto));
    Ok(())
}

fn painel_provincia(
    mut contextos: EguiContexts,
    selecionada: Res<ProvinciaSelecionada>,
    catalogo: Res<RecursoCatalogo>,
    mundo: Res<RecursoMundo>,
) -> Result {
    if selecionada.0 == 0 {
        return Ok(());
    }
    let Some(prov) = catalogo.0.obter_por_id(selecionada.0) else { return Ok(()) };
    let mut raiz = ui_raiz(contextos.ctx_mut()?);
    egui::Panel::right("painel_provincia").min_size(260.0).show(&mut raiz, |ui| {
        ui.heading(&prov.nome);
        ui.label(format!("ID da província: {}", prov.id));
        ui.label(format!("Código ADM1: {}", prov.adm1_code));
        ui.label(format!("País moderno: {}", prov.pais_moderno));
        if let Some(tag) = mundo.dono_por_provincia.get(&prov.id) {
            let nome = mundo.paises.get(tag).map(|p| p.nome.as_str()).unwrap_or("?");
            ui.label(format!("Dono em 1945: {nome} ({tag})"));
            if let Some(pais) = mundo.paises.get(tag) {
                ui.separator();
                resumo_pais(ui, pais);
                ui.separator();
            }
        }
        if let Some(pop) = mundo.populacao_por_provincia.get(&prov.id) {
            ui.label(format!("População da província: {pop}"));
        }
        ui.label(format!("Vizinhas: {}", prov.vizinhos.len()));
    });
    Ok(())
}
