//! IA militar: guerras históricas por gatilho, alianças (OTAN / Pacto de Varsóvia),
//! escolha de alvos e postura dos exércitos, aceitação de paz pelo perdedor e notícias no Jornal.

use super::diplomacia_ui::DiploSim;
use super::economy::RecursoMundo;
use super::eventos_ui::EstadoEv;
use super::guerra_sim::{DadosGuerra, GuerraSim, acao_declarar_guerra, acao_propor_paz, normalizar_tag};
use super::jogador::PaisJogador;
use super::jogador::Notificacoes;
use super::time::EventoTickDiario;
use bevy::prelude::*;
use eventos::Noticia;
use sim_diplomacia::{Bloco, Diplomacia, TipoPacto};
use sim_guerra::{Divisao, Exercito, PaisMilitar, PosturaOperacional};
use std::collections::BTreeSet;

/// Memória da IA militar.
#[derive(Resource, Default)]
pub struct IaGuerra {
    /// Ids de guerras históricas já disparadas.
    pub disparadas: BTreeSet<String>,
    pub dias: u32,
}

pub struct GuerraIaPlugin;

impl Plugin for GuerraIaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<IaGuerra>().add_systems(
            Update,
            tick_ia_guerra
                .run_if(resource_exists::<GuerraSim>)
                .run_if(resource_exists::<DiploSim>)
                .run_if(resource_exists::<PaisJogador>)
                .run_if(crate::estado::em_partida),
        );
    }
}

/// Aliados de `tag`: pactos de defesa/aliança bilaterais e membros da mesma OTAN / Pacto de Varsóvia.
pub fn aliados_de(d: &Diplomacia, tag: &str) -> Vec<String> {
    let mut v: BTreeSet<String> = BTreeSet::new();
    for p in d.pactos_de(tag) {
        if matches!(p.tipo, TipoPacto::Alianca | TipoPacto::PactoDefesa) {
            v.insert(if p.a == tag { p.b.clone() } else { p.a.clone() });
        }
    }
    for org in ["otan", "pacto_varsovia"] {
        if d.em_org(org, tag) {
            for o in d.orgs.iter().filter(|o| o.id == org) {
                v.extend(o.membros.iter().cloned());
            }
        }
    }
    v.remove(tag);
    v.into_iter().map(|t| normalizar_tag(&t)).collect()
}

fn em_guerra_com(g: &GuerraSim, a: &str, b: &str) -> bool {
    g.0.guerras().iter().any(|w| w.ativa && w.sao_inimigos(a, b))
}

fn existe(g: &GuerraSim, tag: &str) -> bool {
    g.0.paises().iter().any(|p| p.tag == tag)
}

/// Declara guerra e, se `blocos`, convoca os aliados de cada lado (que não estejam do lado oposto).
pub fn declarar_com_aliados(g: &mut GuerraSim, d: &Diplomacia, atk: &str, def: &str, nome: &str, blocos: bool) -> Result<String, String> {
    let id = acao_declarar_guerra(g, atk, def, nome)?;
    if blocos {
        let atk = normalizar_tag(atk);
        let def = normalizar_tag(def);
        let aa = aliados_de(d, &atk);
        let ad = aliados_de(d, &def);
        let tags: Vec<String> = g.0.paises().iter().map(|p| p.tag.clone()).collect();
        if let Some(w) = g.0.cenario.guerras.iter_mut().find(|w| w.id == id) {
            for t in aa.into_iter().filter(|t| tags.contains(t) && *t != def && !ad.contains(t)) {
                if !w.atacantes.contains(&t) {
                    w.atacantes.push(t);
                }
            }
            for t in ad.into_iter().filter(|t| tags.contains(t) && !w.atacantes.contains(t)) {
                if !w.defensores.contains(&t) {
                    w.defensores.push(t);
                }
            }
        }
    }
    Ok(id)
}

fn noticia(ev: &mut EstadoEv, mundo: &RecursoMundo, texto: String) {
    ev.estado.jornal.push(Noticia { data: (mundo.0.data.ano, mundo.0.data.mes, mundo.0.data.dia), texto });
    if ev.estado.jornal.len() > 200 {
        ev.estado.jornal.remove(0);
    }
}

/// Dias para a IA reavaliar alvos, posturas e paz.
const PERIODO_IA: u32 = 7;

