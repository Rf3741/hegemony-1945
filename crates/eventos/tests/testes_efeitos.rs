use eventos::condicoes::{Condicao, Op};
use eventos::contexto::ContextoEfeitos;
use eventos::politicos::{CondicoesEvento, EventoPolitico, OpcaoEventoPolitico};
use eventos::{
    CatalogoEventos, EstadoEventos, Evento, Modificador, OpcaoEvento, Pendente, aplicar_opcao_com_contexto,
    avaliar_dia_com_contexto, dias,
};
use sim_diplomacia::{Bloco, Diplomacia, InfoPais};
use sim_economia::{CenarioEconomia, ModoCadencia, MundoEconomico};
use sim_politica::{CenarioPolitica, MundoPolitico};
use simulacao::mundo::MundoJogo;
use simulacao::politica::CategoriaLei;
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn mundo_teste() -> MundoJogo {
    let r = raiz();
    MundoJogo::carregar_de_arquivos(
        r.join("assets/map/provinces.ron"),
        r.join("data/scenario_1945/countries_1945.ron"),
        r.join("data/scenario_1945/province_owners_1945.ron"),
    )
    .expect("mundo")
    .1
}

fn politica_teste() -> MundoPolitico {
    let r = raiz();
    let cen = CenarioPolitica::carregar_diretorio(&r.join("assets/data/politica"))
        .expect("cenario politica");
    MundoPolitico::novo(cen.catalogo, cen.paises, 1945).expect("mundo politico")
}

fn economia_teste() -> MundoEconomico {
    let r = raiz();
    let mut cen = CenarioEconomia::carregar_diretorio(&r.join("assets/data/economia"))
        .expect("cenario economia");
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    MundoEconomico::novo(cen).expect("mundo economia")
}

fn diplomacia_teste() -> Diplomacia {
    let paises = vec![
        InfoPais {
            tag: "USA".to_string(),
            bloco: Bloco::Ocidental,
        },
        InfoPais {
            tag: "SOV".to_string(),
            bloco: Bloco::Sovietico,
        },
        InfoPais {
            tag: "BRA".to_string(),
            bloco: Bloco::NaoAlinhado,
        },
    ];
    Diplomacia::novo(paises)
}

#[test]
fn teste_carregar_eventos_politicos_e_historicos() {
    let c = CatalogoEventos::carregar(&raiz().join("assets/data/eventos")).expect("eventos");
    // 3 arquivos históricos + marcos de era + 80 eventos políticos
    assert!(
        c.eventos.len() >= 380,
        "Total esperado >= 380, obtido: {}",
        c.eventos.len()
    );
    // Verifica presença de evento político sistêmico
    assert!(
        c.por_id("crise_mocao_de_desconfianca").is_some(),
        "Evento político crise_mocao_de_desconfianca deve estar presente"
    );
}

#[test]
fn teste_efeito_mudar_lei() {
    let mut m = mundo_teste();
    let mut pol = politica_teste();
    let mut eco = economia_teste();
    let mut dipl = diplomacia_teste();
    let mut est = EstadoEventos::default();

    let ev = Evento {
        id: "teste_mudar_lei".to_string(),
        titulo: "Reforma Constitucional e Econômica".to_string(),
        descricao: "Promulgação de novas leis nacionais.".to_string(),
        data: (1945, 1, 1),
        paises: vec!["USA".to_string()],
        condicoes: vec![],
        opcoes: vec![OpcaoEvento {
            id: "aprovar_reforma".to_string(),
            texto: "Aprovar reformas".to_string(),
            descricao_resultado: "Leis promulgadas.".to_string(),
            efeitos: vec![
                Modificador {
                    id: "mudar_lei:governo:sufragio_universal".to_string(),
                    valor: 1.0,
                },
                Modificador {
                    id: "lei:economia:capitalismo_de_intervencao".to_string(),
                    valor: 1.0,
                },
            ],
            ia_peso: 100,
        }],
        chance: 1.0,
        generico: true,
    };

    let mut ctx = ContextoEfeitos::com_tudo(&mut m, &mut pol, &mut eco, &mut dipl);
    aplicar_opcao_com_contexto(&mut ctx, &mut est, &ev, 0, "USA");

    // 1. Verifica no MundoJogo
    assert_eq!(
        m.paises["USA"]
            .politica
            .leis_ativas
            .get(&CategoriaLei::Governo)
            .map(|s| s.as_str()),
        Some("sufragio_universal")
    );
    assert_eq!(
        m.paises["USA"]
            .politica
            .leis_ativas
            .get(&CategoriaLei::Economia)
            .map(|s| s.as_str()),
        Some("capitalismo_de_intervencao")
    );

    // 2. Verifica no sim_politica
    let p_id_pol = pol
        .paises()
        .iter()
        .find(|p| p.tag == "USA")
        .map(|p| p.id.clone())
        .unwrap();
    assert_eq!(
        pol.pais(&p_id_pol)
            .unwrap()
            .leis_ativas
            .get("governo")
            .map(|s| s.as_str()),
        Some("sufragio_universal")
    );

    // 3. Verifica registro em est.leis_trocadas
    assert!(
        est.leis_trocadas
            .iter()
            .any(|(p, cat, lei)| p == "USA" && cat == "governo" && lei == "sufragio_universal")
    );
}

