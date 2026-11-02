use bandeiras_resolver::*;

fn est(forma: &str, ideo: &str) -> EstadoPais {
    EstadoPais { ano: 1945, mes: 1, forma_governo: forma.into(), ideologia_dominante: ideo.into(), religiao_estatal: "estado_laico".into(), ..Default::default() }
}

#[test]
fn carrega_catalogo_completo() {
    let c = catalogo_embutido();
    assert!(c.tags().count() >= 155 + 91);
    assert!(c.pais("ITA").is_some() && c.pais("GHA").is_some() && c.pais("DEU").is_some());
}

#[test]
fn italia_1945_reino_e_1946_republica() {
    let mut e = est("monarquia_constitucional", "democracia_liberal");
    let r = resolver_bandeira("ITA", &e);
    assert!(r.e_base, "1945: bandeira base do Reino");
    e.ano = 1946; e.mes = 6;
    e.forma_governo = "republica_parlamentarista".into();
    let r = resolver_bandeira("ITA", &e);
    assert_eq!(r.id, "ita_republica");
    assert_eq!(r.nome, "República Italiana");
}

#[test]
fn urss_sempre_comunista() {
    for (forma, ideo, partido) in [
        ("estado_partido_unico", "comunismo_marxista_leninista", None),
        ("estado_partido_unico", "comunismo_marxista_leninista", Some("marxismo_leninismo")),
        ("estado_partido_unico", "comunismo_marxista_leninista", Some("trotskismo")),
    ] {
        let mut e = est(forma, ideo);
        e.ideologia_partido = partido.map(String::from);
        let r = resolver_bandeira("SOV", &e);
        assert_eq!(r.id, "sov_comunista");
        assert_eq!(r.arquivo, "svg/SOV.svg");
        assert!(r.nome.contains("Socialistas Soviéticas"));
    }
}

#[test]
fn hungria_com_partido_comunista_troca_de_bandeira() {
    let mut e = est("republica_parlamentarista", "democracia_liberal");
    e.ideologia_partido = Some("democracia_crista".into());
    assert_eq!(resolver_bandeira("HUN", &e).id, "hun_republica");
    e.ideologia_partido = Some("marxismo_leninismo".into());
    e.partido = Some("hun_mdp".into());
    let r = resolver_bandeira("HUN", &e);
    assert_eq!(r.id, "hun_comunista");
    assert_eq!(r.nome, "República Popular da Hungria");
    assert!(r.cor.0 > r.cor.1);
}

#[test]
fn colonia_que_se_torna_independente() {
    let mut e = est("administracao_colonial", "colonialismo_imperial");
    e.status = Status::Colonia;
    e.suserano = Some("GBR".into());
    assert!(resolver_bandeira("GHA", &e).e_base);
    e.status = Status::Independente;
    e.suserano = None;
    e.forma_governo = "republica_presidencialista".into();
    e.ideologia_dominante = "democracia_liberal".into();
    let r = resolver_bandeira("GHA", &e);
    assert_eq!(r.id, "gha_independente");
    // regime comunista vence a independente liberal (mais especifico); Angola usa a historica
    e.ideologia_partido = Some("marxismo_leninismo".into());
    // Independência + partido comunista no poder = mudança real de regime: é aí que a regra
    // genérica de ideologia pode renomear o país.
    e.mudanca_regime = true;
    assert_eq!(resolver_bandeira("GHA", &e).id, "comunista");
    assert_eq!(resolver_bandeira("ANG", &e).id, "ang_mpla");
}

/// Variante religiosa: a lei de religião de estado sozinha não basta, o clero tem de dominar o governo.
#[test]
fn teocracia_exige_a_lei_e_o_clero_no_governo() {
    let mut e = EstadoPais {
        ano: 1945,
        mes: 1,
        forma_governo: "autocracia_monarquica".into(),
        religiao_estatal: "religiao_oficial_obrigatoria".into(),
        ideologia_dominante: "tradicionalismo_monarquico".into(),
        ..Default::default()
    };
    assert_ne!(resolver_bandeira("POR", &e).id, "teocracia", "sem o clero no governo não é teocracia");
    e.grupo_religioso_no_poder = true;
    let r = resolver_bandeira("POR", &e);
    assert_eq!(r.id, "teocracia");
    assert!(r.nome.contains("Teocrático"), "{}", r.nome);
}