fn tick_ia_guerra(
    mut leitor: MessageReader<EventoTickDiario>,
    mut g: ResMut<GuerraSim>,
    mut ia: ResMut<IaGuerra>,
    mut diplo: ResMut<DiploSim>,
    dados: Option<Res<DadosGuerra>>,
    mundo: Res<RecursoMundo>,
    jogador: Res<PaisJogador>,
    op: Option<Res<super::smoke::OpcoesCli>>,
    mut ev: Option<ResMut<EstadoEv>>,
    notificacoes: Option<ResMut<Notificacoes>>,
) {
    let mut houve_dia_1 = false;
    let mut n = 0usize;
    for tick in leitor.read() {
        n += 1;
        if tick.nova_data.dia == 1 {
            houve_dia_1 = true;
        }
    }
    if n == 0 {
        return;
    }
    let eh_espectador = op.as_ref().is_some_and(|o| o.espectador_ativo());
    let eu = normalizar_tag(&jogador.0);
    let data = (mundo.0.data.ano as i64, mundo.0.data.mes as i64, mundo.0.data.dia as i64);
    let mut avisos: Vec<String> = Vec::new();

    // 1. Guerras históricas por gatilho de data e tensão.
    if let Some(dados) = dados {
        for h in &dados.0.guerras_historicas {
            if ia.disparadas.contains(&h.id) {
                continue;
            }
            let c = &h.gatilho_disparo.condicao_data;
            if data < (c.ano as i64, c.mes as i64, c.dia as i64) {
                continue;
            }
            if diplo.0.tensao < h.gatilho_disparo.condicao_tensao_minima {
                continue;
            }
            if h.id == "guerra_civil_chinesa" {
                criar_rebelde_chines(&mut g);
            }
            let lado_a: Vec<String> = h.beligerantes_lado_a.iter().map(|b| normalizar_tag(&b.tag)).filter(|t| existe(&g, t)).collect();
            let lado_b: Vec<String> = h.beligerantes_lado_b.iter().map(|b| normalizar_tag(&b.tag)).filter(|t| existe(&g, t)).collect();
            if lado_a.is_empty() || lado_b.is_empty() || em_guerra_com(&g, &lado_a[0], &lado_b[0]) {
                continue;
            }
            // Consequência em cadeia: se ambos os beligerantes principais compartilham o mesmo bloco ideológico,
            // o conflito por procuração bipolar é evitado pacificamente.
            let bloco_a = diplo.0.bloco(&lado_a[0]);
            let bloco_b = diplo.0.bloco(&lado_b[0]);
            if bloco_a == bloco_b && bloco_a != Bloco::NaoAlinhado {
                ia.disparadas.insert(h.id.clone());
                avisos.push(format!("CONFLITO EVITADO: {} não eclode pois ambos os lados compartilham alinhamento geopolítico.", h.nome));
                continue;
            }
            ia.disparadas.insert(h.id.clone());
            // Lado B é o agressor nos conflitos de invasão (ex.: Coreia do Norte); quando o gatilho é de
            // insurgência ou guerra civil, o lado A é o governo. Usamos o lado B como atacante.
            match acao_declarar_guerra(&mut g, &lado_b[0], &lado_a[0], &h.nome) {
                Ok(id) => {
                    if let Some(w) = g.0.cenario.guerras.iter_mut().find(|w| w.id == id) {
                        // Aliados só ingressam se forem coerentes com o alinhamento real do momento
                        for b_aliado in lado_b.iter().skip(1) {
                            if diplo.0.bloco(b_aliado) != bloco_a || diplo.0.bloco(b_aliado) == Bloco::NaoAlinhado {
                                w.atacantes.push(b_aliado.clone());
                            }
                        }
                        for a_aliado in lado_a.iter().skip(1) {
                            if diplo.0.bloco(a_aliado) != bloco_b || diplo.0.bloco(a_aliado) == Bloco::NaoAlinhado {
                                w.defensores.push(a_aliado.clone());
                            }
                        }
                    }
                    diplo.0.tensao = (diplo.0.tensao + h.impacto_tensao_mundial_vitoria.abs().min(10.0)).min(100.0);
                    avisos.push(format!("GUERRA: {} eclode. {}", h.nome, h.gatilho_disparo.descricao));
                }
                Err(e) => warn!("guerra historica {}: {e}", h.id),
            }
        }
    }

    ia.dias += n as u32;

    // 2. IA declara guerra: tensão ativa e rivalidade entre blocos ou hostilidade severa (1o dia de cada mês).
    if (mundo.0.data.dia == 1 || houve_dia_1) && diplo.0.tensao >= 20.0 {
        let ano_mes = (mundo.0.data.ano as u32).wrapping_mul(12) + mundo.0.data.mes as u32;
        let mut rivais: Vec<(String, String)> = diplo
            .0
            .pactos
            .iter()
            .filter(|p| p.tipo == TipoPacto::Rivalidade || p.tipo == TipoPacto::Embargo)
            .map(|p| (normalizar_tag(&p.a), normalizar_tag(&p.b)))
            .filter(|(a, b)| (eh_espectador || (*a != eu && *b != eu)) && existe(&g, a) && existe(&g, b) && !em_guerra_com(&g, a, b))
            .collect();

        if rivais.is_empty() {
            let tags: Vec<String> = diplo.0.paises.iter().map(|p| normalizar_tag(&p.tag)).collect();
            for i in 0..tags.len() {
                for j in (i + 1)..tags.len() {
                    let a = &tags[i];
                    let b = &tags[j];
                    if (!eh_espectador && (*a == eu || *b == eu)) || !existe(&g, a) || !existe(&g, b) || em_guerra_com(&g, a, b) {
                        continue;
                    }
                    let rel = diplo.0.relacao(a, b);
                    let opostos = (diplo.0.bloco(a) == Bloco::Ocidental && diplo.0.bloco(b) == Bloco::Sovietico)
                        || (diplo.0.bloco(a) == Bloco::Sovietico && diplo.0.bloco(b) == Bloco::Ocidental);
                    if rel <= -35.0 || (opostos && rel <= -15.0) {
                        rivais.push((a.clone(), b.clone()));
                        if rivais.len() >= 12 {
                            break;
                        }
                    }
                }
                if rivais.len() >= 12 {
                    break;
                }
            }
        }

        if !rivais.is_empty() {
            let (a, b) = rivais[(ano_mes as usize * 7919) % rivais.len()].clone();
            // Probabilidade proporcional à tensão (mínimo 10%, determinística pela data).
            let chance = ((diplo.0.tensao - 18.0) / 32.0 * 100.0).clamp(10.0, 75.0) as u32;
            if ano_mes.wrapping_mul(2654435761) % 100 < chance {
                let nome = format!("Guerra {} - {}", a, b);
                if let Ok(_) = declarar_com_aliados(&mut g, &diplo.0, &a, &b, &nome, true) {
                    diplo.0.tensao = (diplo.0.tensao + 6.0).min(100.0);
                    avisos.push(format!("GUERRA: {a} declara guerra a {b}; os respectivos blocos entram no conflito."));
                }
            }
        }
    }

    // 3. Postura, alvos e paz (periodicamente).
    if ia.dias >= PERIODO_IA {
        ia.dias = 0;
        let ativas: Vec<_> = g.0.guerras().iter().filter(|w| w.ativa).cloned().collect();
        for w in &ativas {
            // Postura e alvo dos exércitos controlados pela IA.
            let regs: Vec<_> = g.0.regioes().iter().map(|r| (r.adm1_code.clone(), r.dono_tag.clone(), r.controlador_tag.clone(), r.pontos_vitoria)).collect();
            for e in g.0.exercitos_mut() {
                if (!eh_espectador && e.pais_tag == eu) || !(w.eh_atacante(&e.pais_tag) || w.eh_defensor(&e.pais_tag)) {
                    continue;
                }
                let score = if w.eh_atacante(&e.pais_tag) { w.pontuacao_guerra } else { -w.pontuacao_guerra };
                e.postura = if score > 10.0 {
                    PosturaOperacional::AssaltoAgressivo
                } else if score < -25.0 {
                    PosturaOperacional::DefesaEmProfundidade
                } else {
                    PosturaOperacional::OfensivaEquilibrada
                };
                let inimigos: Vec<&String> = if w.eh_atacante(&e.pais_tag) { w.defensores.iter().collect() } else { w.atacantes.iter().collect() };
                let ja_ok = e.regiao_objetivo_adm1.as_ref().is_some_and(|o| regs.iter().any(|r| &r.0 == o && r.2 != e.pais_tag));
                if !ja_ok {
                    e.regiao_objetivo_adm1 = regs
                        .iter()
                        .filter(|r| inimigos.contains(&&r.1) && r.2 != e.pais_tag)
                        .max_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|r| r.0.clone());
                }
            }

            // Paz: pontuação decisiva ou impasse prolongado (>= 3 anos de guerra).
            let anos_guerra = (mundo.0.data.ano - w.data_inicio.ano as i32).max(0);
            let impasse = anos_guerra >= 3;
            let atk_perde = w.pontuacao_guerra <= -25.0 || w.capitulou_atacantes || (impasse && w.pontuacao_guerra < 0.0);
            let def_perde = w.pontuacao_guerra >= 25.0 || w.capitulou_defensores || (impasse && w.pontuacao_guerra >= 0.0);
            if !(atk_perde || def_perde || impasse) {
                continue;
            }
            let (vencedores, perdedores) = if def_perde { (&w.atacantes, &w.defensores) } else { (&w.defensores, &w.atacantes) };
            // A IA só decide quando o perdedor não é o jogador; se o vencedor é o jogador, a paz é dele.
            if !eh_espectador && (perdedores.contains(&eu) || vencedores.contains(&eu)) {
                continue;
            }
            let Some(prop) = vencedores.first().cloned() else { continue };
            if let Ok(t) = acao_propor_paz(&mut g, &w.id, &prop, true, !impasse) {
                let _ = t;
                if impasse && !w.capitulou_atacantes && !w.capitulou_defensores {
                    avisos.push(format!("ARMISTÍCIO: {} é encerrada por armistício após anos de combate.", w.nome_pt));
                } else {
                    avisos.push(format!("PAZ: {} termina; {} impõe os termos a {}.", w.nome_pt, prop, perdedores.join(", ")));
                }
            }
        }
    }

    if let Some(ev) = ev.as_deref_mut() {
        for a in &avisos {
            noticia(ev, &mundo, a.clone());
        }
    }
    if let Some(mut n) = notificacoes {
        for a in &avisos {
            n.push(Notificacoes::mensagem_modelada(a, &jogador.0), mundo.0.data.dia as f64);
        }
    }
}

