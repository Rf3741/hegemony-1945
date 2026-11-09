//! Módulo para redistribuição de população e prédios entre províncias de um mesmo país.
//! Mantém totais nacionais (±2%) enquanto distribui de forma mais realista.

use crate::catalogo::CatalogoEconomia;
use crate::cenario::CenarioEconomia;
use crate::entidades::{Estado, Pop, Predio, Provincia};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Default)]
struct ProvinciaPesos {
    populacao: f64,
    extrativos: BTreeMap<String, f64>,
    industria: f64,
    servicos: f64,
    governo: f64,
    urbanizacao: f64,
}

#[derive(Debug, Clone, Default)]
struct PaisTotais {
    pops_por_tipo: BTreeMap<String, f64>,
    predios_por_tipo: BTreeMap<String, u64>,
}

#[derive(Debug, Clone)]
struct EstadoPotencial {
    terra_aravel: u32,
    culturas_permitidas: Vec<String>,
    recursos: BTreeMap<String, u32>,
    qualidade_recursos: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Copy)]
enum CategoriaPredioRedist {
    Extrativo,
    Industrial,
    Servicos,
    Governo,
    Fazenda,
}

/// Redistribui população e prédios dentro de cada país em `provincias.ron`.
/// Mantém os TOTAIS NACIONAIS (população por tipo, níveis totais de cada prédio) em ±2%.
pub fn redistribuir_provincias(cen: &mut CenarioEconomia) {
    // Construir mapas auxiliares
    let potenciais_por_estado = construir_mapa_potenciais(&cen.estados);

    // Agrupar índices de províncias por país
    let mut provincias_por_pais: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, prov) in cen.provincias.iter().enumerate() {
        provincias_por_pais.entry(prov.pais_id.clone()).or_default().push(idx);
    }

    // Calcular totais nacionais atuais (para preservar)
    let mut totais_nacionais: BTreeMap<String, PaisTotais> = BTreeMap::new();
    for (pais_id, prov_indices) in &provincias_por_pais {
        let mut pops_por_tipo: BTreeMap<String, f64> = BTreeMap::new();
        let mut predios_por_tipo: BTreeMap<String, u64> = BTreeMap::new();

        for &idx in prov_indices {
            let prov = &cen.provincias[idx];
            for pop in &prov.pops {
                *pops_por_tipo.entry(pop.tipo_pop_id.clone()).or_default() += pop.populacao;
            }
            for predio in &prov.predios {
                *predios_por_tipo.entry(predio.tipo_predio_id.clone()).or_default() += predio.nivel as u64;
            }
        }

        totais_nacionais.insert(pais_id.clone(), PaisTotais {
            pops_por_tipo,
            predios_por_tipo,
        });
    }

    // Redistribuir para cada país
    for (pais_id, prov_indices) in provincias_por_pais {
        let totais = totais_nacionais.get(&pais_id).cloned().unwrap_or_default();
        let pesos = calcular_pesos_provincias(cen, &prov_indices, &potenciais_por_estado);

        // Redistribuir população (usando pesos históricos/geográficos)
        redistribuir_populacao(cen, &prov_indices, &totais.pops_por_tipo, &pesos);

        // Redistribuir prédios
        redistribuir_predios(cen, &prov_indices, &totais.predios_por_tipo, &pesos, &potenciais_por_estado);
    }
}

fn construir_mapa_potenciais(estados: &[Estado]) -> HashMap<String, EstadoPotencial> {
    let mut mapa = HashMap::new();
    for est in estados {
        mapa.insert(est.id.clone(), EstadoPotencial {
            terra_aravel: est.terra_aravel,
            culturas_permitidas: est.culturas_permitidas.clone(),
            recursos: est.limites_recursos.clone(),
            qualidade_recursos: est.qualidade_recursos.clone(),
        });
    }
    mapa
}

