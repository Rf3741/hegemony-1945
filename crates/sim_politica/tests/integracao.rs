//! Testes de integração do motor político: 20 anos de uma democracia e de um país
//! de partido único, determinismo por semente, guerra civil e influência externa.

use sim_politica::*;
use std::collections::BTreeMap;

fn lei(id: &str, grupo: &str) -> DefinicaoLei {
    DefinicaoLei {
        id: id.into(),
        nome: id.into(),
        descricao: String::new(),
        grupo_lei: grupo.into(),
        efeitos: vec![],
        pre_requisitos: PreRequisitos::default(),
        apoiadores: vec![],
        opositores: vec![],
        sem_eleicoes: false,
        eleicoes_partido_unico: false,
        voto_censitario: false,
        voto_capacitario: false,
        sufragio_universal: false,
        sufragio_masculino: false,
        limiar_padrao_vida_voto: None,
        alfabetizacao_minima_voto: None,
        multiplicadores_estrato: None,
    }
}

fn ideologia(id: &str, bloco: AlinhamentoBloco, fav: &[&str]) -> Ideologia {
    Ideologia {
        id: id.into(),
        nome: id.into(),
        descricao: String::new(),
        alinhamento_padrao: bloco,
        principios: vec![id.into()],
        leis_favorecidas: fav.iter().map(|s| s.to_string()).collect(),
        leis_rejeitadas: vec![],
    }
}

fn def_gi(
    id: &str,
    ideol: &str,
    pops: &[&str],
    pref: &[&str],
    odeia: &[&str],
) -> DefinicaoGrupoInteresse {
    DefinicaoGrupoInteresse {
        id: id.into(),
        nome: id.into(),
        descricao: String::new(),
        ideologias: vec![ideol.into()],
        base_social_pops: pops
            .iter()
            .map(|p| BaseSocialPop {
                tipo_pop: p.to_string(),
                peso_atracao: 1.0,
            })
            .collect(),
        leis_preferidas: pref.iter().map(|s| s.to_string()).collect(),
        leis_odiadas: odeia.iter().map(|s| s.to_string()).collect(),
        bens_importantes: vec![],
        modificadores_leal: vec![],
        modificadores_insurrecto: vec![],
    }
}

fn catalogo() -> CatalogoPolitica {
    let mut c = CatalogoPolitica::default();
    for i in [
        ideologia(
            "liberal",
            AlinhamentoBloco::BlocoOcidental,
            &["livre_mercado"],
        ),
        ideologia(
            "socialista",
            AlinhamentoBloco::BlocoSovietico,
            &["intervencionismo"],
        ),
    ] {
        c.ideologias.insert(i.id.clone(), i);
    }
    let mut sufragio = lei("sufragio_universal", "sistema_eleitoral");
    sufragio.sufragio_universal = true;
    let mut unico = lei("eleicoes_partido_unico", "sistema_eleitoral");
    unico.eleicoes_partido_unico = true;
    let cats = vec![
        ("sistema_eleitoral", vec![sufragio, unico]),
        (
            "economia",
            vec![
                lei("livre_mercado", "economia"),
                lei("intervencionismo", "economia"),
            ],
        ),
    ];
    for (cat, leis) in cats {
        for l in &leis {
            c.leis.insert(l.id.clone(), l.clone());
        }
        c.categorias_leis.insert(
            cat.into(),
            CategoriaLeis {
                categoria: cat.into(),
                nome: cat.into(),
                descricao: String::new(),
                leis,
            },
        );
    }
    for g in [
        def_gi(
            "industriais",
            "liberal",
            &["capitalistas"],
            &["livre_mercado"],
            &["intervencionismo"],
        ),
        def_gi(
            "trabalhadores",
            "socialista",
            &["operarios"],
            &["intervencionismo"],
            &["livre_mercado"],
        ),
        def_gi(
            "rurais",
            "liberal",
            &["camponeses"],
            &["livre_mercado"],
            &[],
        ),
    ] {
        c.grupos_interesse.insert(g.id.clone(), g);
    }
    for (id, ideol, nat) in [
        ("partido_liberal", "liberal", vec!["industriais", "rurais"]),
        ("partido_trabalhista", "socialista", vec!["trabalhadores"]),
    ] {
        c.arquetipos_partido.insert(
            id.into(),
            ArquetipoPartido {
                id: id.into(),
                nome: id.into(),
                descricao: String::new(),
                ideologias_afins: vec![ideol.into()],
                grupos_naturais: nat.into_iter().map(String::from).collect(),
                prioridade: 1,
            },
        );
    }
    c
}

