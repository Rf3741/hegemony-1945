//! Cidades níveis 1 a 8: sprite do atlas (ou prédios procedurais como reserva), animação de
//! crescimento e iluminação noturna. Os dados vêm de [`EntradaVisual`].

use crate::atlas::AtlasVisual;
use crate::config::VisualMapaConfig;
use crate::entrada::{AnoVisual, BlocoVisual, EntradaVisual};
use crate::era::{GerenciadorEras, indice_era_para_ano};
use crate::lod::MarcadorLod;
use bevy::prelude::*;
use std::collections::HashMap;

/// Componente que define uma cidade no mundo (antes `Cidade`).
#[derive(Component, Debug, Clone)]
pub struct CidadeVisual {
    pub id: String,
    pub nome: String,
    pub pais_tag: String,
    /// (longitude, latitude) em graus.
    pub posicao: (f32, f32),
    pub posicao_mundo: Vec2,
    pub bloco: BlocoVisual,
    /// Era local pedida pela entrada (`None` = global).
    pub era_local: Option<usize>,
    /// Era efetiva usada no sprite.
    pub era_idx: usize,
    pub nivel_atual_fracionario: f32,
    pub nivel_atual_inteiro: u8,
    pub populacao: u32,
    pub eh_capital: bool,
    /// Temporizador da animação de pop ao subir de nível [0.0..=1.0].
    pub timer_animacao_nivel: f32,
    /// Nome original (antes das trocas históricas); `nome` é o nome vigente no ano.
    pub nome_base: String,
    pub ano_fundacao: Option<u16>,
    pub mudancas_nome: Vec<(u16, String)>,
    pub eh_porto: bool,
    /// `false` antes do ano de fundação (a cidade fica oculta).
    pub existe: bool,
}

/// Índices id -> entidade, mantidos pelos sistemas de sincronia.
#[derive(Resource, Default)]
pub struct IndiceEntidades {
    pub cidades: HashMap<String, Entity>,
    pub instalacoes: HashMap<String, Entity>,
    pub conexoes: HashMap<String, Entity>,
}

/// Sprite do atlas que representa a cidade (filho da entidade `CidadeVisual`).
#[derive(Component, Debug, Clone, Copy)]
pub struct SpriteCidade;

/// Rótulo de texto com o nome da cidade (filho da entidade `CidadeVisual`).
#[derive(Component, Debug, Clone, Copy)]
pub struct RotuloNomeCidade;

/// Tipo estrutural do edifício urbano.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoPredioUrbano {
    ResidenciaBaixa,
    EdificioMedio,
    BlocoResidencial,
    TorreComercial,
    ArranhaCeu,
}

/// Prédio procedural individual (reserva quando não há atlas).
#[derive(Component, Debug, Clone)]
pub struct PredioCidade {
    pub cidade_id: String,
    pub offset_local: Vec2,
    pub tamanho: Vec2,
    pub altura_base: f32,
    pub nivel_requerido: u8,
    pub tipo: TipoPredioUrbano,
    /// Escala animada atual (0.0 invisível até 1.0 construído).
    pub escala_animada: f32,
    pub semente_cor: f32,
}

/// Nível inteiro 1..=8 a partir do nível fracionário da entrada.
#[inline]
pub fn nivel_inteiro(nivel: f32) -> u8 {
    (nivel.floor().max(1.0) as u8).min(8)
}

/// Tamanho do sprite (unidades de mundo) por nível.
#[inline]
pub fn tamanho_sprite_cidade(nivel: u8, escala: f32) -> f32 {
    (5.0 + nivel as f32 * 1.6) * escala
}

