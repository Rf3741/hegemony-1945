//! Carregador dos dados reais do cenário de 1945 (`cenario_1945_economia` e `territorio`).
//!
//! Converte os dados da simulação econômica real em entradas visuais para o mapa vivo:
//! - 40.884 construções econômicas de 4.594 províncias com níveis reais de 1945.
//! - 1.517 cidades com população urbana e classificação de nível 1 a 8.
//! - Mapeamento de coordenadas via centróides de `provinces.ron` do mapa Miller (5632x2048).

use crate::entrada::{
    BlocoVisual, CidadeDemograficaEntrada, EntradaEconomiaVisual, PredioEconomiaEntrada,
};
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Caminho padrão para os dados do cenário econômico de 1945 no staging.
pub const RAIZ_CENARIO_STAGING: &str = "/home/rafael/claude/guerra-fria-staging/cenario_1945_economia";

/// Caminho padrão para os dados de mapa no repositório.
pub const MAPA_PROVINCES_PADRAO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/map/provinces.ron");

/// Caminho padrão para as cidades do território no repositório.
pub const CIDADES_TERRITORIO_PADRAO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data/territorio/cidades.ron");

#[derive(Debug, Deserialize)]
struct ProvinciaMapaRon {
    adm1_code: String,
    centroide: (f32, f32),
}

#[derive(Debug, Deserialize)]
struct MapaRon {
    provincias: Vec<ProvinciaMapaRon>,
}

#[derive(Debug, Deserialize)]
struct CidadeTerritorioRon {
    id: String,
    nome: String,
    populacao_1945: u64,
    latitude: f32,
    longitude: f32,
    estado_id: String,
    pais_tag: String,
    #[serde(default)]
    eh_capital_estado: bool,
}

/// Carrega as coordenadas (longitude, latitude) dos centroides das províncias a partir de `provinces.ron`.
pub fn carregar_centroides_provincias(caminho: &Path) -> HashMap<String, (f32, f32)> {
    let mut mapa = HashMap::new();
    if let Ok(texto) = std::fs::read_to_string(caminho) {
        if let Ok(ron_data) = ron::from_str::<MapaRon>(&texto) {
            for p in ron_data.provincias {
                mapa.insert(p.adm1_code, p.centroide);
            }
        }
    }
    mapa
}

/// Carrega as cidades mundiais com população de 1945 a partir de `cidades.ron`.
pub fn carregar_cidades_cenario(caminho: &Path) -> Vec<CidadeDemograficaEntrada> {
    let mut cidades = Vec::new();
    let conteudo = std::fs::read_to_string(caminho)
        .or_else(|_| std::fs::read_to_string("/home/rafael/claude/guerra-fria-staging/territorio_dados/cidades.ron"))
        .or_else(|_| std::fs::read_to_string("/home/rafael/claude/guerra-fria/assets/data/territorio/cidades.ron"));
    if let Ok(texto) = conteudo {
        if let Ok(mapa) = ron::from_str::<HashMap<String, CidadeTerritorioRon>>(&texto) {
            for (_, c) in mapa {
                cidades.push(CidadeDemograficaEntrada {
                    id: c.id,
                    nome: c.nome,
                    estado_id: c.estado_id,
                    pais_tag: c.pais_tag.clone(),
                    populacao_urbana: c.populacao_1945,
                    eh_capital: c.eh_capital_estado,
                    posicao: (c.longitude, c.latitude),
                    bloco: Some(BlocoVisual::de_pais(&c.pais_tag)),
                });
            }
        }
    }
    // Ordenar determinísticamente por ID
    cidades.sort_by(|a, b| a.id.cmp(&b.id));
    cidades
}

/// Lê uma string até a próxima aspa dupla, retornando o conteúdo e o resto.
fn extrair_entre_aspas(s: &str) -> Option<(&str, &str)> {
    let start = s.find('"')? + 1;
    let end = s[start..].find('"')? + start;
    Some((&s[start..end], &s[end + 1..]))
}

/// Extrai o valor de um campo com chave `chave:` (ex: `estado_id:"afg_cabul_central"`).
fn extrair_campo_string<'a>(linha: &'a str, chave: &str) -> Option<&'a str> {
    let idx = linha.find(chave)?;
    let resto = &linha[idx + chave.len()..];
    let (val, _) = extrair_entre_aspas(resto)?;
    Some(val)
}

