//! Tela do Mercado Global e Nacional (2) no estilo Victoria 3.
//!
//! Funcionalidades:
//! - Tabela de bens com colunas ordenáveis (Nome, Preço Nacional, Preço Mundial, Variação 30d, Oferta, Demanda e Saldo).
//! - Filtros rápidos por categoria econômica e busca textual por nome/id.
//! - Cores de alta e baixa para variações e desvios de preço base.
//! - Sparkline gráfica de 30 dias desenhada em cada linha da tabela com detecção de tendência e min/max.
//! - Painel de detalhes ao clicar num bem:
//!   - Diagnóstico econômico completo explicando causas da movimentação de preço (escassez, excesso, importação).
//!   - Gráfico de preço histórico (mundial x nacional x base) com seleção de escala (30 dias, 1 ano, 3 anos, 5 anos, todo histórico).
//!   - Maiores produtores e consumidores mundiais com barras de participação de mercado.
//!   - Cadeia produtiva: prédios que produzem e consomem, métodos de produção e contagem de níveis no país do jogador.
//! - Suporte a ícones PNG (`assets/icones/`) com cache no egui e fallbacks inteligentes.
//! - Tooltips ricos em pt-BR com diagnóstico de mercado e recomendações práticas.

use super::economia_sim::{EconomiaSim, VistaBem, VistaEconomia};
use super::jogador::PaisJogador;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use egui_plot::{Legend, Line, LineStyle, Plot, PlotPoints};
use sim_economia::CategoriaBem;
use std::collections::HashMap;
use std::path::PathBuf;

// ---------------------------------------------------------------------------------------------
// Cache e Exibição de Ícones
// ---------------------------------------------------------------------------------------------

/// Cache de texturas dos ícones (`assets/icones/<id>_32.png` ou `<id>_64.png`).
#[derive(Resource, Default)]
pub struct Icones {
    pub dir: PathBuf,
    cache: HashMap<String, Option<egui::TextureHandle>>,
}

impl Icones {
    /// Carrega e exibe o ícone correspondente ao ID (bem, prédio ou tipo de pop).
    /// Tenta `{id}_32.png`, depois `{id}_64.png`, e recorre a aliases conhecidos quando necessário.
    pub fn mostrar(&mut self, ui: &mut egui::Ui, id: &str, tam: f32) -> egui::Response {
        let ctx = ui.ctx().clone();
        let dir = if self.dir.as_os_str().is_empty() || !self.dir.exists() {
            PathBuf::from("assets/icones")
        } else {
            self.dir.clone()
        };
        let tex = self.cache.entry(id.to_string()).or_insert_with(|| {
            let cam32 = dir.join(format!("{id}_32.png"));
            let cam = if cam32.exists() {
                cam32
            } else {
                let cam64 = dir.join(format!("{id}_64.png"));
                if cam64.exists() {
                    cam64
                } else {
                    let alias = match id {
                        "setor_construcao" => "construcao",
                        "pop_comerciantes" => "pop_capitalistas",
                        _ => id,
                    };
                    let ca32 = dir.join(format!("{alias}_32.png"));
                    if ca32.exists() {
                        ca32
                    } else {
                        dir.join(format!("{alias}_64.png"))
                    }
                }
            };
            let img = image::open(cam).ok()?.to_rgba8();
            let (w, h) = img.dimensions();
            let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
            Some(ctx.load_texture(format!("icone_{id}"), ci, egui::TextureOptions::LINEAR))
        });
        match tex {
            Some(t) => ui.add(egui::Image::new(egui::load::SizedTexture::new(t.id(), egui::vec2(tam, tam)))),
            None => {
                let resp = ui.allocate_response(egui::vec2(tam, tam), egui::Sense::hover());
                ui_refinada::theme::icone::desenhar(
                    ui.painter(),
                    resp.rect,
                    ui_refinada::chave_para(id),
                    ui_refinada::menu::TINTA,
                    ui_refinada::menu::VERMELHO,
                );
                resp
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Filtros, Ordenação e Estado do Painel
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Filtro {
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
    pub const TODOS: [Filtro; 9] = [
        Self::Todos,
        Self::Alimentos,
        Self::Materias,
        Self::Energia,
        Self::Industrial,
        Self::Consumo,
        Self::Luxo,
        Self::Militar,
        Self::Servicos,
    ];

    pub fn nome(self) -> &'static str {
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

    pub fn icone(self) -> &'static str {
        match self {
            Self::Todos => "caixa",
            Self::Alimentos => "trigo",
            Self::Materias => "picareta",
            Self::Energia => "raio",
            Self::Industrial => "fabrica",
            Self::Consumo => "caixa",
            Self::Luxo => "diamante",
            Self::Militar => "canhao",
            Self::Servicos => "engrenagem",
        }
    }

    pub fn aceita(self, c: CategoriaBem) -> bool {
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

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ColunaOrdenacao {
    Nome,
    PrecoNacional,
    PrecoMundial,
    #[default]
    Variacao,
    Oferta,
    Demanda,
    Saldo,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SentidoOrdenacao {
    #[default]
    Descendente,
    Ascendente,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum EscalaTempoGrafico {
    Dias30,
    #[default]
    Ano1,
    Anos3,
    Anos5,
    Tudo,
}

impl EscalaTempoGrafico {
    pub fn dias(self) -> usize {
        match self {
            Self::Dias30 => 30,
            Self::Ano1 => 365,
            Self::Anos3 => 1095,
            Self::Anos5 => 1825,
            Self::Tudo => usize::MAX,
        }
    }

    pub fn rotulo(self) -> &'static str {
        match self {
            Self::Dias30 => "30 dias",
            Self::Ano1 => "1 ano",
            Self::Anos3 => "3 anos",
            Self::Anos5 => "5 anos",
            Self::Tudo => "Todo histórico",
        }
    }
}

#[derive(Resource)]
pub struct PainelMercado {
    pub aberto: bool,
    pub filtro: Filtro,
    pub busca: String,
    pub coluna_ordem: ColunaOrdenacao,
    pub sentido_ordem: SentidoOrdenacao,
    pub escala_grafico: EscalaTempoGrafico,
    pub selecionado: Option<String>,
}

impl Default for PainelMercado {
    fn default() -> Self {
        Self {
            aberto: false,
            filtro: Filtro::Todos,
            busca: String::new(),
            coluna_ordem: ColunaOrdenacao::Variacao,
            sentido_ordem: SentidoOrdenacao::Descendente,
            escala_grafico: EscalaTempoGrafico::Ano1,
            selecionado: None,
        }
    }
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
                tela_mercado
                    .run_if(resource_exists::<PaisJogador>)
                    .run_if(resource_exists::<EconomiaSim>)
                    .run_if(crate::estado::em_partida),
            );
    }
}

fn alternar_mercado(teclas: Res<ButtonInput<KeyCode>>, mut p: ResMut<PainelMercado>) {
    if teclas.just_pressed(KeyCode::Digit2) || teclas.just_pressed(KeyCode::Numpad2) {
        p.aberto = !p.aberto;
    }
}

// ---------------------------------------------------------------------------------------------
// Cores e Formatações
// ---------------------------------------------------------------------------------------------

/// Cor de variação percentual (alta = verde, baixa = vermelho, estável = cinza).
pub fn cor_variacao(v: f64) -> egui::Color32 {
    if v > 0.005 {
        ui_refinada::menu::SUCESSO
    } else if v < -0.005 {
        ui_refinada::menu::PERIGO
    } else {
        ui_refinada::menu::TINTA_SUAVE
    }
}

/// Cor do preço de mercado em relação ao preço base.
pub fn cor_preco_relativo(preco: f64, preco_base: f64) -> egui::Color32 {
    if preco_base <= 0.0 {
        return ui_refinada::menu::TINTA;
    }
    let r = preco / preco_base;
    if r > 1.25 {
        ui_refinada::menu::PERIGO // Vermelho: muito caro / escassez severa
    } else if r > 1.05 {
        ui_refinada::menu::AVISO // Dourado/laranja: caro / escassez
    } else if r < 0.75 {
        ui_refinada::menu::AZUL_MARINHO // Azul: muito barato / excesso severo
    } else if r < 0.95 {
        ui_refinada::menu::SUCESSO // Verde escuro: barato / excesso
    } else {
        ui_refinada::menu::TINTA // Equilibrado
    }
}

pub fn fmt_var(v: f64) -> String {
    if v.abs() < 0.0005 { "plano".to_string() } else { format!("{:+.1}%", v * 100.0) }
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

pub fn nome_categoria(c: CategoriaBem) -> &'static str {
    match c {
        CategoriaBem::AlimentoBasico => "Alimento Básico",
        CategoriaBem::MateriaPrima => "Matéria-Prima",
        CategoriaBem::Energia => "Energia",
        CategoriaBem::Industrial => "Bem Industrial",
        CategoriaBem::Consumo => "Bem de Consumo",
        CategoriaBem::Luxo => "Artigo de Luxo",
        CategoriaBem::Militar => "Bem Militar",
        CategoriaBem::LocalNaoComercializavel => "Serviço Local",
    }
}

// ---------------------------------------------------------------------------------------------
// Sparkline de 30 Dias
// ---------------------------------------------------------------------------------------------

/// Desenha uma sparkline suave dos últimos `dias` pontos de uma série histórica.
pub fn desenhar_sparkline(
    ui: &mut egui::Ui,
    serie: &[f32],
    dias: usize,
    largura: f32,
    altura: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(largura, altura), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let n = serie.len();
    let inicio = n.saturating_sub(dias);
    let slice = &serie[inicio..];

    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, ui_refinada::menu::PAPEL_ALT);

    if slice.len() >= 2 {
        let mut min_val = f32::INFINITY;
        let mut max_val = f32::NEG_INFINITY;
        for &v in slice {
            if v < min_val {
                min_val = v;
            }
            if v > max_val {
                max_val = v;
            }
        }
        let diff = (max_val - min_val).max(0.001 * min_val.abs().max(1.0));

        let primeiro = slice[0];
        let ultimo = slice[slice.len() - 1];
        let cor = if ultimo > primeiro * 1.005 {
            ui_refinada::menu::SUCESSO // Alta: verde escuro
        } else if ultimo < primeiro * 0.995 {
            ui_refinada::menu::PERIGO // Baixa: vermelho escuro
        } else {
            ui_refinada::menu::TINTA_SUAVE // Neutro: tinta suave
        };

        let pad_y = 2.0;
        let alt_util = (rect.height() - pad_y * 2.0).max(1.0);
        let larg_util = rect.width();

        let pts: Vec<egui::Pos2> = slice
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let frac_x = i as f32 / (slice.len() - 1) as f32;
                let frac_y = (v - min_val) / diff;
                let x = rect.left() + frac_x * larg_util;
                let y = rect.bottom() - pad_y - frac_y * alt_util;
                egui::pos2(x, y)
            })
            .collect();

        painter.add(egui::Shape::line(pts.clone(), egui::Stroke::new(1.5, cor)));
        if let Some(&fim) = pts.last() {
            painter.circle_filled(fim, 2.0, cor);
        }
    }

    response
}

// ---------------------------------------------------------------------------------------------
// Diagnóstico Econômico e Tooltips (Escassez, Excesso, Importação)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoMercado {
    EscassezCritica,
    EscassezModerada,
    Equilibrado,
    ExcessoModerado,
    ExcessoSevero,
}

