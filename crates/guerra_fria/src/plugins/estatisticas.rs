//! Estatísticas e Ledger Nacional (estilo Victoria 3) - Tecla 7
//!
//! - Gráficos históricos com `egui_plot`: PIB, PIB per capita, população, desemprego,
//!   padrão de vida, dívida, tesouro e produção por bem, comparando múltiplos países.
//! - Rankings de potência com as "Grandes Potências" (Top 8) no estilo Victoria 3,
//!   divididos em Potência Econômica, Militar e de Influência / Geopolítica.
//! - Ledger geral com tabela completa de todos os países ordenável por qualquer coluna.
//! - Histórico compactado mensalmente em memória (4 bytes por valor escalar).

use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::jogador::{PaisJogador, fmt_din, fmt_pop};
use super::time::RecursoRelogio;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use egui_plot::{Corner, Legend, Line, Plot, PlotPoints};
use std::collections::{BTreeMap, BTreeSet};

// -------------------------------------------------------------------------------------------------
// Tipos de dados e histórico compacto
// -------------------------------------------------------------------------------------------------

/// Métrica selecionada para visualização nos gráficos temporais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetricaGrafico {
    #[default]
    Pib,
    PibPerCapita,
    Populacao,
    Desemprego,
    PadraoVida,
    Divida,
    Tesouro,
    ProducaoBem,
}

impl MetricaGrafico {
    pub const TODAS: [Self; 8] = [
        Self::Pib,
        Self::PibPerCapita,
        Self::Populacao,
        Self::Desemprego,
        Self::PadraoVida,
        Self::Divida,
        Self::Tesouro,
        Self::ProducaoBem,
    ];

    pub fn titulo(self) -> &'static str {
        match self {
            Self::Pib => "PIB Anual",
            Self::PibPerCapita => "PIB Per Capita",
            Self::Populacao => "População Total",
            Self::Desemprego => "Taxa de Desemprego",
            Self::PadraoVida => "Padrão de Vida (SoL)",
            Self::Divida => "Dívida Soberana",
            Self::Tesouro => "Tesouro Nacional",
            Self::ProducaoBem => "Produção por Bem",
        }
    }

    pub fn unidade(self) -> &'static str {
        match self {
            Self::Pib | Self::Divida | Self::Tesouro => "US$",
            Self::PibPerCapita => "US$/hab",
            Self::Populacao => "habitantes",
            Self::Desemprego => "%",
            Self::PadraoVida => "nível SoL (1 a 50)",
            Self::ProducaoBem => "unidades/dia",
        }
    }

    pub fn formatar_valor(self, v: f64) -> String {
        match self {
            Self::Pib | Self::Divida | Self::Tesouro => fmt_din(v),
            Self::PibPerCapita => format!("${:.1}", v),
            Self::Populacao => fmt_pop(v.max(0.0) as u64),
            Self::Desemprego => format!("{:.1}%", v * 100.0),
            Self::PadraoVida => format!("{:.1}", v),
            Self::ProducaoBem => format!("{:.1}", v),
        }
    }

    /// Fator de divisão aplicado ao eixo do gráfico, para os números caberem na régua.
    pub fn escala(self) -> f64 {
        match self {
            Self::Pib | Self::Divida | Self::Tesouro => 1e9,
            Self::Populacao => 1e6,
            Self::ProducaoBem => 1e3,
            _ => 1.0,
        }
    }

    /// Nome da unidade já na escala do eixo (ex.: `US$ bi`).
    pub fn unidade_eixo(self) -> &'static str {
        match self {
            Self::Pib | Self::Divida | Self::Tesouro => "US$ bi",
            Self::Populacao => "mi de habitantes",
            Self::ProducaoBem => "mil unidades/dia",
            other => other.unidade(),
        }
    }
}

/// Cabeçalho fixo de tabela (coluna sem ordenação).
fn cabecalho_fixo(ui: &mut egui::Ui, nome: &str) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(nome.to_uppercase())
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(11.0)
                .strong()
                .color(ui_refinada::menu::TINTA_SUAVE),
        )
        .selectable(false),
    );
}

fn cabecalho_fixo_w(ui: &mut egui::Ui, nome: &str, largura: f32) {
    ui.add_sized(
        [largura, 20.0],
        egui::Label::new(
            egui::RichText::new(nome.to_uppercase())
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(11.0)
                .strong()
                .color(ui_refinada::menu::TINTA_SUAVE),
        )
        .selectable(false),
    );
}

/// Cabeçalho clicável: fundo azul-marinho na coluna ordenada, com triângulo desenhado pelo painter.
fn botao_coluna(ui: &mut egui::Ui, nome: &str, ativa: bool, desc: bool) -> bool {
    let resp = ui.add(
        egui::Button::new(
            egui::RichText::new(format!("{nome}   ").to_uppercase())
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(11.0)
                .strong()
                .color(if ativa { ui_refinada::menu::PAPEL } else { ui_refinada::menu::TINTA }),
        )
        .fill(if ativa { ui_refinada::menu::AZUL_MARINHO } else { ui_refinada::menu::PAPEL_ALT })
        .stroke(egui::Stroke::new(1.0, ui_refinada::menu::PAPEL_BORDA)),
    );
    if ativa {
        let r = resp.rect;
        let x = r.max.x - 9.0;
        let y = r.center().y;
        let tri: Vec<egui::Pos2> = if desc {
            vec![egui::pos2(x - 4.0, y - 2.5), egui::pos2(x + 4.0, y - 2.5), egui::pos2(x, y + 3.0)]
        } else {
            vec![egui::pos2(x, y - 3.0), egui::pos2(x + 4.0, y + 2.5), egui::pos2(x - 4.0, y + 2.5)]
        };
        ui.painter().add(egui::Shape::convex_polygon(tri, ui_refinada::menu::PAPEL, egui::Stroke::NONE));
    }
    resp.clicked()
}

fn botao_coluna_w(ui: &mut egui::Ui, nome: &str, largura: f32, ativa: bool, desc: bool) -> bool {
    let resp = ui.add_sized(
        [largura, 20.0],
        egui::Button::new(
            egui::RichText::new(format!("{nome}   ").to_uppercase())
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(11.0)
                .strong()
                .color(if ativa { ui_refinada::menu::PAPEL } else { ui_refinada::menu::TINTA }),
        )
        .fill(if ativa { ui_refinada::menu::AZUL_MARINHO } else { ui_refinada::menu::PAPEL_ALT })
        .stroke(egui::Stroke::new(1.0, ui_refinada::menu::PAPEL_BORDA)),
    );
    if ativa {
        let r = resp.rect;
        let x = r.max.x - 9.0;
        let y = r.center().y;
        let tri: Vec<egui::Pos2> = if desc {
            vec![egui::pos2(x - 4.0, y - 2.5), egui::pos2(x + 4.0, y - 2.5), egui::pos2(x, y + 3.0)]
        } else {
            vec![egui::pos2(x, y - 3.0), egui::pos2(x + 4.0, y + 2.5), egui::pos2(x - 4.0, y + 2.5)]
        };
        ui.painter().add(egui::Shape::convex_polygon(tri, ui_refinada::menu::PAPEL, egui::Stroke::NONE));
    }
    resp.clicked()
}

/// Registro compacto de um ponto mensal para um país (4 bytes por campo escalar).
#[derive(Debug, Clone, Default)]
pub struct PontoHistoricoPais {
    pub pib: f32,
    pub pib_per_capita: f32,
    pub populacao: f32,
    pub desemprego: f32,
    pub padrao_vida: f32,
    pub divida: f32,
    pub tesouro: f32,
    /// Produção por índice do bem no catálogo
    pub producao_bens: Vec<f32>,
}

/// Série histórica de um país com identificação e pontos mensais.
#[derive(Debug, Clone, Default)]
pub struct HistoricoPais {
    pub id: String,
    pub tag: String,
    pub nome: String,
    pub cor: egui::Color32,
    pub pontos: Vec<PontoHistoricoPais>,
}

/// Mês e ano do calendário da simulação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MesSimulado {
    pub ano: i32,
    pub mes: u8,
}

impl MesSimulado {
    pub fn formatar_curto(self) -> String {
        let mes_str = match self.mes {
            1 => "Jan",
            2 => "Fev",
            3 => "Mar",
            4 => "Abr",
            5 => "Mai",
            6 => "Jun",
            7 => "Jul",
            8 => "Ago",
            9 => "Set",
            10 => "Out",
            11 => "Nov",
            12 => "Dez",
            _ => "Mês",
        };
        format!("{}/{}", mes_str, self.ano)
    }
}

/// Item descritivo de um bem econômico do catálogo para seleção no gráfico.
#[derive(Debug, Clone)]
pub struct DescricaoBem {
    pub idx: usize,
    pub id: String,
    pub nome: String,
}