/// Carrega o cenário completo de 1945 gerando um [`EntradaEconomiaVisual`].
pub fn carregar_cenario_1945(
    caminho_provincias_eco: &Path,
    caminho_provinces_mapa: &Path,
    caminho_cidades: &Path,
) -> EntradaEconomiaVisual {
    info!("visual_mapa: carregando cenário real de 1945...");
    let coords = carregar_centroides_provincias(caminho_provinces_mapa);
    let cidades = carregar_cidades_cenario(caminho_cidades);

    let mut predios = Vec::with_capacity(45_000);

    if let Ok(texto) = std::fs::read_to_string(caminho_provincias_eco) {
        for linha in texto.lines() {
            if !linha.starts_with("    (id:") {
                continue;
            }

            let prov_id = extrair_campo_string(linha, "id:").unwrap_or("");
            let estado_id = extrair_campo_string(linha, "estado_id:").unwrap_or("");
            let pais_id = extrair_campo_string(linha, "pais_id:").unwrap_or("");
            let codigo_externo = extrair_campo_string(linha, "codigo_externo:Some(");

            let pos = codigo_externo.and_then(|adm1| coords.get(adm1).copied());
            let pais_tag = pais_id.to_uppercase();
            let bloco = Some(BlocoVisual::de_pais(&pais_tag));

            // Extrair fatia de prédios: `predios:[(tipo_predio_id:"...",nivel:...`
            if let Some(idx_predios) = linha.find("predios:[") {
                let resto_predios = &linha[idx_predios + 9..];
                if let Some(fim_predios) = resto_predios.rfind("])") {
                    let slice = &resto_predios[..fim_predios];
                    
                    // Iterar sobre cada prédio na lista
                    let mut cursor = slice;
                    let mut p_idx = 0;
                    while let Some(pos_tid) = cursor.find("tipo_predio_id:") {
                        cursor = &cursor[pos_tid + 15..];
                        let Some((tipo_id, depois_tipo)) = extrair_entre_aspas(cursor) else { break };
                        cursor = depois_tipo;

                        let mut nivel = 1u32;
                        if let Some(pos_niv) = cursor.find("nivel:") {
                            let depois_niv = &cursor[pos_niv + 6..];
                            let fim_num = depois_niv
                                .find(|c: char| !c.is_ascii_digit())
                                .unwrap_or(depois_niv.len());
                            if let Ok(n) = depois_niv[..fim_num].parse::<u32>() {
                                nivel = n;
                            }
                            cursor = &depois_niv[fim_num..];
                        }

                        let p_id = format!("{}_{}_{}", prov_id, tipo_id, p_idx);
                        p_idx += 1;

                        predios.push(PredioEconomiaEntrada {
                            id: p_id,
                            tipo: tipo_id.to_string(),
                            estado_id: estado_id.to_string(),
                            provincia_id: if prov_id.is_empty() { None } else { Some(prov_id.to_string()) },
                            pais_tag: pais_tag.clone(),
                            nivel,
                            metodos_ativos: Vec::new(),
                            producao: 1.0,
                            posicao: pos,
                            bloco,
                            era: Some(0), // Era inicial de 1945
                            ativa: true,
                        });
                    }
                }
            }
        }
    } else {
        warn!("visual_mapa: arquivo de províncias econômicas não encontrado em {}", caminho_provincias_eco.display());
    }

    info!(
        "visual_mapa: cenário carregado com {} prédios e {} cidades.",
        predios.len(),
        cidades.len()
    );

    EntradaEconomiaVisual { predios, cidades }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entrada::populacao_urbana_para_nivel;

    #[test]
    fn carregar_centroides_provincias_teste() {
        let coords = carregar_centroides_provincias(Path::new(MAPA_PROVINCES_PADRAO));
        assert!(coords.len() >= 4500, "Deve conter as ~4596 províncias do mapa");
        assert!(coords.contains_key("ARG-1309"));
    }

    #[test]
    fn carregar_cidades_cenario_teste() {
        let cidades = carregar_cidades_cenario(Path::new(CIDADES_TERRITORIO_PADRAO));
        assert!(cidades.len() >= 1500, "Deve carregar mais de 1500 cidades");
        let sp = cidades.iter().find(|c| c.id == "sao_paulo").unwrap();
        assert_eq!(sp.pais_tag, "BRA");
        assert!(sp.populacao_urbana > 1_000_000);
        assert_eq!(populacao_urbana_para_nivel(sp.populacao_urbana), 6);
    }

    #[test]
    fn carregar_cenario_1945_completo() {
        let caminho_eco = Path::new(RAIZ_CENARIO_STAGING).join("provincias.ron");
        if caminho_eco.exists() {
            let eco = carregar_cenario_1945(
                &caminho_eco,
                Path::new(MAPA_PROVINCES_PADRAO),
                Path::new(CIDADES_TERRITORIO_PADRAO),
            );
            assert!(eco.total_predios() >= 40_000, "Deve conter os ~40.884 prédios iniciais");
            assert!(eco.total_cidades() >= 1_500, "Deve conter as 1.517 cidades");
        }
    }
}
