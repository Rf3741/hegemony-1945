//! Renderização, layout em espiral e animação dos prédios da economia real.
//!
//! - Layout sem sobreposição: Espiral de Fermat concêntrica ao redor da capital/centroide.
//! - Agregação por estado: um sprite representativo por tipo de prédio com tamanho proporcional ao nível total.
//! - Animação de surgimento suave (spawn) e de pulso ("pop") apenas para novas obras concluídas em zoom de perto.
//! - Fumaça sutil, cinza-clara e semitransparente, visível exclusivamente no zoom de perto.
//! - Integração com LOD: prédios econômicos visíveis apenas em `NivelLod::Local`.

use crate::atlas::AtlasVisual;
use crate::config::VisualMapaConfig;
use crate::entrada::{AnoVisual, BlocoVisual, EntradaEconomiaVisual, PredioEconomiaEntrada};
use crate::era::{GerenciadorEras, indice_era_para_ano};
use crate::industria::{ParticulaFumaca, RecursosFumaca};
use crate::lod::{DetalheLod, MarcadorLod, NivelLod};
use crate::mapeamento::CatalogoMapeamentoPredios;
use bevy::prelude::*;
use std::collections::HashMap;

/// Ângulo áureo em radianos (137.507764°): garante empacotamento espacial uniforme na espiral.
pub const ANGULO_AUREO_RAD: f32 = 2.3999632;

/// Calcula o deslocamento 2D (offset) para o k-ésimo prédio ao redor do centroide geográfico.
/// Utiliza a espiral de Fermat ($r = c \sqrt{k}$), que distribui os pontos com densidade uniforme
/// sem sobreposição e sem artefatos de grade.
#[inline]
pub fn calcular_offset_espiral(indice: usize, espacamento: f32) -> Vec2 {
    if indice == 0 {
        return Vec2::ZERO;
    }
    let theta = indice as f32 * ANGULO_AUREO_RAD;
    let r = espacamento * (indice as f32).sqrt();
    Vec2::new(r * theta.cos(), r * theta.sin())
}

/// Tamanho base do sprite em unidades de mundo, escalado sublinearmente com o nível do prédio.
#[inline]
pub fn tamanho_sprite_predio(nivel: u32, escala: f32) -> f32 {
    let bonus = (nivel.min(100) as f32).sqrt() * 2.2;
    (10.0 + bonus) * escala
}

/// Componente que representa um prédio individual desenhado no mapa.
#[derive(Component, Debug, Clone)]
pub struct PredioVisual {
    pub id: String,
    pub tipo: String,
    pub estado_id: String,
    pub provincia_id: Option<String>,
    pub pais_tag: String,
    pub nivel: u32,
    pub nivel_anterior: u32,
    pub bloco: BlocoVisual,
    pub era_idx: usize,
    pub producao: f32,
    pub escala_animada: f32,
    pub timer_pop_nivel: f32,
    pub timer_fumaca: f32,
    pub emite_fumaca: bool,
    pub tipo_fumaca: Option<String>,
}

/// Componente para o registro agregado de estado.
#[derive(Component, Debug, Clone)]
pub struct AgregadoEstadoVisual {
    pub estado_id: String,
    pub total_predios: u32,
    pub total_niveis: u32,
    pub predios_excedentes: u32,
}

/// Marcador para o texto da contagem dentro do ícone de estado.
#[derive(Component, Debug, Clone, Copy)]
pub struct TextoContagemEstado;

/// Índice de entidades criadas para os prédios da economia e registros estaduais.
#[derive(Resource, Default)]
pub struct IndiceEntidadesEconomia {
    pub predios: HashMap<String, Entity>,
    pub agregados_estado: HashMap<String, Entity>,
}

/// Recursos de malha e materiais para fallback e ícones de estado.
#[derive(Resource)]
pub struct RecursosPredios {
    pub mesh_quad: Handle<Mesh>,
    pub mesh_badge: Handle<Mesh>,
    pub mat_badge_fundo: Handle<ColorMaterial>,
    pub mat_badge_borda: Handle<ColorMaterial>,
}

pub fn setup_recursos_predios(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(RecursosPredios {
        mesh_quad: meshes.add(Rectangle::new(1.0, 1.0)),
        mesh_badge: meshes.add(Circle::new(7.0)),
        mat_badge_fundo: materials.add(ColorMaterial::from_color(Color::srgba(0.12, 0.16, 0.22, 0.90))),
        mat_badge_borda: materials.add(ColorMaterial::from_color(Color::srgb(0.95, 0.75, 0.25))),
    });
}

