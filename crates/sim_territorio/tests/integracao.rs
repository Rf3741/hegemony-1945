//! Testes de integração do `sim_territorio`: catálogo em RON, hierarquia, urbanização,
//! eras visuais, infraestrutura, transferência e descolonização.

use sim_territorio::*;
use std::collections::BTreeMap;
use std::path::Path;

fn catalogo_fixture() -> CatalogoTerritorio {
    CatalogoTerritorio::carregar_diretorio(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
    )
    .expect("fixtures devem carregar")
}

fn regiao(adm1: &str, estado: &str, pais: &str, pop: u64, costeira: bool) -> Regiao {
    Regiao {
        adm1_code: adm1.into(),
        nome: adm1.into(),
        nome_pt: adm1.into(),
        estado_id: estado.into(),
        pais_tag: pais.into(),
        controlador_tag: pais.into(),
        populacao: pop,
        taxa_urbanizacao: 0.1,
        terreno: TipoTerreno::Planicie,
        eh_costeira: costeira,
        destruicao_guerra: 0.0,
        pops: vec![
            PopRegional::nova(
                "brasileira",
                "catolica",
                SetorEmpregoPop::Rural,
                pop * 7 / 10,
            ),
            PopRegional::nova(
                "brasileira",
                "catolica",
                SetorEmpregoPop::Servicos,
                pop - pop * 7 / 10,
            ),
        ],
        recursos: BTreeMap::new(),
        predios: BTreeMap::new(),
    }
}

/// Mundo mínimo: um país (BRA) com um estado grande e sem prédios.
fn mundo_industrializavel() -> MundoTerritorial {
    let cenario = CenarioTerritorio {
        data: DataJogo::nova(1945, 1, 1).unwrap(),
        regioes_estrategicas: vec![RegiaoEstrategica::nova(
            "sudeste_brasileiro",
            "Sudeste Brasileiro",
            Continente::AmericaSul,
            true,
        )],
        paises: vec![PaisTerritorial::novo(
            "BRA",
            "Brasil",
            "estado_sp",
            vec!["brasileira".into()],
            "catolica",
        )],
        estados: vec![Estado::novo(
            "estado_sp",
            "São Paulo",
            "sudeste_brasileiro",
            "BRA",
            true,
            2,
        )],
        regioes: vec![regiao("BRA-1", "estado_sp", "BRA", 12_000_000, true)],
        movimentos_independencia: vec![],
    };
    MundoTerritorial::novo(catalogo_fixture(), cenario).unwrap()
}

#[test]
fn fixtures_carregam_e_indexam() {
    let c = catalogo_fixture();
    assert!(c.buscar_predio("industria_pesada").is_some());
    assert!(c.buscar_cultura("anglo_americano").is_some());
    assert_eq!(c.tecnologias_eras.len(), 8);
    assert_eq!(c.metodos_eras.len(), 8);
    assert!(c.buscar_predio("inexistente").is_none());
}

#[test]
fn mundo_padrao_tem_hierarquia_consistente() {
    let m = MundoTerritorial::padrao_guerra_fria();
    assert!(!m.estados.is_empty());
    for r in m.regioes.values() {
        assert!(m.obter_estado(&r.estado_id).is_some());
    }
    for e in m.estados.values() {
        assert!(m.obter_pais(&e.dono_tag).is_some());
        assert!(!e.cidades.is_empty());
        assert!(e.tem_nucleo(&e.dono_tag));
    }
    assert!(!m.estados_do_pais("USA").is_empty());
    assert!(!m.regioes_do_estado("california").is_empty());
}

#[test]
fn ids_invalidos_sao_rejeitados() {
    assert!(validar_tag_pais("USA").is_ok());
    assert!(validar_tag_pais("usa").is_err());
    assert!(validar_tag_pais("USAA").is_err());
    assert!(validar_id_snake_case("Estado", "sao_paulo").is_ok());
    assert!(validar_id_snake_case("Estado", "São Paulo").is_err());
}

#[test]
fn referencia_inexistente_e_duplicata_falham() {
    let mut cen = CenarioTerritorio::default();
    cen.paises
        .push(PaisTerritorial::novo("BRA", "Brasil", "x", vec![], ""));
    cen.estados
        .push(Estado::novo("e1", "E1", "sem_regiao", "BRA", false, 1));
    assert!(MundoTerritorial::novo(catalogo_fixture(), cen).is_err());

    let mut cen = CenarioTerritorio::default();
    cen.paises
        .push(PaisTerritorial::novo("BRA", "Brasil", "x", vec![], ""));
    cen.paises
        .push(PaisTerritorial::novo("BRA", "Brasil", "x", vec![], ""));
    assert!(MundoTerritorial::novo(catalogo_fixture(), cen).is_err());
}

