//! Testes da IA de países e setor privado para fechamento, reabertura e demolição com histerese.

use ia_paises::IaEconomia;
use sim_economia::{CenarioEconomia, ModoCadencia, MundoEconomico};
use std::path::PathBuf;

fn dir_fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sim_economia/tests/fixtures")
}

#[test]
fn ia_fecha_predio_com_prejuizo_persistente_respeita_histerese_e_reabre_quando_compensa() {
    let mut cenario = CenarioEconomia::carregar_diretorio(&dir_fixtures()).expect("fixtures");
    cenario.config.modo_cadencia = ModoCadencia::DiarioCompleto;

    // Configura a mina_ferro em Liège com salário inflado e sem reservas, gerando prejuízo persistente
    let prov_liege = cenario
        .provincias
        .iter_mut()
        .find(|p| p.id == "liege")
        .expect("liege");
    let mina = prov_liege
        .predios
        .iter_mut()
        .find(|b| b.tipo_predio_id == "mina_ferro")
        .expect("mina_ferro");
    mina.salario_base = 28.0;
    mina.reservas_caixa = 0.0;

    let mut mundo = MundoEconomico::novo(cenario).expect("mundo");
    let mut ia = IaEconomia::novo();

    // 1. Antes do limiar de prejuízo persistente (ex.: 20 dias), a IA NÃO deve fechar o prédio (histerese)
    for _ in 0..20 {
        if let Some(pr) = mundo.obter_provincia_mut("liege")
            && let Some(b) = pr.predios.iter_mut().find(|b| b.tipo_predio_id == "mina_ferro")
        {
            b.salario_base = 28.0;
            b.reservas_caixa = 0.0;
        }
        mundo.executar_tick_diario();
    }
    ia.gerir_predios_pais(&mut mundo, "belgica");
    let mina_cedo = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_ferro")
        .unwrap();
    assert!(
        !mina_cedo.fechado,
        "histerese: não deve fechar prédio com poucos dias de prejuízo ({})",
        mina_cedo.dias_prejuizo
    );

    // 2. Após prejuízo persistente (>= 75 dias, dia 80 antes do passo mensal privado do dia 90), a IA fecha o prédio
    for _ in 0..60 {
        if let Some(pr) = mundo.obter_provincia_mut("liege")
            && let Some(b) = pr.predios.iter_mut().find(|b| b.tipo_predio_id == "mina_ferro")
        {
            b.salario_base = 28.0;
            b.reservas_caixa = 0.0;
        }
        mundo.executar_tick_diario();
    }
    ia.gerir_predios_pais(&mut mundo, "belgica");

    let mina_fechada = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_ferro")
        .unwrap();
    assert!(
        mina_fechada.fechado,
        "IA deve fechar prédio após prejuízo persistente"
    );
    assert!(ia.predios_fechados >= 1);

    // 3. Histerese pós-fechamento: mesmo que fique lucrativo no dia seguinte, não reabre antes do cooldown
    if let Some(pr) = mundo.obter_provincia_mut("liege")
        && let Some(b) = pr.predios.iter_mut().find(|b| b.tipo_predio_id == "mina_ferro")
    {
        b.salario_base = 2.0;
    }
    mundo.avancar_dias(15);
    ia.gerir_predios_pais(&mut mundo, "belgica");
    let mina_cooldown = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_ferro")
        .unwrap();
    assert!(
        mina_cooldown.fechado,
        "histerese: não deve reabrir imediatamente durante o período de cooldown"
    );

    // 4. Passado o período de histerese (>= 120 dias) e com margem positiva, a IA reabre o prédio
    for _ in 0..115 {
        if let Some(p) = mundo.obter_pais_mut("belgica") {
            p.fundo_investimento_privado = 0.0;
        }
        mundo.executar_tick_diario();
    }
    if let Some(p) = mundo.obter_pais_mut("belgica") {
        p.fundo_investimento_privado = 50_000.0;
        p.orcamento.tesouro = 50_000.0;
    }
    ia.gerir_predios_pais(&mut mundo, "belgica");

    let mina_reaberta = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_ferro")
        .unwrap();
    assert!(!mina_reaberta.fechado, "IA deve reabrir prédio quando volta a ser sólido após histerese");
    assert!(mina_reaberta.esta_em_retomada());
    assert!(ia.predios_reabertos >= 1);
}

#[test]
fn setor_privado_fecha_e_ia_demole_apos_fechamento_prolongado_sem_retorno() {
    let mut cenario = CenarioEconomia::carregar_diretorio(&dir_fixtures()).expect("fixtures");
    cenario.config.modo_cadencia = ModoCadencia::DiarioCompleto;

    let mut mundo = MundoEconomico::novo(cenario).expect("mundo");
    let mut ia = IaEconomia::novo();

    // 1. Setor privado fecha automaticamente no passo mensal após >= 75 dias de prejuízo
    for _ in 0..95 {
        if let Some(pr) = mundo.obter_provincia_mut("liege")
            && let Some(b) = pr.predios.iter_mut().find(|b| b.tipo_predio_id == "mina_ferro")
        {
            b.salario_base = 30.0;
            b.reservas_caixa = 0.0;
        }
        mundo.executar_tick_diario();
    }

    let (fechado_priv, nivel_antes) = {
        let b = mundo
            .obter_provincia("liege")
            .unwrap()
            .predios
            .iter()
            .find(|b| b.tipo_predio_id == "mina_ferro")
            .unwrap();
        (b.fechado, b.nivel)
    };
    assert!(
        fechado_priv,
        "setor privado deve fechar automaticamente prédio deficitário persistente"
    );

    // 2. Mantém fechado e inviável por mais de 270 dias -> IA demole 1 nível para liberar espaço
    for _ in 0..280 {
        if let Some(pr) = mundo.obter_provincia_mut("liege")
            && let Some(b) = pr.predios.iter_mut().find(|b| b.tipo_predio_id == "mina_ferro")
        {
            b.salario_base = 35.0;
        }
        mundo.executar_tick_diario();
    }
    ia.gerir_predios_pais(&mut mundo, "belgica");

    let nivel_depois = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_ferro")
        .map_or(0, |b| b.nivel);
    assert!(
        nivel_depois < nivel_antes,
        "prédio fechado sem viabilidade por >270 dias deve perder nível por demolição ({nivel_antes} -> {nivel_depois})"
    );
    mundo.auditar_estabilidade().expect("estabilidade intacta");
}
