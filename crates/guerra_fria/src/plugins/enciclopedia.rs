//! Enciclopédia, Sistema de Ajuda, Tooltips Aninhados e Mensagens (Hegemony: 1945).
//!
//! Implementa:
//! 1. Tooltips aninhados estilo Victoria 3 para métricas e HUD.
//! 2. Tela Enciclopédia (tecla 8) com verbetes, busca, filtros de categoria e hiperlinks.
//! 3. Tutorial guiado de 25 passos (tutorial.ron) integrado com janela interativa.
//! 4. Sistema de modelos de notificação (mensagens.ron) com severidade e interpolação.

use super::construcao_ui::ConstrucaoUi;
use super::jogador::PaisJogador;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

pub const RON_TUTORIAL: &str = include_str!("../../../../assets/data/enciclopedia/tutorial.ron");
pub const RON_CONCEITOS: &str = include_str!("../../../../assets/data/enciclopedia/conceitos.ron");
pub const RON_TOOLTIPS: &str = include_str!("../../../../assets/data/enciclopedia/tooltips.ron");
pub const RON_MENSAGENS: &str = include_str!("../../../../assets/data/enciclopedia/mensagens.ron");

// ==============================================================================
// ESTRUTURAS DE DADOS
// ==============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PassoTutorial {
    pub id: String,
    pub ordem: u32,
    pub titulo: String,
    pub categoria: String,
    pub pais_foco: Option<String>,
    pub texto: String,
    pub objetivo: String,
    pub elemento_hud: String,
    pub dica: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Conceito {
    pub id: String,
    pub nome: String,
    pub categoria: String,
    pub definicao_mecanica: String,
    pub contexto_historico: String,
    pub conceitos_relacionados: Vec<String>,
    pub dica_estrategica: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename = "Tooltip")]
pub struct TooltipDef {
    pub id: String,
    pub alvo_ui: String,
    pub titulo: String,
    pub descricao: String,
    pub formula_ou_origem: String,
    pub efeito_ou_acao: String,
    pub atalho_teclado: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModeloMensagem {
    pub id: String,
    pub categoria: String,
    pub tipo_alerta: String,
    pub titulo_template: String,
    pub mensagem_template: String,
    pub placeholders: Vec<String>,
    pub som_efeito: String,
}


// ==============================================================================
// DADOS CARREGADOS (globais, somente leitura)
// ==============================================================================

pub struct Dados {
    pub tutorial: Vec<PassoTutorial>,
    pub conceitos: Vec<Conceito>,
    pub tooltips: HashMap<String, TooltipDef>,
    pub mensagens: HashMap<String, ModeloMensagem>,
    idx_conceito: HashMap<String, usize>,
}

impl Dados {
    fn carregar() -> Self {
        let tutorial: Vec<PassoTutorial> = ron::from_str(RON_TUTORIAL).unwrap_or_else(|e| {
            error!("enciclopedia: tutorial.ron: {e}");
            Vec::new()
        });
        let conceitos: Vec<Conceito> = ron::from_str(RON_CONCEITOS).unwrap_or_else(|e| {
            error!("enciclopedia: conceitos.ron: {e}");
            Vec::new()
        });
        let tooltips: Vec<TooltipDef> = ron::from_str(RON_TOOLTIPS).unwrap_or_else(|e| {
            error!("enciclopedia: tooltips.ron: {e}");
            Vec::new()
        });
        let mensagens: Vec<ModeloMensagem> = ron::from_str(RON_MENSAGENS).unwrap_or_else(|e| {
            error!("enciclopedia: mensagens.ron: {e}");
            Vec::new()
        });
        let idx_conceito = conceitos.iter().enumerate().map(|(i, c)| (c.id.clone(), i)).collect();
        Self {
            tutorial,
            conceitos,
            tooltips: tooltips.into_iter().map(|t| (t.id.clone(), t)).collect(),
            mensagens: mensagens.into_iter().map(|m| (m.id.clone(), m)).collect(),
            idx_conceito,
        }
    }

