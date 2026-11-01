//! Aplicação de efeitos em cascata sobre todos os subsistemas de simulação.

use crate::contexto::ContextoEfeitos;
use crate::{EstadoEventos, Modificador};
use simulacao::politica::CategoriaLei;

/// Dedução da categoria institucional de uma lei a partir de seu ID.
pub fn categoria_da_lei(lei: &str) -> CategoriaLei {
    match lei {
        "sufragio_universal"
        | "sufragio_masculino"
        | "voto_censitario"
        | "voto_capacitario"
        | "partido_unico"
        | "estado_partido_unico"
        | "autocracia"
        | "oligarquia"
        | "governador_colonial"
        | "ocupacao_militar"
        | "republica_parlamentarista"
        | "republica_presidencialista"
        | "sem_eleicoes" => CategoriaLei::Governo,

        "capitalismo_de_mercado"
        | "capitalismo_de_intervencao"
        | "economia_planejada"
        | "cooperativismo"
        | "livre_mercado"
        | "protecionismo"
        | "socialismo_estatal" => CategoriaLei::Economia,

        "trabalho_livre"
        | "regulamentacao_basica"
        | "direitos_trabalhistas_amplos"
        | "proibicao_sindicatos" => CategoriaLei::Trabalho,

        "segregacao_apartheid"
        | "liberdades_restringidas"
        | "liberdades_civis_plenas"
        | "estado_policial"
        | "lei_direitos_civis"
        | "igualdade_racial" => CategoriaLei::DireitosCivis,

        "tributacao_proporcional"
        | "tributacao_progressiva"
        | "imposto_sobre_terra"
        | "tributacao_consumo" => CategoriaLei::SistemaTributario,

        "sistema_privado"
        | "sistema_publico_basico"
        | "estado_bem_estar_social" => CategoriaLei::SaudeEEducacao,

        "exercito_profissional"
        | "alistamento_obrigatorio"
        | "guarda_nacional"
        | "milicia_popular_comunitaria" => CategoriaLei::Exercito,

        _ => CategoriaLei::Governo,
    }
}

