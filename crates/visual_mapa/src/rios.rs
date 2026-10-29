//! Rios principais (Natural Earth `ne_10m_rivers_lake_centerlines`, domínio público).
//!
//! O arquivo `assets/geo/rios.ron` é gerado por `scripts/gerar_rios_ron.py` já na projeção Miller
//! do mapa (coordenadas de mundo). As malhas são criadas uma única vez no `Startup`: uma para os
//! rios principais (sempre visíveis) e outra para os afluentes (só no zoom médio e de perto).
//! Cada malha tem 3 cópias (wrap horizontal, como o sprite do mapa).

use crate::linhas::ConstrutorLinhas;
use crate::lod::{EstadoLod, NivelLod};
use bevy::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct RioDado {
    /// `scalerank` do Natural Earth (1 = maior rio).
    pub ordem: u8,
    pub pontos: Vec<(f32, f32)>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ArquivoRios {
    pub rios: Vec<RioDado>,
}

/// Caminho do arquivo de rios (primeiro candidato existente).
#[derive(Resource, Debug, Clone)]
pub struct ConfigRios {
    pub arquivo: PathBuf,
    pub ativo: bool,
    pub largura_mapa: f32,
}

impl Default for ConfigRios {
    fn default() -> Self {
        let mut candidatos = vec![PathBuf::from("assets/geo/rios.ron")];
        if let Ok(d) = std::env::var("GF_ROOT") {
            candidatos.push(PathBuf::from(d).join("assets/geo/rios.ron"));
        }
        candidatos.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/geo/rios.ron"));
        let arquivo = candidatos.iter().find(|p| p.exists()).cloned().unwrap_or_else(|| candidatos[0].clone());
        Self { arquivo, ativo: true, largura_mapa: 5632.0 }
    }
}

/// Camada de rios: `principal` = sempre visível; senão só fora do zoom global.
#[derive(Component, Debug, Clone, Copy)]
pub struct CamadaRios {
    pub principal: bool,
}

pub fn carregar_arquivo_rios(caminho: &Path) -> Result<ArquivoRios, String> {
    let txt = std::fs::read_to_string(caminho).map_err(|e| format!("{}: {e}", caminho.display()))?;
    ron::from_str(&txt).map_err(|e| e.to_string())
}

/// Largura (unidades de mundo) e cor por ordem do rio.
pub fn estilo_rio(ordem: u8) -> (f32, Color) {
    match ordem {
        0..=3 => (2.6, Color::srgb(0.30, 0.55, 0.84)),
        4..=5 => (2.0, Color::srgb(0.32, 0.58, 0.86)),
        6..=7 => (1.5, Color::srgb(0.36, 0.61, 0.87)),
        _ => (1.1, Color::srgb(0.40, 0.64, 0.88)),
    }
}

/// Gera as malhas (principais, afluentes). Função pura.
pub fn construir_malhas_rios(arq: &ArquivoRios) -> (ConstrutorLinhas, ConstrutorLinhas) {
    let (mut principal, mut menor) = (ConstrutorLinhas::novo(), ConstrutorLinhas::novo());
    for r in &arq.rios {
        let pts: Vec<Vec2> = r.pontos.iter().map(|&(x, y)| Vec2::new(x, y)).collect();
        let (w, cor) = estilo_rio(r.ordem);
        let dest = if r.ordem <= 6 { &mut principal } else { &mut menor };
        dest.polilinha(&pts, w, cor);
    }
    (principal, menor)
}

pub fn criar_rios_sistema(
    mut commands: Commands,
    cfg: Res<ConfigRios>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materiais: ResMut<Assets<ColorMaterial>>,
) {
    if !cfg.ativo {
        return;
    }
    let arq = match carregar_arquivo_rios(&cfg.arquivo) {
        Ok(a) => a,
        Err(e) => {
            warn!("visual_mapa: rios indisponíveis ({e}).");
            return;
        }
    };
    let (principal, menor) = construir_malhas_rios(&arq);
    let mat = materiais.add(ColorMaterial::from_color(Color::WHITE));
    for (constr, eh_principal, z) in [(menor, false, 1.5), (principal, true, 1.6)] {
        if constr.vazio() {
            continue;
        }
        let h = meshes.add(constr.construir());
        for k in [-1.0f32, 0.0, 1.0] {
            commands.spawn((
                Mesh2d(h.clone()),
                MeshMaterial2d(mat.clone()),
                Transform::from_xyz(k * cfg.largura_mapa, 0.0, z),
                CamadaRios { principal: eh_principal },
            ));
        }
    }
    info!("visual_mapa: {} rios carregados.", arq.rios.len());
}

/// Afluentes só aparecem fora do zoom global.
pub fn lod_rios_sistema(lod: Res<EstadoLod>, mut q: Query<(&CamadaRios, &mut Visibility)>) {
    if !lod.is_changed() {
        return;
    }
    for (c, mut v) in &mut q {
        *v = if c.principal || lod.nivel_atual != NivelLod::Global {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arquivo_real_carrega() {
        let cfg = ConfigRios::default();
        if !cfg.arquivo.exists() {
            return;
        }
        let a = carregar_arquivo_rios(&cfg.arquivo).expect("rios.ron");
        assert!(a.rios.len() > 1000);
        let (p, m) = construir_malhas_rios(&a);
        assert!(!p.vazio() && !m.vazio());
    }

    #[test]
    fn rio_maior_mais_largo() {
        assert!(estilo_rio(1).0 > estilo_rio(9).0);
    }
}
