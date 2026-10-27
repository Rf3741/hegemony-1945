//! CenarioTerritorio e MundoTerritorial: motor integrado com indexação O(1),
//! hierarquia completa, avanço diário de simulação e API ergonômica.

use crate::catalogo::{CatalogoTerritorio, ler_arquivo_ron};
use crate::descolonizacao::{
    RelatorioDescolonizacao, atualizar_fervor_diario, conceder_independencia_negociada,
};
use crate::entidades::{
    Estado, MovimentoIndependencia, PaisTerritorial, PopRegional, Regiao, RegiaoEstrategica,
};
use crate::eras::{calcular_era_visual_estado, calcular_era_visual_pais};
use crate::erro::{ErroTerritorio, ResultadoTerritorio, validar_adm1_code, validar_id_snake_case};
use crate::tipos::{
    DataJogo, EraTecnologicaVisual, ModoTransferenciaTerritorial, SetorEmpregoPop,
    TipoInfraestrutura,
};
use crate::transferencia::{RelatorioTransferencia, transferir_estado};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Cenário territorial serializável em RON com todas as definições geopolíticas de partida.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CenarioTerritorio {
    pub data: DataJogo,
    pub regioes_estrategicas: Vec<RegiaoEstrategica>,
    pub paises: Vec<PaisTerritorial>,
    pub estados: Vec<Estado>,
    pub regioes: Vec<Regiao>,
    pub movimentos_independencia: Vec<MovimentoIndependencia>,
}

impl CenarioTerritorio {
    /// Carrega o cenário completo a partir de uma pasta de dados contendo:
    /// `regioes_estrategicas.ron`, `paises.ron`, `estados.ron`, `regioes.ron` e `movimentos_independencia.ron`.
    pub fn carregar_diretorio(dir: &Path) -> ResultadoTerritorio<Self> {
        let regioes_estrategicas: Vec<RegiaoEstrategica> =
            ler_arquivo_ron(&dir.join("regioes_estrategicas.ron"))?;
        let paises: Vec<PaisTerritorial> = ler_arquivo_ron(&dir.join("paises.ron"))?;
        let estados: Vec<Estado> = ler_arquivo_ron(&dir.join("estados.ron"))?;
        let regioes: Vec<Regiao> = ler_arquivo_ron(&dir.join("regioes.ron"))?;
        let movimentos_independencia: Vec<MovimentoIndependencia> =
            ler_arquivo_ron(&dir.join("movimentos_independencia.ron"))?;

        Ok(Self {
            data: DataJogo::default(),
            regioes_estrategicas,
            paises,
            estados,
            regioes,
            movimentos_independencia,
        })
    }
}

/// Motor dinâmico do território com índices de acesso rápido O(1),
/// integridade referencial validada e ciclo de simulação diária.
#[derive(Debug, Clone)]
pub struct MundoTerritorial {
    pub catalogo: CatalogoTerritorio,
    pub data: DataJogo,
    pub regioes_estrategicas: BTreeMap<String, RegiaoEstrategica>,
    pub paises: BTreeMap<String, PaisTerritorial>,
    pub estados: BTreeMap<String, Estado>,
    pub regioes: BTreeMap<String, Regiao>,

    /// Índices de hierarquia espacial O(1)
    pub(crate) regioes_por_estado: BTreeMap<String, Vec<String>>,
    pub(crate) estados_por_pais: BTreeMap<String, Vec<String>>,
    pub(crate) estados_por_regiao_estrategica: BTreeMap<String, Vec<String>>,

    pub movimentos_independencia: Vec<MovimentoIndependencia>,
}

