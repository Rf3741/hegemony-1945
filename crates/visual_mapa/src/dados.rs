//! Dados de catálogo RON (cidades, conexões, indústrias, eras) e utilitários de leitura RON.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Dados estáticos de uma cidade carregados do catálogo RON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CidadeDados {
    pub id: String,
    pub nome: String,
    pub pais_tag: String,
    #[serde(default)]
    pub adm1_code: String,
    pub posicao: (f32, f32),
    pub populacao_1945: u32,
    pub populacao_1991: u32,
    pub nivel_1945: u8,
    pub nivel_1991: u8,
    pub eh_capital: bool,
}

/// Dados de uma conexão de transporte entre duas cidades.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConexaoDados {
    pub id: String,
    pub origem_id: String,
    pub destino_id: String,
    pub tipo: String,
    pub nivel_1945: u8,
    pub nivel_1991: u8,
    pub ano_construcao: u16,
}

/// Dados de um complexo industrial, mina, poço de petróleo, usina ou porto.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndustriaDados {
    pub id: String,
    pub nome: String,
    pub categoria: String,
    pub pais_tag: String,
    pub posicao: (f32, f32),
    pub ano_construcao: u16,
    pub nivel_inicial: u8,
    pub nivel_maximo: u8,
    pub taxa_fumaca: f32,
}

/// Configuração visual e de estilo de uma das 5 eras da Guerra Fria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EraVisual {
    pub ano_inicio: u16,
    pub nome: String,
    pub codinome: String,
    pub descricao: String,
    pub estilo_arquitetura: String,
    pub cor_primaria: (u8, u8, u8),
    pub cor_secundaria: (u8, u8, u8),
    pub cor_concreto: (u8, u8, u8),
    pub cor_fumaca: (u8, u8, u8),
    pub cor_janela_dia: (u8, u8, u8),
    pub cor_janela_noite: (u8, u8, u8),
    pub altura_predio_escala: f32,
    pub fator_arranha_ceus: f32,
}

/// Raiz dos dados visuais no disco: `$GF_DATA_DIR/visual` se definida, senão os assets do crate.
pub fn raiz_dados_padrao() -> PathBuf {
    if let Ok(d) = std::env::var("GF_DATA_DIR") {
        let p = PathBuf::from(d).join("visual");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/visual"))
}

/// Lê e desserializa um arquivo RON; `None` (com aviso) em caso de erro.
pub fn ler_ron<T: for<'de> Deserialize<'de>>(caminho: &Path) -> Option<T> {
    let texto = std::fs::read_to_string(caminho).ok()?;
    match ron::from_str::<T>(&texto) {
        Ok(v) => Some(v),
        Err(e) => {
            bevy::log::warn!("falha ao ler {}: {e}", caminho.display());
            None
        }
    }
}

/// Carrega as eras de `<raiz>/eras.ron`, com o conjunto embutido como reserva.
pub fn carregar_eras(raiz: &Path) -> Vec<EraVisual> {
    ler_ron::<Vec<EraVisual>>(&raiz.join("eras.ron"))
        .filter(|e| !e.is_empty())
        .unwrap_or_else(eras_padrao_reserva)
}

fn era(
    ano: u16, nome: &str, cod: &str, desc: &str, estilo: &str, c: [(u8, u8, u8); 6], h: f32, a: f32,
) -> EraVisual {
    EraVisual {
        ano_inicio: ano,
        nome: nome.into(),
        codinome: cod.into(),
        descricao: desc.into(),
        estilo_arquitetura: estilo.into(),
        cor_primaria: c[0],
        cor_secundaria: c[1],
        cor_concreto: c[2],
        cor_fumaca: c[3],
        cor_janela_dia: c[4],
        cor_janela_noite: c[5],
        altura_predio_escala: h,
        fator_arranha_ceus: a,
    }
}

/// Eras embutidas (reserva se `eras.ron` não existir).
pub fn eras_padrao_reserva() -> Vec<EraVisual> {
    vec![
        era(1945, "Reconstrução do Pós-Guerra", "era_1945", "Tijolo e chaminés de carvão.", "tijolo_chamine",
            [(139, 58, 43), (61, 67, 74), (120, 115, 105), (50, 48, 45), (45, 52, 58), (240, 185, 80)], 1.0, 0.05),
        era(1955, "Guerra Fria Clássica", "era_1955", "Modernismo industrial inicial.", "alvenaria_industrial",
            [(110, 118, 128), (85, 45, 40), (145, 142, 135), (70, 70, 75), (55, 65, 75), (245, 195, 95)], 1.25, 0.15),
        era(1965, "Era Espacial e Brutalismo", "era_1965", "Concreto armado maciço.", "concreto_armado",
            [(138, 146, 154), (74, 88, 104), (165, 168, 170), (95, 95, 100), (70, 85, 100), (230, 210, 120)], 1.6, 0.35),
        era(1975, "Crise do Petróleo e Modernismo", "era_1975", "Aço estrutural e torres.", "aco_estrutural",
            [(90, 122, 154), (175, 182, 188), (180, 182, 185), (120, 125, 130), (90, 130, 165), (210, 220, 140)], 2.0, 0.60),
        era(1985, "Era da Informação", "era_1985", "Vidro espelhado e arranha-céus.", "vidro_aco",
            [(108, 180, 238), (216, 226, 220), (205, 210, 215), (150, 155, 160), (120, 175, 220), (255, 235, 160)], 2.5, 0.85),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eras_do_arquivo() {
        let eras = carregar_eras(&raiz_dados_padrao());
        assert_eq!(eras.len(), 5);
        assert_eq!(eras[0].ano_inicio, 1945);
        assert_eq!(eras[4].ano_inicio, 1985);
    }

    #[test]
    fn eras_reserva() {
        assert_eq!(eras_padrao_reserva().len(), 5);
    }
}
