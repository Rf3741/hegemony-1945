//! Cenário político completo contendo catálogo estático, países e populações.

use crate::catalogo::CatalogoPolitica;
use crate::entidades::pais::PaisPolitica;
use crate::erro::{ErroPolitica, ResultadoPolitica};
use crate::motor::MundoPolitico;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Cenário configurado para inicialização de uma simulação política.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CenarioPolitica {
    pub catalogo: CatalogoPolitica,
    pub paises: Vec<PaisPolitica>,
    #[serde(default = "padrao_semente")]
    pub semente_padrao: u64,
}

const fn padrao_semente() -> u64 {
    19_450_101
}

impl CenarioPolitica {
    /// Cria um novo cenário político em memória.
    pub fn novo(catalogo: CatalogoPolitica, paises: Vec<PaisPolitica>, semente: u64) -> Self {
        Self {
            catalogo,
            paises,
            semente_padrao: semente,
        }
    }

    /// Constrói uma instância ativa e validada de `MundoPolitico`.
    pub fn construir_mundo(self) -> ResultadoPolitica<MundoPolitico> {
        MundoPolitico::novo(self.catalogo, self.paises, self.semente_padrao)
    }

    /// Carrega o catálogo e a lista de países a partir de um diretório de arquivos RON.
    pub fn carregar_diretorio<P: AsRef<Path>>(dir: P) -> ResultadoPolitica<Self> {
        let dir_path = dir.as_ref();

        // 1. Catálogo
        let cat_path = dir_path.join("catalogo_politica.ron");
        let catalogo = if cat_path.exists() {
            CatalogoPolitica::de_arquivo_ron(&cat_path)?
        } else {
            // Tenta carregar arquivos separados se existirem
            let mut cat = CatalogoPolitica::default();
            let ideol_path = dir_path.join("ideologias.ron");
            if ideol_path.exists() {
                let conteudo = fs::read_to_string(&ideol_path).map_err(|e| ErroPolitica::Io {
                    caminho: ideol_path.clone(),
                    origem: e,
                })?;
                let lista: Vec<crate::catalogo::Ideologia> =
                    ron::from_str(&conteudo).map_err(|e| ErroPolitica::DesserializacaoRon {
                        contexto: "ideologias.ron".into(),
                        origem: e,
                    })?;
                for ideol in lista {
                    cat.ideologias.insert(ideol.id.clone(), ideol);
                }
            }
            let leis_path = dir_path.join("leis.ron");
            if leis_path.exists() {
                let conteudo = fs::read_to_string(&leis_path).map_err(|e| ErroPolitica::Io {
                    caminho: leis_path.clone(),
                    origem: e,
                })?;
                let lista: Vec<crate::catalogo::CategoriaLeis> =
                    ron::from_str(&conteudo).map_err(|e| ErroPolitica::DesserializacaoRon {
                        contexto: "leis.ron".into(),
                        origem: e,
                    })?;
                for cat_leis in lista {
                    for lei in &cat_leis.leis {
                        cat.leis.insert(lei.id.clone(), lei.clone());
                    }
                    cat.categorias_leis
                        .insert(cat_leis.categoria.clone(), cat_leis);
                }
            }
            let gis_path = dir_path.join("grupos_interesse.ron");
            if gis_path.exists() {
                let conteudo = fs::read_to_string(&gis_path).map_err(|e| ErroPolitica::Io {
                    caminho: gis_path.clone(),
                    origem: e,
                })?;
                let lista: Vec<crate::catalogo::DefinicaoGrupoInteresse> = ron::from_str(&conteudo)
                    .map_err(|e| ErroPolitica::DesserializacaoRon {
                        contexto: "grupos_interesse.ron".into(),
                        origem: e,
                    })?;
                for gi in lista {
                    cat.grupos_interesse.insert(gi.id.clone(), gi);
                }
            }
            cat
        };

        // 2. Países
        let paises_path = dir_path.join("paises_politica.ron");
        let paises = if paises_path.exists() {
            let conteudo = fs::read_to_string(&paises_path).map_err(|e| ErroPolitica::Io {
                caminho: paises_path.clone(),
                origem: e,
            })?;
            let lista: Vec<PaisPolitica> =
                ron::from_str(&conteudo).map_err(|e| ErroPolitica::DesserializacaoRon {
                    contexto: "paises_politica.ron".into(),
                    origem: e,
                })?;
            lista
        } else {
            Vec::new()
        };

        Ok(Self {
            catalogo,
            paises,
            semente_padrao: 19_450_101,
        })
    }
}