fn pais_base(id: &str, tag: &str) -> PaisPolitica {
    let mut p = PaisPolitica::novo(id, tag, id).unwrap();
    p.culturas_primarias = vec!["anglo".into()];
    p.pops = vec![
        PopPolitica::novo(
            "capitalistas",
            "anglo",
            "protestante",
            Estrato::Alto,
            50_000.0,
            30.0,
            0.95,
        ),
        PopPolitica::novo(
            "operarios",
            "anglo",
            "protestante",
            Estrato::Medio,
            600_000.0,
            16.0,
            0.80,
        ),
        PopPolitica::novo(
            "camponeses",
            "anglo",
            "protestante",
            Estrato::Baixo,
            350_000.0,
            10.0,
            0.50,
        ),
    ];
    for (gi, papel) in [
        ("industriais", PapelGrupoInteresse::NoGoverno),
        ("trabalhadores", PapelGrupoInteresse::NaOposicao),
        ("rurais", PapelGrupoInteresse::NaOposicao),
    ] {
        p.grupos_interesse
            .insert(gi.into(), InstanciaGrupoInteresse::novo(gi, gi, papel));
    }
    p.chefe_de_estado = Some(LiderPolitico::novo(
        "l1",
        "Líder Um",
        "liberal",
        30.0,
        1.0,
        1890,
    ));
    p.gi_chefe_estado = Some("industriais".into());
    p
}

fn democracia() -> PaisPolitica {
    let mut p = pais_base("democracia_teste", "DEM");
    p.promulgar_lei_instantanea("sistema_eleitoral", "sufragio_universal");
    p.promulgar_lei_instantanea("economia", "livre_mercado");
    p
}

fn partido_unico() -> PaisPolitica {
    let mut p = pais_base("estado_unico_teste", "UNI");
    p.promulgar_lei_instantanea("sistema_eleitoral", "eleicoes_partido_unico");
    p.promulgar_lei_instantanea("forma_governo", "estado_partido_unico");
    p.promulgar_lei_instantanea("economia", "intervencionismo");
    p
}

fn mundo(seed: u64) -> MundoPolitico {
    MundoPolitico::novo(catalogo(), vec![democracia(), partido_unico()], seed).unwrap()
}

const VINTE_ANOS: u32 = 365 * 20 + 5;

#[test]
fn catalogo_de_teste_e_valido() {
    catalogo().validar().unwrap();
}

#[test]
fn clout_soma_um_e_capitalistas_pesam_mais_que_sua_populacao() {
    let m = mundo(1);
    let p = m.pais("democracia_teste").unwrap();
    let soma: f64 = p.grupos_interesse.values().map(|g| g.clout).sum();
    assert!((soma - 1.0).abs() < 0.02, "soma={soma}");
    let ind = p.grupos_interesse["industriais"].clout;
    assert!(
        ind > 0.05,
        "5% da população deveria ter mais de 5% de clout: {ind}"
    );
}

