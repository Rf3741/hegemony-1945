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

    // Mina pode ser construída mesmo sem depósito, mas com rendimento muito inferior.
    let mina = &cat.tipos_predio[tipo("mina_ferro")];
    let sem_dep = eco.estados().iter().find(|e| e.limite_recurso("ferro") == 0).unwrap();
    let idx_mina = tipo("mina_ferro");
    let prov_sem = eco.provincias().iter().find(|p| p.estado_id == sem_dep.id).unwrap();
    assert!(sim_economia::estado_comporta_novo_nivel(cat, sem_dep, prov_sem, idx_mina), "construções liberadas mesmo sem jazida");
    let com_dep = eco.estados().iter().find(|e| e.limite_recurso("ferro") > 0).unwrap();
    let rend_com = com_dep.rendimento_recurso(mina);
    let rend_sem = sem_dep.rendimento_recurso(mina);
    assert!(rend_com >= rend_sem * 2.0, "região com jazida deve render muito mais ({rend_com}) que sem ({rend_sem})");
    assert!(rend_sem > 0.0, "sem jazida rende pouco, mas não zero ({rend_sem})");
}

/// Nos dados de 1945 o petróleo está espalhado por todos os estados dos países produtores, então a
/// preferência é verificada em qualquer processador industrial cujo bônus de cadeia varia dentro de um
/// país (refinaria, siderúrgica, móveis, alimentos...): a IA deve escolher um estado de alta cadeia local.
#[test]
fn ia_processa_onde_extrai() {
    let eco = mundo_aquecido();
    let cat = eco.catalogo();
    let mut verificados = 0;
    let mut refinarias = 0;
    let mut acima = 0;
    for (ti, t) in cat.tipos_predio.iter().enumerate() {
        if t.categoria != sim_economia::CategoriaPredio::Industrial || t.financiado_pelo_governo {
            continue;
        }
        for pais in eco.paises() {
            let est_pais: Vec<_> = eco.estados().iter().filter(|e| e.pais_id == pais.id).collect();
            let bonus: Vec<f64> = est_pais.iter().map(|e| e.bonus_cadeia(ti)).collect();
            let (max, min) = (bonus.iter().cloned().fold(0.0, f64::max), bonus.iter().cloned().fold(1.0, f64::min));
            if est_pais.len() < 3 || max < min + 0.06 {
                continue;
            }
            let Some(prov_id) = IaEconomia::melhor_provincia_para(&eco, &pais.id, &t.id) else { continue };
            let prov = eco.obter_provincia(&prov_id).unwrap();
            let est = eco.obter_estado(&prov.estado_id).unwrap();
            let media = bonus.iter().sum::<f64>() / bonus.len() as f64;
            if est.bonus_cadeia(ti) >= media {
                acima += 1;
            }
            eprintln!("{} / {}: IA escolheu {} (+{:.1}%); média +{:.1}%, máx +{:.1}%", pais.id, t.id, est.id, est.bonus_cadeia(ti) * 100.0, media * 100.0, max * 100.0);
            verificados += 1;
            if t.id == "refinaria_petroleo" {
                refinarias += 1;
            }
        }
    }
    eprintln!("verificados {verificados} (país, tipo); refinarias {refinarias}");
    assert!(verificados >= 3, "poucos casos verificados: {verificados}");
    // Outros fatores (infraestrutura, mão de obra, salário) também pesam; ainda assim a maioria das
    // escolhas deve cair em estados com cadeia local acima da média do país.
    assert!(acima * 10 >= verificados * 6, "a IA só escolheu cadeia acima da média em {acima}/{verificados} casos");
}
