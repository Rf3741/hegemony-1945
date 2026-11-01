//! Ponte entre o mapa do cliente e as camadas de rios/rotas do `visual_mapa`.
//!
//! - Aponta `assets/geo/rios.ron` para a raiz do projeto.
//! - Gera a [`MascaraTerra`] a partir de [`DadosMapa`] (as rotas não cruzam o oceano).

use super::economy::RecursoMundo;
use super::map::{DadosMapa, RaizProjeto};
use sim_economia::{DataJogo, canal_disponivel};
use visual_mapa::trafego::{Canal, CanaisFechados};
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
        app.init_resource::<CanaisFechados>().add_systems(Update, atualizar_canais_fechados.run_if(resource_exists::<RecursoMundo>));
        app.add_systems(Update, criar_mascara_terra.run_if(resource_exists::<DadosMapa>.and_then(not(resource_exists::<MascaraTerra>))));
    }
}

fn criar_mascara_terra(mut commands: Commands, dados: Res<DadosMapa>) {
    let terra = dados.ids.iter().map(|&id| id != 0).collect();
    commands.insert_resource(MascaraTerra { largura: dados.largura, altura: dados.altura, terra });
}

/// Consulta o calendário de canais do `sim_economia` e avisa o tráfego de navios (rotas contornam).
fn atualizar_canais_fechados(mundo: Res<RecursoMundo>, mut fechados: ResMut<CanaisFechados>) {
    let d = mundo.0.data;
    let Some(data) = DataJogo::nova(d.ano.clamp(0, 9999) as u16, d.mes, d.dia) else { return };
    let novos: Vec<Canal> = [(Canal::Suez, "suez"), (Canal::Panama, "panama"), (Canal::Kiel, "kiel"), (Canal::Corinto, "corinto")]
        .into_iter()
        .filter(|(_, id)| !canal_disponivel(id, data))
        .map(|(c, _)| c)
        .collect();
    // Antes da inauguração (ex.: Panamá/Suez já abertos em 1945) o canal "fechado" só vale se existir no calendário.
    if fechados.0 != novos {
        fechados.0 = novos;
    }
}