impl MundoTerritorial {
    /// Instancia o mundo territorial, valida a hierarquia de 3 níveis e inicializa os equilíbrios.
    pub fn novo(
        catalogo: CatalogoTerritorio,
        cenario: CenarioTerritorio,
    ) -> ResultadoTerritorio<Self> {
        let mut mundo = Self {
            catalogo,
            data: cenario.data,
            regioes_estrategicas: BTreeMap::new(),
            paises: BTreeMap::new(),
            estados: BTreeMap::new(),
            regioes: BTreeMap::new(),
            regioes_por_estado: BTreeMap::new(),
            estados_por_pais: BTreeMap::new(),
            estados_por_regiao_estrategica: BTreeMap::new(),
            movimentos_independencia: cenario.movimentos_independencia,
        };

        // 1. Registra Regiões Estratégicas
        for re in cenario.regioes_estrategicas {
            validar_id_snake_case("RegiaoEstrategica", &re.id)?;
            if mundo
                .regioes_estrategicas
                .insert(re.id.clone(), re.clone())
                .is_some()
            {
                return Err(ErroTerritorio::IdentificadorDuplicado {
                    entidade: "RegiaoEstrategica",
                    id: re.id,
                });
            }
        }

        // 2. Registra Países
        for p in cenario.paises {
            crate::erro::validar_tag_pais(&p.tag)?;
            if mundo.paises.insert(p.tag.clone(), p.clone()).is_some() {
                return Err(ErroTerritorio::IdentificadorDuplicado {
                    entidade: "PaisTerritorial",
                    id: p.tag,
                });
            }
        }

        // 3. Registra Estados
        for e in cenario.estados {
            validar_id_snake_case("Estado", &e.id)?;
            if !mundo
                .regioes_estrategicas
                .contains_key(&e.regiao_estrategica_id)
            {
                return Err(ErroTerritorio::ReferenciaInexistente {
                    origem_entidade: "Estado",
                    origem_id: e.id.clone(),
                    alvo_entidade: "RegiaoEstrategica",
                    alvo_id: e.regiao_estrategica_id.clone(),
                });
            }
            if !mundo.paises.contains_key(&e.dono_tag) {
                return Err(ErroTerritorio::ReferenciaInexistente {
                    origem_entidade: "Estado",
                    origem_id: e.id.clone(),
                    alvo_entidade: "PaisTerritorial (dono)",
                    alvo_id: e.dono_tag.clone(),
                });
            }
            if mundo.estados.insert(e.id.clone(), e.clone()).is_some() {
                return Err(ErroTerritorio::IdentificadorDuplicado {
                    entidade: "Estado",
                    id: e.id,
                });
            }
        }

        // 4. Registra Regiões e valida a hierarquia completa
        for mut r in cenario.regioes {
            validar_adm1_code(&r.adm1_code)?;
            if !mundo.estados.contains_key(&r.estado_id) {
                return Err(ErroTerritorio::ReferenciaInexistente {
                    origem_entidade: "Regiao",
                    origem_id: r.adm1_code.clone(),
                    alvo_entidade: "Estado",
                    alvo_id: r.estado_id.clone(),
                });
            }
            r.recalcular_populacao();
            let adm1 = r.adm1_code.clone();
            let est_id = r.estado_id.clone();
            if mundo.regioes.insert(adm1.clone(), r).is_some() {
                return Err(ErroTerritorio::IdentificadorDuplicado {
                    entidade: "Regiao",
                    id: adm1,
                });
            }
            mundo
                .regioes_por_estado
                .entry(est_id)
                .or_default()
                .push(adm1);
        }

        // 5. Constrói índices reversos e sincroniza listas
        for (est_id, est) in &mundo.estados {
            mundo
                .estados_por_pais
                .entry(est.dono_tag.clone())
                .or_default()
                .push(est_id.clone());
            mundo
                .estados_por_regiao_estrategica
                .entry(est.regiao_estrategica_id.clone())
                .or_default()
                .push(est_id.clone());
        }

        // 6. Inicializa equilíbrio urbano e infraestrutura em todos os estados
        mundo.inicializar_todos_estados();

        Ok(mundo)
    }

