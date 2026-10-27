//! Catálogo estático de definições territoriais carregáveis de arquivos RON:
//! culturas, prédios urbanos, tecnologias de era e métodos de produção.

use crate::erro::{ErroTerritorio, ResultadoTerritorio, validar_id_snake_case};
use crate::tipos::EraTecnologicaVisual;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Registro de cultura (compatível com `nacoes/culturas.ron`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cultura {
    pub id: String,
    pub nome: String,
    pub grupo_linguistico: String,
    pub heranca_cultural: String,
}

/// Classificação funcional-urbana de um prédio para efeitos territoriais e visuais.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum ClassePredioUrbano {
    #[default]
    RuralAgricola,
    ExtrativoMineral,
    IndustriaLeve,
    IndustriaPesada,
    IndustriaAltaTecnologia,
    UrbanoServicosGovernamental,
    EnergiaInfraestrutura,
    Militar,
}

/// Definição de características urbanas, industriais e de infraestrutura de um prédio.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefinicaoPredioUrbano {
    pub id: String,
    pub nome: String,
    pub classe_urbana: ClassePredioUrbano,
    /// Pontos de urbanização adicionados por nível do prédio (estilo Victoria 3).
    pub pontos_urbanizacao_por_nivel: f64,
    /// Empregos industriais formais gerados por nível quando ativo.
    #[serde(default)]
    pub empregos_industriais_por_nivel: u32,
    /// Empregos no setor de serviços gerados por nível.
    #[serde(default)]
    pub empregos_servicos_por_nivel: u32,
    /// Empregos rurais associados por nível.
    #[serde(default)]
    pub empregos_rurais_por_nivel: u32,
    /// Contribuição ponderada para o nível industrial visual do estado.
    #[serde(default)]
    pub peso_nivel_industrial: f64,
    /// Demanda diária de capacidade rodoviária por nível.
    #[serde(default)]
    pub demanda_estradas_por_nivel: f64,
    /// Demanda diária de capacidade ferroviária por nível.
    #[serde(default)]
    pub demanda_ferrovias_por_nivel: f64,
    /// Demanda diária de capacidade portuária por nível.
    #[serde(default)]
    pub demanda_portos_por_nivel: f64,
    /// Demanda diária de capacidade aeroportuária por nível.
    #[serde(default)]
    pub demanda_aeroportos_por_nivel: f64,
    /// Demanda diária de energia elétrica por nível (MW-equivalente).
    #[serde(default)]
    pub demanda_eletricidade_por_nivel: f64,
    /// Capacidade de geração elétrica adicionada à rede do estado por nível.
    #[serde(default)]
    pub capacidade_geracao_eletrica: f64,
    /// Capacidade portuária adicionada à rede do estado por nível.
    #[serde(default)]
    pub capacidade_portuaria_gerada: f64,
}

/// Pilar de pesquisa e aplicação tecnológica.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum PilarTecnologico {
    #[default]
    ProducaoComputacao,
    SociedadeUrbanismo,
    InfraestruturaEnergia,
    AeroespacialNuclear,
}

/// Definição de tecnologia associada a uma era visual.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefinicaoTecnologiaEra {
    pub id: String,
    pub nome: String,
    pub era: EraTecnologicaVisual,
    pub pilar: PilarTecnologico,
    #[serde(default = "padrao_peso_um")]
    pub peso_visual: f64,
    #[serde(default)]
    pub descricao: String,
}

const fn padrao_peso_um() -> f64 {
    1.0
}

/// Mapeamento de método de produção (`sim_economia`) para a era tecnológica visual mínima.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefinicaoMetodoEra {
    pub metodo_id: String,
    pub nome: String,
    pub era_minima: EraTecnologicaVisual,
    #[serde(default = "padrao_peso_um")]
    pub peso_adocao: f64,
}

