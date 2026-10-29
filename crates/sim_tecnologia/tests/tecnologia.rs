use sim_tecnologia::*;
use std::path::PathBuf;
use std::sync::Arc;

fn cat() -> Arc<CatalogoTec> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/tecnologia");
    Arc::new(CatalogoTec::carregar(&dir).expect("carregar"))
}

fn mundo() -> MundoTec {
    let b = ConfigBlocos {
        grandes_potencias: vec!["USA".into(), "SOV".into(), "GBR".into()],
        lider_do: [("BRA".to_string(), "USA".to_string())].into(),
    };
    MundoTec::novo(cat(), b)
}

#[test]
fn carrega_sem_erro_e_sem_ciclos() {
    let c = cat();
    assert_eq!(c.techs.len(), 152);
    assert!(c.iniciais.len() >= 150);
    for r in Ramo::TODOS {
        assert!(c.techs.iter().any(|t| t.ramo == r));
    }
}

#[test]
fn pesquisa_avanca_e_conclui() {
    let mut m = mundo();
    let ano = 1945;
    let e = EntradaPais { niveis_universidade: 10.0, alfabetizacao: 0.95, estabilidade: 1.0, ..Default::default() };
    let alvo = m.catalogo.techs.iter().find(|t| m.pode_iniciar("USA", t, ano)).expect("disponivel").id.clone();
    m.iniciar("USA", &alvo, ano).unwrap();
    m.tick_pais("USA", ano, &e);
    assert!(m.paises["USA"].progresso[&alvo] > 0.0);
    let mut feito = false;
    for _ in 0..20000 {
        if m.tick_pais("USA", ano, &e).is_some() {
            feito = true;
            break;
        }
    }
    assert!(feito && m.conhece("USA", &alvo));
}

#[test]
fn difusao_barateia() {
    let mut m = mundo();
    let e = EntradaPais { alfabetizacao: 0.9, ..Default::default() };
    // tech que USA conhece e BRA nao
    let t = m.catalogo.techs.iter().find(|t| m.conhece("USA", &t.id) && !m.conhece("BRA", &t.id) && t.ano_historico <= 1945).cloned();
    let t = t.expect("tech");
    let com = m.custo_efetivo("BRA", &t, 1945, &e);
    m.paises.get_mut("USA").unwrap().conhecidas.remove(&t.id);
    let sem = m.custo_efetivo("BRA", &t, 1945, &e);
    assert!(com < sem, "{com} {sem}");
}

#[test]
fn antecedencia_e_pontos() {
    assert_eq!(penalidade_antecedencia(1950, 1955), 0.20 * 5.0 + 0.03 * 25.0);
    assert_eq!(penalidade_antecedencia(1960, 1955), 0.0);
    let p0 = pontos_diarios(&EntradaPais::default());
    let p1 = pontos_diarios(&EntradaPais { niveis_universidade: 5.0, ..Default::default() });
    assert!(p1 > p0);
}

#[test]
fn desbloqueio_funciona() {
    let mut m = mundo();
    // acha tech com predio que USA nao conhece
    let c = m.catalogo.clone();
    let t = c.techs.iter().find(|t| !t.desbloqueia.predios.is_empty() && !m.conhece("BRA", &t.id) && c.libera_predio[&t.desbloqueia.predios[0]].len() == 1).expect("tech");
    let p = &t.desbloqueia.predios[0];
    assert!(!m.predio_liberado("BRA", p));
    assert!(m.bloqueios_predios("BRA").contains_key(p));
    m.paises.get_mut("BRA").unwrap().conhecidas.insert(t.id.clone());
    assert!(m.predio_liberado("BRA", p));
    // item nao citado e livre
    assert!(m.predio_liberado("BRA", "predio_inexistente_livre"));
    assert!(m.era_visual("USA") >= 1945);
}

#[test]
fn ia_escolhe_e_fila() {
    let mut m = mundo();
    let e = EntradaPais::default();
    m.escolher_ia("BRA", 1945, &e);
    assert!(m.paises["BRA"].ativa.is_some());
}