/// Gera deterministicamente a lista de prédios para uma dada cidade.
pub fn gerar_predios_cidade(id: &str, nivel_inicial: u8, nivel_max: u8) -> Vec<PredioCidade> {
    let mut predios = Vec::new();
    let semente = id.bytes().fold(1945u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let mut rng_state = semente;

    let mut proximo_f32 = || -> f32 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((rng_state >> 32) as u32 as f32) / (u32::MAX as f32)
    };

    // Quantidade total de prédios proporcional ao nível máximo projetado em 1991 (até ~60 prédios por megalópole)
    let total_predios = match nivel_max {
        1 => 4,
        2 => 8,
        3 => 14,
        4 => 22,
        5 => 32,
        6 => 44,
        7 => 58,
        _ => 72,
    };

    for i in 0..total_predios {
        // Nível em que o prédio surge na cidade (1..=8)
        let nivel_req = ((i as f32 / total_predios as f32) * 8.0).floor() as u8 + 1;
        
        // Disposição radial em anéis orgânicos ao redor do centro
        let raio_base = 3.0 + (nivel_req as f32) * 2.8 + proximo_f32() * 3.5;
        let angulo = proximo_f32() * std::f32::consts::TAU;
        let offset = Vec2::new(raio_base * angulo.cos(), raio_base * angulo.sin() * 0.7);

        let (tipo, tam_x, tam_y, alt) = if nivel_req >= 7 {
            (TipoPredioUrbano::ArranhaCeu, 4.0 + proximo_f32() * 2.5, 4.0 + proximo_f32() * 2.5, 14.0 + proximo_f32() * 10.0)
        } else if nivel_req >= 5 {
            (TipoPredioUrbano::TorreComercial, 4.5 + proximo_f32() * 2.0, 4.5 + proximo_f32() * 2.0, 10.0 + proximo_f32() * 6.0)
        } else if nivel_req >= 3 {
            (TipoPredioUrbano::BlocoResidencial, 5.0 + proximo_f32() * 2.0, 3.5 + proximo_f32() * 2.0, 6.0 + proximo_f32() * 4.0)
        } else if nivel_req >= 2 {
            (TipoPredioUrbano::EdificioMedio, 4.0 + proximo_f32() * 1.5, 4.0 + proximo_f32() * 1.5, 4.0 + proximo_f32() * 3.0)
        } else {
            (TipoPredioUrbano::ResidenciaBaixa, 3.5 + proximo_f32() * 1.5, 3.0 + proximo_f32() * 1.5, 2.5 + proximo_f32() * 2.0)
        };

        predios.push(PredioCidade {
            cidade_id: id.to_string(),
            offset_local: offset,
            tamanho: Vec2::new(tam_x, tam_y),
            altura_base: alt,
            nivel_requerido: nivel_req,
            tipo,
            escala_animada: if nivel_req <= nivel_inicial { 1.0 } else { 0.0 },
            semente_cor: proximo_f32(),
        });
    }

    predios
}