/// Categoria geopolítica de classificação estilo Victoria 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GrauPotencia {
    GrandePotencia, // Ranks 1-8
    PotenciaMaior,  // Ranks 9-20
    PotenciaMenor,  // Ranks 21+
}

impl GrauPotencia {
    pub fn rotulo(self) -> &'static str {
        match self {
            Self::GrandePotencia => "Grande Potência",
            Self::PotenciaMaior => "Potência Maior",
            Self::PotenciaMenor => "Potência Menor",
        }
    }

    pub fn badge_cor(self) -> egui::Color32 {
        match self {
            Self::GrandePotencia => ui_refinada::menu::OURO,
            Self::PotenciaMaior => ui_refinada::menu::AZUL_MARINHO,
            Self::PotenciaMenor => ui_refinada::menu::TINTA_SUAVE,
        }
    }

    /// Chave de glifo vetorial (painter) do grau, no lugar de emoji.
    pub fn glifo(self) -> &'static str {
        match self {
            Self::GrandePotencia => "coroa",
            Self::PotenciaMaior => "estrela",
            Self::PotenciaMenor => "caixa",
        }
    }
}

/// Avaliação de potência e prestígio de um país.
#[derive(Debug, Clone)]
pub struct EntradaRankingPotencia {
    pub rank_geral: usize,
    pub grau: GrauPotencia,
    pub id: String,
    pub tag: String,
    pub nome: String,
    pub cor: egui::Color32,
    pub pontuacao_total: f64,
    pub pontuacao_eco: f64,
    pub pontuacao_mil: f64,
    pub pontuacao_inf: f64,
    pub pib_anual: f64,
    pub populacao: f64,
    pub padrao_vida: f64,
    pub desemprego: f64,
    pub forca_militar: f64,
    pub tesouro: f64,
    pub divida: f64,
    pub esfera: &'static str,
}

/// Colunas da tabela geral ordenável do Ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColunaLedger {
    #[default]
    RankGeral,
    Nome,
    PontuacaoTotal,
    PontuacaoEco,
    Pib,
    PontuacaoMil,
    ForcaMil,
    PontuacaoInf,
    Populacao,
    PadraoVida,
    Desemprego,
    Tesouro,
    Divida,
}

/// Abas principais da interface de Estatísticas e Ledger (7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AbaLedger {
    #[default]
    GraficosHistoricos,
    RankingsPotencias,
    TabelaPaises,
    /// Rankings globais por categoria (Top 10 + tabela ordenável).
    RankingsCategoria,
}

// -------------------------------------------------------------------------------------------------
// Recurso de histórico e estado do Ledger
// -------------------------------------------------------------------------------------------------

/// Recurso ECS que mantém o histórico temporal compacto e as preferências de visualização.
#[derive(Resource)]
pub struct RecursoLedger {
    pub aberto: bool,
    pub aba_ativa: AbaLedger,
    pub metrica_grafico: MetricaGrafico,
    pub bem_selecionado_idx: usize,
    pub catalogo_bens: Vec<DescricaoBem>,
    /// Meses registrados no histórico
    pub meses: Vec<MesSimulado>,
    /// Histórico compactado por país: id -> HistoricoPais
    pub historico_paises: BTreeMap<String, HistoricoPais>,
    /// Países marcados para comparação no gráfico
    pub paises_selecionados: BTreeSet<String>,
    /// Filtro de texto para pesquisar países nos gráficos e no ledger
    pub filtro_busca: String,
    /// Último mês em que o snapshot foi gravado
    pub ultimo_mes_gravado: Option<MesSimulado>,
    /// Ordenação da tabela
    pub coluna_ordenacao: ColunaLedger,
    pub ordenacao_decrescente: bool,
    /// Filtro de esfera/bloco na tabela
    pub filtro_bloco: Option<String>,
    /// Cache de rankings calculados no último tick
    pub rankings_cache: Vec<EntradaRankingPotencia>,
    /// Rankings por categoria: categoria exibida no Top 10, ordenação da tabela e linhas calculadas.
    pub cat_selecionada: Categoria,
    pub cat_ordem: Categoria,
    pub cat_desc: bool,
    pub linhas_cat: Vec<LinhaCategoria>,
    pub top_economicas: Vec<EntradaRankingPotencia>,
    pub top_militares: Vec<EntradaRankingPotencia>,
    pub top_influencia: Vec<EntradaRankingPotencia>,
    pub tabela_cache: Vec<EntradaRankingPotencia>,
    pub tabela_cache_busca: String,
    pub tabela_cache_bloco: Option<String>,
    pub tabela_cache_col: ColunaLedger,
    pub tabela_cache_desc: bool,
    pub tabela_cache_versao: usize,
    pub lista_grafico_cache: Vec<(usize, String, String, String)>,
    pub lista_grafico_busca: String,
    pub lista_grafico_versao: usize,
}

