//! Cálculo e progressão das 5 eras tecnológicas visuais por país e por estado:
//! 1945, 1955, 1965, 1975 e 1985.

use crate::catalogo::CatalogoTerritorio;
use crate::infraestrutura::RedeInfraestruturaEstado;
use crate::tipos::{EraTecnologicaVisual, EstiloArquitetonico};
use std::collections::{BTreeMap, BTreeSet};

/// Calcula a era tecnológica visual nacional de um país.
/// Retorna `(era_visual, indice_modernidade, era_teto_tecnologico)`.
pub fn calcular_era_visual_pais(
    catalogo: &CatalogoTerritorio,
    tecnologias_desbloqueadas: &BTreeSet<String>,
    metodos_totais_pais: &BTreeMap<String, u32>,
) -> (EraTecnologicaVisual, f64, EraTecnologicaVisual) {
    // 1. Teto e pontuação tecnológica obtida pelas pesquisas concluídas
    let mut contagem_por_era = [0u32; 5];
    let mut total_por_era = [0u32; 5];

    for t in &catalogo.tecnologias_eras {
        let idx = t.era.indice();
        total_por_era[idx] += 1;
        if tecnologias_desbloqueadas.contains(&t.id) {
            contagem_por_era[idx] += 1;
        }
    }

    // Normaliza pontuação das tecnologias (0.0 a 100.0)
    let peso_total_tec: f64 = catalogo
        .tecnologias_eras
        .iter()
        .map(|t| t.peso_visual)
        .sum();
    let pontuacao_tec_norm = if peso_total_tec > 0.0 {
        // Teto de referência ponderado
        let mut soma_ativa = 0.0;
        for t in &catalogo.tecnologias_eras {
            if tecnologias_desbloqueadas.contains(&t.id) {
                soma_ativa += (t.era.indice() as f64) * 25.0 * t.peso_visual;
            }
        }
        (soma_ativa / (peso_total_tec * 0.75)).clamp(0.0, 100.0)
    } else {
        0.0
    };

    // Teto tecnológico: um país só pode exibir visualmente uma era se tiver ao menos 1 tecnologia dela
    let mut teto_idx = 0;
    for (idx, &cnt) in contagem_por_era.iter().enumerate() {
        if cnt > 0 {
            teto_idx = idx;
        }
    }
    let era_teto = EraTecnologicaVisual::a_partir_indice(teto_idx);

    // 2. Pontuação derivada da adoção real de métodos de produção nos prédios do país
    let mut soma_metodos = 0.0;
    let mut peso_metodos = 0.0;

    for (metodo_id, &qtd_niveis) in metodos_totais_pais {
        if qtd_niveis == 0 {
            continue;
        }
        let qtd_f = qtd_niveis as f64;
        if let Some(def_met) = catalogo.buscar_metodo(metodo_id) {
            let valor_era = (def_met.era_minima.indice() as f64) * 25.0;
            soma_metodos += valor_era * qtd_f * def_met.peso_adocao;
            peso_metodos += qtd_f * def_met.peso_adocao;
        }
    }

    let pontuacao_metodos_norm = if peso_metodos > 0.0 {
        (soma_metodos / peso_metodos).clamp(0.0, 100.0)
    } else {
        pontuacao_tec_norm * 0.75
    };

    // 3. Índice integrado de modernidade do país (55% tecnologias, 45% métodos adotados)
    let mut indice_modernidade = 0.55 * pontuacao_tec_norm + 0.45 * pontuacao_metodos_norm;

    // Limita o índice ao teto máximo da tecnologia pesquisada
    let teto_valor = ((teto_idx as f64) * 25.0 + 24.0).clamp(0.0, 100.0);
    indice_modernidade = indice_modernidade.min(teto_valor);

    let era_visual = converter_indice_em_era(indice_modernidade, era_teto);
    (era_visual, indice_modernidade, era_teto)
}

