//! Tela de Tecnologia (F5) estilo Victoria 3 + ponte com o motor puro `sim_tecnologia`.
//!
//! * Pesquisa diária de todos os países (a IA escolhe por prioridade simples).
//! * Pontos de pesquisa: níveis de universidade e alfabetização vindos do `MundoEconomico`
//!   (quando disponível) ou PIB per capita (reserva).
//! * Desbloqueios: publica em `VistaEconomia` os prédios/métodos ainda travados; a UI de construção
//!   e de troca de método respeita, e `EconomiaSim::enviar` recusa comandos travados.
//! * A era visual do jogador limita o `AnoVisual` do `visual_mapa`.

use super::economia_sim::{ALIAS_TAGS, EconomiaSim, EconomiaSimSet};
use super::jogador::PaisJogador;
use super::map::RaizProjeto;
use super::time::{EventoTickDiario, RecursoRelogio};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use sim_tecnologia::{CatalogoTec, ConfigBlocos, EntradaPais, MundoTec, Ramo, Tecnologia};
use std::collections::BTreeMap;
use std::sync::Arc;

const GRANDES_POTENCIAS: [&str; 4] = ["USA", "SOV", "GBR", "FRA"];
const BLOCO_SOVIETICO: [&str; 10] = ["SOV", "POL", "ROM", "BUL", "HUN", "CZE", "ALB", "MON", "CCP", "YUG"];
const INTERVALO_ENTRADAS: u64 = 30;

#[derive(Resource)]
pub struct TecnologiaSim {
    pub mundo: MundoTec,
    pub entradas: BTreeMap<String, EntradaPais>,
    /// Tag econômica do jogador.
    pub tag_jogador: Option<String>,
    pub era_jogador: u32,
    pub aberto: bool,
    pub mensagem: String,
    bloq_predios: Arc<BTreeMap<String, String>>,
    bloq_metodos: Arc<BTreeMap<String, String>>,
    sujo: bool,
    ultimo_refresh: u64,
    ramo: Option<Ramo>,
    /// Avisos de pesquisas concluídas do jogador, para o HUD/tela.
    pub concluidas_jogador: Vec<String>,
}

impl TecnologiaSim {
    pub fn restaurar_estado(&mut self, paises: BTreeMap<String, sim_tecnologia::EstadoPais>) {
        self.mundo.restaurar_estado(paises);
        self.sujo = true;
        self.ultimo_refresh = u64::MAX;
        self.concluidas_jogador.clear();
    }
}

impl TecnologiaSim {
    pub fn carregar(raiz: &std::path::Path) -> Result<Self, String> {
        let cat = CatalogoTec::carregar(&raiz.join("assets/data/tecnologia")).map_err(|e| e.to_string())?;
        let blocos = blocos_para(&cat);
        let mundo = MundoTec::novo(Arc::new(cat), blocos);
        Ok(Self {
            mundo,
            entradas: BTreeMap::new(),
            tag_jogador: None,
            era_jogador: 1945,
            aberto: false,
            mensagem: String::new(),
            bloq_predios: Arc::default(),
            bloq_metodos: Arc::default(),
            sujo: true,
            ultimo_refresh: u64::MAX,
            ramo: None,
            concluidas_jogador: Vec::new(),
        })
    }

    fn entrada(&self, tag: &str) -> EntradaPais {
        self.entradas.get(tag).copied().unwrap_or_default()
    }
}

pub fn blocos_para(cat: &CatalogoTec) -> ConfigBlocos {
    let mut b = ConfigBlocos { grandes_potencias: GRANDES_POTENCIAS.iter().map(|s| s.to_string()).collect(), ..Default::default() };
    for tag in cat.iniciais.keys() {
        let lider = if BLOCO_SOVIETICO.contains(&tag.as_str()) { "SOV" } else { "USA" };
        b.lider_do.insert(tag.clone(), lider.to_string());
    }
    b
}

/// Tag econômica a partir da tag do legado.
fn tag_economica(legado: &str) -> String {
    ALIAS_TAGS.iter().find(|(l, _)| *l == legado).map_or(legado, |(_, e)| *e).to_string()
}

pub struct TecnologiaUiPlugin;