    /// Cria um mundo de exemplo padrão com dados da Guerra Fria (1945).
    pub fn padrao_guerra_fria() -> Self {
        let catalogo = CatalogoTerritorio::padrao_guerra_fria();
        let cenario = CenarioTerritorio {
            data: DataJogo::nova(1945, 1, 1).unwrap(),
            regioes_estrategicas: vec![
                RegiaoEstrategica::nova(
                    "europa_central",
                    "Europa Central",
                    crate::tipos::Continente::Europa,
                    false,
                ),
                RegiaoEstrategica::nova(
                    "sudeste_brasileiro",
                    "Sudeste Brasileiro",
                    crate::tipos::Continente::AmericaSul,
                    true,
                ),
                RegiaoEstrategica::nova(
                    "costa_oeste_eua",
                    "Costa Oeste dos Estados Unidos",
                    crate::tipos::Continente::AmericaNorte,
                    true,
                ),
                RegiaoEstrategica::nova(
                    "subcontinente_indiano",
                    "Subcontinente Indiano",
                    crate::tipos::Continente::Asia,
                    true,
                ),
                RegiaoEstrategica::nova(
                    "norte_da_africa",
                    "Norte da África e Magrebe",
                    crate::tipos::Continente::Africa,
                    true,
                ),
            ],
            paises: vec![
                PaisTerritorial::novo(
                    "USA",
                    "Estados Unidos da América",
                    "california",
                    vec!["anglo_americano".into(), "afro_americano".into()],
                    "protestante",
                ),
                PaisTerritorial::novo(
                    "SOV",
                    "União Soviética",
                    "oblast_moscou",
                    vec!["russa".into(), "ucraniana".into()],
                    "ateismo_secular",
                ),
                PaisTerritorial::novo(
                    "BRA",
                    "Brasil",
                    "estado_sao_paulo",
                    vec!["brasileira".into()],
                    "catolica",
                ),
                PaisTerritorial::novo(
                    "GBR",
                    "Reino Unido",
                    "inglaterra",
                    vec!["britanica".into()],
                    "protestante",
                ),
                PaisTerritorial::novo(
                    "FRA",
                    "França",
                    "ile_de_france",
                    vec!["francesa".into()],
                    "catolica",
                ),
                PaisTerritorial::novo(
                    "IND",
                    "República da Índia",
                    "delhi",
                    vec!["hindustani".into(), "punjabi".into()],
                    "hindu",
                ),
                PaisTerritorial::novo(
                    "PAK",
                    "Paquistão",
                    "punjab_ocidental",
                    vec!["punjabi".into()],
                    "islamica_sunita",
                ),
                PaisTerritorial::novo(
                    "ALG",
                    "Argélia",
                    "argel",
                    vec!["arabe_magrebina".into(), "berbere".into()],
                    "islamica_sunita",
                ),
            ],
            estados: vec![
                Estado::novo(
                    "estado_sao_paulo",
                    "São Paulo",
                    "sudeste_brasileiro",
                    "BRA",
                    true,
                    3,
                ),
                Estado::novo(
                    "california",
                    "Califórnia",
                    "costa_oeste_eua",
                    "USA",
                    true,
                    5,
                ),
                Estado::novo(
                    "bengala",
                    "Bengala e Calcutá",
                    "subcontinente_indiano",
                    "GBR",
                    true,
                    2,
                ),
                Estado::novo(
                    "argelia_costeira",
                    "Argélia Departamental",
                    "norte_da_africa",
                    "FRA",
                    true,
                    2,
                ),
            ],
            regioes: vec![
                Regiao {
                    adm1_code: "BRA-1311".into(),
                    nome: "São Paulo".into(),
                    nome_pt: "São Paulo".into(),
                    estado_id: "estado_sao_paulo".into(),
                    pais_tag: "BRA".into(),
                    controlador_tag: "BRA".into(),
                    populacao: 7200000,
                    taxa_urbanizacao: 0.44,
                    terreno: crate::tipos::TipoTerreno::Colinas,
                    eh_costeira: true,
                    destruicao_guerra: 0.0,
                    pops: vec![
                        PopRegional::nova(
                            "brasileira",
                            "catolica",
                            SetorEmpregoPop::Industrial,
                            2500000,
                        ),
                        PopRegional::nova(
                            "brasileira",
                            "catolica",
                            SetorEmpregoPop::Servicos,
                            1800000,
                        ),
                        PopRegional::nova(
                            "brasileira",
                            "catolica",
                            SetorEmpregoPop::Rural,
                            2900000,
                        ),
                    ],
                    recursos: BTreeMap::from([("terra_aravel".into(), 100), ("pesca".into(), 40)]),
                    predios: BTreeMap::from([
                        ("fazenda_graos".into(), 40),
                        ("industria_alimentos".into(), 15),
                        ("industria_bens_consumo".into(), 20),
                        ("porto_comercial".into(), 8),
                        ("usina_eletrica".into(), 10),
                        ("centro_urbano".into(), 12),
                    ]),
                },
                Regiao {
                    adm1_code: "USA-3521".into(),
                    nome: "California".into(),
                    nome_pt: "Califórnia".into(),
                    estado_id: "california".into(),
                    pais_tag: "USA".into(),
                    controlador_tag: "USA".into(),
                    populacao: 8654432,
                    taxa_urbanizacao: 0.85,
                    terreno: crate::tipos::TipoTerreno::Planicie,
                    eh_costeira: true,
                    destruicao_guerra: 0.0,
                    pops: vec![
                        PopRegional::nova(
                            "anglo_americano",
                            "protestante",
                            SetorEmpregoPop::Industrial,
                            3200000,
                        ),
                        PopRegional::nova(
                            "anglo_americano",
                            "protestante",
                            SetorEmpregoPop::Servicos,
                            4000000,
                        ),
                        PopRegional::nova(
                            "afro_americano",
                            "protestante",
                            SetorEmpregoPop::Industrial,
                            600000,
                        ),
                        PopRegional::nova(
                            "anglo_americano",
                            "protestante",
                            SetorEmpregoPop::Rural,
                            854432,
                        ),
                    ],
                    recursos: BTreeMap::from([
                        ("petroleo".into(), 110),
                        ("terra_aravel".into(), 120),
                    ]),
                    predios: BTreeMap::from([
                        ("campo_petrolifero".into(), 30),
                        ("industria_pesada".into(), 25),
                        ("industria_bens_consumo".into(), 35),
                        ("porto_comercial".into(), 15),
                        ("usina_eletrica".into(), 20),
                        ("centro_urbano".into(), 25),
                    ]),
                },
                Regiao {
                    adm1_code: "IND-2101".into(),
                    nome: "Bengal".into(),
                    nome_pt: "Bengala".into(),
                    estado_id: "bengala".into(),
                    pais_tag: "GBR".into(),
                    controlador_tag: "GBR".into(),
                    populacao: 60000000,
                    taxa_urbanizacao: 0.12,
                    terreno: crate::tipos::TipoTerreno::Planicie,
                    eh_costeira: true,
                    destruicao_guerra: 0.05,
                    pops: vec![
                        PopRegional::nova("bengali", "hindu", SetorEmpregoPop::Rural, 26000000),
                        PopRegional::nova(
                            "bengali",
                            "islamica_sunita",
                            SetorEmpregoPop::Rural,
                            28000000,
                        ),
                        PopRegional::nova("bengali", "hindu", SetorEmpregoPop::Industrial, 3000000),
                        PopRegional::nova(
                            "bengali",
                            "islamica_sunita",
                            SetorEmpregoPop::Servicos,
                            3000000,
                        ),
                    ],
                    recursos: BTreeMap::from([("terra_aravel".into(), 300), ("pesca".into(), 120)]),
                    predios: BTreeMap::from([
                        ("fazenda_graos".into(), 180),
                        ("industria_alimentos".into(), 20),
                        ("porto_comercial".into(), 8),
                    ]),
                },
                Regiao {
                    adm1_code: "DZA-1001".into(),
                    nome: "Alger".into(),
                    nome_pt: "Argel".into(),
                    estado_id: "argelia_costeira".into(),
                    pais_tag: "FRA".into(),
                    controlador_tag: "FRA".into(),
                    populacao: 3500000,
                    taxa_urbanizacao: 0.28,
                    terreno: crate::tipos::TipoTerreno::Colinas,
                    eh_costeira: true,
                    destruicao_guerra: 0.0,
                    pops: vec![
                        PopRegional::nova(
                            "arabe_magrebina",
                            "islamica_sunita",
                            SetorEmpregoPop::Rural,
                            2000000,
                        ),
                        PopRegional::nova(
                            "arabe_magrebina",
                            "islamica_sunita",
                            SetorEmpregoPop::Industrial,
                            500000,
                        ),
                        PopRegional::nova(
                            "berbere",
                            "islamica_sunita",
                            SetorEmpregoPop::Rural,
                            600000,
                        ),
                        PopRegional::nova(
                            "francesa",
                            "catolica",
                            SetorEmpregoPop::Servicos,
                            400000,
                        ),
                    ],
                    recursos: BTreeMap::from([
                        ("petroleo".into(), 60),
                        ("terra_aravel".into(), 80),
                    ]),
                    predios: BTreeMap::from([
                        ("fazenda_graos".into(), 30),
                        ("campo_petrolifero".into(), 8),
                        ("porto_comercial".into(), 5),
                    ]),
                },
            ],
            movimentos_independencia: vec![MovimentoIndependencia {
                id: "fln_argelia".into(),
                pais_pretendente_tag: "ALG".into(),
                nome_pais_pretendente: "Argélia Democrática e Popular".into(),
                metropole_tag: "FRA".into(),
                estados_alvo: vec!["argelia_costeira".into()],
                fervor_independencia: 35.0,
                apoio_externo_tag: Some("SOV".into()),
                culturas_primarias_pretendente: vec!["arabe_magrebina".into(), "berbere".into()],
                lei_cidadania_inicial: crate::tipos::LeiCidadania::CidadaniaCivicaJusSoli,
                ativo: true,
                em_guerra: false,
            }],
        };

        Self::novo(catalogo, cenario).unwrap()
    }

