//! Tutorial guiado do menu principal: partida especial com os EUA, passo a passo, com overlay escuro,
//! "buraco" no elemento destacado, seta/retângulo pulsante e bloqueio de atalhos fora do passo atual.
//!
//! A máquina de estados (`Maquina`) é pura e testada sem janela; os sistemas só leem o estado do jogo
//! (`Fatos`), alimentam a máquina e desenham a interface.
use super::comercio_ui::PainelComercio;
use super::diplomacia_ui::{DiploSim, PainelDiplomacia};
use super::construcao_ui::ConstrucaoUi;
use super::economy::RecursoMundo;
use super::eventos_ui::EstadoEv;
use super::jogador::{PainelPais, PaisJogador};
use super::map::{DadosMapa, ProvinciaSelecionada, RecursoCatalogo};
use super::mercado_ui::PainelMercado;
use super::politica_sim::PoliticaSim;
use super::saveload::PedirSalvar;
use super::tecnologia_ui::TecnologiaSim;
use super::time::RecursoRelogio;
use crate::estado::AppState;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use std::collections::HashMap;

const PAPEL: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xDE, 0xC8);
const VERMELHO: egui::Color32 = egui::Color32::from_rgb(0x8C, 0x25, 0x1E);
const TINTA: egui::Color32 = egui::Color32::from_rgb(0x23, 0x20, 0x1B);

// ---------------------------------------------------------------------------------------------
// Registro de alvos (retângulos dos widgets), guardado na memória do egui
// ---------------------------------------------------------------------------------------------

type MapaAlvos = HashMap<&'static str, (egui::Rect, u64)>;

/// Registra o retângulo de um widget-alvo (chamado pelos painéis a cada quadro; custo desprezível).
pub fn alvo(ctx: &egui::Context, nome: &'static str, rect: egui::Rect) {
    let quadro = ctx.cumulative_frame_nr();
    ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with::<MapaAlvos>(egui::Id::NULL, Default::default).insert(nome, (rect, quadro));
    });
}

/// Atalho: registra o retângulo de uma `Response`.
pub fn marcar(r: &egui::Response, nome: &'static str) {
    alvo(&r.ctx, nome, r.rect);
}

fn alvo_recente(ctx: &egui::Context, nome: &str) -> Option<egui::Rect> {
    let quadro = ctx.cumulative_frame_nr();
    ctx.data(|d| d.get_temp::<MapaAlvos>(egui::Id::NULL)).and_then(|m| m.get(nome).copied()).filter(|(_, q)| quadro.saturating_sub(*q) <= 3).map(|(r, _)| r)
}

// ---------------------------------------------------------------------------------------------
// Passos
// ---------------------------------------------------------------------------------------------