impl Plugin for TecnologiaUiPlugin {
    fn build(&self, app: &mut App) {
        let raiz = app.world().get_resource::<RaizProjeto>().map(|r| r.0.clone());
        if let Some(raiz) = raiz {
            match TecnologiaSim::carregar(&raiz) {
                Ok(t) => {
                    app.insert_resource(t);
                }
                Err(e) => error!("tecnologia: {e}"),
            }
        }
        app.add_systems(
            Update,
            (
                alternar_tecnologia,
                avancar_tecnologia.after(EconomiaSimSet::Recolher),
                publicar_bloqueios.after(EconomiaSimSet::Recolher).before(EconomiaSimSet::Despachar),
            )
                .run_if(resource_exists::<TecnologiaSim>)
                .run_if(resource_exists::<EconomiaSim>)
                .run_if(crate::estado::em_partida),
        )
        .add_systems(
            EguiPrimaryContextPass,
            tela_tecnologia
                .run_if(resource_exists::<TecnologiaSim>)
                .run_if(resource_exists::<PaisJogador>)
                .run_if(crate::estado::em_partida),
        );
    }
}

fn alternar_tecnologia(teclas: Res<ButtonInput<KeyCode>>, mut t: ResMut<TecnologiaSim>) {
    if teclas.just_pressed(KeyCode::F5) {
        t.aberto = !t.aberto;
    }
}

/// Recalcula as entradas (universidades, alfabetização) de todos os países.
fn atualizar_entradas(t: &mut TecnologiaSim, eco: &EconomiaSim) {
    let mut por_pais: BTreeMap<&str, (f64, f64, f64)> = BTreeMap::new(); // univ, pop*alfa, pop
    let mut tags: BTreeMap<String, String> = BTreeMap::new(); // id -> tag
    if let Some(n) = &eco.nucleo {
        for p in n.mundo.paises() {
            if let Some(tag) = &p.tag {
                tags.insert(p.id.clone(), tag.clone());
            }
        }
        for pr in n.mundo.provincias() {
            let e = por_pais.entry(pr.pais_id.as_str()).or_default();
            for b in &pr.predios {
                if b.tipo_predio_id == "universidade_pesquisa" {
                    e.0 += b.nivel as f64;
                }
            }
            for pop in &pr.pops {
                e.1 += pop.populacao * pop.alfabetizacao;
                e.2 += pop.populacao;
            }
        }
        for (id, (univ, pa, pop)) in &por_pais {
            if let Some(tag) = tags.get(*id) {
                let alfa = if *pop > 0.0 { pa / pop } else { 0.5 };
                t.entradas.insert(tag.clone(), EntradaPais { niveis_universidade: *univ, alfabetizacao: alfa, ..Default::default() });
            }
        }
    } else {
        // Reserva: PIB per capita do ranking.
        for r in &eco.vista.ranking {
            let tag = r.id.to_uppercase();
            if r.populacao > 0.0 {
                t.entradas.insert(tag, EntradaPais::por_pib_per_capita(r.pib_anual / r.populacao, 0.5));
            }
        }
    }
}

fn avancar_tecnologia(
    mut leitor: MessageReader<EventoTickDiario>,
    mut t: ResMut<TecnologiaSim>,
    eco: Res<EconomiaSim>,
    jogador: Option<Res<PaisJogador>>,
) {
    if let Some(j) = jogador {
        let tag = tag_economica(&j.0);
        if t.tag_jogador.as_deref() != Some(&tag) {
            t.tag_jogador = Some(tag);
            t.sujo = true;
        }
    }
    let t = &mut *t;
    for ev in leitor.read() {
        if ev.total_ticks.abs_diff(t.ultimo_refresh) >= INTERVALO_ENTRADAS || t.ultimo_refresh == u64::MAX {
            // Com o núcleo ocupado em lote usa-se a reserva; tenta de novo no próximo dia.
            let tinha_nucleo = eco.nucleo.is_some();
            atualizar_entradas(t, &eco);
            if tinha_nucleo || t.ultimo_refresh == u64::MAX {
                t.ultimo_refresh = ev.total_ticks;
            }
        }
        let ano = ev.nova_data.ano;
        let tags: Vec<String> = t.mundo.paises.keys().cloned().collect();
        for tag in tags {
            let e = t.entrada(&tag);
            let eh_jogador = t.tag_jogador.as_deref() == Some(tag.as_str());
            if !eh_jogador {
                t.mundo.escolher_ia(&tag, ano, &e);
            }
            if let Some(feita) = t.mundo.tick_pais(&tag, ano, &e) {
                if eh_jogador {
                    let nome = t.mundo.catalogo.obter(&feita).map_or(feita.clone(), |x| x.nome.clone());
                    t.concluidas_jogador.push(nome.clone());
                    t.mensagem = format!("Pesquisa concluída: {nome}");
                    t.sujo = true;
                }
            }
        }
        // Países com IA que viraram jogador "ocioso" não pesquisam sozinhos: pelo desenho, o jogador escolhe.
    }
    if t.sujo {
        if let Some(tag) = t.tag_jogador.clone() {
            t.bloq_predios = Arc::new(t.mundo.bloqueios_predios(&tag));
            t.bloq_metodos = Arc::new(t.mundo.bloqueios_metodos(&tag));
            t.era_jogador = t.mundo.era_visual(&tag);
        }
        t.sujo = false;
    }
}