    /// Inicializa imediatamente todos os estados no ponto de equilíbrio estrutural.
    fn inicializar_todos_estados(&mut self) {
        let estado_ids: Vec<String> = self.estados.keys().cloned().collect();

        for est_id in estado_ids {
            let mut predios_totais: BTreeMap<String, u32> = BTreeMap::new();
            let mut pop_total: u64 = 0;
            let mut eh_costeiro = false;
            let mut terreno = crate::tipos::TipoTerreno::Planicie;

            if let Some(reg_codigos) = self.regioes_por_estado.get(&est_id) {
                for reg_cod in reg_codigos {
                    if let Some(r) = self.regioes.get(reg_cod) {
                        pop_total += r.populacao;
                        if r.eh_costeira {
                            eh_costeiro = true;
                        }
                        terreno = r.terreno;
                        for (p_id, &niv) in &r.predios {
                            *predios_totais.entry(p_id.clone()).or_insert(0) += niv;
                        }
                    }
                }
            }

            if let Some(estado) = self.estados.get_mut(&est_id) {
                estado.rede_infraestrutura.eh_costeiro = eh_costeiro;

                // Calcula bônus de infraestrutura de prédios
                let mut bonus_geracao = 0.0;
                let mut bonus_portos = 0.0;
                for (p_id, &niv) in &predios_totais {
                    if let Some(def) = self.catalogo.buscar_predio(p_id) {
                        bonus_geracao += (niv as f64) * def.capacidade_geracao_eletrica;
                        bonus_portos += (niv as f64) * def.capacidade_portuaria_gerada;
                    }
                }

                estado.rede_infraestrutura.recalcular_capacidades(
                    terreno,
                    EraTecnologicaVisual::Era1945,
                    bonus_geracao,
                    bonus_portos,
                );

                estado
                    .desenvolvimento_urbano
                    .inicializar_equilibrio_imediato(
                        &self.catalogo,
                        &predios_totais,
                        pop_total,
                        &estado.rede_infraestrutura,
                    );

                // Se o estado não tiver cidades cadastradas, gera automaticamente a capital estadual
                if estado.cidades.is_empty() {
                    let reg_primaria = self
                        .regioes_por_estado
                        .get(&est_id)
                        .and_then(|v| v.first().cloned())
                        .unwrap_or_else(|| "REG-0001".into());
                    estado.cidades.push(crate::urbanizacao::Cidade::nova(
                        format!("{}_capital", estado.id),
                        format!("Capital de {}", estado.nome),
                        reg_primaria,
                        true,
                    ));
                }

                let era = estado
                    .era_visual_override
                    .unwrap_or(EraTecnologicaVisual::Era1945);
                estado.desenvolvimento_urbano.alocar_cidades(
                    &mut estado.cidades,
                    era,
                    &estado.rede_infraestrutura,
                );
            }
        }
    }