/// Sincroniza a [`EntradaEconomiaVisual`] com as entidades do mundo Bevy,
/// agregando construções por estado e tipo de prédio.
pub fn sincronizar_economia_sistema(
    mut commands: Commands,
    entrada: Res<EntradaEconomiaVisual>,
    cfg: Res<VisualMapaConfig>,
    atlas: Res<AtlasVisual>,
    catalogo: Res<CatalogoMapeamentoPredios>,
    ano: Res<AnoVisual>,
    estado_lod: Option<Res<crate::lod::EstadoLod>>,
    mut indice: ResMut<IndiceEntidadesEconomia>,
    mut query_predios: Query<(&mut PredioVisual, &mut Transform)>,
    mut query_agregados: Query<&mut AgregadoEstadoVisual>,
    rec: Option<Res<RecursosPredios>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Some(rec) = rec else { return };
    let era_global = indice_era_para_ano(ano.0);
    let primeira_carga = indice.predios.is_empty();
    let zoom_perto = estado_lod.as_ref().map_or(true, |lod| lod.nivel_atual == NivelLod::Local);

    // 1. Agrupar prédios por estado
    let mut por_estado: HashMap<&str, Vec<&PredioEconomiaEntrada>> = HashMap::new();
    for p in &entrada.predios {
        if p.ativa {
            por_estado.entry(&p.estado_id).or_default().push(p);
        }
    }

    // 2. Processar cada estado
    for (estado_id, lista) in por_estado {
        let total_predios = lista.len() as u32;
        let total_niveis: u32 = lista.iter().map(|p| p.nivel).sum();

        // Determina a posição central do estado
        let centro_mundo = if let Some(p_com_pos) = lista.iter().find(|p| p.posicao.is_some()) {
            let (lon, lat) = p_com_pos.posicao.unwrap();
            cfg.projecao.geo_para_mundo(lon, lat)
        } else {
            let hash = estado_id.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
            let lon = ((hash % 360) as f32) - 180.0;
            let lat = (((hash / 360) % 140) as f32) - 70.0;
            cfg.projecao.geo_para_mundo(lon, lat)
        };

        // Mantém a entidade de AgregadoEstadoVisual para integridade do ECS
        if let Some(&ent_agregado) = indice.agregados_estado.get(estado_id) {
            if let Ok(mut ag) = query_agregados.get_mut(ent_agregado) {
                ag.total_predios = total_predios;
                ag.total_niveis = total_niveis;
                ag.predios_excedentes = 0;
            }
        } else {
            let ent_badge = commands
                .spawn((
                    AgregadoEstadoVisual {
                        estado_id: estado_id.to_string(),
                        total_predios,
                        total_niveis,
                        predios_excedentes: 0,
                    },
                    MarcadorLod,
                    Visibility::Hidden,
                ))
                .id();
            indice.agregados_estado.insert(estado_id.to_string(), ent_badge);
        }

        // 3. Agregação por estado e tipo (Feedback 6: um sprite representativo por tipo com nível total como tamanho)
        let mut por_tipo: HashMap<String, (u32, f32, String, BlocoVisual, Option<(f32, f32)>)> = HashMap::new();
        for p in &lista {
            let bloco = p.bloco.unwrap_or_else(|| BlocoVisual::de_pais(&p.pais_tag));
            let entry = por_tipo.entry(p.tipo.clone()).or_insert((0, 0.0, p.pais_tag.clone(), bloco, p.posicao));
            entry.0 += p.nivel;
            entry.1 += p.producao * (p.nivel as f32);
            if entry.4.is_none() && p.posicao.is_some() {
                entry.4 = p.posicao;
            }
        }

        let mut tipos_ordenados: Vec<_> = por_tipo.into_iter().collect();
        tipos_ordenados.sort_by(|a, b| b.1.0.cmp(&a.1.0).then_with(|| a.0.cmp(&b.0)));

        for (k, (tipo, (soma_niveis, soma_prod, pais_tag, bloco, pos_tipo))) in tipos_ordenados.into_iter().enumerate() {
            let offset = calcular_offset_espiral(k, cfg.espacamento_predios);
            let pos_mundo = if let Some((lon, lat)) = pos_tipo {
                cfg.projecao.geo_para_mundo(lon, lat) + offset * 0.4
            } else {
                centro_mundo + offset
            };

            let producao_media = if soma_niveis > 0 { soma_prod / (soma_niveis as f32) } else { 1.0 };
            let era_idx = era_global.min(4);
            let tam = tamanho_sprite_predio(soma_niveis, cfg.escala_sprites);
            let id_agregado = format!("{}:{}", estado_id, tipo);

            if let Some(&ent) = indice.predios.get(&id_agregado) {
                if let Ok((mut pv, mut tr)) = query_predios.get_mut(ent) {
                    if soma_niveis != pv.nivel {
                        pv.nivel_anterior = pv.nivel;
                        pv.nivel = soma_niveis;
                        if soma_niveis > pv.nivel_anterior && zoom_perto {
                            pv.timer_pop_nivel = 1.0;
                        }
                    }
                    pv.producao = producao_media;
                    pv.bloco = bloco;
                    pv.era_idx = era_idx;
                    tr.translation = pos_mundo.extend(6.0);
                }
            } else {
                let emite_fumaca = catalogo.emite_fumaca(&tipo);
                let tipo_fumaca = catalogo.tipo_fumaca(&tipo).map(String::from);
                let escala_inicial = if primeira_carga || !zoom_perto { 1.0 } else { 0.0 };

                let comp_predio = PredioVisual {
                    id: id_agregado.clone(),
                    tipo: tipo.clone(),
                    estado_id: estado_id.to_string(),
                    provincia_id: None,
                    pais_tag: pais_tag.clone(),
                    nivel: soma_niveis,
                    nivel_anterior: soma_niveis,
                    bloco,
                    era_idx,
                    producao: producao_media,
                    escala_animada: escala_inicial,
                    timer_pop_nivel: 0.0,
                    timer_fumaca: 0.0,
                    emite_fumaca,
                    tipo_fumaca,
                };

                let mut cmd = commands.spawn((
                    comp_predio,
                    Transform::from_translation(pos_mundo.extend(6.0)).with_scale(Vec3::splat(escala_inicial)),
                    DetalheLod,
                    Visibility::Hidden,
                ));

                // Sprite via Atlas de Texturas
                let sprite_carregado = if atlas.disponivel() {
                    catalogo
                        .resolver_sprite(&tipo, bloco, ano.0)
                        .and_then(|sprite_id| atlas.sprite_por_id(sprite_id, tam))
                } else {
                    None
                };

                if let Some(sprite) = sprite_carregado {
                    cmd.insert(sprite);
                } else {
                    // Fallback Procedural
                    let cor = catalogo.cor_procedural(&tipo);
                    let mat = materials.add(ColorMaterial::from_color(cor));
                    cmd.insert((
                        Mesh2d(rec.mesh_quad.clone()),
                        MeshMaterial2d(mat),
                    ));
                }

                let id_ent = cmd.id();
                indice.predios.insert(id_agregado, id_ent);

                // Mapeia também os IDs individuais para acesso transparente
                for p in &lista {
                    if p.tipo == tipo {
                        indice.predios.insert(p.id.clone(), id_ent);
                    }
                }
            }
        }
    }
}