    pub fn conceito(&self, id: &str) -> Option<&Conceito> {
        self.idx_conceito.get(id).map(|&i| &self.conceitos[i])
    }
}

/// Dados globais (carregados na primeira chamada).
pub fn dados() -> &'static Dados {
    static D: OnceLock<Dados> = OnceLock::new();
    D.get_or_init(Dados::carregar)
}

pub fn nome_categoria(cat: &str) -> &str {
    match cat {
        "demografia_e_sociedade" => "Demografia e sociedade",
        "economia_e_mercado" => "Economia e mercado",
        "producao_e_infraestrutura" => "Produção e infraestrutura",
        "financas_publicas" => "Finanças públicas",
        "politica_e_governo" => "Política e governo",
        "geopolitica_e_diplomacia" => "Geopolítica e diplomacia",
        "contexto_historico_1945" => "Contexto histórico",
        outro => outro,
    }
}

// ==============================================================================
// MENSAGENS (modelos com {placeholders})
// ==============================================================================

/// Substitui `{chave}` pelos valores dados; chaves sem valor ficam como estão.
pub fn interpolar(modelo: &str, valores: &[(&str, String)]) -> String {
    let mut s = modelo.to_string();
    for (k, v) in valores {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// Gera (título, mensagem, tipo_alerta) a partir de um modelo de `mensagens.ron`.
pub fn formatar_mensagem(id: &str, valores: &[(&str, String)]) -> Option<(String, String, String)> {
    let m = dados().mensagens.get(id)?;
    Some((interpolar(&m.titulo_template, valores), interpolar(&m.mensagem_template, valores), m.tipo_alerta.clone()))
}

/// Texto de notificação pronto ("Título: mensagem"), ou `reserva` se o modelo não existir.
pub fn texto_notificacao(id: &str, valores: &[(&str, String)], reserva: &str) -> String {
    match formatar_mensagem(id, valores) {
        Some((t, m, _)) => format!("{t}: {m}"),
        None => reserva.to_string(),
    }
}

// ==============================================================================
// TOOLTIPS ANINHADOS
// ==============================================================================

/// Conceitos exibidos como 2º nível dentro de cada tooltip.
fn conceitos_do_tooltip(id: &str) -> &'static [&'static str] {
    match id {
        "tooltip_topbar_tesouro" => &["tesouro_nacional", "balanco_fiscal", "divida_publica"],
        "tooltip_topbar_balanco_diario" => &["balanco_fiscal", "imposto_de_renda", "tarifas_alfandegarias"],
        "tooltip_topbar_padrao_ouro" => &["padrao_bretton_woods", "banco_mundial_e_fmi"],
        "tooltip_topbar_inflacao" => &["inflacao", "preco_de_mercado"],
        "tooltip_topbar_legitimidade" => &["legitimidade_governamental", "grupo_de_interesse"],
        "tooltip_topbar_estabilidade" => &["estabilidade_nacional", "radicalismo_e_lealdade"],
        "tooltip_topbar_prestigio" => &["esfera_de_influencia", "bipolaridade_global"],
        "tooltip_topbar_doomsday_clock" => &["relogio_do_juizo_final", "monopolio_nuclear"],
        "tooltip_mapmode_esferas" => &["esfera_de_influencia", "cortina_de_ferro"],
        "tooltip_mapmode_economico" => &["mercado_nacional", "acesso_ao_mercado"],
        "tooltip_orcamento_aliquota_renda" => &["imposto_de_renda", "balanco_fiscal"],
        "tooltip_orcamento_tarifas" => &["tarifas_alfandegarias", "substituicao_de_importacoes"],
        "tooltip_mercado_balanco_bem" => &["oferta_e_demanda", "preco_de_mercado"],
        "tooltip_construcao_pontos_ativos" => &["setor_de_construcao", "fila_de_construcao"],
        "tooltip_politica_progresso_lei" => &["tramitacao_de_leis", "forca_politica"],
        "tooltip_provincia_acesso_mercado" => &["acesso_ao_mercado", "infraestrutura_de_transporte"],
        _ => &[],
    }
}

/// Estado global simples do tooltip: pedido de abrir um verbete ao clicar.
static PEDIDO_VERBETE: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn primeira_frase(s: &str) -> &str {
    match s.find(". ") {
        Some(i) => &s[..=i],
        None => s,
    }
}

/// Tooltip de um verbete: definição, com os relacionados como 2º nível (Ctrl expande).
fn corpo_conceito(ui: &mut egui::Ui, id: &str, nivel: u8) {
    let d = dados();
    let Some(c) = d.conceito(id) else { return };
    ui.strong(&c.nome);
    if nivel <= 1 {
        ui.label(primeira_frase(&c.definicao_mecanica));
        return;
    }
    ui.label(&c.definicao_mecanica);
    ui.add_space(2.0);
    ui.label(egui::RichText::new(&c.dica_estrategica).italics().weak());
    if !c.conceitos_relacionados.is_empty() {
        let nomes: Vec<&str> = c.conceitos_relacionados.iter().filter_map(|r| d.conceito(r).map(|x| x.nome.as_str())).collect();
        ui.label(egui::RichText::new(format!("Ver também: {}", nomes.join(", "))).small().weak());
    }
}

/// Anexa um tooltip aninhado (id de `tooltips.ron`) a uma resposta da UI.
/// Nível 1: título, descrição, cálculo e efeito. Nível 2: conceitos ligados (1ª frase).
/// Segurar Ctrl abre o nível 3 (definição completa e dica). Clique do meio abre o verbete.
pub fn dica(r: egui::Response, tooltip_id: &str) -> egui::Response {
    dica_com(r, tooltip_id, |_| {})
}

/// Como [`dica`], com um detalhamento dinâmico (números do cálculo) no topo.
pub fn dica_com(r: egui::Response, tooltip_id: &str, detalhe: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    if !super::config::tooltips_ativos() {
        return r;
    }
    let d = dados();
    let Some(t) = d.tooltips.get(tooltip_id) else { return r };
    let ligados = conceitos_do_tooltip(tooltip_id);
    if r.middle_clicked() && let Some(c) = ligados.first() {
        *PEDIDO_VERBETE.lock().unwrap() = Some((*c).to_string());
    }
    r.on_hover_ui(|ui| {
        ui.set_max_width(380.0);
        let ctrl = ui.input(|i| i.modifiers.ctrl);
        ui.strong(&t.titulo);
        if let Some(a) = &t.atalho_teclado {
            ui.label(egui::RichText::new(format!("[{a}]")).small().weak());
        }
        ui.label(&t.descricao);
        detalhe(ui);
        ui.separator();
        ui.label(egui::RichText::new("Cálculo / origem").small().strong());
        ui.label(egui::RichText::new(&t.formula_ou_origem).small());
        ui.label(egui::RichText::new("Efeito / ação").small().strong());
        ui.label(egui::RichText::new(&t.efeito_ou_acao).small());
        if !ligados.is_empty() {
            ui.separator();
            for id in ligados {
                egui::Frame::group(ui.style()).show(ui, |ui| corpo_conceito(ui, id, if ctrl { 2 } else { 1 }));
            }
            let dica = if ctrl { "Clique do meio: abrir na enciclopédia (8)" } else { "Segure Ctrl para expandir | clique do meio: enciclopédia (8)" };
            ui.label(egui::RichText::new(dica).small().weak());
        }
    })
}

/// Tooltip direto de um verbete (para números sem tooltip próprio).
pub fn dica_conceito(r: egui::Response, conceito_id: &str) -> egui::Response {
    let d = dados();
    let Some(c) = d.conceito(conceito_id) else { return r };
    if r.middle_clicked() {
        *PEDIDO_VERBETE.lock().unwrap() = Some(conceito_id.to_string());
    }
    r.on_hover_ui(|ui| {
        ui.set_max_width(380.0);
        let ctrl = ui.input(|i| i.modifiers.ctrl);
        corpo_conceito(ui, conceito_id, if ctrl { 2 } else { 1 });
        ui.separator();
        ui.label(egui::RichText::new("Relacionados").small().strong());
        for rel in &c.conceitos_relacionados {
            if let Some(x) = d.conceito(rel) {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.strong(&x.nome);
                    ui.label(egui::RichText::new(primeira_frase(&x.definicao_mecanica)).small());
                });
            }
        }
        ui.label(egui::RichText::new("Segure Ctrl para expandir | clique do meio: enciclopédia (8)").small().weak());
    })
}