/// Cria a facção comunista como beligerante militar antes da Guerra Civil Chinesa.
/// O país político/econômico continua sendo formado pelo evento de 1949; aqui a facção
/// precisa existir já em 1946 para que o gatilho histórico não seja descartado.
fn criar_rebelde_chines(g: &mut GuerraSim) {
    if existe(g, "CCP") {
        return;
    }
    let ids = ["951", "956", "961", "2119", "2223", "2224", "3796", "1813", "1828", "1839", "1804", "1811", "1814", "1818"];
    let regioes: Vec<String> = g
        .0
        .cenario
        .regioes
        .iter()
        .filter(|r| {
            ids.iter().any(|id| r.adm1_code == format!("prov_{id}") || r.adm1_code == format!("CHN-{id}"))
                || (r.dono_tag == "CHI" && (r.adm1_code == "prov_961" || r.adm1_code == "prov_956" || r.adm1_code == "prov_3796"))
        })
        .map(|r| r.adm1_code.clone())
        .collect();
    let capital = regioes.first().cloned().unwrap_or_else(|| "prov_961".to_string());
    for r in &mut g.0.cenario.regioes {
        if regioes.contains(&r.adm1_code) {
            r.dono_tag = "CCP".into();
            r.controlador_tag = "CCP".into();
        }
    }
    let pais_idx = g.0.cenario.paises.len();
    g.0.cenario.paises.push(PaisMilitar {
        tag: "CCP".into(),
        nome_pt: "Comunistas chineses".into(),
        capital_adm1: capital.clone(),
        doutrina_id: "doutrina_padrao".into(),
        tecnologias_ativas: Vec::new(),
        cansaco_guerra: 0.0,
        baixas_totais: 0,
        populacao_mobilizavel: 8_000_000,
        estoque_suprimento: 0.8,
        eficiencia_naval: 0.4,
    });
    g.0.mapa_paises.insert("CCP".into(), pais_idx);
    let div_id = "div_ccp_1".to_string();
    g.0.cenario.divisoes.push(Divisao {
        id: div_id.clone(),
        nome_pt: "Exército Popular de Libertação".into(),
        pais_tag: "CCP".into(),
        template_id: "divisao_infantaria_1945".into(),
        regiao_adm1: capital.clone(),
        efetivos_atuais: 60_000,
        equipamento_fracao: 0.8,
        organizacao: 40.0,
        moral: 0.9,
        entrincheiramento: 2.0,
        suprimento_atual: 0.8,
        experiencia: 0.4,
    });
    g.0.cenario.exercitos.push(Exercito {
        id: "exe_ccp_1".into(),
        nome_pt: "Exército Popular de Libertação".into(),
        pais_tag: "CCP".into(),
        comandante_id: None,
        divisoes_ids: vec![div_id],
        postura: PosturaOperacional::OfensivaEquilibrada,
        frente_id: None,
        regiao_objetivo_adm1: None,
    });
    g.0.invalidar_cache_territorial();
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn blocos_entram_juntos() {
        let mut d = Diplomacia::novo(vec![]);
        assert!(aliados_de(&d, "USA").is_empty());
        for o in d.orgs.iter_mut() {
            if o.id == "otan" || o.id == "pacto_varsovia" {
                o.formada = Some((1955, 5));
                o.membros = o.candidatos.iter().map(|t| t.to_string()).collect();
            }
        }
        let otan = aliados_de(&d, "USA");
        assert!(otan.contains(&"GBR".to_string()) && otan.contains(&"FRA".to_string()));
        assert!(!otan.contains(&"SOV".to_string()));
        let pv = aliados_de(&d, "SOV");
        assert!(pv.contains(&"POL".to_string()) && !pv.contains(&"USA".to_string()));
    }
}