/// Onde o jogador deve olhar/clicar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alvo {
    /// Área livre do mapa (nada em volta fica clicável).
    Mapa,
    /// Tela toda escurecida (só teclado).
    Teclado,
    /// Um widget registrado por id.
    Widget(&'static str),
    /// Um widget e as janelas abertas (para passos "abra e faça algo na janela").
    WidgetEJanelas(&'static str),
    /// Província (por nome de estado) do país do jogador.
    Provincia(&'static str),
}

/// Condição de avanço: (estado atual, estado ao entrar no passo) -> concluído?
#[derive(Clone, Copy)]
pub enum Cond {
    Manual,
    Auto(fn(&Fatos, &Fatos) -> bool),
}

pub struct Passo {
    pub titulo: &'static str,
    pub texto: &'static str,
    pub objetivo: &'static str,
    pub alvo: Alvo,
    /// Teclas liberadas neste passo (modificadores sempre passam).
    pub teclas: &'static [KeyCode],
    pub cond: Cond,
}

use KeyCode::*;

pub static PASSOS: [Passo; 18] = [
    Passo {
        titulo: "O mapa e a câmera",
        texto: "Bem-vindo, Presidente! Você governa os Estados Unidos em 1º de janeiro de 1945. O mapa mostra as províncias de todos os países. Mova a câmera com W A S D ou as setas, arraste com o mouse e use a roda para aproximar ou afastar.",
        objetivo: "Mova a câmera ou use o zoom.",
        alvo: Alvo::Mapa,
        teclas: &[KeyW, KeyA, KeyS, KeyD, ArrowUp, ArrowDown, ArrowLeft, ArrowRight],
        cond: Cond::Auto(|a, b| a.camera != b.camera),
    },
    Passo {
        titulo: "Escolha uma província",
        texto: "Clique numa província para selecioná-la. O painel à direita mostra o dono, a população, a economia do país e, nas suas terras, a seção \"Construir aqui\". Vamos começar pela Califórnia, marcada no mapa.",
        objetivo: "Clique na Califórnia.",
        alvo: Alvo::Provincia("Califórnia"),
        teclas: &[],
        cond: Cond::Auto(|a, _| a.prov_usa),
    },
    Passo {
        titulo: "A barra de status (HUD)",
        texto: "No topo ficam o Tesouro, o saldo diário, a Dívida, o PIB, a população, o padrão de vida e a Legitimidade do governo. Passe o mouse sobre cada número para ver um tooltip explicando o que ele significa.",
        objetivo: "Passe o mouse nos números e clique em \"Próximo\".",
        alvo: Alvo::Widget("hud"),
        teclas: &[],
        cond: Cond::Manual,
    },
    Passo {
        titulo: "O relógio: continuar",
        texto: "O tempo começa pausado. A data e os controles ficam no canto superior direito. Clique em \"Continuar\" (ou aperte Espaço) para o tempo correr.",
        objetivo: "Clique em \"Continuar\".",
        alvo: Alvo::Widget("pausa"),
        teclas: &[Space],
        cond: Cond::Auto(|a, _| !a.pausado),
    },
    Passo {
        titulo: "O relógio: velocidade",
        texto: "As barrinhas, o \"+\" e o \"−\" mudam a velocidade (também as teclas + e −). Escolha outra velocidade.",
        objetivo: "Troque a velocidade do jogo.",
        alvo: Alvo::Widget("relogio"),
        teclas: &[Equal, Minus, NumpadAdd, NumpadSubtract, Space],
        cond: Cond::Auto(|a, b| a.velocidade != b.velocidade),
    },
    Passo {
        titulo: "O relógio: pausar",
        texto: "Pausar é a sua melhor ferramenta: com o tempo parado você lê, planeja e dá ordens sem pressa. Clique em \"Pausar\" (ou Espaço).",
        objetivo: "Pause o jogo.",
        alvo: Alvo::Widget("pausa"),
        teclas: &[Space],
        cond: Cond::Auto(|a, _| a.pausado),
    },
    Passo {
        titulo: "Modos de mapa",
        texto: "Os modos de mapa pintam o mundo por PIB, população, ideologia, alinhamento e mais. Troque pelo seletor no canto inferior direito (setas < >, a lista) ou apertando M.",
        objetivo: "Troque o modo de mapa.",
        alvo: Alvo::Widget("modos"),
        teclas: &[KeyM],
        cond: Cond::Auto(|a, b| a.modo_mapa != b.modo_mapa),
    },
    Passo {
        titulo: "Escolha um estado para construir",
        texto: "O Texas é grande, tem terra, petróleo e espaço para a indústria. Clique nele no mapa: cada estado tem seus próprios recursos e infraestrutura.",
        objetivo: "Clique no Texas.",
        alvo: Alvo::Provincia("Texas"),
        teclas: &[],
        cond: Cond::Auto(|a, b| a.prov_usa && a.prov_id != b.prov_id),
    },
    Passo {
        titulo: "Construir um prédio",
        texto: "No painel da direita, em \"Construir aqui\", a lista mostra o que cabe no estado e o lucro esperado por dia. Escolha um prédio de bom lucro e clique em \"Construir\". Obras gastam bens e pontos de construção.",
        objetivo: "Mande construir um prédio.",
        alvo: Alvo::Widget("painel_provincia"),
        teclas: &[],
        cond: Cond::Auto(|a, b| a.obras > b.obras),
    },
    Passo {
        titulo: "A tela e a fila de obras",
        texto: "A tela de construção (tecla B ou botão verde) mostra todos os modos de mapa de potencial e a fila de obras, com o progresso de cada uma, estatal ou privada. Abra-a para ver a sua obra na fila.",
        objetivo: "Abra a tela de construção (B).",
        alvo: Alvo::Widget("btn_construir"),
        teclas: &[KeyB],
        cond: Cond::Auto(|a, _| a.construcao_aberta),
    },
    Passo {
        titulo: "Mercado (2)",
        texto: "O mercado mostra preço, oferta e demanda de cada bem. Preços altos indicam escassez e oportunidade de lucro; baixos, excesso de produção. Abra o painel.",
        objetivo: "Abra o Mercado (2).",
        alvo: Alvo::Widget("btn_mercado"),
        teclas: &[Digit2, Numpad2],
        cond: Cond::Auto(|a, _| a.mercado_aberto),
    },
    Passo {
        titulo: "Comércio (3)",
        texto: "Aqui você abre rotas comerciais com outros países, define tarifas e firma acordos. Exportar o que sobra e importar o que falta aumenta a riqueza da nação.",
        objetivo: "Abra o Comércio (3).",
        alvo: Alvo::Widget("btn_comercio"),
        teclas: &[Digit3, Numpad3],
        cond: Cond::Auto(|a, _| a.comercio_aberto),
    },
    Passo {
        titulo: "Política (1): propor uma lei",
        texto: "No painel do País (1), na parte de Leis, abra uma categoria e clique em \"Propor lei\". Só uma lei tramita por vez e passa por fases até a votação, dependendo do apoio dos grupos de interesse.",
        objetivo: "Abra o painel (1) e proponha uma lei.",
        alvo: Alvo::WidgetEJanelas("btn_pais"),
        teclas: &[Digit1, Numpad1],
        cond: Cond::Auto(|a, _| a.lei_em_tramitacao),
    },
    Passo {
        titulo: "Diplomacia (4)",
        texto: "Abra a Diplomacia, escolha o Reino Unido na lista e use \"Melhorar relações\". Relações melhores abrem caminho para acordos comerciais e alianças.",
        objetivo: "Melhore as relações com o Reino Unido.",
        alvo: Alvo::WidgetEJanelas("btn_diplo"),
        teclas: &[Digit4, Numpad4],
        cond: Cond::Auto(|a, b| a.relacao_gbr > b.relacao_gbr + 0.001),
    },
    Passo {
        titulo: "Tecnologia (5)",
        texto: "Abra a Tecnologia, procure uma pesquisa disponível e clique em \"Pesquisar\" (ou \"+ Fila\"). Tecnologias liberam prédios e métodos de produção melhores.",
        objetivo: "Comece uma pesquisa.",
        alvo: Alvo::WidgetEJanelas("btn_tec"),
        teclas: &[Digit5, Numpad5],
        cond: Cond::Auto(|a, _| a.pesquisando),
    },
    Passo {
        titulo: "Eventos e o Jornal (J)",
        texto: "Durante o jogo surgem eventos com escolhas que mudam o país. O Jornal reúne as notícias e decisões recentes. Aperte J para abrir.",
        objetivo: "Abra o Jornal (J).",
        alvo: Alvo::Teclado,
        teclas: &[KeyJ],
        cond: Cond::Auto(|a, _| a.jornal_aberto),
    },
    Passo {
        titulo: "Salvar o jogo",
        texto: "Salve com Ctrl+S (salvamento rápido) ou pelo botão \"Salvar / Carregar\", que também carrega e gerencia seus jogos. O jogo também grava um autosave a cada ano.",
        objetivo: "Salve com Ctrl+S ou pelo menu.",
        alvo: Alvo::WidgetEJanelas("salvar"),
        teclas: &[KeyS],
        cond: Cond::Auto(|a, b| a.salvos > b.salvos),
    },
    Passo {
        titulo: "Tutorial concluído",
        texto: "Parabéns! Você viu o mapa, o relógio, a construção, o mercado, o comércio, a política, a diplomacia, a tecnologia, o jornal e o salvamento. A enciclopédia (8) explica cada conceito em detalhe. Boa sorte, Presidente.",
        objetivo: "Escolha como continuar.",
        alvo: Alvo::Teclado,
        teclas: &[],
        cond: Cond::Manual,
    },
];

// ---------------------------------------------------------------------------------------------
// Fatos e máquina de estados (pura)
// ---------------------------------------------------------------------------------------------

/// Retrato do estado do jogo relevante ao tutorial.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fatos {
    pub camera: (i32, i32, i32),
    pub prov_id: u32,
    pub prov_usa: bool,
    pub pausado: bool,
    pub velocidade: u8,
    pub modo_mapa: String,
    pub construcao_aberta: bool,
    pub obras: u32,
    pub mercado_aberto: bool,
    pub comercio_aberto: bool,
    pub lei_em_tramitacao: bool,
    pub relacao_gbr: f32,
    pub pesquisando: bool,
    pub jornal_aberto: bool,
    pub salvos: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoTut {
    Andando,
    ConfirmandoSaida,
    Concluido,
    Saiu,
}

#[derive(Debug, Clone)]
pub struct Maquina {
    pub passo: usize,
    pub base: Fatos,
    pub estado: EstadoTut,
    pub pulos: u32,
}

impl Maquina {
    pub fn nova(f: &Fatos) -> Self {
        Self { passo: 0, base: f.clone(), estado: EstadoTut::Andando, pulos: 0 }
    }

    pub fn passo_atual(&self) -> &'static Passo {
        &PASSOS[self.passo.min(PASSOS.len() - 1)]
    }

    pub fn ultimo(&self) -> bool {
        self.passo + 1 >= PASSOS.len()
    }

    /// Avança se a condição do passo foi cumprida. Devolve true quando mudou de passo.
    pub fn atualizar(&mut self, f: &Fatos) -> bool {
        if self.estado != EstadoTut::Andando {
            return false;
        }
        match self.passo_atual().cond {
            Cond::Auto(c) if c(f, &self.base) => self.avancar(f),
            _ => false,
        }
    }

    fn avancar(&mut self, f: &Fatos) -> bool {
        if self.ultimo() {
            self.estado = EstadoTut::Concluido;
            return false;
        }
        self.passo += 1;
        self.base = f.clone();
        true
    }

    /// "Próximo" (passos manuais).
    pub fn proximo(&mut self, f: &Fatos) -> bool {
        if self.estado == EstadoTut::Andando && matches!(self.passo_atual().cond, Cond::Manual) { self.avancar(f) } else { false }
    }

    /// "Pular passo": avança sem cumprir a condição.
    pub fn pular(&mut self, f: &Fatos) -> bool {
        if self.estado != EstadoTut::Andando {
            return false;
        }
        self.pulos += 1;
        self.avancar(f)
    }

    pub fn pedir_saida(&mut self) {
        if self.estado == EstadoTut::Andando {
            self.estado = EstadoTut::ConfirmandoSaida;
        }
    }

    pub fn cancelar_saida(&mut self) {
        if self.estado == EstadoTut::ConfirmandoSaida {
            self.estado = EstadoTut::Andando;
        }
    }

    pub fn confirmar_saida(&mut self) {
        if self.estado == EstadoTut::ConfirmandoSaida {
            self.estado = EstadoTut::Saiu;
        }
    }

    /// A tecla passa? Modificadores sempre; o resto só se o passo liberar (e nunca durante a confirmação).
    pub fn tecla_permitida(&self, k: KeyCode) -> bool {
        if matches!(k, ShiftLeft | ShiftRight | ControlLeft | ControlRight | AltLeft | AltRight | SuperLeft | SuperRight) {
            return true;
        }
        self.estado == EstadoTut::Andando && self.passo_atual().teclas.contains(&k)
    }
}

/// O clique em `p` passa? Só dentro do buraco (se houver) ou da caixa do tutorial.
pub fn clique_permitido(buraco: Option<egui::Rect>, caixa: egui::Rect, p: egui::Pos2) -> bool {
    caixa.contains(p) || buraco.is_some_and(|b| b.contains(p))
}

// ---------------------------------------------------------------------------------------------
// Recurso e plugin
// ---------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct TutorialGuiado {
    /// Tutorial em andamento (partida especial).
    pub ativo: bool,
    /// Pedido do menu: configurar a partida ao entrar em Jogando.
    pub pendente: bool,
    maquina: Option<Maquina>,
    /// Passo cujas preparações (fechar painéis, centrar câmera) já foram aplicadas.
    aplicado: Option<usize>,
    obras: u32,
    salvos: u32,
}