impl Default for RecursoLedger {
    fn default() -> Self {
        Self {
            aberto: false,
            aba_ativa: AbaLedger::GraficosHistoricos,
            metrica_grafico: MetricaGrafico::Pib,
            bem_selecionado_idx: 0,
            catalogo_bens: Vec::new(),
            meses: Vec::new(),
            historico_paises: BTreeMap::new(),
            paises_selecionados: BTreeSet::new(),
            filtro_busca: String::new(),
            ultimo_mes_gravado: None,
            coluna_ordenacao: ColunaLedger::PontuacaoTotal,
            ordenacao_decrescente: true,
            filtro_bloco: None,
            rankings_cache: Vec::new(),
            cat_selecionada: Categoria::Pib,
            cat_ordem: Categoria::Pib,
            cat_desc: true,
            linhas_cat: Vec::new(),
            top_economicas: Vec::new(),
            top_militares: Vec::new(),
            top_influencia: Vec::new(),
            tabela_cache: Vec::new(),
            tabela_cache_busca: String::new(),
            tabela_cache_bloco: None,
            tabela_cache_col: ColunaLedger::PontuacaoTotal,
            tabela_cache_desc: true,
            tabela_cache_versao: 0,
            lista_grafico_cache: Vec::new(),
            lista_grafico_busca: String::new(),
            lista_grafico_versao: 0,
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Paleta de cores para gráficos
// -------------------------------------------------------------------------------------------------

/// Tintas de série para gráficos: todas escuras o bastante para ler sobre o papel creme.
const PALETA_CORES_GRAFICO: [egui::Color32; 16] = [
    egui::Color32::from_rgb(25, 55, 96),   // azul-marinho
    egui::Color32::from_rgb(140, 37, 30),  // vermelho cartaz
    egui::Color32::from_rgb(27, 110, 40),  // verde-escuro
    egui::Color32::from_rgb(162, 92, 8),   // ocre
    egui::Color32::from_rgb(23, 22, 20),   // tinta
    egui::Color32::from_rgb(90, 60, 120),  // ameixa
    egui::Color32::from_rgb(176, 90, 40),  // laranja queimado
    egui::Color32::from_rgb(30, 100, 110), // verde-petróleo
    egui::Color32::from_rgb(110, 110, 45), // oliva
    egui::Color32::from_rgb(105, 25, 45),  // bordô
    egui::Color32::from_rgb(60, 95, 140),  // azul-aço
    egui::Color32::from_rgb(120, 80, 35),  // sépia
    egui::Color32::from_rgb(40, 80, 60),   // floresta
    egui::Color32::from_rgb(75, 50, 90),   // uva
    egui::Color32::from_rgb(150, 60, 50),  // telha
    egui::Color32::from_rgb(45, 75, 90),   // azul-grafite
];

fn cor_para_indice(idx: usize) -> egui::Color32 {
    PALETA_CORES_GRAFICO[idx % PALETA_CORES_GRAFICO.len()]
}

// -------------------------------------------------------------------------------------------------
// Plugin Bevy
// -------------------------------------------------------------------------------------------------

pub struct EstatisticasPlugin;

impl Plugin for EstatisticasPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RecursoLedger>()
            .add_systems(
                Update,
                (
                    alternar_tecla_f7,
                    coletar_historico_mensal,
                    atualizar_rankings_sistema,
                )
                    .chain()
                    .run_if(in_state(crate::estado::AppState::Jogando)),
            )
            .add_systems(Update, (atualizar_rankings_categoria, tecla_7_na_pausa).run_if(crate::estado::em_partida))
            .add_systems(EguiPrimaryContextPass, desenhar_tela_ledger.run_if(crate::estado::em_partida));
    }
}

// -------------------------------------------------------------------------------------------------
// Sistemas de atualização e coleta
// -------------------------------------------------------------------------------------------------

fn alternar_tecla_f7(
    teclas: Res<ButtonInput<KeyCode>>,
    mut ledger: ResMut<RecursoLedger>,
) {
    if teclas.just_pressed(KeyCode::Digit7) || teclas.just_pressed(KeyCode::Numpad7) {
        ledger.aberto = !ledger.aberto;
    }
}

/// Coleta e armazena os dados mensais de forma compacta.
fn coletar_historico_mensal(
    mut ledger: ResMut<RecursoLedger>,
    relogio: Res<RecursoRelogio>,
    eco: Option<Res<EconomiaSim>>,
    mundo_geo: Res<RecursoMundo>,
    jogador: Option<Res<PaisJogador>>,
) {
    let Some(eco) = eco else { return };
    if eco.vista.ranking.is_empty() {
        return;
    }

    // Inicializa catálogo de bens caso esteja vazio
    if ledger.catalogo_bens.is_empty() {
        if let Some(cat) = &eco.vista.catalogo {
            ledger.catalogo_bens = cat
                .bens
                .iter()
                .enumerate()
                .map(|(i, b)| DescricaoBem {
                    idx: i,
                    id: b.id.clone(),
                    nome: b.nome.clone(),
                })
                .collect();
        } else if !eco.vista.bens.is_empty() {
            ledger.catalogo_bens = eco
                .vista
                .bens
                .iter()
                .enumerate()
                .map(|(i, b)| DescricaoBem {
                    idx: i,
                    id: b.id.clone(),
                    nome: b.nome.clone(),
                })
                .collect();
        }
    }

    // Garante que o país do jogador esteja selecionado por padrão na primeira carga
    if ledger.paises_selecionados.is_empty() {
        if let Some(j) = &jogador {
            let id_eco = eco.id_pais(&j.0).unwrap_or_else(|| j.0.to_lowercase());
            ledger.paises_selecionados.insert(id_eco);
        }
    }

    let mes_atual = MesSimulado {
        ano: relogio.data.ano,
        mes: relogio.data.mes,
    };

    // Só grava uma vez por mês
    if ledger.ultimo_mes_gravado == Some(mes_atual) {
        return;
    }

    // Grava o snapshot do mês
    ledger.meses.push(mes_atual);
    ledger.ultimo_mes_gravado = Some(mes_atual);

    let num_bens = ledger.catalogo_bens.len();

    for p in &eco.vista.ranking {
        let tag = if p.tag.is_empty() { p.id.to_uppercase() } else { p.tag.clone() };
        let cor_geo = mundo_geo
            .paises
            .get(&tag)
            .map(|gp| egui::Color32::from_rgb(gp.cor.0, gp.cor.1, gp.cor.2))
            .unwrap_or_else(|| egui::Color32::from_rgb(180, 180, 180));

        let hist = ledger.historico_paises.entry(p.id.clone()).or_insert_with(|| {
            HistoricoPais {
                id: p.id.clone(),
                tag: tag.clone(),
                nome: p.nome.clone(),
                cor: cor_geo,
                pontos: Vec::new(),
            }
        });

        hist.nome = p.nome.clone();
        hist.tag = tag;
        hist.cor = cor_geo;

        let pib_capita = if p.populacao > 0.0 {
            (p.pib_anual / p.populacao) as f32
        } else {
            0.0
        };

        let mut prod_bens_f32 = vec![0.0_f32; num_bens];
        for (i, &qtd) in p.producao_bens.iter().enumerate() {
            if i < num_bens {
                prod_bens_f32[i] = qtd as f32;
            }
        }

        hist.pontos.push(PontoHistoricoPais {
            pib: p.pib_anual as f32,
            pib_per_capita: pib_capita,
            populacao: p.populacao as f32,
            desemprego: p.desemprego as f32,
            padrao_vida: p.padrao_vida as f32,
            divida: p.divida as f32,
            tesouro: p.tesouro as f32,
            producao_bens: prod_bens_f32,
        });
    }
}

/// Atualiza o cálculo dos rankings de potência estilo Victoria 3.
fn atualizar_rankings_sistema(
    mut ledger: ResMut<RecursoLedger>,
    eco: Option<Res<EconomiaSim>>,
    mundo_geo: Res<RecursoMundo>,
    diplo: Option<Res<super::diplomacia_ui::DiploSim>>,
) {
    let Some(eco) = eco else { return };
    if eco.vista.ranking.is_empty() {
        return;
    }
    let eco_mudou = eco.is_changed();
    let diplo_mudou = diplo.as_ref().is_some_and(|d| d.is_changed());
    if !eco_mudou && !diplo_mudou && !ledger.rankings_cache.is_empty() {
        return;
    }

    // Calcula os maiores valores para normalização relativa
    let mut max_pib = 1.0_f64;
    let mut max_mil = 1.0_f64;

    for p in &eco.vista.ranking {
        if p.pib_anual > max_pib {
            max_pib = p.pib_anual;
        }
        if p.forca_militar > max_mil {
            max_mil = p.forca_militar;
        }
    }

    let mut entradas: Vec<EntradaRankingPotencia> = Vec::new();

    for p in &eco.vista.ranking {
        let tag = if p.tag.is_empty() { p.id.to_uppercase() } else { p.tag.clone() };
        let cor_geo = mundo_geo
            .paises
            .get(&tag)
            .map(|gp| egui::Color32::from_rgb(gp.cor.0, gp.cor.1, gp.cor.2))
            .unwrap_or_else(|| egui::Color32::from_rgb(180, 180, 180));

        // 1. Potência Econômica (escala até 1000 pts)
        // Combina PIB relativo (80%) com Padrão de Vida e População (20%)
        let ratio_pib = (p.pib_anual / max_pib).clamp(0.0, 1.0);
        let sol_bonus = (p.padrao_vida / 25.0).clamp(0.0, 1.5);
        let pont_eco = ratio_pib * 850.0 + sol_bonus * 150.0;

        // 2. Potência Militar (escala até 1000 pts)
        let ratio_mil = (p.forca_militar / max_mil).clamp(0.0, 1.0);
        let gasto_mil_est = if let Some(gp) = mundo_geo.paises.get(&tag) {
            gp.orcamento.despesa_militar_diaria * 365.0
        } else {
            0.0
        };
        let bonus_gasto = (gasto_mil_est / (max_pib * 0.10).max(1.0)).clamp(0.0, 1.0);
        let pont_mil = ratio_mil * 850.0 + bonus_gasto * 150.0;

        // 3. Potência de Influência / Geopolítica (escala até 1000 pts)
        // Baseada em esfera de influência, liderança de blocos e Conselho de Segurança ONU
        let mut pont_inf = 50.0; // Base soberana

        // Membros Permanentes do Conselho de Segurança da ONU ganham prestígio de Grande Potência
        let eh_membro_onu_p5 = matches!(tag.as_str(), "USA" | "SOV" | "GBR" | "FRA" | "CHI");
        if eh_membro_onu_p5 {
            pont_inf += 350.0;
        }

        // Bônus de esferas e alianças de sim_diplomacia
        let esfera_nome = if let Some(d) = &diplo {
            match d.0.esfera(&tag) {
                sim_diplomacia::Esfera::Eua => "Bloco Ocidental",
                sim_diplomacia::Esfera::Urss => "Bloco Soviético",
                sim_diplomacia::Esfera::Disputada => "Disputada",
                sim_diplomacia::Esfera::NaoAlinhado => "Não Alinhado",
            }
        } else if let Some(gp) = mundo_geo.paises.get(&tag) {
            match gp.alinhamento_inicial {
                simulacao::mapa::AlinhamentoGeopolitico::BlocoOcidental => "Bloco Ocidental",
                simulacao::mapa::AlinhamentoGeopolitico::BlocoSovietico => "Bloco Soviético",
                _ => "Não Alinhado",
            }
        } else {
            "Neutro"
        };

        if let Some(d) = &diplo {
            if tag == "USA" || tag == "SOV" {
                // As duas superpotências recebem pontos proporcionais à soma da influência que projetam
                let mut soma_inf = 0.0_f64;
                for p_alvo in &d.0.paises {
                    let (u, s) = d.0.influencia(&p_alvo.tag);
                    soma_inf += if tag == "USA" { u as f64 } else { s as f64 };
                }
                pont_inf += (soma_inf * 0.15).clamp(0.0, 450.0);
            } else {
                let pactos = d.0.pactos.iter().filter(|pac| pac.a == tag || pac.b == tag).count();
                pont_inf += (pactos as f64 * 40.0).clamp(0.0, 200.0);
            }
        }

        // Bônus colonial/suserano: potências com súditos/colônias
        let num_suditos = mundo_geo
            .paises
            .values()
            .filter(|outro| outro.suserano.as_deref() == Some(&tag))
            .count();
        pont_inf += (num_suditos as f64 * 35.0).clamp(0.0, 250.0);

        let pont_total = pont_eco * 0.40 + pont_mil * 0.35 + pont_inf * 0.25;

        entradas.push(EntradaRankingPotencia {
            rank_geral: 0,
            grau: GrauPotencia::PotenciaMenor,
            id: p.id.clone(),
            tag,
            nome: p.nome.clone(),
            cor: cor_geo,
            pontuacao_total: pont_total,
            pontuacao_eco: pont_eco,
            pontuacao_mil: pont_mil,
            pontuacao_inf: pont_inf,
            pib_anual: p.pib_anual,
            populacao: p.populacao,
            padrao_vida: p.padrao_vida,
            desemprego: p.desemprego,
            forca_militar: p.forca_militar,
            tesouro: p.tesouro,
            divida: p.divida,
            esfera: esfera_nome,
        });
    }

    // Ordena do maior para o menor
    entradas.sort_by(|a, b| b.pontuacao_total.total_cmp(&a.pontuacao_total));

    // Atribui os ranks e os graus estilo Victoria 3
    for (i, e) in entradas.iter_mut().enumerate() {
        let pos = i + 1;
        e.rank_geral = pos;
        e.grau = if pos <= 8 {
            GrauPotencia::GrandePotencia
        } else if pos <= 20 {
            GrauPotencia::PotenciaMaior
        } else {
            GrauPotencia::PotenciaMenor
        };
    }

    let mut eco_top = entradas.clone();
    eco_top.sort_by(|a, b| b.pib_anual.total_cmp(&a.pib_anual));
    eco_top.truncate(10);
    ledger.top_economicas = eco_top;

    let mut mil_top = entradas.clone();
    mil_top.sort_by(|a, b| b.pontuacao_mil.total_cmp(&a.pontuacao_mil));
    mil_top.truncate(10);
    ledger.top_militares = mil_top;

    let mut inf_top = entradas.clone();
    inf_top.sort_by(|a, b| b.pontuacao_inf.total_cmp(&a.pontuacao_inf));
    inf_top.truncate(10);
    ledger.top_influencia = inf_top;

    ledger.rankings_cache = entradas;
    ledger.tabela_cache.clear();
}

// -------------------------------------------------------------------------------------------------
// Interface Gráfica egui (7)
// -------------------------------------------------------------------------------------------------

fn desenhar_tela_ledger(
    mut contextos: EguiContexts,
    mut ledger: ResMut<RecursoLedger>,
    jogador: Option<Res<PaisJogador>>,
    eco: Option<Res<EconomiaSim>>,
) -> Result {
    if !ledger.aberto {
        return Ok(());
    }

    let ctx = contextos.ctx_mut()?;
    let tag_jogador = jogador.map(|j| j.0.clone()).unwrap_or_default();
    let id_jogador_eco = eco.as_ref().and_then(|e| e.id_pais(&tag_jogador)).unwrap_or_else(|| tag_jogador.to_lowercase());

    let mut aberto = ledger.aberto;

    ui_refinada::janela_dossie("ESTATÍSTICAS & LEDGER GERAL (7)", None)
        .default_pos([16.0, 96.0])
        .default_width(1200.0)
        .default_height(720.0)
        .min_width(900.0)
        .min_height(550.0)
        .show(ctx, |ui| {
            ui_refinada::faixa_titulo(ui, "Estatísticas & Ledger Mundial", Some("7"), Some(&mut aberto));
            ui.horizontal(|ui| {
                let rotulo_aba1 = format!("Gráficos · {}", ledger.metrica_grafico.titulo());
                if ui_refinada::aba_glifo(ui, ledger.aba_ativa == AbaLedger::GraficosHistoricos, Some("grafico"), &rotulo_aba1) {
                    ledger.aba_ativa = AbaLedger::GraficosHistoricos;
                }
                if ui_refinada::aba_glifo(ui, ledger.aba_ativa == AbaLedger::RankingsPotencias, Some("coroa"), "Grandes Potências (Top 8)") {
                    ledger.aba_ativa = AbaLedger::RankingsPotencias;
                }
                if ui_refinada::aba_glifo(ui, ledger.aba_ativa == AbaLedger::TabelaPaises, Some("caixa"), "Ledger de Países") {
                    ledger.aba_ativa = AbaLedger::TabelaPaises;
                }
                if ui_refinada::aba_glifo(ui, ledger.aba_ativa == AbaLedger::RankingsCategoria, Some("estrela"), "Rankings Globais") {
                    ledger.aba_ativa = AbaLedger::RankingsCategoria;
                }
            });

            ui.separator();

            match ledger.aba_ativa {
                AbaLedger::GraficosHistoricos => {
                    desenhar_aba_graficos(ui, &mut ledger, &id_jogador_eco);
                }
                AbaLedger::RankingsPotencias => {
                    desenhar_aba_potencias(ui, &ledger, &id_jogador_eco);
                }
                AbaLedger::TabelaPaises => {
                    desenhar_aba_tabela(ui, &mut ledger, &id_jogador_eco);
                }
                AbaLedger::RankingsCategoria => {
                    desenhar_aba_categorias(ui, &mut ledger, &tag_jogador);
                }
            }
        });

    ledger.aberto = aberto;
    Ok(())
}

// -------------------------------------------------------------------------------------------------
// Aba 1: Gráficos Históricos (egui_plot)
// -------------------------------------------------------------------------------------------------

fn desenhar_aba_graficos(
    ui: &mut egui::Ui,
    ledger: &mut RecursoLedger,
    id_jogador: &str,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("MÉTRICA").small().strong().color(ui_refinada::menu::TINTA_SUAVE));
        for metrica in MetricaGrafico::TODAS {
            if ui_refinada::aba_glifo(ui, ledger.metrica_grafico == metrica, Some("grafico"), metrica.titulo()) {
                ledger.metrica_grafico = metrica;
            }
        }
    });

    if ledger.metrica_grafico == MetricaGrafico::ProducaoBem {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.strong("Bem de Produção:");
            let nome_atual = ledger
                .catalogo_bens
                .get(ledger.bem_selecionado_idx)
                .map(|b| b.nome.as_str())
                .unwrap_or("Selecione um bem");

            egui::ComboBox::from_id_salt("cb_bem_estatisticas")
                .selected_text(nome_atual)
                .width(260.0)
                .show_ui(ui, |ui| {
                    for b in &ledger.catalogo_bens {
                        ui.selectable_value(&mut ledger.bem_selecionado_idx, b.idx, &b.nome);
                    }
                });
        });
    }

    ui.add_space(4.0);
    ui.separator();

    // Layout em 2 colunas: Esquerda (Seleção de Países) e Direita (Gráfico egui_plot)
    ui.columns(2, |cols| {
        // Coluna 1: Painel de Seleção de Países (largura fixa ~280px)
        cols[0].vertical(|ui| {
            ui.strong("Países para Comparação:");
            ui.horizontal(|ui| {
                if ui_refinada::botao_estilo_secundario(ui, "Meu País").on_hover_text("Seleciona apenas o seu país").clicked() {
                    ledger.paises_selecionados.clear();
                    ledger.paises_selecionados.insert(id_jogador.to_string());
                }
                if ui_refinada::botao_estilo_secundario(ui, "Top 8").on_hover_text("Seleciona as 8 Grandes Potências").clicked() {
                    ledger.paises_selecionados.clear();
                    for e in ledger.rankings_cache.iter().take(8) {
                        ledger.paises_selecionados.insert(e.id.clone());
                    }
                }
                if ui_refinada::botao_estilo_secundario(ui, "Limpar").clicked() {
                    ledger.paises_selecionados.clear();
                }
            });

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui_refinada::glifo_ui(ui, "lupa", 14.0, None);
                ui.text_edit_singleline(&mut ledger.filtro_busca);
            });

            ui.add_space(4.0);
            let termo = ledger.filtro_busca.trim().to_lowercase();

            // Cache dos países para o gráfico
            if ledger.lista_grafico_cache.is_empty()
                || ledger.lista_grafico_busca != termo
                || ledger.lista_grafico_versao != ledger.historico_paises.len()
            {
                ledger.lista_grafico_busca = termo.clone();
                ledger.lista_grafico_versao = ledger.historico_paises.len();
                let mut paises_lista: Vec<_> = ledger.historico_paises.values().collect();
                paises_lista.sort_by(|a, b| a.nome.cmp(&b.nome));
                let mut filtrada = Vec::new();
                for (idx, p) in paises_lista.iter().enumerate() {
                    if !termo.is_empty()
                        && !p.nome.to_lowercase().contains(&termo)
                        && !p.tag.to_lowercase().contains(&termo)
                    {
                        continue;
                    }
                    filtrada.push((idx, p.id.clone(), p.nome.clone(), p.tag.clone()));
                }
                ledger.lista_grafico_cache = filtrada;
            }

            let total_itens = ledger.lista_grafico_cache.len();
            let mut alternar_selecao = None;
            egui::ScrollArea::vertical()
                .id_salt("scroll_paises_grafico")
                .max_height(480.0)
                .show_rows(ui, 22.0, total_itens, |ui, row_range| {
                    for i in row_range {
                        let (idx, ref pid, ref pnome, ref ptag) = ledger.lista_grafico_cache[i];
                        let marcado = ledger.paises_selecionados.contains(pid);
                        let eh_jogador = pid == id_jogador;
                        let cor_item = cor_para_indice(idx);

                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, cor_item);

                            let mut checked = marcado;
                            let rotulo = if eh_jogador {
                                format!("{pnome} (Você)")
                            } else {
                                format!("{pnome} ({ptag})")
                            };

                            if ui.checkbox(&mut checked, rotulo).changed() {
                                alternar_selecao = Some((pid.clone(), checked));
                            }
                        });
                    }
                });

            if let Some((pid, checked)) = alternar_selecao {
                if checked {
                    ledger.paises_selecionados.insert(pid);
                } else {
                    ledger.paises_selecionados.remove(&pid);
                }
            }
        });

        // Coluna 2: Gráfico Histórico Interativo com egui_plot
        cols[1].vertical(|ui| {
            let metrica = ledger.metrica_grafico;
            let bem_idx = ledger.bem_selecionado_idx;
            let nome_metrica = if metrica == MetricaGrafico::ProducaoBem {
                let nome_bem = ledger.catalogo_bens.get(bem_idx).map(|b| b.nome.as_str()).unwrap_or("Bem");
                format!("Produção: {}", nome_bem)
            } else {
                metrica.titulo().to_string()
            };

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(&nome_metrica)
                        .family(ui_refinada::familia_oswald(ui.ctx()))
                        .size(17.0)
                        .strong()
                        .color(ui_refinada::menu::AZUL_MARINHO),
                );
                ui.weak(format!("(em {}) · {} meses simulados", metrica.unidade_eixo(), ledger.meses.len()));
            });

            if ledger.meses.is_empty() {
                ui.label("Aguardando o primeiro fechamento mensal da simulação econômica...");
                return;
            }

            if ledger.paises_selecionados.is_empty() {
                ui.label("Nenhum país selecionado para visualização. Marque ao menos um país na lista ao lado.");
                return;
            }

            let meses_nomes: Vec<String> = ledger.meses.iter().map(|m| m.formatar_curto()).collect();

            // Monta as linhas para cada país selecionado
            let mut linhas: Vec<(String, egui::Color32, PlotPoints<'static>)> = Vec::new();
            let escala = metrica.escala();

            for (idx, p_id) in ledger.paises_selecionados.iter().enumerate() {
                if let Some(hist) = ledger.historico_paises.get(p_id) {
                    let cor = cor_para_indice(idx);
                    let rotulo = format!("{} ({})", hist.nome, hist.tag);

                    let pontos: PlotPoints<'static> = hist
                        .pontos
                        .iter()
                        .enumerate()
                        .map(|(mes_i, pt)| {
                            let val: f64 = match metrica {
                                MetricaGrafico::Pib => pt.pib as f64,
                                MetricaGrafico::PibPerCapita => pt.pib_per_capita as f64,
                                MetricaGrafico::Populacao => pt.populacao as f64,
                                MetricaGrafico::Desemprego => (pt.desemprego * 100.0) as f64,
                                MetricaGrafico::PadraoVida => pt.padrao_vida as f64,
                                MetricaGrafico::Divida => pt.divida as f64,
                                MetricaGrafico::Tesouro => pt.tesouro as f64,
                                MetricaGrafico::ProducaoBem => {
                                    pt.producao_bens.get(bem_idx).copied().unwrap_or(0.0) as f64
                                }
                            };
                            [mes_i as f64, val / escala]
                        })
                        .collect();

                    linhas.push((rotulo, cor, pontos));
                }
            }

            let num_meses = ledger.meses.len();

            Plot::new("plot_historico_ledger")
                .height(420.0)
                .grid_color(ui_refinada::menu::PAPEL_BORDA)
                .y_axis_label(metrica.unidade_eixo())
                .legend(Legend::default().position(Corner::RightTop))
                .x_axis_label("mês da simulação")
                .allow_scroll(true)
                .allow_zoom(true)
                .allow_drag(true)
                .show(ui, |plot_ui| {
                    for (rotulo, cor, pts) in linhas {
                        plot_ui.line(
                            Line::new(rotulo, pts)
                                .color(cor)
                                .width(2.5),
                        );
                    }
                });

            ui.horizontal_wrapped(|ui| {
                if let Some(primeiro) = meses_nomes.first() {
                    ui.weak(format!("Início: {}", primeiro));
                }
                if let Some(ultimo) = meses_nomes.last() {
                    ui.weak(format!("· Data atual: {} (mês {})", ultimo, num_meses));
                }
                ui.weak("· roda do mouse = zoom, arrastar = navegar");
            });
        });
    });
}