/// Aplica um modificador individual no contexto de simulação.
pub fn aplicar_efeito_contextual(
    ctx: &mut ContextoEfeitos,
    est: &mut EstadoEventos,
    pais: &str,
    m: &Modificador,
) {
    // 1. Armazena acumulado histórico
    *est.modificadores.entry((pais.to_string(), m.id.clone())).or_insert(0.0) += m.valor;

    let id = m.id.as_str();

    // 2. Modificadores temporários com duração explícita: "temp:<id>:<dias>"
    if let Some(resto) = id.strip_prefix("temp:") {
        if let Some((mod_nome, dias_str)) = resto.split_once(':') {
            if let Ok(dias) = dias_str.parse::<u32>() {
                est.adicionar_modificador_temporario(pais, mod_nome, m.valor, dias);
                // Também replica para sim_politica se houver GI correspondente
                if let Some(pol) = &mut ctx.politica {
                    if let Some(p_id) = pol.paises().iter().find(|p| p.tag == pais).map(|p| p.id.clone()) {
                        if let Some(p_pol) = pol.pais_mut(&p_id) {
                            if let Some(gi_nome) = mod_nome.strip_prefix("aprovacao_") {
                                if let Some(gi) = p_pol.grupos_interesse.get_mut(gi_nome) {
                                    gi.modificadores_temporarios.push(sim_politica::ModificadorTemporario {
                                        id: format!("evento_{mod_nome}"),
                                        valor: m.valor as f64,
                                        dias_restantes: dias,
                                    });
                                }
                            }
                        }
                    }
                }
                return;
            }
        }
    }

    // 3. Mudar Leis: "mudar_lei:<lei>", "mudar_lei:<cat>:<lei>", "lei:<lei>", "promulgar_lei:<lei>"
    if id.starts_with("mudar_lei:") || id.starts_with("lei:") || id.starts_with("promulgar_lei:") {
        let espec = id
            .strip_prefix("mudar_lei:")
            .or_else(|| id.strip_prefix("lei:"))
            .or_else(|| id.strip_prefix("promulgar_lei:"))
            .unwrap_or(id);

        let (cat_opt, lei_id) = if let Some((c, l)) = espec.split_once(':') {
            (Some(c), l)
        } else {
            (None, espec)
        };

        let cat_enum = categoria_da_lei(lei_id);
        let cat_str = cat_opt.unwrap_or_else(|| cat_enum.chave());

        // A) Atualiza simulação clássica (MundoJogo)
        if let Some(p) = ctx.mundo.paises.get_mut(pais) {
            p.politica.leis_ativas.insert(cat_enum, lei_id.to_string());
            p.politica.recalcular_legitimidade();
        }

        // B) Atualiza sim_politica
        if let Some(pol) = &mut ctx.politica {
            if let Some(p_id) = pol.paises().iter().find(|p| p.tag == pais).map(|p| p.id.clone()) {
                if let Some(p_pol) = pol.pais_mut(&p_id) {
                    p_pol.promulgar_lei_instantanea(cat_str, lei_id);
                }
            }
        }

        // C) Atualiza sim_economia se for lei econômica
        if let Some(eco) = &mut ctx.economia {
            let eco_pais_id = eco
                .paises()
                .iter()
                .find(|p| p.tag.as_deref() == Some(pais))
                .map(|p| p.id.clone());
            if let Some(ep_id) = eco_pais_id {
                let _ = eco.promulgar_lei(&ep_id, lei_id);
            }
        }

        est.registrar_mudanca_lei(pais, cat_str, lei_id);
        return;
    }

    // 4. Flags de país: "flag:<nome>", "definir_flag:<nome>", "remover_flag:<nome>"
    if let Some(f) = id.strip_prefix("flag:").or_else(|| id.strip_prefix("definir_flag:")) {
        if m.valor > 0.0 {
            est.definir_flag(pais, f);
        } else {
            est.remover_flag(pais, f);
        }
        return;
    }
    if let Some(f) = id.strip_prefix("remover_flag:") {
        est.remover_flag(pais, f);
        return;
    }

    // 5. Variáveis customizadas de país: "var:<nome>", "variavel:<nome>"
    if let Some(v_nome) = id.strip_prefix("var:").or_else(|| id.strip_prefix("variavel:")) {
        est.ajustar_variavel(pais, v_nome, m.valor);
        return;
    }

    // 6. Relações Diplomáticas: "relacao:<TAG>", "relacao:<TAG1>:<TAG2>"
    if let Some(resto) = id.strip_prefix("relacao:") {
        let (p_a, p_b) = if let Some((a, b)) = resto.split_once(':') {
            (a, b)
        } else {
            (pais, resto)
        };
        est.ajustar_relacao(p_a, p_b, m.valor);
        if let Some(dipl) = &mut ctx.diplomacia {
            let atual = dipl.relacao(p_a, p_b);
            dipl.definir_relacao(p_a, p_b, atual + m.valor);
        }
        return;
    }

    // 7. Efeitos Sistêmicos Específicos & Ligação com sim_politica e sim_economia
    match id {
        "tensao_guerra_fria" => {
            ctx.mundo.tensao_mundial = (ctx.mundo.tensao_mundial + m.valor * 10.0).clamp(0.0, 100.0);
            if let Some(dipl) = &mut ctx.diplomacia {
                dipl.tensao = (dipl.tensao + m.valor * 10.0).clamp(0.0, 100.0);
            }
        }

        "legitimidade_base"
        | "estabilidade_politica"
        | "autoridade_estatal"
        | "legitimidade_internacional"
        | "legitimidade_soberania_popular" => {
            let k = if id == "legitimidade_internacional" { 0.25 } else { 1.0 };
            if let Some(p) = ctx.mundo.paises.get_mut(pais) {
                p.politica.legitimidade = (p.politica.legitimidade + m.valor * k).clamp(0.0, 100.0);
            }
            if let Some(pol) = &mut ctx.politica {
                if let Some(p_id) = pol.paises().iter().find(|p| p.tag == pais).map(|p| p.id.clone()) {
                    if let Some(p_pol) = pol.pais_mut(&p_id) {
                        p_pol.legitimidade = (p_pol.legitimidade + (m.valor * k) as f64).clamp(0.0, 100.0);
                    }
                }
            }
        }

        "tesouro" => {
            if let Some(p) = ctx.mundo.paises.get_mut(pais) {
                p.orcamento.tesouro += m.valor as f64;
            }
            if let Some(eco) = &mut ctx.economia {
                let eco_id = eco.paises().iter().find(|p| p.tag.as_deref() == Some(pais)).map(|p| p.id.clone());
                if let Some(eid) = eco_id {
                    if let Some(p) = eco.obter_pais_mut(&eid) {
                        p.orcamento.tesouro += m.valor as f64;
                    }
                }
            }
        }

        "crescimento_pib" => {
            if let Some(p) = ctx.mundo.paises.get_mut(pais) {
                p.orcamento.tesouro += p.pib_anual_estimado * m.valor as f64 * 0.1;
            }
            if let Some(eco) = &mut ctx.economia {
                let eco_id = eco.paises().iter().find(|p| p.tag.as_deref() == Some(pais)).map(|p| p.id.clone());
                if let Some(eid) = eco_id {
                    if let Some(p) = eco.obter_pais_mut(&eid) {
                        p.orcamento.tesouro += p.pib_diario * 365.0 * (m.valor as f64) * 0.05;
                    }
                }
            }
        }

        "investimento_privado_fator" => {
            if let Some(eco) = &mut ctx.economia {
                let eco_id = eco.paises().iter().find(|p| p.tag.as_deref() == Some(pais)).map(|p| p.id.clone());
                if let Some(eid) = eco_id {
                    if let Some(p) = eco.obter_pais_mut(&eid) {
                        p.fundo_investimento_privado += 500.0 * (m.valor as f64);
                    }
                }
            }
        }

        "arrecadacao_tributaria_comercial" => {
            if let Some(eco) = &mut ctx.economia {
                let eco_id = eco.paises().iter().find(|p| p.tag.as_deref() == Some(pais)).map(|p| p.id.clone());
                if let Some(eid) = eco_id {
                    if let Some(p) = eco.obter_pais_mut(&eid) {
                        p.eficiencia_arrecadacao = (p.eficiencia_arrecadacao + m.valor as f64).clamp(0.1, 2.0);
                    }
                }
            }
        }

        "custo_construcao_estatal" => {
            if let Some(eco) = &mut ctx.economia {
                let eco_id = eco.paises().iter().find(|p| p.tag.as_deref() == Some(pais)).map(|p| p.id.clone());
                if let Some(eid) = eco_id {
                    if let Some(p) = eco.obter_pais_mut(&eid) {
                        p.pontos_construcao_base = (p.pontos_construcao_base * (1.0 - m.valor as f64)).max(1.0);
                    }
                }
            }
        }

        // Aprovação e Clout de Grupos de Interesse em sim_politica
        outro if outro.starts_with("aprovacao_") => {
            let gi_nome = outro.trim_start_matches("aprovacao_");
            if let Some(pol) = &mut ctx.politica {
                if let Some(p_id) = pol.paises().iter().find(|p| p.tag == pais).map(|p| p.id.clone()) {
                    if let Some(p_pol) = pol.pais_mut(&p_id) {
                        if let Some(gi) = p_pol.grupos_interesse.get_mut(gi_nome) {
                            gi.aprovacao = (gi.aprovacao + m.valor as f64).clamp(-20.0, 20.0);
                        }
                    }
                }
            }
        }

        outro if outro.starts_with("forca_") => {
            let gi_nome = outro.trim_start_matches("forca_");
            if let Some(pol) = &mut ctx.politica {
                if let Some(p_id) = pol.paises().iter().find(|p| p.tag == pais).map(|p| p.id.clone()) {
                    if let Some(p_pol) = pol.pais_mut(&p_id) {
                        if let Some(gi) = p_pol.grupos_interesse.get_mut(gi_nome) {
                            gi.clout = (gi.clout + m.valor as f64).clamp(0.01, 1.0);
                        }
                    }
                }
            }
        }

        // Radicalismo em sim_politica
        outro if outro.starts_with("radicalismo_") => {
            if let Some(pol) = &mut ctx.politica {
                if let Some(p_id) = pol.paises().iter().find(|p| p.tag == pais).map(|p| p.id.clone()) {
                    if let Some(p_pol) = pol.pais_mut(&p_id) {
                        let fator = (1.0 + m.valor as f64).clamp(0.5, 2.5);
                        for pop in &mut p_pol.pops {
                            pop.radicais = (pop.radicais * fator).min(pop.populacao);
                        }
                    }
                }
            }
        }

        _ => {
            est.avisar_uma_vez(&format!("efeito:{}", m.id));
        }
    }
}
