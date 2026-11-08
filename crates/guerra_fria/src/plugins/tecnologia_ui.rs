//! Tela de Tecnologia (5) estilo Victoria 3 + ponte com o motor puro `sim_tecnologia`.
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

#[derive(Clone, Default)]
pub struct LayoutRamoCache {
    pub pres: Vec<Vec<usize>>,
    pub anos: Vec<i32>,
    pub lay: Vec<(usize, usize)>,
}

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
    pan: egui::Vec2,
    zoom: f32,
    /// Avisos de pesquisas concluídas do jogador, para o HUD/tela.
    pub concluidas_jogador: Vec<String>,
    pub cache_layout: BTreeMap<Ramo, LayoutRamoCache>,
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

        let mut cache_layout = BTreeMap::new();
        for &ramo in &Ramo::TODOS {
            let techs: Vec<&Tecnologia> = cat.techs.iter().filter(|x| x.ramo == ramo).collect();
            let idx: BTreeMap<&str, usize> = techs.iter().enumerate().map(|(i, x)| (x.id.as_str(), i)).collect();
            let pres: Vec<Vec<usize>> = techs.iter().map(|x| x.pre_requisitos.iter().filter_map(|p| idx.get(p.as_str()).copied()).collect()).collect();
            let eras: Vec<u32> = techs.iter().map(|x| x.era_visual).collect();
            let anos: Vec<i32> = techs.iter().map(|x| x.ano_historico).collect();
            let lay = layout_arvore(&eras, &anos, &pres);
            cache_layout.insert(ramo, LayoutRamoCache { pres, anos, lay });
        }

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
            pan: egui::vec2(16.0, 16.0),
            zoom: 1.0,
            concluidas_jogador: Vec::new(),
            cache_layout,
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
    if teclas.just_pressed(KeyCode::Digit5) || teclas.just_pressed(KeyCode::Numpad5) {
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

/// Ganho relativo de produtividade por tecnologia civil concluída (+0,6%).
const GANHO_PRODUTIVIDADE_TECNOLOGIA: f64 = 0.006;

fn avancar_tecnologia(
    mut leitor: MessageReader<EventoTickDiario>,
    mut t: ResMut<TecnologiaSim>,
    mut eco: ResMut<EconomiaSim>,
    jogador: Option<Res<PaisJogador>>,
    op: Option<Res<super::smoke::OpcoesCli>>,
) {
    if let Some(j) = jogador {
        let tag = tag_economica(&j.0);
        if t.tag_jogador.as_deref() != Some(&tag) {
            t.tag_jogador = Some(tag);
            t.sujo = true;
        }
    }
    let eh_espectador = op.as_ref().is_some_and(|o| o.espectador_ativo());
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
            if !eh_jogador || eh_espectador {
                t.mundo.escolher_ia(&tag, ano, &e);
            }
            if let Some(feita) = t.mundo.tick_pais(&tag, ano, &e) {
                // Tecnologia civil (produção/sociedade) eleva a produtividade total dos fatores do país.
                if t.mundo.catalogo.obter(&feita).is_some_and(|x| matches!(x.ramo, Ramo::Producao | Ramo::Sociedade)) {
                    eco.registrar_ganho_tecnologico(&tag, GANHO_PRODUTIVIDADE_TECNOLOGIA);
                }
                eco.registrar_tecnologia_pesquisada(&tag, &feita);
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
        Ramo::Producao => ui_refinada::menu::AVISO,
        Ramo::Sociedade => ui_refinada::menu::SUCESSO,
        Ramo::Militar => ui_refinada::menu::VERMELHO,
        Ramo::EspacialNuclear => ui_refinada::menu::AZUL_MARINHO,
    }
}

fn glifo_ramo(r: Ramo) -> &'static str {
    match r {
        Ramo::Producao => "fabrica",
        Ramo::Sociedade => "pessoa",
        Ramo::Militar => "canhao",
        Ramo::EspacialNuclear => "estrela",
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
    let mut pan_zoom = (t.pan, t.zoom);
    {
        let m = &t.mundo;
        let est = m.paises.get(&tag);
        let resp_janela = ui_refinada::janela_dossie("Tecnologia (5)", None)
            .default_pos([20.0, 96.0])
            .default_size([1180.0, 700.0])
            .show(ctx, |ui| {
            ui_refinada::faixa_titulo(ui, "Árvore Tecnológica", Some("5"), Some(&mut aberto));
            let Some(est) = est else {
                ui.label("Este país não tem dados de tecnologia.");
                return;
            };
            // ---- Placar: pontos, reserva, conhecidas, era ----
            ui.horizontal_wrapped(|ui| {
                ui_refinada::chapa(
                    ui,
                    "raio",
                    "Pontos/dia",
                    &format!("{:.1}", est.pontos_dia.max(sim_tecnologia::pontos_diarios(&entrada))),
                    Some(ui_refinada::menu::VERMELHO),
                );
                ui_refinada::chapa(
                    ui,
                    "moeda",
                    "Reserva",
                    &format!("{:.0}/{:.0}", est.buffer, sim_tecnologia::TETO_BUFFER),
                    None,
                );
                ui_refinada::chapa(
                    ui,
                    "livro",
                    "Conhecidas",
                    &format!("{}/{}", est.conhecidas.len(), m.catalogo.techs.len()),
                    None,
                );
                ui_refinada::chapa(ui, "estrela", "Era visual", &format!("{}", m.era_visual(&tag)), None);
                if !t.mensagem.is_empty() {
                    ui.colored_label(ui_refinada::menu::SUCESSO, &t.mensagem);
                }
            });
            ui_refinada::moldura_cartao().show(ui, |ui| {
                match est.ativa.as_deref().and_then(|id| m.catalogo.obter(id)) {
                    Some(tec) => {
                        let custo = m.custo_efetivo(&tag, tec, ano, &entrada);
                        let prog = est.progresso.get(&tec.id).copied().unwrap_or(0.0);
                        let pts = sim_tecnologia::pontos_diarios(&entrada).max(0.1);
                        let dias = custo * (1.0 - prog) / pts;
                        let cor = cor_ramo(tec.ramo);
                        let rotulo = format!("Pesquisando · {} ", tec.nome);
                        ui.horizontal(|ui| {
                            ui_refinada::glifo_ui(ui, glifo_ramo(tec.ramo), 17.0, Some(cor));
                            ui.label(
                                egui::RichText::new(rotulo.as_str())
                                    .family(ui_refinada::familia_oswald(ui.ctx()))
                                    .size(16.0)
                                    .strong()
                                    .color(cor),
                            );
                            ui.weak(format!("termina em {:.0} d", dias));
                        });
                        let restante = format!("{:.0}% de {:.0} pts", prog * 100.0, custo);
                        ui_refinada::widgets::barra_status(
                            ui,
                            None,
                            (prog.clamp(0.0, 1.0) * custo) as f32,
                            custo as f32,
                            cor,
                            ui.available_width(),
                            Some(&restante),
                        );
                    }
                    None => {
                        ui.colored_label(ui_refinada::menu::AVISO, "Nenhuma pesquisa ativa: escolha uma tecnologia (os pontos ficam acumulados na reserva).");
                    }
                }
                if !est.fila.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Fila:");
                        for id in &est.fila {
                            if ui.small_button(format!("{}  x", nome_tec(m, id))).on_hover_text("Remover da fila").clicked() {
                                acoes.push(Acao::Remover(id.clone()));
                            }
                        }
                    });
                }
            });
            let ramo_atual = t.ramo.unwrap_or(Ramo::Producao);
            ui.horizontal(|ui| {
                for r in Ramo::TODOS {
                    if ui_refinada::aba_glifo(ui, ramo_atual == r, Some(glifo_ramo(r)), r.nome()) {
                        acoes.push(Acao::Ramo(Some(r)));
                    }
                }
                ui.weak("  arraste = mover, scroll = zoom, clique = pesquisar/fila");
            });
            ui.separator();

            if let Some(cache) = t.cache_layout.get(&ramo_atual) {
                arvore(ui, m, &tag, ramo_atual, est, ano, &entrada, &mut pan_zoom, &mut acoes, cache);
            }
        });
        if let Some(r) = resp_janela {
            super::tutorial_guiado::marcar(&r.response, "janela_tec");
        }
    }
    let tec = &mut *t;
    tec.pan = pan_zoom.0;
    tec.zoom = pan_zoom.1;
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

