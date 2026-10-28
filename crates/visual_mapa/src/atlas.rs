//! Atlas de sprites (`atlas.png` + `atlas.ron`, de `arte_cidades`): 120 cidades
//! (8 níveis x 5 eras x 3 blocos), 62 indústrias e 18 efeitos. Sem atlas, o plugin cai no
//! visual procedural (retângulos).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct RectRon {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Debug, Deserialize)]
struct SpriteRon {
    categoria: String,
    bloco: String,
    #[serde(default)]
    nivel: u8,
    era: String,
    #[serde(default)]
    tipo_industria: Option<String>,
    rect: RectRon,
}

#[derive(Debug, Deserialize)]
struct MetaRon {
    textura: String,
    largura: u32,
    altura: u32,
}

#[derive(Debug, Deserialize)]
struct AtlasRon {
    meta: MetaRon,
    sprites: HashMap<String, SpriteRon>,
}

/// Índice (puro, sem Bevy) dos sprites do atlas: chave -> posição na lista de retângulos.
#[derive(Debug, Default, Clone)]
pub struct IndiceAtlas {
    pub retangulos: Vec<URect>,
    por_id: HashMap<String, usize>,
    cidades: HashMap<(String, u8, u8), usize>,
    industrias: HashMap<(String, u8), usize>,
    pub largura: u32,
    pub altura: u32,
    pub textura: String,
}

fn era_idx(s: &str) -> Option<u8> {
    let ano: i32 = s.strip_prefix("era_")?.parse().ok()?;
    Some(((ano - 1945) / 10).clamp(0, 4) as u8)
}

impl IndiceAtlas {
    /// Constrói o índice a partir do texto de `atlas.ron`.
    pub fn de_texto(texto: &str) -> Result<Self, String> {
        let ron: AtlasRon = ron::from_str(texto).map_err(|e| e.to_string())?;
        let mut chaves: Vec<&String> = ron.sprites.keys().collect();
        chaves.sort(); // ordem determinística
        let mut ind = IndiceAtlas {
            largura: ron.meta.largura,
            altura: ron.meta.altura,
            textura: ron.meta.textura.clone(),
            ..default()
        };
        for k in chaves {
            let s = &ron.sprites[k];
            let i = ind.retangulos.len();
            ind.retangulos.push(URect::new(s.rect.x, s.rect.y, s.rect.x + s.rect.w, s.rect.y + s.rect.h));
            ind.por_id.insert(k.clone(), i);
            let Some(e) = era_idx(&s.era) else { continue };
            match s.categoria.as_str() {
                "cidades" => {
                    ind.cidades.insert((s.bloco.clone(), s.nivel, e), i);
                }
                "industrias" => {
                    if let Some(t) = &s.tipo_industria {
                        ind.industrias.insert((t.clone(), e), i);
                    }
                }
                _ => {}
            }
        }
        Ok(ind)
    }

    /// Retorna o índice de textura correspondente ao id exato do sprite no atlas (ex.: "industria_siderurgica_era_1945").
    pub fn indice_por_id(&self, id: &str) -> Option<usize> {
        self.por_id.get(id).copied()
    }

    /// Sprite da cidade (nível 1..=8, era 0..=4). Retorna `None` se o bloco não existir.
    pub fn indice_cidade(&self, bloco: &str, nivel: u8, era: usize) -> Option<usize> {
        self.cidades.get(&(bloco.to_string(), nivel.clamp(1, 8), era.min(4) as u8)).copied()
    }

    /// Sprite da indústria; se faltar a era (ex.: usina nuclear só existe a partir de uma era),
    /// usa a era mais próxima disponível.
    pub fn indice_industria(&self, tipo: &str, era: usize) -> Option<usize> {
        let tipo = tipo_atlas(tipo);
        let era = era.min(4) as i32;
        (0..5).map(|d| (era - d).max(0)).chain((0..5).map(|d| (era + d).min(4)))
            .find_map(|e| self.industrias.get(&(tipo.to_string(), e as u8)).copied())
    }

    pub fn total_cidades(&self) -> usize {
        self.cidades.len()
    }

    pub fn total_industrias(&self) -> usize {
        self.industrias.len()
    }
}

/// Traduz as categorias genéricas do jogo para o `tipo_industria` do atlas.
pub fn tipo_atlas(tipo: &str) -> &str {
    match tipo {
        "fabrica" => "fabrica_generica",
        "usina" => "usina_carvao",
        outro => outro,
    }
}