/// Catálogo global de regras estáticas e dados-mestre do território.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CatalogoTerritorio {
    pub culturas: Vec<Cultura>,
    pub predios_urbanos: Vec<DefinicaoPredioUrbano>,
    pub tecnologias_eras: Vec<DefinicaoTecnologiaEra>,
    pub metodos_eras: Vec<DefinicaoMetodoEra>,

    #[serde(skip)]
    pub(crate) indice_culturas: BTreeMap<String, usize>,
    #[serde(skip)]
    pub(crate) indice_predios: BTreeMap<String, usize>,
    #[serde(skip)]
    pub(crate) indice_tecnologias: BTreeMap<String, usize>,
    #[serde(skip)]
    pub(crate) indice_metodos: BTreeMap<String, usize>,
}

/// Função auxiliar genérica para desserializar um arquivo RON.
pub fn ler_arquivo_ron<T: for<'de> Deserialize<'de>>(caminho: &Path) -> ResultadoTerritorio<T> {
    let conteudo = fs::read_to_string(caminho).map_err(|origem| ErroTerritorio::Io {
        caminho: caminho.to_path_buf(),
        origem,
    })?;
    ron::from_str(&conteudo).map_err(|origem| ErroTerritorio::DesserializacaoRon {
        contexto: caminho.display().to_string(),
        origem,
    })
}

/// Função auxiliar genérica para desserializar uma string em formato RON.
pub fn ler_str_ron<T: for<'de> Deserialize<'de>>(
    conteudo: &str,
    contexto: &str,
) -> ResultadoTerritorio<T> {
    ron::from_str(conteudo).map_err(|origem| ErroTerritorio::DesserializacaoRon {
        contexto: contexto.to_string(),
        origem,
    })
}

impl CatalogoTerritorio {
    /// Carrega o catálogo completo a partir de uma pasta com os arquivos:
    /// `culturas.ron`, `predios_urbanos.ron`, `tecnologias_eras.ron` e `metodos_eras.ron`.
    pub fn carregar_diretorio(dir: &Path) -> ResultadoTerritorio<Self> {
        let culturas: Vec<Cultura> = ler_arquivo_ron(&dir.join("culturas.ron"))?;
        let predios_urbanos: Vec<DefinicaoPredioUrbano> =
            ler_arquivo_ron(&dir.join("predios_urbanos.ron"))?;
        let tecnologias_eras: Vec<DefinicaoTecnologiaEra> =
            ler_arquivo_ron(&dir.join("tecnologias_eras.ron"))?;
        let metodos_eras: Vec<DefinicaoMetodoEra> = ler_arquivo_ron(&dir.join("metodos_eras.ron"))?;

        let mut catalogo = Self {
            culturas,
            predios_urbanos,
            tecnologias_eras,
            metodos_eras,
            ..Default::default()
        };
        catalogo.validar_e_indexar()?;
        Ok(catalogo)
    }

    /// Valida integridade referencial, sintaxe snake_case e constrói índices O(1).
    pub fn validar_e_indexar(&mut self) -> ResultadoTerritorio<()> {
        self.indice_culturas.clear();
        self.indice_predios.clear();
        self.indice_tecnologias.clear();
        self.indice_metodos.clear();

        for (idx, c) in self.culturas.iter().enumerate() {
            validar_id_snake_case("Cultura", &c.id)?;
            if self.indice_culturas.insert(c.id.clone(), idx).is_some() {
                return Err(ErroTerritorio::IdentificadorDuplicado {
                    entidade: "Cultura",
                    id: c.id.clone(),
                });
            }
        }

        for (idx, p) in self.predios_urbanos.iter().enumerate() {
            validar_id_snake_case("DefinicaoPredioUrbano", &p.id)?;
            if self.indice_predios.insert(p.id.clone(), idx).is_some() {
                return Err(ErroTerritorio::IdentificadorDuplicado {
                    entidade: "DefinicaoPredioUrbano",
                    id: p.id.clone(),
                });
            }
        }

        for (idx, t) in self.tecnologias_eras.iter().enumerate() {
            validar_id_snake_case("DefinicaoTecnologiaEra", &t.id)?;
            if self.indice_tecnologias.insert(t.id.clone(), idx).is_some() {
                return Err(ErroTerritorio::IdentificadorDuplicado {
                    entidade: "DefinicaoTecnologiaEra",
                    id: t.id.clone(),
                });
            }
        }

        for (idx, m) in self.metodos_eras.iter().enumerate() {
            validar_id_snake_case("DefinicaoMetodoEra", &m.metodo_id)?;
            if self
                .indice_metodos
                .insert(m.metodo_id.clone(), idx)
                .is_some()
            {
                return Err(ErroTerritorio::IdentificadorDuplicado {
                    entidade: "DefinicaoMetodoEra",
                    id: m.metodo_id.clone(),
                });
            }
        }

        Ok(())
    }

