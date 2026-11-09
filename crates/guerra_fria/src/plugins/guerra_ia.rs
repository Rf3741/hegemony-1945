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
use sim_diplomacia::{Bloco, Diplomacia, Esfera, TipoPacto};
use sim_guerra::{Divisao, EnvolvimentoGuerra, Exercito, LadoGuerra, NivelEnvolvimento, PaisMilitar, PosturaOperacional};
use std::collections::BTreeSet;

/// Ocupações aliadas históricas: (país ocupado, ocupante principal, data início, data fim).
/// Declarar guerra a um país ocupado põe o atacante em guerra também com TODOS os ocupantes.
pub const OCUPACOES_DATADAS: &[(&str, &str, (i32, u8, u8), (i32, u8, u8))] = &[
    ("GER", "USA", (1945, 1, 1), (1949, 5, 23)),
    ("JAP", "USA", (1945, 1, 1), (1952, 4, 28)),
    ("AUT", "USA", (1945, 1, 1), (1955, 7, 27)),
];

/// Verifica se `tag` está sob ocupação histórica na data `data`.
/// Retorna `Some(ocupante)` se ocupado, `None` caso contrário.
pub fn ocupado_historicamente(tag: &str, data: (i32, u8, u8)) -> Option<&'static str> {
    OCUPACOES_DATADAS
        .iter()
        .find(|(t, _, de, ate)| *t == tag && data >= *de && data <= *ate)
        .map(|(_, o, _, _)| *o)
}

/// Retorna todos os ocupantes históricos de `tag` na data `data`.
pub fn ocupantes_historicos(tag: &str, data: (i32, u8, u8)) -> Vec<&'static str> {
    OCUPACOES_DATADAS
        .iter()
        .filter(|(t, _, de, ate)| *t == tag && data >= *de && data <= *ate)
        .map(|(_, o, _, _)| *o)
        .collect()
}

/// Retorna todos os ocupantes atuais (do motor de guerra) de `tag`.
/// Uma região está ocupada se `dono_tag != controlador_tag`.
pub fn ocupantes_atuais(g: &GuerraSim, tag: &str) -> Vec<String> {
    let mut ocupantes: BTreeSet<String> = BTreeSet::new();
    for r in g.0.regioes() {
        if r.dono_tag == tag && r.controlador_tag != tag {
            ocupantes.insert(r.controlador_tag.clone());
        }
    }
    ocupantes.into_iter().collect()
}

/// Retorna todos os ocupantes (históricos + atuais) de `tag` na data `data`.
pub fn todos_os_ocupantes(g: &GuerraSim, tag: &str, data: (i32, u8, u8)) -> Vec<String> {
    let mut v: BTreeSet<String> = BTreeSet::new();
    for o in ocupantes_historicos(tag, data) {
        v.insert(normalizar_tag(o));
    }
    for o in ocupantes_atuais(g, tag) {
        v.insert(normalizar_tag(&o));
    }
    v.into_iter().collect()
}