#[test]
fn data_avanca_com_bissexto() {
    let mut d = DataJogo::nova(1948, 2, 28).unwrap();
    d.avancar_dia();
    assert_eq!((d.mes, d.dia), (2, 29));
    d.avancar_dia();
    assert_eq!((d.mes, d.dia), (3, 1));
    assert!(DataJogo::nova(1949, 2, 29).is_none());
}

#[test]
fn nivel_cidade_limites_1_a_8() {
    assert_eq!(NivelCidade::a_partir_u8(0).valor(), 1);
    assert_eq!(NivelCidade::a_partir_u8(99).valor(), 8);
    assert_eq!(NivelIndustrial::a_partir_u8(0).valor(), 0);
    assert_eq!(NivelIndustrial::a_partir_u8(99).valor(), 8);
}

#[test]
fn construir_predio_eleva_alvos_urbanos() {
    let mut m = mundo_industrializavel();
    let antes = m
        .obter_estado("estado_sp")
        .unwrap()
        .desenvolvimento_urbano
        .indice_urbano_alvo;
    m.construir_predio("BRA-1", "industria_pesada", 20).unwrap();
    m.construir_predio("BRA-1", "centro_urbano", 20).unwrap();
    let u = &m.obter_estado("estado_sp").unwrap().desenvolvimento_urbano;
    assert!(u.indice_urbano_alvo > antes);
    assert!(u.indice_industrial_alvo > 0.0);
    assert!(u.empregos_industriais_totais > 0);
    assert!(
        m.construir_predio("NAO-EXISTE", "industria_pesada", 1)
            .is_err()
    );
}

#[test]
fn urbanizacao_e_empregos_nao_excedem_forca_de_trabalho() {
    let mut m = mundo_industrializavel();
    m.construir_predio("BRA-1", "industria_pesada", 5000)
        .unwrap();
    let u = &m.obter_estado("estado_sp").unwrap().desenvolvimento_urbano;
    let forca = (12_000_000f64 * 0.48).round() as u64;
    assert!(
        u.empregos_industriais_totais + u.empregos_servicos_totais + u.empregos_rurais_totais
            <= forca + 2
    );
    assert!(u.taxa_urbanizacao_alvo <= 0.97);
    assert!(u.indice_urbano_alvo <= 100.0);
}

#[test]
fn estado_sem_predios_comeca_no_nivel_minimo() {
    let m = mundo_industrializavel();
    let u = &m.obter_estado("estado_sp").unwrap().desenvolvimento_urbano;
    assert!(u.nivel_cidade.valor() <= 3);
    assert_eq!(u.nivel_industrial.valor(), 0);
}

#[test]
fn simula_30_anos_de_industrializacao_com_nivel_suave() {
    let mut m = mundo_industrializavel();
    let sequencia = [
        "fazenda_graos",
        "industria_alimentos",
        "industria_bens_consumo",
        "centro_urbano",
        "usina_eletrica",
        "industria_pesada",
        "porto_comercial",
        "usina_siderurgica",
        "industria_eletronica",
    ];
    let inicial = m
        .obter_estado("estado_sp")
        .unwrap()
        .desenvolvimento_urbano
        .nivel_cidade
        .valor();
    let ind_inicial = m
        .obter_estado("estado_sp")
        .unwrap()
        .desenvolvimento_urbano
        .nivel_industrial
        .valor();

    let mut ultimo = inicial;
    let mut ultimo_ind = ind_inicial;
    let mut trocas = 0u32;
    let mut dias_desde_troca = u32::MAX;
    let mut ult_indice = m
        .obter_estado("estado_sp")
        .unwrap()
        .desenvolvimento_urbano
        .indice_urbano_suavizado;

    for ano in 0..30u32 {
        // Cada trimestre constrói um pouco de cada prédio da sequência liberada pelo ano.
        for trimestre in 0..4u32 {
            let liberados = (((ano * 4 + trimestre) / 4) as usize + 2).min(sequencia.len());
            for p in &sequencia[..liberados] {
                m.construir_predio("BRA-1", p, 2).unwrap();
            }
            for _ in 0..91 {
                m.avancar_dia();
                let u = &m.obter_estado("estado_sp").unwrap().desenvolvimento_urbano;
                // O índice contínuo nunca salta de forma abrupta.
                assert!(
                    (u.indice_urbano_suavizado - ult_indice).abs() < 0.5,
                    "salto de índice"
                );
                ult_indice = u.indice_urbano_suavizado;
                let n = u.nivel_cidade.valor();
                assert!(n.abs_diff(ultimo) <= 1, "nível pulou de {ultimo} para {n}");
                assert!(u.nivel_industrial.valor().abs_diff(ultimo_ind) <= 1);
                if n != ultimo {
                    assert!(
                        dias_desde_troca >= 30,
                        "troca de nível rápida demais ({dias_desde_troca} dias)"
                    );
                    trocas += 1;
                    dias_desde_troca = 0;
                    assert!(n > ultimo, "nível não deve cair durante o crescimento");
                }
                dias_desde_troca = dias_desde_troca.saturating_add(1);
                ultimo = n;
                ultimo_ind = u.nivel_industrial.valor();
            }
        }
    }
    let u = &m.obter_estado("estado_sp").unwrap().desenvolvimento_urbano;
    assert!(
        u.nivel_cidade.valor() > inicial,
        "cidade deve crescer em 30 anos"
    );
    assert!(u.nivel_industrial.valor() > ind_inicial);
    assert!(trocas >= 2, "esperava ao menos 2 degraus, houve {trocas}");
    assert!(m.data.ano >= 1974, "ano {}", m.data.ano);
}