// -------------------------------------------------------------------------------------------------
// Aba 2: Rankings de Potência Estilo Victoria 3 (Top 8 Grandes Potências)
// -------------------------------------------------------------------------------------------------

fn desenhar_aba_potencias(
    ui: &mut egui::Ui,
    ledger: &RecursoLedger,
    id_jogador: &str,
) {
    ui.horizontal_wrapped(|ui| {
        ui_refinada::glifo_ui(ui, "coroa", 20.0, Some(ui_refinada::menu::OURO));
        ui.label(
            egui::RichText::new("GRANDES POTÊNCIAS DE 1945")
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(19.0)
                .strong()
                .color(ui_refinada::menu::AZUL_MARINHO),
        );
        ui.weak("— classificação geopolítica (Top 8)");
    });
    ui.weak("A pontuação combina poder econômico (PIB e padrão de vida), poder militar (forças armadas e gastos) e projeção de influência.");

    ui.add_space(8.0);

    // Normalizadores das barras (máximo entre os oito primeiros)
    let topo: Vec<&EntradaRankingPotencia> = ledger.rankings_cache.iter().take(8).collect();
    let max_de = |get: fn(&EntradaRankingPotencia) -> f64| {
        topo.iter().map(|p| get(p)).fold(0.0f64, f64::max).max(1.0)
    };
    let max_total = max_de(|p| p.pontuacao_total);
    let max_eco = max_de(|p| p.pontuacao_eco);
    let max_mil = max_de(|p| p.pontuacao_mil);
    let max_inf = max_de(|p| p.pontuacao_inf);

    egui::ScrollArea::vertical()
        .id_salt("scroll_potencias")
        .show(ui, |ui| {
            ui.columns(4, |cols| {
                for (i, p) in ledger.rankings_cache.iter().take(8).enumerate() {
                    let col_idx = i % 4;
                    let eh_jogador = p.id == id_jogador;
                    let cor_borda = if eh_jogador {
                        ui_refinada::menu::VERMELHO
                    } else {
                        ui_refinada::menu::PAPEL_BORDA
                    };
                    egui::Frame::new()
                        .fill(ui_refinada::menu::PAPEL_CARD)
                        .stroke(egui::Stroke::new(if eh_jogador { 2.0 } else { 1.0 }, cor_borda))
                        .inner_margin(egui::Margin::symmetric(9, 6))
                        .show(&mut cols[col_idx], |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("#{}", p.rank_geral))
                                        .family(ui_refinada::familia_oswald(ui.ctx()))
                                        .size(16.0)
                                        .strong()
                                        .color(ui_refinada::menu::AZUL_MARINHO)
                                        .background_color(ui_refinada::menu::PAPEL_CLARO),
                                );
                                ui_refinada::glifo_ui(ui, p.grau.glifo(), 16.0, Some(p.grau.badge_cor()));
                                ui.label(
                                    egui::RichText::new(&p.nome)
                                        .family(ui_refinada::familia_oswald(ui.ctx()))
                                        .size(15.0)
                                        .strong()
                                        .color(ui_refinada::menu::TINTA),
                                );
                                let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 11.0), egui::Sense::hover());
                                ui.painter().rect_filled(r, 0.0, p.cor);
                                ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.0, ui_refinada::menu::TINTA), egui::StrokeKind::Inside);
                            });
                            ui.label(egui::RichText::new(p.grau.rotulo()).small().color(ui_refinada::menu::TINTA_SUAVE));
                            if eh_jogador {
                                ui.colored_label(ui_refinada::menu::VERMELHO, egui::RichText::new("SEU PAÍS").small().strong());
                            }
                            ui_refinada::linha_decorativa(ui);
                            let barra = |ui: &mut egui::Ui, rot: &str, v: f64, m: f64, cor: egui::Color32| {
                                ui.horizontal(|ui| {
                                    ui.add_sized([58.0, 12.0], egui::Label::new(egui::RichText::new(rot).size(10.0).color(ui_refinada::menu::TINTA_SUAVE)));
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(72.0, 9.0), egui::Sense::hover());
                                    let pt = ui.painter_at(rect);
                                    pt.rect_filled(rect, 0.0, ui_refinada::menu::PAPEL_CLARO);
                                    let cheio = egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + rect.width() * (v / m).clamp(0.0, 1.0) as f32, rect.max.y));
                                    pt.rect_filled(cheio, 0.0, cor);
                                    pt.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, ui_refinada::menu::TINTA_SUAVE), egui::StrokeKind::Inside);
                                    ui.add_sized([46.0, 12.0], egui::Label::new(egui::RichText::new(format!("{:.0}", v)).small()));
                                });
                            };
                            barra(ui, "Total", p.pontuacao_total, max_total, ui_refinada::menu::TINTA);
                            barra(ui, "Economia", p.pontuacao_eco, max_eco, ui_refinada::menu::AZUL_MARINHO);
                            barra(ui, "Militar", p.pontuacao_mil, max_mil, ui_refinada::menu::VERMELHO);
                            barra(ui, "Influência", p.pontuacao_inf, max_inf, ui_refinada::menu::SUCESSO);
                            ui.add_space(2.0);
                            ui.small(format!("PIB · {}", fmt_din(p.pib_anual)));
                            ui.small(format!("População · {}", fmt_pop(p.populacao as u64)));
                            ui.small(format!("Padrão de vida · {:.1}", p.padrao_vida));
                        });
                    cols[col_idx].add_space(8.0);
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.label(
                egui::RichText::new("PILARES COMPARATIVOS DE POTÊNCIA (TOP 10 POR DIMENSÃO)")
                    .family(ui_refinada::familia_oswald(ui.ctx()))
                    .size(16.0)
                    .strong()
                    .color(ui_refinada::menu::AZUL_MARINHO),
            );

            ui.columns(3, |cols| {
                let cab = |ui: &mut egui::Ui, glifo: &str, t: &str| {
                    ui.horizontal(|ui| {
                        ui_refinada::glifo_ui(ui, glifo, 15.0, None);
                        ui.label(
                            egui::RichText::new(t)
                                .family(ui_refinada::familia_oswald(ui.ctx()))
                                .size(14.0)
                                .strong()
                                .color(ui_refinada::menu::TINTA),
                        );
                    });
                    ui_refinada::linha_decorativa(ui);
                };
                // Coluna 1: Maiores Potências Econômicas
                cols[0].group(|ui| {
                    cab(ui, "moeda", "Potência Econômica (PIB)");
                    for (i, p) in ledger.top_economicas.iter().take(10).enumerate() {
                        let eh_jog = p.id == id_jogador;
                        let txt = format!("{}. {} — {}", i + 1, p.nome, fmt_din(p.pib_anual));
                        if eh_jog {
                            ui.colored_label(ui_refinada::menu::SUCESSO, egui::RichText::new(txt).strong());
                        } else {
                            ui.label(txt);
                        }
                    }
                });

                // Coluna 2: Maiores Potências Militares
                cols[1].group(|ui| {
                    cab(ui, "canhao", "Potência Militar (Força Armada)");
                    for (i, p) in ledger.top_militares.iter().take(10).enumerate() {
                        let eh_jog = p.id == id_jogador;
                        let txt = format!("{}. {} — {:.0} pts", i + 1, p.nome, p.pontuacao_mil);
                        if eh_jog {
                            ui.colored_label(ui_refinada::menu::SUCESSO, egui::RichText::new(txt).strong());
                        } else {
                            ui.label(txt);
                        }
                    }
                });

                // Coluna 3: Maior Influência & Geopolítica
                cols[2].group(|ui| {
                    cab(ui, "torre", "Projeção de Influência");
                    for (i, p) in ledger.top_influencia.iter().take(10).enumerate() {
                        let eh_jog = p.id == id_jogador;
                        let txt = format!("{}. {} — {:.0} pts ({})", i + 1, p.nome, p.pontuacao_inf, p.esfera);
                        if eh_jog {
                            ui.colored_label(ui_refinada::menu::SUCESSO, egui::RichText::new(txt).strong());
                        } else {
                            ui.label(txt);
                        }
                    }
                });
            });
        });
}