#[test]
fn teste_efeito_flags_de_pais() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    let ev_set = Evento {
        id: "teste_flag_set".to_string(),
        titulo: "Pacto de Ajuda Externa".to_string(),
        descricao: "Define flag do plano.".to_string(),
        data: (1945, 1, 1),
        paises: vec!["USA".to_string()],
        condicoes: vec![],
        opcoes: vec![OpcaoEvento {
            id: "aceitar_plano".to_string(),
            texto: "Aceitar".to_string(),
            descricao_resultado: "".to_string(),
            efeitos: vec![Modificador {
                id: "flag:plano_marshall_aceito".to_string(),
                valor: 1.0,
            }],
            ia_peso: 100,
        }],
        chance: 1.0,
        generico: true,
    };

    aplicar_opcao_com_contexto(&mut ContextoEfeitos::apenas_mundo(&mut m), &mut est, &ev_set, 0, "USA");
    assert!(est.tem_flag("USA", "plano_marshall_aceito"));
    assert!(est.avaliar(&m, "USA", &Condicao::TemFlag("plano_marshall_aceito".into())));
    assert!(!est.avaliar(&m, "USA", &Condicao::NaoTemFlag("plano_marshall_aceito".into())));

    // Remove a flag
    let ev_rem = Evento {
        id: "teste_flag_rem".to_string(),
        titulo: "Revogação".to_string(),
        descricao: "Remove flag do plano.".to_string(),
        data: (1945, 1, 1),
        paises: vec!["USA".to_string()],
        condicoes: vec![],
        opcoes: vec![OpcaoEvento {
            id: "revogar".to_string(),
            texto: "Revogar".to_string(),
            descricao_resultado: "".to_string(),
            efeitos: vec![Modificador {
                id: "remover_flag:plano_marshall_aceito".to_string(),
                valor: 1.0,
            }],
            ia_peso: 100,
        }],
        chance: 1.0,
        generico: true,
    };
    aplicar_opcao_com_contexto(&mut ContextoEfeitos::apenas_mundo(&mut m), &mut est, &ev_rem, 0, "USA");
    assert!(!est.tem_flag("USA", "plano_marshall_aceito"));
    assert!(est.avaliar(&m, "USA", &Condicao::NaoTemFlag("plano_marshall_aceito".into())));
}

#[test]
fn teste_efeito_variaveis_de_pais() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    let ev = Evento {
        id: "teste_var".to_string(),
        titulo: "Mobilização Popular".to_string(),
        descricao: "Incrementa variável de apoio.".to_string(),
        data: (1945, 1, 1),
        paises: vec!["BRA".to_string()],
        condicoes: vec![],
        opcoes: vec![OpcaoEvento {
            id: "mobilizar".to_string(),
            texto: "Mobilizar".to_string(),
            descricao_resultado: "".to_string(),
            efeitos: vec![Modificador {
                id: "var:apoio_popular_reforma".to_string(),
                valor: 35.0,
            }],
            ia_peso: 100,
        }],
        chance: 1.0,
        generico: true,
    };

    aplicar_opcao_com_contexto(&mut ContextoEfeitos::apenas_mundo(&mut m), &mut est, &ev, 0, "BRA");
    assert_eq!(est.obter_variavel("BRA", "apoio_popular_reforma"), 35.0);
    assert!(est.avaliar(
        &m,
        "BRA",
        &Condicao::Compara("apoio_popular_reforma".into(), Op::Ge, 30.0)
    ));
    assert!(!est.avaliar(
        &m,
        "BRA",
        &Condicao::Compara("apoio_popular_reforma".into(), Op::Lt, 30.0)
    ));
}