const NO_L: f32 = 180.0;
const NO_A: f32 = 66.0;
const GAP_X: f32 = 56.0;
const GAP_Y: f32 = 14.0;

/// Nome do nó dividido em no máximo duas linhas (`max` caracteres por linha), com reticências
/// quando ainda sobra texto. Evita rótulo cortado no cartão da árvore.
fn nome_em_linhas(nome: &str, max: usize) -> (String, Option<String>) {
    let mut l1 = String::new();
    let mut l2 = String::new();
    let mut transbordou = false;
    for p in nome.split_whitespace() {
        let alvo = if l2.is_empty() && (l1.is_empty() || l1.chars().count() + 1 + p.chars().count() <= max) {
            &mut l1
        } else if l2.is_empty() || l2.chars().count() + 1 + p.chars().count() <= max {
            &mut l2
        } else {
            transbordou = true;
            break;
        };
        if !alvo.is_empty() {
            alvo.push(' ');
        }
        alvo.push_str(p);
    }
    if transbordou {
        let base: String = l2.chars().take(max.saturating_sub(3)).collect();
        l2 = format!("{base}...");
    }
    (l1, if l2.is_empty() { None } else { Some(l2) })
}

/// Layout em colunas: coluna = max(índice da era no ramo, coluna dos pré-requisitos + 1);
/// linha = ordem por média da posição dos pré-requisitos (baricentro), sem sobreposição.
/// Retorna (coluna, linha) por nó. `pres` só contém pré-requisitos dentro do mesmo conjunto.
pub fn layout_arvore(eras: &[u32], anos: &[i32], pres: &[Vec<usize>]) -> Vec<(usize, usize)> {
    let n = eras.len();
    let mut era_ord: Vec<u32> = eras.to_vec();
    era_ord.sort_unstable();
    era_ord.dedup();
    let mut col: Vec<usize> = eras.iter().map(|e| era_ord.iter().position(|x| x == e).unwrap_or(0)).collect();
    for _ in 0..=n {
        let mut mudou = false;
        for i in 0..n {
            for &p in &pres[i] {
                if p != i && col[i] <= col[p] && col[p] < n {
                    col[i] = col[p] + 1;
                    mudou = true;
                }
            }
        }
        if !mudou {
            break;
        }
    }
    let ncol = col.iter().copied().max().map_or(0, |m| m + 1);
    let mut colunas: Vec<Vec<usize>> = vec![Vec::new(); ncol];
    for i in 0..n {
        colunas[col[i]].push(i);
    }
    let mut pos = vec![0.0f32; n];
    for c in colunas.iter_mut() {
        c.sort_by_key(|&i| (anos[i], i));
        for (r, &i) in c.iter().enumerate() {
            pos[i] = r as f32;
        }
    }
    let mut filhos: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for &p in &pres[i] {
            filhos[p].push(i);
        }
    }
    let media = |v: &[usize], pos: &[f32], atual: f32| -> f32 {
        if v.is_empty() { atual } else { v.iter().map(|&j| pos[j]).sum::<f32>() / v.len() as f32 }
    };
    for passo in 0..6 {
        let ordem: Vec<usize> = if passo % 2 == 0 { (0..ncol).collect() } else { (0..ncol).rev().collect() };
        for c in ordem {
            let mut chaves: Vec<(f32, usize)> = colunas[c]
                .iter()
                .map(|&i| {
                    let v = if passo % 2 == 0 { &pres[i] } else { &filhos[i] };
                    (media(v, &pos, pos[i]), i)
                })
                .collect();
            chaves.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            for (r, (_, i)) in chaves.iter().enumerate() {
                pos[*i] = r as f32;
            }
            colunas[c] = chaves.into_iter().map(|(_, i)| i).collect();
        }
    }
    (0..n).map(|i| (col[i], pos[i] as usize)).collect()
}