// -------------------------------------------------------------------------------------------------
// Aba 3: Ledger Geral (Tabela Ordenável de Todos os Países)
// -------------------------------------------------------------------------------------------------

fn desenhar_aba_tabela(
    ui: &mut egui::Ui,
    ledger: &mut RecursoLedger,
    id_jogador: &str,
) {
    ui.horizontal(|ui| {
        ui.strong("Filtro:");
        ui.text_edit_singleline(&mut ledger.filtro_busca);
        if ui.button("Limpar").clicked() {
            ledger.filtro_busca.clear();
        }

        ui.separator();
        ui.label("Bloco Geopolítico:");
        if ui.selectable_label(ledger.filtro_bloco.is_none(), "Todos").clicked() {
            ledger.filtro_bloco = None;
        }
        for bloco in &["Bloco Ocidental", "Bloco Soviético", "Não Alinhado"] {
            let sel = ledger.filtro_bloco.as_deref() == Some(*bloco);
            if ui.selectable_label(sel, *bloco).clicked() {
                ledger.filtro_bloco = Some(bloco.to_string());
            }
        }
    });

    ui.add_space(4.0);

    let precisa_recalcular = ledger.tabela_cache.is_empty()
        || ledger.tabela_cache_busca != ledger.filtro_busca
        || ledger.tabela_cache_bloco != ledger.filtro_bloco
        || ledger.tabela_cache_col != ledger.coluna_ordenacao
        || ledger.tabela_cache_desc != ledger.ordenacao_decrescente;

    if precisa_recalcular {
        let mut lista = ledger.rankings_cache.clone();

        // Filtra busca e bloco
        let termo = ledger.filtro_busca.trim().to_lowercase();
        if !termo.is_empty() {
            lista.retain(|p| p.nome.to_lowercase().contains(&termo) || p.tag.to_lowercase().contains(&termo));
        }
        if let Some(b) = &ledger.filtro_bloco {
            lista.retain(|p| p.esfera == b.as_str());
        }

        // Ordenação
        let col = ledger.coluna_ordenacao;
        let desc = ledger.ordenacao_decrescente;

        lista.sort_by(|a, b| {
            let cmp = match col {
                ColunaLedger::RankGeral => a.rank_geral.cmp(&b.rank_geral),
                ColunaLedger::Nome => a.nome.cmp(&b.nome),
                ColunaLedger::PontuacaoTotal => a.pontuacao_total.total_cmp(&b.pontuacao_total),
                ColunaLedger::PontuacaoEco => a.pontuacao_eco.total_cmp(&b.pontuacao_eco),
                ColunaLedger::Pib => a.pib_anual.total_cmp(&b.pib_anual),
                ColunaLedger::PontuacaoMil => a.pontuacao_mil.total_cmp(&b.pontuacao_mil),
                ColunaLedger::ForcaMil => a.forca_militar.total_cmp(&b.forca_militar),
                ColunaLedger::PontuacaoInf => a.pontuacao_inf.total_cmp(&b.pontuacao_inf),
                ColunaLedger::Populacao => a.populacao.total_cmp(&b.populacao),
                ColunaLedger::PadraoVida => a.padrao_vida.total_cmp(&b.padrao_vida),
                ColunaLedger::Desemprego => a.desemprego.total_cmp(&b.desemprego),
                ColunaLedger::Tesouro => a.tesouro.total_cmp(&b.tesouro),
                ColunaLedger::Divida => a.divida.total_cmp(&b.divida),
            };
            if desc { cmp.reverse() } else { cmp }
        });

        ledger.tabela_cache_busca = ledger.filtro_busca.clone();
        ledger.tabela_cache_bloco = ledger.filtro_bloco.clone();
        ledger.tabela_cache_col = ledger.coluna_ordenacao;
        ledger.tabela_cache_desc = ledger.ordenacao_decrescente;
        ledger.tabela_cache = lista;
    }

    let col_ord = ledger.coluna_ordenacao;
    let desc_ord = ledger.ordenacao_decrescente;
    let mut nova_col_ord = None;

    // Cabeçalho fixo da tabela
    ui.horizontal(|ui| {
        if botao_coluna_w(ui, "Rank", 50.0, col_ord == ColunaLedger::RankGeral, desc_ord) { nova_col_ord = Some(ColunaLedger::RankGeral); }
        if botao_coluna_w(ui, "País", 180.0, col_ord == ColunaLedger::Nome, desc_ord) { nova_col_ord = Some(ColunaLedger::Nome); }
        cabecalho_fixo_w(ui, "Grau", 110.0);
        if botao_coluna_w(ui, "Pontos", 65.0, col_ord == ColunaLedger::PontuacaoTotal, desc_ord) { nova_col_ord = Some(ColunaLedger::PontuacaoTotal); }
        if botao_coluna_w(ui, "PIB", 80.0, col_ord == ColunaLedger::Pib, desc_ord) { nova_col_ord = Some(ColunaLedger::Pib); }
        if botao_coluna_w(ui, "Militar", 65.0, col_ord == ColunaLedger::PontuacaoMil, desc_ord) { nova_col_ord = Some(ColunaLedger::PontuacaoMil); }
        if botao_coluna_w(ui, "Influência", 75.0, col_ord == ColunaLedger::PontuacaoInf, desc_ord) { nova_col_ord = Some(ColunaLedger::PontuacaoInf); }
        if botao_coluna_w(ui, "População", 80.0, col_ord == ColunaLedger::Populacao, desc_ord) { nova_col_ord = Some(ColunaLedger::Populacao); }
        if botao_coluna_w(ui, "SoL", 50.0, col_ord == ColunaLedger::PadraoVida, desc_ord) { nova_col_ord = Some(ColunaLedger::PadraoVida); }
        if botao_coluna_w(ui, "Desemp.", 65.0, col_ord == ColunaLedger::Desemprego, desc_ord) { nova_col_ord = Some(ColunaLedger::Desemprego); }
        if botao_coluna_w(ui, "Tesouro", 80.0, col_ord == ColunaLedger::Tesouro, desc_ord) { nova_col_ord = Some(ColunaLedger::Tesouro); }
        if botao_coluna_w(ui, "Dívida", 80.0, col_ord == ColunaLedger::Divida, desc_ord) { nova_col_ord = Some(ColunaLedger::Divida); }
        cabecalho_fixo_w(ui, "Esfera", 120.0);
    });
    ui.separator();

    let total_linhas = ledger.tabela_cache.len();
    egui::ScrollArea::vertical()
        .id_salt("scroll_tabela_ledger")
        .max_height(580.0)
        .show_rows(ui, 22.0, total_linhas, |ui, row_range| {
            for idx in row_range {
                let p = &ledger.tabela_cache[idx];
                let eh_jogador = p.id == id_jogador;
                let cor_destaque = if eh_jogador { ui_refinada::menu::VERMELHO } else { ui_refinada::menu::TINTA };

                ui.horizontal(|ui| {
                    ui.add_sized([50.0, 20.0], egui::Label::new(egui::RichText::new(format!("#{}", p.rank_geral)).color(cor_destaque)));

                    ui.allocate_ui_with_layout(egui::vec2(180.0, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 10.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, 0.0, p.cor);
                        ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.0, ui_refinada::menu::TINTA_SUAVE), egui::StrokeKind::Inside);
                        if eh_jogador {
                            ui.colored_label(cor_destaque, egui::RichText::new(format!("{} ({}) · você", p.nome, p.tag)).strong());
                        } else {
                            ui.colored_label(cor_destaque, format!("{} ({})", p.nome, p.tag));
                        }
                    });

                    ui.allocate_ui_with_layout(egui::vec2(110.0, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui_refinada::glifo_ui(ui, p.grau.glifo(), 13.0, Some(p.grau.badge_cor()));
                        ui.colored_label(ui_refinada::menu::TINTA_SUAVE, p.grau.rotulo());
                    });

                    ui.add_sized([65.0, 20.0], egui::Label::new(egui::RichText::new(format!("{:.0}", p.pontuacao_total)).color(cor_destaque).strong()));
                    ui.add_sized([80.0, 20.0], egui::Label::new(fmt_din(p.pib_anual)));
                    ui.add_sized([65.0, 20.0], egui::Label::new(format!("{:.0}", p.pontuacao_mil)));
                    ui.add_sized([75.0, 20.0], egui::Label::new(format!("{:.0}", p.pontuacao_inf)));
                    ui.add_sized([80.0, 20.0], egui::Label::new(fmt_pop(p.populacao as u64)));
                    ui.add_sized([50.0, 20.0], egui::Label::new(format!("{:.1}", p.padrao_vida)));
                    ui.add_sized([65.0, 20.0], egui::Label::new(format!("{:.1}%", p.desemprego * 100.0)));
                    ui.add_sized([80.0, 20.0], egui::Label::new(fmt_din(p.tesouro)));
                    ui.add_sized([80.0, 20.0], egui::Label::new(fmt_din(p.divida)));
                    ui.add_sized([120.0, 20.0], egui::Label::new(p.esfera));
                });
            }
        });

    if let Some(nova_col) = nova_col_ord {
        if ledger.coluna_ordenacao == nova_col {
            ledger.ordenacao_decrescente = !ledger.ordenacao_decrescente;
        } else {
            ledger.coluna_ordenacao = nova_col;
            ledger.ordenacao_decrescente = true;
        }
        ledger.tabela_cache.clear();
    }
}