impl TutorialGuiado {
    /// Chamado pelo botão do menu.
    pub fn iniciar(&mut self) {
        *self = Self { ativo: true, pendente: true, ..Default::default() };
    }
}

fn ativo(t: Res<TutorialGuiado>) -> bool {
    t.ativo
}

pub struct TutorialGuiadoPlugin;

impl Plugin for TutorialGuiadoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TutorialGuiado>()
            .add_systems(PreUpdate, bloquear_teclas.after(bevy::input::InputSystems).run_if(ativo))
            .add_systems(
                Update,
                (iniciar_partida, avancar_passos.before(super::construcao_ui::aplicar_pedidos))
                    .chain()
                    .run_if(ativo)
                    .run_if(in_state(AppState::Jogando)),
            )
            .add_systems(EguiPrimaryContextPass, desenhar.run_if(ativo).run_if(crate::estado::em_partida));
    }
}

#[derive(SystemParam)]
struct Paineis<'w> {
    pais: ResMut<'w, PainelPais>,
    mercado: ResMut<'w, PainelMercado>,
    comercio: ResMut<'w, PainelComercio>,
    constr: ResMut<'w, ConstrucaoUi>,
    diplo: Option<ResMut<'w, PainelDiplomacia>>,
    tec: Option<ResMut<'w, TecnologiaSim>>,
    eventos: Option<ResMut<'w, EstadoEv>>,
}