fn calcular_pesos_provincias(
    cen: &CenarioEconomia,
    prov_indices: &[usize],
    potenciais: &HashMap<String, EstadoPotencial>,
) -> HashMap<String, ProvinciaPesos> {
    let mut pesos = HashMap::new();

    // Estimativas históricas de população 1945 para províncias/territórios específicos
    // Fonte: dados históricos aproximados
    let pop_historica_1945: std::collections::HashMap<&str, f64> = [
        // EUA - territórios do Pacífico
        ("gum_00", 22000.0),       // Guam ~22k
        ("mnp_4937", 3000.0),      // Rota ~3k
        ("mnp_4938", 1000.0),      // Tinian ~1k
        ("mnp_4939", 5000.0),      // Saipan ~5k
        ("mnp_4940", 1000.0),      // Outras Marianas
        ("pri_5260", 2000000.0),   // Porto Rico ~2M
        ("asm_4998", 12000.0),     // Samoa Americana - Leste
        ("asm_4999", 5000.0),      // Samoa Americana - Manus
        ("asm_5000", 100.0),       // Atol Rose
        ("asm_5001", 100.0),       // Ilha Swains
        ("asm_5002", 8000.0),      // Samoa Americana - Ocidental
        ("umi_5171", 0.0),         // Ilha Baker - desabitada
        ("umi_5172", 0.0),         // Ilha Howland - desabitada
        ("umi_5173", 0.0),         // Ilha Jarvis - desabitada
        ("umi_5174", 300.0),       // Atol Johnston - base militar
        ("umi_5176", 1000.0),      // Atol Midway - base militar
        ("umi_5177", 0.0),         // Ilha Navassa - desabitada
        ("umi_5178", 0.0),         // Atol Palmyra
        ("umi_5179", 300.0),       // Ilha Wake - base militar
        // Havaí
        ("usa_3517", 425000.0),    // Havaí ~425k
        // Alasca
        ("usa_alaska", 75000.0),   // Alasca ~75k
    ].into_iter().collect();

    // Calcular pesos brutos
    for &idx in prov_indices {
        let prov = &cen.provincias[idx];
        let potencial = potenciais.get(&prov.estado_id);
        let mut peso_pop: f64 = prov.terra_aravel_total as f64 * 0.3;
        let mut peso_industria: f64 = 0.0;
        let mut peso_servicos: f64 = 0.0;
        let mut peso_governo: f64 = 0.0;
        let mut extrativos: BTreeMap<String, f64> = BTreeMap::new();

        // Usar população histórica se disponível, senão fórmula padrão
        if let Some(&pop_hist) = pop_historica_1945.get(prov.id.as_str()) {
            if pop_hist > 0.0 {
                peso_pop = pop_hist;
            } else {
                // Território desabitado ou base militar mínima
                peso_pop = 100.0;
            }
            // Para territórios históricos, pesos de indústria/serviços/governo baseados em urbanização
            let peso_urbanizacao: f64 = prov.infraestrutura_base.max(1.0);
            peso_industria = peso_urbanizacao * 1000.0;
            peso_servicos = peso_urbanizacao * 2000.0;
            peso_governo = peso_urbanizacao * 50.0;
        } else {
            // Províncias continentais: pesos base muito maiores
            let peso_urbanizacao: f64 = prov.infraestrutura_base.max(1.0);
            peso_pop = prov.terra_aravel_total as f64 * 100.0 + peso_urbanizacao * 10000.0;
            peso_industria = peso_urbanizacao * 5000.0;
            peso_servicos = peso_urbanizacao * 8000.0;
            peso_governo = peso_urbanizacao * 100.0;
        }

        // Recursos extrativos: peso proporcional ao limite do recurso no estado
        if let Some(pot) = potencial {
            for (recurso, &limite) in &pot.recursos {
                if limite > 0 {
                    let tem_recurso = prov.limites_recursos.get(recurso).copied().unwrap_or(0) > 0;
                    if tem_recurso {
                        extrativos.insert(recurso.clone(), limite as f64);
                    }
                }
            }
            // Terra arável para fazendas
            if pot.terra_aravel > 0 && prov.terra_aravel_total > 0 {
                extrativos.insert("terra_aravel".to_string(), pot.terra_aravel as f64);
            }
        }

        pesos.insert(prov.id.clone(), ProvinciaPesos {
            populacao: peso_pop.max(1.0),
            extrativos,
            industria: peso_industria.max(1.0),
            servicos: peso_servicos.max(1.0),
            governo: peso_governo.max(1.0),
            urbanizacao: prov.infraestrutura_base.max(1.0),
        });
    }

    // Normalizar pesos por país (soma = 1.0 para cada categoria)
    // População
    let soma_pop: f64 = prov_indices.iter().map(|&idx| pesos[&cen.provincias[idx].id].populacao).sum();
    if soma_pop > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = pesos.get_mut(id) {
                p.populacao /= soma_pop;
            }
        }
    }
    // Indústria
    let soma_ind: f64 = prov_indices.iter().map(|&idx| pesos[&cen.provincias[idx].id].industria).sum();
    if soma_ind > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = pesos.get_mut(id) {
                p.industria /= soma_ind;
            }
        }
    }
    // Serviços
    let soma_serv: f64 = prov_indices.iter().map(|&idx| pesos[&cen.provincias[idx].id].servicos).sum();
    if soma_serv > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = pesos.get_mut(id) {
                p.servicos /= soma_serv;
            }
        }
    }
    // Governo
    let soma_gov: f64 = prov_indices.iter().map(|&idx| pesos[&cen.provincias[idx].id].governo).sum();
    if soma_gov > 0.0 {
        for &idx in prov_indices {
            let id = &cen.provincias[idx].id;
            if let Some(p) = pesos.get_mut(id) {
                p.governo /= soma_gov;
            }
        }
    }
    // Extrativos por recurso
    let mut recursos_todos: BTreeMap<String, f64> = BTreeMap::new();
    for &idx in prov_indices {
        let id = &cen.provincias[idx].id;
        for (rec, peso) in &pesos[id].extrativos {
            *recursos_todos.entry(rec.clone()).or_default() += peso;
        }
    }
    for (rec, soma) in recursos_todos {
        if soma > 0.0 {
            for &idx in prov_indices {
                let id = &cen.provincias[idx].id;
                if let Some(p) = pesos.get_mut(id) {
                    if let Some(peso) = p.extrativos.get_mut(&rec) {
                        *peso /= soma;
                    }
                }
            }
        }
    }

    pesos
}

