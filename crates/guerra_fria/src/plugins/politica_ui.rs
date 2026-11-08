//! Aba POLÍTICA (1) estilo Victoria 3: GIs, partidos e eleições, leis, legitimidade e movimentos.
use super::politica_sim::{ComandoPol, PoliticaSim};
use bevy_egui::egui;
use sim_politica::{FaseLegislativa, HumorGrupoInteresse, PapelGrupoInteresse, PaisPolitica};

fn cor_aprovacao(a: f64) -> egui::Color32 {
    if a >= 5.0 {
        ui_refinada::menu::SUCESSO
    } else if a <= -5.0 {
        ui_refinada::menu::PERIGO
    } else {
        ui_refinada::menu::AVISO
    }
}

/// Cabeçalho de coluna em caixa alta, na tinta suave do cartaz.
fn cab(ui: &mut egui::Ui, h: &str) {
    ui.label(egui::RichText::new(h.to_uppercase()).size(12.0).strong().color(ui_refinada::menu::TINTA_SUAVE));
}

fn humor_pt(h: HumorGrupoInteresse) -> &'static str {
    match h {
        HumorGrupoInteresse::Radical => "radical",
        HumorGrupoInteresse::Irritado => "irritado",
        HumorGrupoInteresse::Neutro => "neutro",
        HumorGrupoInteresse::Feliz => "feliz",
        HumorGrupoInteresse::Leal => "leal",
    }
}

fn fase_pt(f: FaseLegislativa) -> &'static str {
    match f {
        FaseLegislativa::Introducao => "Introdução",
        FaseLegislativa::Comite => "Comitê",
        FaseLegislativa::VotacaoFinal => "Votação final",
    }
}

pub fn aba_politica(ui: &mut egui::Ui, pol: &mut PoliticaSim) {
    let Some(p) = pol.vista.pais.clone() else {
        ui.label("Sem dados políticos para este país (ou a simulação ainda está calculando).");
        return;
    };
    let mut cmds: Vec<ComandoPol> = Vec::new();
    cabecalho(ui, &p, &pol.vista.data.map(|d| d.to_string()).unwrap_or_default());
    ui.add_space(8.0);
    leis(ui, pol, &p, &mut cmds);
    ui.add_space(8.0);
    grupos(ui, &p);
    ui.add_space(8.0);
    partidos(ui, &p);
    ui.add_space(8.0);
    movimentos(ui, &p);
    for c in cmds {
        pol.enviar(c);
    }
}

fn cabecalho(ui: &mut egui::Ui, p: &PaisPolitica, data: &str) {
    ui_refinada::faixa_titulo(ui, &format!("Política de {}", p.nome), None, None);
    ui.label(egui::RichText::new(data).small().color(ui_refinada::menu::TINTA_SUAVE));
    let l = p.legitimidade;
    let cor = if l >= 60.0 { ui_refinada::menu::SUCESSO } else if l >= 35.0 { ui_refinada::menu::AVISO } else { ui_refinada::menu::PERIGO };
    ui_refinada::barra_status(
        ui,
        Some("Legitimidade"),
        l as f32,
        100.0,
        cor,
        160.0,
        Some(&format!("{l:.0}% · {:?}", p.faixa_legitimidade)),
    );
    if let Some(c) = &p.chefe_de_estado {
        ui_refinada::chapa(
            ui,
            "coroa",
            "Chefe de Estado",
            &format!("{} · popularidade {:.0}%", c.nome, c.popularidade.clamp(0.0, 100.0)),
            None,
        );
    }
    let pop = p.populacao_total().max(1.0);
    ui.label(format!(
        "Radicalismo: {:.1}% da população radical, {:.1}% leal | padrão de vida médio {:.1}",
        100.0 * p.total_radicais() / pop,
        100.0 * p.total_leais() / pop,
        p.padrao_de_vida_medio()
    ));
    if !p.crises_ativas.is_empty() {
        for c in p.crises_ativas.iter().filter(|c| !c.resolvida) {
            ui.colored_label(ui_refinada::menu::PERIGO, format!("Crise: {:?} (intensidade {:.0}%, {} dias)", c.tipo_crise, c.intensidade * 100.0, c.dias_ativa));
        }
    }
}

