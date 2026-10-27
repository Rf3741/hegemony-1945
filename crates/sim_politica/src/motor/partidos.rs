//! Formação dinâmica de partidos políticos por coalizões de Grupos de Interesse
//! com afinidade ideológica, moduladas pela ideologia dos líderes e influência externa.

use crate::catalogo::CatalogoPolitica;
use crate::entidades::pais::PaisPolitica;
use crate::entidades::partido::InstanciaPartido;
use std::collections::BTreeMap;

/// Atualiza dinamicamente a composição dos partidos políticos de um país.
pub fn atualizar_partidos_dinamicos(pais: &mut PaisPolitica, catalogo: &CatalogoPolitica) {
    let lei_eleitoral_id = pais
        .leis_ativas
        .get("sistema_eleitoral")
        .or_else(|| pais.leis_ativas.get("governo"))
        .map(|s| s.as_str())
        .unwrap_or("republica_presidencialista");

    let def_lei = catalogo.leis.get(lei_eleitoral_id);

    // 1. Em regimes sem eleições, partidos são desativados
    if def_lei.map(|l| l.sem_eleicoes).unwrap_or(false) || lei_eleitoral_id == "sem_eleicoes" {
        pais.partidos.clear();
        return;
    }

    // 2. Em regime de partido único, todos os grupos leais/alinhados convergem para o partido oficial
    let eh_partido_unico = def_lei.map(|l| l.eleicoes_partido_unico).unwrap_or(false)
        || lei_eleitoral_id == "eleicoes_partido_unico"
        || pais.leis_ativas.get("forma_governo").map(|s| s.as_str())
            == Some("estado_partido_unico");

    if eh_partido_unico {
        let mut grupos_oficiais = Vec::new();
        for (gi_id, gi) in &pais.grupos_interesse {
            // Grupos do regime integram o partido de vanguarda
            if !gi.eh_marginalizado() {
                grupos_oficiais.push(gi_id.clone());
            }
        }

        pais.partidos.clear();
        let mut partido_vanguarda = InstanciaPartido::novo(
            "partido_comunista_vanguarda",
            "Partido Comunista (Vanguarda Oficial)",
            "partido_comunista_vanguarda",
        );
        partido_vanguarda.grupos_membros = grupos_oficiais;
        pais.partidos
            .insert("partido_comunista_vanguarda".to_string(), partido_vanguarda);
        return;
    }

    // 3. Regimes pluripartidários: cada grupo de interesse avalia afinidade com os arquétipos
    let mut distribuicao_partidos: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (gi_id, gi) in &pais.grupos_interesse {
        if gi.eh_marginalizado() {
            continue;
        }

        let def_gi = catalogo.grupos_interesse.get(gi_id);
        let mut melhor_arquetipo: Option<String> = None;
        let mut maior_afinidade = 5.0; // limiar mínimo para fundar/aderir a partido

        // Ideologia ativa do grupo: ideologia do líder (se houver) tem peso prioritário
        let ideol_lider = gi.lider.as_ref().map(|l| l.ideologia_id.as_str());

        for (arq_id, arq) in &catalogo.arquetipos_partido {
            let mut afinidade = 0.0;

            // Bônus se o grupo é membro natural do arquétipo
            if arq.grupos_naturais.iter().any(|g| g == gi_id) {
                afinidade += 25.0;
            }

            // Afinidade com as ideologias do arquétipo
            if let Some(def) = def_gi {
                for ideol_gi in &def.ideologias {
                    if arq.ideologias_afins.iter().any(|ia| ia == ideol_gi) {
                        afinidade += 15.0;
                    }
                }
            }

            // O líder pessoal atrai fortemente o grupo para seu partido de afinidade
            if let Some(lider_id) = ideol_lider
                && arq.ideologias_afins.iter().any(|ia| ia == lider_id)
            {
                afinidade += 20.0;
            }

            // Influência externa e apoio de superpotência
            if gi.bonus_financiamento_externo > 0.0 {
                if arq_id == "partido_liberal"
                    || arq_id == "partido_conservador"
                    || arq_id == "partido_democrata_cristao"
                {
                    if pais.influencia_externa.pontos_eua > 30.0 {
                        afinidade += 10.0;
                    }
                } else if (arq_id == "partido_comunista_vanguarda"
                    || arq_id == "partido_social_democrata")
                    && pais.influencia_externa.pontos_urss > 30.0
                {
                    afinidade += 10.0;
                }
            }

            if afinidade > maior_afinidade {
                maior_afinidade = afinidade;
                melhor_arquetipo = Some(arq_id.clone());
            }
        }

        if let Some(arq_escolhido) = melhor_arquetipo {
            distribuicao_partidos
                .entry(arq_escolhido)
                .or_default()
                .push(gi_id.clone());
        }
    }

    // 4. Cria e atualiza as instâncias dos partidos com base nas coalizões formadas
    let mut novos_partidos = BTreeMap::new();
    for (arq_id, membros) in distribuicao_partidos {
        if membros.is_empty() {
            continue;
        }

        let nome_partido = catalogo
            .arquetipos_partido
            .get(&arq_id)
            .map(|a| a.nome.clone())
            .unwrap_or_else(|| arq_id.replace('_', " "));

        let mut inst = pais.partidos.remove(&arq_id).unwrap_or_else(|| {
            InstanciaPartido::novo(arq_id.clone(), nome_partido, arq_id.clone())
        });

        inst.grupos_membros = membros;
        novos_partidos.insert(arq_id, inst);
    }

    pais.partidos = novos_partidos;
}