// -------------------------------------------------------------------------------------------------
// Aba 4: Rankings globais por categoria
// -------------------------------------------------------------------------------------------------

/// Categorias de ranking (todas ordenadas do maior valor para o menor por padrão).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Categoria {
    Exportacoes,
    Importacoes,
    SaldoComercial,
    Populacao,
    #[default]
    Pib,
    PibPerCapita,
    Crescimento,
    ForcaMilitar,
    Industria,
    Tecnologia,
    PadraoVida,
    Desemprego,
    Divida,
    Influencia,
    Turismo,
}

impl Categoria {
    pub const TODAS: [Categoria; 15] = [
        Self::Pib,
        Self::PibPerCapita,
        Self::Crescimento,
        Self::Populacao,
        Self::Exportacoes,
        Self::Importacoes,
        Self::SaldoComercial,
        Self::ForcaMilitar,
        Self::Industria,
        Self::Tecnologia,
        Self::PadraoVida,
        Self::Desemprego,
        Self::Divida,
        Self::Influencia,
        Self::Turismo,
    ];

    pub fn nome(self) -> &'static str {
        match self {
            Self::Exportacoes => "Exportações",
            Self::Importacoes => "Importações",
            Self::SaldoComercial => "Saldo comercial",
            Self::Populacao => "População",
            Self::Pib => "PIB",
            Self::PibPerCapita => "PIB per capita",
            Self::Crescimento => "Crescimento",
            Self::ForcaMilitar => "Força militar",
            Self::Industria => "Produção industrial",
            Self::Tecnologia => "Tecnologia",
            Self::PadraoVida => "Padrão de vida",
            Self::Desemprego => "Desemprego",
            Self::Divida => "Dívida",
            Self::Influencia => "Influência",
            Self::Turismo => "Turismo",
        }
    }

    pub fn formatar(self, v: f64) -> String {
        match self {
            Self::Populacao => fmt_pop(v.max(0.0) as u64),
            Self::Exportacoes | Self::Importacoes | Self::SaldoComercial | Self::Pib | Self::Divida | Self::Turismo => format!("US$ {}", fmt_din(v)),
            Self::PibPerCapita => format!("US$ {:.0}", v),
            Self::Crescimento => format!("{:+.1}%", v),
            Self::Desemprego => format!("{:.1}%", v),
            Self::ForcaMilitar | Self::Industria | Self::Influencia => fmt_pop(v.max(0.0) as u64),
            Self::Tecnologia => format!("{:.0} techs", v),
            Self::PadraoVida => format!("{:.1}", v),
        }
    }
}