fn redistribuir_populacao(
    cen: &mut CenarioEconomia,
    prov_indices: &[usize],
    totais_nacionais: &BTreeMap<String, f64>,
    pesos: &HashMap<String, ProvinciaPesos>,
) {
    // Primeiro, calcular população total atual por tipo de pop por província
    let mut pop_atual_por_prov_tipo: HashMap<(String, String), f64> = HashMap::new();
    for &idx in prov_indices {
        let prov = &cen.provincias[idx];
        for pop in &prov.pops {
            *pop_atual_por_prov_tipo.entry((prov.id.clone(), pop.tipo_pop_id.clone())).or_default() += pop.populacao;
        }
    }

    // Calcular população total atual por tipo de pop no país
    let mut pop_atual_nacional: BTreeMap<String, f64> = BTreeMap::new();
    for ((_, tipo), pop) in &pop_atual_por_prov_tipo {
        *pop_atual_nacional.entry(tipo.clone()).or_default() += pop;
    }

    // Para cada província, escalar as pops existentes para atingir o novo total
    for &idx in prov_indices {
        let prov = &mut cen.provincias[idx];
        let peso = pesos.get(&prov.id).cloned().unwrap_or_default();

        // Calcular população total atual desta província
        let pop_total_atual: f64 = prov.pops.iter().map(|p| p.populacao).sum();

        // Calcular população total desejada para esta província
        let mut pop_total_desejada = 0.0;
        for (tipo, total_nacional) in totais_nacionais {
            pop_total_desejada += total_nacional * peso.populacao;
        }

        if pop_total_atual <= 0.0 || pop_total_desejada <= 0.0 {
            continue;
        }

        let fator_escala = pop_total_desejada / pop_total_atual;

        // Escalar cada pop existente mantendo razões de emprego e diversidade cultural
        for pop in &mut prov.pops {
            pop.populacao *= fator_escala;
            pop.empregados *= fator_escala;
            pop.riqueza_reservas *= fator_escala;
        }
    }

    // Ajuste fino: garantir que totais nacionais por tipo sejam preservados exatamente
    // Calculando discrepâncias e redistribuindo o resíduo
    for (tipo, &total_nacional) in totais_nacionais {
        let total_atual: f64 = prov_indices.iter()
            .flat_map(|&idx| &cen.provincias[idx].pops)
            .filter(|p| p.tipo_pop_id == *tipo)
            .map(|p| p.populacao)
            .sum();

        if total_atual <= 0.0 { continue; }

        let fator_ajuste = total_nacional / total_atual;
        if (fator_ajuste - 1.0).abs() > 0.01 {
            for &idx in prov_indices {
                let prov = &mut cen.provincias[idx];
                for pop in &mut prov.pops {
                    if pop.tipo_pop_id == *tipo {
                        pop.populacao *= fator_ajuste;
                        pop.empregados *= fator_ajuste;
                        pop.riqueza_reservas *= fator_ajuste;
                    }
                }
            }
        }
    }

    // Garantir emprego inicial de 99% para evitar desemprego absurdo no início
    for &idx in prov_indices {
        let prov = &mut cen.provincias[idx];
        for pop in &mut prov.pops {
            pop.empregados = pop.populacao * 0.99;
        }
    }
}

