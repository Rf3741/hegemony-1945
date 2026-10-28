//! Instalações industriais (fábricas, minas, poços, usinas, portos) e partículas de fumaça proporcionais à produção.

use crate::atlas::AtlasVisual;
use crate::cidade::IndiceEntidades;
use crate::config::VisualMapaConfig;
use crate::entrada::{AnoVisual, EntradaVisual};
use crate::era::{GerenciadorEras, indice_era_para_ano};
use crate::lod::{EstadoLod, NivelLod};
use bevy::prelude::*;

/// Categoria da instalação industrial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CategoriaIndustria {
    Fabrica,
    Mina,
    PocoPetroleo,
    Usina,
    Porto,
}

impl CategoriaIndustria {
    pub fn de_str(s: &str) -> Self {
        match s {
            "mina" => Self::Mina,
            "poco_petroleo" | "refinaria" => Self::PocoPetroleo,
            "usina" | "usina_carvao" | "usina_hidreletrica" | "usina_nuclear" => Self::Usina,
            "porto" | "aeroporto" | "estacao_ferroviaria" => Self::Porto,
            _ => Self::Fabrica,
        }
    }

    pub fn nome_pt(&self) -> &'static str {
        match self {
            Self::Fabrica => "Fábrica / Manufatura",
            Self::Mina => "Complexo Minerador",
            Self::PocoPetroleo => "Campo Petrolífero",
            Self::Usina => "Usina de Energia",
            Self::Porto => "Terminal Portuário",
        }
    }

    /// Cor representativa do ícone industrial.
    pub fn cor_icone(&self) -> Color {
        match self {
            Self::Fabrica => Color::srgb(0.75, 0.45, 0.25),      // Terracota industrial
            Self::Mina => Color::srgb(0.40, 0.42, 0.45),         // Cinza ardósia / carvão
            Self::PocoPetroleo => Color::srgb(0.18, 0.20, 0.22), // Preto betume
            Self::Usina => Color::srgb(0.85, 0.70, 0.20),        // Amarelo energia elétrica
            Self::Porto => Color::srgb(0.20, 0.55, 0.75),        // Azul marinho portuário
        }
    }

    /// Retorna se esta categoria de indústria possui chaminés ativas emissoras de fumaça.
    pub fn emite_fumaca(&self) -> bool {
        matches!(self, Self::Fabrica | Self::Mina | Self::Usina | Self::PocoPetroleo)
    }
}

/// Componente para uma instalação industrial individual.
#[derive(Component, Debug, Clone)]
pub struct InstalacaoIndustrial {
    pub id: String,
    pub nome: String,
    pub tipo: String,
    pub categoria: CategoriaIndustria,
    pub nivel: u8,
    pub ativa: bool,
    pub era_local: Option<usize>,
    pub era_idx: usize,
    pub producao_relativa: f32, // 0.0 a 1.0
    pub taxa_poluicao: f32,
    pub escala_animada: f32,
    pub timer_emissao_fumaca: f32,
}

/// Partícula de fumaça emitida por uma chaminé.
#[derive(Component, Debug, Clone)]
pub struct ParticulaFumaca {
    pub velocidade: Vec2,
    pub tempo_vida: f32,
    pub vida_maxima: f32,
    pub tamanho_inicial: f32,
    pub tamanho_final: f32,
}

