//! Índices diários por estado: níveis por tipo de prédio, produção local por bem e bônus de
//! cadeia local (processar onde se extrai). Uma única passada O(prédios) + O(estados x tipos
//! industriais x insumos); a produção e a IA só fazem consultas O(1).

use crate::catalogo::CatalogoEconomia;
use crate::entidades::{Estado, Provincia};
use crate::tipos::CategoriaPredio;

/// Bônus máximo de produtividade quando todos os insumos saem do próprio estado.
pub const BONUS_CADEIA_MAX: f64 = 0.20;
/// Peso da oferta dos demais estados do mesmo país (aproximação de "vizinhos": o modelo
/// econômico não tem malha de adjacência) sobre a parcela de insumo não coberta localmente.
pub const PESO_OUTROS_ESTADOS: f64 = 0.25;

/// Insumos (índice do bem, peso por valor) do método padrão de cada tipo industrial.
fn insumos_por_tipo(cat: &CatalogoEconomia) -> Vec<Vec<(usize, f64)>> {
    cat.tipos_predio
        .iter()
        .map(|t| {
            if t.categoria != CategoriaPredio::Industrial {
                return Vec::new();
            }
            let mut v: Vec<(usize, f64)> = Vec::new();
            for mid in &t.metodos_padrao {
                let Some(m) = cat.metodos_producao.iter().find(|m| &m.id == mid) else { continue };
                for (bid, qtd) in &m.insumos {
                    if let Some(g) = cat.bens.iter().position(|b| &b.id == bid) {
                        let w = qtd * cat.bens[g].preco_base.max(1.0);
                        match v.iter_mut().find(|(i, _)| *i == g) {
                            Some(e) => e.1 += w,
                            None => v.push((g, w)),
                        }
                    }
                }
            }
            v
        })
        .collect()
}

/// Recalcula os índices de todos os estados a partir do último tick de produção.
pub(crate) fn atualizar_indices_estados(cat: &CatalogoEconomia, estados: &mut [Estado], provincias: &[Provincia]) {
    let (nt, nb) = (cat.tipos_predio.len(), cat.bens.len());
    let n_paises = estados.iter().map(|e| e.pais_idx + 1).max().unwrap_or(0);
    for e in estados.iter_mut() {
        e.niveis_por_tipo.clear();
        e.niveis_por_tipo.resize(nt, 0);
        e.producao_local_bem.clear();
        e.producao_local_bem.resize(nb, 0.0);
        e.bonus_cadeia_tipo.clear();
        e.bonus_cadeia_tipo.resize(nt, 0.0);
    }
    // Demanda de insumos por estado (temporária) e totais por país.
    let mut dem = vec![0.0f32; estados.len() * nb];
    for pr in provincias {
        let e = pr.estado_idx;
        for b in &pr.predios {
            if b.nivel == 0 {
                continue;
            }
            estados[e].niveis_por_tipo[b.cache.tipo_idx] += b.nivel;
            let esc = b.cache.escala_efetiva_ultimo_tick;
            if esc > 0.0 {
                for &(g, q) in b.cache.produtos() {
                    estados[e].producao_local_bem[g as usize] += (esc * q) as f32;
                }
                for &(g, q) in b.cache.insumos() {
                    dem[e * nb + g as usize] += (esc * q) as f32;
                }
            }
        }
    }
    let mut prod_pais = vec![0.0f32; n_paises * nb];
    let mut dem_pais = vec![0.0f32; n_paises * nb];
    for (ei, e) in estados.iter().enumerate() {
        for g in 0..nb {
            prod_pais[e.pais_idx * nb + g] += e.producao_local_bem[g];
            dem_pais[e.pais_idx * nb + g] += dem[ei * nb + g];
        }
    }
    let insumos = insumos_por_tipo(cat);
    for (ei, e) in estados.iter_mut().enumerate() {
        let pi = e.pais_idx;
        for (ti, ins) in insumos.iter().enumerate() {
            if ins.is_empty() {
                continue;
            }
            let (mut soma, mut peso) = (0.0f64, 0.0f64);
            for &(g, w) in ins {
                let p_e = e.producao_local_bem[g] as f64;
                let d_e = dem[ei * nb + g] as f64;
                let cob_e = if p_e <= 1e-9 { 0.0 } else { (p_e / d_e.max(1e-9)).min(1.0) };
                let p_o = (prod_pais[pi * nb + g] as f64 - p_e).max(0.0);
                let d_o = (dem_pais[pi * nb + g] as f64 - d_e).max(0.0);
                let cob_o = if p_o <= 1e-9 { 0.0 } else { (p_o / d_o.max(1e-9)).min(1.0) };
                soma += w * (cob_e + (1.0 - cob_e) * PESO_OUTROS_ESTADOS * cob_o);
                peso += w;
            }
            e.bonus_cadeia_tipo[ti] = (BONUS_CADEIA_MAX * soma / peso.max(1e-9)) as f32;
        }
    }
}