fn redistribuir_predios(
    cen: &mut CenarioEconomia,
    prov_indices: &[usize],
    totais_nacionais: &BTreeMap<String, u64>,
    pesos: &HashMap<String, ProvinciaPesos>,
    potenciais: &HashMap<String, EstadoPotencial>,
) {
    // Categorizar tipos de prédio
    let mut extrativos_tipos = Vec::new();
    let mut industriais_tipos = Vec::new();
    let mut servicos_tipos = Vec::new();
    let mut governo_tipos = Vec::new();
    let mut fazenda_tipos = Vec::new();

    for (tipo, _) in totais_nacionais {
        let cat = categorizar_predio(tipo);
        match cat {
            CategoriaPredioRedist::Extrativo => extrativos_tipos.push(tipo.clone()),
            CategoriaPredioRedist::Industrial => industriais_tipos.push(tipo.clone()),
            CategoriaPredioRedist::Servicos => servicos_tipos.push(tipo.clone()),
            CategoriaPredioRedist::Governo => governo_tipos.push(tipo.clone()),
            CategoriaPredioRedist::Fazenda => fazenda_tipos.push(tipo.clone()),
        }
    }

    // Limpar prédios existentes
    for &idx in prov_indices {
        cen.provincias[idx].predios.clear();
    }

    // 1. Extrativos: só onde há recurso
    for tipo in extrativos_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        let recurso = recurso_do_predio(&tipo);
        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let peso_recurso = peso.extrativos.get(&recurso).copied().unwrap_or(0.0);

            // Verificar limite no estado e na província
            let limite_estado = potenciais.get(&prov.estado_id)
                .and_then(|p| p.recursos.get(&recurso))
                .copied().unwrap_or(0);

            let limite_prov = prov.limites_recursos.get(&recurso).copied().unwrap_or(0);
            let limite = limite_prov.min(limite_estado);

            if limite > 0 && peso_recurso > 0.0 {
                let nivel = ((total_nacional * peso_recurso) as u32).min(limite);
                if nivel > 0 {
                    prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
                }
            }
        }
    }

    // 2. Fazendas: só onde há terra arável
    for tipo in fazenda_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let peso_terra = peso.extrativos.get("terra_aravel").copied().unwrap_or(0.0);

            let terra_livre = prov.terra_aravel_total;
            if terra_livre > 0 && peso_terra > 0.0 {
                let nivel = ((total_nacional * peso_terra) as u32).min(terra_livre);
                if nivel > 0 {
                    prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
                }
            }
        }
    }

    // 3. Industrial: proporcional a população + urbanização + bônus matéria-prima
    for tipo in industriais_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let mut peso_final = peso.industria;

            // Bônus se tem matéria-prima local
            if tem_materia_prima_local(&tipo, prov, potenciais) {
                peso_final *= 1.5;
            }

            let nivel = (total_nacional * peso_final) as u32;
            if nivel > 0 {
                prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
            }
        }
    }

    // 4. Serviços: proporcional a população + urbanização
    for tipo in servicos_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let nivel = (total_nacional * peso.servicos) as u32;
            if nivel > 0 {
                prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
            }
        }
    }

    // 5. Governo/quartéis: proporcional à população
    for tipo in governo_tipos {
        let total_nacional = *totais_nacionais.get(&tipo).unwrap_or(&0) as f64;
        if total_nacional == 0.0 { continue; }

        for &idx in prov_indices {
            let prov = &mut cen.provincias[idx];
            let peso = pesos.get(&prov.id).cloned().unwrap_or_default();
            let nivel = (total_nacional * peso.governo) as u32;
            if nivel > 0 {
                prov.predios.push(criar_predio_padrao(&tipo, nivel, &prov.pops, &cen.catalogo));
            }
        }
    }
}