/// Animação de surgimento suave (spawn), pulso ao subir de nível e atualização da era.
pub fn animar_predios_sistema(
    time: Res<Time>,
    cfg: Res<VisualMapaConfig>,
    camada: Option<Res<crate::config::CamadaConstrucoesVisivel>>,
    ano: Res<AnoVisual>,
    atlas: Res<AtlasVisual>,
    catalogo: Res<CatalogoMapeamentoPredios>,
    eras: Res<GerenciadorEras>,
    mut query: Query<(&mut PredioVisual, &mut Transform, Option<&mut Sprite>)>,
) {
    if !camada.map_or(true, |c| c.0) {
        return;
    }
    let dt = time.delta_secs();
    let era_global = indice_era_para_ano(ano.0);
    let tinta = if eras.modo_noturno {
        Color::srgb(0.65, 0.70, 0.90)
    } else {
        Color::WHITE
    };

    for (mut p, mut tr, mut sprite) in &mut query {
        if p.escala_animada < 1.0 {
            p.escala_animada = (p.escala_animada + dt * 3.5).min(1.0);
        }

        let mut fator_pop = 1.0;
        if p.timer_pop_nivel > 0.0 {
            p.timer_pop_nivel = (p.timer_pop_nivel - dt * 2.5).max(0.0);
            fator_pop = 1.0 + (p.timer_pop_nivel * std::f32::consts::PI).sin() * 0.35;
        }

        let tam_base = tamanho_sprite_predio(p.nivel, cfg.escala_sprites);
        let escala_efetiva = p.escala_animada * fator_pop;

        if let Some(ref mut s) = sprite {
            s.custom_size = Some(Vec2::splat(tam_base));
            tr.scale = Vec3::splat(escala_efetiva);
            if s.color != tinta {
                s.color = tinta;
            }

            // Atualização de era visual se o relógio mudou de era
            if p.era_idx != era_global {
                p.era_idx = era_global;
                if let Some(a) = atlas.carregado.as_ref() {
                    if let Some(sprite_id) = catalogo.resolver_sprite(&p.tipo, p.bloco, ano.0) {
                        if let Some(idx) = a.indice.indice_por_id(sprite_id) {
                            if let Some(ref mut ta) = s.texture_atlas {
                                ta.index = idx;
                            }
                        }
                    }
                }
            }
        } else {
            // Procedural quad
            tr.scale = Vec3::new(tam_base * escala_efetiva, tam_base * escala_efetiva, 1.0);
        }
    }
}