fn cor_estado(conhecida: bool, ativa: bool, disp: bool) -> egui::Color32 {
    if conhecida {
        ui_refinada::menu::SUCESSO.gamma_multiply(0.20)
    } else if ativa {
        ui_refinada::menu::AZUL_MARINHO.gamma_multiply(0.22)
    } else if disp {
        ui_refinada::menu::PAPEL_CLARO
    } else {
        ui_refinada::menu::PAPEL_ALT
    }
}

#[allow(clippy::too_many_arguments)]
fn arvore(
    ui: &mut egui::Ui,
    m: &MundoTec,
    tag: &str,
    ramo: Ramo,
    est: &sim_tecnologia::EstadoPais,
    ano: i32,
    entrada: &EntradaPais,
    pan_zoom: &mut (egui::Vec2, f32),
    acoes: &mut Vec<Acao>,
    cache: &LayoutRamoCache,
) {
    let techs: Vec<&Tecnologia> = m.catalogo.techs.iter().filter(|x| x.ramo == ramo).collect();
    let pres = &cache.pres;
    let anos = &cache.anos;
    let lay = &cache.lay;

    let tam = egui::vec2(ui.available_width(), ui.available_height().max(420.0));
    let oswald = ui_refinada::familia_oswald(ui.ctx());
    let (resp, painter) = ui.allocate_painter(tam, egui::Sense::click_and_drag());
    let rect = resp.rect;
    let painter = painter.with_clip_rect(rect);
    painter.rect_filled(rect, 4.0, ui_refinada::menu::PAPEL_CARD);
    if resp.dragged() {
        pan_zoom.0 += resp.drag_delta();
    }
    if resp.hovered() {
        let rolar = ui.input(|i| i.smooth_scroll_delta.y);
        if rolar != 0.0 {
            let z0 = pan_zoom.1;
            let z1 = (z0 * (1.0 + rolar * 0.002)).clamp(0.35, 2.2);
            if let Some(p) = resp.hover_pos() {
                // mantém o ponto sob o cursor fixo
                let rel = p - rect.min - pan_zoom.0;
                pan_zoom.0 -= rel * (z1 / z0 - 1.0);
            }
            pan_zoom.1 = z1;
        }
    }
    let z = pan_zoom.1;
    let origem = rect.min + pan_zoom.0;
    let ncol = lay.iter().map(|l| l.0).max().map_or(0, |c| c + 1);
    let nr = |i: usize| -> egui::Rect {
        let (c, r) = lay[i];
        let p = origem + egui::vec2(c as f32 * (NO_L + GAP_X), r as f32 * (NO_A + GAP_Y)) * z;
        egui::Rect::from_min_size(p, egui::vec2(NO_L, NO_A) * z)
    };
    // cabeçalhos de coluna: ano médio da coluna
    for c in 0..ncol {
        let ys: Vec<i32> = (0..techs.len()).filter(|&i| lay[i].0 == c).map(|i| anos[i]).collect();
        if ys.is_empty() {
            continue;
        }
        let (mn, mx) = (ys.iter().min().copied().unwrap_or(0), ys.iter().max().copied().unwrap_or(0));
        let x = origem.x + c as f32 * (NO_L + GAP_X) * z;
        let txt = if mn == mx { format!("{mn}") } else { format!("{mn}–{mx}") };
        painter.text(egui::pos2(x, rect.min.y + 2.0), egui::Align2::LEFT_TOP, txt, egui::FontId::proportional(12.0), ui_refinada::menu::TINTA_SUAVE);
    }
    // linhas
    let cr = cor_ramo(ramo);
    for (i, ps) in pres.iter().enumerate() {
        for &p in ps {
            let (a, b) = (nr(p), nr(i));
            if !a.union(b).intersects(rect) {
                continue;
            }
            let (ini, fim) = (a.right_center(), b.left_center());
            let dx = ((fim.x - ini.x) * 0.5).max(10.0 * z);
            let ok = est.conhecidas.contains(&techs[p].id);
            let cor = if ok { cr } else { ui_refinada::menu::PAPEL_BORDA };
            let st = egui::Stroke::new((1.6 * z).max(1.0), cor);
            painter.add(egui::epaint::CubicBezierShape::from_points_stroke([ini, ini + egui::vec2(dx, 0.0), fim - egui::vec2(dx, 0.0), fim], false, egui::Color32::TRANSPARENT, st));
            let s = 5.0 * z.max(0.6);
            painter.add(egui::Shape::convex_polygon(vec![fim, fim - egui::vec2(s * 1.6, s), fim - egui::vec2(s * 1.6, -s)], cor, egui::Stroke::NONE));
        }
    }
    // nós
    let pts = sim_tecnologia::pontos_diarios(entrada).max(0.1);
    for (i, tec) in techs.iter().enumerate() {
        let r = nr(i);
        if !r.intersects(rect) {
            continue;
        }
        let conhecida = est.conhecidas.contains(&tec.id);
        let ativa = est.ativa.as_deref() == Some(tec.id.as_str());
        let disp = m.disponivel(tag, tec);
        let na_fila = est.fila.contains(&tec.id);
        let custo = m.custo_efetivo(tag, tec, ano, entrada);
        let r_int = ui.interact(r.intersect(rect), resp.id.with(("no", i)), egui::Sense::click());
        let mut fundo = cor_estado(conhecida, ativa, disp);
        if r_int.hovered() {
            fundo = fundo.gamma_multiply(1.35);
        }
        let borda = if ativa { ui_refinada::menu::AZUL_MARINHO } else if na_fila { ui_refinada::menu::AVISO } else if conhecida { ui_refinada::menu::SUCESSO } else { cr.gamma_multiply(if disp { 1.0 } else { 0.4 }) };
        painter.rect(r, 5.0 * z, fundo, egui::Stroke::new(1.5, borda), egui::StrokeKind::Inside);
        let txt_cor = if conhecida || disp || ativa { ui_refinada::menu::TINTA } else { ui_refinada::menu::TINTA_SUAVE };
        let fs = (13.0 * z).max(7.0);
        if z >= 0.45 {
            let max_chars = (((NO_L - 16.0) / (fs * 0.5)) as usize).clamp(8, 60);
            let (l1, l2) = nome_em_linhas(&tec.nome, max_chars);
            painter.text(r.min + egui::vec2(8.0, 5.0) * z, egui::Align2::LEFT_TOP, &l1, egui::FontId::new(fs, oswald.clone()), txt_cor);
            let sub_y = if let Some(l2) = &l2 {
                painter.text(r.min + egui::vec2(8.0, 19.0) * z, egui::Align2::LEFT_TOP, l2, egui::FontId::new(fs, oswald.clone()), txt_cor);
                33.0
            } else {
                28.0
            };
            let sub = if conhecida { "pesquisada".to_string() } else { format!("{:.0} pts · ~{:.0} d · {}", custo, custo / pts, tec.ano_historico) };
            painter.text(r.min + egui::vec2(8.0, sub_y) * z, egui::Align2::LEFT_TOP, sub, egui::FontId::proportional((11.0 * z).max(7.0)), if conhecida { ui_refinada::menu::SUCESSO } else { ui_refinada::menu::TINTA_SUAVE });
            if conhecida {
                let o = r.max - egui::vec2(15.0, 17.0) * z;
                let s = egui::Stroke::new((2.2 * z).max(1.2), ui_refinada::menu::SUCESSO);
                let vin = o + egui::vec2(3.4, 3.6) * z;
                painter.line_segment([o, vin], s);
                painter.line_segment([vin, o + egui::vec2(8.0, -3.6) * z], s);
            }
        }
        if ativa {
            let prog = est.progresso.get(&tec.id).copied().unwrap_or(0.0).clamp(0.0, 1.0) as f32;
            let barra = egui::Rect::from_min_max(egui::pos2(r.min.x + 6.0 * z, r.max.y - 12.0 * z), egui::pos2(r.max.x - 6.0 * z, r.max.y - 6.0 * z));
            painter.rect_filled(barra, 2.0, ui_refinada::menu::PAPEL_BORDA);
            painter.rect_filled(egui::Rect::from_min_size(barra.min, egui::vec2(barra.width() * prog, barra.height())), 2.0, ui_refinada::menu::AZUL_MARINHO);
        } else if !conhecida {
            if let Some(&p) = est.progresso.get(&tec.id) {
                let barra = egui::Rect::from_min_max(egui::pos2(r.min.x + 6.0 * z, r.max.y - 9.0 * z), egui::pos2(r.max.x - 6.0 * z, r.max.y - 6.0 * z));
                painter.rect_filled(egui::Rect::from_min_size(barra.min, egui::vec2(barra.width() * p.clamp(0.0, 1.0) as f32, barra.height())), 1.0, ui_refinada::menu::OURO);
            }
        }
        if r_int.clicked() && !conhecida && !ativa {
            if disp && m.pode_iniciar(tag, tec, ano) && est.ativa.is_none() {
                acoes.push(Acao::Iniciar(tec.id.clone()));
            } else if !na_fila {
                acoes.push(Acao::Enfileirar(tec.id.clone()));
            }
        }
        r_int.on_hover_ui(|ui| tooltip(ui, m, tec, est, ano, entrada, custo, disp));
    }
}