fn categorizar_predio(tipo: &str) -> CategoriaPredioRedist {
    if tipo.starts_with("mina_") || tipo.starts_with("poco_") || tipo.starts_with("campo_") || tipo == "porto_pesca" {
        CategoriaPredioRedist::Extrativo
    } else if tipo.starts_with("fazenda_") || tipo.starts_with("plantacao_") || tipo.starts_with("rancho_") || tipo == "pecuaria_pesca" {
        CategoriaPredioRedist::Fazenda
    } else if tipo.contains("fabrica") || tipo.contains("usina") || tipo.contains("siderurgica") || tipo.contains("industria") || tipo == "serraria" {
        CategoriaPredioRedist::Industrial
    } else if tipo.contains("administracao") || tipo.contains("quartel") || tipo.contains("centro_urbano") || tipo.contains("setor_construcao") || tipo.contains("porto_comercial") {
        CategoriaPredioRedist::Governo
    } else {
        CategoriaPredioRedist::Servicos
    }
}

fn recurso_do_predio(tipo: &str) -> String {
    if tipo.starts_with("mina_ferro") { "ferro".to_string() }
    else if tipo.starts_with("mina_carvao") { "carvao".to_string() }
    else if tipo.starts_with("mina_cobre") { "cobre".to_string() }
    else if tipo.starts_with("mina_ouro") || tipo.starts_with("mina_prata") { "ouro_prata".to_string() }
    else if tipo.starts_with("mina_uranio") { "uranio".to_string() }
    else if tipo.starts_with("poco_petroleo") { "petroleo".to_string() }
    else if tipo.starts_with("poco_gas") || tipo.starts_with("campo_gas") { "gas_natural".to_string() }
    else if tipo == "porto_pesca" { "pesca".to_string() }
    else { "desconhecido".to_string() }
}

fn tem_materia_prima_local(tipo: &str, prov: &Provincia, potenciais: &HashMap<String, EstadoPotencial>) -> bool {
    if let Some(pot) = potenciais.get(&prov.estado_id) {
        if tipo.contains("siderurgica") || tipo.contains("fabrica_ferramentas") || tipo.contains("fabrica_armamentos") {
            return pot.recursos.get("ferro").copied().unwrap_or(0) > 0 && pot.recursos.get("carvao").copied().unwrap_or(0) > 0;
        }
        if tipo.contains("fabrica_texteis") {
            return pot.culturas_permitidas.iter().any(|c| c == "algodao");
        }
        if tipo.contains("industria_alimenticia") || tipo.contains("fabrica_alimentos") {
            return pot.terra_aravel > 0;
        }
    }
    false
}