/// Malha e material compartilhados da fumaça.
#[derive(Resource)]
pub struct RecursosFumaca {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

pub fn criar_recursos_fumaca(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(RecursosFumaca {
        mesh: meshes.add(Circle::new(1.0)),
        material: materials.add(ColorMaterial::from_color(Color::srgba(0.85, 0.85, 0.88, 0.30))),
    });
}

/// Tamanho do sprite de instalação por nível (unidades de mundo).
#[inline]
pub fn tamanho_sprite_industria(nivel: u8, escala: f32) -> f32 {
    (9.0 + nivel.min(5) as f32 * 1.5) * escala
}

/// Sincroniza as instalações de [`EntradaVisual`] (criação e atualização).
pub fn sincronizar_industrias_sistema(
    mut commands: Commands,
    entrada: Res<EntradaVisual>,
    cfg: Res<VisualMapaConfig>,
    atlas: Res<AtlasVisual>,
    mut indice: ResMut<IndiceEntidades>,
    mut query: Query<&mut InstalacaoIndustrial>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut recursos: Local<Option<(Handle<Mesh>, [Handle<ColorMaterial>; 5])>>,
) {
    let (mesh, mats) = recursos.get_or_insert_with(|| {
        let cats = [
            CategoriaIndustria::Fabrica, CategoriaIndustria::Mina, CategoriaIndustria::PocoPetroleo,
            CategoriaIndustria::Usina, CategoriaIndustria::Porto,
        ];
        (
            meshes.add(Rectangle::new(6.0 * cfg.escala_sprites, 6.0 * cfg.escala_sprites)),
            cats.map(|c| materials.add(ColorMaterial::from_color(c.cor_icone()))),
        )
    });
    let mut novas = 0;
    for i in entrada.instalacoes.iter().take(cfg.limite_instalacoes) {
        if let Some(&e) = indice.instalacoes.get(&i.id) {
            if let Ok(mut v) = query.get_mut(e) {
                v.nivel = i.nivel;
                v.ativa = i.ativa;
                v.producao_relativa = if i.ativa { i.producao.clamp(0.0, 1.0) } else { 0.0 };
                v.taxa_poluicao = i.poluicao;
                v.era_local = i.era;
            }
            continue;
        }
        let cat = CategoriaIndustria::de_str(&i.tipo);
        let pos = cfg.projecao.geo_para_mundo(i.posicao.0, i.posicao.1);
        let inst = InstalacaoIndustrial {
            id: i.id.clone(),
            nome: i.nome.clone(),
            tipo: i.tipo.clone(),
            categoria: cat,
            nivel: i.nivel,
            ativa: i.ativa,
            era_local: i.era,
            era_idx: i.era.unwrap_or(0).min(4),
            producao_relativa: if i.ativa { i.producao.clamp(0.0, 1.0) } else { 0.0 },
            taxa_poluicao: i.poluicao,
            escala_animada: if i.ativa { 1.0 } else { 0.0 },
            timer_emissao_fumaca: 0.0,
        };
        let tr = Transform::from_translation(pos.extend(14.0))
            .with_scale(if i.ativa { Vec3::ONE } else { Vec3::ZERO });
        let sprite = atlas
            .carregado
            .as_ref()
            .and_then(|a| a.indice.indice_industria(&i.tipo, inst.era_idx))
            .and_then(|k| atlas.sprite(k, tamanho_sprite_industria(i.nivel, cfg.escala_sprites)));
        let id_ent = if let Some(sprite) = sprite {
            commands.spawn((sprite, tr, inst, Visibility::default())).id()
        } else {
            let m = match cat {
                CategoriaIndustria::Fabrica => 0,
                CategoriaIndustria::Mina => 1,
                CategoriaIndustria::PocoPetroleo => 2,
                CategoriaIndustria::Usina => 3,
                CategoriaIndustria::Porto => 4,
            };
            commands.spawn((Mesh2d(mesh.clone()), MeshMaterial2d(mats[m].clone()), tr, inst, Visibility::default())).id()
        };
        indice.instalacoes.insert(i.id.clone(), id_ent);
        novas += 1;
    }
    if novas > 0 {
        info!("visual_mapa: {novas} instalações criadas.");
    }
}

/// Anima o aparecimento (por `ativa`), atualiza o quadro do atlas pela era e aplica a noite.
pub fn atualizar_industrias_sistema(
    ano: Res<AnoVisual>,
    time: Res<Time>,
    atlas: Res<AtlasVisual>,
    eras: Res<GerenciadorEras>,
    mut query: Query<(&mut InstalacaoIndustrial, &mut Transform, Option<&mut Sprite>)>,
) {
    let dt = time.delta_secs();
    let era_global = indice_era_para_ano(ano.0);
    let tinta = if eras.modo_noturno { Color::srgb(0.6, 0.65, 0.85) } else { Color::WHITE };
    for (mut ind, mut tr, sprite) in &mut query {
        let alvo = if ind.ativa { 1.0 } else { 0.0 };
        if (ind.escala_animada - alvo).abs() > 0.001 {
            ind.escala_animada += (alvo - ind.escala_animada) * (dt * 4.0).min(1.0);
            tr.scale = Vec3::splat(ind.escala_animada);
        } else if ind.escala_animada != alvo {
            ind.escala_animada = alvo;
            tr.scale = Vec3::splat(alvo);
        }
        let era = ind.era_local.unwrap_or(era_global).min(4);
        if era != ind.era_idx {
            ind.era_idx = era;
            if let (Some(mut s), Some(a)) = (sprite, atlas.carregado.as_ref()) {
                if let Some(k) = a.indice.indice_industria(&ind.tipo, era) {
                    if let Some(t) = s.texture_atlas.as_mut() {
                        t.index = k;
                    }
                }
            }
        } else if let Some(mut s) = sprite {
            if s.color != tinta {
                s.color = tinta;
            }
        }
    }
}

/// Emite fumaça das chaminés proporcionalmente à produção (apenas no zoom de perto Local).
pub fn emitir_fumaca_sistema(
    mut commands: Commands,
    time: Res<Time>,
    lod: Res<EstadoLod>,
    eras: Res<GerenciadorEras>,
    rec: Option<Res<RecursosFumaca>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut query: Query<(&mut InstalacaoIndustrial, &Transform)>,
) {
    let Some(rec) = rec else { return };
    if lod.nivel_atual != NivelLod::Local {
        return;
    }
    let dt = time.delta_secs();
    if let Some(mut m) = materials.get_mut(&rec.material) {
        let cor = eras.cor_fumaca_bevy();
        if m.color != cor {
            m.color = cor;
        }
    }
    for (mut ind, transform) in &mut query {
        if ind.escala_animada < 0.5 || !ind.categoria.emite_fumaca() || ind.producao_relativa <= 0.05 {
            continue;
        }
        ind.timer_emissao_fumaca += dt;
        let taxa = (ind.producao_relativa * ind.taxa_poluicao).clamp(0.1, 1.0);
        let intervalo = 2.5 / taxa;
        if ind.timer_emissao_fumaca >= intervalo {
            ind.timer_emissao_fumaca = 0.0;
            let pos = transform.translation.truncate() + Vec2::new(1.0, 2.5);
            commands.spawn((
                Mesh2d(rec.mesh.clone()),
                MeshMaterial2d(rec.material.clone()),
                Transform::from_translation(pos.extend(20.0)).with_scale(Vec3::splat(1.0)),
                ParticulaFumaca {
                    velocidade: Vec2::new(0.4 + (pos.x * 0.05).sin() * 0.4, 2.5 + taxa * 1.5),
                    tempo_vida: 0.0,
                    vida_maxima: 2.2,
                    tamanho_inicial: 0.8,
                    tamanho_final: 2.0,
                },
            ));
        }
    }
}

/// Move, expande e remove as partículas de fumaça (e limpa tudo ao sair do zoom de perto ou ocultar camada).
pub fn atualizar_particulas_fumaca_sistema(
    mut commands: Commands,
    time: Res<Time>,
    camada: Option<Res<crate::config::CamadaConstrucoesVisivel>>,
    lod: Option<Res<EstadoLod>>,
    mut query: Query<(Entity, &mut ParticulaFumaca, &mut Transform)>,
) {
    let visivel = camada.map_or(true, |c| c.0);
    let zoom_perto = lod.map_or(true, |l| l.nivel_atual == NivelLod::Local);

    if !visivel || !zoom_perto {
        for (e, _, _) in &query {
            commands.entity(e).despawn();
        }
        return;
    }

    let dt = time.delta_secs();
    for (e, mut p, mut tr) in &mut query {
        p.tempo_vida += dt;
        if p.tempo_vida >= p.vida_maxima {
            commands.entity(e).despawn();
            continue;
        }
        let prog = p.tempo_vida / p.vida_maxima;
        tr.translation.x += p.velocidade.x * dt;
        tr.translation.y += p.velocidade.y * dt;
        tr.scale = Vec3::splat(p.tamanho_inicial + (p.tamanho_final - p.tamanho_inicial) * prog);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categoria_industria() {
        assert_eq!(CategoriaIndustria::de_str("mina"), CategoriaIndustria::Mina);
        assert_eq!(CategoriaIndustria::de_str("porto"), CategoriaIndustria::Porto);
        assert_eq!(CategoriaIndustria::de_str("usina_nuclear"), CategoriaIndustria::Usina);
        assert_eq!(CategoriaIndustria::de_str("refinaria"), CategoriaIndustria::PocoPetroleo);
        assert!(CategoriaIndustria::Fabrica.emite_fumaca());
        assert!(!CategoriaIndustria::Porto.emite_fumaca());
    }

    #[test]
    fn sprite_industria_cresce() {
        assert!(tamanho_sprite_industria(5, 1.0) > tamanho_sprite_industria(1, 1.0));
    }
}