    /// Retorna uma região pelo código adm1_code.
    #[inline]
    pub fn obter_regiao(&self, adm1_code: &str) -> Option<&Regiao> {
        self.regioes.get(adm1_code)
    }

    /// Retorna uma região mutável pelo código adm1_code.
    #[inline]
    pub fn obter_regiao_mut(&mut self, adm1_code: &str) -> Option<&mut Regiao> {
        self.regioes.get_mut(adm1_code)
    }

    /// Retorna um estado pelo id snake_case.
    #[inline]
    pub fn obter_estado(&self, estado_id: &str) -> Option<&Estado> {
        self.estados.get(estado_id)
    }

    /// Retorna um estado mutável pelo id snake_case.
    #[inline]
    pub fn obter_estado_mut(&mut self, estado_id: &str) -> Option<&mut Estado> {
        self.estados.get_mut(estado_id)
    }

    /// Retorna um país pela tag de 3 letras.
    #[inline]
    pub fn obter_pais(&self, tag: &str) -> Option<&PaisTerritorial> {
        self.paises.get(tag)
    }

    /// Retorna um país mutável pela tag de 3 letras.
    #[inline]
    pub fn obter_pais_mut(&mut self, tag: &str) -> Option<&mut PaisTerritorial> {
        self.paises.get_mut(tag)
    }

    /// Retorna a lista de regiões pertencentes a um determinado estado.
    pub fn regioes_do_estado(&self, estado_id: &str) -> Vec<&Regiao> {
        if let Some(codigos) = self.regioes_por_estado.get(estado_id) {
            codigos.iter().filter_map(|c| self.regioes.get(c)).collect()
        } else {
            Vec::new()
        }
    }

    /// Retorna a lista de estados pertencentes a um determinado país.
    pub fn estados_do_pais(&self, tag: &str) -> Vec<&Estado> {
        if let Some(ids) = self.estados_por_pais.get(tag) {
            ids.iter().filter_map(|id| self.estados.get(id)).collect()
        } else {
            Vec::new()
        }
    }

    /// Constrói ou amplia o nível de um prédio em uma região específica.
    pub fn construir_predio(
        &mut self,
        adm1_code: &str,
        predio_id: &str,
        delta_nivel: u32,
    ) -> ResultadoTerritorio<()> {
        let estado_id = {
            let regiao = self.regioes.get_mut(adm1_code).ok_or_else(|| {
                ErroTerritorio::ReferenciaInexistente {
                    origem_entidade: "OperacaoConstruir",
                    origem_id: adm1_code.to_string(),
                    alvo_entidade: "Regiao",
                    alvo_id: adm1_code.to_string(),
                }
            })?;
            *regiao.predios.entry(predio_id.to_string()).or_insert(0) += delta_nivel;
            regiao.estado_id.clone()
        };

        // Recalcula imediatamente a demanda de infraestrutura e metas urbanas do estado
        self.atualizar_estado_apos_construcao(&estado_id);
        Ok(())
    }