#[test]
fn cidades_refletem_populacao_e_era() {
    let mut m = mundo_industrializavel();
    m.construir_predio("BRA-1", "centro_urbano", 40).unwrap();
    m.avancar_dias(30);
    let e = m.obter_estado("estado_sp").unwrap();
    let soma: u64 = e.cidades.iter().map(|c| c.populacao_urbana).sum();
    let alvo = e.desenvolvimento_urbano.populacao_urbana_total;
    assert!(soma.abs_diff(alvo) <= e.cidades.len() as u64);
    assert!(e.cidades.iter().all(|c| c.tamanho_visual >= 1.0));
}

#[test]
fn era_do_pais_sobe_com_tecnologias_e_respeita_teto() {
    let c = catalogo_fixture();
    let vazio = Default::default();
    let (era0, mod0, _) = calcular_era_visual_pais(&c, &vazio, &BTreeMap::new());
    assert_eq!(era0, EraTecnologicaVisual::Era1945);

    let mut techs = std::collections::BTreeSet::new();
    techs.insert("reconstrucao_e_vapor".to_string());
    techs.insert("linha_montagem_fordista".to_string());
    techs.insert("arquitetura_modernista_concreto".to_string());
    let (era1, mod1, teto1) = calcular_era_visual_pais(&c, &techs, &BTreeMap::new());
    assert_eq!(teto1, EraTecnologicaVisual::Era1955);
    assert!(mod1 > mod0);
    assert!(era1 <= teto1);

    // Métodos avançados sem tecnologia não ultrapassam o teto.
    let metodos = BTreeMap::from([("automacao_robotica_cnc".to_string(), 100u32)]);
    let (era2, _, teto2) = calcular_era_visual_pais(&c, &techs, &metodos);
    assert!(era2 <= teto2);
}

#[test]
fn era_do_estado_nunca_passa_do_teto_nacional() {
    let c = catalogo_fixture();
    let infra = RedeInfraestruturaEstado::novo(true, 5);
    let metodos = BTreeMap::from([("automacao_robotica_cnc".to_string(), 50u32)]);
    let (era, _) = calcular_era_visual_estado(
        &c,
        &metodos,
        &infra,
        90.0,
        EraTecnologicaVisual::Era1955,
        true,
    );
    assert!(era <= EraTecnologicaVisual::Era1955);
}

#[test]
fn desbloquear_tecnologia_atualiza_era_nacional_no_tick() {
    let mut m = mundo_industrializavel();
    for t in [
        "reconstrucao_e_vapor",
        "linha_montagem_fordista",
        "arquitetura_modernista_concreto",
        "aviacao_comercial_jato",
        "conteinerizacao_maritima",
    ] {
        m.desbloquear_tecnologia("BRA", t).unwrap();
    }
    m.avancar_dia();
    let p = m.obter_pais("BRA").unwrap();
    assert!(p.era_teto_tecnologico >= EraTecnologicaVisual::Era1965);
    assert!(p.indice_modernidade > 0.0);
    assert!(m.desbloquear_tecnologia("XXX", "a").is_err());
}

