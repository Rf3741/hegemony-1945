//! Gerador de dados históricos plausíveis de 1945 para os View-Models da interface.
//! Cobre Estados Unidos da América (USA) e União Soviética (SOV).

use crate::view_model::{
    construcao::ConstrucaoViewModel,
    diplomacia::DiplomaciaViewModel,
    estado::EstadoViewModel,
    eventos::EventoModalViewModel,
    mercado::MercadoViewModel,
    notificacoes::NotificacoesViewModel,
    orcamento::OrcamentoViewModel,
    pais::PaisViewModel,
    politica::PoliticaViewModel,
    ranking::RankingViewModel,
    top_bar::TopBarViewModel,
};

/// Estrutura de agregação que contém todos os modelos de visualização ativos.
#[derive(Debug, Clone, Default)]
pub struct CenariosUi1945 {
    pub top_bar: TopBarViewModel,
    pub pais: PaisViewModel,
    pub estado: EstadoViewModel,
    pub mercado: MercadoViewModel,
    pub construcao: ConstrucaoViewModel,
    pub orcamento: OrcamentoViewModel,
    pub politica: PoliticaViewModel,
    pub diplomacia: DiplomaciaViewModel,
    pub ranking: RankingViewModel,
    pub notificacoes: NotificacoesViewModel,
    pub evento: EventoModalViewModel,
}

impl CenariosUi1945 {
    /// Carrega o cenário histórico padrão dos Estados Unidos em 01/01/1945.
    pub fn eua_1945() -> Self {
        Self {
            top_bar: TopBarViewModel::default(),
            pais: PaisViewModel::default(),
            estado: EstadoViewModel::default(),
            mercado: MercadoViewModel::default(),
            construcao: ConstrucaoViewModel::default(),
            orcamento: OrcamentoViewModel::default(),
            politica: PoliticaViewModel::default(),
            diplomacia: DiplomaciaViewModel::default(),
            ranking: RankingViewModel::default(),
            notificacoes: NotificacoesViewModel::default(),
            evento: EventoModalViewModel::default(),
        }
    }

    /// Carrega o cenário histórico alternativo da União Soviética em 01/01/1945.
    pub fn urss_1945() -> Self {
        let mut dados = Self::default();
        dados.top_bar.tag_pais = "SOV".to_string();
        dados.top_bar.nome_pais = "União Soviética".to_string();
        dados.top_bar.classificacao_ranking = "#2 Superpotência".to_string();
        dados.top_bar.nome_lider = "Josef Stalin".to_string();
        dados.top_bar.titulo_lider = "Secretário-Geral do PCUS".to_string();
        dados.top_bar.tesouro_milhoes = 6800.0;
        dados.top_bar.balanco_diario = 18200.0;
        dados.top_bar.pib_bilhoes = 145.2;
        dados.top_bar.populacao_milhoes = 168.5;
        dados.top_bar.mao_de_obra_milhoes = 62.0;
        dados.top_bar.ic_alocada = 142;
        dados.top_bar.ic_total = 160;
        dados.top_bar.legitimidade_pct = 88.0;
        dados.top_bar.estabilidade_pct = 78.0;
        dados.top_bar.prestigio_pontos = 1080;

        dados.pais.tag = "SOV".to_string();
        dados.pais.nome_oficial = "União das Repúblicas Socialistas Soviéticas".to_string();
        dados.pais.capital = "Moscou".to_string();
        dados.pais.forma_governo = "Estado Socialista de Partido Único".to_string();
        dados.pais.ideologia_dominante = "Marxismo-Leninismo / Comunismo".to_string();
        dados.pais.lider_nome = "Josef Stalin".to_string();
        dados.pais.vice_lider_nome = "Vyacheslav Molotov".to_string();
        dados.pais.receita_semanal_milhoes = 120.0;
        dados.pais.despesa_semanal_milhoes = 105.0;

        dados
    }
}