fn publicar_bloqueios(t: Res<TecnologiaSim>, mut eco: ResMut<EconomiaSim>) {
    if !Arc::ptr_eq(&eco.vista.bloqueio_predios, &t.bloq_predios) {
        eco.vista.bloqueio_predios = t.bloq_predios.clone();
    }
    if !Arc::ptr_eq(&eco.vista.bloqueio_metodos, &t.bloq_metodos) {
        eco.vista.bloqueio_metodos = t.bloq_metodos.clone();
    }
}

// ---------------------------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------------------------

enum Acao {
    Iniciar(String),
    Enfileirar(String),
    Remover(String),
    Ramo(Option<Ramo>),
}

fn bonito(id: &str) -> String {
    let mut s = id.replace('_', " ");
    if let Some(c) = s.get_mut(0..1) {
        c.make_ascii_uppercase();
    }
    s
}

fn nome_tec(m: &MundoTec, id: &str) -> String {
    m.catalogo.obter(id).map_or_else(|| id.to_string(), |t| t.nome.clone())
}

fn cor_ramo(r: Ramo) -> egui::Color32 {
    match r {
        Ramo::Producao => egui::Color32::from_rgb(214, 160, 70),
        Ramo::Sociedade => egui::Color32::from_rgb(110, 180, 120),
        Ramo::Militar => egui::Color32::from_rgb(200, 90, 80),
        Ramo::EspacialNuclear => egui::Color32::from_rgb(120, 150, 220),
    }
}