/// Valores de todas as categorias de um país (indexados por `Categoria as usize`).
#[derive(Debug, Clone)]
pub struct LinhaCategoria {
    pub tag: String,
    pub nome: String,
    pub cor: egui::Color32,
    pub valores: [f64; 15],
}

impl LinhaCategoria {
    pub fn valor(&self, c: Categoria) -> f64 {
        self.valores[c as usize]
    }
}

fn tecla_7_na_pausa(teclas: Res<ButtonInput<KeyCode>>, estado: Res<State<crate::estado::AppState>>, mut ledger: ResMut<RecursoLedger>) {
    // Em Jogando a tecla 7 já é tratada por `alternar_tecla_f7`; em Pausado precisa funcionar também.
    if *estado.get() == crate::estado::AppState::Pausado && (teclas.just_pressed(KeyCode::Digit7) || teclas.just_pressed(KeyCode::Numpad7)) {
        ledger.aberto = !ledger.aberto;
    }
}

/// Recalcula as linhas dos rankings por categoria (no máximo 1x por segundo e só com a aba aberta).
fn atualizar_rankings_categoria(
    mut ledger: ResMut<RecursoLedger>,
    eco: Option<Res<EconomiaSim>>,
    tec: Option<Res<super::tecnologia_ui::TecnologiaSim>>,
    mundo_geo: Res<RecursoMundo>,
    tempo: Res<Time<Real>>,
    mut acumulado: Local<f32>,
) {
    *acumulado += tempo.delta_secs();
    if !ledger.aberto || ledger.aba_ativa != AbaLedger::RankingsCategoria {
        *acumulado = 1.0; // recalcula assim que a aba abrir
        return;
    }
    if *acumulado < 1.0 && !ledger.linhas_cat.is_empty() {
        return;
    }
    *acumulado = 0.0;
    let Some(eco) = eco else { return };
    let mut linhas = Vec::with_capacity(eco.vista.ranking.len());
    for p in &eco.vista.ranking {
        if p.populacao <= 0.0 {
            continue;
        }
        let cor = mundo_geo.paises.get(&p.tag).map(|g| egui::Color32::from_rgb(g.cor.0, g.cor.1, g.cor.2)).unwrap_or(egui::Color32::GRAY);
        let crescimento = ledger
            .historico_paises
            .get(&p.id)
            .filter(|h| h.pontos.len() >= 2)
            .map(|h| {
                let ult = h.pontos.last().map(|x| x.pib as f64).unwrap_or(0.0);
                let ini = h.pontos[h.pontos.len().saturating_sub(13)].pib as f64;
                if ini > 0.0 { (ult / ini - 1.0) * 100.0 } else { 0.0 }
            })
            .unwrap_or(0.0);
        let techs = tec.as_ref().and_then(|t| t.mundo.paises.get(&p.id).or_else(|| t.mundo.paises.get(&p.tag))).map(|e| e.conhecidas.len()).unwrap_or(0);
        let influencia = ledger.rankings_cache.iter().find(|e| e.tag == p.tag).map(|e| e.pontuacao_inf).unwrap_or(0.0);
        let mut v = [0.0; 15];
        v[Categoria::Exportacoes as usize] = p.exportacoes * 365.0;
        v[Categoria::Importacoes as usize] = p.importacoes * 365.0;
        v[Categoria::SaldoComercial as usize] = (p.exportacoes - p.importacoes) * 365.0;
        v[Categoria::Populacao as usize] = p.populacao;
        v[Categoria::Pib as usize] = p.pib_anual;
        v[Categoria::PibPerCapita as usize] = p.pib_anual / p.populacao;
        v[Categoria::Crescimento as usize] = crescimento;
        v[Categoria::ForcaMilitar as usize] = p.forca_militar;
        v[Categoria::Industria as usize] = p.producao_bens.iter().sum();
        v[Categoria::Tecnologia as usize] = techs as f64;
        v[Categoria::PadraoVida as usize] = p.padrao_vida;
        v[Categoria::Desemprego as usize] = p.desemprego * 100.0;
        v[Categoria::Divida as usize] = p.divida;
        v[Categoria::Influencia as usize] = influencia;
        v[Categoria::Turismo as usize] = p.turismo_receita_dia * 365.0;
        linhas.push(LinhaCategoria { tag: p.tag.clone(), nome: p.nome.clone(), cor, valores: v });
    }
    ledger.linhas_cat = linhas;
}

