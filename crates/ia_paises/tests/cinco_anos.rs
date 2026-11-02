use ia_paises::{IaEconomia, IaPolitica};
use sim_economia::{CenarioEconomia, ModoCadencia, MundoEconomico};
use sim_politica::{CenarioPolitica, MundoPolitico};
use std::path::PathBuf;

fn raiz() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn economia() -> MundoEconomico {
    let mut cen = CenarioEconomia::carregar_diretorio(&raiz().join("assets/data/economia")).expect("economia");
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    MundoEconomico::novo(cen).expect("mundo economia")
}

fn politica() -> MundoPolitico {
    let cen = CenarioPolitica::carregar_diretorio(&raiz().join("assets/data/politica")).expect("politica");
    MundoPolitico::novo(cen.catalogo, cen.paises, 1945).expect("mundo politico")
}

fn em_calote(m: &MundoEconomico) -> usize {
    m.paises().iter().filter(|p| p.orcamento.em_calote).count()
}

/// Roda 5 anos; `ia` liga ou desliga a IA. Devolve (calotes, pico, leis, propostas, obras, ajustes, crescimento do PIB).
fn rodar(ia: bool) -> (usize, usize, usize, u64, u64, u64, f64) {
    let mut eco = economia();
    let mut pol = politica();
    let mut ie = IaEconomia::novo();
    let mut ip = IaPolitica::novo();
    for _ in 0..90 {
        eco.executar_tick_diario();
    }
    let pib_inicial: f64 = eco.paises().iter().map(|p| p.pib_diario).sum();
    let leis_ini: Vec<_> = pol.paises().iter().map(|p| p.leis_ativas.clone()).collect();
    let mut pico = 0;
    for dia in 0..(365 * 5 + 1) {
        eco.executar_tick_diario();
        pol.executar_tick_diario();
        if ia {
            ie.passo_diario(&mut eco, &[]);
            ip.passo_diario(&mut pol, &[]);
        }
        if dia % 30 == 0 {
            pico = pico.max(em_calote(&eco));
        }
    }
    let mudadas = pol.paises().iter().zip(&leis_ini).filter(|(p, l)| &p.leis_ativas != *l).count();
    let n = eco.paises().len();
    eprintln!(
        "IA={ia}: paises={n}, calote no fim={}, pico={pico}, paises com lei mudada={mudadas}, propostas={}, obras estatais IA={}, ajustes fiscais={}",
        em_calote(&eco), ip.propostas, ie.obras_enfileiradas, ie.ajustes_fiscais
    );
    eco.auditar_estabilidade().expect("estabilidade da economia");
    let pib_final: f64 = eco.paises().iter().map(|p| p.pib_diario).sum();
    let crescimento = pib_final / pib_inicial - 1.0;
    eprintln!("IA={ia}: crescimento do PIB mundial em 5 anos = {:.1}%", crescimento * 100.0);
    (em_calote(&eco), pico, mudadas, ip.propostas, ie.obras_enfileiradas, ie.ajustes_fiscais, crescimento)
}

#[test]
fn cinco_anos_com_ia_sem_falencia_em_massa_e_com_mudanca_de_leis() {
    let (sem_fim, _, _, _, _, _, _) = rodar(false);
    let (fim, pico, mudadas, propostas, obras, ajustes, crescimento) = rodar(true);
    let n = economia().paises().len();
    assert!(fim <= sem_fim, "a IA nao pode piorar os calotes ({fim} > {sem_fim})");
    assert!(pico * 10 <= n, "falencia em massa: pico {pico} de {n}");
    assert!(propostas > 0 && mudadas > 0, "nenhuma lei mudou ({propostas} propostas, {mudadas} paises)");
    assert!(obras + ajustes > 0, "IA economica inativa");
    assert!(crescimento >= 0.03, "PIB mundial não cresceu em 5 anos: {:.1}%", crescimento * 100.0);
}