#[test]
fn teste_efeito_modificadores_temporarios_com_duracao() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    let ev = Evento {
        id: "teste_mod_temp".to_string(),
        titulo: "Plano Quinquenal de Emergência".to_string(),
        descricao: "Modificadores temporários com duração em dias.".to_string(),
        data: (1945, 1, 1),
        paises: vec!["USA".to_string()],
        condicoes: vec![],
        opcoes: vec![OpcaoEvento {
            id: "ativar_plano".to_string(),
            texto: "Ativar".to_string(),
            descricao_resultado: "".to_string(),
            efeitos: vec![
                Modificador {
                    id: "temp:produtividade_industrial:30".to_string(),
                    valor: 0.25,
                },
                Modificador {
                    id: "temp:estabilidade_politica:60".to_string(),
                    valor: 15.0,
                },
                Modificador {
                    id: "temp:radicalismo_geral:90".to_string(),
                    valor: -0.20,
                },
            ],
            ia_peso: 100,
        }],
        chance: 1.0,
        generico: true,
    };

    let mut ctx = ContextoEfeitos::apenas_mundo(&mut m);
    aplicar_opcao_com_contexto(&mut ctx, &mut est, &ev, 0, "USA");

    assert_eq!(est.modificadores_temporarios.len(), 3);
    assert!((est.modificador_total("USA", "economia") - 0.25).abs() < 1e-4);
    assert!((est.modificador_total("USA", "estabilidade") - 15.0).abs() < 1e-4);
    assert!((est.modificador_total("USA", "radicalismo") - (-0.20)).abs() < 1e-4);

    // Avança 30 dias: o primeiro modificador (30 dias) deve expirar
    est.avancar_dias(30);
    assert_eq!(est.modificadores_temporarios.len(), 2);
    assert_eq!(est.modificador_total("USA", "economia"), 0.0);
    assert!((est.modificador_total("USA", "estabilidade") - 15.0).abs() < 1e-4);

    // Avança mais 30 dias: o segundo modificador (60 dias) deve expirar
    est.avancar_dias(30);
    assert_eq!(est.modificadores_temporarios.len(), 1);
    assert_eq!(est.modificador_total("USA", "estabilidade"), 0.0);
    assert!((est.modificador_total("USA", "radicalismo") - (-0.20)).abs() < 1e-4);

    // Avança mais 30 dias: o último modificador (90 dias) deve expirar
    est.avancar_dias(30);
    assert_eq!(est.modificadores_temporarios.len(), 0);
    assert_eq!(est.modificador_total("USA", "radicalismo"), 0.0);
}

#[test]
fn teste_efeito_relacoes_diplomaticas() {
    let mut m = mundo_teste();
    let mut dipl = diplomacia_teste();
    let mut est = EstadoEventos::default();

    let rel_inicial = dipl.relacao("USA", "SOV");

    let ev = Evento {
        id: "teste_relacao".to_string(),
        titulo: "Tratado Bilateral de Não Agressão".to_string(),
        descricao: "Aproximação diplomática.".to_string(),
        data: (1945, 1, 1),
        paises: vec!["USA".to_string()],
        condicoes: vec![],
        opcoes: vec![OpcaoEvento {
            id: "assinar_tratado".to_string(),
            texto: "Assinar".to_string(),
            descricao_resultado: "".to_string(),
            efeitos: vec![Modificador {
                id: "relacao:SOV".to_string(),
                valor: 25.0,
            }],
            ia_peso: 100,
        }],
        chance: 1.0,
        generico: true,
    };

    let mut ctx = ContextoEfeitos {
        mundo: &mut m,
        politica: None,
        economia: None,
        diplomacia: Some(&mut dipl),
    };
    aplicar_opcao_com_contexto(&mut ctx, &mut est, &ev, 0, "USA");

    // Verifica no EstadoEventos
    assert_eq!(est.obter_relacao("USA", "SOV"), 25.0);

    // Verifica no motor sim_diplomacia
    let rel_nova = dipl.relacao("USA", "SOV");
    assert!((rel_nova - (rel_inicial + 25.0)).abs() < 1e-3);
}