#[allow(clippy::too_many_arguments)]
fn tooltip(ui: &mut egui::Ui, m: &MundoTec, tec: &Tecnologia, est: &sim_tecnologia::EstadoPais, ano: i32, _entrada: &EntradaPais, custo: f64, disp: bool) {
    ui.set_max_width(320.0);
    ui.colored_label(cor_ramo(tec.ramo), egui::RichText::new(&tec.nome).strong().size(15.0));
    if !tec.descricao.is_empty() {
        ui.label(&tec.descricao);
    }
    if est.conhecidas.contains(&tec.id) {
        ui.colored_label(ui_refinada::menu::SUCESSO, "Já pesquisada");
    } else {
        let base = tec.custo * (1.0 + sim_tecnologia::penalidade_antecedencia(ano, tec.ano_historico));
        let desc = (1.0 - custo / base) * 100.0;
        ui.small(format!("Custo {:.0} pts | histórica {}", custo, tec.ano_historico));
        if desc > 0.5 {
            ui.small(egui::RichText::new(format!("Desconto de difusão: -{:.0}%", desc)).color(ui_refinada::menu::INFO));
        }
        if disp && ano < tec.ano_historico - 10 {
            ui.small(egui::RichText::new("Adiantada demais (mais de 10 anos)").color(ui_refinada::menu::PERIGO));
        }
        ui.small(if est.ativa.is_none() { "Clique: pesquisar" } else { "Clique: pôr na fila" });
    }
    if !tec.pre_requisitos.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.small("Requer:");
            for p in &tec.pre_requisitos {
                let c = if est.conhecidas.contains(p) { ui_refinada::menu::SUCESSO } else { ui_refinada::menu::PERIGO };
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
        ui.small(egui::RichText::new(format!("Libera: {}", u.join(", "))).color(ui_refinada::menu::VERMELHO));
    }
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

    fn checa(eras: &[u32], anos: &[i32], pres: &[Vec<usize>]) {
        let l = layout_arvore(eras, anos, pres);
        let mut vistos = std::collections::HashSet::new();
        for (i, &(c, r)) in l.iter().enumerate() {
            assert!(vistos.insert((c, r)), "nós sobrepostos em {c},{r}");
            for &p in &pres[i] {
                assert!(l[p].0 < c, "pré-requisito {p} não está antes de {i}");
            }
        }
    }

    #[test]
    fn layout_sem_sobreposicao_e_prereq_antes() {
        checa(&[1, 1, 1, 2, 2, 2], &[1945, 1946, 1947, 1950, 1951, 1952], &[vec![], vec![], vec![], vec![0, 1], vec![2], vec![3]]);
        checa(&[1, 1, 1], &[1945, 1945, 1945], &[vec![], vec![0], vec![1]]);
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let t = TecnologiaSim::carregar(&raiz).expect("carregar");
        for r in Ramo::TODOS {
            let techs: Vec<&Tecnologia> = t.mundo.catalogo.techs.iter().filter(|x| x.ramo == r).collect();
            let idx: BTreeMap<&str, usize> = techs.iter().enumerate().map(|(i, x)| (x.id.as_str(), i)).collect();
            let pres: Vec<Vec<usize>> = techs.iter().map(|x| x.pre_requisitos.iter().filter_map(|p| idx.get(p.as_str()).copied()).collect()).collect();
            let eras: Vec<u32> = techs.iter().map(|x| x.era_visual).collect();
            let anos: Vec<i32> = techs.iter().map(|x| x.ano_historico).collect();
            checa(&eras, &anos, &pres);
        }
    }
}