#[test]
fn estilo_arquitetonico_segue_bloco_e_colonia() {
    assert_eq!(
        determinar_estilo_arquitetonico(Some("bloco_sovietico"), None, false),
        EstiloArquitetonico::SocialistaPlanejado
    );
    assert_eq!(
        determinar_estilo_arquitetonico(Some("bloco_ocidental"), None, false),
        EstiloArquitetonico::OcidentalCorporativo
    );
    assert_eq!(
        determinar_estilo_arquitetonico(Some("bloco_ocidental"), None, true),
        EstiloArquitetonico::ColonialTradicional
    );
}

#[test]
fn infraestrutura_uso_capacidade_e_dano() {
    let mut r = RedeInfraestruturaEstado::novo(true, 3);
    r.recalcular_capacidades(
        TipoTerreno::Planicie,
        EraTecnologicaVisual::Era1945,
        0.0,
        0.0,
    );
    let cap = r.estradas.capacidade_efetiva;
    assert!(cap > 0.0);
    r.zerar_uso();
    r.adicionar_uso(TipoInfraestrutura::Estradas, cap * 2.0);
    assert!(r.estradas.sobrecarregado());
    assert!(r.estradas.saldo() < 0.0);
    r.aplicar_dano_geral(0.5);
    r.recalcular_capacidades(
        TipoTerreno::Planicie,
        EraTecnologicaVisual::Era1945,
        0.0,
        0.0,
    );
    assert!(r.estradas.capacidade_efetiva < cap);
    r.reparar_geral(0.2);
    r.reparar_geral(0.2);
    r.recalcular_capacidades(
        TipoTerreno::Planicie,
        EraTecnologicaVisual::Era1945,
        0.0,
        0.0,
    );
    assert!(r.estradas.capacidade_efetiva > cap * 0.5);
}

#[test]
fn infraestrutura_interior_nao_tem_porto_e_era_melhora_capacidade() {
    let interior = RedeInfraestruturaEstado::novo(false, 4);
    assert_eq!(interior.portos.nivel, 0);
    let mut a = RedeInfraestruturaEstado::novo(true, 4);
    let mut b = a.clone();
    a.recalcular_capacidades(
        TipoTerreno::Planicie,
        EraTecnologicaVisual::Era1945,
        0.0,
        0.0,
    );
    b.recalcular_capacidades(
        TipoTerreno::Planicie,
        EraTecnologicaVisual::Era1985,
        0.0,
        0.0,
    );
    assert!(b.estradas.capacidade_efetiva > a.estradas.capacidade_efetiva);
    assert!((0.0..=1.0).contains(&a.acesso_mercado()));
    assert!((0.0..=1.0).contains(&a.cobertura_eletrica()));
    assert!(a.capacidade_suprimento_militar() >= 0.0);
}

#[test]
fn terreno_montanhoso_reduz_capacidade() {
    let mut p = RedeInfraestruturaEstado::novo(true, 4);
    let mut mt = p.clone();
    p.recalcular_capacidades(
        TipoTerreno::Planicie,
        EraTecnologicaVisual::Era1945,
        0.0,
        0.0,
    );
    mt.recalcular_capacidades(
        TipoTerreno::Montanhas,
        EraTecnologicaVisual::Era1945,
        0.0,
        0.0,
    );
    assert!(mt.estradas.capacidade_efetiva < p.estradas.capacidade_efetiva);
}

#[test]
fn transferencia_muda_dono_e_indices() {
    let mut m = MundoTerritorial::padrao_guerra_fria();
    let antes = m.estados_do_pais("FRA").len();
    let rel = m
        .transferir_estado_operacao(
            "argelia_costeira",
            "USA",
            ModoTransferenciaTerritorial::ConquistaMilitar,
        )
        .unwrap();
    assert!(rel.populacao_final > 0);
    let e = m.obter_estado("argelia_costeira").unwrap();
    assert_eq!(e.dono_tag, "USA");
    assert_eq!(e.controlador_tag, "USA");
    assert!(!e.incorporado);
    assert_eq!(m.estados_do_pais("FRA").len(), antes - 1);
    assert!(
        m.estados_do_pais("USA")
            .iter()
            .any(|s| s.id == "argelia_costeira")
    );
    assert!(!m.regioes_do_estado("argelia_costeira").is_empty());
    assert!(
        m.transferir_estado_operacao(
            "nao_existe",
            "USA",
            ModoTransferenciaTerritorial::ConquistaMilitar
        )
        .is_err()
    );
    assert!(
        m.transferir_estado_operacao(
            "california",
            "ZZZ",
            ModoTransferenciaTerritorial::ConquistaMilitar
        )
        .is_err()
    );
}