#[test]
fn teste_efeito_ligacao_sim_politica() {
    let mut m = mundo_teste();
    let mut pol = politica_teste();
    let mut est = EstadoEventos::default();

    let p_id_pol = pol
        .paises()
        .iter()
        .find(|p| p.tag == "USA")
        .map(|p| p.id.clone())
        .unwrap();
    let aprov_inicial = pol
        .pais(&p_id_pol)
        .unwrap()
        .grupos_interesse
        .get("intelectuais")
        .map(|g| g.aprovacao)
        .unwrap_or(0.0);
    let leg_inicial = pol.pais(&p_id_pol).unwrap().legitimidade;

    let ev = Evento {
        id: "teste_politica".to_string(),
        titulo: "Congresso de Intelectuais e Cientistas".to_string(),
        descricao: "Apoio governamental à classe intelectual.".to_string(),
        data: (1945, 1, 1),
        paises: vec!["USA".to_string()],
        condicoes: vec![],
        opcoes: vec![OpcaoEvento {
            id: "financiar_congresso".to_string(),
            texto: "Financiar".to_string(),
            descricao_resultado: "".to_string(),
            efeitos: vec![
                Modificador {
                    id: "aprovacao_intelectuais".to_string(),
                    valor: 12.0,
                },
                Modificador {
                    id: "legitimidade_base".to_string(),
                    valor: 10.0,
                },
            ],
            ia_peso: 100,
        }],
        chance: 1.0,
        generico: true,
    };

    let mut ctx = ContextoEfeitos::com_politica(&mut m, &mut pol);
    aplicar_opcao_com_contexto(&mut ctx, &mut est, &ev, 0, "USA");

    let p_atualizado = pol.pais(&p_id_pol).unwrap();
    let aprov_nova = p_atualizado.grupos_interesse["intelectuais"].aprovacao;
    assert!((aprov_nova - (aprov_inicial + 12.0)).abs() < 1e-3);
    assert!((p_atualizado.legitimidade - (leg_inicial + 10.0).min(100.0)).abs() < 1e-3);
}

#[test]
fn teste_efeito_ligacao_sim_economia() {
    let mut m = mundo_teste();
    let mut eco = economia_teste();
    let mut est = EstadoEventos::default();

    let eco_id = eco
        .paises()
        .iter()
        .find(|p| p.tag.as_deref() == Some("USA"))
        .map(|p| p.id.clone())
        .unwrap();
    let tesouro_inicial = eco
        .paises()
        .iter()
        .find(|p| p.id == eco_id)
        .map(|p| p.orcamento.tesouro)
        .unwrap_or(0.0);
    let fundo_inicial = eco
        .paises()
        .iter()
        .find(|p| p.id == eco_id)
        .map(|p| p.fundo_investimento_privado)
        .unwrap_or(0.0);

    let ev = Evento {
        id: "teste_economia".to_string(),
        titulo: "Bônus de Recuperação e Reinvestimento".to_string(),
        descricao: "Injeção financeira na economia nacional.".to_string(),
        data: (1945, 1, 1),
        paises: vec!["USA".to_string()],
        condicoes: vec![],
        opcoes: vec![OpcaoEvento {
            id: "aceitar_bonus".to_string(),
            texto: "Aceitar".to_string(),
            descricao_resultado: "".to_string(),
            efeitos: vec![
                Modificador {
                    id: "tesouro".to_string(),
                    valor: 1500.0,
                },
                Modificador {
                    id: "investimento_privado_fator".to_string(),
                    valor: 4.0, // 4.0 * 500 = +2000 no fundo privado
                },
            ],
            ia_peso: 100,
        }],
        chance: 1.0,
        generico: true,
    };

    let mut ctx = ContextoEfeitos::com_economia(&mut m, &mut eco);
    aplicar_opcao_com_contexto(&mut ctx, &mut est, &ev, 0, "USA");

    let p_atualizado = eco.paises().iter().find(|p| p.id == eco_id).unwrap();
    assert!((p_atualizado.orcamento.tesouro - (tesouro_inicial + 1500.0)).abs() < 1e-3);
    assert!((p_atualizado.fundo_investimento_privado - (fundo_inicial + 2000.0)).abs() < 1e-3);
}