impl EstadoMercado {
    pub fn cor(self) -> egui::Color32 {
        match self {
            Self::EscassezCritica => ui_refinada::menu::PERIGO,
            Self::EscassezModerada => ui_refinada::menu::AVISO,
            Self::Equilibrado => ui_refinada::menu::SUCESSO,
            Self::ExcessoModerado => ui_refinada::menu::INFO,
            Self::ExcessoSevero => ui_refinada::menu::AZUL_MARINHO,
        }
    }

    /// Rotulo curto do estado, para selos e abas.
    pub fn rotulo(self) -> &'static str {
        match self {
            Self::EscassezCritica => "Escassez crítica",
            Self::EscassezModerada => "Escassez moderada",
            Self::Equilibrado => "Equilibrado",
            Self::ExcessoModerado => "Excesso moderado",
            Self::ExcessoSevero => "Excesso severo",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticoPreco {
    pub estado: EstadoMercado,
    pub titulo: String,
    pub motivo: String,
    pub impacto_comercio: String,
    pub comparacao_mundial: String,
    pub tendencia_recente: String,
    pub recomendacao: String,
}

pub fn gerar_diagnostico(b: &VistaBem) -> DiagnosticoPreco {
    let o = b.oferta_nacional;
    let d = b.demanda_nacional;
    let p_base = b.preco_base;
    let p_nac = b.preco_nacional;
    let p_mun = b.preco_mundial;
    let diff_base_pct = if p_base > 0.0 { (p_nac - p_base) / p_base * 100.0 } else { 0.0 };
    let saldo = b.saldo_comercial;
    let var30 = b.var30_nacional;

    let (estado, titulo, motivo, recomendacao) = if o <= 1e-6 && d > 1e-6 {
        (
            EstadoMercado::EscassezCritica,
            "ESCASSEZ CRÍTICA (Sem Produção Local)".to_string(),
            format!(
                "Demanda nacional de {} un., sem nenhuma oferta interna disponível. O preço opera no teto ({:+.1}% vs base).",
                fmt_qtd(d),
                diff_base_pct
            ),
            "Construa prédios produtores deste bem ou abra rotas de importação para evitar gargalos industriais.".to_string(),
        )
    } else if d > o * 1.50 {
        let def_pct = (d / o.max(1.0) - 1.0) * 100.0;
        (
            EstadoMercado::EscassezCritica,
            "ESCASEZ SEVERA NO MERCADO LOCAL".to_string(),
            format!(
                "Demanda nacional ({}) supera a produção interna ({}) em {:.0}%. Preço inflacionado ({:+.1}% vs base).",
                fmt_qtd(d),
                fmt_qtd(o),
                def_pct,
                diff_base_pct
            ),
            "Forte incentivo para expandir capacidade produtiva local ou reduzir tarifas de importação.".to_string(),
        )
    } else if d > o * 1.10 {
        let def_pct = (d / o.max(1.0) - 1.0) * 100.0;
        (
            EstadoMercado::EscassezModerada,
            "ESCASEZ MODERADA NO MERCADO LOCAL".to_string(),
            format!(
                "Demanda nacional ({}) é {:.0}% maior que a oferta interna ({}). Preço pressionado para cima ({:+.1}% vs base).",
                fmt_qtd(d),
                def_pct,
                fmt_qtd(o),
                diff_base_pct
            ),
            "Bom momento para reinvestir lucros na expansão de fábricas e lavouras deste setor.".to_string(),
        )
    } else if o > d * 1.50 {
        let exc_pct = (o / d.max(1.0) - 1.0) * 100.0;
        (
            EstadoMercado::ExcessoSevero,
            "EXCESSO SEVERO DE OFERTA".to_string(),
            format!(
                "Produção nacional ({}) supera o consumo interno ({}) em {:.0}%. Estoques elevados derrubaram o preço ({:+.1}% vs base).",
                fmt_qtd(o),
                fmt_qtd(d),
                exc_pct,
                diff_base_pct
            ),
            "Exportar excedente para mercados externos ou subsidiar consumo para manter as fábricas lucrativas.".to_string(),
        )
    } else if o > d * 1.10 {
        let exc_pct = (o / d.max(1.0) - 1.0) * 100.0;
        (
            EstadoMercado::ExcessoModerado,
            "EXCESSO MODERADO DE PRODUÇÃO".to_string(),
            format!(
                "Oferta nacional ({}) excede o consumo local ({}) em {:.0}%. Preço ligeiramente deprimido ({:+.1}% vs base).",
                fmt_qtd(o),
                fmt_qtd(d),
                exc_pct,
                diff_base_pct
            ),
            "Indústrias consumidoras deste insumo se beneficiam de custos baixos.".to_string(),
        )
    } else {
        (
            EstadoMercado::Equilibrado,
            "MERCADO EM EQUILÍBRIO".to_string(),
            format!(
                "Oferta ({}) e demanda ({}) estão alinhadas. O preço (${:.1}) opera próximo à cotação base (${:.1}).",
                fmt_qtd(o),
                fmt_qtd(d),
                p_nac,
                p_base
            ),
            "Mercado estável; planeje novas expansões acompanhando o crescimento econômico geral.".to_string(),
        )
    };

    let impacto_comercio = if saldo > 1.0 {
        let pct_coberta = if d > 0.0 { (saldo / d) * 100.0 } else { 0.0 };
        format!(
            "Importação Líquida: O país importa +{} un. ({:.1}% da demanda local), mitigando a pressão inflacionária.",
            fmt_qtd(saldo),
            pct_coberta
        )
    } else if saldo < -1.0 {
        format!(
            "Exportação Líquida: Produtores escoam {} un. ao exterior, drenando excedentes e garantindo receitas.",
            fmt_qtd(-saldo)
        )
    } else if !b.comercializavel {
        "Bem Local: Intransferível entre mercados (serviço ou infraestrutura puramente doméstica).".to_string()
    } else {
        "Sem Fluxo Comercial Significativo: Atendido predominantemente pela produção interna.".to_string()
    };

    let diff_mun_pct = if p_mun > 0.0 { (p_nac - p_mun) / p_mun * 100.0 } else { 0.0 };
    let comparacao_mundial = if diff_mun_pct > 8.0 {
        format!(
            "Mercado Mundial: Preço local (${:.1}) está {:.1}% mais CARO que o mundial (${:.1}). Importar é economicamente vantajoso.",
            p_nac, diff_mun_pct, p_mun
        )
    } else if diff_mun_pct < -8.0 {
        format!(
            "Mercado Mundial: Preço local (${:.1}) está {:.1}% mais BARATO que o mundial (${:.1}). Forte competitividade para exportar.",
            p_nac, -diff_mun_pct, p_mun
        )
    } else {
        format!(
            "Mercado Mundial: Cotação local (${:.1}) converge com o valor de equilíbrio mundial (${:.1}).",
            p_nac, p_mun
        )
    };

    let tendencia_recente = if var30 > 0.02 {
        format!(
            "Tendência (30d): Alta de {:+.1}% nos últimos 30 dias por elevação no consumo ou aperto na cadeia de insumos.",
            var30 * 100.0
        )
    } else if var30 < -0.02 {
        format!(
            "Tendência (30d): Queda de {:+.1}% nos últimos 30 dias por expansão da oferta ou desaquecimento na demanda.",
            var30 * 100.0
        )
    } else {
        format!("Tendência (30d): Cotação estável nos últimos 30 dias ({:+.1}%).", var30 * 100.0)
    };

    DiagnosticoPreco {
        estado,
        titulo,
        motivo,
        impacto_comercio,
        comparacao_mundial,
        tendencia_recente,
        recomendacao,
    }
}

pub fn render_tooltip_diagnostico(ui: &mut egui::Ui, b: &VistaBem) {
    let diag = gerar_diagnostico(b);
    ui.set_max_width(330.0);
    ui.horizontal(|ui| {
        ui_refinada::glifo_ui(ui, ui_refinada::chave_para(&b.id), 15.0, None);
        ui.label(
            egui::RichText::new(&b.nome)
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(17.0)
                .strong()
                .color(ui_refinada::menu::AZUL_MARINHO),
        );
        ui.colored_label(diag.estado.cor(), diag.estado.rotulo());
    });
    ui_refinada::linha_decorativa(ui);
    egui::Grid::new("grade_tooltip_preco").striped(true).spacing([12.0, 2.0]).show(ui, |ui| {
        ui.weak("Preço nacional");
        ui.label(format!("${:.1}", b.preco_nacional));
        ui.end_row();
        ui.weak("Preço-base");
        ui.label(format!("${:.1}", b.preco_base));
        ui.end_row();
        ui.weak("Cotação mundial");
        ui.label(format!("${:.1}", b.preco_mundial));
        ui.end_row();
        ui.weak("Oferta / demanda");
        ui.label(format!("{} / {}", fmt_qtd(b.oferta_nacional), fmt_qtd(b.demanda_nacional)));
        ui.end_row();
    });
    let rotulo = |ui: &mut egui::Ui, t: &str| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(t.to_uppercase())
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(13.0)
                .strong()
                .color(ui_refinada::menu::TINTA_SUAVE),
        );
    };
    rotulo(ui, "Diagnóstico da formação de preço");
    let corpo = |ui: &mut egui::Ui, t: &str| {
        ui.add(egui::Label::new(egui::RichText::new(t).size(11.5).color(ui_refinada::menu::TINTA)).selectable(false));
    };
    corpo(ui, &diag.motivo);
    rotulo(ui, "Comércio internacional");
    corpo(ui, &diag.impacto_comercio);
    rotulo(ui, "Cotação mundial");
    corpo(ui, &diag.comparacao_mundial);
    rotulo(ui, "Variação dos últimos 30 dias");
    corpo(ui, &diag.tendencia_recente);
    ui.add_space(4.0);
    ui.colored_label(ui_refinada::menu::AVISO, format!("Recomendação: {}", diag.recomendacao));
}

