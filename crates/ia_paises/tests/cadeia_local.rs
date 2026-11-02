//! Recursos locais dominam: processar onde se extrai (cadeia local) e extrair só onde há o recurso.
use ia_paises::IaEconomia;
use sim_economia::{CenarioEconomia, ModoCadencia, MundoEconomico};
use std::path::PathBuf;

fn mundo_aquecido() -> MundoEconomico {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cen = CenarioEconomia::carregar_diretorio(&raiz.join("assets/data/economia")).expect("economia");
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    let mut eco = MundoEconomico::novo(cen).expect("mundo economia");
    for _ in 0..30 {
        eco.executar_tick_diario();
    }
    eco
}

#[test]
fn cadeia_local_e_recurso_dominam_a_estimativa() {
    let eco = mundo_aquecido();
    let cat = eco.catalogo();
    let tipo = |id: &str| cat.tipos_predio.iter().position(|t| t.id == id).unwrap_or_else(|| panic!("tipo {id}"));
    let bem = |id: &str| cat.bens.iter().position(|b| b.id == id).unwrap_or_else(|| panic!("bem {id}"));

    // Siderúrgica: o estado com mais insumos locais ganha bônus bem maior que um estado sem produção.
    let sid = tipo("siderurgica");
    let melhor = eco.estados().iter().max_by(|a, b| a.bonus_cadeia(sid).total_cmp(&b.bonus_cadeia(sid))).unwrap();
    let pior = eco
        .estados()
        .iter()
        .filter(|e| e.pais_id == melhor.pais_id)
        .min_by(|a, b| a.bonus_cadeia(sid).total_cmp(&b.bonus_cadeia(sid)))
        .unwrap();
    eprintln!(
        "siderúrgica: melhor {} +{:.1}%, pior {} +{:.1}%",
        melhor.id,
        melhor.bonus_cadeia(sid) * 100.0,
        pior.id,
        pior.bonus_cadeia(sid) * 100.0
    );
    assert!(melhor.bonus_cadeia(sid) > 0.05, "algum estado deveria ter bônus de cadeia na siderúrgica");
    assert!(melhor.bonus_cadeia(sid) > pior.bonus_cadeia(sid) + 0.03);
    let locais = ["minerio_ferro", "ferro", "carvao"]
        .iter()
        .filter_map(|id| cat.bens.iter().position(|b| b.id == *id))
        .filter(|&g| melhor.producao_local(g) > 0.0)
        .count();
    assert!(locais >= 1, "o melhor estado deveria produzir ferro/carvão");

    // Refinaria: bônus maior onde o petróleo é extraído.
    let refinaria = tipo("refinaria_petroleo");
    let pet = bem("petroleo");
    let com = eco.estados().iter().filter(|e| e.producao_local(pet) > 0.0).map(|e| e.bonus_cadeia(refinaria)).fold(0.0, f64::max);
    assert!(com > 0.05, "refinaria em estado com petróleo deve ter bônus (obtido {com})");
    let sem = eco
        .estados()
        .iter()
        .filter(|e| e.producao_local(pet) <= 0.0 && e.limite_recurso("petroleo") == 0)
        .map(|e| e.bonus_cadeia(refinaria))
        .fold(0.0, f64::max);
    assert!(com > sem, "bônus com petróleo local ({com}) deve superar o sem ({sem})");

    // Mina só rende onde há depósito.
    let mina = &cat.tipos_predio[tipo("mina_ferro")];
    let sem_dep = eco.estados().iter().find(|e| e.limite_recurso("ferro") == 0).unwrap();
    let idx_mina = tipo("mina_ferro");
    let prov_sem = eco.provincias().iter().find(|p| p.estado_id == sem_dep.id).unwrap();
    assert!(!sim_economia::estado_comporta_novo_nivel(cat, sem_dep, prov_sem, idx_mina), "sem ferro não se constrói mina");
    let com_dep = eco.estados().iter().find(|e| e.limite_recurso("ferro") > 0).unwrap();
    assert!(com_dep.rendimento_recurso(mina) > 0.0);
}

#[test]
fn ia_poe_a_refinaria_onde_ha_petroleo() {
    let eco = mundo_aquecido();
    let cat = eco.catalogo();
    let refinaria = cat.tipos_predio.iter().position(|t| t.id == "refinaria_petroleo").unwrap();
    let pet = cat.bens.iter().position(|b| b.id == "petroleo").unwrap();
    let mut verificados = 0;
    for pais in eco.paises() {
        let est_pais: Vec<_> = eco.estados().iter().filter(|e| e.pais_id == pais.id).collect();
        let com = est_pais.iter().filter(|e| e.producao_local(pet) > 0.0).count();
        if com == 0 || com == est_pais.len() || est_pais.len() < 3 {
            continue;
        }
        let Some(prov_id) = IaEconomia::melhor_provincia_para(&eco, &pais.id, "refinaria_petroleo") else { continue };
        let prov = eco.obter_provincia(&prov_id).unwrap();
        let est = eco.obter_estado(&prov.estado_id).unwrap();
        eprintln!(
            "{}: refinaria em {} (bônus {:.1}%, petróleo local {:.1})",
            pais.id,
            est.id,
            est.bonus_cadeia(refinaria) * 100.0,
            est.producao_local(pet)
        );
        let melhor_bonus = est_pais.iter().map(|e| e.bonus_cadeia(refinaria)).fold(0.0, f64::max);
        assert!(est.bonus_cadeia(refinaria) >= melhor_bonus * 0.6, "{}: a IA escolheu um estado com pouca cadeia local", pais.id);
        verificados += 1;
    }
    assert!(verificados >= 1, "nenhum país adequado para o teste");
}