#[test]
fn nucleos_e_reivindicacoes() {
    let mut e = Estado::novo("x", "X", "r", "BRA", false, 1);
    assert!(e.tem_nucleo("BRA"));
    e.adicionar_nucleo("ARG");
    assert!(e.tem_nucleo("ARG"));
    e.remover_nucleo("ARG");
    assert!(!e.tem_nucleo("ARG"));
    e.adicionar_reivindicacao("ARG", TipoReivindicacao::Historica, DataJogo::default());
    assert!(e.tem_reivindicacao("ARG"));
    e.remover_reivindicacao("ARG");
    assert!(!e.tem_reivindicacao("ARG"));
}

#[test]
fn independencia_negociada_cria_pais_e_move_estado() {
    let mut m = MundoTerritorial::padrao_guerra_fria();
    let rel = m.conceder_independencia_operacao("fln_argelia").unwrap();
    assert_eq!(rel.via, ViaDescolonizacao::IndependenciaNegociada);
    assert_eq!(rel.pais_emancipado_tag, "ALG");
    let e = m.obter_estado("argelia_costeira").unwrap();
    assert_eq!(e.dono_tag, "ALG");
    assert!(e.tem_nucleo("ALG"));
    assert!(
        m.estados_do_pais("ALG")
            .iter()
            .any(|s| s.id == "argelia_costeira")
    );
    assert!(
        m.estados_do_pais("FRA")
            .iter()
            .all(|s| s.id != "argelia_costeira")
    );
    assert!(!m.movimentos_independencia[0].ativo);
    // Regiões devolvidas ao mapa
    assert!(!m.regioes_do_estado("argelia_costeira").is_empty());
    assert!(m.conceder_independencia_operacao("inexistente").is_err());
}

#[test]
fn fervor_cresce_e_custos_colonial() {
    let mut m = MundoTerritorial::padrao_guerra_fria();
    let f0 = m.movimentos_independencia[0].fervor_independencia;
    m.avancar_dias(60);
    let f1 = m.movimentos_independencia[0].fervor_independencia;
    assert!(f1 >= f0 && f1 <= 100.0);
    assert!(custo_manutencao_colonial(80.0, 100.0) > custo_manutencao_colonial(10.0, 100.0));
    assert!(eficiencia_extrativa_colonial(90.0) < eficiencia_extrativa_colonial(10.0));
    assert!((0.15..=1.0).contains(&eficiencia_extrativa_colonial(100.0)));
}

#[test]
fn simulacao_multi_ano_do_mundo_padrao_e_estavel() {
    let mut m = MundoTerritorial::padrao_guerra_fria();
    m.avancar_dias(365 * 2);
    for e in m.estados.values() {
        let u = &e.desenvolvimento_urbano;
        assert!((1..=8).contains(&u.nivel_cidade.valor()));
        assert!(u.nivel_industrial.valor() <= 8);
        assert!(u.indice_urbano_suavizado.is_finite());
        assert!(u.populacao_urbana_total <= u.populacao_total);
    }
}

#[test]
fn descritor_visual_valores_normalizados() {
    let m = MundoTerritorial::padrao_guerra_fria();
    let e = m.obter_estado("california").unwrap();
    let d = e.desenvolvimento_urbano.descritor_visual(
        EraTecnologicaVisual::Era1965,
        EstiloArquitetonico::OcidentalCorporativo,
        &e.rede_infraestrutura,
    );
    for v in [
        d.densidade_urbana,
        d.densidade_fabricas,
        d.iluminacao_noturna,
        d.polucao_fumaca,
    ] {
        assert!((0.0..=1.0).contains(&v));
    }
    assert_eq!(
        d.nivel_cidade,
        e.desenvolvimento_urbano.nivel_cidade.valor()
    );
}

#[test]
fn serializacao_ron_ida_e_volta_do_cenario() {
    let c = CenarioTerritorio {
        data: DataJogo::nova(1950, 6, 25).unwrap(),
        paises: vec![PaisTerritorial::novo("BRA", "Brasil", "e", vec![], "")],
        ..Default::default()
    };
    let s = ron::to_string(&c).unwrap();
    let c2: CenarioTerritorio = ron::from_str(&s).unwrap();
    assert_eq!(c, c2);
}