fn bandeira_ou_cor(ui: &mut egui::Ui, l: &LinhaCategoria) {
    if !super::bandeiras_ui::bandeira_ui(ui, &l.tag, 14.0) {
        let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(r, 2.0, l.cor);
        ui.painter().rect_stroke(r, 2.0, egui::Stroke::new(1.0, egui::Color32::BLACK), egui::StrokeKind::Inside);
    }
}

fn desenhar_aba_categorias(ui: &mut egui::Ui, ledger: &mut RecursoLedger, tag_jogador: &str) {
    ui.horizontal_wrapped(|ui| {
        ui_refinada::glifo_ui(ui, "estrela", 19.0, Some(ui_refinada::menu::OURO));
        ui.label(
            egui::RichText::new("RANKINGS GLOBAIS POR CATEGORIA")
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(18.0)
                .strong()
                .color(ui_refinada::menu::AZUL_MARINHO),
        );
        ui.weak("— escolha a categoria; clique no cabeçalho da tabela para ordenar.");
    });
    ui.horizontal_wrapped(|ui| {
        for c in Categoria::TODAS {
            if ui_refinada::aba_glifo(ui, ledger.cat_selecionada == c, Some(ui_refinada::chave_para(c.nome())), c.nome()) {
                ledger.cat_selecionada = c;
                ledger.cat_ordem = c;
                ledger.cat_desc = true;
            }
        }
    });
    ui.separator();
    if ledger.linhas_cat.is_empty() {
        ui.label("Calculando os rankings...");
        return;
    }
    let cat = ledger.cat_selecionada;
    let mut top: Vec<&LinhaCategoria> = ledger.linhas_cat.iter().collect();
    top.sort_by(|a, b| b.valor(cat).total_cmp(&a.valor(cat)));
    let maximo = top.first().map(|l| l.valor(cat).abs()).unwrap_or(1.0).max(1e-9);
    let posicao_jogador = top.iter().position(|l| l.tag == tag_jogador);

    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(380.0);
            ui.label(
                egui::RichText::new(format!("TOP 10 · {}", cat.nome().to_uppercase()))
                    .family(ui_refinada::familia_oswald(ui.ctx()))
                    .size(15.0)
                    .strong()
                    .color(ui_refinada::menu::TINTA),
            );
            for (i, l) in top.iter().take(10).enumerate() {
                ui.horizontal(|ui| {
                    ui.add_sized([26.0, 18.0], egui::Label::new(egui::RichText::new(format!("{}º", i + 1)).monospace()));
                    bandeira_ou_cor(ui, l);
                    let nome = if l.tag == tag_jogador { egui::RichText::new(&l.nome).strong().color(ui_refinada::menu::VERMELHO) } else { egui::RichText::new(&l.nome) };
                    ui.add_sized([110.0, 18.0], egui::Label::new(nome).truncate());
                    let (r, _) = ui.allocate_exact_size(egui::vec2(70.0, 14.0), egui::Sense::hover());
                    let frac = (l.valor(cat).abs() / maximo).clamp(0.0, 1.0) as f32;
                    ui.painter().rect_filled(r, 0.0, ui_refinada::menu::PAPEL_CLARO);
                    ui.painter().rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(r.width() * frac, r.height())), 0.0, l.cor);
                    ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(1.0, ui_refinada::menu::TINTA_SUAVE), egui::StrokeKind::Inside);
                    ui.label(egui::RichText::new(cat.formatar(l.valor(cat))).monospace().size(12.0));
                });
            }
            if let Some(p) = posicao_jogador
                && p >= 10
            {
                ui.separator();
                ui.label(format!("Seu país: {}º de {}  ({})", p + 1, top.len(), cat.formatar(top[p].valor(cat))));
            }
        });
        ui.separator();
        // Tabela completa, ordenável por qualquer categoria.
        let ord = ledger.cat_ordem;
        let desc = ledger.cat_desc;
        let mut linhas: Vec<&LinhaCategoria> = ledger.linhas_cat.iter().collect();
        linhas.sort_by(|a, b| {
            let o = a.valor(ord).total_cmp(&b.valor(ord));
            if desc { o.reverse() } else { o }
        });
        let mut nova_ord = None;
        egui::ScrollArea::both().id_salt("scroll_rank_categorias").max_height(500.0).show(ui, |ui| {
            egui::Grid::new("grid_rank_categorias").striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                cabecalho_fixo(ui, "#");
                cabecalho_fixo(ui, "País");
                for c in Categoria::TODAS {
                    if botao_coluna(ui, c.nome(), ord == c, desc) {
                        nova_ord = Some(c);
                    }
                }
                ui.end_row();
                for (i, l) in linhas.iter().enumerate() {
                    ui.label(egui::RichText::new(format!("{}", i + 1)).size(12.0).color(ui_refinada::menu::TINTA_SUAVE));
                    ui.horizontal(|ui| {
                        bandeira_ou_cor(ui, l);
                        let t = if l.tag == tag_jogador { egui::RichText::new(&l.nome).size(12.0).strong().color(ui_refinada::menu::VERMELHO) } else { egui::RichText::new(&l.nome).size(12.0).color(ui_refinada::menu::TINTA) };
                        ui.label(t);
                    });
                    for c in Categoria::TODAS {
                        ui.label(egui::RichText::new(c.formatar(l.valor(c))).size(12.0).color(ui_refinada::menu::TINTA));
                    }
                    ui.end_row();
                }
            });
        });
        if let Some(c) = nova_ord {
            if ledger.cat_ordem == c {
                ledger.cat_desc = !ledger.cat_desc;
            } else {
                ledger.cat_ordem = c;
                ledger.cat_desc = true;
            }
            ledger.cat_selecionada = c;
        }
    });
}