fn grupos(ui: &mut egui::Ui, p: &PaisPolitica) {
    ui_refinada::titulo_secao(ui, "Grupos de interesse");
    let mut gis: Vec<_> = p.grupos_interesse.values().collect();
    gis.sort_by(|a, b| b.clout.total_cmp(&a.clout));
    egui::Grid::new("pol_gis").striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
        for h in ["Grupo", "Líder", "Influência", "", "Aprovação", "Humor", "Papel"] {
            cab(ui, h);
        }
        ui.end_row();
        for g in gis {
            ui.label(&g.nome);
            ui.label(g.lider.as_ref().map_or("-".to_string(), |l| l.nome.clone()));
            ui_refinada::barra_trilho(ui, (g.clout * 4.0) as f32, ui_refinada::menu::OURO, 46.0, 9.0);
            ui.label(format!("{:.1}%", g.clout * 100.0));
            ui.colored_label(cor_aprovacao(g.aprovacao), format!("{:+.1}", g.aprovacao));
            ui.label(humor_pt(g.humor));
            match g.papel {
                PapelGrupoInteresse::NoGoverno => ui.colored_label(ui_refinada::menu::SUCESSO, "no governo"),
                PapelGrupoInteresse::NaOposicao => ui.label("oposição"),
                PapelGrupoInteresse::Marginalizado => ui.weak("marginalizado"),
            };
            ui.end_row();
        }
    });
}

fn partidos(ui: &mut egui::Ui, p: &PaisPolitica) {
    ui_refinada::titulo_secao(ui, "Partidos e eleições");
    match p.dias_para_proxima_eleicao {
        0 => ui.label("Eleição iminente."),
        d => ui.label(format!("Próxima eleição em {d} dias (~{:.1} anos){}", d as f32 / 365.0, if p.campanha_eleitoral.as_ref().is_some_and(|c| c.em_andamento) { " - CAMPANHA EM ANDAMENTO" } else { "" })),
    };
    let mut ps: Vec<_> = p.partidos.values().collect();
    ps.sort_by(|a, b| b.cadeiras_parlamento.cmp(&a.cadeiras_parlamento).then(b.momento_campanha.total_cmp(&a.momento_campanha)));
    if let Some(e) = &p.ultima_eleicao {
        let venc = e.partido_vencedor_id.as_ref().and_then(|v| p.partidos.get(v)).map_or("-".to_string(), |x| x.nome.clone());
        ui.label(format!("Última eleição ({}): vencedor {venc}; participação {:.0}%", e.data, e.taxa_participacao * 100.0));
    } else {
        ui.label("Nenhuma eleição realizada ainda no jogo.");
    }
    let max_votos = ps.iter().map(|x| x.percentual_votos_ultima_eleicao).fold(0.0_f64, f64::max).max(0.01);
    egui::Grid::new("pol_partidos").striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
        for h in ["Partido", "Votos", "", "Cadeiras", "Momento"] {
            cab(ui, h);
        }
        ui.end_row();
        for pt in ps {
            ui.label(&pt.nome);
            ui.label(format!("{:.1}%", pt.percentual_votos_ultima_eleicao * 100.0));
            ui_refinada::barra_trilho(ui, (pt.percentual_votos_ultima_eleicao / max_votos) as f32, ui_refinada::menu::AZUL_MARINHO, 60.0, 9.0);
            ui.label(pt.cadeiras_parlamento.to_string());
            ui.colored_label(ui_refinada::cor_variacao(pt.momento_campanha), format!("{:+.2}", pt.momento_campanha));
            ui.end_row();
        }
    });
}

