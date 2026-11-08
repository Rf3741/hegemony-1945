//! # `sim_economia`
//!
//! Biblioteca Rust pura (sem dependência do Bevy) para simulação macro e microeconômica
//! estilo *Victoria 3* ambientada na Guerra Fria a partir de `01/01/1945`.
//!
//! ## Principais recursos
//! - **Esquema orientado a dados em RON (`serde` + `ron`)**: [`Bem`], [`TipoPop`], [`LeiEconomica`],
//!   [`MetodoProducao`], [`TipoPredio`], [`Predio`], [`Pop`], [`Provincia`], [`Estado`], [`Pais`] e [`Mercado`].
//! - **Ciclo diário/semanal determinístico e sem alocações recorrentes** em [`MundoEconomico`]:
//!   1. Produção dos prédios por métodos ativos, preenchimento de vagas, acesso ao mercado e economia de escala;
//!   2. Formação de preços por oferta e demanda, escassez física de insumos e rotas de importação/exportação;
//!   3. Salários, reservas de caixa, lucros, fundo privado de investimento e dividendos (capitalistas, aristocratas, estatais, cooperativas);
//!   4. Consumo das pops por estrato (Lei de Engel) com substituição elástica de bens;
//!   5. Padrão de vida (`1..99`) e alfabetização (`0..1`);
//!   6. Crescimento vegetativo (transição demográfica) e migração interna atraída por vagas e padrão de vida;
//!   7. Contratação, promoção/qualificação profissional, demissão e ajuste salarial dinâmico;
//!   8. Impostos (renda, dividendos, consumo, sobre a terra), eficiência burocrática, gastos públicos (burocracia, forças armadas, construção, subsídios), dívida soberana e juros;
//!   9. Fila nacional de construção com pontos de construção estatais e privados.

pub mod canais;
pub mod catalogo;
pub mod cenario;
pub mod entidades;
pub mod erro;
pub mod gerador;
pub mod motor;
pub mod tipos;

pub use catalogo::{
    Bem, CatalogoEconomia, LeiEconomica, MetodoProducao, SubstituicaoNecessidade, TipoPop,
    TipoPredio, ler_arquivo_ron, ler_str_ron,
};
pub use canais::{CanalHistorico, canal_disponivel, canais, dono_canal, pedagio_diario};
pub use cenario::CenarioEconomia;
pub use entidades::{
    BlocoComercial, ConfigSimulacao, DetalheRotaComercial, Estado, ItemConstrucao, Mercado,
    OpcaoParceiroComercial, OrcamentoGoverno, Pais, PoliticaFiscal, Pop, PotencialRegional, Predio,
    Provincia, ResumoComercioPais, RotaComercial,
};
pub use erro::{ErroEconomia, ResultadoEconomia, validar_id_snake_case};
pub use gerador::{ParametrosEscalaMundial, gerar_cenario_escala_mundial};
pub use motor::consumo::calcular_fatias_substituicao;
pub use motor::demografia::taxa_crescimento_anual_pop;
pub use motor::mercado::{atualizar_precos_mercado_isolado, calcular_preco_alvo};
pub use motor::producao::atualizar_infraestrutura_estados;
pub use motor::ciclos_demanda::{
    ChoqueEventoAtivo, ChoqueEventoDemanda, CurvaDemandaBem, EstadoCiclosDemanda,
    GatilhoTecnologiaDemanda, PontoEraDemanda, TabelaCiclosDemanda,
};
pub use motor::distribuicao::CUSTO_MANUTENCAO_FECHADO_POR_NIVEL;
pub use motor::gestao_predios::{
    CUSTO_MINIMO_REABRIR_POR_NIVEL, FRACAO_CUSTO_REABRIR, FRACAO_REEMBOLSO_DEMOLIR,
    REEMBOLSO_MINIMO_DEMOLIR_POR_NIVEL, ResultadoAcaoPredio,
    calcular_custo_reabertura_por_precos, calcular_reembolso_demolicao_por_precos,
    estimar_margem_operacional_predio, valor_materiais_construcao_niveis,
};
pub use motor::global::{ConfigMercadoGlobal, MercadoGlobal};
pub mod estimativa;
pub use estimativa::{calcular_com_perfil, precos_por_indice, PerfilPredio, calcular_lucratividade_esperada, ContextoLucratividade, EstimativaLucratividade};
pub use motor::governo::{
    estado_comporta_novo_nivel, estado_niveis_recurso, materiais_por_ponto,
    provincia_comporta_novo_nivel,
};
pub use motor::{EstadoEconomiaSalvo, MundoEconomico, RelatorioTick, TemposFasesTick};
pub use tipos::{
    CategoriaBem, CategoriaNecessidade, CategoriaPredio, DataJogo, Estrato, FormulaPreco,
    GrupoLeiEconomica, ModoCadencia, NUM_NECESSIDADES, RegimePropriedade, SlotMetodo,
};
