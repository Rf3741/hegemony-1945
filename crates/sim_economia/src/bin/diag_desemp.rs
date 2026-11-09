use sim_economia::{CenarioEconomia, MundoEconomico};
use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
    let mut cen = CenarioEconomia::carregar_diretorio(&dir).unwrap();
    
    // Set pop.empregados = pop.populacao * 0.8696 for working pops in USA
    for prov in &mut cen.provincias {
        if prov.pais_id == "usa" {
            for pop in &mut prov.pops {
                if pop.tipo_pop_id != "camponeses" {
                    pop.empregados = pop.populacao * 0.8696;
                }
            }
        }
    }
    
    cen.config.modo_cadencia = sim_economia::ModoCadencia::DiarioComAjusteSemanal;
    let mut m = MundoEconomico::novo(cen).unwrap();

    println!("Day 0 taxa_desemprego: {:.2}%", m.taxa_desemprego_media_pais("usa").unwrap() * 100.0);
    for d in 1..=90 {
        m.executar_tick_diario();
        if d % 10 == 0 || d == 90 {
            println!("Day {} taxa_desemprego: {:.2}%", d, m.taxa_desemprego_media_pais("usa").unwrap() * 100.0);
        }
    }
}
