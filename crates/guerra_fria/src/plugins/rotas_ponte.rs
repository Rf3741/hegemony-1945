//! Ponte entre o mapa do cliente e as camadas de rios/rotas do `visual_mapa`.
//!
//! - Aponta `assets/geo/rios.ron` para a raiz do projeto.
//! - Gera a [`MascaraTerra`] a partir de [`DadosMapa`] (as rotas não cruzam o oceano).

use super::map::{DadosMapa, RaizProjeto};
use bevy::prelude::*;
use visual_mapa::rios::ConfigRios;
use visual_mapa::rotas::MascaraTerra;

pub struct RotasPontePlugin;

impl Plugin for RotasPontePlugin {
    fn build(&self, app: &mut App) {
        if let Some(raiz) = app.world().get_resource::<RaizProjeto>() {
            let arquivo = raiz.0.join("assets/geo/rios.ron");
            app.insert_resource(ConfigRios { arquivo, ..default() });
        }
        app.add_systems(Update, criar_mascara_terra.run_if(resource_exists::<DadosMapa>.and_then(not(resource_exists::<MascaraTerra>))));
    }
}

fn criar_mascara_terra(mut commands: Commands, dados: Res<DadosMapa>) {
    let terra = dados.ids.iter().map(|&id| id != 0).collect();
    commands.insert_resource(MascaraTerra { largura: dados.largura, altura: dados.altura, terra });
}
