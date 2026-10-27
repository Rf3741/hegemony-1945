//! Catálogo de dados militares orientados a dados: terrenos, tipos de unidade,
//! templates de divisão, tecnologias militares e traços de comandante.

use crate::erro::{ErroGuerra, ResultadoGuerra, validar_id_snake_case};
use crate::tipos::ClasseUnidade;
use ron::de::from_str as ron_from_str;
use ron::ser::{PrettyConfig, to_string_pretty};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Lê e desserializa um arquivo RON em uma estrutura genérica `T`.
pub fn ler_arquivo_ron<T: for<'de> Deserialize<'de>>(caminho: &Path) -> ResultadoGuerra<T> {
    let conteudo = fs::read_to_string(caminho).map_err(|origem| ErroGuerra::Io {
        caminho: caminho.to_path_buf(),
        origem,
    })?;
    ler_str_ron(&conteudo, &caminho.display().to_string())
}

/// Desserializa uma string no formato RON com contexto para mensagens de erro claras.
pub fn ler_str_ron<T: for<'de> Deserialize<'de>>(
    conteudo: &str,
    contexto: &str,
) -> ResultadoGuerra<T> {
    ron_from_str(conteudo).map_err(|origem| ErroGuerra::DesserializacaoRon {
        contexto: contexto.to_string(),
        origem,
    })
}

/// Propriedades de combate e mobilidade de um tipo de terreno.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TipoTerreno {
    pub id: String,
    pub nome_pt: String,
    #[serde(default = "padrao_largura_combate")]
    pub largura_combate: u32,
    #[serde(default = "padrao_um")]
    pub modificador_ataque: f64,
    #[serde(default = "padrao_um")]
    pub modificador_defesa: f64,
    #[serde(default = "padrao_um")]
    pub modificador_blindados: f64,
    #[serde(default = "padrao_um")]
    pub custo_suprimento: f64,
    #[serde(default)]
    pub atrito_base_diario: f64,
    #[serde(default = "padrao_um")]
    pub velocidade_avanco: f64,
}

const fn padrao_largura_combate() -> u32 {
    80
}

const fn padrao_um() -> f64 {
    1.0
}

/// Especificação técnica e tática de um batalhão componente de uma divisão.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TipoUnidade {
    pub id: String,
    pub nome_pt: String,
    pub classe: ClasseUnidade,
    pub ataque_leve: f64,
    pub ataque_pesado: f64,
    pub defesa: f64,
    pub ruptura: f64,
    pub blindagem: f64,
    pub perfuracao: f64,
    pub organizacao_base: f64,
    pub moral_recuperacao: f64,
    pub largura_combate: u32,
    pub efetivos_por_batalhao: u32,
    pub consumo_suprimento: f64,
    pub velocidade_base: f64,
    pub dureza: f64,
}

/// Template doutrinário de uma divisão, combinando múltiplos batalhões.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateDivisao {
    pub id: String,
    pub nome_pt: String,
    pub batalhoes: Vec<(String, u32)>,
}

/// Estatísticas agregadas calculadas para uma divisão a partir de seu template.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EstatisticasDivisao {
    pub efetivos_max: u32,
    pub ataque_leve: f64,
    pub ataque_pesado: f64,
    pub defesa: f64,
    pub ruptura: f64,
    pub blindagem: f64,
    pub perfuracao: f64,
    pub organizacao_max: f64,
    pub moral_recuperacao: f64,
    pub largura_combate: u32,
    pub consumo_suprimento: f64,
    pub velocidade: f64,
    pub dureza: f64,
    pub entrincheiramento_max: f64,
}

/// Tecnologia militar pesquisável que concede bônus permanentes às forças armadas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TecnologiaMilitar {
    pub id: String,
    pub nome_pt: String,
    #[serde(default = "padrao_era")]
    pub era: u8,
    #[serde(default)]
    pub bonus_ataque: f64,
    #[serde(default)]
    pub bonus_defesa: f64,
    #[serde(default)]
    pub bonus_ruptura: f64,
    #[serde(default)]
    pub bonus_organizacao_max: f64,
    #[serde(default)]
    pub bonus_recuperacao_moral: f64,
    #[serde(default)]
    pub bonus_entrincheiramento_max: f64,
    #[serde(default)]
    pub bonus_alcance_suprimento: f64,
    #[serde(default)]
    pub reducao_consumo_suprimento: f64,
    #[serde(default = "padrao_um_u8")]
    pub nivel_visual_militar: u8,
}

const fn padrao_era() -> u8 {
    1
}

const fn padrao_um_u8() -> u8 {
    1
}