#[test]
fn teste_disparo_evento_politico_generico() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    // Evento genérico com leis obrigatórias e legitimidade máxima
    let ev_pol = EventoPolitico {
        id: "crise_teste_mocao".to_string(),
        titulo: "Moção de Teste".to_string(),
        categoria: "crise_governo".to_string(),
        descricao: "Crise política.".to_string(),
        condicoes: CondicoesEvento {
            leis_obrigatorias: vec!["sufragio_universal".to_string()],
            leis_proibidas: vec!["estado_partido_unico".to_string()],
            legitimidade_maxima: 45.0,
        },
        opcoes: vec![OpcaoEventoPolitico {
            id: "opcao_a".to_string(),
            texto: "Opção A".to_string(),
            descricao_resultado: "".to_string(),
            efeitos: vec![Modificador {
                id: "legitimidade_base".to_string(),
                valor: 20.0,
            }],
            ia_peso: 100,
        }],
    };

    let ev = ev_pol.em_evento();
    let cat = CatalogoEventos {
        eventos: vec![ev],
    };

    // 1. Configura USA com sufragio_universal, mas legitimidade alta (70.0) -> NÃO deve disparar
    m.paises.get_mut("USA").unwrap().politica.leis_ativas.insert(
        CategoriaLei::Governo,
        "sufragio_universal".to_string(),
    );
    m.paises.get_mut("USA").unwrap().politica.legitimidade = 70.0;
    let disp = avaliar_dia_com_contexto(
        &cat,
        &mut est,
        &mut ContextoEfeitos::apenas_mundo(&mut m),
        "USA",
    );
    assert!(disp.is_empty(), "Não deve disparar com legitimidade alta");

    // 2. Reduz legitimidade para 40.0 (<= 45.0) -> DEVE disparar!
    m.paises.get_mut("USA").unwrap().politica.legitimidade = 40.0;
    let disp = avaliar_dia_com_contexto(
        &cat,
        &mut est,
        &mut ContextoEfeitos::apenas_mundo(&mut m),
        "USA",
    );
    assert_eq!(disp, vec!["crise_teste_mocao".to_string()]);
    assert_eq!(est.pendentes.len(), 1);
    assert_eq!(est.pendentes[0].evento_id, "crise_teste_mocao");
}

#[test]
fn teste_integridade_catalogo_todos_eventos() {
    let r = raiz();
    let m = mundo_teste();
    let cat = CatalogoEventos::carregar(&r.join("assets/data/eventos")).expect("catalogo");

    let mut tags_validas: std::collections::BTreeSet<String> = m.paises.keys().cloned().collect();
    let txt_formaveis =
        std::fs::read_to_string(r.join("data/formaveis/formaveis.ron")).expect("formaveis.ron");
    let formaveis: Vec<simulacao::formacao::Formavel> =
        ron::from_str(&txt_formaveis).expect("parse formaveis.ron");
    for f in formaveis {
        tags_validas.insert(f.tag);
    }

    let ids_catalogo: std::collections::BTreeSet<String> =
        cat.eventos.iter().map(|e| e.id.clone()).collect();

    let mut tags_inexistentes = Vec::new();
    let mut pos_evento_quebrados = Vec::new();
    let mut condicoes_desconhecidas = Vec::new();

    for ev in &cat.eventos {
        for tag in &ev.paises {
            if !tags_validas.contains(tag) {
                tags_inexistentes.push(format!("{}: tag de país '{}' inexistente", ev.id, tag));
            }
        }
        for c in &ev.condicoes {
            match c {
                Condicao::Soberano(tag)
                | Condicao::RelacaoMin(tag, _)
                | Condicao::RelacaoMax(tag, _) => {
                    if !tags_validas.contains(tag) {
                        tags_inexistentes.push(format!(
                            "{}: tag '{}' em condição {:?} inexistente",
                            ev.id, tag, c
                        ));
                    }
                }
                Condicao::PosEvento(ref_id) => {
                    if !ids_catalogo.contains(ref_id) {
                        pos_evento_quebrados
                            .push(format!("{}: pos_evento '{}' inexistente", ev.id, ref_id));
                    }
                }
                Condicao::Desconhecida(raw) => {
                    condicoes_desconhecidas
                        .push(format!("{}: condição desconhecida '{}'", ev.id, raw));
                }
                _ => {}
            }
        }
    }

    assert!(
        tags_inexistentes.is_empty(),
        "Tags inexistentes encontradas:\n{}",
        tags_inexistentes.join("\n")
    );
    assert!(
        pos_evento_quebrados.is_empty(),
        "Referências pos_evento quebradas:\n{}",
        pos_evento_quebrados.join("\n")
    );
    assert!(
        condicoes_desconhecidas.is_empty(),
        "Condições desconhecidas encontradas:\n{}",
        condicoes_desconhecidas.join("\n")
    );
}