/// Sincroniza as cidades de [`EntradaVisual`]: cria as novas e atualiza as existentes.
pub fn sincronizar_cidades_sistema(
    mut commands: Commands,
    entrada: Res<EntradaVisual>,
    cfg: Res<VisualMapaConfig>,
    atlas: Res<AtlasVisual>,
    eras: Res<GerenciadorEras>,
    mut indice: ResMut<IndiceEntidades>,
    mut cidades: Query<&mut CidadeVisual>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    fonte: Option<Res<crate::cidade_v3::FonteRotulos>>,
    ano: Res<AnoVisual>,
    mut recursos: Local<Option<RecursosCidade>>,
) {
    let rec = recursos.get_or_insert_with(|| RecursosCidade {
        mesh_predio: meshes.add(Rectangle::new(1.0, 1.0)),
        mesh_marcador: meshes.add(Circle::new(2.5 * cfg.escala_sprites)),
        mat_predio: materials.add(ColorMaterial::from_color(eras.cor_primaria_bevy())),
        mat_capital: materials.add(ColorMaterial::from_color(Color::srgba(0.95, 0.80, 0.25, 0.85))),
        mat_cidade: materials.add(ColorMaterial::from_color(Color::srgba(0.85, 0.88, 0.92, 0.70))),
    });
    let mut novas = 0;
    for c in &entrada.cidades {
        if let Some(&e) = indice.cidades.get(&c.id) {
            if let Ok(mut v) = cidades.get_mut(e) {
                v.nivel_atual_fracionario = c.nivel.clamp(1.0, 8.0);
                v.populacao = c.populacao;
                v.eh_capital = c.eh_capital;
                v.era_local = c.era;
                v.eh_porto = c.eh_porto;
            }
            continue;
        }
        let pos = cfg.projecao.geo_para_mundo(c.posicao.0, c.posicao.1);
        let niv = nivel_inteiro(c.nivel);
        let bloco = c.bloco.unwrap_or_else(|| BlocoVisual::de_pais(&c.pais_tag));
        let cidade = CidadeVisual {
            id: c.id.clone(),
            nome: c.nome.clone(),
            pais_tag: c.pais_tag.clone(),
            posicao: c.posicao,
            posicao_mundo: pos,
            bloco,
            era_local: c.era,
            era_idx: 0,
            nivel_atual_fracionario: c.nivel.clamp(1.0, 8.0),
            nivel_atual_inteiro: niv,
            populacao: c.populacao,
            eh_capital: c.eh_capital,
            timer_animacao_nivel: 0.0,
            nome_base: c.nome.clone(),
            ano_fundacao: c.ano_fundacao,
            mudancas_nome: c.mudancas_nome.clone(),
            eh_porto: c.eh_porto,
            existe: crate::cidade_v3::existe_no_ano(c.ano_fundacao, ano.0),
        };
        let existe_agora = cidade.existe;
        let mat_lod = if c.eh_capital { rec.mat_capital.clone() } else { rec.mat_cidade.clone() };
        let mut ent = commands.spawn((
            cidade,
            Transform::from_translation(pos.extend(10.0)),
            if existe_agora { Visibility::Inherited } else { Visibility::Hidden },
        ));
        let id_ent = ent.id();
        ent.with_children(|p| {
            p.spawn((
                Mesh2d(rec.mesh_marcador.clone()),
                MeshMaterial2d(mat_lod),
                Transform::from_xyz(0.0, 0.0, 5.0),
                MarcadorLod,
            ));
            let fonte_h = fonte.as_ref().map(|f| f.0.clone());
            let mk_fonte = || {
                let mut tf = TextFont::from_font_size(crate::cidade_v3::FONTE_BASE_PX);
                if let Some(h) = fonte_h.clone() {
                    tf.font = h.into();
                }
                tf
            };
            // Os detalhes (prédios, luzes, fumaça, guindastes, tráfego) são criados sob demanda
            // por `cidade_v3::materializar_detalhes_sistema`.
            p.spawn((
                Text2d::new(c.nome.clone()),
                mk_fonte(),
                TextColor(Color::srgba(0.96, 0.95, 0.90, 0.95)),
                Transform::from_xyz(0.0, -8.0, 6.0),
                Visibility::Hidden,
                RotuloNomeCidade,
            ))
            .with_children(|r| {
                r.spawn((
                    Text2d::new(c.nome.clone()),
                    mk_fonte(),
                    TextColor(Color::srgba(0.02, 0.03, 0.06, 0.85)),
                    Transform::from_xyz(0.9, -0.9, -0.1),
                    crate::cidade_v3::SombraRotulo,
                ));
            });
        });
        indice.cidades.insert(c.id.clone(), id_ent);
        novas += 1;
    }
    if novas > 0 {
        info!("visual_mapa: {novas} cidades criadas (atlas: {}).", atlas.disponivel());
    }
}

/// Recursos compartilhados (batching) das cidades.
#[allow(dead_code)]
pub struct RecursosCidade {
    mesh_predio: Handle<Mesh>,
    mesh_marcador: Handle<Mesh>,
    mat_predio: Handle<ColorMaterial>,
    mat_capital: Handle<ColorMaterial>,
    mat_cidade: Handle<ColorMaterial>,
}

/// Aplica o nível alvo: nível inteiro, era efetiva e o pop de crescimento.
pub fn atualizar_cidades_sistema(
    ano: Res<AnoVisual>,
    time: Res<Time>,
    mut cidades: Query<&mut CidadeVisual>,
) {
    let dt = time.delta_secs();
    let era_global = indice_era_para_ano(ano.0);
    for mut c in &mut cidades {
        let novo = nivel_inteiro(c.nivel_atual_fracionario);
        let era = c.era_local.unwrap_or(era_global).min(4);
        // Só escreve quando muda (evita marcar o componente todo quadro).
        if novo != c.nivel_atual_inteiro {
            if novo > c.nivel_atual_inteiro {
                c.timer_animacao_nivel = 1.0;
            }
            c.nivel_atual_inteiro = novo;
        }
        if era != c.era_idx {
            c.era_idx = era;
        }
        if c.timer_animacao_nivel > 0.0 {
            c.timer_animacao_nivel = (c.timer_animacao_nivel - dt * 2.0).max(0.0);
        }
    }
}