/// Regra genérica de ideologia ("República Revolucionária") não renomeia país sem mudança de regime,
/// e mesmo com mudança precisa da ideologia do partido no poder (não a crença do chefe de Estado).
#[test]
fn regra_generica_de_ideologia_precisa_de_regime_e_do_partido_no_poder() {
    let mut e = EstadoPais {
        ano: 1945,
        mes: 1,
        forma_governo: "republica_parlamentarista".into(),
        religiao_estatal: "estado_laico".into(),
        ideologia_dominante: "democracia_liberal".into(),
        ideologia_chefe: Some("nacionalismo_republicano".into()),
        ..Default::default()
    };
    assert_ne!(resolver_bandeira("FRA", &e).id, "nacionalista", "de Gaulle não faz da França uma república revolucionária");
    e.mudanca_regime = true;
    assert_ne!(resolver_bandeira("FRA", &e).id, "nacionalista", "mudança de regime sem o partido no poder não basta");
    e.ideologia_partido = Some("nacionalismo_republicano".into());
    let r = resolver_bandeira("FRA", &e);
    assert_eq!(r.id, "nacionalista");
    assert!(r.nome.contains("Revolucionária"), "{}", r.nome);
}

/// Com o regime do cenário (sem mudança), o nome mostrado é o nome histórico do cenário.
#[test]
fn regime_inicial_mostra_o_nome_do_cenario() {
    let mut e = EstadoPais {
        ano: 1945,
        mes: 1,
        forma_governo: "estado_partido_unico".into(),
        religiao_estatal: "ateismo_estatal_combatente".into(),
        ideologia_dominante: "comunismo".into(),
        ideologia_chefe: Some("comunismo_marxista_leninista".into()),
        nome_cenario: "União Soviética".into(),
        ..Default::default()
    };
    let r = resolver_bandeira("SOV", &e);
    assert_eq!(r.id, "sov_comunista", "a bandeira soviética é a correta para 1945");
    assert_eq!(r.nome, "União Soviética", "o nome é o do cenário enquanto nada mudou");
    e.mudanca_regime = true;
    assert!(resolver_bandeira("SOV", &e).nome.contains("Socialistas Soviéticas"), "depois de mudar o regime, vale o título do catálogo");
}

#[test]
fn legenda_de_ocupacao_nao_vai_no_nome() {
    assert_eq!(sem_ocupacao("Alemanha Ocupada (Conselho de Controle)"), "Alemanha");
    assert_eq!(sem_ocupacao("Japão Ocupado (SCAP)"), "Japão");
    assert_eq!(sem_ocupacao("Áustria Ocupada"), "Áustria");
    assert_eq!(sem_ocupacao("França (Governo Provisório)"), "França (Governo Provisório)");

    let mut e = est("republica_parlamentarista", "democracia_liberal");
    e.nome_cenario = "Áustria Ocupada".into();
    assert_eq!(resolver_bandeira("AUT", &e).nome, "Áustria", "a ocupação se mostra pela hachura, não pelo nome");
}

#[test]
fn ocupacao_e_guerra_civil() {
    let mut e = est("republica_parlamentarista", "democracia_liberal");
    e.ocupante = Some("GER".into());
    let r = resolver_bandeira("FRA", &e);
    assert_eq!(r.id, "ocupado");
    assert_eq!(r.sobreposicao, Sobreposicao::CantaoOcupante);
    assert_eq!(r.tag_cantao.as_deref(), Some("GER"));
    e.ocupante = None;
    e.guerra_civil = true;
    assert_eq!(resolver_bandeira("FRA", &e).id, "guerra_civil");
}

#[test]
fn mais_especifico_vence_taliba() {
    let mut e = est("autocracia_monarquica", "tradicionalismo_monarquico");
    e.ideologia_partido = Some("islamismo_politico".into());
    assert_eq!(resolver_bandeira("AFG", &e).id, "afg_islamico");
    e.religiao_estatal = "religiao_oficial_obrigatoria".into();
    e.grupo_religioso_no_poder = true;
    assert_eq!(resolver_bandeira("AFG", &e).id, "afg_emirado");
}

#[test]
fn alemanha_dividida_por_flags_e_tag_desconhecida() {
    let mut e = est("ocupacao_militar_estrangeira", "democracia_liberal");
    e.flags.push("rfa".into());
    assert_eq!(resolver_bandeira("GER", &e).id, "ger_rfa");
    e.flags = vec!["rda".into()];
    e.ideologia_partido = Some("marxismo_leninismo".into());
    assert_eq!(resolver_bandeira("GER", &e).id, "ger_rda");
    assert_eq!(resolver_bandeira("ZZZ", &e).id, "desconhecida");
}

#[test]
#[ignore = "exige svg/png de bandeiras_dinamicas (assets ainda nao integrados)"]
fn todo_arquivo_do_catalogo_existe() {
    let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for p in catalogo_embutido().dados().paises.iter() {
        assert!(raiz.join(&p.base.arquivo).exists(), "{}", p.base.arquivo);
        for v in &p.variantes {
            assert!(raiz.join(&v.arquivo).exists(), "{} {}", p.tag, v.arquivo);
        }
    }
}
