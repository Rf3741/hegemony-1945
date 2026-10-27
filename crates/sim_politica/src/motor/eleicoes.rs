//! Execução do ciclo eleitoral periódico: campanha, momento eleitoral,
//! apuração de votos por Pop e alocação de cadeiras parlamentares.

use crate::catalogo::CatalogoPolitica;
use crate::entidades::eleicao::{CampanhaEleitoral, ResultadoEleicao};
use crate::entidades::pais::PaisPolitica;
use crate::rng::RngDeterministico;
use crate::tipos::DataJogo;
use std::collections::BTreeMap;

/// Processa o ciclo diário eleitoral, campanhas e realização do pleito.
pub fn processar_ciclo_eleitoral_diario(
    pais: &mut PaisPolitica,
    data_atual: DataJogo,
    catalogo: &CatalogoPolitica,
    rng: &mut RngDeterministico,
) {
    let lei_eleitoral_id = pais
        .leis_ativas
        .get("sistema_eleitoral")
        .or_else(|| pais.leis_ativas.get("governo"))
        .map(|s| s.as_str())
        .unwrap_or("republica_presidencialista");

    let def_lei = catalogo.leis.get(lei_eleitoral_id);

    // Se a nação não tem eleições, mantém o contador pausado
    if def_lei.map(|l| l.sem_eleicoes).unwrap_or(false) || lei_eleitoral_id == "sem_eleicoes" {
        pais.campanha_eleitoral = None;
        pais.dias_para_proxima_eleicao = 1460;
        return;
    }

    // Avança a contagem regressiva para a eleição
    if pais.dias_para_proxima_eleicao > 0 {
        pais.dias_para_proxima_eleicao -= 1;
    }

    // Início da Campanha Eleitoral (90 dias antes do pleito)
    if pais.dias_para_proxima_eleicao <= 90 && pais.dias_para_proxima_eleicao > 0 {
        if pais.campanha_eleitoral.is_none() {
            pais.campanha_eleitoral = Some(CampanhaEleitoral {
                em_andamento: true,
                dias_restantes: pais.dias_para_proxima_eleicao,
                duracao_total: 90,
            });
        } else if let Some(camp) = &mut pais.campanha_eleitoral {
            camp.dias_restantes = pais.dias_para_proxima_eleicao;
        }

        // Progresso do momento de campanha dos partidos
        for part in pais.partidos.values_mut() {
            let mut bonus_lideres = 0.0;
            for gi_id in &part.grupos_membros {
                if let Some(gi) = pais.grupos_interesse.get(gi_id)
                    && let Some(lider) = &gi.lider
                {
                    bonus_lideres += lider.modificador_momento_eleitoral();
                }
            }

            // Flutuação diária determinística da campanha
            let flutuacao = rng.faixa_f64(-0.15, 0.15);
            let bonus_externo = part.bonus_financiamento_externo * 0.02;

            part.momento_campanha =
                (part.momento_campanha + bonus_lideres * 0.05 + bonus_externo + flutuacao)
                    .clamp(-50.0, 50.0);
        }
    }

    // Dia da Eleição
    if pais.dias_para_proxima_eleicao == 0 {
        apurar_eleicao(pais, data_atual, catalogo);

        // Encerra a campanha e agenda o próximo pleito para 4 anos (1.460 dias)
        pais.campanha_eleitoral = None;
        pais.dias_para_proxima_eleicao = 1460;
    }
}