fn leis(ui: &mut egui::Ui, pol: &PoliticaSim, p: &PaisPolitica, cmds: &mut Vec<ComandoPol>) {
    let cat = &pol.catalogo;
    ui_refinada::titulo_secao(ui, "Leis e tramitação legislativa");
    let nome_lei = |id: &str| cat.leis.get(id).map_or(id.to_string(), |l| l.nome.clone());
    if let Some(t) = &p.lei_em_tramitacao {
        ui_refinada::moldura_cartao().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui_refinada::glifo_ui(ui, "balanca", 15.0, Some(ui_refinada::menu::AVISO));
                ui.colored_label(ui_refinada::menu::AVISO, format!("Em tramitação: {} -> {}", nome_lei(&t.lei_anterior_id), nome_lei(&t.lei_alvo_id)));
            });
            ui.horizontal(|ui| {
                ui.label(format!("{} · {} dias na fase · entraves {}/3", fase_pt(t.fase), t.dias_na_fase, t.contagem_entraves));
                ui_refinada::barra_trilho(ui, (t.progresso_fase / 100.0) as f32, ui_refinada::menu::AZUL_MARINHO, 140.0, 11.0);
            });
            ui.label(format!("Chance de avançar {:.0}% | debate {:.0}% | travar {:.0}%", t.chance_avanco * 100.0, t.chance_debate * 100.0, t.chance_travar * 100.0));
            if !p.faixa_legitimidade.permite_tramitacao_leis() {
                ui.colored_label(ui_refinada::menu::PERIGO, "Governo ilegítimo: paralisia legislativa.");
            }
            if ui_refinada::botao_estilo_secundario(ui, "Retirar proposta").clicked() {
                cmds.push(ComandoPol::CancelarLei);
            }
        });
    } else {
        ui.weak("Nenhuma lei em tramitação (só uma por vez).");
    }
    let livre = p.lei_em_tramitacao.is_none();
    for (cid, c) in &cat.categorias_leis {
        let atual = p.leis_ativas.get(cid).cloned().unwrap_or_default();
        egui::CollapsingHeader::new(
            egui::RichText::new(format!("{} · {}", c.nome.to_uppercase(), nome_lei(&atual)))
                .family(ui_refinada::familia_oswald(ui.ctx()))
                .size(15.0)
                .color(ui_refinada::menu::AZUL_MARINHO),
        )
        .id_salt(("pol_cat", cid))
        .show(ui, |ui| {
            for l in &c.leis {
                if l.id == atual {
                    continue;
                }
                let faltam: Vec<&String> = l.pre_requisitos.leis_obrigatorias.iter().filter(|x| !p.leis_ativas.values().any(|v| v == *x)).collect();
                let proibidas: Vec<&String> = l.pre_requisitos.leis_proibidas.iter().filter(|x| p.leis_ativas.values().any(|v| v == *x)).collect();
                let ok = faltam.is_empty() && proibidas.is_empty();
                let apoio: f64 = l.apoiadores.iter().filter_map(|g| p.grupos_interesse.get(g)).map(|g| g.clout).sum();
                let oposicao: f64 = l.opositores.iter().filter_map(|g| p.grupos_interesse.get(g)).map(|g| g.clout).sum();
                ui.horizontal_wrapped(|ui| {
                    let b = ui.add_enabled(livre && ok, egui::Button::new("Propor lei"));
                    if b.clicked() {
                        cmds.push(ComandoPol::ProporLei { categoria: cid.clone(), lei: l.id.clone() });
                    }
                    if !ok {
                        b.on_disabled_hover_text("Pré-requisitos não atendidos");
                    }
                    ui_refinada::glifo_ui(ui, ui_refinada::chave_para(&l.id), 14.0, None);
                    ui.strong(&l.nome);
                    ui_refinada::barra_trilho(ui, (apoio * 3.0) as f32, ui_refinada::menu::SUCESSO, 44.0, 8.0);
                    ui_refinada::barra_trilho(ui, (oposicao * 3.0) as f32, ui_refinada::menu::PERIGO, 44.0, 8.0);
                    ui.label(format!("apoio {:.0}% / oposição {:.0}%", apoio * 100.0, oposicao * 100.0));
                    let chance = (apoio * 0.65 - oposicao * 0.5 + 0.35).clamp(0.05, 0.95);
                    ui.weak(format!("chance estimada {:.0}%", chance * 100.0));
                })
                .response
                .on_hover_text(&l.descricao);
            }
        });
    }
}

fn movimentos(ui: &mut egui::Ui, p: &PaisPolitica) {
    ui_refinada::titulo_secao(ui, "Movimentos políticos");
    if p.movimentos.is_empty() {
        ui.weak("Nenhum movimento ativo.");
        return;
    }
    egui::Grid::new("pol_mov").striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
        for h in ["Movimento", "Apoio", "Radicalismo", "Termômetro", "Revolução"] {
            cab(ui, h);
        }
        ui.end_row();
        for m in &p.movimentos {
            ui.label(&m.nome);
            ui.label(format!("{:.0}%", m.apoio * 100.0));
            ui.label(format!("{:.0}%", m.radicalismo * 100.0));
            ui_refinada::barra_trilho(ui, (m.termometro_revolucao / 100.0) as f32, ui_refinada::menu::VERMELHO, 70.0, 9.0);
            if m.em_revolucao {
                ui.colored_label(ui_refinada::menu::PERIGO, "EM REVOLUÇÃO");
            } else {
                ui.label(format!("{:.0}", m.termometro_revolucao));
            }
            ui.end_row();
        }
    });
}