impl Paineis<'_> {
    fn fechar_todos(&mut self) {
        self.pais.aberto = false;
        self.mercado.aberto = false;
        self.comercio.aberto = false;
        self.constr.aberta = false;
        if let Some(d) = self.diplo.as_mut() {
            d.aberto = false;
        }
        if let Some(t) = self.tec.as_mut() {
            t.aberto = false;
        }
        if let Some(e) = self.eventos.as_mut() {
            e.jornal_aberto = false;
        }
    }
}

#[derive(SystemParam)]
struct Entradas<'w, 's> {
    camera: Query<'w, 's, (&'static Transform, &'static Projection), With<Camera2d>>,
    selecionada: Res<'w, ProvinciaSelecionada>,
    mundo: Res<'w, RecursoMundo>,
    relogio: Res<'w, RecursoRelogio>,
    modo: Option<Res<'w, super::hud_mapas::ModoTematico>>,
    pol: Option<Res<'w, PoliticaSim>>,
    diplo_sim: Option<Res<'w, DiploSim>>,
    salvar: MessageReader<'w, 's, PedirSalvar>,
}

fn coletar(e: &mut Entradas, p: &Paineis, t: &mut TutorialGuiado) -> Fatos {
    let camera = e.camera.single().map_or((0, 0, 0), |(tf, pr)| {
        let esc = if let Projection::Orthographic(o) = pr { o.scale } else { 1.0 };
        (tf.translation.x as i32 / 8, tf.translation.y as i32 / 8, (esc * 20.0) as i32)
    });
    t.salvos += e.salvar.read().count() as u32;
    Fatos {
        camera,
        prov_id: e.selecionada.0,
        prov_usa: e.selecionada.0 != 0 && e.mundo.dono_por_provincia.get(&e.selecionada.0).is_some_and(|d| d == "USA"),
        pausado: e.relogio.pausado,
        velocidade: e.relogio.velocidade.nivel(),
        modo_mapa: e.modo.as_ref().map(|m| format!("{:?}", **m)).unwrap_or_default(),
        construcao_aberta: p.constr.aberta,
        obras: t.obras,
        mercado_aberto: p.mercado.aberto,
        comercio_aberto: p.comercio.aberto,
        lei_em_tramitacao: e.pol.as_ref().and_then(|x| x.vista.pais.as_ref()).is_some_and(|x| x.lei_em_tramitacao.is_some()),
        relacao_gbr: e.diplo_sim.as_ref().map_or(0.0, |d| d.0.relacao("USA", "GBR")),
        pesquisando: p.tec.as_ref().is_some_and(|tec| {
            tec.tag_jogador.as_ref().and_then(|tag| tec.mundo.paises.get(tag)).is_some_and(|est| est.ativa.is_some() || !est.fila.is_empty())
        }),
        jornal_aberto: p.eventos.as_ref().is_some_and(|ev| ev.jornal_aberto),
        salvos: t.salvos,
    }
}