// ---------------------------------------------------------------------------------------------
// Tela Principal de Mercado (2)
// ---------------------------------------------------------------------------------------------

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
    let v = &eco.vista;

    ui_refinada::janela_dossie("MERCADO GLOBAL & NACIONAL (2)", None)
        .default_pos([20.0, 92.0])
        .default_size([1240.0, 620.0])
        .resizable(false)
        .show(ctx, |ui| {
            ui_refinada::decorar_janela(ui);
            ui_refinada::faixa_titulo(ui, "Mercado Global & Nacional", Some("2"), None);
            if v.bens.is_empty() {
                ui.label("Aguardando os primeiros dados do motor econômico...");
                return;
            }

            // Barra superior: Busca, estatísticas e resumo
            ui.horizontal_wrapped(|ui| {
                ui_refinada::glifo_ui(ui, "lupa", 15.0, Some(ui_refinada::menu::VERMELHO));
                let resp_busca = ui.add(
                    egui::TextEdit::singleline(&mut painel.busca)
                        .hint_text("Buscar bem por nome ou ID...")
                        .desired_width(200.0),
                );
                if !painel.busca.is_empty() && ui.button("Limpar").on_hover_text("Limpar busca").clicked() {
                    painel.busca.clear();
                }
                ui.separator();
                ui.label(format!("Bens catalogados: {}", v.bens.len()));
                ui.separator();
                ui.label(format!(
                    "Turismo: receita {}/dia, gasto no exterior {}/dia, segurança {:.0}%",
                    crate::plugins::jogador::fmt_din(v.turismo_receita_dia),
                    crate::plugins::jogador::fmt_din(v.turismo_gasto_exterior_dia),
                    v.turismo_seguranca * 100.0
                ));
                if resp_busca.changed() && painel.selecionado.is_some() {
                    // Mantém seleção se ainda estiver visível
                }
            });
            ui.label(
                egui::RichText::new("Dica: clique no cabeçalho de uma coluna para ordenar; clique num bem para inspecionar.")
                    .size(11.0)
                    .color(ui_refinada::menu::TINTA_SUAVE),
            );

            // Linha de filtros por categoria (glifos vetoriais + contagem)
            ui.horizontal_wrapped(|ui| {
                for f in Filtro::TODOS {
                    let contagem = if f == Filtro::Todos {
                        v.bens.len()
                    } else {
                        v.bens.iter().filter(|b| f.aceita(b.categoria)).count()
                    };
                    if ui_refinada::aba_glifo(ui, painel.filtro == f, Some(f.icone()), f.nome()) {
                        painel.filtro = f;
                    }
                    ui.label(egui::RichText::new(format!("{contagem}")).small().color(ui_refinada::menu::TINTA_SUAVE));
                    ui.add_space(3.0);
                }
            });

            ui.add_space(4.0);
            ui_refinada::linha_decorativa(ui);

            // Filtragem e ordenação da lista
            let busca_norm = painel.busca.trim().to_lowercase();
            let mut lista: Vec<&VistaBem> = v
                .bens
                .iter()
                .filter(|b| {
                    if !painel.filtro.aceita(b.categoria) {
                        return false;
                    }
                    if !busca_norm.is_empty() {
                        let nome_ok = b.nome.to_lowercase().contains(&busca_norm);
                        let id_ok = b.id.to_lowercase().contains(&busca_norm);
                        if !nome_ok && !id_ok {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            // Ordenação configurada
            match (painel.coluna_ordem, painel.sentido_ordem) {
                (ColunaOrdenacao::Nome, SentidoOrdenacao::Ascendente) => lista.sort_by(|a, b| a.nome.cmp(&b.nome)),
                (ColunaOrdenacao::Nome, SentidoOrdenacao::Descendente) => lista.sort_by(|a, b| b.nome.cmp(&a.nome)),
                (ColunaOrdenacao::PrecoNacional, SentidoOrdenacao::Ascendente) => {
                    lista.sort_by(|a, b| a.preco_nacional.total_cmp(&b.preco_nacional))
                }
                (ColunaOrdenacao::PrecoNacional, SentidoOrdenacao::Descendente) => {
                    lista.sort_by(|a, b| b.preco_nacional.total_cmp(&a.preco_nacional))
                }
                (ColunaOrdenacao::PrecoMundial, SentidoOrdenacao::Ascendente) => {
                    lista.sort_by(|a, b| a.preco_mundial.total_cmp(&b.preco_mundial))
                }
                (ColunaOrdenacao::PrecoMundial, SentidoOrdenacao::Descendente) => {
                    lista.sort_by(|a, b| b.preco_mundial.total_cmp(&a.preco_mundial))
                }
                (ColunaOrdenacao::Variacao, SentidoOrdenacao::Ascendente) => {
                    lista.sort_by(|a, b| a.var30_nacional.total_cmp(&b.var30_nacional))
                }
                (ColunaOrdenacao::Variacao, SentidoOrdenacao::Descendente) => {
                    lista.sort_by(|a, b| b.var30_nacional.total_cmp(&a.var30_nacional))
                }
                (ColunaOrdenacao::Oferta, SentidoOrdenacao::Ascendente) => {
                    lista.sort_by(|a, b| a.oferta_nacional.total_cmp(&b.oferta_nacional))
                }
                (ColunaOrdenacao::Oferta, SentidoOrdenacao::Descendente) => {
                    lista.sort_by(|a, b| b.oferta_nacional.total_cmp(&a.oferta_nacional))
                }
                (ColunaOrdenacao::Demanda, SentidoOrdenacao::Ascendente) => {
                    lista.sort_by(|a, b| a.demanda_nacional.total_cmp(&b.demanda_nacional))
                }
                (ColunaOrdenacao::Demanda, SentidoOrdenacao::Descendente) => {
                    lista.sort_by(|a, b| b.demanda_nacional.total_cmp(&a.demanda_nacional))
                }
                (ColunaOrdenacao::Saldo, SentidoOrdenacao::Ascendente) => {
                    lista.sort_by(|a, b| a.saldo_comercial.total_cmp(&b.saldo_comercial))
                }
                (ColunaOrdenacao::Saldo, SentidoOrdenacao::Descendente) => {
                    lista.sort_by(|a, b| b.saldo_comercial.total_cmp(&a.saldo_comercial))
                }
            }

            // Layout em duas colunas (Esquerda: Tabela de Bens | Direita: Painel Detalhado)
            ui.columns(2, |cols| {
                // =====================================================================
                // Coluna Esquerda: Tabela de Bens Victoria 3
                // =====================================================================
                egui::ScrollArea::vertical().id_salt("tab_bens_scroll").max_height(430.0).show(&mut cols[0], |ui| {
                    egui::Grid::new("grade_bens_f2")
                        .striped(true)
                        .num_columns(8)
                        .spacing([6.0, 4.0])
                        .show(ui, |ui| {
                            // Cabeçalhos clicáveis: triângulo desenhado pelo painter quando a coluna ordena.
                            let mut cabecalho_btn = |ui: &mut egui::Ui, col: ColunaOrdenacao, tit: &str, largura: f32| {
                                let ativo = painel.coluna_ordem == col;
                                let asc = painel.sentido_ordem == SentidoOrdenacao::Ascendente;
                                let resp = ui.add_sized(
                                    [largura, 18.0],
                                    egui::Label::new(
                                        egui::RichText::new(tit.to_uppercase())
                                            .family(ui_refinada::familia_oswald(ui.ctx()))
                                            .size(11.0)
                                            .strong()
                                            .color(if ativo { ui_refinada::menu::VERMELHO } else { ui_refinada::menu::TINTA_SUAVE })
                                            .background_color(if ativo { ui_refinada::menu::PAPEL_ALT } else { egui::Color32::TRANSPARENT }),
                                    )
                                    .selectable(false)
                                    .sense(egui::Sense::click()),
                                );
                                if ativo {
                                    let r = resp.rect;
                                    let x = r.max.x - 4.0;
                                    let y = r.center().y;
                                    let tri: Vec<egui::Pos2> = if asc {
                                        vec![egui::pos2(x, y - 3.5), egui::pos2(x + 4.0, y + 2.5), egui::pos2(x - 4.0, y + 2.5)]
                                    } else {
                                        vec![egui::pos2(x - 4.0, y - 2.5), egui::pos2(x + 4.0, y - 2.5), egui::pos2(x, y + 3.5)]
                                    };
                                    ui.painter().add(egui::Shape::convex_polygon(tri, ui_refinada::menu::VERMELHO, egui::Stroke::NONE));
                                }
                                if resp.clicked() {
                                    if painel.coluna_ordem == col {
                                        painel.sentido_ordem = match painel.sentido_ordem {
                                            SentidoOrdenacao::Ascendente => SentidoOrdenacao::Descendente,
                                            SentidoOrdenacao::Descendente => SentidoOrdenacao::Ascendente,
                                        };
                                    } else {
                                        painel.coluna_ordem = col;
                                        painel.sentido_ordem = match col {
                                            ColunaOrdenacao::Nome => SentidoOrdenacao::Ascendente,
                                            _ => SentidoOrdenacao::Descendente,
                                        };
                                    }
                                }
                            };

                            cabecalho_btn(ui, ColunaOrdenacao::Nome, "Bem", 132.0);
                            cabecalho_btn(ui, ColunaOrdenacao::PrecoNacional, "Nacional", 62.0);
                            cabecalho_btn(ui, ColunaOrdenacao::PrecoMundial, "Mundial", 50.0);
                            cabecalho_btn(ui, ColunaOrdenacao::Variacao, "Var 30d", 60.0);
                            ui.add_sized([44.0, 18.0], egui::Label::new(egui::RichText::new("TEND.").family(ui_refinada::familia_oswald(ui.ctx())).size(11.0).strong().color(ui_refinada::menu::TINTA_SUAVE)).selectable(false));
                            cabecalho_btn(ui, ColunaOrdenacao::Oferta, "Oferta", 46.0);
                            cabecalho_btn(ui, ColunaOrdenacao::Demanda, "Dem.", 44.0);
                            cabecalho_btn(ui, ColunaOrdenacao::Saldo, "Saldo", 76.0);
                            ui.end_row();

                            if lista.is_empty() {
                                ui.label(egui::RichText::new("Nenhum bem encontrado com os filtros atuais.").italics());
                                for _ in 0..7 {
                                    ui.label("");
                                }
                                ui.end_row();
                                return;
                            }

                            for b in &lista {
                                let selecionado = painel.selecionado.as_deref() == Some(&b.id);

                                // Coluna 1: Ícone + Nome do Bem (largura fixa, sem quebrar a grade)
                                ui.allocate_ui_with_layout(
                                    egui::Vec2::new(132.0, 20.0),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        icones.mostrar(ui, &b.id, 18.0);
                                        let nome_corto: String = if b.nome.chars().count() > 16 {
                                            format!("{}...", b.nome.chars().take(16).collect::<String>())
                                        } else {
                                            b.nome.clone()
                                        };
                                        let btn = ui.selectable_label(
                                            selecionado,
                                            egui::RichText::new(nome_corto).size(12.0).color(ui_refinada::menu::TINTA),
                                        );
                                        if btn.clicked() {
                                            painel.selecionado = Some(b.id.clone());
                                        }
                                        btn.on_hover_ui(|ui| render_tooltip_diagnostico(ui, b));
                                    },
                                );

                                // Coluna 2: Preço Nacional + desvio vs preço base
                                let diff_base_pct = if b.preco_base > 0.0 {
                                    (b.preco_nacional - b.preco_base) / b.preco_base * 100.0
                                } else {
                                    0.0
                                };
                                let cor_p_nac = cor_preco_relativo(b.preco_nacional, b.preco_base);
                                let lbl_p_nac = ui.add_sized(
                                    [62.0, 18.0],
                                    egui::Label::new(egui::RichText::new(format!("{:.1} {:+.0}%", b.preco_nacional, diff_base_pct)).size(12.0).color(cor_p_nac)).selectable(false),
                                );
                                lbl_p_nac.on_hover_ui(|ui| render_tooltip_diagnostico(ui, b));

                                // Coluna 3: Preço Mundial
                                ui.add_sized([50.0, 18.0], egui::Label::new(egui::RichText::new(format!("{:.1}", b.preco_mundial)).size(12.0)).selectable(false));

                                // Coluna 4: Variação 30d (Nacional)
                                let lbl_var = ui.add_sized(
                                    [60.0, 18.0],
                                    egui::Label::new(egui::RichText::new(fmt_var(b.var30_nacional)).size(12.0).color(cor_variacao(b.var30_nacional))).selectable(false),
                                );
                                lbl_var.on_hover_ui(|ui| render_tooltip_diagnostico(ui, b));

                                // Coluna 5: Sparkline de 30 dias
                                let resp_spark = desenhar_sparkline(ui, &b.hist_nacional, 30, 44.0, 16.0);
                                resp_spark.on_hover_ui(|ui| {
                                    let n = b.hist_nacional.len();
                                    let slice = &b.hist_nacional[n.saturating_sub(30)..];
                                    if !slice.is_empty() {
                                        let min_val = slice.iter().copied().fold(f32::INFINITY, f32::min);
                                        let max_val = slice.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                                        ui.strong(format!("Cotação 30d: {}", b.nome));
                                        ui.label(format!("Mínimo: ${min_val:.1} | Máximo: ${max_val:.1}"));
                                        ui.label(format!("Tendência nacional: {}", fmt_var(b.var30_nacional)));
                                    }
                                });

                                // Coluna 6: Oferta Nacional
                                ui.add_sized([46.0, 18.0], egui::Label::new(egui::RichText::new(fmt_qtd(b.oferta_nacional)).size(12.0)).selectable(false));

                                // Coluna 7: Demanda Nacional
                                ui.add_sized([44.0, 18.0], egui::Label::new(egui::RichText::new(fmt_qtd(b.demanda_nacional)).size(12.0)).selectable(false));

                                // Coluna 8: Saldo Comercial (Líquido)
                                let s = b.saldo_comercial;
                                let (txt, cor_saldo) = if s > 1.0 {
                                    (format!("{} imp", fmt_qtd(s)), ui_refinada::menu::INFO)
                                } else if s < -1.0 {
                                    (format!("{} exp", fmt_qtd(s)), ui_refinada::menu::AVISO)
                                } else {
                                    ("equilibrado".to_string(), ui_refinada::menu::TINTA_SUAVE)
                                };
                                let lbl_saldo = ui.add_sized(
                                    [76.0, 18.0],
                                    egui::Label::new(egui::RichText::new(txt).size(12.0).color(cor_saldo)).selectable(false),
                                );
                                lbl_saldo.on_hover_ui(|ui| render_tooltip_diagnostico(ui, b));

                                ui.end_row();
                            }
                        });
                });

                // =====================================================================
                // Coluna Direita: Painel Detalhado Victoria 3
                // =====================================================================
                let ui_dir = &mut cols[1];
                let escolhido = painel.selecionado.as_deref().and_then(|id| v.bens.iter().find(|b| b.id == id));

                match escolhido {
                    None => {
                        ui_dir.vertical_centered(|ui| {
                            ui.add_space(60.0);
                            ui_refinada::glifo_ui(ui, "lupa", 34.0, Some(ui_refinada::menu::VERMELHO));
                            ui.add_space(6.0);
                            ui_refinada::titulo_secao(ui, "Inspeção Detalhada de Mercado");
                            ui.add_space(6.0);
                            ui.label("Selecione um bem na tabela ao lado para visualizar:");
                            ui.label("• Gráfico histórico de preços (1, 3 e 5 anos)");
                            ui.label("• Diagnóstico detalhado de escassez, excesso e importações");
                            ui.label("• Maiores produtores e consumidores no mercado global");
                            ui.label("• Prédios e métodos que produzem e consomem o bem");
                        });
                    }
                    Some(b) => {
                        detalhe_bem(ui_dir, b, v, &mut painel, &mut icones);
                    }
                }
            });
        });

    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Painel Detalhado do Bem Selecionado
// ---------------------------------------------------------------------------------------------

fn niveis_predio_no_pais(v: &VistaEconomia, tipo_predio_id: &str) -> u32 {
    let mut total = 0;
    for prov in &v.provincias {
        for p in &prov.predios {
            if p.tipo_predio_id == tipo_predio_id {
                total += p.nivel;
            }
        }
    }
    total
}

fn detalhe_bem(
    ui: &mut egui::Ui,
    b: &VistaBem,
    v: &VistaEconomia,
    painel: &mut PainelMercado,
    icones: &mut Icones,
) {
    egui::ScrollArea::vertical().id_salt("detalhe_bem_scroll").show(ui, |ui| {
        // Cabeçalho do Bem
        ui.horizontal(|ui| {
            icones.mostrar(ui, &b.id, 34.0);
            ui.vertical(|ui| {
                ui.heading(&b.nome);
                ui.horizontal(|ui| {
                    ui.colored_label(ui_refinada::menu::TINTA_SUAVE, nome_categoria(b.categoria));
                    ui.separator();
                    if b.comercializavel {
                        ui.colored_label(ui_refinada::menu::SUCESSO, "Comercializável Globalmente");
                    } else {
                        ui.colored_label(ui_refinada::menu::AVISO, "Bem Local (Intransferível)");
                    }
                });
            });
        });

        ui.add_space(4.0);

        // Indicadores rápidos em grade
        egui::Grid::new("grid_indicadores_bem").num_columns(4).spacing([12.0, 4.0]).show(ui, |ui| {
            ui.label("Preço Base:");
            ui.strong(format!("${:.1}", b.preco_base));

            ui.label("Preço Nacional:");
            let diff_base = if b.preco_base > 0.0 { (b.preco_nacional - b.preco_base) / b.preco_base * 100.0 } else { 0.0 };
            ui.colored_label(cor_preco_relativo(b.preco_nacional, b.preco_base), format!("${:.1} ({:+.1}%)", b.preco_nacional, diff_base));
            ui.end_row();

            ui.label("Preço Mundial:");
            ui.strong(format!("${:.1}", b.preco_mundial));

            ui.label("Variação 30d (Nac.):");
            ui.colored_label(cor_variacao(b.var30_nacional), fmt_var(b.var30_nacional));
            ui.end_row();

            ui.label("Oferta Nacional:");
            ui.label(fmt_qtd(b.oferta_nacional));

            ui.label("Demanda Nacional:");
            ui.label(fmt_qtd(b.demanda_nacional));
            ui.end_row();

            ui.label("Saldo Comercial:");
            let s = b.saldo_comercial;
            if s > 1.0 {
                ui.colored_label(ui_refinada::menu::INFO, format!("+{} (Importa)", fmt_qtd(s)));
            } else if s < -1.0 {
                ui.colored_label(ui_refinada::menu::AVISO, format!("{} (Exporta)", fmt_qtd(s)));
            } else {
                ui.colored_label(ui_refinada::menu::TINTA_SUAVE, "Equilibrado");
            }

            ui.label("Variação 30d (Mun.):");
            ui.colored_label(cor_variacao(b.var30_mundial), fmt_var(b.var30_mundial));
            ui.end_row();
        });

        ui.add_space(6.0);

        // Caixa de Diagnóstico Econômico (Causas de Preço, Escassez, Excesso, Importação)
        let diag = gerar_diagnostico(b);
        ui_refinada::moldura_cartao()
            .stroke(egui::Stroke::new(1.0, diag.estado.cor()))
            .show(ui, |ui| {
                ui_refinada::titulo_secao(ui, "Diagnóstico de Mercado");
                ui.horizontal(|ui| {
                    ui_refinada::glifo_ui(ui, "grafico", 14.0, Some(diag.estado.cor()));
                    ui.colored_label(diag.estado.cor(), &diag.titulo);
                });
                ui.add_space(2.0);
                ui.label(&diag.motivo);
                ui.label(&diag.impacto_comercio);
                ui.label(&diag.comparacao_mundial);
                ui.label(&diag.tendencia_recente);
                ui_refinada::linha_decorativa(ui);
                ui.colored_label(ui_refinada::menu::AVISO, format!("Recomendação: {}", diag.recomendacao));
            });

        ui.add_space(8.0);

        // Seletor de Escala de Tempo e Gráfico Histórico (1, 3 e 5 Anos)
        ui.horizontal(|ui| {
            ui.strong("Histórico de Preço:");
            for esc in [
                EscalaTempoGrafico::Dias30,
                EscalaTempoGrafico::Ano1,
                EscalaTempoGrafico::Anos3,
                EscalaTempoGrafico::Anos5,
                EscalaTempoGrafico::Tudo,
            ] {
                if ui.selectable_label(painel.escala_grafico == esc, esc.rotulo()).clicked() {
                    painel.escala_grafico = esc;
                }
            }
        });

        // Preparação das fatias de dados para o plot
        let num_dias = b.hist_mundial.len().min(b.hist_nacional.len());
        let janela = painel.escala_grafico.dias().min(num_dias);
        let inicio = num_dias.saturating_sub(janela);

        let slice_mun = &b.hist_mundial[inicio..num_dias];
        let slice_nac = &b.hist_nacional[inicio..num_dias];

        let serie_pts = |slice: &[f32]| -> PlotPoints<'static> {
            slice
                .iter()
                .enumerate()
                .map(|(i, &p)| {
                    let dia = (inicio + i) as f64;
                    [dia, p as f64]
                })
                .collect()
        };

        Plot::new("plot_preco_mercado")
            .height(180.0)
            .legend(Legend::default().position(egui_plot::Corner::LeftTop))
            .x_axis_label("dias de jogo")
            .allow_scroll(false)
            .show(ui, |pu| {
                pu.line(
                    Line::new("Preço Mundial", serie_pts(slice_mun))
                        .color(ui_refinada::menu::OURO)
                        .width(2.0),
                );
                pu.line(
                    Line::new("Seu Mercado", serie_pts(slice_nac))
                        .color(ui_refinada::menu::AZUL_MARINHO)
                        .width(2.0),
                );
                let x0 = inicio as f64;
                let x1 = num_dias.max(inicio + 1) as f64;
                pu.line(
                    Line::new("Preço Base", PlotPoints::from(vec![[x0, b.preco_base], [x1, b.preco_base]]))
                        .color(ui_refinada::menu::TINTA_SUAVE)
                        .style(LineStyle::Dashed { length: 5.0 })
                        .width(1.0),
                );
            });

        ui.add_space(8.0);

        // Maiores Produtores e Consumidores (Mercados Globais)
        ui.columns(2, |cols| {
            ui_refinada::titulo_secao(&mut cols[0], "Maiores produtores mundiais");
            if b.produtores.is_empty() {
                cols[0].label("Nenhum dado de produção disponível.");
            } else {
                for (n, q) in &b.produtores {
                    let pct = if b.oferta_mundial > 0.0 { (q / b.oferta_mundial) * 100.0 } else { 0.0 };
                    cols[0].horizontal(|ui| {
                        ui.add_sized([130.0, 16.0], egui::Label::new(format!("{n}")));
                        ui.strong(fmt_qtd(*q));
                        ui_refinada::barra_trilho(ui, (pct / 100.0) as f32, ui_refinada::menu::SUCESSO, 54.0, 9.0);
                        ui.colored_label(ui_refinada::menu::TINTA_SUAVE, format!("({pct:.1}%)"));
                    });
                }
            }

            ui_refinada::titulo_secao(&mut cols[1], "Maiores consumidores mundiais");
            if b.consumidores.is_empty() {
                cols[1].label("Nenhum dado de consumo disponível.");
            } else {
                for (n, q) in &b.consumidores {
                    let pct = if b.demanda_mundial > 0.0 { (q / b.demanda_mundial) * 100.0 } else { 0.0 };
                    cols[1].horizontal(|ui| {
                        ui.add_sized([130.0, 16.0], egui::Label::new(format!("{n}")));
                        ui.strong(fmt_qtd(*q));
                        ui_refinada::barra_trilho(ui, (pct / 100.0) as f32, ui_refinada::menu::AVISO, 54.0, 9.0);
                        ui.colored_label(ui_refinada::menu::TINTA_SUAVE, format!("({pct:.1}%)"));
                    });
                }
            }
        });

        // Cadeia de Produção e Consumo: Prédios e Métodos
        if let Some(cat) = &v.catalogo {
            ui.separator();
            ui.columns(2, |cols| {
                // Prédios que produzem
                ui_refinada::titulo_secao(&mut cols[0], "Prédios que produzem");
                let mut produzem = Vec::new();
                for t in &cat.tipos_predio {
                    let mut metodos = Vec::new();
                    for mid in &t.metodos_permitidos {
                        if let Some(m) = cat.metodos_producao.iter().find(|m| &m.id == mid) {
                            if let Some(&qtd) = m.produtos.get(&b.id) {
                                metodos.push((m.nome.clone(), qtd));
                            }
                        }
                    }
                    if !metodos.is_empty() {
                        let niv = niveis_predio_no_pais(v, &t.id);
                        produzem.push((t, metodos, niv));
                    }
                }
                produzem.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.nome.cmp(&b.0.nome)));

                if produzem.is_empty() {
                    cols[0].label(egui::RichText::new("Nenhum prédio catalogado produz este bem diretamente.").italics());
                } else {
                    for (t, mets, niv) in produzem {
                        cols[0].horizontal(|ui| {
                            icones.mostrar(ui, &t.id, 18.0);
                            ui.strong(&t.nome);
                            if niv > 0 {
                                ui.colored_label(ui_refinada::menu::SUCESSO, format!("[Nível {niv}]"));
                            } else {
                                ui.colored_label(ui_refinada::menu::TINTA_SUAVE, "[0 no país]");
                            }
                        });
                        for (mnome, qtd) in mets {
                            cols[0].horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.small(&mnome);
                                ui.colored_label(ui_refinada::menu::SUCESSO, format!("+{qtd:.1}/nível"));
                            });
                        }
                    }
                }

                // Prédios que consomem
                ui_refinada::titulo_secao(&mut cols[1], "Prédios que consomem");
                let mut consomem = Vec::new();
                for t in &cat.tipos_predio {
                    let mut metodos = Vec::new();
                    for mid in &t.metodos_permitidos {
                        if let Some(m) = cat.metodos_producao.iter().find(|m| &m.id == mid) {
                            if let Some(&qtd) = m.insumos.get(&b.id) {
                                metodos.push((m.nome.clone(), qtd));
                            }
                        }
                    }
                    if !metodos.is_empty() {
                        let niv = niveis_predio_no_pais(v, &t.id);
                        consomem.push((t, metodos, niv));
                    }
                }
                consomem.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.nome.cmp(&b.0.nome)));

                if consomem.is_empty() {
                    cols[1].label(egui::RichText::new("Nenhum prédio catalogado consome este bem como insumo.").italics());
                } else {
                    for (t, mets, niv) in consomem {
                        cols[1].horizontal(|ui| {
                            icones.mostrar(ui, &t.id, 18.0);
                            ui.strong(&t.nome);
                            if niv > 0 {
                                ui.colored_label(ui_refinada::menu::SUCESSO, format!("[Nível {niv}]"));
                            } else {
                                ui.colored_label(ui_refinada::menu::TINTA_SUAVE, "[0 no país]");
                            }
                        });
                        for (mnome, qtd) in mets {
                            cols[1].horizontal(|ui| {
                                ui.add_space(24.0);
                                ui.small(&mnome);
                                ui.colored_label(ui_refinada::menu::PERIGO, format!("-{qtd:.1}/nível"));
                            });
                        }
                    }
                }
            });
        }
    });
}

