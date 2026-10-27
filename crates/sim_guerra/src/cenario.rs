//! Cenário modular de guerra serializável em arquivos RON (`CenarioGuerra`).

use crate::catalogo::{CatalogoGuerra, ler_arquivo_ron, ler_str_ron};
use crate::entidades::{
    Comandante, ConfigGuerra, Divisao, Exercito, Guerra, PaisMilitar, RegiaoMapa,
};
use crate::erro::{ErroGuerra, ResultadoGuerra};
use crate::grafo::ArestaRegiao;
use crate::tipos::DataJogo;
use ron::ser::{PrettyConfig, to_string_pretty};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const fn padrao_seed() -> u64 {
    42
}

/// Pacote completo contendo catálogo, geografia, beligerantes e ordens de batalha.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CenarioGuerra {
    #[serde(default)]
    pub data_inicial: DataJogo,
    #[serde(default = "padrao_seed")]
    pub seed: u64,
    #[serde(default)]
    pub config: ConfigGuerra,
    pub catalogo: CatalogoGuerra,
    pub regioes: Vec<RegiaoMapa>,
    pub arestas: Vec<ArestaRegiao>,
    pub paises: Vec<PaisMilitar>,
    #[serde(default)]
    pub comandantes: Vec<Comandante>,
    #[serde(default)]
    pub divisoes: Vec<Divisao>,
    #[serde(default)]
    pub exercitos: Vec<Exercito>,
    #[serde(default)]
    pub guerras: Vec<Guerra>,
}

fn escrever_arquivo_ron<T: Serialize>(caminho: &Path, valor: &T) -> ResultadoGuerra<()> {
    let pretty = PrettyConfig::new()
        .depth_limit(5)
        .separate_tuple_members(true)
        .enumerate_arrays(false);
    let texto = to_string_pretty(valor, pretty).map_err(|origem| ErroGuerra::SerializacaoRon {
        contexto: caminho.display().to_string(),
        origem,
    })?;
    fs::write(caminho, texto).map_err(|origem| ErroGuerra::Io {
        caminho: caminho.to_path_buf(),
        origem,
    })
}

impl CenarioGuerra {
    /// Carrega o cenário completo a partir de uma pasta contendo os arquivos RON.
    pub fn carregar_diretorio(dir: &Path) -> ResultadoGuerra<Self> {
        let catalogo = CatalogoGuerra::carregar_diretorio(dir)?;
        let regioes: Vec<RegiaoMapa> = ler_arquivo_ron(&dir.join("regioes.ron"))?;
        let arestas: Vec<ArestaRegiao> = ler_arquivo_ron(&dir.join("arestas.ron"))?;
        let paises: Vec<PaisMilitar> = ler_arquivo_ron(&dir.join("paises.ron"))?;

        let cam_com = dir.join("comandantes.ron");
        let comandantes: Vec<Comandante> = if cam_com.exists() {
            ler_arquivo_ron(&cam_com)?
        } else {
            Vec::new()
        };

        let cam_div = dir.join("divisoes.ron");
        let divisoes: Vec<Divisao> = if cam_div.exists() {
            ler_arquivo_ron(&cam_div)?
        } else {
            Vec::new()
        };

        let cam_exe = dir.join("exercitos.ron");
        let exercitos: Vec<Exercito> = if cam_exe.exists() {
            ler_arquivo_ron(&cam_exe)?
        } else {
            Vec::new()
        };

        let cam_gue = dir.join("guerras.ron");
        let guerras: Vec<Guerra> = if cam_gue.exists() {
            ler_arquivo_ron(&cam_gue)?
        } else {
            Vec::new()
        };

        let cam_cfg = dir.join("config_guerra.ron");
        let config: ConfigGuerra = if cam_cfg.exists() {
            ler_arquivo_ron(&cam_cfg)?
        } else {
            ConfigGuerra::default()
        };

        Ok(Self {
            data_inicial: DataJogo::default(),
            seed: 42,
            config,
            catalogo,
            regioes,
            arestas,
            paises,
            comandantes,
            divisoes,
            exercitos,
            guerras,
        })
    }

    /// Salva os componentes do cenário em arquivos RON formatados.
    pub fn salvar_diretorio(&self, dir: &Path) -> ResultadoGuerra<()> {
        fs::create_dir_all(dir).map_err(|origem| ErroGuerra::Io {
            caminho: dir.to_path_buf(),
            origem,
        })?;
        self.catalogo.salvar_diretorio(dir)?;
        escrever_arquivo_ron(&dir.join("regioes.ron"), &self.regioes)?;
        escrever_arquivo_ron(&dir.join("arestas.ron"), &self.arestas)?;
        escrever_arquivo_ron(&dir.join("paises.ron"), &self.paises)?;
        escrever_arquivo_ron(&dir.join("comandantes.ron"), &self.comandantes)?;
        escrever_arquivo_ron(&dir.join("divisoes.ron"), &self.divisoes)?;
        escrever_arquivo_ron(&dir.join("exercitos.ron"), &self.exercitos)?;
        escrever_arquivo_ron(&dir.join("guerras.ron"), &self.guerras)?;
        escrever_arquivo_ron(&dir.join("config_guerra.ron"), &self.config)?;
        Ok(())
    }

    /// Serializa todo o cenário em uma string RON única.
    pub fn para_ron_string(&self) -> ResultadoGuerra<String> {
        let pretty = PrettyConfig::new().depth_limit(5);
        to_string_pretty(self, pretty).map_err(|origem| ErroGuerra::SerializacaoRon {
            contexto: "CenarioGuerra".into(),
            origem,
        })
    }

    /// Desserializa um cenário completo a partir de uma string RON.
    pub fn de_ron_str(conteudo: &str) -> ResultadoGuerra<Self> {
        let mut cenario: Self = ler_str_ron(conteudo, "CenarioGuerra")?;
        cenario.catalogo.validar_e_indexar()?;
        Ok(cenario)
    }
}