/// Primeiro quadro em Jogando: pausa o relógio e cria a máquina.
fn iniciar_partida(mut t: ResMut<TutorialGuiado>, mut relogio: ResMut<RecursoRelogio>, mut e: Entradas, mut p: Paineis) {
    if !t.pendente {
        return;
    }
    t.pendente = false;
    relogio.pausado = true;
    p.fechar_todos();
    let f = coletar(&mut e, &p, &mut t);
    t.maquina = Some(Maquina::nova(&f));
    t.aplicado = None;
}

fn avancar_passos(mut t: ResMut<TutorialGuiado>, mut e: Entradas, mut p: Paineis) {
    // Contabiliza pedidos de obra antes de `aplicar_pedidos` esvaziá-los.
    t.obras += p.constr.pedidos.len() as u32;
    let f = coletar(&mut e, &p, &mut t);
    let Some(mut m) = t.maquina.take() else { return };
    if m.atualizar(&f) {
        p.fechar_todos();
    }
    t.maquina = Some(m);
}

/// Bloqueia, antes de qualquer sistema do jogo, as teclas que o passo atual não libera.
fn bloquear_teclas(mut teclas: ResMut<ButtonInput<KeyCode>>, t: Res<TutorialGuiado>) {
    let Some(m) = &t.maquina else { return };
    let barradas: Vec<KeyCode> = teclas.get_pressed().chain(teclas.get_just_pressed()).copied().filter(|k| !m.tecla_permitida(*k)).collect();
    for k in barradas {
        teclas.reset(k);
    }
}

// ---------------------------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------------------------

fn uniao(a: Option<egui::Rect>, b: egui::Rect) -> egui::Rect {
    a.map_or(b, |a| a.union(b))
}

/// Retângulos das janelas abertas (áreas de ordem Middle, exceto elementos fixos do mapa).
fn janelas_abertas(ctx: &egui::Context) -> Vec<egui::Rect> {
    let ignorar = [egui::Id::new("seletor_modo_mapa"), egui::Id::new("legenda_mapa")];
    ctx.memory(|mem| {
        mem.layer_ids()
            .filter(|l| l.order == egui::Order::Middle && !ignorar.contains(&l.id))
            .filter_map(|l| mem.area_rect(l.id))
            .filter(|r| r.width() > 120.0 && r.height() > 60.0)
            .collect()
    })
}

fn posicao_provincia(estado: &str, cat: &RecursoCatalogo, mundo: &RecursoMundo, dados: &DadosMapa) -> Option<Vec2> {
    cat.0
        .provincias
        .iter()
        .filter(|p| p.estado == estado && mundo.dono_por_provincia.get(&p.id).is_some_and(|d| d == "USA"))
        .max_by(|a, b| a.area.partial_cmp(&b.area).unwrap_or(std::cmp::Ordering::Equal))
        .map(|p| Vec2::new(p.centroide_px.0 as f32 - dados.largura as f32 / 2.0, dados.altura as f32 / 2.0 - p.centroide_px.1 as f32))
}