    /// Desbloqueia uma tecnologia em um país e atualiza a sua era visual nacional.
    pub fn desbloquear_tecnologia(
        &mut self,
        tag: &str,
        tecnologia_id: &str,
    ) -> ResultadoTerritorio<()> {
        let pais =
            self.paises
                .get_mut(tag)
                .ok_or_else(|| ErroTerritorio::ReferenciaInexistente {
                    origem_entidade: "DesbloquearTecnologia",
                    origem_id: tag.to_string(),
                    alvo_entidade: "PaisTerritorial",
                    alvo_id: tag.to_string(),
                })?;

        pais.tecnologias_desbloqueadas
            .insert(tecnologia_id.to_string());
        Ok(())
    }

    /// Executa a transferência formal de soberania de um estado.
    pub fn transferir_estado_operacao(
        &mut self,
        estado_id: &str,
        novo_dono_tag: &str,
        modo: ModoTransferenciaTerritorial,
    ) -> ResultadoTerritorio<RelatorioTransferencia> {
        let antigo_dono_tag = {
            let estado = self.estados.get(estado_id).ok_or_else(|| {
                ErroTerritorio::ReferenciaInexistente {
                    origem_entidade: "TransferirEstado",
                    origem_id: estado_id.to_string(),
                    alvo_entidade: "Estado",
                    alvo_id: estado_id.to_string(),
                }
            })?;
            estado.dono_tag.clone()
        };

        let novo_dono = self.paises.get(novo_dono_tag).cloned().ok_or_else(|| {
            ErroTerritorio::ReferenciaInexistente {
                origem_entidade: "TransferirEstado",
                origem_id: estado_id.to_string(),
                alvo_entidade: "PaisTerritorial",
                alvo_id: novo_dono_tag.to_string(),
            }
        })?;

        let antigo_dono = self.paises.get(&antigo_dono_tag).cloned().unwrap();

        let codigos_regioes = self
            .regioes_por_estado
            .get(estado_id)
            .cloned()
            .unwrap_or_default();

        // Retira temporariamente as regiões do mapa para obter empréstimos mutáveis disjuntos.
        let mut regioes_tmp: Vec<Regiao> = codigos_regioes
            .iter()
            .filter_map(|c| self.regioes.remove(c))
            .collect();
        let mut regioes_ptrs: Vec<&mut Regiao> = regioes_tmp.iter_mut().collect();

        let estado = self.estados.get_mut(estado_id).unwrap();
        let rel = transferir_estado(
            estado,
            &mut regioes_ptrs,
            &novo_dono,
            &antigo_dono,
            &self.catalogo,
            modo,
            None,
        );
        drop(regioes_ptrs);
        for r in regioes_tmp {
            self.regioes.insert(r.adm1_code.clone(), r);
        }

        // Atualiza índice de estados por país
        if let Some(lista) = self.estados_por_pais.get_mut(&antigo_dono_tag) {
            lista.retain(|id| id != estado_id);
        }
        self.estados_por_pais
            .entry(novo_dono_tag.to_string())
            .or_default()
            .push(estado_id.to_string());

        Ok(rel)
    }

