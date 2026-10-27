//! Cálculo da Legitimidade Governamental (0.0 a 100.0) a partir da composição da coalizão,
//! ideologia dos parceiros ministeriais, presença do chefe de Estado e peso parlamentar.

use crate::catalogo::CatalogoPolitica;
use crate::entidades::pais::PaisPolitica;
use crate::tipos::FaixaLegitimidade;

/// Recalcula a legitimidade governamental de um país conforme a fórmula canônica do Victoria 3.
pub fn calcular_legitimidade_governo(pais: &mut PaisPolitica, catalogo: &CatalogoPolitica) {
    let forma_governo = pais
        .leis_ativas
        .get("forma_governo")
        .map(|s| s.as_str())
        .unwrap_or("republica_presidencialista");

    let eh_partido_unico =
        forma_governo == "estado_partido_unico" || forma_governo == "partido_unico";

    // 1. Base máxima institucional do regime
    let base_regime = match forma_governo {
        "republica_parlamentarista" => 110.0,
        "monarquia_constitucional" => 105.0,
        "republica_presidencialista" => 100.0,
        "estado_partido_unico" | "partido_unico" => 100.0,
        "autocracia_monarquica" | "autocracia" => 90.0,
        "oligarquia_militar" | "oligarquia" => 85.0,
        "administracao_colonial" | "governador_colonial" => 70.0,
        _ => 95.0,
    };

    // 2. Identifica os IGs que compõem o governo
    let mut clout_governo = 0.0;
    let mut grupos_governo_ids: Vec<String> = Vec::new();

    for (gi_id, gi) in &pais.grupos_interesse {
        if gi.eh_no_governo() {
            clout_governo += gi.clout;
            grupos_governo_ids.push(gi_id.clone());
        }
    }

    // Em regime de partido único, o partido de vanguarda deve estar no governo
    if eh_partido_unico {
        let vanguarda_no_governo = grupos_governo_ids
            .iter()
            .any(|id| id == "partido_comunista" || id == "nomenklatura_partidaria");
        if !vanguarda_no_governo {
            pais.legitimidade = 0.0;
            pais.faixa_legitimidade = FaixaLegitimidade::Ilegitimo;
            return;
        }
    }

    // 3. Parcela de Clout dos governistas
    let mut pontos = base_regime * clout_governo.clamp(0.0, 1.0);

    // 4. Bônus por presença do Chefe de Estado (Monarca, Presidente, Secretário-Geral)
    if let Some(gi_chefe) = &pais.gi_chefe_estado {
        if grupos_governo_ids.iter().any(|id| id == gi_chefe) {
            pontos += 15.0;
        } else {
            // Cohabitação ou chefe de estado isolado gera atrito
            pontos -= 10.0;
        }
    } else if let Some(chefe) = &pais.chefe_de_estado {
        // Se o chefe de estado tem ideologia compatível com algum membro do governo
        let compatibilidade = grupos_governo_ids.iter().any(|id| {
            if let Some(gi) = pais.grupos_interesse.get(id)
                && let Some(def_gi) = catalogo.grupos_interesse.get(id)
            {
                return def_gi.ideologias.contains(&chefe.ideologia_id)
                    || gi
                        .lider
                        .as_ref()
                        .map(|l| l.ideologia_id == chefe.ideologia_id)
                        .unwrap_or(false);
            }
            false
        });
        if compatibilidade {
            pontos += 10.0;
        }
    }

    // 5. Penalidade por Incoerência Ideológica de Coalizão
    let mut penalidade_ideologica = 0.0;
    let n = grupos_governo_ids.len();
    if n >= 2 {
        for i in 0..n {
            for j in (i + 1)..n {
                let id_a = &grupos_governo_ids[i];
                let id_b = &grupos_governo_ids[j];

                let gi_a = pais.grupos_interesse.get(id_a);
                let gi_b = pais.grupos_interesse.get(id_b);

                let ideol_a_id = gi_a
                    .and_then(|g| g.lider.as_ref())
                    .map(|l| l.ideologia_id.as_str())
                    .or_else(|| {
                        catalogo
                            .grupos_interesse
                            .get(id_a)
                            .and_then(|g| g.ideologias.first().map(|s| s.as_str()))
                    })
                    .unwrap_or("liberalismo");

                let ideol_b_id = gi_b
                    .and_then(|g| g.lider.as_ref())
                    .map(|l| l.ideologia_id.as_str())
                    .or_else(|| {
                        catalogo
                            .grupos_interesse
                            .get(id_b)
                            .and_then(|g| g.ideologias.first().map(|s| s.as_str()))
                    })
                    .unwrap_or("liberalismo");

                if let (Some(ia), Some(ib)) = (
                    catalogo.ideologias.get(ideol_a_id),
                    catalogo.ideologias.get(ideol_b_id),
                ) {
                    let dist = ia.distancia(ib);
                    penalidade_ideologica += dist * 14.0;
                }
            }
        }
    }
    pontos -= penalidade_ideologica;

    // 6. Apoio Parlamentar / Votos Eleitorais da Coalizão
    if let Some(eleicao) = &pais.ultima_eleicao {
        let mut cadeiras_governo = 0u32;
        let total_cadeiras: u32 = eleicao.cadeiras_por_partido.values().sum();

        for part in pais.partidos.values() {
            // Se o partido contém membros no governo
            let participa = part
                .grupos_membros
                .iter()
                .any(|m| grupos_governo_ids.contains(m));
            if participa {
                cadeiras_governo += eleicao
                    .cadeiras_por_partido
                    .get(&part.id)
                    .copied()
                    .unwrap_or(0);
            }
        }

        if total_cadeiras > 0 {
            let fracao_cadeiras = cadeiras_governo as f64 / total_cadeiras as f64;
            // Bônus se maioria (>= 50%), penalidade se governo minoritário (< 50%)
            pontos += 25.0 * (fracao_cadeiras - 0.50);
        }
    }

    // 7. Penalidade por fragmentação excessiva (mais de 2 facções no gabinete)
    if n > 2 {
        pontos -= (n - 2) as f64 * 4.0;
    }

    // Normaliza para o intervalo 0..=100
    pais.legitimidade = pontos.clamp(0.0, 100.0);
    pais.faixa_legitimidade = FaixaLegitimidade::de_pontos(pais.legitimidade);
}
