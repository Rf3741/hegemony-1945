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
pub use theme::{cold_war_style, cold_war_visuals, setup_cold_war_fonts};
pub use view_model::*;

/// Retorna a versão da crate.
pub fn versao() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