// ---------------------------------------------------------------------------------------------
// Testes Unitários
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod testes {
    use super::*;

    fn criar_bem_exemplo(id: &str, nome: &str, cat: CategoriaBem, o: f64, d: f64, p_nac: f64, p_base: f64, saldo: f64, var: f64) -> VistaBem {
        VistaBem {
            idx: 0,
            id: id.to_string(),
            nome: nome.to_string(),
            categoria: cat,
            comercializavel: true,
            preco_base: p_base,
            preco_mundial: p_base,
            preco_nacional: p_nac,
            var30_mundial: var,
            var30_nacional: var,
            oferta_nacional: o,
            demanda_nacional: d,
            saldo_comercial: saldo,
            oferta_mundial: o * 10.0,
            demanda_mundial: d * 10.0,
            produtores: vec![("Mercado Brasil".into(), o)],
            consumidores: vec![("Mercado Brasil".into(), d)],
            hist_mundial: vec![p_base as f32; 60],
            hist_nacional: vec![p_nac as f32; 60],
        }
    }

    #[test]
    fn filtro_categoria_e_busca() {
        let b_graos = criar_bem_exemplo("graos", "Grãos", CategoriaBem::AlimentoBasico, 100.0, 100.0, 20.0, 20.0, 0.0, 0.0);
        let b_ferro = criar_bem_exemplo("ferro", "Minério de Ferro", CategoriaBem::MateriaPrima, 200.0, 150.0, 30.0, 30.0, 0.0, 0.0);

        assert!(Filtro::Todos.aceita(b_graos.categoria));
        assert!(Filtro::Todos.aceita(b_ferro.categoria));

        assert!(Filtro::Alimentos.aceita(b_graos.categoria));
        assert!(!Filtro::Alimentos.aceita(b_ferro.categoria));

        assert!(Filtro::Materias.aceita(b_ferro.categoria));
        assert!(!Filtro::Materias.aceita(b_graos.categoria));
    }

    #[test]
    fn diagnostico_escassez_gera_alerta_correto() {
        // Demanda supera oferta: escassez
        let b = criar_bem_exemplo("aco", "Aço", CategoriaBem::Industrial, 100.0, 200.0, 60.0, 40.0, 50.0, 0.15);
        let diag = gerar_diagnostico(&b);

        assert!(matches!(diag.estado, EstadoMercado::EscassezCritica | EstadoMercado::EscassezModerada));
        assert!(diag.motivo.contains("supera"));
        assert!(diag.impacto_comercio.contains("Importação Líquida"));
        assert!(diag.tendencia_recente.contains("Alta de +15.0%"));
    }

    #[test]
    fn diagnostico_excesso_e_exportacao() {
        // Oferta supera demanda: excesso
        let b = criar_bem_exemplo("petroleo", "Petróleo", CategoriaBem::Energia, 300.0, 100.0, 15.0, 20.0, -80.0, -0.08);
        let diag = gerar_diagnostico(&b);

        assert!(matches!(diag.estado, EstadoMercado::ExcessoSevero | EstadoMercado::ExcessoModerado));
        assert!(diag.motivo.contains("supera") || diag.motivo.contains("excede"));
        assert!(diag.impacto_comercio.contains("Exportação Líquida"));
        assert!(diag.tendencia_recente.contains("Queda de -8.0%"));
    }

    #[test]
    fn diagnostico_mercado_equilibrado() {
        let b = criar_bem_exemplo("tecido", "Tecido", CategoriaBem::Industrial, 100.0, 102.0, 20.0, 20.0, 0.0, 0.001);
        let diag = gerar_diagnostico(&b);

        assert_eq!(diag.estado, EstadoMercado::Equilibrado);
        assert!(diag.motivo.contains("alinhadas"));
    }

    #[test]
    fn formatacoes_valores_e_variacao() {
        assert_eq!(fmt_var(0.125), "+12.5%");
        assert_eq!(fmt_var(-0.063), "-6.3%");
        assert_eq!(fmt_var(0.001), "+0.1%");
        assert_eq!(fmt_var(0.0), "plano");

        assert_eq!(fmt_qtd(500.0), "500");
        assert_eq!(fmt_qtd(1500.0), "1.5 mil");
        assert_eq!(fmt_qtd(2_500_000.0), "2.50 mi");
        assert_eq!(fmt_qtd(3_200_000_000.0), "3.20 bi");
    }

    #[test]
    fn cores_preco_relativo_diferenciam_niveis() {
        let base = 100.0;
        let c_muito_caro = cor_preco_relativo(130.0, base);
        let c_caro = cor_preco_relativo(110.0, base);
        let c_equilibrado = cor_preco_relativo(100.0, base);
        let c_barato = cor_preco_relativo(90.0, base);
        let c_muito_barato = cor_preco_relativo(70.0, base);

        assert_ne!(c_muito_caro, c_equilibrado);
        assert_ne!(c_caro, c_barato);
        assert_ne!(c_muito_barato, c_equilibrado);
    }

    #[test]
    fn ordenacao_colunas_funciona() {
        let b1 = criar_bem_exemplo("madeira", "Madeira", CategoriaBem::MateriaPrima, 50.0, 40.0, 15.0, 10.0, -10.0, 0.05);
        let b2 = criar_bem_exemplo("carvao", "Carvão", CategoriaBem::Energia, 200.0, 300.0, 50.0, 30.0, 100.0, 0.25);
        let b3 = criar_bem_exemplo("aco", "Aço", CategoriaBem::Industrial, 80.0, 120.0, 35.0, 25.0, 40.0, -0.10);

        let mut lista = vec![&b1, &b2, &b3];

        // Ordenar por Nome Ascendente
        lista.sort_by(|a, b| a.nome.cmp(&b.nome));
        assert_eq!(lista[0].id, "aco");
        assert_eq!(lista[1].id, "carvao");
        assert_eq!(lista[2].id, "madeira");

        // Ordenar por Preço Nacional Descendente
        lista.sort_by(|a, b| b.preco_nacional.total_cmp(&a.preco_nacional));
        assert_eq!(lista[0].id, "carvao"); // 50.0
        assert_eq!(lista[1].id, "aco");    // 35.0
        assert_eq!(lista[2].id, "madeira"); // 15.0

        // Ordenar por Variação 30d Descendente
        lista.sort_by(|a, b| b.var30_nacional.total_cmp(&a.var30_nacional));
        assert_eq!(lista[0].id, "carvao"); // +25%
        assert_eq!(lista[1].id, "madeira"); // +5%
        assert_eq!(lista[2].id, "aco");    // -10%

        // Ordenar por Saldo Comercial Descendente
        lista.sort_by(|a, b| b.saldo_comercial.total_cmp(&a.saldo_comercial));
        assert_eq!(lista[0].id, "carvao"); // +100
        assert_eq!(lista[1].id, "aco");    // +40
        assert_eq!(lista[2].id, "madeira"); // -10
    }

    #[test]
    fn escalas_tempo_grafico_valores() {
        assert_eq!(EscalaTempoGrafico::Dias30.dias(), 30);
        assert_eq!(EscalaTempoGrafico::Ano1.dias(), 365);
        assert_eq!(EscalaTempoGrafico::Anos3.dias(), 1095);
        assert_eq!(EscalaTempoGrafico::Anos5.dias(), 1825);
        assert_eq!(EscalaTempoGrafico::Tudo.dias(), usize::MAX);

        assert_eq!(EscalaTempoGrafico::Dias30.rotulo(), "30 dias");
        assert_eq!(EscalaTempoGrafico::Ano1.rotulo(), "1 ano");
        assert_eq!(EscalaTempoGrafico::Anos3.rotulo(), "3 anos");
        assert_eq!(EscalaTempoGrafico::Anos5.rotulo(), "5 anos");
    }
}
