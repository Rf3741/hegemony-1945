//! Rios principais (Natural Earth `ne_10m_rivers_lake_centerlines`, domínio público).
//!
//! O arquivo `assets/geo/rios.ron` é gerado por `scripts/gerar_rios_ron.py` já na projeção Miller
//! do mapa (coordenadas de mundo). As malhas são criadas uma única vez no `Startup`: uma para os
//! rios principais (sempre visíveis) e outra para os afluentes (só no zoom médio e de perto).
//! Cada malha tem 3 cópias (wrap horizontal, como o sprite do mapa).

use crate::linhas::ConstrutorLinhas;
use crate::estilo::EstiloMapa;
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
    let mut arq: ArquivoRios = ron::from_str(&txt).map_err(|e| e.to_string())?;
    // O arquivo está em coordenadas de pixel centradas; o mundo tem y esticado (ESCALA_Y_MAPA).
    for r in &mut arq.rios {
        for p in &mut r.pontos {
            p.1 *= gf_mapa::mapa::ESCALA_Y_MAPA;
        }
    }
    Ok(arq)
}

/// Cor por ordem do rio (azul suave, semitransparente).
pub fn estilo_rio(ordem: u8) -> (f32, Color) {
    // O primeiro valor é só um peso relativo (maior rio = maior); a largura real é em pixels de tela.
    match ordem {
        0..=3 => (1.0, Color::srgba(0.36, 0.60, 0.86, 0.5)),
        4..=5 => (0.9, Color::srgba(0.38, 0.62, 0.88, 0.5)),
        6..=7 => (0.7, Color::srgba(0.40, 0.65, 0.89, 0.45)),
        _ => (0.6, Color::srgba(0.42, 0.67, 0.90, 0.4)),
    }
}

/// Gera as malhas (principais, afluentes) para a escala de câmera dada: a largura vira
/// `px * escala` unidades de mundo, ou seja, `px` pixels na tela. Função pura.
pub fn construir_malhas_rios_escala(arq: &ArquivoRios, escala: f32, estilo: &crate::estilo::EstiloMapa) -> (ConstrutorLinhas, ConstrutorLinhas) {
    let (mut principal, mut menor) = (ConstrutorLinhas::novo(), ConstrutorLinhas::novo());
    for r in &arq.rios {
        let pts: Vec<Vec2> = r.pontos.iter().map(|&(x, y)| Vec2::new(x, y)).collect();
        let (peso, mut cor) = estilo_rio(r.ordem);
        let eh_principal = r.ordem <= 6;
        let px = if eh_principal { estilo.rios_px_principal } else { estilo.rios_px_menor } * peso;
        cor.set_alpha(cor.alpha() * estilo.rios_alfa / 0.5);
        let dest = if eh_principal { &mut principal } else { &mut menor };
        dest.polilinha(&pts, (px * escala).max(0.2), cor);
    }
    (principal, menor)
}

/// Compatibilidade: malhas na escala 1.0.
pub fn construir_malhas_rios(arq: &ArquivoRios) -> (ConstrutorLinhas, ConstrutorLinhas) {
    construir_malhas_rios_escala(arq, 1.0, &crate::estilo::EstiloMapa::default())
}

/// Dados mantidos para refazer as malhas quando o zoom muda (largura constante na tela).
#[derive(Resource)]
pub struct RiosDados {
    pub arq: ArquivoRios,
    pub malha_principal: Handle<Mesh>,
    pub malha_menor: Handle<Mesh>,
    pub balde: i32,
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
    let estilo = EstiloMapa::default();
    let (principal, menor) = construir_malhas_rios_escala(&arq, 1.0, &estilo);
    let mut hs: [Handle<Mesh>; 2] = [Handle::default(), Handle::default()];
    let mat = materiais.add(ColorMaterial::from_color(Color::WHITE));
    for (constr, eh_principal, z) in [(menor, false, 1.5), (principal, true, 1.6)] {
        if constr.vazio() {
            continue;
        }
        let h = meshes.add(constr.construir());
        hs[eh_principal as usize] = h.clone();
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
    commands.insert_resource(RiosDados { malha_principal: hs[1].clone(), malha_menor: hs[0].clone(), arq, balde: i32::MIN });
}

/// Refaz as malhas quando o zoom muda (balde de meio passo em log2) e esconde afluentes ao longe.
pub fn lod_rios_sistema(
    estilo: Res<EstiloMapa>,
    camera: Query<&Projection, With<Camera2d>>,
    dados: Option<ResMut<RiosDados>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut q: Query<(&CamadaRios, &mut Visibility)>,
) {
    let escala = camera.single().ok().map(|p| match p { Projection::Orthographic(o) => o.scale, _ => 1.0 }).unwrap_or(1.0);
    for (c, mut v) in &mut q {
        let alvo = if c.principal || escala < estilo.rios_menores_escala_max { Visibility::Inherited } else { Visibility::Hidden };
        if *v != alvo {
            *v = alvo;
        }
    }
    let Some(mut d) = dados else { return };
    let balde = (escala.max(0.05).log2() * 2.0).round() as i32;
    if balde == d.balde {
        return;
    }
    d.balde = balde;
    let esc = 2f32.powf(balde as f32 / 2.0);
    let (p, m) = construir_malhas_rios_escala(&d.arq, esc, &estilo);
    if !p.vazio() {
        let _ = meshes.insert(&d.malha_principal, p.construir());
    }
    if !m.vazio() {
        let _ = meshes.insert(&d.malha_menor, m.construir());
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
        assert!(estilo_rio(1).1.alpha() < 1.0);
    }
}