#[allow(clippy::too_many_arguments)]
fn desenhar(
    mut contextos: EguiContexts,
    mut t: ResMut<TutorialGuiado>,
    mut e: Entradas,
    mut p: Paineis,
    cat: Option<Res<RecursoCatalogo>>,
    dados: Option<Res<DadosMapa>>,
    mut camera: Query<(&Camera, &GlobalTransform, &mut Transform, &mut Projection), With<Camera2d>>,
    mut prox: ResMut<NextState<AppState>>,
    mut menu: ResMut<super::menu_ui::TelaMenu>,
    jogador: Option<Res<PaisJogador>>,
    mut ligado_relogio: ResMut<RecursoRelogio>,
) -> Result {
    let ctx = contextos.ctx_mut()?.clone();
    if t.maquina.is_none() {
        return Ok(());
    }
    let f = coletar(&mut e, &p, &mut t);
    let tela = ctx.viewport_rect();
    let tempo = ctx.input(|i| i.time) as f32;
    ctx.request_repaint();
    let mut m = t.maquina.take().expect("máquina presente");

    // Preparação ao entrar no passo: centra a câmera na província-alvo.
    let passo = m.passo_atual();
    let pos_prov = if let (Alvo::Provincia(est), Some(cat), Some(dados), Some(j)) = (passo.alvo, cat.as_ref(), dados.as_ref(), jogador.as_ref()) {
        let _ = j;
        posicao_provincia(est, cat, &e.mundo, dados)
    } else {
        None
    };
    if t.aplicado != Some(m.passo) {
        t.aplicado = Some(m.passo);
        if let (Some(pp), Ok((_, _, mut tf, mut proj))) = (pos_prov, camera.single_mut()) {
            tf.translation.x = pp.x;
            tf.translation.y = pp.y;
            if let Projection::Orthographic(o) = &mut *proj {
                o.scale = o.scale.clamp(0.3, 1.2);
            }
        }
    }

    // Buraco no overlay.
    let mut buraco: Option<egui::Rect> = None;
    let mut circulo = false;
    match passo.alvo {
        Alvo::Mapa => {
            // Área do mapa: abaixo das barras do topo e à esquerda do painel da província.
            let topo = alvo_recente(&ctx, "hud").map_or(130.0, |h| h.max.y + 70.0);
            let direita = alvo_recente(&ctx, "painel_provincia").map_or(tela.max.x, |r| r.min.x);
            buraco = Some(egui::Rect::from_min_max(egui::pos2(tela.min.x, topo), egui::pos2(direita, tela.max.y)));
        }
        Alvo::Teclado => {}
        Alvo::Widget(n) => buraco = alvo_recente(&ctx, n),
        Alvo::WidgetEJanelas(n) => {
            buraco = alvo_recente(&ctx, n);
            for r in janelas_abertas(&ctx) {
                buraco = Some(uniao(buraco, r));
            }
        }
        Alvo::Provincia(_) => {
            if let (Some(pp), Ok((cam, gt, _, _))) = (pos_prov, camera.single())
                && let Ok(v) = cam.world_to_viewport(gt, pp.extend(0.0))
            {
                buraco = Some(egui::Rect::from_center_size(egui::pos2(v.x, v.y), egui::vec2(64.0, 64.0)));
                circulo = true;
            }
        }
    }
    let confirmando = m.estado == EstadoTut::ConfirmandoSaida;
    let buraco_efetivo = if confirmando { None } else { buraco.map(|b| b.expand(6.0).intersect(tela)) };

    // Overlay: quatro faixas em volta do buraco (ou a tela toda). Elas consomem os cliques.
    let escuro = egui::Color32::from_black_alpha(165);
    let faixas: Vec<egui::Rect> = match buraco_efetivo {
        None => vec![tela],
        Some(b) => vec![
            egui::Rect::from_min_max(tela.min, egui::pos2(tela.max.x, b.min.y)),
            egui::Rect::from_min_max(egui::pos2(tela.min.x, b.max.y), tela.max),
            egui::Rect::from_min_max(egui::pos2(tela.min.x, b.min.y), egui::pos2(b.min.x, b.max.y)),
            egui::Rect::from_min_max(egui::pos2(b.max.x, b.min.y), egui::pos2(tela.max.x, b.max.y)),
        ],
    };
    for (i, r) in faixas.iter().enumerate().filter(|(_, r)| r.width() > 0.5 && r.height() > 0.5) {
        egui::Area::new(egui::Id::new(("tut_faixa", i))).order(egui::Order::Foreground).fixed_pos(r.min).interactable(true).show(&ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(r.size(), egui::Sense::click_and_drag());
            ui.painter().rect_filled(rect, 0.0, escuro);
        });
    }

    // Indicação visual: retângulo/círculo pulsante e seta.
    if let Some(b) = buraco_efetivo {
        let pulso = 0.5 + 0.5 * (tempo * 4.0).sin();
        let pintor = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("tut_indicacao")));
        let cor = egui::Color32::from_rgba_unmultiplied(0xE0, 0x3A, 0x2E, (150.0 + 105.0 * pulso) as u8);
        let r = b.expand(2.0 + 4.0 * pulso);
        if circulo {
            pintor.circle_stroke(r.center(), r.width() / 2.0, egui::Stroke::new(3.5, cor));
        } else if !matches!(passo.alvo, Alvo::Mapa) {
            pintor.rect_stroke(r, 4.0, egui::Stroke::new(3.5, cor), egui::StrokeKind::Outside);
        }
        if !matches!(passo.alvo, Alvo::Mapa) {
            let salto = 6.0 * (tempo * 6.0).sin().abs();
            let (ponta, dir) = if b.min.y - tela.min.y > 90.0 {
                (egui::pos2(b.center().x, b.min.y - 6.0 - salto), egui::vec2(0.0, 1.0))
            } else if tela.max.y - b.max.y > 90.0 {
                (egui::pos2(b.center().x, b.max.y + 6.0 + salto), egui::vec2(0.0, -1.0))
            } else {
                (egui::pos2(b.min.x - 6.0 - salto, b.center().y), egui::vec2(1.0, 0.0))
            };
            let lado = egui::vec2(-dir.y, dir.x);
            let base = ponta - dir * 26.0;
            pintor.add(egui::Shape::convex_polygon(vec![ponta, base + lado * 14.0, base - lado * 14.0], cor, egui::Stroke::new(1.5, TINTA)));
            pintor.line_segment([base, base - dir * 22.0], egui::Stroke::new(5.0, cor));
        }
    }

    // Caixa do tutorial (sempre interativa), fora do buraco.
    let larg = 390.0;
    let alt = 230.0;
    let cands = [
        egui::pos2(tela.min.x + 16.0, tela.max.y - alt - 16.0),
        egui::pos2(tela.max.x - larg - 16.0, tela.max.y - alt - 16.0),
        egui::pos2(tela.min.x + 16.0, tela.min.y + 120.0),
        egui::pos2(tela.max.x - larg - 16.0, tela.min.y + 120.0),
    ];
    let livre = |pos: &egui::Pos2| buraco_efetivo.is_none_or(|b| !b.expand(30.0).intersects(egui::Rect::from_min_size(*pos, egui::vec2(larg, alt))));
    let pos_caixa = if confirmando { egui::pos2(tela.center().x - larg / 2.0, tela.center().y - 80.0) } else { cands.iter().copied().find(livre).unwrap_or(cands[0]) };

    let moldura = egui::Frame::NONE.fill(PAPEL).stroke(egui::Stroke::new(2.5, VERMELHO)).inner_margin(14.0);
    let mut acao_proximo = false;
    let mut acao_pular = false;
    let mut acao_sair = false;
    let mut acao_confirmar = false;
    let mut acao_cancelar = false;
    let mut acao_livre = false;
    let mut acao_menu = false;
    egui::Area::new(egui::Id::new("tut_caixa")).order(egui::Order::Tooltip).fixed_pos(pos_caixa).interactable(true).show(&ctx, |ui| {
        moldura.show(ui, |ui| {
            ui.set_width(larg);
            ui.style_mut().visuals.override_text_color = Some(TINTA);
            if confirmando {
                ui.label(egui::RichText::new("Sair do tutorial?").size(20.0).strong());
                ui.label("Você volta ao menu principal e perde o progresso desta partida de treino.");
                ui.horizontal(|ui| {
                    if ui.button("Sim, sair").clicked() {
                        acao_confirmar = true;
                    }
                    if ui.button("Continuar tutorial").clicked() {
                        acao_cancelar = true;
                    }
                });
                return;
            }
            let passo = m.passo_atual();
            ui.label(egui::RichText::new(format!("Tutorial {}/{}", m.passo + 1, PASSOS.len())).small().color(VERMELHO));
            ui.label(egui::RichText::new(passo.titulo).size(20.0).strong());
            ui.add_space(4.0);
            ui.label(egui::RichText::new(passo.texto).size(15.0));
            ui.add_space(6.0);
            ui.label(egui::RichText::new(format!("Objetivo: {}", passo.objetivo)).strong().color(VERMELHO));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if m.estado == EstadoTut::Concluido || (m.ultimo() && matches!(passo.cond, Cond::Manual)) {
                    if ui.button("Continuar jogando com os EUA").clicked() {
                        acao_livre = true;
                    }
                    if ui.button("Voltar ao menu").clicked() {
                        acao_menu = true;
                    }
                    return;
                }
                if matches!(passo.cond, Cond::Manual) && ui.button("Próximo").clicked() {
                    acao_proximo = true;
                }
                if ui.add(egui::Button::new(egui::RichText::new("Pular passo").small()).frame(false)).clicked() {
                    acao_pular = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Sair do tutorial").clicked() {
                        acao_sair = true;
                    }
                });
            });
        });
    });

    let mut mudou = false;
    if acao_proximo {
        mudou = m.proximo(&f);
    }
    if acao_pular {
        mudou = m.pular(&f);
    }
    if acao_sair {
        m.pedir_saida();
    }
    if acao_cancelar {
        m.cancelar_saida();
    }
    if acao_confirmar {
        m.confirmar_saida();
    }
    if mudou {
        p.fechar_todos();
    }
    if acao_livre {
        t.ativo = false;
        ligado_relogio.pausado = true;
    } else if acao_menu || m.estado == EstadoTut::Saiu {
        t.ativo = false;
        *menu = super::menu_ui::TelaMenu::Titulo;
        prox.set(AppState::Menu);
    }
    t.maquina = Some(m);
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    fn base() -> Fatos {
        Fatos { pausado: true, velocidade: 2, ..Default::default() }
    }

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> egui::Rect {
        egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1))
    }

    #[test]
    fn passos_entre_12_e_18() {
        assert!((12..=18).contains(&PASSOS.len()));
        assert!(matches!(PASSOS[PASSOS.len() - 1].cond, Cond::Manual));
    }

    #[test]
    fn avanca_so_quando_a_condicao_vale() {
        let mut f = base();
        let mut m = Maquina::nova(&f);
        assert!(!m.atualizar(&f), "sem mudança não avança");
        f.camera = (5, 0, 20);
        assert!(m.atualizar(&f));
        assert_eq!(m.passo, 1);
        // passo 2 (seleção) exige província dos EUA
        f.prov_id = 7;
        assert!(!m.atualizar(&f));
        f.prov_usa = true;
        assert!(m.atualizar(&f));
        assert_eq!(m.passo, 2);
        // passo manual não avança sozinho, só com "Próximo"
        assert!(!m.atualizar(&f));
        assert!(m.proximo(&f));
        assert_eq!(m.passo, 3);
        // "Próximo" não vale em passo automático
        assert!(!m.proximo(&f));
    }

    #[test]
    fn condicoes_comparam_com_a_base_do_passo() {
        let mut f = base();
        let mut m = Maquina::nova(&f);
        m.passo = 4; // velocidade
        m.base = f.clone();
        assert!(!m.atualizar(&f));
        f.velocidade = 4;
        assert!(m.atualizar(&f));
        m.passo = 8; // construir
        m.base = f.clone();
        f.obras += 1;
        assert!(m.atualizar(&f));
    }

    #[test]
    fn bloqueio_de_teclas_fora_do_passo() {
        let f = base();
        let mut m = Maquina::nova(&f);
        assert!(m.tecla_permitida(KeyW));
        assert!(!m.tecla_permitida(KeyB));
        assert!(!m.tecla_permitida(Digit2));
        assert!(!m.tecla_permitida(Escape));
        assert!(m.tecla_permitida(ShiftLeft));
        m.passo = 9; // abrir construção
        assert!(m.tecla_permitida(KeyB));
        assert!(!m.tecla_permitida(KeyW));
        m.pedir_saida();
        assert!(!m.tecla_permitida(KeyB), "durante a confirmação só modificadores");
    }

    #[test]
    fn cliques_so_no_buraco_ou_na_caixa() {
        let buraco = Some(rect(100.0, 100.0, 200.0, 150.0));
        let caixa = rect(0.0, 500.0, 300.0, 700.0);
        assert!(clique_permitido(buraco, caixa, egui::pos2(150.0, 120.0)));
        assert!(clique_permitido(buraco, caixa, egui::pos2(10.0, 600.0)));
        assert!(!clique_permitido(buraco, caixa, egui::pos2(400.0, 300.0)));
        assert!(!clique_permitido(None, caixa, egui::pos2(150.0, 120.0)));
    }

    #[test]
    fn pular_avanca_sem_condicao_e_conta() {
        let f = base();
        let mut m = Maquina::nova(&f);
        assert!(m.pular(&f));
        assert_eq!((m.passo, m.pulos), (1, 1));
        for _ in 0..PASSOS.len() * 2 {
            m.pular(&f);
        }
        assert_eq!(m.estado, EstadoTut::Concluido);
        assert_eq!(m.passo, PASSOS.len() - 1);
    }

    #[test]
    fn sair_exige_confirmacao() {
        let f = base();
        let mut m = Maquina::nova(&f);
        m.confirmar_saida();
        assert_eq!(m.estado, EstadoTut::Andando, "sem pedir não sai");
        m.pedir_saida();
        assert_eq!(m.estado, EstadoTut::ConfirmandoSaida);
        let mut g = f.clone();
        g.camera = (9, 9, 9);
        assert!(!m.atualizar(&g), "congelado enquanto confirma");
        m.cancelar_saida();
        assert_eq!(m.estado, EstadoTut::Andando);
        m.pedir_saida();
        m.confirmar_saida();
        assert_eq!(m.estado, EstadoTut::Saiu);
    }

    #[test]
    fn conclusao_ao_final() {
        let f = base();
        let mut m = Maquina::nova(&f);
        m.passo = PASSOS.len() - 1;
        assert!(m.ultimo());
        assert!(!m.proximo(&f));
        assert_eq!(m.estado, EstadoTut::Concluido);
    }
}