/// Realiza a apuração de votos por Pop, aloca cadeiras e define o partido vencedor.
fn apurar_eleicao(pais: &mut PaisPolitica, data_atual: DataJogo, catalogo: &CatalogoPolitica) {
    let lei_eleitoral_id = pais
        .leis_ativas
        .get("sistema_eleitoral")
        .or_else(|| pais.leis_ativas.get("governo"))
        .map(|s| s.as_str())
        .unwrap_or("republica_presidencialista");

    let def_lei = catalogo.leis.get(lei_eleitoral_id);
    let eh_partido_unico = def_lei.map(|l| l.eleicoes_partido_unico).unwrap_or(false)
        || lei_eleitoral_id == "eleicoes_partido_unico";

    let mut votos_por_partido: BTreeMap<String, f64> = BTreeMap::new();
    let mut total_votos = 0.0;

    if eh_partido_unico {
        // Eleição de lista única plebiscitária (participação maciça e voto na vanguarda)
        let part_id = "partido_comunista_vanguarda".to_string();
        let votos = pais.populacao_total() * 0.95;
        votos_por_partido.insert(part_id.clone(), votos);
        total_votos = votos;

        let mut cadeiras = BTreeMap::new();
        cadeiras.insert(part_id.clone(), 100);

        let mut percentuais = BTreeMap::new();
        percentuais.insert(part_id.clone(), 1.0);

        let resultado = ResultadoEleicao {
            data: data_atual,
            total_votos_validos: total_votos,
            taxa_participacao: 0.95,
            votos_por_partido,
            percentual_por_partido: percentuais,
            cadeiras_por_partido: cadeiras,
            partido_vencedor_id: Some(part_id),
        };

        pais.ultima_eleicao = Some(resultado);
        return;
    }

    // Eleição pluripartidária competitiva
    for part_id in pais.partidos.keys() {
        votos_por_partido.insert(part_id.clone(), 0.0);
    }

    // Calcula os votos emitidos por cada Pop
    for pop in &pais.pops {
        let mod_sufragio = def_lei
            .map(|l| {
                l.multiplicador_eleitoral_pop(
                    pop.estrato.indice(),
                    pop.alfabetizacao,
                    pop.padrao_de_vida,
                )
            })
            .unwrap_or(1.0);

        if mod_sufragio <= 0.0 {
            continue;
        }

        let votos_pop = pop.populacao * mod_sufragio;

        // Distribui os votos do Pop entre os partidos conforme a filiação de seus IGs apoiados
        for (part_id, part) in &pais.partidos {
            let mut fracao_partido = 0.0;
            for gi_id in &part.grupos_membros {
                if let Some(&f) = pop.apoio_por_gi.get(gi_id) {
                    fracao_partido += f;
                }
            }

            if fracao_partido > 0.0 {
                let fator_campanha = 1.0
                    + (part.momento_campanha / 100.0)
                    + (part.bonus_financiamento_externo / 100.0);
                let votos_destinados = votos_pop * fracao_partido * fator_campanha.max(0.1);
                *votos_por_partido.entry(part_id.clone()).or_insert(0.0) += votos_destinados;
                total_votos += votos_destinados;
            }
        }
    }

    // Calcula percentuais
    let mut percentuais: BTreeMap<String, f64> = BTreeMap::new();
    let mut vencedor_id: Option<String> = None;
    let mut maior_voto = -1.0;

    for (part_id, &votos) in &votos_por_partido {
        let perc = if total_votos > 0.0 {
            votos / total_votos
        } else {
            0.0
        };
        percentuais.insert(part_id.clone(), perc);

        if let Some(part) = pais.partidos.get_mut(part_id) {
            part.votos_ultima_eleicao = votos;
            part.percentual_votos_ultima_eleicao = perc;
            part.momento_campanha = 0.0; // reinicia o momento para o novo ciclo
        }

        if votos > maior_voto {
            maior_voto = votos;
            vencedor_id = Some(part_id.clone());
        }
    }

    // Aloca as 100 cadeiras parlamentares pelo método D'Hondt com cláusula de barreira de 3%
    let cadeiras = ResultadoEleicao::calcular_cadeiras_dhondt(100, &votos_por_partido, 0.03);

    for (part_id, &assentos) in &cadeiras {
        if let Some(part) = pais.partidos.get_mut(part_id) {
            part.cadeiras_parlamento = assentos;
        }
    }

    let taxa_part = if pais.populacao_total() > 0.0 {
        (total_votos / pais.populacao_total()).clamp(0.0, 1.0)
    } else {
        0.80
    };

    let resultado = ResultadoEleicao {
        data: data_atual,
        total_votos_validos: total_votos,
        taxa_participacao: taxa_part,
        votos_por_partido,
        percentual_por_partido: percentuais,
        cadeiras_por_partido: cadeiras,
        partido_vencedor_id: vencedor_id.clone(),
    };

    pais.ultima_eleicao = Some(resultado);

    // Se habilitada a auto-reforma ministerial, convoca os grupos do partido vencedor para o governo
    if pais.auto_reformar_governo_eleicao
        && let Some(venc) = vencedor_id
        && let Some(part_venc) = pais.partidos.get(&venc)
    {
        let novos_governo = part_venc.grupos_membros.clone();
        let _ = pais.reformar_governo(&novos_governo);
    }
}
