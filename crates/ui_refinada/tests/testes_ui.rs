//! Testes unitários para a UI refinada de Guerra Fria 1945.

use ui_refinada::mock_data::CenariosUi1945;
use ui_refinada::theme::paleta::cor_pais;
use ui_refinada::view_model::{
    construcao::ConstrucaoViewModel,
    estado::EstadoViewModel,
    mercado::MercadoViewModel,
    orcamento::OrcamentoViewModel,
    politica::PoliticaViewModel,
    ranking::RankingViewModel,
};

#[test]
fn teste_carregamento_cenario_padrao_eua() {
    let cenario = CenariosUi1945::eua_1945();
    assert_eq!(cenario.top_bar.tag_pais, "USA");
    assert_eq!(cenario.top_bar.nome_pais, "Estados Unidos da América");
    assert!(cenario.top_bar.balanco_diario > 0.0);
    assert_eq!(cenario.top_bar.nivel_defcon, 4);
    assert_eq!(cenario.pais.forma_governo, "República Presidencialista Constitucional");
}

#[test]
fn teste_carregamento_cenario_urss() {
    let cenario = CenariosUi1945::urss_1945();
    assert_eq!(cenario.top_bar.tag_pais, "SOV");
    assert_eq!(cenario.top_bar.nome_lider, "Josef Stalin");
    assert_eq!(cenario.pais.capital, "Moscou");
    assert!(cenario.top_bar.prestigio_pontos > 1000);
}

#[test]
fn teste_calculo_dinamico_orcamento() {
    let mut orc = OrcamentoViewModel::default();
    let rec_inicial = orc.receita_semanal_estimada();
    let desp_inicial = orc.despesa_semanal_estimada();
    let saldo_inicial = orc.saldo_liquido_semanal();

    assert!(rec_inicial > 150.0);
    assert!(desp_inicial > 100.0);
    // Em 1945 o orçamento de guerra é deficitário.
    assert!(saldo_inicial < 0.0);

    // Teste de aumento de tributos corporativos
    orc.imposto_renda_alta_pct = 95.0;
    let rec_aumentada = orc.receita_semanal_estimada();
    assert!(rec_aumentada > rec_inicial);

    // Teste de explosão de gastos militares
    orc.despesa_forcas_armadas_milhoes = 150.0;
    let desp_aumentada = orc.despesa_semanal_estimada();
    assert!(desp_aumentada > desp_inicial);
}

#[test]
fn teste_distribuicao_pops_estado() {
    let estado = EstadoViewModel::default();
    assert_eq!(estado.adm1_code, "USA-3522");
    assert_eq!(estado.nome, "Pensilvânia");

    // Verifica que a soma dos percentuais de profissões é ~100%
    let soma_pct: f32 = estado.pops_profissoes.iter().map(|p| p.percentual).sum();
    assert!((soma_pct - 100.0).abs() < 1.0);

    // Verifica prédios
    assert!(!estado.predios.is_empty());
    let siderurgica = &estado.predios[0];
    assert_eq!(siderurgica.nivel, 8);
    assert!(siderurgica.trabalhadores_empregados > 30_000);
}

#[test]
fn teste_mercado_e_cotacoes() {
    let mercado = MercadoViewModel::default();
    assert_eq!(mercado.bem_selecionado_id, "aco");
    assert!(!mercado.bens.is_empty());
    assert!(!mercado.historico_precos.is_empty());

    let aco = mercado.bens.iter().find(|b| b.id == "aco").expect("Aço deve existir");
    assert_eq!(aco.preco_base, 50.0);
    assert!(aco.preco_atual < aco.preco_base); // desvio negativo
}

#[test]
fn teste_fila_construcao() {
    let mut construcao = ConstrucaoViewModel::default();
    assert_eq!(construcao.fila.len(), 4);

    // Teste de reordenação (swap)
    let p0 = construcao.fila[0].id.clone();
    let p1 = construcao.fila[1].id.clone();
    construcao.fila.swap(0, 1);
    assert_eq!(construcao.fila[0].id, p1);
    assert_eq!(construcao.fila[1].id, p0);
}

#[test]
fn teste_politica_leis_e_grupos() {
    let pol = PoliticaViewModel::default();
    assert_eq!(pol.leis_ativas.len(), 7); // 7 categorias institucionais
    assert_eq!(pol.grupos_interesse.len(), 5);

    let lei_tram = pol.lei_em_tramitacao.expect("Deve haver lei em tramitação");
    assert!(lei_tram.probabilidade_aprovacao_pct > 50.0);
}

#[test]
fn teste_ranking_potencias() {
    let rank = RankingViewModel::default();
    assert_eq!(rank.potencias.len(), 8);
    assert_eq!(rank.potencias[0].tag, "USA");
    assert_eq!(rank.potencias[1].tag, "SOV");
    assert_eq!(rank.potencias[2].tag, "GBR");
}

#[test]
fn teste_cores_paises() {
    let cor_usa = cor_pais("USA");
    let cor_sov = cor_pais("SOV");
    assert_ne!(cor_usa, cor_sov);
}
