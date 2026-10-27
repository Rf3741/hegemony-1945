use sim_economia::{CenarioEconomia, MercadoGlobal, ModoCadencia, MundoEconomico};
use std::path::Path;
use std::time::Instant;

fn obter_memoria_kb() -> (u64, u64) {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return (0, 0);
    };
    let mut rss = 0;
    let mut hwm = 0;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            if let Some(val) = line.split_whitespace().nth(1) {
                rss = val.parse().unwrap_or(0);
            }
        } else if line.starts_with("VmHWM:") {
            if let Some(val) = line.split_whitespace().nth(1) {
                hwm = val.parse().unwrap_or(0);
            }
        }
    }
    (rss, hwm)
}

fn main() {
    let (rss_inicio, hwm_inicio) = obter_memoria_kb();
    println!("=== MEDIÇÃO DE DESEMPENHO E MEMÓRIA (CENÁRIO 1945) ===");
    println!("Início: RSS = {} MB, Pico (HWM) = {} MB", rss_inicio / 1024, hwm_inicio / 1024);

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
    let t_carga = Instant::now();
    let mut cen = CenarioEconomia::carregar_diretorio(&dir).expect("carregar cenário");
    cen.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;
    let dur_leitura = t_carga.elapsed();

    let (rss_pos_ron, hwm_pos_ron) = obter_memoria_kb();
    println!("1. Leitura RON: RSS = {} MB, Pico = {} MB (tempo: {:?})", rss_pos_ron / 1024, hwm_pos_ron / 1024, dur_leitura);

    let t_mundo = Instant::now();
    let mut mundo = MundoEconomico::novo(cen).expect("mundo economico");
    let dur_mundo = t_mundo.elapsed();

    let (rss_mundo, hwm_mundo) = obter_memoria_kb();
    println!("2. Inicialização MundoEconomico: RSS = {} MB, Pico = {} MB (tempo: {:?})", rss_mundo / 1024, hwm_mundo / 1024, dur_mundo);
    println!("   Total províncias: {}, estados: {}, países: {}, mercados: {}",
        mundo.provincias().len(), mundo.estados().len(), mundo.paises().len(), mundo.mercados().len());

    let mut global = MercadoGlobal::novo(&mundo);
    let m_usa = mundo.indice_mercado_do_pais("usa");
    global.acompanhar_mercado(m_usa);

    // Aquecimento de 90 dias
    let t_aquece = Instant::now();
    for _ in 0..90 {
        mundo.executar_tick_diario();
        mundo.atualizar_mercado_global(&mut global);
    }
    let dur_aquece = t_aquece.elapsed();
    let (rss_aquece, hwm_aquece) = obter_memoria_kb();
    println!("3. Aquecimento 90 dias: RSS = {} MB, Pico = {} MB (tempo: {:?}, {:.2} ms/dia)",
        rss_aquece / 1024, hwm_aquece / 1024, dur_aquece, dur_aquece.as_secs_f64() * 1000.0 / 90.0);

    // Medição de 5 anos (1826 dias)
    let total_dias = 1826u32;
    let t_sim = Instant::now();
    let mut max_tick = std::time::Duration::ZERO;

    for dia in 1..=total_dias {
        let t0 = Instant::now();
        let _rel = mundo.executar_tick_diario();
        mundo.atualizar_mercado_global(&mut global);
        let dt = t0.elapsed();
        if dt > max_tick {
            max_tick = dt;
        }

        if dia % 365 == 0 || dia == total_dias {
            let ano = dia / 365;
            let (rss, hwm) = obter_memoria_kb();
            let decorrido = t_sim.elapsed();
            let medio = decorrido.as_secs_f64() * 1000.0 / dia as f64;
            println!("   Ano {} (dia {}/{}): RSS = {} MB, Pico = {} MB, Tick Médio = {:.2} ms/dia",
                ano, dia, total_dias, rss / 1024, hwm / 1024, medio);
        }
    }

    let dur_sim = t_sim.elapsed();
    let (rss_fim, hwm_fim) = obter_memoria_kb();
    let ms_por_dia = dur_sim.as_secs_f64() * 1000.0 / total_dias as f64;
    println!("4. Fim 5 anos (1826 dias): RSS = {} MB, Pico = {} MB", rss_fim / 1024, hwm_fim / 1024);
    println!("   Tempo total: {:?}, Tick Médio: {:.2} ms/dia, Tick Máximo: {:?}", dur_sim, ms_por_dia, max_tick);

    if let Some(p) = mundo.obter_pais("usa") {
        let sol = mundo.padrao_de_vida_medio_pais("usa").unwrap_or(0.0);
        println!("EUA: PIB diário = {:.2}, tesouro = {:.2}, SoL = {:.1}", p.pib_diario, p.orcamento.tesouro, sol);
    }
    if let Some(p) = mundo.obter_pais("sov") {
        let sol = mundo.padrao_de_vida_medio_pais("sov").unwrap_or(0.0);
        println!("URSS: PIB diário = {:.2}, tesouro = {:.2}, SoL = {:.1}", p.pib_diario, p.orcamento.tesouro, sol);
    }
}