fn tela_tecnologia(mut contextos: EguiContexts, mut t: ResMut<TecnologiaSim>, relogio: Res<RecursoRelogio>) -> Result {
    if !t.aberto {
        return Ok(());
    }
    let Some(tag) = t.tag_jogador.clone() else { return Ok(()) };
    let ctx = contextos.ctx_mut()?;
    let ano = relogio.data.ano;
    let mut aberto = true;
    let mut acoes: Vec<Acao> = Vec::new();
    let entrada = t.entrada(&tag);
    {
        let m = &t.mundo;
        let est = m.paises.get(&tag);
        egui::Window::new("Tecnologia (F5)").open(&mut aberto).default_size([1180.0, 700.0]).show(ctx, |ui| {
            let Some(est) = est else {
                ui.label("Este país não tem dados de tecnologia.");
                return;
            };
            // ---- Cabeçalho: pesquisa atual, fila, pontos ----
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Pontos/dia: {:.1}", est.pontos_dia.max(sim_tecnologia::pontos_diarios(&entrada))));
                ui.separator();
                ui.label(format!("Reserva: {:.0}/{:.0}", est.buffer, sim_tecnologia::TETO_BUFFER));
                ui.separator();
                ui.label(format!("Conhecidas: {}/{}", est.conhecidas.len(), m.catalogo.techs.len()));
                ui.separator();
                ui.label(format!("Era visual: {}", m.era_visual(&tag)));
                if !t.mensagem.is_empty() {
                    ui.separator();
                    ui.colored_label(egui::Color32::LIGHT_GREEN, &t.mensagem);
                }
            });
            ui.group(|ui| {
                match est.ativa.as_deref().and_then(|id| m.catalogo.obter(id)) {
                    Some(tec) => {
                        let custo = m.custo_efetivo(&tag, tec, ano, &entrada);
                        let prog = est.progresso.get(&tec.id).copied().unwrap_or(0.0);
                        let pts = sim_tecnologia::pontos_diarios(&entrada).max(0.1);
                        let dias = custo * (1.0 - prog) / pts;
                        ui.horizontal(|ui| {
                            ui.colored_label(cor_ramo(tec.ramo), egui::RichText::new(format!("Pesquisando: {}", tec.nome)).strong().size(16.0));
                            ui.label(format!("{:.0} d restantes", dias));
                        });
                        ui.add(egui::ProgressBar::new(prog.clamp(0.0, 1.0) as f32).text(format!("{:.0}% de {:.0} pts", prog * 100.0, custo)).desired_width(ui.available_width()));
                    }
                    None => {
                        ui.colored_label(egui::Color32::YELLOW, "Nenhuma pesquisa ativa: escolha uma tecnologia (os pontos ficam acumulados na reserva).");
                    }
                }
                if !est.fila.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Fila:");
                        for id in &est.fila {
                            if ui.small_button(format!("{} x", nome_tec(m, id))).on_hover_text("Remover da fila").clicked() {
                                acoes.push(Acao::Remover(id.clone()));
                            }
                        }
                    });
                }
            });
            ui.horizontal(|ui| {
                ui.label("Ramo:");
                if ui.selectable_label(t.ramo.is_none(), "Todos").clicked() {
                    acoes.push(Acao::Ramo(None));
                }
                for r in Ramo::TODOS {
                    if ui.selectable_label(t.ramo == Some(r), r.nome()).clicked() {
                        acoes.push(Acao::Ramo(Some(r)));
                    }
                }
            });
            ui.separator();

            // ---- Colunas por ramo ----
            let ramos: Vec<Ramo> = Ramo::TODOS.iter().copied().filter(|r| t.ramo.is_none() || t.ramo == Some(*r)).collect();
            ui.columns(ramos.len(), |cols| {
                for (ci, r) in ramos.iter().enumerate() {
                    let ui = &mut cols[ci];
                    ui.colored_label(cor_ramo(*r), egui::RichText::new(r.nome()).strong().size(15.0));
                    egui::ScrollArea::vertical().id_salt(("tec_col", ci)).auto_shrink([false, false]).show(ui, |ui| {
                        let mut lista: Vec<&Tecnologia> = m.catalogo.techs.iter().filter(|x| x.ramo == *r).collect();
                        lista.sort_by(|a, b| (a.era_visual, a.ano_historico).cmp(&(b.era_visual, b.ano_historico)).then(a.custo.total_cmp(&b.custo)));
                        let mut era_ant = 0;
                        for tec in lista {
                            if tec.era_visual != era_ant {
                                era_ant = tec.era_visual;
                                ui.add_space(4.0);
                                ui.weak(format!("— Era {} —", era_ant));
                            }
                            card(ui, m, &tag, tec, est, ano, &entrada, &mut acoes);
                        }
                    });
                }
            });
        });
    }
    let tec = &mut *t;
    for a in acoes {
        match a {
            Acao::Iniciar(id) => match tec.mundo.iniciar(&tag, &id, ano) {
                Ok(()) => tec.mensagem.clear(),
                Err(e) => tec.mensagem = e,
            },
            Acao::Enfileirar(id) => {
                if let Err(e) = tec.mundo.enfileirar(&tag, &id) {
                    tec.mensagem = e;
                }
            }
            Acao::Remover(id) => tec.mundo.remover_da_fila(&tag, &id),
            Acao::Ramo(r) => tec.ramo = r,
        }
    }
    if !aberto {
        tec.aberto = false;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn card(ui: &mut egui::Ui, m: &MundoTec, tag: &str, tec: &Tecnologia, est: &sim_tecnologia::EstadoPais, ano: i32, entrada: &EntradaPais, acoes: &mut Vec<Acao>) {
    let conhecida = est.conhecidas.contains(&tec.id);
    let ativa = est.ativa.as_deref() == Some(tec.id.as_str());
    let disp = m.disponivel(tag, tec);
    let na_fila = est.fila.contains(&tec.id);
    let fundo = if conhecida {
        egui::Color32::from_rgb(28, 52, 34)
    } else if ativa {
        egui::Color32::from_rgb(28, 42, 70)
    } else if disp {
        egui::Color32::from_gray(40)
    } else {
        egui::Color32::from_gray(26)
    };
    egui::Frame::group(ui.style()).fill(fundo).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            let mut txt = egui::RichText::new(&tec.nome).strong();
            if !conhecida && !disp {
                txt = txt.weak();
            }
            ui.label(txt);
            if conhecida {
                ui.colored_label(egui::Color32::LIGHT_GREEN, "✓");
            }
        })
        .response
        .on_hover_text(&tec.descricao);
        if conhecida {
            return;
        }
        let custo = m.custo_efetivo(tag, tec, ano, entrada);
        let pts = sim_tecnologia::pontos_diarios(entrada).max(0.1);
        let desc = (1.0 - custo / (tec.custo * (1.0 + sim_tecnologia::penalidade_antecedencia(ano, tec.ano_historico)))) * 100.0;
        ui.small(format!("Custo {:.0} pts | ~{:.0} dias | hist. {}", custo, custo / pts, tec.ano_historico));
        if desc > 0.5 {
            ui.small(egui::RichText::new(format!("Difusão: -{:.0}%", desc)).color(egui::Color32::LIGHT_BLUE));
        }
        if !tec.pre_requisitos.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.small("Requer:");
                for p in &tec.pre_requisitos {
                    let c = if est.conhecidas.contains(p) { egui::Color32::LIGHT_GREEN } else { egui::Color32::LIGHT_RED };
                    ui.small(egui::RichText::new(nome_tec(m, p)).color(c));
                }
            });
        }
        let d = &tec.desbloqueia;
        let mut u: Vec<String> = Vec::new();
        u.extend(d.predios.iter().map(|x| format!("prédio {}", bonito(x))));
        u.extend(d.metodos_producao.iter().map(|x| format!("método {}", bonito(x))));
        u.extend(d.leis.iter().map(|x| format!("lei {}", bonito(x))));
        u.extend(d.unidades_militares.iter().map(|x| format!("unidade {}", bonito(x))));
        if !u.is_empty() {
            ui.small(egui::RichText::new(format!("Libera: {}", u.join(", "))).color(egui::Color32::from_rgb(220, 200, 120)));
        }
        if ativa {
            let prog = est.progresso.get(&tec.id).copied().unwrap_or(0.0);
            ui.add(egui::ProgressBar::new(prog.clamp(0.0, 1.0) as f32).desired_width(ui.available_width()));
        } else if disp {
            if let Some(&p) = est.progresso.get(&tec.id) {
                ui.small(format!("Progresso parcial: {:.0}%", p * 100.0));
            }
            if m.pode_iniciar(tag, tec, ano) {
                ui.horizontal(|ui| {
                    if ui.small_button("Pesquisar").clicked() {
                        acoes.push(Acao::Iniciar(tec.id.clone()));
                    }
                    if !na_fila && ui.small_button("+ Fila").clicked() {
                        acoes.push(Acao::Enfileirar(tec.id.clone()));
                    }
                    if na_fila {
                        ui.small("na fila");
                    }
                });
            } else {
                ui.small(egui::RichText::new("Adiantada demais (mais de 10 anos)").color(egui::Color32::LIGHT_RED));
            }
        } else if !na_fila && ui.small_button("+ Fila").on_hover_text("Entra na fila; inicia quando os pré-requisitos forem cumpridos").clicked() {
            acoes.push(Acao::Enfileirar(tec.id.clone()));
        }
    });
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn carrega_pesquisa_e_desbloqueia() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut t = TecnologiaSim::carregar(&raiz).expect("carregar");
        let e = EntradaPais { niveis_universidade: 30.0, alfabetizacao: 0.95, estabilidade: 1.0, ..Default::default() };
        let tag = "BRA";
        let antes = t.mundo.bloqueios_predios(tag).len();
        assert!(antes > 0, "deveria haver prédios travados para BRA");
        let mut feitas = 0;
        for dia in 0..4000u32 {
            t.mundo.escolher_ia(tag, 1945 + (dia / 365) as i32, &e);
            if t.mundo.tick_pais(tag, 1945 + (dia / 365) as i32, &e).is_some() {
                feitas += 1;
            }
        }
        assert!(feitas > 3, "pesquisas concluídas: {feitas}");
        assert!(t.mundo.bloqueios_predios(tag).len() <= antes);
    }
}