    /// Concede a independência negociada a um movimento anticolonial registrado.
    pub fn conceder_independencia_operacao(
        &mut self,
        movimento_id: &str,
    ) -> ResultadoTerritorio<RelatorioDescolonizacao> {
        let mov_idx = self
            .movimentos_independencia
            .iter()
            .position(|m| m.id == movimento_id)
            .ok_or_else(|| ErroTerritorio::ReferenciaInexistente {
                origem_entidade: "ConcederIndependencia",
                origem_id: movimento_id.to_string(),
                alvo_entidade: "MovimentoIndependencia",
                alvo_id: movimento_id.to_string(),
            })?;

        let metropole_tag = self.movimentos_independencia[mov_idx].metropole_tag.clone();
        let pret_tag = self.movimentos_independencia[mov_idx]
            .pais_pretendente_tag
            .clone();

        let metropole = self.paises.get(&metropole_tag).cloned().unwrap();
        let mut novo_pais = self
            .paises
            .get(&pret_tag)
            .cloned()
            .unwrap_or_else(|| PaisTerritorial::novo(&pret_tag, &pret_tag, "", vec![], ""));

        let mut regioes_agrupadas: BTreeMap<String, Vec<Regiao>> = BTreeMap::new();
        for est_id in &self.movimentos_independencia[mov_idx].estados_alvo {
            if let Some(codigos) = self.regioes_por_estado.get(est_id) {
                let mut lista = Vec::new();
                for c in codigos {
                    if let Some(r) = self.regioes.remove(c) {
                        lista.push(r);
                    }
                }
                regioes_agrupadas.insert(est_id.clone(), lista);
            }
        }

        let rel = conceder_independencia_negociada(
            &mut self.movimentos_independencia[mov_idx],
            &metropole,
            &mut novo_pais,
            &mut self.estados,
            &mut regioes_agrupadas,
            &self.catalogo,
        );

        // Devolve as regiões ao mapa principal
        for (_est_id, lista) in regioes_agrupadas {
            for r in lista {
                self.regioes.insert(r.adm1_code.clone(), r);
            }
        }

        // Sincroniza o índice de estados por país com os estados emancipados
        for est_id in &rel.estados_emancipados {
            if let Some(lista) = self.estados_por_pais.get_mut(&metropole_tag) {
                lista.retain(|id| id != est_id);
            }
            self.estados_por_pais
                .entry(novo_pais.tag.clone())
                .or_default()
                .push(est_id.clone());
        }

        self.paises.insert(novo_pais.tag.clone(), novo_pais);
        Ok(rel)
    }