/// Traço característico de personalidade ou doutrina militar de um comandante.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TracoComandante {
    pub id: String,
    pub nome_pt: String,
    #[serde(default)]
    pub bonus_ataque: f64,
    #[serde(default)]
    pub bonus_defesa: f64,
    #[serde(default)]
    pub bonus_organizacao: f64,
    #[serde(default)]
    pub bonus_moral: f64,
    #[serde(default)]
    pub bonus_suprimento: f64,
    #[serde(default)]
    pub bonus_entrincheiramento: f64,
    #[serde(default)]
    pub bonus_velocidade_avanco: f64,
}

/// Catálogo central de definições militares carregadas dos arquivos RON.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CatalogoGuerra {
    pub terrenos: Vec<TipoTerreno>,
    pub unidades: Vec<TipoUnidade>,
    pub templates: Vec<TemplateDivisao>,
    #[serde(default)]
    pub tecnologias: Vec<TecnologiaMilitar>,
    #[serde(default)]
    pub tracos: Vec<TracoComandante>,

    #[serde(skip)]
    pub mapa_terrenos: HashMap<String, usize>,
    #[serde(skip)]
    pub mapa_unidades: HashMap<String, usize>,
    #[serde(skip)]
    pub mapa_templates: HashMap<String, usize>,
    #[serde(skip)]
    pub mapa_tecnologias: HashMap<String, usize>,
    #[serde(skip)]
    pub mapa_tracos: HashMap<String, usize>,
    #[serde(skip)]
    pub templates_compilados: Vec<EstatisticasDivisao>,
}

impl CatalogoGuerra {
    /// Carrega o catálogo completo a partir de uma pasta contendo os arquivos RON.
    pub fn carregar_diretorio(dir: &Path) -> ResultadoGuerra<Self> {
        let terrenos: Vec<TipoTerreno> = ler_arquivo_ron(&dir.join("terrenos.ron"))?;
        let unidades: Vec<TipoUnidade> = ler_arquivo_ron(&dir.join("unidades.ron"))?;
        let templates: Vec<TemplateDivisao> = ler_arquivo_ron(&dir.join("templates_divisao.ron"))?;

        let cam_tec = dir.join("tecnologias_militares.ron");
        let tecnologias: Vec<TecnologiaMilitar> = if cam_tec.exists() {
            ler_arquivo_ron(&cam_tec)?
        } else {
            Vec::new()
        };

        let cam_tracos = dir.join("tracos_comandante.ron");
        let tracos: Vec<TracoComandante> = if cam_tracos.exists() {
            ler_arquivo_ron(&cam_tracos)?
        } else {
            Vec::new()
        };

        let mut cat = Self {
            terrenos,
            unidades,
            templates,
            tecnologias,
            tracos,
            ..Default::default()
        };
        cat.validar_e_indexar()?;
        Ok(cat)
    }

    /// Salva o catálogo em arquivos formatados no diretório especificado.
    pub fn salvar_diretorio(&self, dir: &Path) -> ResultadoGuerra<()> {
        fs::create_dir_all(dir).map_err(|origem| ErroGuerra::Io {
            caminho: dir.to_path_buf(),
            origem,
        })?;

        let pretty = PrettyConfig::new()
            .depth_limit(5)
            .separate_tuple_members(true);

        fn escrever_ron<T: Serialize>(
            caminho: &Path,
            valor: &T,
            pretty: PrettyConfig,
        ) -> ResultadoGuerra<()> {
            let texto =
                to_string_pretty(valor, pretty).map_err(|origem| ErroGuerra::SerializacaoRon {
                    contexto: caminho.display().to_string(),
                    origem,
                })?;
            fs::write(caminho, texto).map_err(|origem| ErroGuerra::Io {
                caminho: caminho.to_path_buf(),
                origem,
            })
        }

        escrever_ron(&dir.join("terrenos.ron"), &self.terrenos, pretty.clone())?;
        escrever_ron(&dir.join("unidades.ron"), &self.unidades, pretty.clone())?;
        escrever_ron(
            &dir.join("templates_divisao.ron"),
            &self.templates,
            pretty.clone(),
        )?;
        escrever_ron(
            &dir.join("tecnologias_militares.ron"),
            &self.tecnologias,
            pretty.clone(),
        )?;
        escrever_ron(&dir.join("tracos_comandante.ron"), &self.tracos, pretty)?;

        Ok(())
    }