// ==============================================================================
// ESTADO DA UI: ENCICLOPÉDIA E TUTORIAL
// ==============================================================================

#[derive(Resource, Default)]
pub struct EnciclopediaUi {
    pub aberta: bool,
    pub busca: String,
    pub categoria: Option<String>,
    pub atual: Option<String>,
    pub historico: Vec<String>,
}

impl EnciclopediaUi {
    pub fn abrir_verbete(&mut self, id: &str) {
        if self.atual.as_deref() != Some(id) {
            if let Some(a) = self.atual.take() {
                self.historico.push(a);
            }
            self.atual = Some(id.to_string());
        }
        self.aberta = true;
    }
}

/// Verbetes que casam com a busca/categoria (ordem do arquivo).
pub fn filtrar_conceitos<'a>(d: &'a Dados, busca: &str, categoria: Option<&str>) -> Vec<&'a Conceito> {
    let b = busca.trim().to_lowercase();
    d.conceitos
        .iter()
        .filter(|c| categoria.is_none_or(|k| c.categoria == k))
        .filter(|c| b.is_empty() || c.nome.to_lowercase().contains(&b) || c.definicao_mecanica.to_lowercase().contains(&b) || c.contexto_historico.to_lowercase().contains(&b))
        .collect()
}

#[derive(Resource, Default)]
pub struct TutorialUi {
    /// Pergunta inicial já respondida (ou descartada).
    pub respondeu: bool,
    /// Passo atual (índice na lista filtrada); None = tutorial fechado.
    pub passo: Option<usize>,
}