    /// Busca a definição de uma cultura pelo identificador.
    pub fn buscar_cultura(&self, id: &str) -> Option<&Cultura> {
        self.indice_culturas.get(id).map(|&idx| &self.culturas[idx])
    }

    /// Busca a definição de um prédio pelo identificador.
    pub fn buscar_predio(&self, id: &str) -> Option<&DefinicaoPredioUrbano> {
        self.indice_predios
            .get(id)
            .map(|&idx| &self.predios_urbanos[idx])
    }

    /// Busca a tecnologia de era pelo identificador.
    pub fn buscar_tecnologia(&self, id: &str) -> Option<&DefinicaoTecnologiaEra> {
        self.indice_tecnologias
            .get(id)
            .map(|&idx| &self.tecnologias_eras[idx])
    }

    /// Busca a era visual de um método de produção.
    pub fn buscar_metodo(&self, id: &str) -> Option<&DefinicaoMetodoEra> {
        self.indice_metodos
            .get(id)
            .map(|&idx| &self.metodos_eras[idx])
    }

    /// Catálogo padrão com dados históricos da Guerra Fria embutidos.
    pub fn padrao_guerra_fria() -> Self {
        let mut catalogo = Self {
            culturas: vec![
                Cultura {
                    id: "anglo_americano".into(),
                    nome: "Anglo-Americana".into(),
                    grupo_linguistico: "germanico".into(),
                    heranca_cultural: "europeia".into(),
                },
                Cultura {
                    id: "afro_americano".into(),
                    nome: "Afro-Americana".into(),
                    grupo_linguistico: "germanico".into(),
                    heranca_cultural: "africana".into(),
                },
                Cultura {
                    id: "brasileira".into(),
                    nome: "Brasileira".into(),
                    grupo_linguistico: "romanico".into(),
                    heranca_cultural: "latino_americana".into(),
                },
                Cultura {
                    id: "russa".into(),
                    nome: "Russa".into(),
                    grupo_linguistico: "eslavo".into(),
                    heranca_cultural: "europeia_oriental".into(),
                },
                Cultura {
                    id: "ucraniana".into(),
                    nome: "Ucraniana".into(),
                    grupo_linguistico: "eslavo".into(),
                    heranca_cultural: "europeia_oriental".into(),
                },
                Cultura {
                    id: "polonesa".into(),
                    nome: "Polonesa".into(),
                    grupo_linguistico: "eslavo".into(),
                    heranca_cultural: "europeia_oriental".into(),
                },
                Cultura {
                    id: "alema".into(),
                    nome: "Alemã".into(),
                    grupo_linguistico: "germanico".into(),
                    heranca_cultural: "europeia".into(),
                },
                Cultura {
                    id: "francesa".into(),
                    nome: "Francesa".into(),
                    grupo_linguistico: "romanico".into(),
                    heranca_cultural: "europeia".into(),
                },
                Cultura {
                    id: "britanica".into(),
                    nome: "Britânica".into(),
                    grupo_linguistico: "germanico".into(),
                    heranca_cultural: "europeia".into(),
                },
                Cultura {
                    id: "hindustani".into(),
                    nome: "Hindustâni (Índia)".into(),
                    grupo_linguistico: "indo_ariano".into(),
                    heranca_cultural: "sul_asiatica".into(),
                },
                Cultura {
                    id: "punjabi".into(),
                    nome: "Punjabi (Paquistão/Índia)".into(),
                    grupo_linguistico: "indo_ariano".into(),
                    heranca_cultural: "sul_asiatica".into(),
                },
                Cultura {
                    id: "bengali".into(),
                    nome: "Bengali".into(),
                    grupo_linguistico: "indo_ariano".into(),
                    heranca_cultural: "sul_asiatica".into(),
                },
                Cultura {
                    id: "arabe_magrebina".into(),
                    nome: "Árabe Magrebina (Argélia)".into(),
                    grupo_linguistico: "semitico".into(),
                    heranca_cultural: "arabe_islamica".into(),
                },
                Cultura {
                    id: "berbere".into(),
                    nome: "Berbere".into(),
                    grupo_linguistico: "berbere".into(),
                    heranca_cultural: "norte_africana".into(),
                },
            ],
            predios_urbanos: vec![
                DefinicaoPredioUrbano {
                    id: "fazenda".into(),
                    nome: "Fazenda e Agropecuária".into(),
                    classe_urbana: ClassePredioUrbano::RuralAgricola,
                    pontos_urbanizacao_por_nivel: 4.0,
                    empregos_industriais_por_nivel: 0,
                    empregos_servicos_por_nivel: 30,
                    empregos_rurais_por_nivel: 450,
                    peso_nivel_industrial: 0.1,
                    demanda_estradas_por_nivel: 1.5,
                    demanda_ferrovias_por_nivel: 0.5,
                    demanda_portos_por_nivel: 0.2,
                    demanda_aeroportos_por_nivel: 0.0,
                    demanda_eletricidade_por_nivel: 0.2,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "fazenda_graos".into(),
                    nome: "Fazenda de Grãos".into(),
                    classe_urbana: ClassePredioUrbano::RuralAgricola,
                    pontos_urbanizacao_por_nivel: 5.0,
                    empregos_industriais_por_nivel: 0,
                    empregos_servicos_por_nivel: 40,
                    empregos_rurais_por_nivel: 500,
                    peso_nivel_industrial: 0.1,
                    demanda_estradas_por_nivel: 2.0,
                    demanda_ferrovias_por_nivel: 1.0,
                    demanda_portos_por_nivel: 0.5,
                    demanda_aeroportos_por_nivel: 0.0,
                    demanda_eletricidade_por_nivel: 0.3,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "mina_carvao".into(),
                    nome: "Mina de Carvão".into(),
                    classe_urbana: ClassePredioUrbano::ExtrativoMineral,
                    pontos_urbanizacao_por_nivel: 15.0,
                    empregos_industriais_por_nivel: 450,
                    empregos_servicos_por_nivel: 80,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 1.8,
                    demanda_estradas_por_nivel: 2.0,
                    demanda_ferrovias_por_nivel: 5.0,
                    demanda_portos_por_nivel: 1.5,
                    demanda_aeroportos_por_nivel: 0.0,
                    demanda_eletricidade_por_nivel: 1.5,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "mina_ferro".into(),
                    nome: "Mina de Minério de Ferro".into(),
                    classe_urbana: ClassePredioUrbano::ExtrativoMineral,
                    pontos_urbanizacao_por_nivel: 16.0,
                    empregos_industriais_por_nivel: 460,
                    empregos_servicos_por_nivel: 80,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 1.9,
                    demanda_estradas_por_nivel: 2.0,
                    demanda_ferrovias_por_nivel: 6.0,
                    demanda_portos_por_nivel: 2.0,
                    demanda_aeroportos_por_nivel: 0.0,
                    demanda_eletricidade_por_nivel: 1.6,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "campo_petrolifero".into(),
                    nome: "Campo Petrolífero".into(),
                    classe_urbana: ClassePredioUrbano::ExtrativoMineral,
                    pontos_urbanizacao_por_nivel: 18.0,
                    empregos_industriais_por_nivel: 400,
                    empregos_servicos_por_nivel: 120,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 2.5,
                    demanda_estradas_por_nivel: 3.0,
                    demanda_ferrovias_por_nivel: 4.0,
                    demanda_portos_por_nivel: 3.5,
                    demanda_aeroportos_por_nivel: 0.5,
                    demanda_eletricidade_por_nivel: 2.0,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "industria_alimentos".into(),
                    nome: "Indústria de Alimentos e Conservas".into(),
                    classe_urbana: ClassePredioUrbano::IndustriaLeve,
                    pontos_urbanizacao_por_nivel: 24.0,
                    empregos_industriais_por_nivel: 500,
                    empregos_servicos_por_nivel: 140,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 2.0,
                    demanda_estradas_por_nivel: 4.0,
                    demanda_ferrovias_por_nivel: 3.0,
                    demanda_portos_por_nivel: 1.5,
                    demanda_aeroportos_por_nivel: 0.2,
                    demanda_eletricidade_por_nivel: 2.5,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "industria_bens_consumo".into(),
                    nome: "Manufatura de Bens de Consumo e Têxtil".into(),
                    classe_urbana: ClassePredioUrbano::IndustriaLeve,
                    pontos_urbanizacao_por_nivel: 28.0,
                    empregos_industriais_por_nivel: 550,
                    empregos_servicos_por_nivel: 160,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 2.5,
                    demanda_estradas_por_nivel: 5.0,
                    demanda_ferrovias_por_nivel: 3.5,
                    demanda_portos_por_nivel: 2.0,
                    demanda_aeroportos_por_nivel: 0.3,
                    demanda_eletricidade_por_nivel: 3.0,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "industria_pesada".into(),
                    nome: "Complexo Siderúrgico e Metalúrgico".into(),
                    classe_urbana: ClassePredioUrbano::IndustriaPesada,
                    pontos_urbanizacao_por_nivel: 36.0,
                    empregos_industriais_por_nivel: 600,
                    empregos_servicos_por_nivel: 180,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 4.5,
                    demanda_estradas_por_nivel: 6.0,
                    demanda_ferrovias_por_nivel: 8.0,
                    demanda_portos_por_nivel: 4.0,
                    demanda_aeroportos_por_nivel: 0.5,
                    demanda_eletricidade_por_nivel: 6.5,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "usina_siderurgica".into(),
                    nome: "Usina Siderúrgica e Aciaria".into(),
                    classe_urbana: ClassePredioUrbano::IndustriaPesada,
                    pontos_urbanizacao_por_nivel: 38.0,
                    empregos_industriais_por_nivel: 620,
                    empregos_servicos_por_nivel: 190,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 4.8,
                    demanda_estradas_por_nivel: 6.0,
                    demanda_ferrovias_por_nivel: 9.0,
                    demanda_portos_por_nivel: 4.5,
                    demanda_aeroportos_por_nivel: 0.5,
                    demanda_eletricidade_por_nivel: 7.0,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "industria_eletronica".into(),
                    nome: "Fábrica Eletrônica e Informática".into(),
                    classe_urbana: ClassePredioUrbano::IndustriaAltaTecnologia,
                    pontos_urbanizacao_por_nivel: 42.0,
                    empregos_industriais_por_nivel: 480,
                    empregos_servicos_por_nivel: 280,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 4.0,
                    demanda_estradas_por_nivel: 4.0,
                    demanda_ferrovias_por_nivel: 2.5,
                    demanda_portos_por_nivel: 2.0,
                    demanda_aeroportos_por_nivel: 2.5,
                    demanda_eletricidade_por_nivel: 5.5,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "centro_urbano".into(),
                    nome: "Centro Urbano e Financeiro".into(),
                    classe_urbana: ClassePredioUrbano::UrbanoServicosGovernamental,
                    pontos_urbanizacao_por_nivel: 50.0,
                    empregos_industriais_por_nivel: 50,
                    empregos_servicos_por_nivel: 750,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 0.5,
                    demanda_estradas_por_nivel: 8.0,
                    demanda_ferrovias_por_nivel: 4.0,
                    demanda_portos_por_nivel: 2.0,
                    demanda_aeroportos_por_nivel: 3.0,
                    demanda_eletricidade_por_nivel: 6.0,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 0.0,
                },
                DefinicaoPredioUrbano {
                    id: "porto_comercial".into(),
                    nome: "Porto Comercial e Marítimo".into(),
                    classe_urbana: ClassePredioUrbano::EnergiaInfraestrutura,
                    pontos_urbanizacao_por_nivel: 35.0,
                    empregos_industriais_por_nivel: 250,
                    empregos_servicos_por_nivel: 450,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 2.0,
                    demanda_estradas_por_nivel: 5.0,
                    demanda_ferrovias_por_nivel: 7.0,
                    demanda_portos_por_nivel: 0.0,
                    demanda_aeroportos_por_nivel: 1.0,
                    demanda_eletricidade_por_nivel: 3.5,
                    capacidade_geracao_eletrica: 0.0,
                    capacidade_portuaria_gerada: 45.0,
                },
                DefinicaoPredioUrbano {
                    id: "usina_eletrica".into(),
                    nome: "Usina de Energia Elétrica".into(),
                    classe_urbana: ClassePredioUrbano::EnergiaInfraestrutura,
                    pontos_urbanizacao_por_nivel: 20.0,
                    empregos_industriais_por_nivel: 200,
                    empregos_servicos_por_nivel: 100,
                    empregos_rurais_por_nivel: 0,
                    peso_nivel_industrial: 1.5,
                    demanda_estradas_por_nivel: 2.0,
                    demanda_ferrovias_por_nivel: 4.0,
                    demanda_portos_por_nivel: 1.0,
                    demanda_aeroportos_por_nivel: 0.0,
                    demanda_eletricidade_por_nivel: 0.0,
                    capacidade_geracao_eletrica: 80.0,
                    capacidade_portuaria_gerada: 0.0,
                },
            ],
            tecnologias_eras: vec![
                DefinicaoTecnologiaEra {
                    id: "reconstrucao_e_vapor".into(),
                    nome: "Reconstrução do Pós-Guerra e Máquinas a Vapor".into(),
                    era: EraTecnologicaVisual::Era1945,
                    pilar: PilarTecnologico::ProducaoComputacao,
                    peso_visual: 1.0,
                    descricao: "Restabelecimento fabril com alvenaria tradicional e caldeiras."
                        .into(),
                },
                DefinicaoTecnologiaEra {
                    id: "linha_montagem_fordista".into(),
                    nome: "Linha de Montagem Fordista e Eletrodomésticos".into(),
                    era: EraTecnologicaVisual::Era1955,
                    pilar: PilarTecnologico::ProducaoComputacao,
                    peso_visual: 1.5,
                    descricao: "Produção em série, galpões amplos e expansão suburbana.".into(),
                },
                DefinicaoTecnologiaEra {
                    id: "arquitetura_modernista_concreto".into(),
                    nome: "Modernismo em Concreto Armado e Rodovias".into(),
                    era: EraTecnologicaVisual::Era1955,
                    pilar: PilarTecnologico::SociedadeUrbanismo,
                    peso_visual: 1.5,
                    descricao: "Blocos residenciais modernistas e avenidas arteriais.".into(),
                },
                DefinicaoTecnologiaEra {
                    id: "aviacao_comercial_jato".into(),
                    nome: "Aviação Comercial a Jato e Aeroportos Internacionais".into(),
                    era: EraTecnologicaVisual::Era1965,
                    pilar: PilarTecnologico::AeroespacialNuclear,
                    peso_visual: 2.0,
                    descricao: "Conexão aérea rápida entre metrópoles e pistas asfaltadas.".into(),
                },
                DefinicaoTecnologiaEra {
                    id: "conteinerizacao_maritima".into(),
                    nome: "Conteinerização Marítima Intermodal".into(),
                    era: EraTecnologicaVisual::Era1965,
                    pilar: PilarTecnologico::InfraestruturaEnergia,
                    peso_visual: 2.0,
                    descricao: "Guindastes portuários modernos e pátios de contêineres.".into(),
                },
                DefinicaoTecnologiaEra {
                    id: "energia_nuclear_civil".into(),
                    nome: "Centrais Termonucleares Civis".into(),
                    era: EraTecnologicaVisual::Era1975,
                    pilar: PilarTecnologico::InfraestruturaEnergia,
                    peso_visual: 2.5,
                    descricao: "Torres de resfriamento hiperbólicas e eletrificação limpa.".into(),
                },
                DefinicaoTecnologiaEra {
                    id: "automacao_maquinas_ferramenta".into(),
                    nome: "Controle Numérico e Automação Industrial".into(),
                    era: EraTecnologicaVisual::Era1975,
                    pilar: PilarTecnologico::ProducaoComputacao,
                    peso_visual: 2.5,
                    descricao: "Usinagem de alta precisão e centros de comando eletrônicos.".into(),
                },
                DefinicaoTecnologiaEra {
                    id: "microeletronica_e_fibra_optica".into(),
                    nome: "Microeletrônica, Robótica e Fibra Óptica".into(),
                    era: EraTecnologicaVisual::Era1985,
                    pilar: PilarTecnologico::ProducaoComputacao,
                    peso_visual: 3.0,
                    descricao: "Robótica fabril CNC e parques tecnológicos integrados.".into(),
                },
            ],
            metodos_eras: vec![
                DefinicaoMetodoEra {
                    metodo_id: "agricultura_tradicional".into(),
                    nome: "Agricultura com Tração Animal".into(),
                    era_minima: EraTecnologicaVisual::Era1945,
                    peso_adocao: 1.0,
                },
                DefinicaoMetodoEra {
                    metodo_id: "siderurgia_forno_aberto".into(),
                    nome: "Forno Siemens-Martin".into(),
                    era_minima: EraTecnologicaVisual::Era1945,
                    peso_adocao: 1.0,
                },
                DefinicaoMetodoEra {
                    metodo_id: "agricultura_mecanizada".into(),
                    nome: "Agricultura Mecanizada com Tratores".into(),
                    era_minima: EraTecnologicaVisual::Era1955,
                    peso_adocao: 1.5,
                },
                DefinicaoMetodoEra {
                    metodo_id: "linha_montagem_eletrodomesticos".into(),
                    nome: "Linha de Montagem Eletrodomésticos".into(),
                    era_minima: EraTecnologicaVisual::Era1955,
                    peso_adocao: 1.5,
                },
                DefinicaoMetodoEra {
                    metodo_id: "siderurgia_arco_eletrico".into(),
                    nome: "Aciaria Elétrica de Alta Eficiência".into(),
                    era_minima: EraTecnologicaVisual::Era1965,
                    peso_adocao: 2.0,
                },
                DefinicaoMetodoEra {
                    metodo_id: "automacao_esteiras_ferrovia".into(),
                    nome: "Esteiras e Ramal Ferroviário Automatizado".into(),
                    era_minima: EraTecnologicaVisual::Era1965,
                    peso_adocao: 2.0,
                },
                DefinicaoMetodoEra {
                    metodo_id: "geracao_termonuclear_civil".into(),
                    nome: "Geração Termonuclear PWR/BWR".into(),
                    era_minima: EraTecnologicaVisual::Era1975,
                    peso_adocao: 2.5,
                },
                DefinicaoMetodoEra {
                    metodo_id: "automacao_robotica_cnc".into(),
                    nome: "Robótica Industrial e Células CNC".into(),
                    era_minima: EraTecnologicaVisual::Era1985,
                    peso_adocao: 3.0,
                },
            ],
            ..Default::default()
        };
        catalogo.validar_e_indexar().unwrap();
        catalogo
    }
}
