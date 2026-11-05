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
            Self::GrandePotencia => egui::Color32::from_rgb(255, 215, 0), // Dourado
            Self::PotenciaMaior => egui::Color32::from_rgb(180, 200, 240), // Prateado-azulado
            Self::PotenciaMenor => egui::Color32::from_rgb(160, 160, 160), // Cinza
        }
    }

    pub fn icone(self) -> &'static str {
        match self {
            Self::GrandePotencia => "👑",
            Self::PotenciaMaior => "⭐",
            Self::PotenciaMenor => "🔹",
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

const PALETA_CORES_GRAFICO: [egui::Color32; 16] = [
    egui::Color32::from_rgb(80, 160, 240),  // Azul claro
    egui::Color32::from_rgb(240, 80, 80),   // Vermelho vívido
    egui::Color32::from_rgb(80, 220, 120),  // Verde esmeralda
    egui::Color32::from_rgb(250, 200, 60),  // Dourado / Amarelo
    egui::Color32::from_rgb(190, 100, 240), // Roxo / Violeta
    egui::Color32::from_rgb(250, 140, 60),  // Laranja
    egui::Color32::from_rgb(60, 230, 230),  // Ciano
    egui::Color32::from_rgb(255, 120, 180), // Rosa
    egui::Color32::from_rgb(160, 230, 80),  // Lima
    egui::Color32::from_rgb(140, 180, 255), // Azul céu
    egui::Color32::from_rgb(255, 180, 120), // Pêssego
    egui::Color32::from_rgb(180, 220, 200), // Menta
    egui::Color32::from_rgb(220, 160, 255), // Lavanda
    egui::Color32::from_rgb(255, 230, 140), // Creme
    egui::Color32::from_rgb(140, 210, 240), // Turquesa
    egui::Color32::from_rgb(230, 130, 130), // Coral
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

    ui_refinada::janela_estilo_menu("ESTATÍSTICAS & LEDGER GERAL (7)")
        .open(&mut aberto)
        .default_width(1200.0)
        .default_height(720.0)
        .min_width(900.0)
        .min_height(550.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui_refinada::decorar_janela(ui);
            ui_refinada::cabecalho_painel(ui, "ESTATÍSTICAS & LEDGER MUNDIAL (7)");
            ui.horizontal(|ui| {
                let rotulo_aba1 = format!("📈 Gráficos Históricos ({})", ledger.metrica_grafico.titulo());
                if ui_refinada::aba_estilo_menu(ui, ledger.aba_ativa == AbaLedger::GraficosHistoricos, &rotulo_aba1) {
                    ledger.aba_ativa = AbaLedger::GraficosHistoricos;
                }
                if ui_refinada::aba_estilo_menu(ui, ledger.aba_ativa == AbaLedger::RankingsPotencias, "👑 Grandes Potências (Top 8)") {
                    ledger.aba_ativa = AbaLedger::RankingsPotencias;
                }
                if ui_refinada::aba_estilo_menu(ui, ledger.aba_ativa == AbaLedger::TabelaPaises, "📋 Ledger de Países (Tabela)") {
                    ledger.aba_ativa = AbaLedger::TabelaPaises;
                }
                if ui_refinada::aba_estilo_menu(ui, ledger.aba_ativa == AbaLedger::RankingsCategoria, "🏆 Rankings Globais") {
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
    ui.horizontal(|ui| {
        ui.strong("Métrica:");
        for metrica in MetricaGrafico::TODAS {
            let ativa = ledger.metrica_grafico == metrica;
            if ui.selectable_label(ativa, metrica.titulo()).clicked() {
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
                if ui.button("Meu País").on_hover_text("Seleciona apenas o seu país").clicked() {
                    ledger.paises_selecionados.clear();
                    ledger.paises_selecionados.insert(id_jogador.to_string());
                }
                if ui.button("Top 8").on_hover_text("Seleciona as 8 Grandes Potências").clicked() {
                    ledger.paises_selecionados.clear();
                    for e in ledger.rankings_cache.iter().take(8) {
                        ledger.paises_selecionados.insert(e.id.clone());
                    }
                }
                if ui.button("Limpar").clicked() {
                    ledger.paises_selecionados.clear();
                }
            });

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label("🔍");
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

            ui.horizontal(|ui| {
                ui.heading(&nome_metrica);
                ui.label(format!("(em {})", metrica.unidade()));
                ui.label(format!("— {} meses simulados", ledger.meses.len()));
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
                            [mes_i as f64, val]
                        })
                        .collect();

                    linhas.push((rotulo, cor, pontos));
                }
            }

            let num_meses = ledger.meses.len();

            Plot::new("plot_historico_ledger")
                .height(480.0)
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

            ui.horizontal(|ui| {
                if let Some(primeiro) = meses_nomes.first() {
                    ui.label(format!("Início: {}", primeiro));
                }
                if let Some(ultimo) = meses_nomes.last() {
                    ui.label(format!("| Data atual: {} (mês {})", ultimo, num_meses));
                }
                ui.label("| Dica: use roda do mouse para zoom e arraste com botão esquerdo para navegar");
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
    ui.horizontal(|ui| {
        ui.heading("👑 Grandes Potências de 1945");
        ui.label("— Classificação geopolítica oficial estilo Victoria 3 (Top 8)");
    });
    ui.label("A pontuação de potência combina poder econômico (PIB e SoL), poder militar (forças armadas e gastos) e projeção de influência global.");

    ui.add_space(8.0);

    // Grid dos 8 primeiros colocados em cartões luxuosos estilo Victoria 3
    egui::ScrollArea::vertical()
        .id_salt("scroll_potencias")
        .show(ui, |ui| {
            ui.columns(4, |cols| {
                for (i, p) in ledger.rankings_cache.iter().take(8).enumerate() {
                    let col_idx = i % 4;
                    cols[col_idx].group(|ui| {
                        let eh_jogador = p.id == id_jogador;
                        let moldura_cor = if eh_jogador {
                            egui::Color32::from_rgb(255, 215, 0)
                        } else {
                            p.grau.badge_cor()
                        };

                        ui.horizontal(|ui| {
                            ui.colored_label(moldura_cor, egui::RichText::new(format!("#{}: {}", p.rank_geral, p.grau.icone())).strong().size(18.0));
                            let (r, _) = ui.allocate_exact_size(egui::vec2(16.0, 12.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, 2.0, p.cor);
                            ui.strong(&p.nome);
                        });

                        if eh_jogador {
                            ui.colored_label(ui_refinada::menu::SUCESSO, "★ SEU PAÍS ★");
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Pontuação Total:");
                            ui.colored_label(ui_refinada::menu::OURO, egui::RichText::new(format!("{:.0} pts", p.pontuacao_total)).strong());
                        });

                        ui.add_space(4.0);
                        ui.label(format!("• Economia: {:.0} pts ({})", p.pontuacao_eco, fmt_din(p.pib_anual)));
                        ui.label(format!("• Força Militar: {:.0} pts", p.pontuacao_mil));
                        ui.label(format!("• Influência: {:.0} pts ({})", p.pontuacao_inf, p.esfera));
                        ui.label(format!("• População: {}", fmt_pop(p.populacao as u64)));
                        ui.label(format!("• Padrão de Vida: {:.1}", p.padrao_vida));
                    });
                    cols[col_idx].add_space(8.0);
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.heading("Pilares Comparativos de Potência (Top 10 por Dimensão)");

            ui.columns(3, |cols| {
                // Coluna 1: Maiores Potências Econômicas
                cols[0].group(|ui| {
                    ui.strong("📈 Potência Econômica (PIB)");
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
                    ui.strong("⚔️ Potência Militar (Força Armada)");
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
                    ui.strong("🌐 Projeção de Influência");
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

    let rotulo_cabecalho = |col_target: ColunaLedger, nome: &str| -> String {
        if col_ord == col_target {
            let seta = if desc_ord { " ▼" } else { " ▲" };
            format!("{}{}", nome, seta)
        } else {
            nome.to_string()
        }
    };

    // Cabeçalho fixo da tabela
    ui.horizontal(|ui| {
        if ui.add_sized([50.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::RankGeral, "Rank"))).clicked() {
            nova_col_ord = Some(ColunaLedger::RankGeral);
        }
        if ui.add_sized([180.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::Nome, "País"))).clicked() {
            nova_col_ord = Some(ColunaLedger::Nome);
        }
        ui.add_sized([110.0, 20.0], egui::Label::new(egui::RichText::new("Grau").strong()));
        if ui.add_sized([60.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::PontuacaoTotal, "Pontos"))).clicked() {
            nova_col_ord = Some(ColunaLedger::PontuacaoTotal);
        }
        if ui.add_sized([80.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::Pib, "PIB Anual"))).clicked() {
            nova_col_ord = Some(ColunaLedger::Pib);
        }
        if ui.add_sized([60.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::PontuacaoMil, "Militar"))).clicked() {
            nova_col_ord = Some(ColunaLedger::PontuacaoMil);
        }
        if ui.add_sized([70.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::PontuacaoInf, "Influência"))).clicked() {
            nova_col_ord = Some(ColunaLedger::PontuacaoInf);
        }
        if ui.add_sized([80.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::Populacao, "População"))).clicked() {
            nova_col_ord = Some(ColunaLedger::Populacao);
        }
        if ui.add_sized([50.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::PadraoVida, "SoL"))).clicked() {
            nova_col_ord = Some(ColunaLedger::PadraoVida);
        }
        if ui.add_sized([60.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::Desemprego, "Desemp."))).clicked() {
            nova_col_ord = Some(ColunaLedger::Desemprego);
        }
        if ui.add_sized([80.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::Tesouro, "Tesouro"))).clicked() {
            nova_col_ord = Some(ColunaLedger::Tesouro);
        }
        if ui.add_sized([80.0, 20.0], egui::Button::new(rotulo_cabecalho(ColunaLedger::Divida, "Dívida"))).clicked() {
            nova_col_ord = Some(ColunaLedger::Divida);
        }
        ui.add_sized([120.0, 20.0], egui::Label::new(egui::RichText::new("Esfera").strong()));
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

                    let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 10.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 1.5, p.cor);

                    let nome_txt = if eh_jogador {
                        egui::RichText::new(format!("{} ({}) ★", p.nome, p.tag)).color(cor_destaque).strong()
                    } else {
                        egui::RichText::new(format!("{} ({})", p.nome, p.tag)).color(cor_destaque)
                    };
                    ui.add_sized([162.0, 20.0], egui::Label::new(nome_txt).truncate());

                    ui.add_sized([110.0, 20.0], egui::Label::new(egui::RichText::new(format!("{} {}", p.grau.icone(), p.grau.rotulo())).color(p.grau.badge_cor())));
                    ui.add_sized([60.0, 20.0], egui::Label::new(egui::RichText::new(format!("{:.0}", p.pontuacao_total)).color(egui::Color32::from_rgb(250, 220, 90))));
                    ui.add_sized([80.0, 20.0], egui::Label::new(fmt_din(p.pib_anual)));
                    ui.add_sized([60.0, 20.0], egui::Label::new(format!("{:.0}", p.pontuacao_mil)));
                    ui.add_sized([70.0, 20.0], egui::Label::new(format!("{:.0}", p.pontuacao_inf)));
                    ui.add_sized([80.0, 20.0], egui::Label::new(fmt_pop(p.populacao as u64)));
                    ui.add_sized([50.0, 20.0], egui::Label::new(format!("{:.1}", p.padrao_vida)));
                    ui.add_sized([60.0, 20.0], egui::Label::new(format!("{:.1}%", p.desemprego * 100.0)));
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
    ui.horizontal(|ui| {
        ui.heading("🏆 Rankings Globais");
        ui.label("— escolha a categoria para ver o Top 10; clique no cabeçalho da tabela para ordenar.");
    });
    ui.horizontal_wrapped(|ui| {
        for c in Categoria::TODAS {
            if ui.selectable_label(ledger.cat_selecionada == c, c.nome()).clicked() {
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
            ui.strong(format!("Top 10 — {}", cat.nome()));
            for (i, l) in top.iter().take(10).enumerate() {
                ui.horizontal(|ui| {
                    ui.add_sized([26.0, 18.0], egui::Label::new(egui::RichText::new(format!("{}º", i + 1)).monospace()));
                    bandeira_ou_cor(ui, l);
                    let nome = if l.tag == tag_jogador { egui::RichText::new(&l.nome).strong().color(egui::Color32::from_rgb(250, 200, 60)) } else { egui::RichText::new(&l.nome) };
                    ui.add_sized([110.0, 18.0], egui::Label::new(nome).truncate());
                    let (r, _) = ui.allocate_exact_size(egui::vec2(70.0, 14.0), egui::Sense::hover());
                    let frac = (l.valor(cat).abs() / maximo).clamp(0.0, 1.0) as f32;
                    ui.painter().rect_filled(r, 2.0, egui::Color32::from_black_alpha(90));
                    ui.painter().rect_filled(egui::Rect::from_min_size(r.min, egui::vec2(r.width() * frac, r.height())), 2.0, l.cor);
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
                ui.strong("#");
                ui.strong("País");
                for c in Categoria::TODAS {
                    let seta = if ord == c { if desc { " ▼" } else { " ▲" } } else { "" };
                    if ui.button(format!("{}{}", c.nome(), seta)).clicked() {
                        nova_ord = Some(c);
                    }
                }
                ui.end_row();
                for (i, l) in linhas.iter().enumerate() {
                    ui.label(format!("{}", i + 1));
                    ui.horizontal(|ui| {
                        bandeira_ou_cor(ui, l);
                        let t = if l.tag == tag_jogador { egui::RichText::new(&l.nome).strong().color(egui::Color32::from_rgb(250, 200, 60)) } else { egui::RichText::new(&l.nome) };
                        ui.label(t);
                    });
                    for c in Categoria::TODAS {
                        ui.label(egui::RichText::new(c.formatar(l.valor(c))).monospace().size(12.0));
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
