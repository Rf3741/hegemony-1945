//! Plugin Bevy de divergência geopolítica e probabilidade histórica na Guerra Fria.
//!
//! Integra o `MotorDivergencia` (`crates/simulacao`) com o ECS:
//! - Inicializa o motor com a semente da partida (`--semente` ou padrão).
//! - No tick diário, avalia o estado mundial (economia, legitimidade, tensão, guerras).
//! - Dispara revoluções socialistas e golpes pró-ocidente conforme influência e estabilidade.
//! - Aplica condições de vitória da Guerra Fria (vitória ocidental / soviética / impasse).
//! - Sincroniza com diplomacia (blocos, pactos, OTAN, Pacto de Varsóvia), economia e jornal.

use super::diplomacia_ui::DiploSim;
use super::economia_sim::{Comando, EconomiaSim};
use super::economy::RecursoMundo;
use super::eventos_ui::EstadoEv;
use super::formacao_sim::FormacaoSim;
use super::smoke::OpcoesCli;
use super::time::EventoTickDiario;
use bevy::prelude::*;
use eventos::Noticia;
use sim_diplomacia::Bloco;
use simulacao::{DesfechoGuerraFria, EventoDivergencia, MotorDivergencia};

/// Recurso ECS que mantém o estado do motor de divergência probabilística.
#[derive(Resource)]
pub struct DivergenciaSim(pub MotorDivergencia);

pub struct DivergenciaPlugin;

impl Plugin for DivergenciaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, inicializar_divergencia)
            .add_systems(
                Update,
                tick_divergencia
                    .run_if(resource_exists::<DivergenciaSim>)
                    .run_if(resource_exists::<RecursoMundo>)
                    .run_if(crate::estado::em_partida),
            );
    }
}

fn inicializar_divergencia(mut commands: Commands, op: Option<Res<OpcoesCli>>) {
    let semente = op.as_ref().map_or(19450815, |o| o.semente_efetiva());
    info!("DivergenciaSim inicializado com semente {semente}");
    commands.insert_resource(DivergenciaSim(MotorDivergencia::novo(semente)));
}

fn tick_divergencia(
    mut leitor: MessageReader<EventoTickDiario>,
    mut div: ResMut<DivergenciaSim>,
    mut mundo: ResMut<RecursoMundo>,
    mut diplo: Option<ResMut<DiploSim>>,
    mut form: Option<ResMut<FormacaoSim>>,
    mut ev: Option<ResMut<EstadoEv>>,
    mut eco: Option<ResMut<EconomiaSim>>,
) {
    let mut houve_tick = false;
    for _ in leitor.read() {
        houve_tick = true;
    }
    if !houve_tick {
        return;
    }

    let data = mundo.0.data;

    // Coleta influências geopolíticas dos países
    let influencias: Vec<(String, (f32, f32))> = if let Some(d) = diplo.as_ref() {
        d.0.paises
            .iter()
            .map(|p| (p.tag.clone(), d.0.influencia(&p.tag)))
            .collect()
    } else {
        Vec::new()
    };

    let tensao = diplo.as_ref().map_or(20.0, |d| d.0.tensao);

    // Avalia o passo anual / mensal de divergência
    let eventos = div.0.avaliar_passo(&mut mundo.0, data, tensao, &influencias);

    // Sincroniza autorizações de formação com FormacaoSim
    if let Some(f) = form.as_deref_mut() {
        f.nucleo.dissolucao_urss_autorizada = div.0.dissolucao_urss_autorizada;
        f.nucleo.reunificacao_alema_autorizada = div.0.reunificacao_alema_autorizada;
    }

    // Processa os eventos de divergência gerados
    for e in eventos {
        match e {
            EventoDivergencia::RevolucaoSocialista { tag, nome, ano: _ } => {
                if let Some(d) = diplo.as_deref_mut() {
                    d.0.definir_bloco(&tag, Bloco::Sovietico);
                    d.0.romper_pactos_com_bloco(&tag, Bloco::Ocidental);
                    d.0.dar_influencia(&tag, "SOV", 50.0);
                }
                if let Some(e_sim) = eco.as_deref_mut() {
                    e_sim.enviar(Comando::CancelarAcordoBilateral { tag_a: tag.clone(), tag_b: "USA".into() });
                    e_sim.enviar(Comando::AcordoComercialBilateral { tag_a: tag.clone(), tag_b: "SOV".into() });
                }
                info!("Divergência: Revolução Socialista em {nome} ({tag})");
            }
            EventoDivergencia::GolpeProOcidental { tag, nome, ano: _ } => {
                if let Some(d) = diplo.as_deref_mut() {
                    d.0.definir_bloco(&tag, Bloco::Ocidental);
                    d.0.romper_pactos_com_bloco(&tag, Bloco::Sovietico);
                    d.0.dar_influencia(&tag, "USA", 50.0);
                }
                if let Some(e_sim) = eco.as_deref_mut() {
                    e_sim.enviar(Comando::CancelarAcordoBilateral { tag_a: tag.clone(), tag_b: "SOV".into() });
                    e_sim.enviar(Comando::AcordoComercialBilateral { tag_a: tag.clone(), tag_b: "USA".into() });
                }
                info!("Divergência: Golpe Pró-Ocidente em {nome} ({tag})");
            }
            EventoDivergencia::VitoriaGuerraFria { vencedor, ano, descricao } => {
                info!("Divergência: Fim da Guerra Fria em {ano} - {}: {descricao}", vencedor.rotulo());
                if let Some(d) = diplo.as_deref_mut() {
                    match vencedor {
                        DesfechoGuerraFria::VitoriaOcidental => {
                            d.0.dissolver_organizacao("pacto_varsovia");
                            d.0.tensao = (d.0.tensao - 30.0).max(5.0);
                        }
                        DesfechoGuerraFria::VitoriaSovietica => {
                            d.0.dissolver_organizacao("otan");
                            d.0.tensao = (d.0.tensao - 30.0).max(5.0);
                        }
                        _ => {}
                    }
                }
            }
            EventoDivergencia::ReunificacaoAlema { ano, sob_rfa } => {
                info!("Divergência: Reunificação Alemã em {ano} (sob RFA: {sob_rfa})");
            }
        }
    }

    // Drena notícias pendentes para o Jornal do jogo
    if let Some(ev_mut) = ev.as_deref_mut() {
        for (a, m, d, texto) in div.0.noticias_pendentes.drain(..) {
            ev_mut.estado.jornal.push(Noticia {
                data: (a, m, d),
                texto,
            });
        }
    }
}