    /// Valida IDs, unicidade e compila tabelas rápidas de busca e estatísticas de templates.
    pub fn validar_e_indexar(&mut self) -> ResultadoGuerra<()> {
        self.mapa_terrenos.clear();
        for (i, t) in self.terrenos.iter().enumerate() {
            validar_id_snake_case("terreno", &t.id)?;
            if self.mapa_terrenos.insert(t.id.clone(), i).is_some() {
                return Err(ErroGuerra::IdentificadorDuplicado {
                    entidade: "terreno",
                    id: t.id.clone(),
                });
            }
        }

        self.mapa_unidades.clear();
        for (i, u) in self.unidades.iter().enumerate() {
            validar_id_snake_case("tipo_unidade", &u.id)?;
            if self.mapa_unidades.insert(u.id.clone(), i).is_some() {
                return Err(ErroGuerra::IdentificadorDuplicado {
                    entidade: "tipo_unidade",
                    id: u.id.clone(),
                });
            }
        }

        self.mapa_tecnologias.clear();
        for (i, tec) in self.tecnologias.iter().enumerate() {
            validar_id_snake_case("tecnologia_militar", &tec.id)?;
            if self.mapa_tecnologias.insert(tec.id.clone(), i).is_some() {
                return Err(ErroGuerra::IdentificadorDuplicado {
                    entidade: "tecnologia_militar",
                    id: tec.id.clone(),
                });
            }
        }

        self.mapa_tracos.clear();
        for (i, tr) in self.tracos.iter().enumerate() {
            validar_id_snake_case("traco_comandante", &tr.id)?;
            if self.mapa_tracos.insert(tr.id.clone(), i).is_some() {
                return Err(ErroGuerra::IdentificadorDuplicado {
                    entidade: "traco_comandante",
                    id: tr.id.clone(),
                });
            }
        }

        self.mapa_templates.clear();
        self.templates_compilados.clear();
        self.templates_compilados.reserve(self.templates.len());

        for (i, tmpl) in self.templates.iter().enumerate() {
            validar_id_snake_case("template_divisao", &tmpl.id)?;
            if self.mapa_templates.insert(tmpl.id.clone(), i).is_some() {
                return Err(ErroGuerra::IdentificadorDuplicado {
                    entidade: "template_divisao",
                    id: tmpl.id.clone(),
                });
            }

            let mut stats = EstatisticasDivisao::default();
            let mut total_batalhoes = 0u32;
            let mut soma_org = 0.0;
            let mut soma_moral = 0.0;
            let mut soma_dureza = 0.0;
            let mut tem_engenharia = false;
            let mut velocidade_min = f64::MAX;

            for (unidade_id, qtd) in &tmpl.batalhoes {
                if *qtd == 0 {
                    continue;
                }
                let u_idx = *self.mapa_unidades.get(unidade_id).ok_or_else(|| {
                    ErroGuerra::ReferenciaInexistente {
                        origem_entidade: "template_divisao",
                        origem_id: tmpl.id.clone(),
                        alvo_entidade: "tipo_unidade",
                        alvo_id: unidade_id.clone(),
                    }
                })?;
                let u = &self.unidades[u_idx];

                let q = *qtd as f64;
                stats.efetivos_max += u.efetivos_por_batalhao * *qtd;
                stats.ataque_leve += u.ataque_leve * q;
                stats.ataque_pesado += u.ataque_pesado * q;
                stats.defesa += u.defesa * q;
                stats.ruptura += u.ruptura * q;
                stats.blindagem = stats.blindagem.max(u.blindagem);
                stats.perfuracao = stats.perfuracao.max(u.perfuracao);
                stats.largura_combate += u.largura_combate * *qtd;
                stats.consumo_suprimento += u.consumo_suprimento * q;

                soma_org += u.organizacao_base * q;
                soma_moral += u.moral_recuperacao * q;
                soma_dureza += u.dureza * q;
                total_batalhoes += *qtd;

                if u.classe == ClasseUnidade::Engenharia {
                    tem_engenharia = true;
                }
                if u.velocidade_base < velocidade_min {
                    velocidade_min = u.velocidade_base;
                }
            }

            if total_batalhoes > 0 {
                let tb = total_batalhoes as f64;
                stats.organizacao_max = soma_org / tb;
                stats.moral_recuperacao = soma_moral / tb;
                stats.dureza = (soma_dureza / tb).clamp(0.0, 1.0);
                stats.velocidade = if velocidade_min == f64::MAX {
                    4.0
                } else {
                    velocidade_min
                };
            } else {
                stats.organizacao_max = 30.0;
                stats.moral_recuperacao = 0.30;
                stats.velocidade = 4.0;
            }

            // Entrincheiramento base é 10.0, e engenharia eleva para 18.0
            stats.entrincheiramento_max = if tem_engenharia { 18.0 } else { 10.0 };

            self.templates_compilados.push(stats);
        }

        Ok(())
    }
}