/// Passos aplicáveis ao país do jogador (gerais + os do país em foco).
pub fn passos_para<'a>(d: &'a Dados, tag: &str) -> Vec<&'a PassoTutorial> {
    let mut v: Vec<&PassoTutorial> = d.tutorial.iter().filter(|p| p.pais_foco.as_deref().is_none_or(|f| f == tag)).collect();
    v.sort_by_key(|p| p.ordem);
    v
}

pub struct EnciclopediaPlugin;

impl Plugin for EnciclopediaPlugin {
    fn build(&self, app: &mut App) {
        let _ = dados();
        app.init_resource::<EnciclopediaUi>().init_resource::<TutorialUi>().add_systems(Update, atalho_f9.run_if(crate::estado::em_partida)).add_systems(
            EguiPrimaryContextPass,
            (janela_enciclopedia, janela_tutorial).run_if(crate::estado::em_partida),
        );
    }
}

fn atalho_f9(teclas: Res<ButtonInput<KeyCode>>, mut enc: ResMut<EnciclopediaUi>) {
    if teclas.just_pressed(KeyCode::Digit8) || teclas.just_pressed(KeyCode::Numpad8) {
        enc.aberta = !enc.aberta;
    }
}

fn janela_enciclopedia(mut contextos: EguiContexts, mut enc: ResMut<EnciclopediaUi>) -> Result {
    if let Some(id) = PEDIDO_VERBETE.lock().unwrap().take() {
        enc.abrir_verbete(&id);
    }
    if !enc.aberta {
        return Ok(());
    }
    let d = dados();
    let mut aberta = true;
    let mut ir: Option<String> = None;
    let mut voltar = false;
    let e = &mut *enc;
    egui::Window::new("Enciclopédia (8)").open(&mut aberta).default_size([920.0, 600.0]).show(contextos.ctx_mut()?, |ui| {
        ui.horizontal(|ui| {
            ui.label("Buscar:");
            ui.text_edit_singleline(&mut e.busca);
            if ui.button("Limpar").clicked() {
                e.busca.clear();
            }
            ui.separator();
            if ui.add_enabled(!e.historico.is_empty(), egui::Button::new("< Voltar")).clicked() {
                voltar = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.selectable_label(e.categoria.is_none(), "Todas").clicked() {
                e.categoria = None;
            }
            let mut cats: Vec<&str> = Vec::new();
            for c in &d.conceitos {
                if !cats.contains(&c.categoria.as_str()) {
                    cats.push(&c.categoria);
                }
            }
            for c in cats {
                if ui.selectable_label(e.categoria.as_deref() == Some(c), nome_categoria(c)).clicked() {
                    e.categoria = Some(c.to_string());
                }
            }
        });
        ui.separator();
        let lista = filtrar_conceitos(d, &e.busca, e.categoria.as_deref());
        ui.columns(2, |cols| {
            cols[0].label(format!("{} verbete(s)", lista.len()));
            egui::ScrollArea::vertical().id_salt("enc_lista").show(&mut cols[0], |ui| {
                let mut cat_ant = "";
                for c in &lista {
                    if c.categoria != cat_ant {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(nome_categoria(&c.categoria)).strong().weak());
                        cat_ant = &c.categoria;
                    }
                    if ui.selectable_label(e.atual.as_deref() == Some(c.id.as_str()), &c.nome).clicked() {
                        ir = Some(c.id.clone());
                    }
                }
            });
            egui::ScrollArea::vertical().id_salt("enc_detalhe").show(&mut cols[1], |ui| {
                let Some(c) = e.atual.as_deref().and_then(|id| d.conceito(id)) else {
                    ui.label("Escolha um verbete à esquerda. Passe o mouse sobre os números do jogo para ver explicações; clique do meio num número abre o verbete aqui.");
                    return;
                };
                ui.heading(&c.nome);
                ui.label(egui::RichText::new(nome_categoria(&c.categoria)).small().weak());
                ui.add_space(4.0);
                ui.strong("Mecânica");
                ui.label(&c.definicao_mecanica);
                ui.add_space(4.0);
                ui.strong("Contexto histórico");
                ui.label(&c.contexto_historico);
                ui.add_space(4.0);
                ui.strong("Dica estratégica");
                ui.label(&c.dica_estrategica);
                if !c.conceitos_relacionados.is_empty() {
                    ui.add_space(6.0);
                    ui.strong("Veja também");
                    ui.horizontal_wrapped(|ui| {
                        for rel in &c.conceitos_relacionados {
                            if let Some(x) = d.conceito(rel) {
                                let r = ui.link(&x.nome);
                                let r = r.on_hover_text(primeira_frase(&x.definicao_mecanica));
                                if r.clicked() {
                                    ir = Some(x.id.clone());
                                }
                            }
                        }
                    });
                }
            });
        });
    });
    if voltar {
        enc.atual = enc.historico.pop();
    }
    if let Some(id) = ir {
        enc.abrir_verbete(&id);
    }
    enc.aberta = aberta;
    Ok(())
}

