//! Empacotamento, leitura e escrita RON de um cenário econômico completo (`CenarioEconomia`).

use crate::catalogo::{CatalogoEconomia, ler_arquivo_ron, ler_str_ron};
use crate::entidades::{BlocoComercial, ConfigSimulacao, Estado, Mercado, Pais, Provincia, RotaComercial};
use crate::erro::{ErroEconomia, ResultadoEconomia};
use crate::tipos::DataJogo;
use ron::ser::{PrettyConfig, to_string_pretty};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Pacote completo (catálogo + estado inicial do mapa econômico) pronto para instanciar `MundoEconomico`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CenarioEconomia {
    #[serde(default)]
    pub data_inicial: DataJogo,
    #[serde(default)]
    pub config: ConfigSimulacao,
    pub catalogo: CatalogoEconomia,
    pub mercados: Vec<Mercado>,
    pub paises: Vec<Pais>,
    pub estados: Vec<Estado>,
    pub provincias: Vec<Provincia>,
    #[serde(default)]
    pub rotas_comerciais: Vec<RotaComercial>,
    #[serde(default)]
    pub blocos_comerciais: Vec<BlocoComercial>,
}

fn escrever_arquivo_ron<T: Serialize>(caminho: &Path, valor: &T) -> ResultadoEconomia<()> {
    let pretty = PrettyConfig::new()
        .depth_limit(6)
        .separate_tuple_members(true)
        .enumerate_arrays(false);
    let texto =
        to_string_pretty(valor, pretty).map_err(|origem| ErroEconomia::SerializacaoRon {
            contexto: caminho.display().to_string(),
            origem: Box::new(origem),
        })?;
    fs::write(caminho, texto).map_err(|origem| ErroEconomia::Io {
        caminho: caminho.to_path_buf(),
        origem,
    })
}

impl CenarioEconomia {
    /// Carrega um cenário modular completo a partir de uma pasta contendo os arquivos `.ron`:
    /// - Obrigatórios: `bens.ron`, `tipos_pop.ron`, `leis_economicas.ron`, `metodos_producao.ron`,
    ///   `tipos_predio.ron`, `mercados.ron`, `paises.ron`, `estados.ron`, `provincias.ron`
    /// - Opcionais: `rotas_comerciais.ron`, `config_simulacao.ron`
    pub fn carregar_diretorio(dir: &Path) -> ResultadoEconomia<Self> {
        let catalogo = CatalogoEconomia::carregar_diretorio(dir)?;
        let mercados: Vec<Mercado> = ler_arquivo_ron(&dir.join("mercados.ron"))?;
        let paises: Vec<Pais> = ler_arquivo_ron(&dir.join("paises.ron"))?;
        let mut estados: Vec<Estado> = ler_arquivo_ron(&dir.join("estados.ron"))?;
        let caminho_potenciais = dir.join("potenciais.ron");
        if caminho_potenciais.exists() {
            let potenciais: Vec<crate::entidades::PotencialRegional> =
                ler_arquivo_ron(&caminho_potenciais)?;
            let mut pot_map: std::collections::HashMap<String, crate::entidades::PotencialRegional> =
                potenciais.into_iter().map(|p| (p.estado_id.clone(), p)).collect();
            for est in &mut estados {
                if let Some(pot) = pot_map.remove(&est.id) {
                    est.terra_aravel = pot.terra_aravel;
                    est.culturas_permitidas = pot.culturas_permitidas;
                    est.limites_recursos = pot.recursos;
                    est.qualidade_recursos = pot.qualidade_recursos;
                }
            }
        }
        let provincias: Vec<Provincia> = ler_arquivo_ron(&dir.join("provincias.ron"))?;

        let caminho_rotas = dir.join("rotas_comerciais.ron");
        let rotas_comerciais: Vec<RotaComercial> = if caminho_rotas.exists() {
            ler_arquivo_ron(&caminho_rotas)?
        } else {
            Vec::new()
        };

        let caminho_blocos = dir.join("blocos_comerciais.ron");
        let blocos_comerciais: Vec<BlocoComercial> = if caminho_blocos.exists() {
            ler_arquivo_ron(&caminho_blocos)?
        } else {
            Vec::new()
        };

        let caminho_config = dir.join("config_simulacao.ron");
        let config: ConfigSimulacao = if caminho_config.exists() {
            ler_arquivo_ron(&caminho_config)?
        } else {
            ConfigSimulacao::default()
        };

        Ok(Self {
            data_inicial: DataJogo::default(),
            config,
            catalogo,
            mercados,
            paises,
            estados,
            provincias,
            rotas_comerciais,
            blocos_comerciais,
        })
    }

    /// Salva o cenário modular em arquivos `.ron` formatados dentro de `dir`.
    pub fn salvar_diretorio(&self, dir: &Path) -> ResultadoEconomia<()> {
        fs::create_dir_all(dir).map_err(|origem| ErroEconomia::Io {
            caminho: dir.to_path_buf(),
            origem,
        })?;
        escrever_arquivo_ron(&dir.join("bens.ron"), &self.catalogo.bens)?;
        escrever_arquivo_ron(&dir.join("tipos_pop.ron"), &self.catalogo.tipos_pop)?;
        escrever_arquivo_ron(&dir.join("leis_economicas.ron"), &self.catalogo.leis)?;
        escrever_arquivo_ron(
            &dir.join("metodos_producao.ron"),
            &self.catalogo.metodos_producao,
        )?;
        escrever_arquivo_ron(&dir.join("tipos_predio.ron"), &self.catalogo.tipos_predio)?;
        escrever_arquivo_ron(&dir.join("mercados.ron"), &self.mercados)?;
        escrever_arquivo_ron(&dir.join("paises.ron"), &self.paises)?;
        escrever_arquivo_ron(&dir.join("estados.ron"), &self.estados)?;
        let potenciais: Vec<crate::entidades::PotencialRegional> = self
            .estados
            .iter()
            .map(|e| crate::entidades::PotencialRegional {
                estado_id: e.id.clone(),
                terra_aravel: e.terra_aravel,
                culturas_permitidas: e.culturas_permitidas.clone(),
                recursos: e.limites_recursos.clone(),
                qualidade_recursos: e.qualidade_recursos.clone(),
            })
            .collect();
        escrever_arquivo_ron(&dir.join("potenciais.ron"), &potenciais)?;
        escrever_arquivo_ron(&dir.join("provincias.ron"), &self.provincias)?;
        escrever_arquivo_ron(&dir.join("rotas_comerciais.ron"), &self.rotas_comerciais)?;
        escrever_arquivo_ron(&dir.join("config_simulacao.ron"), &self.config)?;
        Ok(())
    }

    /// Serializa o cenário inteiro em uma única string RON formatada (útil para saves rápidos).
    pub fn para_ron_string(&self) -> ResultadoEconomia<String> {
        let pretty = PrettyConfig::new().depth_limit(6);
        to_string_pretty(self, pretty).map_err(|origem| ErroEconomia::SerializacaoRon {
            contexto: "CenarioEconomia".into(),
            origem: Box::new(origem),
        })
    }

    /// Desserializa um cenário completo a partir de uma única string RON.
    pub fn de_ron_str(conteudo: &str) -> ResultadoEconomia<Self> {
        let mut cenario: Self = ler_str_ron(conteudo, "CenarioEconomia")?;
        cenario.catalogo.validar_e_indexar()?;
        Ok(cenario)
    }
}