/// Recurso com o atlas carregado (ou ausente, para o fallback procedural).
#[derive(Resource, Default)]
pub struct AtlasVisual {
    pub carregado: Option<AtlasCarregado>,
}

pub struct AtlasCarregado {
    pub imagem: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub indice: IndiceAtlas,
}

impl AtlasVisual {
    pub fn disponivel(&self) -> bool {
        self.carregado.is_some()
    }

    /// Cria um `Sprite` do atlas a partir de um índice numérico.
    pub fn sprite(&self, indice: usize, tamanho: f32) -> Option<Sprite> {
        let a = self.carregado.as_ref()?;
        Some(Sprite {
            image: a.imagem.clone(),
            texture_atlas: Some(TextureAtlas { layout: a.layout.clone(), index: indice }),
            custom_size: Some(Vec2::splat(tamanho)),
            ..default()
        })
    }

    /// Cria um `Sprite` do atlas a partir do id exato do sprite (ex.: "industria_siderurgica_era_1945").
    pub fn sprite_por_id(&self, id: &str, tamanho: f32) -> Option<Sprite> {
        let a = self.carregado.as_ref()?;
        let idx = a.indice.indice_por_id(id)?;
        self.sprite(idx, tamanho)
    }
}

/// Carrega `atlas.ron` e `atlas.png` de `raiz` (lendo o PNG direto do disco).
pub fn carregar_atlas(
    raiz: &Path,
    imagens: &mut Assets<Image>,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> Option<AtlasCarregado> {
    let texto = std::fs::read_to_string(raiz.join("atlas.ron")).ok()?;
    let indice = match IndiceAtlas::de_texto(&texto) {
        Ok(i) => i,
        Err(e) => {
            warn!("atlas.ron invalido: {e}");
            return None;
        }
    };
    let png = image::open(raiz.join(&indice.textura)).ok()?.to_rgba8();
    if png.width() != indice.largura || png.height() != indice.altura {
        warn!("atlas.png com tamanho diferente do atlas.ron");
        return None;
    }
    let imagem = Image::new(
        Extent3d { width: png.width(), height: png.height(), depth_or_array_layers: 1 },
        TextureDimension::D2,
        png.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(indice.largura, indice.altura));
    for r in &indice.retangulos {
        layout.add_texture(*r);
    }
    Some(AtlasCarregado { imagem: imagens.add(imagem), layout: layouts.add(layout), indice })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indice() -> IndiceAtlas {
        let t = std::fs::read_to_string(crate::dados::raiz_dados_padrao().join("atlas.ron")).unwrap();
        IndiceAtlas::de_texto(&t).unwrap()
    }

    #[test]
    fn atlas_tem_120_cidades_e_industrias() {
        let a = indice();
        assert_eq!(a.total_cidades(), 120);
        assert_eq!(a.total_industrias(), 62);
        assert_eq!(a.retangulos.len(), 200);
    }

    #[test]
    fn todas_combinacoes_de_cidade_existem() {
        let a = indice();
        for b in ["ocidental", "sovietica", "terceiro_mundo"] {
            for n in 1..=8 {
                for e in 0..5 {
                    assert!(a.indice_cidade(b, n, e).is_some(), "{b} {n} {e}");
                }
            }
        }
    }

    #[test]
    fn industria_com_fallback_de_era() {
        let a = indice();
        assert!(a.indice_industria("fabrica", 0).is_some());
        assert!(a.indice_industria("usina_nuclear", 0).is_some(), "deve cair na era mais proxima");
        assert!(a.indice_industria("porto", 4).is_some());
        assert!(a.indice_industria("inexistente", 2).is_none());
    }

    #[test]
    fn retangulos_dentro_da_textura() {
        let a = indice();
        assert!(a.retangulos.iter().all(|r| r.max.x <= a.largura && r.max.y <= a.altura));
    }

    #[test]
    fn indice_por_id_encontra_sprites() {
        let a = indice();
        assert!(a.indice_por_id("industria_siderurgica_era_1945").is_some());
        assert!(a.indice_por_id("cidade_ocidental_nivel_1_aldeia_era_1945").is_some());
        assert!(a.indice_por_id("efeito_fumaca_densa").is_some());
        assert!(a.indice_por_id("inexistente").is_none());
    }
}