/// Troca o quadro do atlas, anima o "pop" e aplica a tinta noturna nos sprites das cidades.
pub fn atualizar_sprites_cidade_sistema(
    cfg: Res<VisualMapaConfig>,
    atlas: Res<AtlasVisual>,
    eras: Res<GerenciadorEras>,
    cidades: Query<&CidadeVisual>,
    mut sprites: Query<(&ChildOf, &mut Sprite, &mut Transform), With<SpriteCidade>>,
) {
    let Some(a) = atlas.carregado.as_ref() else { return };
    let tinta = if eras.modo_noturno { Color::srgb(0.55, 0.6, 0.85) } else { Color::WHITE };
    for (pai, mut sprite, mut tr) in &mut sprites {
        let Ok(c) = cidades.get(pai.parent()) else { continue };
        if let Some(i) = a.indice.indice_cidade(c.bloco.chave_atlas(), c.nivel_atual_inteiro, c.era_idx) {
            if sprite.texture_atlas.as_ref().is_some_and(|t| t.index != i) {
                if let Some(t) = sprite.texture_atlas.as_mut() {
                    t.index = i;
                }
                sprite.custom_size = Some(Vec2::splat(tamanho_sprite_cidade(c.nivel_atual_inteiro, cfg.escala_sprites)));
            }
        }
        if sprite.color != tinta {
            sprite.color = tinta;
        }
        let pop = if c.timer_animacao_nivel > 0.0 {
            1.0 + (c.timer_animacao_nivel * std::f32::consts::PI).sin() * 0.35
        } else {
            1.0
        };
        if (tr.scale.x - pop).abs() > 1e-3 {
            tr.scale = Vec3::new(pop, pop, 1.0);
        }
    }
}

/// Anima a escala dos prédios procedurais conforme a cidade sobe de nível (reserva sem atlas).
pub fn animar_predios_cidade_sistema(
    time: Res<Time>,
    eras: Res<GerenciadorEras>,
    lod: Option<Res<crate::lod::EstadoLod>>,
    mut materiais: ResMut<Assets<ColorMaterial>>,
    cidades: Query<&CidadeVisual>,
    mut predios: Query<(&ChildOf, &mut PredioCidade, &mut Transform, &MeshMaterial2d<ColorMaterial>)>,
) {
    if lod.as_ref().is_some_and(|l| l.nivel_atual != crate::lod::NivelLod::Local) {
        return;
    }
    let dt = time.delta_secs();
    let altura_era = eras.multiplicador_altura();
    let mut cor_aplicada = false;
    for (pai, mut p, mut tr, mat) in &mut predios {
        if !cor_aplicada {
            // Material compartilhado: atualiza uma vez por quadro com a cor da era (e noite).
            if let Some(mut m) = materiais.get_mut(&mat.0) {
                let base = eras.cor_primaria_bevy();
                m.color = if eras.modo_noturno { eras.cor_janela_bevy() } else { base };
            }
            cor_aplicada = true;
        }
        let Ok(c) = cidades.get(pai.parent()) else { continue };
        let alvo = if c.nivel_atual_inteiro >= p.nivel_requerido { 1.0 } else { 0.0 };
        if (p.escala_animada - alvo).abs() > 0.001 {
            p.escala_animada += (alvo - p.escala_animada) * (dt * 5.0).min(1.0);
        } else {
            p.escala_animada = alvo;
        }
        let pop = if c.nivel_atual_inteiro == p.nivel_requerido && c.timer_animacao_nivel > 0.0 {
            1.0 + (c.timer_animacao_nivel * std::f32::consts::PI).sin() * 0.35
        } else {
            1.0
        };
        let e = p.escala_animada * pop;
        let novo_s = Vec3::new(p.tamanho.x * e, p.tamanho.y * e * altura_era, 1.0);
        if (tr.scale.x - novo_s.x).abs() > 1e-4 {
            tr.scale = novo_s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gerar_predios_cidade_basico() {
        let predios = gerar_predios_cidade("teste_cidade", 4, 7);
        assert!(predios.len() >= 20);
        let iniciais = predios.iter().filter(|p| p.nivel_requerido <= 4).count();
        let ativos = predios.iter().filter(|p| p.escala_animada > 0.0).count();
        assert_eq!(iniciais, ativos);
        // determinístico
        assert_eq!(predios[3].offset_local, gerar_predios_cidade("teste_cidade", 4, 7)[3].offset_local);
    }

    #[test]
    fn nivel_inteiro_limites() {
        assert_eq!(nivel_inteiro(0.2), 1);
        assert_eq!(nivel_inteiro(3.9), 3);
        assert_eq!(nivel_inteiro(12.0), 8);
    }

    #[test]
    fn sprite_cresce_com_nivel() {
        assert!(tamanho_sprite_cidade(8, 1.0) > tamanho_sprite_cidade(1, 1.0));
    }
}
