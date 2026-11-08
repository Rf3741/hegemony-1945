//! ui_refinada: Biblioteca de interface gráfica de alto nível para Guerra Fria (1945+).
//! Construída em Rust + Bevy + `bevy_egui`.

pub mod mock_data;
pub mod plugin;
pub mod screens;
pub mod theme;
pub mod view_model;
pub mod widgets;

pub use mock_data::CenariosUi1945;
pub use plugin::UiPlugin;
pub use theme::{
    aba_estilo_menu, aba_glifo, botao_estilo_menu, botao_estilo_secundario, cabecalho_painel,
    chave_para, chapa, cold_war_style, cold_war_visuals, cor_saldo, cor_variacao, decorar_janela,
    desenhar_moldura_dupla_com_cantos, estilo_papel_ui, familia_oswald, faixa_titulo, glifo_ui,
    janela_dossie, janela_estilo_menu, linha_decorativa, menu, moldura_cartao,
    moldura_cartao_papel, moldura_janela_papel, moldura_painel, moldura_papel_dossie, paleta,
    placa_data, razao_contraste, setup_cold_war_fonts, tabela_linha_alt, titulo_secao,
};
pub use view_model::*;
pub use widgets::{barra_relacao, barra_status, barra_trilho};

/// Retorna a versão da crate.
pub fn versao() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