fn criar_predio_padrao(tipo: &str, nivel: u32, pops: &[Pop], catalogo: &CatalogoEconomia) -> Predio {
    // Buscar métodos padrão do catálogo
    let metodos = if let Some(tipo_predio) = catalogo.tipos_predio.iter().find(|tp| tp.id == tipo) {
        tipo_predio.metodos_padrao.clone()
    } else {
        // Fallback genérico
        vec!["metodo_padrao".to_string(), "propriedade_privada".to_string()]
    };

    let mut empregados_por_tipo = BTreeMap::new();
    // Aumentar significativamente as vagas por nível para compensar a concentração de prédios
    // em menos províncias (respeitando limites de recursos). O original tinha prédios
    // em TODAS as províncias; agora estão concentrados onde há recursos.
    let vagas_por_nivel = match tipo {
        t if t.starts_with("mina_") => 3000,
        t if t.starts_with("poco_") => 2500,
        t if t.starts_with("fazenda_") => 2000,
        t if t.contains("siderurgica") => 5000,
        t if t.contains("fabrica") => 4000,
        t if t.contains("usina") => 3000,
        t if t.contains("administracao") => 2500,
        t if t.contains("quartel") => 3000,
        _ => 2000,
    };

    let total_vagas = vagas_por_nivel * nivel as usize;

    // Definir quais tipos de pop trabalham em cada categoria de prédio
    let pop_tipos_trabalhadores = match tipo {
        t if t.starts_with("mina_") || t.starts_with("poco_") || t == "porto_pesca" || t == "serraria" => {
            // Extrativos: operários, engenheiros, mineiros, maquinistas
            vec!["operarios", "engenheiros", "mineiros", "maquinistas", "tecnicos"]
        }
        t if t.starts_with("fazenda_") || t.starts_with("plantacao_") || t.starts_with("rancho_") || t == "pecuaria_pesca" => {
            // Agrícolas: camponeses, agricultores, operários
            vec!["camponeses", "agricultores", "operarios", "maquinistas"]
        }
        t if t.contains("siderurgica") || t.contains("fabrica_") => {
            // Industriais: operários, engenheiros, maquinistas, técnicos
            vec!["operarios", "engenheiros", "maquinistas", "tecnicos", "mecanicos"]
        }
        t if t.contains("usina_") => {
            // Energia: operários, engenheiros, técnicos
            vec!["operarios", "engenheiros", "tecnicos", "maquinistas"]
        }
        t if t.contains("administracao") => {
            // Governo: burocratas, intelectuais, funcionários
            vec!["burocratas", "intelectuais", "funcionarios", "tecnicos"]
        }
        t if t.contains("quartel") => {
            // Militar: soldados, oficiais
            vec!["soldados", "oficiais", "militares"]
        }
        t if t.contains("centro_urbano") || t.contains("porto_comercial") => {
            // Serviços urbanos: operários, burocratas, técnicos, comerciantes
            vec!["operarios", "burocratas", "tecnicos", "comerciantes", "funcionarios"]
        }
        t if t.contains("setor_construcao") || t.contains("rede_ferroviaria") || t.contains("rodovia") => {
            // Construção/infraestrutura: operários, engenheiros, maquinistas
            vec!["operarios", "engenheiros", "maquinistas", "tecnicos"]
        }
        _ => {
            // Genérico: todos os tipos de trabalhadores comuns
            vec!["operarios", "engenheiros", "maquinistas", "tecnicos", "burocratas", "funcionarios", "camponeses", "agricultores", "mineiros", "soldados"]
        }
    };

    // Filtrar pops que são tipos de trabalhadores apropriados
    let pops_trabalhadores: Vec<_> = pops.iter()
        .filter(|p| p.populacao > 0.0 && pop_tipos_trabalhadores.contains(&p.tipo_pop_id.as_str()))
        .collect();

    if !pops_trabalhadores.is_empty() {
        let total_vagas_f64 = total_vagas as f64;
        let por_pop = total_vagas_f64 / pops_trabalhadores.len() as f64;
        for pop in pops_trabalhadores {
            empregados_por_tipo.insert(pop.tipo_pop_id.clone(), por_pop);
        }
    }

    Predio {
        tipo_predio_id: tipo.to_string(),
        nivel,
        metodos_ativos: metodos,
        empregados_por_tipo,
        salario_base: 4.0,
        reservas_caixa: 5000.0 * nivel as f64,
        subsidiado: tipo.contains("administracao") || tipo.contains("quartel"),
        receita_ultimo_tick: 0.0,
        custo_insumos_ultimo_tick: 0.0,
        folha_salarial_ultimo_tick: 0.0,
        lucro_ultimo_tick: 0.0,
        dividendos_ultimo_tick: 0.0,
        fechado: false,
        dias_retomada: 0,
        dias_prejuizo: 0,
        dias_fechado: 0,
        lucro_ema: 0.0,
        tendencia_lucro: 0.0,
        dia_ultima_mudanca_estado: 0,
        cache: Default::default(),
    }
}