    /// Avança exatamente 1 dia no ciclo de simulação do mundo.
    pub fn avancar_dia(&mut self) {
        self.data.avancar_dia();

        // 1. Atualiza cada estado (agregações, uso de infraestrutura, urbanização e cidades)
        let estado_ids: Vec<String> = self.estados.keys().cloned().collect();

        for est_id in &estado_ids {
            let mut predios_totais: BTreeMap<String, u32> = BTreeMap::new();
            let mut pop_total: u64 = 0;
            let mut eh_costeiro = false;
            let mut terreno = crate::tipos::TipoTerreno::Planicie;

            if let Some(reg_codigos) = self.regioes_por_estado.get(est_id) {
                for reg_cod in reg_codigos {
                    if let Some(r) = self.regioes.get(reg_cod) {
                        pop_total += r.populacao;
                        if r.eh_costeira {
                            eh_costeiro = true;
                        }
                        terreno = r.terreno;
                        for (p_id, &niv) in &r.predios {
                            *predios_totais.entry(p_id.clone()).or_insert(0) += niv;
                        }
                    }
                }
            }

            if let Some(estado) = self.estados.get_mut(est_id) {
                estado.rede_infraestrutura.eh_costeiro = eh_costeiro;

                // Capacidades e bônus de infraestrutura gerados pelos prédios instalados
                let mut bonus_geracao = 0.0;
                let mut bonus_portos = 0.0;
                for (p_id, &niv) in &predios_totais {
                    if let Some(def) = self.catalogo.buscar_predio(p_id) {
                        bonus_geracao += (niv as f64) * def.capacidade_geracao_eletrica;
                        bonus_portos += (niv as f64) * def.capacidade_portuaria_gerada;
                    }
                }

                // Era efetiva para recalcular infraestrutura
                let era_atual = estado
                    .era_visual_override
                    .unwrap_or(EraTecnologicaVisual::Era1945);

                estado.rede_infraestrutura.recalcular_capacidades(
                    terreno,
                    era_atual,
                    bonus_geracao,
                    bonus_portos,
                );

                // Zera e recalcula a demanda de uso dos 5 modais de infraestrutura
                estado.rede_infraestrutura.zerar_uso();

                // Demanda gerada pelos prédios
                for (p_id, &niv) in &predios_totais {
                    let niv_f = niv as f64;
                    if let Some(def) = self.catalogo.buscar_predio(p_id) {
                        estado.rede_infraestrutura.adicionar_uso(
                            TipoInfraestrutura::Estradas,
                            niv_f * def.demanda_estradas_por_nivel,
                        );
                        estado.rede_infraestrutura.adicionar_uso(
                            TipoInfraestrutura::Ferrovias,
                            niv_f * def.demanda_ferrovias_por_nivel,
                        );
                        estado.rede_infraestrutura.adicionar_uso(
                            TipoInfraestrutura::Portos,
                            niv_f * def.demanda_portos_por_nivel,
                        );
                        estado.rede_infraestrutura.adicionar_uso(
                            TipoInfraestrutura::Aeroportos,
                            niv_f * def.demanda_aeroportos_por_nivel,
                        );
                        estado.rede_infraestrutura.adicionar_uso(
                            TipoInfraestrutura::RedeEletrica,
                            niv_f * def.demanda_eletricidade_por_nivel,
                        );
                    }
                }

                // Demanda urbana e demográfica base
                let pop_f = pop_total as f64;
                estado
                    .rede_infraestrutura
                    .adicionar_uso(TipoInfraestrutura::Estradas, pop_f / 80_000.0);
                estado
                    .rede_infraestrutura
                    .adicionar_uso(TipoInfraestrutura::RedeEletrica, pop_f / 50_000.0);

                // Avança tick diário do desenvolvimento urbano com transições suaves
                estado.desenvolvimento_urbano.avancar_tick_diario(
                    &self.catalogo,
                    &predios_totais,
                    pop_total,
                    &estado.rede_infraestrutura,
                );

                // Reparação gradual de danos em tempo de paz
                estado.rede_infraestrutura.reparar_geral(0.002);
            }
        }

        // 2. Atualiza a era visual nacional de cada país
        let pais_tags: Vec<String> = self.paises.keys().cloned().collect();
        for tag in pais_tags {
            let mut metodos_pais: BTreeMap<String, u32> = BTreeMap::new();
            if let Some(estados_ids) = self.estados_por_pais.get(&tag) {
                for eid in estados_ids {
                    if let Some(est) = self.estados.get(eid) {
                        for metodos in est.metodos_ativos.values() {
                            for m in metodos {
                                *metodos_pais.entry(m.clone()).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }

            if let Some(pais) = self.paises.get_mut(&tag) {
                let (era_vis, ind_mod, era_teto) = calcular_era_visual_pais(
                    &self.catalogo,
                    &pais.tecnologias_desbloqueadas,
                    &metodos_pais,
                );
                pais.era_visual = era_vis;
                pais.indice_modernidade = ind_mod;
                pais.era_teto_tecnologico = era_teto;
            }
        }

        // 3. Atualiza era visual de cada estado e aloca o crescimento das cidades
        for est_id in &estado_ids {
            let (dono_tag, eh_capital) = {
                let estado = self.estados.get(est_id).unwrap();
                let eh_cap = if let Some(p) = self.paises.get(&estado.dono_tag) {
                    p.capital_estado_id == estado.id
                } else {
                    false
                };
                (estado.dono_tag.clone(), eh_cap)
            };

            let (ind_mod_pais, era_teto_pais) = if let Some(p) = self.paises.get(&dono_tag) {
                (p.indice_modernidade, p.era_teto_tecnologico)
            } else {
                (0.0, EraTecnologicaVisual::Era1945)
            };

            if let Some(estado) = self.estados.get_mut(est_id) {
                let mut metodos_estado: BTreeMap<String, u32> = BTreeMap::new();
                for metodos in estado.metodos_ativos.values() {
                    for m in metodos {
                        *metodos_estado.entry(m.clone()).or_insert(0) += 1;
                    }
                }

                let era_calculada = if let Some(over) = estado.era_visual_override {
                    over
                } else {
                    let (era_est, _ind_est) = calcular_era_visual_estado(
                        &self.catalogo,
                        &metodos_estado,
                        &estado.rede_infraestrutura,
                        ind_mod_pais,
                        era_teto_pais,
                        eh_capital,
                    );
                    era_est
                };

                estado.desenvolvimento_urbano.alocar_cidades(
                    &mut estado.cidades,
                    era_calculada,
                    &estado.rede_infraestrutura,
                );
            }
        }

        // 4. Atualiza o fervor anticolonial dos movimentos de independência
        for mov in self.movimentos_independencia.iter_mut() {
            if let Some(metropole) = self.paises.get(&mov.metropole_tag) {
                atualizar_fervor_diario(mov, metropole, 0.35, 3.0, 1);
            }
        }
    }

    /// Avança `n` dias sucessivos na simulação.
    pub fn avancar_dias(&mut self, dias: u32) {
        for _ in 0..dias {
            self.avancar_dia();
        }
    }

    /// Recalcula o estado após a adição de prédios.
    fn atualizar_estado_apos_construcao(&mut self, estado_id: &str) {
        let mut predios_totais: BTreeMap<String, u32> = BTreeMap::new();
        let mut pop_total = 0;

        if let Some(codigos) = self.regioes_por_estado.get(estado_id) {
            for c in codigos {
                if let Some(r) = self.regioes.get(c) {
                    pop_total += r.populacao;
                    for (pid, &niv) in &r.predios {
                        *predios_totais.entry(pid.clone()).or_insert(0) += niv;
                    }
                }
            }
        }

        if let Some(estado) = self.estados.get_mut(estado_id) {
            estado.desenvolvimento_urbano.recalcular_alvos(
                &self.catalogo,
                &predios_totais,
                pop_total,
                &estado.rede_infraestrutura,
            );
        }
    }
}