/// Emissão de partículas de fumaça das chaminés industriais (pequena, cinza-clara, apenas no zoom de perto).
pub fn emitir_fumaca_predios_sistema(
    mut commands: Commands,
    time: Res<Time>,
    camada: Option<Res<crate::config::CamadaConstrucoesVisivel>>,
    estado_lod: Option<Res<crate::lod::EstadoLod>>,
    limites_zoom: Option<Res<crate::lod::LimitesZoomVisual>>,
    rec_fumaca: Option<Res<RecursosFumaca>>,
    mut query: Query<(&mut PredioVisual, &Transform)>,
) {
    if !camada.map_or(true, |c| c.0) {
        return;
    }
    // Fumaça: SÓ no zoom de perto (Local)
    if !estado_lod.as_ref().map_or(true, |lod| lod.nivel_atual == NivelLod::Local) {
        return;
    }
    if !limites_zoom.as_ref().map_or(true, |lim| lim.fumaca_ativa) {
        return;
    }
    let Some(rec) = rec_fumaca else { return };
    let dt = time.delta_secs();

    for (mut p, tr) in &mut query {
        if !p.emite_fumaca || p.producao <= 0.1 || p.escala_animada < 0.9 {
            continue;
        }

        p.timer_fumaca += dt;
        let taxa = p.producao.clamp(0.2, 1.0);
        let intervalo = 2.5 / taxa;

        if p.timer_fumaca >= intervalo {
            p.timer_fumaca = 0.0;
            let tam_base = tamanho_sprite_predio(p.nivel, 1.0);
            let pos_topo = tr.translation.truncate() + Vec2::new(0.0, tam_base * 0.40);
            commands.spawn((
                Mesh2d(rec.mesh.clone()),
                MeshMaterial2d(rec.material.clone()),
                Transform::from_translation(pos_topo.extend(22.0)).with_scale(Vec3::splat(1.0)),
                ParticulaFumaca {
                    velocidade: Vec2::new(0.4 + (pos_topo.x * 0.1).sin() * 0.4, 2.5 + taxa * 1.5),
                    tempo_vida: 0.0,
                    vida_maxima: 2.2,
                    tamanho_inicial: 0.8,
                    tamanho_final: 2.0,
                },
            ));
        }
    }
}

/// Alterna a visibilidade da camada de construções no mapa ao pressionar a tecla V.
pub fn alternar_camada_construcoes_sistema(
    teclas: Option<Res<ButtonInput<KeyCode>>>,
    mut camada: ResMut<crate::config::CamadaConstrucoesVisivel>,
    estado_lod: Res<crate::lod::EstadoLod>,
    mut query_detalhes: Query<&mut Visibility, (With<crate::lod::DetalheLod>, Without<crate::lod::MarcadorLod>)>,
    mut query_marcadores: Query<&mut Visibility, (With<crate::lod::MarcadorLod>, Without<crate::lod::DetalheLod>)>,
) {
    let Some(teclas) = teclas else { return };
    if teclas.just_pressed(KeyCode::KeyV) {
        camada.0 = !camada.0;
        let (vm, vd) = if camada.0 {
            crate::lod::visibilidades(estado_lod.nivel_atual)
        } else {
            (Visibility::Hidden, Visibility::Hidden)
        };
        for mut v in &mut query_marcadores {
            *v = vm;
        }
        for mut v in &mut query_detalhes {
            *v = vd;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn espiral_fermat_sem_sobreposicao() {
        let mut posicoes: Vec<Vec2> = Vec::new();
        let espacamento = 15.0;
        for i in 0..100 {
            let pos = calcular_offset_espiral(i, espacamento);
            if i > 0 {
                assert!(pos.length() > 0.0);
            }
            for &outra in &posicoes {
                assert!((pos - outra).length() > 1.0, "Sobreposição entre prédios na espiral!");
            }
            posicoes.push(pos);
        }
    }

    #[test]
    fn tamanho_predio_monotonico_com_nivel() {
        assert!(tamanho_sprite_predio(1, 1.0) < tamanho_sprite_predio(5, 1.0));
        assert!(tamanho_sprite_predio(5, 1.0) < tamanho_sprite_predio(10, 1.0));
        assert!(tamanho_sprite_predio(10, 1.0) < tamanho_sprite_predio(20, 1.0));
    }
}
