//! Mapeamento de tipos de prédio da economia (`tipos_predio.ron`) para sprites do atlas (`atlas.ron`).
//!
//! Cada um dos 56 tipos de construção da simulação econômica é mapeado para um sprite do atlas
//! de `arte_cidades`, considerando o bloco arquitetônico (Ocidental, Soviético, Terceiro Mundo)
//! e a era tecnológica (1945 a 1985). Se o atlas não estiver disponível, o catálogo fornece
//! a cor e o estilo para renderização procedural.

use crate::entrada::BlocoVisual;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Registro de mapeamento de um tipo de prédio individual.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapeamentoPredioRegistro {
    pub id: String,
    pub nome: String,
    pub categoria: String,
    pub emite_fumaca: bool,
    pub tipo_fumaca: Option<String>,
    pub cor_procedural: (u8, u8, u8),
    pub sprites: HashMap<String, HashMap<u16, String>>,
}

#[derive(Debug, Deserialize)]
struct CatalogoRon {
    predios: Vec<MapeamentoPredioRegistro>,
}

/// Catálogo global de mapeamento de todos os prédios da economia para o mapa vivo.
#[derive(Resource, Debug, Clone, Default)]
pub struct CatalogoMapeamentoPredios {
    pub predios: HashMap<String, MapeamentoPredioRegistro>,
}

impl CatalogoMapeamentoPredios {
    /// Constrói o catálogo a partir de uma lista de registros.
    pub fn novo(registros: Vec<MapeamentoPredioRegistro>) -> Self {
        let mut map = HashMap::with_capacity(registros.len());
        for r in registros {
            map.insert(r.id.clone(), r);
        }
        Self { predios: map }
    }

    /// Carrega o catálogo a partir de uma string no formato RON.
    pub fn de_texto(texto: &str) -> Result<Self, String> {
        let ron: CatalogoRon = ron::from_str(texto).map_err(|e| e.to_string())?;
        Ok(Self::novo(ron.predios))
    }

    /// Retorna o registro correspondente ao tipo de prédio da economia.
    pub fn buscar(&self, tipo: &str) -> Option<&MapeamentoPredioRegistro> {
        self.predios.get(tipo)
    }

    /// Resolve o ID exato do sprite no atlas a partir do tipo de prédio, bloco geopolítico e ano contínuo.
    pub fn resolver_sprite(&self, tipo: &str, bloco: BlocoVisual, ano: f32) -> Option<&str> {
        let r = self.buscar(tipo)?;
        let chave_bloco = bloco.chave_atlas();
        let ano_era = match ano as i32 {
            ..=1949 => 1945,
            1950..=1959 => 1955,
            1960..=1969 => 1965,
            1970..=1979 => 1975,
            _ => 1985,
        };

        // 1. Tenta o bloco solicitado
        if let Some(por_ano) = r.sprites.get(chave_bloco) {
            if let Some(s) = por_ano.get(&ano_era) {
                return Some(s.as_str());
            }
            if let Some(s) = por_ano.get(&1945) {
                return Some(s.as_str());
            }
        }

        // 2. Fallback para ocidental
        if let Some(por_ano) = r.sprites.get("ocidental") {
            if let Some(s) = por_ano.get(&ano_era) {
                return Some(s.as_str());
            }
            if let Some(s) = por_ano.get(&1945) {
                return Some(s.as_str());
            }
        }

        // 3. Fallback para qualquer sprite disponível
        r.sprites.values().next().and_then(|m| m.values().next()).map(|s| s.as_str())
    }

    /// Retorna se este tipo de construção emite fumaça industrial.
    pub fn emite_fumaca(&self, tipo: &str) -> bool {
        self.buscar(tipo).map(|r| r.emite_fumaca).unwrap_or(false)
    }

    /// Retorna o identificador do efeito de fumaça associado (ex: "efeito_fumaca_densa").
    pub fn tipo_fumaca(&self, tipo: &str) -> Option<&str> {
        self.buscar(tipo).and_then(|r| r.tipo_fumaca.as_deref())
    }

    /// Retorna a cor para o fallback procedural no Bevy.
    pub fn cor_procedural(&self, tipo: &str) -> Color {
        if let Some(r) = self.buscar(tipo) {
            Color::srgb_u8(r.cor_procedural.0, r.cor_procedural.1, r.cor_procedural.2)
        } else {
            Color::srgb(0.6, 0.6, 0.6)
        }
    }

    /// Total de tipos de prédio mapeados.
    pub fn total(&self) -> usize {
        self.predios.len()
    }
}

/// Carrega o catálogo de `mapeamento_predios.ron` na raiz dos dados especificada.
pub fn carregar_mapeamento_predios(raiz: &Path) -> CatalogoMapeamentoPredios {
    let caminho = raiz.join("mapeamento_predios.ron");
    if let Ok(texto) = std::fs::read_to_string(&caminho) {
        match CatalogoMapeamentoPredios::de_texto(&texto) {
            Ok(cat) => {
                info!("visual_mapa: carregados {} tipos de prédio de {}", cat.total(), caminho.display());
                return cat;
            }
            Err(e) => {
                warn!("visual_mapa: erro ao ler {}: {}", caminho.display(), e);
            }
        }
    }
    warn!("visual_mapa: mapeamento_predios.ron não encontrado em {}; usando catálogo vazio.", raiz.display());
    CatalogoMapeamentoPredios::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carregar_catalogo_padrao() {
        let raiz = crate::dados::raiz_dados_padrao();
        let cat = carregar_mapeamento_predios(&raiz);
        assert!(cat.total() >= 56, "Todos os tipos de prédio devem estar no catálogo (encontrados: {})", cat.total());

        // Testar resolução de sprites conhecidos
        let s_fazenda = cat.resolver_sprite("fazenda_graos", BlocoVisual::Ocidental, 1945.0);
        assert_eq!(s_fazenda, Some("industria_fazenda_mecanizada_era_1945"));

        let s_siderurgica = cat.resolver_sprite("siderurgica", BlocoVisual::Sovietica, 1975.0);
        assert_eq!(s_siderurgica, Some("industria_siderurgica_era_1975"));

        let s_admin = cat.resolver_sprite("administracao_governamental", BlocoVisual::Sovietica, 1945.0);
        assert_eq!(s_admin, Some("cidade_sovietica_nivel_4_cidade_media_era_1945"));

        assert!(cat.emite_fumaca("siderurgica"));
        assert!(!cat.emite_fumaca("fazenda_graos"));
        assert_eq!(cat.tipo_fumaca("usina_termeletrica"), Some("efeito_fumaca_chamine"));
    }
}