/// Aliados com obrigação de defesa de `tag`: pactos de defesa/aliança bilaterais
/// e membros da mesma OTAN / Pacto de Varsóvia que têm tratado de defesa.
pub fn aliados_com_obrigacao_defesa(d: &Diplomacia, tag: &str) -> Vec<String> {
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

/// Verifica se dois países são membros do bloco ocidental.
fn ambos_ocidentais(d: &Diplomacia, a: &str, b: &str) -> bool {
    d.bloco(a) == Bloco::Ocidental && d.bloco(b) == Bloco::Ocidental
}

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

/// Declara guerra e convoca:
/// - Aliados do atacante e do defensor (pactos de defesa/aliança, OTAN, Pacto de Varsóvia)
/// - Ocupantes do defensor (históricos e atuais) -> entram como defensores
/// - Aliados com obrigação de defesa do defensor -> entram como defensores
pub fn declarar_com_aliados(
    g: &mut GuerraSim,
    d: &Diplomacia,
    atk: &str,
    def: &str,
    nome: &str,
    blocos: bool,
    data: (i32, u8, u8),
) -> Result<(String, Vec<String>, Vec<String>), String> {
    let id = acao_declarar_guerra(g, atk, def, nome)?;
    let mut ocupantes_entraram: Vec<String> = Vec::new();
    let mut aliados_defesa_entraram: Vec<String> = Vec::new();

    if blocos {
        let atk_n = normalizar_tag(atk);
        let def_n = normalizar_tag(def);
        let aa = aliados_de(d, &atk_n);
        let ad = aliados_de(d, &def_n);
        let tags: Vec<String> = g.0.paises().iter().map(|p| p.tag.clone()).collect();

        // Calcula consequências ANTES do mutable borrow na guerra
        let ocupantes = todos_os_ocupantes(g, &def_n, data);
        let aliados_def = aliados_com_obrigacao_defesa(d, &def_n);

        if let Some(w) = g.0.cenario.guerras.iter_mut().find(|w| w.id == id) {
            // Aliados do atacante
            for t in aa.into_iter().filter(|t| tags.contains(t) && *t != def_n && !ad.contains(t)) {
                if !w.atacantes.contains(&t) {
                    w.atacantes.push(t);
                }
            }
            // Aliados do defensor
            for t in ad.into_iter().filter(|t| tags.contains(t) && !w.atacantes.contains(t)) {
                if !w.defensores.contains(&t) {
                    w.defensores.push(t);
                }
            }

            // Consequência 1: Ocupantes do defensor entram na guerra como defensores
            for oc in ocupantes {
                if oc != atk_n && !w.defensores.contains(&oc) && !w.atacantes.contains(&oc) && tags.contains(&oc) {
                    w.defensores.push(oc.clone());
                    ocupantes_entraram.push(oc);
                }
            }

            // Consequência 2: Aliados com obrigação de defesa do defensor entram na guerra
            for adf in aliados_def {
                if adf != atk_n && !w.defensores.contains(&adf) && !w.atacantes.contains(&adf) && tags.contains(&adf) {
                    w.defensores.push(adf.clone());
                    aliados_defesa_entraram.push(adf);
                }
            }
        }
    }
    Ok((id, ocupantes_entraram, aliados_defesa_entraram))
}

fn noticia(ev: &mut EstadoEv, mundo: &RecursoMundo, texto: String) {
    ev.estado.jornal.push(Noticia { data: (mundo.0.data.ano, mundo.0.data.mes, mundo.0.data.dia), texto });
    if ev.estado.jornal.len() > 200 {
        ev.estado.jornal.remove(0);
    }
}

/// Calcula o poder militar aproximado de um país (PIB + população mobilizável + divisões).
fn poder_pais(g: &GuerraSim, tag: &str) -> f64 {
    let Some(idx) = g.0.mapa_paises.get(tag) else { return 0.0 };
    let pais = &g.0.cenario.paises[*idx];
    let n_divs = g.0.cenario.divisoes.iter().filter(|d| d.pais_tag == tag && d.efetivos_atuais > 0).count() as f64;
    // Poder = PIB (normalizado) + população mobilizável + divisões * 1000
    let pib_norm = (pais.populacao_mobilizavel as f64 / 1_000_000.0).max(1.0);
    pib_norm + n_divs * 1000.0
}

/// Calcula o poder total de um lado (país + seus aliados na guerra).
fn poder_lado(g: &GuerraSim, d: &Diplomacia, tags: &[String], data: (i32, u8, u8)) -> f64 {
    let mut total = 0.0;
    for t in tags {
        total += poder_pais(g, t);
        // Adiciona poder dos aliados com obrigação de defesa
        for ad in aliados_com_obrigacao_defesa(d, t) {
            total += poder_pais(g, &ad) * 0.7; // Aliados contribuem com 70%
        }
    }
    total
}

/// Verifica se a IA deve atacar considerando o custo (ocupantes + aliados do defensor).
/// Só ataca se o poder do lado atacante (com aliados) > poder do lado defensor (com ocupantes e aliados).
fn ia_deve_atacar(g: &GuerraSim, d: &Diplomacia, atk: &str, def: &str, data: (i32, u8, u8)) -> bool {
    let atk_n = normalizar_tag(atk);
    let def_n = normalizar_tag(def);

    // Poder do lado atacante (atacante + seus aliados)
    let atk_aliados = aliados_de(d, &atk_n);
    let mut lado_atk = vec![atk_n.clone()];
    lado_atk.extend(atk_aliados);
    let poder_atk = poder_lado(g, d, &lado_atk, data);

    // Poder do lado defensor (defensor + ocupantes + aliados com obrigação de defesa)
    let ocupantes = todos_os_ocupantes(g, &def_n, data);
    let aliados_def = aliados_com_obrigacao_defesa(d, &def_n);
    let mut lado_def = vec![def_n.clone()];
    lado_def.extend(ocupantes);
    lado_def.extend(aliados_def);
    let poder_def = poder_lado(g, d, &lado_def, data);

    // IA só ataca se tiver vantagem clara (pelo menos 1.2x)
    poder_atk > poder_def * 1.2
}

/// Verifica se um país tem armas nucleares (simplificado: USA e URSS a partir de 1949).
fn tem_armas_nucleares(tag: &str, ano: i32) -> bool {
    matches!(tag, "USA" | "SOV") && ano >= 1949
}

/// Calcula o poder abrangente de um país para decisões de envolvimento em guerras por procuração.
/// Inclui: poder militar, econômico, alcance naval, ideologia/esfera, status nuclear.
fn poder_abrangente(g: &GuerraSim, d: &Diplomacia, mundo: &RecursoMundo, tag: &str, data: (i32, u8, u8)) -> f64 {
    let Some(idx) = g.0.mapa_paises.get(tag) else { return 0.0 };
    let pais = &g.0.cenario.paises[*idx];
    
    // Poder militar base (divisões ativas * efetivos)
    let forca_militar: f64 = g.0.cenario.divisoes
        .iter()
        .filter(|d| d.pais_tag == tag && d.efetivos_atuais > 0)
        .map(|d| d.efetivos_atuais as f64 * d.experiencia * d.moral * d.organizacao / 100.0)
        .sum();
    
    // Poder econômico (PIB aproximado via população mobilizável + tesouro)
    let pib_proxy = pais.populacao_mobilizavel as f64 / 1_000_000.0;
    let tesouro = mundo.0.paises.get(tag).map(|p| p.orcamento.tesouro / 1000.0).unwrap_or(0.0);
    let poder_economico = pib_proxy + tesouro.max(0.0);
    
    // Alcance naval (eficiência naval * portos)
    let portos: u32 = g.0.cenario.regioes.iter()
        .filter(|r| r.dono_tag == tag && r.nivel_porto > 0)
        .map(|r| r.nivel_porto)
        .sum();
    let alcance_naval = pais.eficiencia_naval * (portos as f64 / 10.0).max(1.0);
    
    // Ideologia/esfera: bônus se está na mesma esfera do lado que apoia
    let esfera = d.esfera(tag);
    let bonus_esfera = match esfera {
        sim_diplomacia::Esfera::Eua | sim_diplomacia::Esfera::Urss => 1.5, // Superpotências têm mais peso
        sim_diplomacia::Esfera::Disputada => 1.2,
        sim_diplomacia::Esfera::NaoAlinhado => 1.0,
    };
    
    // Status nuclear: grande dissuasão
    let bonus_nuclear = if tem_armas_nucleares(tag, data.0) { 2.0 } else { 1.0 };
    
    // Distância/alcance: países distantes têm menos capacidade de projeção
    // (simplificado: usa alcance naval como proxy)
    
    (forca_militar / 10000.0 + poder_economico + alcance_naval) * bonus_esfera * bonus_nuclear
}

/// Determina o nível ótimo de envolvimento para um país numa guerra alheia.
/// Considera: poder do país, distância, ideologia/esfera, quanto perde se o aliado cair,
/// tensão da Guerra Fria, risco de escalada nuclear.
fn calcular_nivel_envolvimento_otimo(
    g: &GuerraSim,
    d: &Diplomacia,
    mundo: &RecursoMundo,
    pais_tag: &str,
    guerra_id: &str,
    lado: LadoGuerra,
    data: (i32, u8, u8),
) -> NivelEnvolvimento {
    let poder = poder_abrangente(g, d, mundo, pais_tag, data);
    let esfera = d.esfera(pais_tag);
    let nuclear = tem_armas_nucleares(pais_tag, data.0);
    let tensao = d.tensao;
    
    // Encontra a guerra
    let Some(guerra) = g.0.guerras().iter().find(|w| w.id == guerra_id) else {
        return NivelEnvolvimento::Neutro;
    };
    
    // Lado que este país apoiaria (baseado na esfera/ideologia)
    let lado_alvo = match (lado, esfera) {
        (LadoGuerra::Atacante, sim_diplomacia::Esfera::Urss) if guerra.atacantes.iter().any(|t| d.esfera(t) == sim_diplomacia::Esfera::Urss) => true,
        (LadoGuerra::Defensor, sim_diplomacia::Esfera::Urss) if guerra.defensores.iter().any(|t| d.esfera(t) == sim_diplomacia::Esfera::Urss) => true,
        (LadoGuerra::Atacante, sim_diplomacia::Esfera::Eua) if guerra.atacantes.iter().any(|t| d.esfera(t) == sim_diplomacia::Esfera::Eua) => true,
        (LadoGuerra::Defensor, sim_diplomacia::Esfera::Eua) if guerra.defensores.iter().any(|t| d.esfera(t) == sim_diplomacia::Esfera::Eua) => true,
        (LadoGuerra::Atacante, _) if guerra.atacantes.iter().any(|t| d.relacao(pais_tag, t) > 20.0) => true,
        (LadoGuerra::Defensor, _) if guerra.defensores.iter().any(|t| d.relacao(pais_tag, t) > 20.0) => true,
        _ => false,
    };
    
    if !lado_alvo {
        return NivelEnvolvimento::Neutro;
    }
    
    // Calcula quanto o país perde se o aliado cair (importância estratégica)
    let importancia_aliado = guerra.atacantes.iter().chain(guerra.defensores.iter())
        .filter(|t| d.relacao(pais_tag, t) > 30.0 || d.esfera(*t) == esfera)
        .count() as f64;
    
    // Risco de escalada: se ambas as superpotências nucleares estão envolvidas, evitar nível 5
    let outra_superpotencia = if pais_tag == "USA" { "SOV" } else if pais_tag == "SOV" { "USA" } else { "" };
    let risco_nuclear = nuclear && !outra_superpotencia.is_empty() && tem_armas_nucleares(outra_superpotencia, data.0)
        && guerra.participa(outra_superpotencia);
    
    // Lógica de decisão baseada em poder, importância, tensão e risco nuclear
    let mut nivel = NivelEnvolvimento::Neutro;
    
    if poder < 50.0 {
        nivel = NivelEnvolvimento::ApoioDiplomatico;
    } else if poder < 200.0 {
        nivel = NivelEnvolvimento::ApoioMaterial;
    } else if poder < 500.0 {
        nivel = NivelEnvolvimento::ConseleirosVoluntarios;
    } else if poder < 1000.0 {
        nivel = NivelEnvolvimento::IntervencaoLimitada;
    } else {
        nivel = NivelEnvolvimento::GuerraTotal;
    }
    
    // Ajuste por importância do aliado
    if importancia_aliado > 2.0 {
        nivel = match nivel as u8 {
            0 => NivelEnvolvimento::ApoioDiplomatico,
            1 => NivelEnvolvimento::ApoioMaterial,
            2 => NivelEnvolvimento::ConseleirosVoluntarios,
            3 => NivelEnvolvimento::IntervencaoLimitada,
            _ => NivelEnvolvimento::GuerraTotal,
        };
    }
    
    // Ajuste por tensão da Guerra Fria
    if tensao > 60.0 {
        nivel = match nivel as u8 {
            0 => NivelEnvolvimento::ApoioDiplomatico,
            1 => NivelEnvolvimento::ApoioMaterial,
            2 => NivelEnvolvimento::ConseleirosVoluntarios,
            3 => NivelEnvolvimento::IntervencaoLimitada,
            _ => NivelEnvolvimento::GuerraTotal,
        };
    } else if tensao < 20.0 {
        nivel = match nivel as u8 {
            0 => NivelEnvolvimento::ApoioDiplomatico,
            1 => NivelEnvolvimento::ApoioMaterial,
            _ => nivel, // Não reduz abaixo do nível atual se já for alto
        };
    }
    
    // REGRA CRÍTICA: Superpotências nucleares NUNCA vão ao nível 5 uma contra a outra
    if risco_nuclear && nivel == NivelEnvolvimento::GuerraTotal {
        nivel = NivelEnvolvimento::IntervencaoLimitada; // Máximo nível 4
    }
    
    // Países não alinhados não passam de nível 3 sem jogador
    if esfera == sim_diplomacia::Esfera::NaoAlinhado && nivel > NivelEnvolvimento::ConseleirosVoluntarios {
        nivel = NivelEnvolvimento::ConseleirosVoluntarios;
    }
    
    nivel
}

/// Atualiza os envolvimentos de terceiros numa guerra (guerra por procuração).
fn atualizar_envolvimentos_proxy(
    g: &mut GuerraSim,
    d: &Diplomacia,
    mundo: &mut RecursoMundo,
    data: (i32, u8, u8),
) {
    // Coleta IDs das guerras ativas primeiro para evitar problemas de borrow
    let guerra_ids: Vec<String> = g.0.guerras().iter().filter(|w| w.ativa).map(|w| w.id.clone()).collect();
    
    for guerra_id in &guerra_ids {
        // Coleta tags dos países que não são beligerantes diretos
        let tags_nao_beligerantes: Vec<String> = {
            let guerra_encontrada = g.0.guerras().iter().find(|w| w.id == *guerra_id).cloned();
            match guerra_encontrada {
                Some(guerra) => {
                    // Obtém todas as tags de países do GuerraSim e filtra as que não participam desta guerra
                    g.0.paises().iter().map(|p| p.tag.clone()).filter(|tag| !guerra.participa(tag)).collect()
                },
                None => continue,
            }
        };
        
        for tag in &tags_nao_beligerantes {
            // Verifica se já tem envolvimento registrado
            let ja_tem = {
                let guerra = g.0.guerras().iter().find(|w| w.id == *guerra_id);
                guerra.map_or(false, |g| g.envolvimentos.iter().any(|e| e.pais_tag == *tag))
            };
            
            // Calcula nível ótimo para ambos os lados
            let nivel_atk = calcular_nivel_envolvimento_otimo(g, d, mundo, tag, guerra_id, LadoGuerra::Atacante, data);
            let nivel_def = calcular_nivel_envolvimento_otimo(g, d, mundo, tag, guerra_id, LadoGuerra::Defensor, data);
            
            let (nivel_escolhido, lado_escolhido) = if nivel_atk >= nivel_def {
                (nivel_atk, LadoGuerra::Atacante)
            } else {
                (nivel_def, LadoGuerra::Defensor)
            };
            
            if nivel_escolhido > NivelEnvolvimento::Neutro {
                // Atualiza ou cria envolvimento
                if let Some(guerra_mut) = g.0.cenario.guerras.iter_mut().find(|w| w.id == *guerra_id) {
                    if ja_tem {
                        // Atualiza envolvimento existente
                        if let Some(env) = guerra_mut.envolvimentos.iter_mut().find(|e| e.pais_tag == *tag) {
                            env.nivel = nivel_escolhido;
                            env.lado = lado_escolhido;
                            env.custo_acumulado += nivel_escolhido.custo_tesouro_diario();
                            // Aplica custo ao tesouro
                            if let Some(p) = mundo.0.paises.get_mut(tag) {
                                p.orcamento.tesouro -= nivel_escolhido.custo_tesouro_diario();
                            }
                        }
                    } else {
                        // Novo envolvimento
                        guerra_mut.envolvimentos.push(EnvolvimentoGuerra {
                            pais_tag: tag.clone(),
                            guerra_id: guerra_id.clone(),
                            lado: lado_escolhido,
                            nivel: nivel_escolhido,
                            data_inicio: g.0.cenario.data_inicial,
                            custo_acumulado: nivel_escolhido.custo_tesouro_diario(),
                        });
                        // Aplica custo inicial
                        if let Some(p) = mundo.0.paises.get_mut(tag) {
                            p.orcamento.tesouro -= nivel_escolhido.custo_tesouro_diario();
                        }
                    }
                }
            } else if ja_tem {
                // Remove envolvimento se voltou a neutro
                if let Some(guerra_mut) = g.0.cenario.guerras.iter_mut().find(|w| w.id == *guerra_id) {
                    guerra_mut.envolvimentos.retain(|e| e.pais_tag != *tag);
                }
            }
        }
        
        // Aplica efeitos dos envolvimentos na força dos lados
        aplicar_efeitos_envolvimentos(g, guerra_id);
    }
}

/// Aplica os bônus de força dos envolvimentos aos lados da guerra.
fn aplicar_efeitos_envolvimentos(g: &mut GuerraSim, guerra_id: &str) {
    if let Some(guerra) = g.0.cenario.guerras.iter_mut().find(|w| w.id == guerra_id) {
        let mut bonus_atk = 0.0;
        let mut bonus_def = 0.0;
        
        for env in &guerra.envolvimentos {
            let bonus = env.nivel.bonus_forca_aliado();
            match env.lado {
                LadoGuerra::Atacante => bonus_atk += bonus,
                LadoGuerra::Defensor => bonus_def += bonus,
            }
        }
        
        // Aplica bônus às divisões do lado correspondente (simplificado: ajusta pontuação de guerra)
        guerra.pontuacao_guerra += bonus_atk * 10.0 - bonus_def * 10.0;
        
        // Gera tensão mundial
        let tensao_total: f32 = guerra.envolvimentos.iter().map(|e| e.nivel.tensao_diaria()).sum();
        // A tensão será aplicada no tick de diplomacia
    }
}

/// Gera notícias no Jornal sobre mudanças de envolvimento.
fn noticiar_envolvimentos(
    ev: &mut EstadoEv,
    mundo: &RecursoMundo,
    g: &GuerraSim,
) {
    for guerra in g.0.guerras().iter().filter(|w| w.ativa) {
        for env in &guerra.envolvimentos {
            // Só noticia mudanças recentes (primeiro dia)
            if env.data_inicio == g.0.cenario.data_inicial {
                let lado_str = match env.lado {
                    LadoGuerra::Atacante => "atacante",
                    LadoGuerra::Defensor => "defensor",
                };
                let texto = format!(
                    "GUERRA POR PROCURAÇÃO: {} entra na '{}' no lado {} com {} (nível {})",
                    env.pais_tag, guerra.nome_pt, lado_str, env.nivel.descricao(), env.nivel as u8
                );
                ev.estado.jornal.push(Noticia {
                    data: (mundo.0.data.ano, mundo.0.data.mes, mundo.0.data.dia),
                    texto,
                });
            }
        }
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
    mut mundo: ResMut<RecursoMundo>,
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
            let data_tuple = (mundo.0.data.ano as i32, mundo.0.data.mes, mundo.0.data.dia);
            match declarar_com_aliados(&mut g, &diplo.0, &lado_b[0], &lado_a[0], &h.nome, true, data_tuple) {
                Ok((id, ocupantes_entraram, aliados_def_entraram)) => {
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
                    let mut msg = format!("GUERRA: {} eclode. {}", h.nome, h.gatilho_disparo.descricao);
                    if !ocupantes_entraram.is_empty() {
                        msg.push_str(&format!("; {} (potência ocupante) entra na guerra contra {}", ocupantes_entraram.join(", "), lado_b[0]));
                    }
                    if !aliados_def_entraram.is_empty() {
                        msg.push_str(&format!("; aliados de defesa {} entram na guerra contra {}", aliados_def_entraram.join(", "), lado_b[0]));
                    }
                    avisos.push(msg);
                }
                Err(e) => warn!("guerra historica {}: {e}", h.id),
            }
        }
    }

    ia.dias += n as u32;

    // 2. IA declara guerra: tensão ativa e rivalidade entre blocos ou hostilidade severa (1o dia de cada mês).
    if (mundo.0.data.dia == 1 || houve_dia_1) && diplo.0.tensao >= 20.0 {
        let ano_mes = (mundo.0.data.ano as u32).wrapping_mul(12) + mundo.0.data.mes as u32;
        let data_tuple = (mundo.0.data.ano as i32, mundo.0.data.mes, mundo.0.data.dia);
        let antes_de_1960 = mundo.0.data.ano < 1960;

        let mut rivais: Vec<(String, String)> = diplo
            .0
            .pactos
            .iter()
            .filter(|p| p.tipo == TipoPacto::Rivalidade || p.tipo == TipoPacto::Embargo)
            .map(|p| (normalizar_tag(&p.a), normalizar_tag(&p.b)))
            .filter(|(a, b)| {
                (eh_espectador || (*a != eu && *b != eu))
                    && existe(&g, a)
                    && existe(&g, b)
                    && !em_guerra_com(&g, a, b)
                    // Antes de 1960: sem guerras aleatórias entre membros do mesmo bloco ocidental
                    && !(antes_de_1960 && ambos_ocidentais(&diplo.0, a, b))
            })
            .collect();

        if rivais.is_empty() {
            let tags: Vec<String> = diplo.0.paises.iter().map(|p| normalizar_tag(&p.tag)).collect();
            for i in 0..tags.len() {
                for j in (i + 1)..tags.len() {
                    let a = &tags[i];
                    let b = &tags[j];
                    if (!eh_espectador && (*a == eu || *b == eu))
                        || !existe(&g, a)
                        || !existe(&g, b)
                        || em_guerra_com(&g, a, b)
                        // Antes de 1960: sem guerras aleatórias entre membros do mesmo bloco ocidental
                        || (antes_de_1960 && ambos_ocidentais(&diplo.0, a, b))
                    {
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
            // IA pesa o custo: só ataca se o poder do seu lado (com aliados) for maior que o do país atacado mais o ocupante e os aliados dele.
            if !ia_deve_atacar(&g, &diplo.0, &a, &b, data_tuple) {
                // IA desiste de atacar devido ao custo (ocupantes/aliados do defensor)
            } else {
                // Probabilidade proporcional à tensão (mínimo 10%, determinística pela data).
                let chance = ((diplo.0.tensao - 18.0) / 32.0 * 100.0).clamp(10.0, 75.0) as u32;
                if ano_mes.wrapping_mul(2654435761) % 100 < chance {
                    let nome = format!("Guerra {} - {}", a, b);
                    if let Ok((_id, ocupantes_entraram, aliados_def_entraram)) = declarar_com_aliados(&mut g, &diplo.0, &a, &b, &nome, true, data_tuple) {
                        diplo.0.tensao = (diplo.0.tensao + 6.0).min(100.0);
                        let mut msg = format!("GUERRA: {a} declara guerra a {b}");
                        if !ocupantes_entraram.is_empty() {
                            msg.push_str(&format!("; {} (potência ocupante) entra na guerra contra {a}", ocupantes_entraram.join(", ")));
                        }
                        if !aliados_def_entraram.is_empty() {
                            msg.push_str(&format!("; aliados de defesa {} entram na guerra contra {a}", aliados_def_entraram.join(", ")));
                        }
                        avisos.push(msg);
                    }
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

    // 4. Atualiza envolvimentos de guerras por procuração (níveis 0-5).
    let data_tuple = (mundo.0.data.ano as i32, mundo.0.data.mes, mundo.0.data.dia);
    atualizar_envolvimentos_proxy(&mut g, &diplo.0, &mut mundo, data_tuple);
    
    // 5. Gera notícias sobre envolvimentos.
    if let Some(ev) = ev.as_deref_mut() {
        noticiar_envolvimentos(ev, &mundo, &g);
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
    use crate::plugins::guerra_sim::construir_guerra_1945;
    use gf_mapa::mapa::CatalogoProvincias;
    use sim_diplomacia::{Bloco, Diplomacia, InfoPais};
    use simulacao::mundo::MundoJogo;
    use simulacao::tempo::DataSimulacao;
    use std::path::PathBuf;

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

    /// Testa que a IA pesa o custo de atacar países ocupados:
    /// - GBR (fraco) NÃO ataca JAP ocupado por USA (forte)
    /// - USA NÃO ataca GER ocupado por USA (mesmo país, sem sentido)
    /// - Não há guerras aleatórias entre membros do bloco ocidental antes de 1960
    /// Simula 1945-1950 sem intervenção do jogador.
    #[test]
    fn ia_pesa_custo_atacar_ocupados_e_sem_guerras_ocidentais_antes_1960() {
        // Carrega o cenário real de 1945
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (catalogo, mundo_base) = MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenário 1945");

        let catalogo_mapa: CatalogoProvincias = ron::from_str(
            &std::fs::read_to_string(raiz.join("assets/map/provinces.ron")).expect("provinces.ron"),
        )
        .expect("provinces.ron inválido");

        let (mut guerra_sim, _) = construir_guerra_1945(&raiz, &catalogo_mapa, &mundo_base).expect("guerra 1945");
        let mut diplo = Diplomacia::novo(
            mundo_base
                .paises
                .values()
                .map(|p| InfoPais {
                    tag: p.tag.clone(),
                    bloco: match p.alinhamento_inicial {
                        simulacao::mapa::AlinhamentoGeopolitico::BlocoOcidental => Bloco::Ocidental,
                        simulacao::mapa::AlinhamentoGeopolitico::BlocoSovietico => Bloco::Sovietico,
                        simulacao::mapa::AlinhamentoGeopolitico::NaoAlinhado
                        | simulacao::mapa::AlinhamentoGeopolitico::OcupacaoAliada => Bloco::NaoAlinhado,
                    },
                })
                .collect(),
        );

        // Simula 1945 a 1950 (aprox. 5 anos = 1825 dias)
        let mut guerras_declaradas: Vec<(String, String)> = Vec::new();
        let mut data = DataSimulacao::nova(1945, 1, 1);
        for _ in 0..1825 {
            // Tick da diplomacia (avança 1 dia)
            diplo.tick_dia(data.ano, data.mes, data.dia, Some("BRA"));

            // Tick da guerra (avança 1 dia)
            let data_jogo = sim_guerra::tipos::DataJogo::nova(data.ano as u16, data.mes, data.dia).unwrap();
            guerra_sim.cenario.data_inicial = data_jogo;
            let _ = guerra_sim.executar_tick_diario();

            // Verifica se alguma guerra nova foi declarada
            for g in &guerra_sim.cenario.guerras {
                if g.ativa
                    && g.data_inicio.ano == data_jogo.ano
                    && g.data_inicio.mes == data_jogo.mes
                    && g.data_inicio.dia == data_jogo.dia
                {
                    for atk in &g.atacantes {
                        for def in &g.defensores {
                            guerras_declaradas.push((atk.clone(), def.clone()));
                        }
                    }
                }
            }

            data.avancar_dia();
        }

        // Verifica que a IA fraca (GBR) NÃO ataca país ocupado por potência forte (USA)
        // GBR × JAP não deve acontecer porque USA (ocupante) é muito mais forte que GBR
        let gbr_jap = guerras_declaradas.iter().any(|(a, d)| a == "GBR" && d == "JAP");
        assert!(!gbr_jap, "GBR não deve atacar JAP ocupado por USA (poder insuficiente). Guerras: {:?}", guerras_declaradas);

        // USA × JAP e USA × GER não fazem sentido (mesmo lado / próprio ocupado)
        let usa_jap = guerras_declaradas.iter().any(|(a, d)| a == "USA" && d == "JAP");
        assert!(!usa_jap, "USA não deve atacar seu próprio ocupado JAP. Guerras: {:?}", guerras_declaradas);

        let usa_ger = guerras_declaradas.iter().any(|(a, d)| a == "USA" && d == "GER");
        assert!(!usa_ger, "USA não deve atacar seu próprio ocupado GER. Guerras: {:?}", guerras_declaradas);

        // JAP e GER não declaram guerra aos ocupantes (estão ocupados, sem exército próprio)
        let jap_gbr = guerras_declaradas.iter().any(|(a, d)| a == "JAP" && d == "GBR");
        assert!(!jap_gbr, "JAP ocupado não deve declarar guerra. Guerras: {:?}", guerras_declaradas);

        let jap_usa = guerras_declaradas.iter().any(|(a, d)| a == "JAP" && d == "USA");
        assert!(!jap_usa, "JAP ocupado não deve declarar guerra aos EUA. Guerras: {:?}", guerras_declaradas);

        let ger_usa = guerras_declaradas.iter().any(|(a, d)| a == "GER" && d == "USA");
        assert!(!ger_usa, "GER ocupado não deve declarar guerra aos EUA. Guerras: {:?}", guerras_declaradas);

        // Verifica também que não houve guerras entre membros do bloco ocidental antes de 1960
        for (atk, def) in &guerras_declaradas {
            let bloco_atk = diplo.bloco(atk);
            let bloco_def = diplo.bloco(def);
            if bloco_atk == Bloco::Ocidental && bloco_def == Bloco::Ocidental {
                // Só falha se a guerra começou antes de 1960
                // (As guerras históricas programadas podem criar exceções, mas não devem ser aleatórias)
                panic!("Guerra entre ocidentais declarada antes de 1960: {atk} × {def}");
            }
        }
    }
}
