//! Cálculo do poder político (clout) dos Grupos de Interesse a partir da riqueza,
//! alfabetização, estratos dos Pops e das leis de sufrágio / distribuição de poder.

use crate::catalogo::CatalogoPolitica;
use crate::entidades::pais::PaisPolitica;
use crate::tipos::{PapelGrupoInteresse, StatusClout};
use std::collections::BTreeMap;

/// Atualiza a força política per capita dos Pops e agrega o Clout de cada Grupo de Interesse.
pub fn calcular_forca_politica_e_clout(pais: &mut PaisPolitica, catalogo: &CatalogoPolitica) {
    if pais.pops.is_empty() {
        return;
    }

    // Identifica as leis de forma de governo e sistema eleitoral
    let lei_eleitoral_id = pais
        .leis_ativas
        .get("sistema_eleitoral")
        .or_else(|| pais.leis_ativas.get("governo"))
        .cloned();

    let def_lei_eleitoral = lei_eleitoral_id
        .as_ref()
        .and_then(|id| catalogo.leis.get(id));

    let eh_partido_unico = pais
        .leis_ativas
        .get("forma_governo")
        .map(|s| s.as_str() == "estado_partido_unico" || s.as_str() == "partido_unico")
        .unwrap_or(false);

    // 1. Zera acumuladores dos Grupos de Interesse
    for gi in pais.grupos_interesse.values_mut() {
        gi.forca_politica_acumulada = 0.0;
        gi.populacao_apoiadora = 0.0;
    }

    // 2. Calcula a força política per capita e total de cada Pop e distribui entre os IGs
    for pop in &mut pais.pops {
        let peso_prof = match pop.tipo_pop_id.as_str() {
            "capitalistas" => 6.0,
            "aristocratas" => 5.0,
            "gerentes" | "oficiais" | "engenheiros" => 2.5,
            "intelectuais" => 2.3,
            "burocratas" => 2.0,
            "maquinistas" => 1.6,
            "clero" | "clerigos" => 1.8,
            "militares" | "soldados" => 1.3,
            "funcionarios" | "comerciantes" => 1.4,
            "operarios" | "mineiros" => 1.0,
            "agricultores" => 0.8,
            "camponeses" => 0.3,
            _ => 1.0,
        };

        // Modificador da lei de sufrágio/eleitoral
        let mod_lei = if let Some(def_lei) = def_lei_eleitoral {
            def_lei.multiplicador_eleitoral_pop(
                pop.estrato.indice(),
                pop.alfabetizacao,
                pop.padrao_de_vida,
            )
        } else {
            1.0
        };

        // Força política individual: N * W_prof * (1 + 0.05*SoL) * (1 + 0.5*Lit) * M_lei
        let fp_individual = pop.populacao
            * peso_prof
            * (1.0 + 0.05 * pop.padrao_de_vida)
            * (1.0 + 0.50 * pop.alfabetizacao)
            * mod_lei;

        pop.forca_politica_bruta = fp_individual;

        // Distribuição de apoio entre IGs com base nas afinidades sociais do catálogo
        if pop.apoio_por_gi.is_empty() {
            let mut pesos_temp: BTreeMap<String, f64> = BTreeMap::new();
            for (gi_id, gi_def) in &catalogo.grupos_interesse {
                for base in &gi_def.base_social_pops {
                    if base.tipo_pop == pop.tipo_pop_id {
                        *pesos_temp.entry(gi_id.clone()).or_insert(0.0) += base.peso_atracao;
                    }
                }
            }
            let soma_pesos: f64 = pesos_temp.values().copied().sum();
            if soma_pesos > 0.0 {
                for (gi_id, peso) in pesos_temp {
                    pop.apoio_por_gi.insert(gi_id, peso / soma_pesos);
                }
            } else {
                // Atribuição de contingência para evitar pops sem representação
                let padrao_gi = match pop.estrato {
                    crate::tipos::Estrato::Baixo => "trabalhadores_sindicalizados",
                    crate::tipos::Estrato::Medio => "pequena_burguesia",
                    crate::tipos::Estrato::Alto => "industriais",
                };
                pop.apoio_por_gi.insert(padrao_gi.to_string(), 1.0);
            }
        }

        // Soma as contribuições do Pop para cada IG
        for (gi_id, &fracao) in &pop.apoio_por_gi {
            if let Some(gi) = pais.grupos_interesse.get_mut(gi_id) {
                gi.forca_politica_acumulada += fp_individual * fracao;
                gi.populacao_apoiadora += pop.populacao * fracao;
            }
        }
    }

    // 3. Aplica bônus institucionais, partidários e financiamento externo (CIA/KGB)
    for (gi_id, gi) in &mut pais.grupos_interesse {
        // Bônus de partido único para o partido de vanguarda comunista
        if eh_partido_unico && (gi_id == "partido_comunista" || gi_id == "nomenklatura_partidaria")
        {
            gi.forca_politica_acumulada *= 2.5;
        }

        // Bônus de financiamento externo direto
        if gi.bonus_financiamento_externo > 0.0 {
            gi.forca_politica_acumulada *= 1.0 + (gi.bonus_financiamento_externo / 100.0);
        }
    }

    // 4. Normalização do Clout (soma = 1.0)
    let soma_forca_total: f64 = pais
        .grupos_interesse
        .values()
        .map(|g| g.forca_politica_acumulada)
        .sum();

    if soma_forca_total > 0.0 {
        for gi in pais.grupos_interesse.values_mut() {
            gi.clout = (gi.forca_politica_acumulada / soma_forca_total).clamp(0.0, 1.0);
            gi.status = StatusClout::de_clout(gi.clout);

            // Se um grupo era do governo mas tornou-se marginalizado, sai do governo (salvo partido único)
            if gi.status == StatusClout::Marginalizado && !eh_partido_unico && gi.eh_no_governo() {
                gi.papel = PapelGrupoInteresse::Marginalizado;
            }
        }
    }
}
