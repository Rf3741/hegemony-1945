//! Interface egui: barra superior (data e controles de tempo) e painel de província.
use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::comercio_ui::PainelComercio;
use super::construcao_ui::{ConstrucaoUi, secao_construir_aqui};
use super::mercado_ui::PainelMercado;
use super::jogador::{PainelPais, PaisJogador, hud_pais, resumo_pais};
use super::map::{ProvinciaSelecionada, RecursoCatalogo};
use super::time::RecursoRelogio;
use bevy::prelude::*;
use super::enciclopedia::dica;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

pub(crate) fn ui_raiz(ctx: &egui::Context) -> egui::Ui {
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
            (barra_superior, barra_hud, super::hud_mapas::barra_extra, painel_provincia)
                .chain()
                .run_if(crate::estado::em_partida),
        );
    }
}

fn barra_superior(
    mut contextos: EguiContexts,
    mut relogio: ResMut<RecursoRelogio>,
    camada_visivel: Option<Res<visual_mapa::CamadaConstrucoesVisivel>>,
    mut salvar: MessageWriter<super::saveload::PedirSalvar>,
    slots: Res<super::saveload::Slots>,
) -> Result {
    let mut raiz = ui_raiz(contextos.ctx_mut()?);
    egui::Panel::top("barra_superior").show(&mut raiz, |ui| {
        ui.horizontal(|ui| {
            dica(ui.heading(relogio.data.formatar_pt_br()), "tooltip_topbar_data");
            ui.separator();
            let rotulo = if relogio.pausado { "Continuar" } else { "Pausar" };
            if dica(ui.button(rotulo), "tooltip_topbar_botao_pausar").clicked() {
                relogio.alternar_pausa();
            }
            for v in VelocidadeJogo::TODAS {
                let ativa = !relogio.pausado && relogio.velocidade == v;
                let id_dica = format!("tooltip_topbar_velocidade_{}", v.nivel());
                if dica(ui.selectable_label(ativa, v.nivel().to_string()), &id_dica).clicked() {
                    relogio.definir_velocidade(v);
                }
            }
            ui.separator();
            if ui.button("Salvar").on_hover_text("Salvamento rápido (Ctrl+S); carregar com Ctrl+L").clicked() {
                salvar.write(super::saveload::PedirSalvar(super::saveload::Slot::Rapido));
            }
            if !slots.mensagem.is_empty() {
                ui.label(&slots.mensagem);
            }
            ui.separator();
            if relogio.pausado {
                ui.label("PAUSADO");
            }
            if let Some(c) = camada_visivel {
                let texto = if c.0 { "🏛 Construções: [V] Ativas" } else { "🏛 Construções: [V] Ocultas" };
                ui.label(texto);
                ui.separator();
            }
            ui.label("Espaço: pausa | +/−: vel | WASD: mover | scroll: zoom | V: construções | M: modos de mapa | 1-9: painéis | Ctrl+S/L: salvar/carregar");
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
    mut comercio: ResMut<PainelComercio>,
    mut constr: ResMut<ConstrucaoUi>,
) -> Result {
    let Some(eco) = eco else { return Ok(()) };
    let Some(p) = jogador.and_then(|j| mundo.paises.get(&j.0)) else { return Ok(()) };
    let mut raiz = ui_raiz(contextos.ctx_mut()?);
    egui::Panel::top("barra_hud").show(&mut raiz, |ui| hud_pais(ui, p, &eco, &mut painel.aberto, &mut mercado.aberto, &mut comercio.aberto, &mut constr.aberta));
    Ok(())
}

fn painel_provincia(
    mut contextos: EguiContexts,
    selecionada: Res<ProvinciaSelecionada>,
    catalogo: Res<RecursoCatalogo>,
    mundo: Res<RecursoMundo>,
    eco: Option<Res<EconomiaSim>>,
    mut constr: ResMut<ConstrucaoUi>,
) -> Result {
    if selecionada.0 == 0 {
        return Ok(());
    }
    let Some(prov) = catalogo.0.obter_por_id(selecionada.0) else { return Ok(()) };
    let mut raiz = ui_raiz(contextos.ctx_mut()?);
    egui::Panel::right("painel_provincia").min_size(260.0).show(&mut raiz, |ui| {
        ui.heading(&prov.nome);
        let estado = if prov.estado.is_empty() { prov.adm1_code.as_str() } else { prov.estado.as_str() };
        ui.label(format!("Estado: {estado}"));
        ui.label(format!("ID da província: {} | Código do estado (ADM1): {}", prov.id, prov.adm1_code));
        ui.label(format!("Área da província: {:.0} km²", prov.area));
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
        let irmas = catalogo.0.provincias.iter().filter(|p| p.adm1_code == prov.adm1_code).count();
        ui.label(format!("Vizinhas: {} | Províncias do estado: {irmas}", prov.vizinhos.len()));
        ui.label("Construção, economia e política são por estado.");
        if let Some(eco) = &eco {
            egui::ScrollArea::vertical().id_salt("scroll_painel_prov").show(ui, |ui| secao_construir_aqui(ui, &eco.vista, &prov.adm1_code, &mut constr));
        }
    });
    Ok(())
}