#[test]
fn teste_condicoes_abertura_cortina_e_socialista() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    let c_cortina = Condicao::parse("abertura_cortina_de_ferro");
    let c_socialista = Condicao::parse("abertura_socialista_em_curso");
    assert_eq!(c_cortina, Condicao::AberturaCortinaDeFerro);
    assert_eq!(c_socialista, Condicao::AberturaSocialistaEmCurso);

    m.data.ano = 1950;
    m.tensao_mundial = 75.0;
    assert!(!est.avaliar(&m, "HUN", &c_cortina));
    assert!(!est.avaliar(&m, "CZE", &c_socialista));

    est.definir_flag("HUN", "desgelo_cortina_ferro");
    assert!(est.avaliar(&m, "HUN", &c_cortina));

    est.definir_flag("CZE", "reforma_socialista");
    assert!(est.avaliar(&m, "CZE", &c_socialista));
}

#[test]
fn teste_fila_eventos_limite_maximo_tres() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    let cat = CatalogoEventos {
        eventos: vec![
            Evento {
                id: "evt1".to_string(),
                titulo: "Evento Um".to_string(),
                descricao: "Desc".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Opção 1".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
            Evento {
                id: "evt2".to_string(),
                titulo: "Evento Dois".to_string(),
                descricao: "Desc".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Opção 1".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
            Evento {
                id: "evt3".to_string(),
                titulo: "Evento Três".to_string(),
                descricao: "Desc".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Opção 1".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
            Evento {
                id: "evt4".to_string(),
                titulo: "Evento Quatro".to_string(),
                descricao: "Desc".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Opção 1".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
            Evento {
                id: "evt5".to_string(),
                titulo: "Evento Cinco".to_string(),
                descricao: "Desc".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Opção 1".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
        ],
    };

    // Preenche a fila manualmente para testar o limitador
    est.pendentes.push(Pendente { evento_id: "evt1".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.pendentes.push(Pendente { evento_id: "evt2".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.pendentes.push(Pendente { evento_id: "evt3".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.disparados.insert("evt1".to_string(), None);
    est.disparados.insert("evt2".to_string(), None);
    est.disparados.insert("evt3".to_string(), None);

    let mut ctx = ContextoEfeitos::apenas_mundo(&mut m);

    // Adiciona evt4 e evt5 - devem ser resolvidos automaticamente pois excedem o limite de 3
    est.pendentes.push(Pendente { evento_id: "evt4".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.disparados.insert("evt4".to_string(), None);
    est.resolver_excedentes_fila(&cat, &mut ctx, "USA");

    est.pendentes.push(Pendente { evento_id: "evt5".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.disparados.insert("evt5".to_string(), None);
    est.resolver_excedentes_fila(&cat, &mut ctx, "USA");

    // A fila deve ter no máximo 3 eventos
    assert_eq!(est.pendentes.len(), 3, "Fila deve ter no máximo 3 eventos");

    // Agora remove do INÍCIO (mais antigos), então evt1 e evt2 são removidos, ficam evt3, evt4, evt5
    assert_eq!(est.pendentes[0].evento_id, "evt3");
    assert_eq!(est.pendentes[1].evento_id, "evt4");
    assert_eq!(est.pendentes[2].evento_id, "evt5");

    // evt1 e evt2 devem ter sido resolvidos automaticamente
    // Verifica no jornal as decisões automáticas
    let decisoes_auto: Vec<_> = est
        .jornal
        .iter()
        .filter(|n| n.texto.starts_with("Decisão automática:"))
        .collect();
    assert_eq!(decisoes_auto.len(), 2, "Duas decisões automáticas esperadas");
    assert!(decisoes_auto.iter().any(|n| n.texto.contains("Evento Um")));
    assert!(decisoes_auto.iter().any(|n| n.texto.contains("Evento Dois")));
    assert!(decisoes_auto.iter().all(|n| n.texto.contains("Opção 1")));
}

#[test]
fn teste_fila_eventos_deduplicacao_por_titulo() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    let cat = CatalogoEventos {
        eventos: vec![
            Evento {
                id: "evt_a".to_string(),
                titulo: "Mesmo Título".to_string(),
                descricao: "Desc A".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Opção A".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
            Evento {
                id: "evt_b".to_string(),
                titulo: "Mesmo Título".to_string(), // Mesmo título!
                descricao: "Desc B".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Opção B".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
            Evento {
                id: "evt_c".to_string(),
                titulo: "Título Diferente".to_string(),
                descricao: "Desc C".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Opção C".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
generico: true,
            },
        ],
    };
    
    let _ctx = ContextoEfeitos::apenas_mundo(&mut m);
    
    // Adiciona evt_a
    est.pendentes.push(Pendente { evento_id: "evt_a".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.disparados.insert("evt_a".to_string(), None);

    // Tenta adicionar evt_b (mesmo título) - deve ser rejeitado pela deduplicação
    assert!(est.tem_pendente_com_titulo(&cat, "Mesmo Título"));
    // Simula a lógica de avaliar_dia_com_contexto: não adiciona se já existe título
    if !est.tem_pendente_com_titulo(&cat, "Mesmo Título") {
        est.pendentes.push(Pendente { evento_id: "evt_b".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
        est.disparados.insert("evt_b".to_string(), None);
    }

    // Adiciona evt_c (título diferente)
    est.pendentes.push(Pendente { evento_id: "evt_c".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.disparados.insert("evt_c".to_string(), None);

    // Apenas 2 eventos devem estar na fila (evt_a e evt_c; evt_b é duplicata por título)
    assert_eq!(est.pendentes.len(), 2, "Fila deve ter 2 eventos (sem duplicatas por título)");
    assert_eq!(est.pendentes[0].evento_id, "evt_a");
    assert_eq!(est.pendentes[1].evento_id, "evt_c");

    // evt_b não deve ter sido adicionado
    assert!(!est.disparados.contains_key("evt_b"));
}

#[test]
fn teste_fila_eventos_decisao_automatica_registra_no_jornal() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    let cat = CatalogoEventos {
        eventos: vec![
            Evento {
                id: "evt1".to_string(),
                titulo: "Crise Política".to_string(),
                descricao: "Desc".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![
                    OpcaoEvento {
                        id: "opt_a".to_string(),
                        texto: "Negociar".to_string(),
                        descricao_resultado: "".to_string(),
                        efeitos: vec![],
                        ia_peso: 100,
                    },
                    OpcaoEvento {
                        id: "opt_b".to_string(),
                        texto: "Reprimir".to_string(),
                        descricao_resultado: "".to_string(),
                        efeitos: vec![],
                        ia_peso: 50,
                    },
                ],
                chance: 1.0,
                generico: true,
            },
            Evento {
                id: "evt2".to_string(),
                titulo: "Revolta Popular".to_string(),
                descricao: "Desc".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Conceder Reformas".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
            Evento {
                id: "evt3".to_string(),
                titulo: "Greve Geral".to_string(),
                descricao: "Desc".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Dialogar".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
            Evento {
                id: "evt4".to_string(),
                titulo: "Motim Militar".to_string(),
                descricao: "Desc".to_string(),
                data: (1945, 1, 1),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Anistia".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true,
            },
        ],
    };

    // Preenche a fila com 3 eventos
    est.pendentes.push(Pendente { evento_id: "evt1".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.pendentes.push(Pendente { evento_id: "evt2".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.pendentes.push(Pendente { evento_id: "evt3".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.disparados.insert("evt1".to_string(), None);
    est.disparados.insert("evt2".to_string(), None);
    est.disparados.insert("evt3".to_string(), None);

    let mut ctx = ContextoEfeitos::apenas_mundo(&mut m);

    // Adiciona evt4 - deve ser resolvido automaticamente o mais antigo (evt1) pois excede o limite de 3
    est.pendentes.push(Pendente { evento_id: "evt4".to_string(), pais: "USA".to_string(), data_adicao: 0, historico: false });
    est.disparados.insert("evt4".to_string(), None);
    est.resolver_excedentes_fila(&cat, &mut ctx, "USA");

    // evt1 deve ter sido resolvido automaticamente com a primeira opção (remove do início)
    let decisoes_auto: Vec<_> = est
        .jornal
        .iter()
        .filter(|n| n.texto.starts_with("Decisão automática:"))
        .collect();
    assert_eq!(decisoes_auto.len(), 1);
    assert!(decisoes_auto[0].texto.contains("Crise Política"));
    assert!(decisoes_auto[0].texto.contains("Negociar")); // primeira opção
    assert_eq!(decisoes_auto[0].texto, "Decisão automática: Crise Política — Negociar");
}

#[test]
fn teste_expiracao_eventos_historicos() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    let cat = CatalogoEventos {
        eventos: vec![
            Evento {
                id: "evt_historico_antigo".to_string(),
                titulo: "Conferência de Yalta".to_string(),
                descricao: "Evento histórico de fevereiro de 1945".to_string(),
                data: (1945, 2, 4),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Aceitar partilha".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: false, // histórico
            },
            Evento {
                id: "evt_generico_recente".to_string(),
                titulo: "Crise Política Recente".to_string(),
                descricao: "Evento genérico recente".to_string(),
                data: (1949, 6, 16),
                paises: vec!["USA".to_string()],
                condicoes: vec![],
                opcoes: vec![OpcaoEvento {
                    id: "opt1".to_string(),
                    texto: "Negociar".to_string(),
                    descricao_resultado: "".to_string(),
                    efeitos: vec![],
                    ia_peso: 100,
                }],
                chance: 1.0,
                generico: true, // genérico não expira
            },
        ],
    };

    let hoje_abs = dias((1949, 6, 16)); // 16/06/1949 - mais de 60 dias depois de Yalta

    // Adiciona evento histórico antigo (simula que entrou na fila em 1945)
    est.pendentes.push(Pendente {
        evento_id: "evt_historico_antigo".to_string(),
        pais: "USA".to_string(),
        data_adicao: dias((1945, 2, 4)), // entrou na fila na data do evento
        historico: true,
    });
    est.disparados.insert("evt_historico_antigo".to_string(), None);

    // Adiciona evento genérico recente
    est.pendentes.push(Pendente {
        evento_id: "evt_generico_recente".to_string(),
        pais: "USA".to_string(),
        data_adicao: hoje_abs - 10, // entrou há 10 dias
        historico: false,
    });
    est.disparados.insert("evt_generico_recente".to_string(), None);

    let mut ctx = ContextoEfeitos::apenas_mundo(&mut m);

    // Chama expiração - o evento histórico deve expirar (passou 60+ dias na fila E passou da data-limite)
    est.expirar_eventos_historicos(&cat, &mut ctx, hoje_abs);

    // Evento histórico deve ter sido removido e resolvido automaticamente
    assert_eq!(est.pendentes.len(), 1);
    assert_eq!(est.pendentes[0].evento_id, "evt_generico_recente");

    // Verifica no jornal a decisão automática do evento expirado
    let decisoes_auto: Vec<_> = est
        .jornal
        .iter()
        .filter(|n| n.texto.starts_with("Decisão automática (expirado):"))
        .collect();
    assert_eq!(decisoes_auto.len(), 1);
    assert!(decisoes_auto[0].texto.contains("Conferência de Yalta"));
    assert!(decisoes_auto[0].texto.contains("Aceitar partilha"));
}

#[test]
fn teste_evento_historico_nao_expira_antes_de_60_dias() {
    let mut m = mundo_teste();
    let mut est = EstadoEventos::default();

    let cat = CatalogoEventos {
        eventos: vec![Evento {
            id: "evt_recente".to_string(),
            titulo: "Evento Recente".to_string(),
            descricao: "Desc".to_string(),
            data: (1949, 5, 1), // 1 mês atrás
            paises: vec!["USA".to_string()],
            condicoes: vec![],
            opcoes: vec![OpcaoEvento {
                id: "opt1".to_string(),
                texto: "Opção 1".to_string(),
                descricao_resultado: "".to_string(),
                efeitos: vec![],
                ia_peso: 100,
            }],
            chance: 1.0,
            generico: false,
        }],
    };

    let hoje_abs = dias((1949, 6, 16)); // 46 dias depois do evento

    est.pendentes.push(Pendente {
        evento_id: "evt_recente".to_string(),
        pais: "USA".to_string(),
        data_adicao: dias((1949, 5, 1)),
        historico: true,
    });
    est.disparados.insert("evt_recente".to_string(), None);

    let mut ctx = ContextoEfeitos::apenas_mundo(&mut m);
    est.expirar_eventos_historicos(&cat, &mut ctx, hoje_abs);

    // Evento NÃO deve expirar (menos de 60 dias na fila e menos de 60 dias da data-limite)
    assert_eq!(est.pendentes.len(), 1);
    assert_eq!(est.pendentes[0].evento_id, "evt_recente");

    let decisoes_auto: Vec<_> = est
        .jornal
        .iter()
        .filter(|n| n.texto.starts_with("Decisão automática (expirado):"))
        .collect();
    assert_eq!(decisoes_auto.len(), 0);
}