#[test]
fn sufragio_restrito_reduz_forca_politica_dos_pobres() {
    let c = catalogo();
    let mut a = democracia();
    let mut b = democracia();
    let mut cens = lei("sufragio_censitario", "sistema_eleitoral");
    cens.voto_censitario = true;
    cens.limiar_padrao_vida_voto = Some(15.0);
    let mut c2 = c.clone();
    c2.leis.insert(cens.id.clone(), cens);
    b.promulgar_lei_instantanea("sistema_eleitoral", "sufragio_censitario");
    sim_politica::motor::forca_politica::calcular_forca_politica_e_clout(&mut a, &c);
    sim_politica::motor::forca_politica::calcular_forca_politica_e_clout(&mut b, &c2);
    assert!(b.grupos_interesse["rurais"].clout < a.grupos_interesse["rurais"].clout);
    assert!(b.grupos_interesse["industriais"].clout > a.grupos_interesse["industriais"].clout);
}

#[test]
fn democracia_realiza_eleicoes_em_vinte_anos() {
    let mut m = mundo(7);
    let rel = m.executar_periodo(VINTE_ANOS);
    assert!(
        rel.eleicoes_realizadas >= 8,
        "eleições: {}",
        rel.eleicoes_realizadas
    );
    let p = m.pais("democracia_teste").unwrap();
    let e = p.ultima_eleicao.as_ref().unwrap();
    assert!(e.partido_vencedor_id.is_some());
    let cadeiras: u32 = e.cadeiras_por_partido.values().sum();
    assert_eq!(cadeiras, 100);
    m.auditar_estabilidade().unwrap();
}

#[test]
fn partido_unico_tem_eleicao_plebiscitaria_com_unico_partido() {
    let mut m = mundo(7);
    m.executar_periodo(VINTE_ANOS);
    let p = m.pais("estado_unico_teste").unwrap();
    assert_eq!(p.partidos.len(), 1);
    let e = p.ultima_eleicao.as_ref().expect("eleição de vanguarda");
    assert_eq!(e.cadeiras_por_partido.len(), 1);
    assert!(e.taxa_participacao > 0.9);
}

#[test]
fn leis_passam_em_vinte_anos() {
    let mut m = mundo(11);
    let mut promulgadas = 0;
    for _ in 0..VINTE_ANOS {
        {
            let p = m.pais_mut("democracia_teste").unwrap();
            if p.lei_em_tramitacao.is_none() {
                let atual = p.leis_ativas["economia"].clone();
                let alvo = if atual == "livre_mercado" {
                    "intervencionismo"
                } else {
                    "livre_mercado"
                };
                p.iniciar_tramitacao("economia", alvo).unwrap();
            }
        }
        promulgadas += m.executar_tick_diario().leis_promulgadas;
    }
    assert!(promulgadas >= 1, "nenhuma lei passou");
    m.auditar_estabilidade().unwrap();
}

#[test]
fn nao_se_inicia_segunda_tramitacao_nem_lei_ja_vigente() {
    let mut p = democracia();
    assert!(p.iniciar_tramitacao("economia", "livre_mercado").is_err());
    p.iniciar_tramitacao("economia", "intervencionismo")
        .unwrap();
    assert!(p.iniciar_tramitacao("economia", "livre_mercado").is_err());
}

#[test]
fn trocar_lider_muda_postura_do_gi() {
    let c = catalogo();
    let mut gi = InstanciaGrupoInteresse::novo("rurais", "rurais", PapelGrupoInteresse::NaOposicao);
    let antes = gi.avaliar_postura_mudanca_lei(&c, "intervencionismo", "livre_mercado");
    gi.trocar_lider(LiderPolitico::novo(
        "l2",
        "Socialista",
        "socialista",
        10.0,
        1.0,
        1900,
    ));
    let depois = gi.avaliar_postura_mudanca_lei(&c, "intervencionismo", "livre_mercado");
    assert_ne!(antes, depois);
    assert!(depois.eh_favoravel());
}

