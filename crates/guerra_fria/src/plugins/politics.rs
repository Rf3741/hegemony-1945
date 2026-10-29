//! Plugin de política: a simulação agora vive em `politica_sim` (motor `sim_politica`).
use super::politica_sim::PoliticaSimPlugin;
use bevy::prelude::*;

pub struct PoliticsPlugin;

impl Plugin for PoliticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PoliticaSimPlugin);
    }
}
