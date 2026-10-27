//! Grafo de adjacência regional com representação compacta CSR (Compressed Sparse Row).
//!
//! Permite travessias ultra-rápidas em memória contígua (L1/L2 cache), busca em largura (BFS),
//! detecção de vizinhos e cálculo de distâncias logísticas sem alocações no heap por consulta.

use crate::erro::{ErroGuerra, ResultadoGuerra};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aresta de adjacência entre duas regiões admin-1 expressa em IDs textuais.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArestaRegiao {
    pub de: String,
    pub para: String,
    #[serde(default = "padrao_distancia_km")]
    pub distancia_km: f64,
    #[serde(default)]
    pub travessia_rio: bool,
    #[serde(default)]
    pub estreito_maritimo: bool,
}

const fn padrao_distancia_km() -> f64 {
    100.0
}

/// Registro compacto de um vizinho compilado na representação CSR.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VizinhoCompilado {
    pub destino_idx: u32,
    pub distancia_km: f32,
    pub travessia_rio: bool,
    pub estreito_maritimo: bool,
}

/// Grafo de regiões compilado em formato Compressed Sparse Row (CSR).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GrafoCompilado {
    pub offsets: Vec<u32>,
    pub vizinhos: Vec<VizinhoCompilado>,
}

impl GrafoCompilado {
    /// Constrói o grafo CSR bidirecional a partir da lista de arestas e mapa de índices.
    pub fn construir(
        num_regioes: usize,
        arestas: &[ArestaRegiao],
        mapa_indices: &HashMap<String, usize>,
    ) -> ResultadoGuerra<Self> {
        let mut adj: Vec<Vec<VizinhoCompilado>> = vec![Vec::new(); num_regioes];

        for (i, aresta) in arestas.iter().enumerate() {
            let u =
                *mapa_indices
                    .get(&aresta.de)
                    .ok_or_else(|| ErroGuerra::ReferenciaInexistente {
                        origem_entidade: "aresta",
                        origem_id: format!("aresta_{i}"),
                        alvo_entidade: "regiao",
                        alvo_id: aresta.de.clone(),
                    })?;

            let v = *mapa_indices.get(&aresta.para).ok_or_else(|| {
                ErroGuerra::ReferenciaInexistente {
                    origem_entidade: "aresta",
                    origem_id: format!("aresta_{i}"),
                    alvo_entidade: "regiao",
                    alvo_id: aresta.para.clone(),
                }
            })?;

            if u == v {
                continue;
            }

            let dist = aresta.distancia_km.max(1.0) as f32;

            // Insere u -> v se já não existir
            if !adj[u].iter().any(|viz| viz.destino_idx as usize == v) {
                adj[u].push(VizinhoCompilado {
                    destino_idx: v as u32,
                    distancia_km: dist,
                    travessia_rio: aresta.travessia_rio,
                    estreito_maritimo: aresta.estreito_maritimo,
                });
            }

            // Insere v -> u (grafo não-direcionado de fronteiras terrestres)
            if !adj[v].iter().any(|viz| viz.destino_idx as usize == u) {
                adj[v].push(VizinhoCompilado {
                    destino_idx: u as u32,
                    distancia_km: dist,
                    travessia_rio: aresta.travessia_rio,
                    estreito_maritimo: aresta.estreito_maritimo,
                });
            }
        }

        // Ordena vizinhos pelo índice de destino para acessos consistentes e determinísticos
        for lista in &mut adj {
            lista.sort_unstable_by_key(|viz| viz.destino_idx);
        }

        let mut offsets = Vec::with_capacity(num_regioes + 1);
        let mut total_vizinhos = 0usize;
        offsets.push(0);

        for lista in &adj {
            total_vizinhos += lista.len();
            offsets.push(total_vizinhos as u32);
        }

        let mut vizinhos = Vec::with_capacity(total_vizinhos);
        for lista in adj {
            vizinhos.extend(lista);
        }

        Ok(Self { offsets, vizinhos })
    }

    /// Retorna a fatia de vizinhos adjacentes a uma determinada região.
    #[inline]
    pub fn vizinhos(&self, regiao_idx: usize) -> &[VizinhoCompilado] {
        if regiao_idx + 1 >= self.offsets.len() {
            return &[];
        }
        let inicio = self.offsets[regiao_idx] as usize;
        let fim = self.offsets[regiao_idx + 1] as usize;
        &self.vizinhos[inicio..fim]
    }

    /// Grau de adjacência (número de vizinhos) de uma região.
    #[inline]
    pub fn grau(&self, regiao_idx: usize) -> usize {
        if regiao_idx + 1 >= self.offsets.len() {
            0
        } else {
            (self.offsets[regiao_idx + 1] - self.offsets[regiao_idx]) as usize
        }
    }

    /// Verifica se duas regiões são imediatamente adjacentes no grafo.
    #[inline]
    pub fn sao_adjacentes(&self, u: usize, v: usize) -> bool {
        self.vizinhos(u)
            .iter()
            .any(|viz| viz.destino_idx as usize == v)
    }

    /// Obtém as propriedades da aresta entre duas regiões adjacentes, se existir.
    #[inline]
    pub fn obter_aresta(&self, u: usize, v: usize) -> Option<VizinhoCompilado> {
        self.vizinhos(u)
            .iter()
            .find(|viz| viz.destino_idx as usize == v)
            .copied()
    }
}