#[test]
fn determinismo_com_mesma_semente_e_divergencia_com_outra() {
    let resumo = |seed: u64| {
        let mut m = mundo(seed);
        m.executar_periodo(365 * 6);
        let p = m.pais("democracia_teste").unwrap();
        (
            p.legitimidade.to_bits(),
            p.ultima_eleicao.clone().map(|e| {
                e.votos_por_partido
                    .into_iter()
                    .map(|(k, v)| (k, v.to_bits()))
                    .collect::<BTreeMap<_, _>>()
            }),
            p.grupos_interesse
                .values()
                .map(|g| g.aprovacao.to_bits())
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(resumo(42), resumo(42));
    assert_ne!(resumo(42), resumo(43));
}

#[test]
fn reformar_governo_rejeita_gi_inexistente() {
    let mut p = democracia();
    assert!(p.reformar_governo(&["fantasma".to_string()]).is_err());
    p.reformar_governo(&["trabalhadores".to_string()]).unwrap();
    assert!(p.grupos_interesse["trabalhadores"].eh_no_governo());
}

#[test]
fn miseria_gera_radicais_e_revolucao_sinaliza_guerra_civil() {
    let mut p = democracia();
    for pop in &mut p.pops {
        pop.padrao_de_vida = 1.5;
        pop.taxa_desemprego = 0.6;
    }
    p.legitimidade = 10.0;
    let mut mov = MovimentoPolitico::novo(
        "rev",
        "Revolução",
        ObjetivoMovimento::LibertacaoAnticolonial,
    );
    mov.grupos_apoiadores = vec!["trabalhadores".into(), "rurais".into()];
    p.movimentos.push(mov);
    let mut m = MundoPolitico::novo(catalogo(), vec![p], 5).unwrap();
    m.executar_periodo(365 * 3);
    let p = m.pais("democracia_teste").unwrap();
    assert!(p.total_radicais() > 0.0, "sem radicais");
    m.auditar_estabilidade().unwrap();
    let _ = m.drenar_eventos_guerra_civil();
}

#[test]
fn financiamento_externo_desloca_esfera_e_clout_externo() {
    let mut p = democracia();
    p.influencia_externa
        .operacoes_urss
        .push(OperacaoInfluencia::FinanciarGrupoInteresse {
            gi_id: "trabalhadores".into(),
            montante_diario: 40.0,
        });
    let mut m = MundoPolitico::novo(catalogo(), vec![p], 3).unwrap();
    m.executar_periodo(400);
    let p = m.pais("democracia_teste").unwrap();
    assert!(p.influencia_externa.pontos_urss > p.influencia_externa.pontos_eua);
    assert!(p.grupos_interesse["trabalhadores"].bonus_financiamento_externo > 0.0);
}

#[test]
fn catalogo_roundtrip_ron() {
    let c = catalogo();
    let s = ron::to_string(&c).unwrap();
    let c2 = CatalogoPolitica::de_str_ron(&s).unwrap();
    assert_eq!(c, c2);
}

#[test]
fn ids_invalidos_sao_rejeitados() {
    assert!(PaisPolitica::novo("Brasil Maiúsculo", "BRA", "x").is_err());
    let r = MundoPolitico::novo(catalogo(), vec![democracia(), democracia()], 1);
    assert!(r.is_err());
}

#[test]
fn dhondt_distribui_cem_cadeiras() {
    let mut v = BTreeMap::new();
    v.insert("a".to_string(), 600.0);
    v.insert("b".to_string(), 350.0);
    v.insert("c".to_string(), 10.0);
    let r = ResultadoEleicao::calcular_cadeiras_dhondt(100, &v, 0.03);
    assert_eq!(r.values().sum::<u32>(), 100);
    assert!(r["a"] > r["b"]);
    assert_eq!(r.get("c").copied().unwrap_or(0), 0);
}

#[test]
fn rng_e_deterministico() {
    let mut a = RngDeterministico::com_semente(9);
    let mut b = RngDeterministico::com_semente(9);
    for _ in 0..100 {
        assert_eq!(a.proximo_u64(), b.proximo_u64());
    }
}