/// Calcula a era tecnológica visual local de um estado individual.
/// Permite que estados industriais de ponta (ex: São Paulo, Califórnia, Ruhr) avancem visualmente
/// antes de áreas rurais atrasadas do mesmo país, respeitando o teto de tecnologias do país soberano.
pub fn calcular_era_visual_estado(
    catalogo: &CatalogoTerritorio,
    metodos_ativos_estado: &BTreeMap<String, u32>,
    infra: &RedeInfraestruturaEstado,
    indice_modernidade_pais: f64,
    era_teto_pais: EraTecnologicaVisual,
    eh_estado_capital: bool,
) -> (EraTecnologicaVisual, f64) {
    // 1. Métodos de produção locais
    let mut soma_metodos = 0.0;
    let mut peso_metodos = 0.0;

    for (metodo_id, &qtd_niveis) in metodos_ativos_estado {
        if qtd_niveis == 0 {
            continue;
        }
        let qtd_f = qtd_niveis as f64;
        if let Some(def_met) = catalogo.buscar_metodo(metodo_id) {
            let valor_era = (def_met.era_minima.indice() as f64) * 25.0;
            soma_metodos += valor_era * qtd_f * def_met.peso_adocao;
            peso_metodos += qtd_f * def_met.peso_adocao;
        }
    }

    let pontuacao_metodos_local = if peso_metodos > 0.0 {
        (soma_metodos / peso_metodos).clamp(0.0, 100.0)
    } else {
        0.0
    };

    // 2. Modernidade e eletrificação da infraestrutura local
    let aeroportos_efetivo = infra.aeroportos.capacidade_efetiva;
    let energia_efetiva = infra.rede_eletrica.capacidade_efetiva;
    let conexao_moderna = (aeroportos_efetivo / 60.0).clamp(0.0, 1.0) * 45.0
        + (energia_efetiva / 120.0).clamp(0.0, 1.0) * 55.0;

    // 3. Difusão do centro nacional para o estado
    let bonus_capital = if eh_estado_capital { 8.0 } else { 0.0 };
    let difusao_nacional = (indice_modernidade_pais * 0.25) + bonus_capital;

    // 4. Índice local integrado
    let mut indice_estado =
        0.55 * pontuacao_metodos_local + 0.25 * conexao_moderna + 0.20 * difusao_nacional;

    // Teto estrito: um estado nunca exibe uma era acima da tecnologia já descoberta pelo país
    let teto_valor = ((era_teto_pais.indice() as f64) * 25.0 + 24.0).clamp(0.0, 100.0);
    indice_estado = indice_estado.min(teto_valor);

    let era_visual_estado = converter_indice_em_era(indice_estado, era_teto_pais);
    (era_visual_estado, indice_estado)
}

/// Converte um índice numérico de modernidade (0.0 a 100.0) na respectiva era visual, limitada pelo teto.
fn converter_indice_em_era(indice: f64, teto: EraTecnologicaVisual) -> EraTecnologicaVisual {
    let era_calculada = if indice < 18.0 {
        EraTecnologicaVisual::Era1945
    } else if indice < 42.0 {
        EraTecnologicaVisual::Era1955
    } else if indice < 66.0 {
        EraTecnologicaVisual::Era1965
    } else if indice < 86.0 {
        EraTecnologicaVisual::Era1975
    } else {
        EraTecnologicaVisual::Era1985
    };

    if era_calculada.indice() > teto.indice() {
        teto
    } else {
        era_calculada
    }
}

/// Determina o estilo arquitetônico dominante com base na ideologia e suserania.
pub fn determinar_estilo_arquitetonico(
    alinhamento: Option<&str>,
    tipo_governo: Option<&str>,
    eh_colonia: bool,
) -> EstiloArquitetonico {
    if eh_colonia {
        return EstiloArquitetonico::ColonialTradicional;
    }

    match alinhamento {
        Some("bloco_sovietico") => EstiloArquitetonico::SocialistaPlanejado,
        Some("bloco_ocidental") => EstiloArquitetonico::OcidentalCorporativo,
        Some("nao_alinhado") => {
            if let Some(gov) = tipo_governo {
                if gov.contains("socialista") || gov.contains("partido_unico") {
                    EstiloArquitetonico::SocialistaPlanejado
                } else if gov.contains("republica") || gov.contains("presidencialista") {
                    EstiloArquitetonico::DesenvolvimentistaModerno
                } else {
                    EstiloArquitetonico::TradicionalHistorico
                }
            } else {
                EstiloArquitetonico::DesenvolvimentistaModerno
            }
        }
        _ => EstiloArquitetonico::TradicionalHistorico,
    }
}
