//! Motor de movimentos políticos, crises sociais (greves, revoltas, golpes militares)
//! e eclosão de Guerra Civil sinalizada ao motor `sim_guerra`.

use crate::catalogo::CatalogoPolitica;
use crate::entidades::crise::{CriseInterna, SinalGuerraCivil};
use crate::entidades::pais::PaisPolitica;
use crate::rng::RngDeterministico;
use crate::tipos::{DataJogo, OrigemGuerraCivil, PotenciaBloco, TipoCrisePolitica};

/// Processa o avanço de movimentos sociais, conspirações de golpe e revoluções populares.
pub fn processar_movimentos_e_crises(
    pais: &mut PaisPolitica,
    data_atual: DataJogo,
    _catalogo: &CatalogoPolitica,
    rng: &mut RngDeterministico,
) -> Vec<SinalGuerraCivil> {
    let mut sinais_guerra: Vec<SinalGuerraCivil> = Vec::new();
    let total_pop = pais.populacao_total();
    let total_radicais = pais.total_radicais();
    let taxa_radicalismo_geral = if total_pop > 0.0 {
        total_radicais / total_pop
    } else {
        0.0
    };

    // 1. Processa Movimentos Políticos e Termômetro de Revolução
    for mov in &mut pais.movimentos {
        // Soma do clout dos IGs que apoiam o movimento
        let mut clout_apoio = 0.0;
        let mut tem_gi_insurrecto = false;

        for gi_id in &mov.grupos_apoiadores {
            if let Some(gi) = pais.grupos_interesse.get(gi_id) {
                clout_apoio += gi.clout;
                if gi.humor.eh_radical() {
                    tem_gi_insurrecto = true;
                }
            }
        }

        // Apoio e Radicalismo do movimento
        mov.apoio = (clout_apoio * 70.0 + taxa_radicalismo_geral * 50.0).clamp(0.0, 100.0);
        mov.radicalismo = if tem_gi_insurrecto {
            (clout_apoio * 40.0 + taxa_radicalismo_geral * 80.0).clamp(10.0, 100.0)
        } else {
            (taxa_radicalismo_geral * 50.0).clamp(0.0, 60.0)
        };

        // Termômetro Revolucionário
        if mov.eh_revolucionario() {
            mov.em_revolucao = true;
            let incremento_diario = ((mov.apoio - 20.0) / 40.0) * 0.25;
            mov.termometro_revolucao =
                (mov.termometro_revolucao + incremento_diario).clamp(0.0, 100.0);

            // Eclosão de Guerra Civil Revolucionária ao atingir 100%
            if mov.termometro_revolucao >= 100.0 {
                let sinal = SinalGuerraCivil {
                    pais_id: pais.id.clone(),
                    pais_tag: pais.tag.clone(),
                    data_eclosao: data_atual,
                    origem: OrigemGuerraCivil::RevolucaoIdeologica,
                    faccao_rebelde_id: format!("{}_rebeldes", mov.id),
                    nome_faccao_rebelde: format!("Frente Revolucionária ({})", mov.nome),
                    ideologia_rebelde_id: "marxismo_leninismo".into(),
                    grupos_rebeldes: mov.grupos_apoiadores.clone(),
                    grupos_legalistas: pais
                        .grupos_interesse
                        .iter()
                        .filter(|(id, _)| !mov.grupos_apoiadores.contains(id))
                        .map(|(id, _)| id.clone())
                        .collect(),
                    lei_demandada: match &mov.objetivo {
                        crate::entidades::movimento::ObjetivoMovimento::PromulgarLei {
                            categoria,
                            lei_id,
                        } => Some((categoria.clone(), lei_id.clone())),
                        _ => None,
                    },
                    proporcao_exercito_rebelde: 0.45,
                    patrocinador_externo: if pais.influencia_externa.pontos_urss > 30.0 {
                        Some(PotenciaBloco::Urss)
                    } else if pais.influencia_externa.pontos_eua > 30.0 {
                        Some(PotenciaBloco::Eua)
                    } else {
                        None
                    },
                };
                sinais_guerra.push(sinal);
                mov.termometro_revolucao = 0.0;
                mov.em_revolucao = false;
            }
        } else {
            // Decaimento gradual se a agitação ceder
            mov.em_revolucao = false;
            mov.termometro_revolucao = (mov.termometro_revolucao - 0.15).max(0.0);
        }
    }

    // 2. Conspiração de Golpe de Estado Militar
    // Condições: Militares Insurrectos (aprovacao <= -10), Poderosos (clout >= 0.20), e Governo Frágil/Ilegítimo (< 35)
    let mut golpe_acionado = false;
    if let Some(mil) = pais.grupos_interesse.get("militares")
        && mil.humor.eh_radical()
        && mil.clout >= 0.20
        && pais.legitimidade < 35.0
    {
        golpe_acionado = true;
    }

    if golpe_acionado && rng.chance(0.02) {
        // Teste de resolução do golpe
        let forca_militar = pais
            .grupos_interesse
            .get("militares")
            .map(|g| g.clout)
            .unwrap_or(0.25);

        let apoio_externo_cia = if pais.influencia_externa.pontos_eua > 40.0 {
            0.15
        } else {
            0.0
        };
        let forca_golpista = forca_militar * (1.0 + apoio_externo_cia);
        let forca_legalista = (pais.legitimidade / 100.0) * 0.60;

        if forca_golpista > forca_legalista * 1.30 {
            // Golpe Militar bem-sucedido: tomada do poder sem guerra civil aberta
            pais.promulgar_lei_instantanea("forma_governo", "oligarquia_militar");
            pais.promulgar_lei_instantanea("sistema_eleitoral", "sem_eleicoes");
            let _ = pais.reformar_governo(&["militares".to_string()]);
            pais.crises_ativas.push(CriseInterna::nova(
                "golpe_militar_instalado",
                TipoCrisePolitica::GolpeMilitar,
                Some("militares".into()),
                80.0,
            ));
        } else if forca_golpista < forca_legalista * 0.75 {
            // Intentona fracassada: generais presos e militares perdem força
            if let Some(mil) = pais.grupos_interesse.get_mut("militares") {
                mil.forca_politica_acumulada *= 0.50;
                mil.aprovacao = -5.0;
            }
        } else {
            // Impasse armado: eclosão de Guerra Civil por Golpe Frustrado
            let sinal = SinalGuerraCivil {
                pais_id: pais.id.clone(),
                pais_tag: pais.tag.clone(),
                data_eclosao: data_atual,
                origem: OrigemGuerraCivil::GolpeMilitarImpasse,
                faccao_rebelde_id: "junta_militar_rebelde".into(),
                nome_faccao_rebelde: "Junta Militar Legalista de Salvação Nacional".into(),
                ideologia_rebelde_id: "militarismo_nacionalista".into(),
                grupos_rebeldes: vec!["militares".into()],
                grupos_legalistas: pais
                    .grupos_interesse
                    .keys()
                    .filter(|&k| k != "militares")
                    .cloned()
                    .collect(),
                lei_demandada: Some(("forma_governo".into(), "oligarquia_militar".into())),
                proporcao_exercito_rebelde: 0.60,
                patrocinador_externo: if apoio_externo_cia > 0.0 {
                    Some(PotenciaBloco::Eua)
                } else {
                    None
                },
            };
            sinais_guerra.push(sinal);
        }
    }

    // 3. Eclosão de Greves Gerais
    let tem_greve = pais
        .grupos_interesse
        .get("trabalhadores_sindicalizados")
        .or_else(|| pais.grupos_interesse.get("sindicatos_operarios"))
        .map(|g| g.humor.eh_radical())
        .unwrap_or(false);

    if tem_greve
        && taxa_radicalismo_geral > 0.25
        && !pais
            .crises_ativas
            .iter()
            .any(|c| c.tipo_crise == TipoCrisePolitica::Greve && !c.resolvida)
    {
        pais.crises_ativas.push(CriseInterna::nova(
            "greve_geral_fabril",
            TipoCrisePolitica::Greve,
            Some("trabalhadores_sindicalizados".into()),
            65.0,
        ));
    }

    // Avança a duração de crises ativas
    for c in &mut pais.crises_ativas {
        c.dias_ativa += 1;
    }

    sinais_guerra
}
