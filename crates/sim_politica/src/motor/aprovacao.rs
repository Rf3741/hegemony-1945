//! Cálculo diário da pontuação de aprovação (-20.0 a +20.0) e humor dos Grupos de Interesse:
//! leis vigentes, bens que lhes importam no mercado, tramitações e eventos temporários.

use crate::catalogo::CatalogoPolitica;
use crate::entidades::pais::PaisPolitica;
use crate::tipos::HumorGrupoInteresse;

/// Preço base de referência de mercado caso não conste na tabela nacional.
const PRECO_BASE_PADRAO: f64 = 25.0;

/// Atualiza a aprovação e o humor de todos os Grupos de Interesse de uma nação.
pub fn atualizar_aprovacao_grupos_interesse(pais: &mut PaisPolitica, catalogo: &CatalogoPolitica) {
    // 1. Calcula o SoL médio e SoL esperado ponderado dos apoiadores de cada GI
    let mut sol_real_gi: Vec<(String, f64, f64, f64)> = Vec::new();
    for gi_id in pais.grupos_interesse.keys() {
        let mut soma_sol = 0.0;
        let mut soma_sol_esp = 0.0;
        let mut soma_pop = 0.0;

        for pop in &pais.pops {
            if let Some(&fracao) = pop.apoio_por_gi.get(gi_id) {
                let pop_grupo = pop.populacao * fracao;
                soma_sol += pop.padrao_de_vida * pop_grupo;
                soma_sol_esp += pop.padrao_de_vida_esperado * pop_grupo;
                soma_pop += pop_grupo;
            }
        }

        let media_sol = if soma_pop > 0.0 {
            soma_sol / soma_pop
        } else {
            12.0
        };
        let media_esp = if soma_pop > 0.0 {
            soma_sol_esp / soma_pop
        } else {
            12.0
        };
        sol_real_gi.push((gi_id.clone(), media_sol, media_esp, soma_pop));
    }

    // 2. Avalia cada Grupo de Interesse
    for (gi_id, media_sol, media_esp, _pop_apoio) in sol_real_gi {
        let Some(gi) = pais.grupos_interesse.get_mut(&gi_id) else {
            continue;
        };

        let mut alvo = 0.0;
        let def_gi = catalogo.grupos_interesse.get(&gi_id);

        // A. Efeito das Leis Vigentes (Postura intrínseca do grupo + Ideologia do líder)
        for lei_id in pais.leis_ativas.values() {
            if let Some(def) = def_gi {
                if def.leis_preferidas.iter().any(|id| id == lei_id) {
                    alvo += 2.5;
                } else if def.leis_odiadas.iter().any(|id| id == lei_id) {
                    alvo -= 3.5;
                }
            }

            // Postura da ideologia pessoal do líder
            if let Some(lider) = &gi.lider
                && let Some(ideol_lider) = catalogo.ideologias.get(&lider.ideologia_id)
            {
                if ideol_lider.leis_favorecidas.iter().any(|id| id == lei_id) {
                    alvo += 1.5;
                } else if ideol_lider.leis_rejeitadas.iter().any(|id| id == lei_id) {
                    alvo -= 2.5;
                }
            }

            // Efeito direto de modificadores de leis que citam aprovação deste grupo
            if let Some(def_lei) = catalogo.leis.get(lei_id) {
                for modif in &def_lei.efeitos {
                    if modif.id == format!("aprovacao_{}", gi_id) {
                        alvo += modif.valor;
                    }
                }
            }
        }

        // B. Efeito de Lei em Tramitação
        if let Some(tram) = &pais.lei_em_tramitacao {
            let postura =
                gi.avaliar_postura_mudanca_lei(catalogo, &tram.lei_alvo_id, &tram.lei_anterior_id);
            match postura {
                crate::tipos::PosturaLei::EndossaFortemente => alvo += 3.0,
                crate::tipos::PosturaLei::Apoia => alvo += 1.5,
                crate::tipos::PosturaLei::Neutro => {}
                crate::tipos::PosturaLei::Opoe => alvo -= 2.0,
                crate::tipos::PosturaLei::RejeitaFortemente => alvo -= 4.0,
            }
        }

        // C. Bens Importantes e Impacto de Preços de Mercado
        if let Some(def) = def_gi {
            for interesse in &def.bens_importantes {
                let preco_mercado = pais
                    .precos_mercado
                    .get(&interesse.bem_id)
                    .copied()
                    .unwrap_or(PRECO_BASE_PADRAO);

                let preco_ref = PRECO_BASE_PADRAO;
                let razao_preco = (preco_mercado - preco_ref) / preco_ref;

                if interesse.deseja_barato {
                    // Quer preço baixo (ex.: operários e alimentos/aquecimento): inflação dói
                    if razao_preco > 0.0 {
                        alvo -= (razao_preco * 4.0 * interesse.peso).min(5.0);
                    } else {
                        alvo += (-razao_preco * 2.0 * interesse.peso).min(3.0);
                    }
                } else {
                    // Quer lucro de venda (ex.: industriais e latifundiários): preço alto gera lucros
                    if razao_preco > 0.0 {
                        alvo += (razao_preco * 3.0 * interesse.peso).min(4.0);
                    } else {
                        alvo -= (-razao_preco * 3.0 * interesse.peso).min(4.0);
                    }
                }
            }
        }

        // D. Satisfação Econômica dos Pops Apoiadores (Padrão de Vida real vs. esperado)
        let delta_sol = (media_sol - media_esp).clamp(-6.0, 6.0);
        alvo += delta_sol * 0.80;

        // E. Bônus por Participação no Gabinete Governamental
        if gi.eh_no_governo() {
            alvo += 4.0;
        } else if !gi.eh_marginalizado() {
            alvo -= 1.5;
        }

        // F. Modificadores Temporários de Eventos
        for m in &gi.modificadores_temporarios {
            alvo += m.valor;
        }

        // 3. Atualiza alvo e relaxa suavemente a aprovação diária
        gi.aprovacao_alvo = alvo.clamp(-20.0, 20.0);
        gi.aprovacao += (gi.aprovacao_alvo - gi.aprovacao) * 0.05;
        gi.aprovacao = gi.aprovacao.clamp(-20.0, 20.0);

        // 4. Decrementa contadores de modificadores temporários
        for m in &mut gi.modificadores_temporarios {
            if m.dias_restantes > 0 {
                m.dias_restantes -= 1;
            }
        }
        gi.modificadores_temporarios
            .retain(|m| m.dias_restantes > 0);

        // 5. Atualiza o humor institucional do grupo
        gi.humor = HumorGrupoInteresse::de_aprovacao(gi.aprovacao);
    }
}