fn janela_tutorial(
    mut contextos: EguiContexts,
    mut tut: ResMut<TutorialUi>,
    jogador: Option<Res<PaisJogador>>,
    constr: Option<Res<ConstrucaoUi>>,
    mut enc: ResMut<EnciclopediaUi>,
) -> Result {
    let Some(jogador) = jogador else { return Ok(()) };
    let d = dados();
    let ctx = contextos.ctx_mut()?;
    let _ = &constr;
    // Sem oferta automática: o tutorial guiado fica no menu principal; este abre só pela enciclopédia.
    tut.respondeu = true;
    let Some(i) = tut.passo else { return Ok(()) };
    let passos = passos_para(d, &jogador.0);
    let Some(p) = passos.get(i) else {
        tut.passo = None;
        return Ok(());
    };
    let mut novo = Some(i);
    egui::Window::new(format!("Tutorial {}/{}", i + 1, passos.len())).collapsible(false).resizable(false).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -60.0]).default_width(460.0).show(ctx, |ui| {
        ui.heading(&p.titulo);
        egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
            ui.label(&p.texto);
        });
        ui.add_space(4.0);
        ui.label(egui::RichText::new(format!("Objetivo: {}", p.objetivo)).strong());
        ui.label(egui::RichText::new(format!("Dica: {}", p.dica)).italics().weak());
        ui.separator();
        ui.horizontal(|ui| {
            if ui.add_enabled(i > 0, egui::Button::new("< Anterior")).clicked() {
                novo = Some(i - 1);
            }
            if i + 1 < passos.len() {
                if ui.button("Próximo >").clicked() {
                    novo = Some(i + 1);
                }
            } else if ui.button("Concluir").clicked() {
                novo = None;
            }
            if ui.button("Sair do tutorial").clicked() {
                novo = None;
            }
            if ui.button("Enciclopédia (8)").clicked() {
                enc.aberta = true;
            }
        });
    });
    tut.passo = novo;
    Ok(())
}

#[cfg(test)]
mod testes_dados {
    use super::*;

    #[test]
    fn parseia_tutorial_ron() {
        let passos: Vec<PassoTutorial> = ron::from_str(RON_TUTORIAL).expect("tutorial.ron deve ser válido");
        assert_eq!(passos.len(), 25);
        assert_eq!(passos[0].id, "passo_01_bem_vindo_a_1945");
        assert_eq!(passos[24].ordem, 25);
    }

    #[test]
    fn parseia_conceitos_ron() {
        let conceitos: Vec<Conceito> = ron::from_str(RON_CONCEITOS).expect("conceitos.ron deve ser válido");
        assert_eq!(conceitos.len(), 60);
        let ids: std::collections::HashSet<_> = conceitos.iter().map(|c| c.id.as_str()).collect();
        // Valida referências cruzadas
        for c in &conceitos {
            for rel in &c.conceitos_relacionados {
                assert!(ids.contains(rel.as_str()), "Conceito '{}' aponta para '{}' inexistente", c.id, rel);
            }
        }
    }

    #[test]
    fn parseia_tooltips_ron() {
        let tooltips: Vec<TooltipDef> = ron::from_str(RON_TOOLTIPS).expect("tooltips.ron deve ser válido");
        assert_eq!(tooltips.len(), 49);
    }

    #[test]
    fn interpola_mensagem() {
        let (t, m, a) = formatar_mensagem("msg_tesouro_insolvente", &[("pais", "Brasil".into()), ("taxa_juros", "9".into())]).unwrap();
        assert!(t.contains("Brasil") || m.contains("Brasil"));
        assert!(!t.contains("{pais}") && !m.contains("{pais}"));
        assert_eq!(a, "critico");
        assert_eq!(texto_notificacao("nao_existe", &[], "reserva"), "reserva");
    }

    #[test]
    fn placeholders_declarados_aparecem_no_modelo() {
        for m in dados().mensagens.values() {
            for p in &m.placeholders {
                let k = format!("{{{p}}}");
                assert!(m.titulo_template.contains(&k) || m.mensagem_template.contains(&k), "{}: {p}", m.id);
            }
        }
    }

    #[test]
    fn tooltips_apontam_para_conceitos_validos() {
        let d = dados();
        for (id, t) in &d.tooltips {
            for c in conceitos_do_tooltip(id) {
                assert!(d.conceito(c).is_some(), "{id} -> {c}");
            }
            assert!(!t.titulo.is_empty());
        }
    }

    #[test]
    fn busca_e_categoria() {
        let d = dados();
        assert_eq!(filtrar_conceitos(d, "", None).len(), 60);
        let inf = filtrar_conceitos(d, "inflação", None);
        assert!(inf.iter().any(|c| c.id == "inflacao"));
        let fin = filtrar_conceitos(d, "", Some("financas_publicas"));
        assert!(!fin.is_empty() && fin.iter().all(|c| c.categoria == "financas_publicas"));
    }

    #[test]
    fn tutorial_filtra_por_pais() {
        let d = dados();
        let bra = passos_para(d, "BRA");
        let usa = passos_para(d, "USA");
        assert!(bra.iter().all(|p| p.pais_foco.as_deref().is_none_or(|f| f == "BRA")));
        assert!(usa.iter().all(|p| p.pais_foco.as_deref().is_none_or(|f| f == "USA")));
        assert!(bra.len() >= 20 && bra.windows(2).all(|w| w[0].ordem < w[1].ordem));
    }

    #[test]
    fn parseia_mensagens_ron() {
        let msgs: Vec<ModeloMensagem> = ron::from_str(RON_MENSAGENS).expect("mensagens.ron deve ser válido");
        assert!(msgs.len() >= 30);
    }
}
