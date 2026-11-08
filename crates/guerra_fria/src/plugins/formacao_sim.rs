//! Descolonização e países formáveis no cliente: na data histórica surgem países novos no mapa
//! (Paquistão 1947, Israel 1948, RFA/RDA 1949, RPC 1949, independências africanas ~1960...).
//!
//! A lógica pura está em `simulacao::formacao`. Aqui: repintar o mapa, registrar o país na diplomacia,
//! escrever no Jornal, refazer os rótulos e avisar o jogador (pop-up que pausa o jogo).

use super::diplomacia_ui::{DiploSim, PainelDiplomacia};
use super::economy::RecursoMundo;
use super::enciclopedia;
use super::eventos_ui::EstadoEv;
use super::guerra_sim::{normalizar_tag, GuerraSim};
use super::jogador::{Notificacoes, PaisJogador};
use super::map::{CoresMapa, RaizProjeto, RecursoCatalogo};
use super::rotulos_pais::{RotuloPais, RotulosPaisCriados};
use super::time::{EventoTickDiario, RecursoRelogio};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use eventos::Noticia;
use sim_diplomacia::{Bloco, InfoPais};
use simulacao::artigo;
use simulacao::formacao::{EventoFormacao, Formacoes};
use simulacao::mapa::AlinhamentoGeopolitico;

#[derive(Resource)]
pub struct FormacaoSim {
    pub nucleo: Formacoes,
    /// Avisos aguardando o jogador (o primeiro é exibido).
    pub avisos: Vec<EventoFormacao>,
    pub economia_pendentes: Vec<EventoFormacao>,
    pub politica_pendentes: Vec<EventoFormacao>,
    retomar: bool,
}

pub struct FormacaoSimPlugin;

impl Plugin for FormacaoSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, carregar).add_systems(
            Update,
            tick_formacao.run_if(resource_exists::<FormacaoSim>).run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida),
        );
        app.add_systems(
            EguiPrimaryContextPass,
            aviso_formacao.run_if(resource_exists::<FormacaoSim>).run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida),
        );
    }
}

fn carregar(mut commands: Commands, raiz: Option<Res<RaizProjeto>>, catalogo: Option<Res<RecursoCatalogo>>) {
    let (Some(raiz), Some(catalogo)) = (raiz, catalogo) else { return };
    let dir = raiz.0.join("data/formaveis");
    let ler = |n: &str| std::fs::read_to_string(dir.join(n)).map_err(|e| e.to_string());
    let res = (|| -> Result<Formacoes, String> {
        let f: Vec<simulacao::formacao::Formavel> = ron::from_str(&ler("formaveis.ron")?).map_err(|e| e.to_string())?;
        let i: Vec<simulacao::formacao::Independencia> = ron::from_str(&ler("independencias_existentes.ron")?).map_err(|e| e.to_string())?;
        let provs = catalogo.0.provincias.iter().map(|p| (p.id, p.adm1_code.as_str(), p.centroide));
        Ok(Formacoes::com_indice(f, i, provs))
    })();
    match res {
        Ok(nucleo) => {
            info!("formacao: {} formáveis carregados", nucleo.formaveis().len());
            commands.insert_resource(FormacaoSim {
                nucleo,
                avisos: Vec::new(),
                economia_pendentes: Vec::new(),
                politica_pendentes: Vec::new(),
                retomar: false,
            });
        }
        Err(e) => error!("formacao: falha ao carregar: {e}"),
    }
}

fn texto_noticia(e: &EventoFormacao) -> String {
    if e.tipo == "desocupacao" {
        e.descricao.clone()
    } else if e.independencia {
        format!("{} conquista a independência.", e.nome)
    } else if e.origens.is_empty() {
        format!("Nasce um novo país: {}.", e.nome)
    } else {
        format!("Nasce um novo país: {} (a partir de {}).", e.nome, e.origens.join(", "))
    }
}

#[allow(clippy::too_many_arguments)]
fn tick_formacao(
    mut leitor: MessageReader<EventoTickDiario>,
    mut form: ResMut<FormacaoSim>,
    mut mundo: ResMut<RecursoMundo>,
    cores: Option<ResMut<CoresMapa>>,
    painel: Option<ResMut<PainelDiplomacia>>,
    diplo: Option<ResMut<DiploSim>>,
    mut guerra: Option<ResMut<GuerraSim>>,
    mut ev: ResMut<EstadoEv>,
    jogador: Res<PaisJogador>,
    rotulos: Query<Entity, With<RotuloPais>>,
    mut commands: Commands,
    mut notificacoes: ResMut<Notificacoes>,
    tempo: Res<Time<Real>>,
) {
    if leitor.read().count() == 0 {
        return;
    }
    let eventos = form.nucleo.processar(&mut mundo.0);
    if eventos.is_empty() {
        return;
    }
    let mut cores = cores;
    let mut diplo = diplo;
    for e in &eventos {
        let d = e.data;
        ev.estado.jornal.push(Noticia { data: (d.ano, d.mes, d.dia), texto: texto_noticia(e) });
        if let Some(pais) = mundo.0.paises.get(&e.tag) {
            if let Some(dp) = diplo.as_deref_mut() {
                dp.0.adicionar_pais(InfoPais {
                    tag: e.tag.clone(),
                    bloco: match pais.alinhamento_inicial {
                        AlinhamentoGeopolitico::BlocoOcidental => Bloco::Ocidental,
                        AlinhamentoGeopolitico::BlocoSovietico => Bloco::Sovietico,
                        _ => Bloco::NaoAlinhado,
                    },
                });
            }
            if let Some(c) = cores.as_deref_mut() {
                repintar_provincias(c, &e.provincias, &e.tag, pais.cor);
            }
            if let Some(g) = guerra.as_deref_mut() {
                let tag_norm = normalizar_tag(&e.tag);
                for &p_id in &e.provincias {
                    let adm_id = format!("prov_{p_id}");
                    if let Some(r) = g.0.cenario.regioes.iter_mut().find(|r| r.adm1_code == adm_id) {
                        r.dono_tag = tag_norm.clone();
                        r.controlador_tag = tag_norm.clone();
                    }
                }
                if !g.0.mapa_paises.contains_key(&tag_norm) {
                    let capital = e.provincias.first().map(|p| format!("prov_{p}")).unwrap_or_else(|| "prov_1".to_string());
                    let pop_mob = (pais.populacao_total() as f64 * 0.025) as u64;
                    let idx = g.0.cenario.paises.len();
                    g.0.cenario.paises.push(sim_guerra::PaisMilitar {
                        tag: tag_norm.clone(),
                        nome_pt: pais.nome.clone(),
                        capital_adm1: capital.clone(),
                        doutrina_id: "doutrina_padrao".to_string(),
                        tecnologias_ativas: Vec::new(),
                        cansaco_guerra: 0.0,
                        baixas_totais: 0,
                        populacao_mobilizavel: pop_mob.max(50_000),
                        estoque_suprimento: 1.0,
                        eficiencia_naval: 1.0,
                    });
                    g.0.mapa_paises.insert(tag_norm.clone(), idx);
                    let div_id = format!("div_{}_{}", tag_norm.to_lowercase(), 1);
                    g.0.cenario.divisoes.push(sim_guerra::Divisao {
                        id: div_id.clone(),
                        nome_pt: format!("1ª Divisão de {}", pais.nome),
                        pais_tag: tag_norm.clone(),
                        template_id: "divisao_infantaria_1945".to_string(),
                        regiao_adm1: capital,
                        efetivos_atuais: 15_000,
                        equipamento_fracao: 0.8,
                        organizacao: 45.0,
                        moral: 0.9,
                        entrincheiramento: 1.0,
                        suprimento_atual: 0.9,
                        experiencia: 0.3,
                    });
                    g.0.cenario.exercitos.push(sim_guerra::Exercito {
                        id: format!("exe_{}_{}", tag_norm.to_lowercase(), 1),
                        nome_pt: format!("Exército de {}", pais.nome),
                        pais_tag: tag_norm.clone(),
                        comandante_id: None,
                        divisoes_ids: vec![div_id],
                        postura: sim_guerra::PosturaOperacional::OfensivaEquilibrada,
                        frente_id: None,
                        regiao_objetivo_adm1: None,
                    });
                }
                g.0.invalidar_cache_territorial();
            }
        }
        // Aviso ao jogador: só para países que ele "vê" (todos), com pausa.
        form.avisos.push(e.clone());
        form.economia_pendentes.push(e.clone());
        form.politica_pendentes.push(e.clone());
        let texto = if e.tipo == "desocupacao" {
            super::enciclopedia::texto_notificacao(
                "msg_desocupacao_historica",
                &[("territorio", e.nome.clone()), ("descricao", texto_noticia(e))],
                &texto_noticia(e),
            )
        } else if e.independencia {
            // A metrópole entra pelo nome com o artigo concordado ("do Reino Unido", "da França").
            let metropole = e
                .origens
                .first()
                .and_then(|t| mundo.0.paises.get(t))
                .map(|p| format!("{} {}", artigo::contraido_de(&p.nome), p.nome))
                .unwrap_or_else(|| "da antiga metrópole".into());
            super::enciclopedia::texto_notificacao(
                "msg_independencia",
                &[("pais", e.nome.clone()), ("metropole", metropole)],
                &texto_noticia(e),
            )
        } else {
            super::enciclopedia::texto_notificacao(
                "msg_evento_historico",
                &[("titulo", e.nome.clone()), ("descricao", texto_noticia(e))],
                &texto_noticia(e),
            )
        };
        notificacoes.push(Notificacoes::mensagem_modelada(&texto, &e.nome), tempo.elapsed_secs_f64());
    }
    if let Some(mut p) = painel {
        p.repintar = true;
    }
    // Rótulos: refaz a partir do zero com os novos donos.
    for ent in &rotulos {
        commands.entity(ent).despawn();
    }
    commands.remove_resource::<RotulosPaisCriados>();
    let _ = jogador;
}

fn repintar_provincias(c: &mut CoresMapa, provs: &[u32], tag: &str, cor: (u8, u8, u8)) {
    // Índice de dono: reaproveita o do país se já tinha província; senão cria um novo.
    let max = c.dono_prov.iter().copied().max().unwrap_or(0);
    let idx = max + 1;
    for &p in provs {
        if let Some(s) = c.cor_prov.get_mut(p as usize) {
            *s = [cor.0, cor.1, cor.2, 255];
        }
        if let Some(s) = c.dono_prov.get_mut(p as usize) {
            *s = idx;
        }
    }
    let _ = tag;
}

fn aviso_formacao(
    mut contextos: EguiContexts,
    mut form: ResMut<FormacaoSim>,
    mut relogio: ResMut<RecursoRelogio>,
    ev: Res<EstadoEv>,
    op: Option<Res<super::smoke::OpcoesCli>>,
) -> Result {
    if op.as_ref().is_some_and(|o| o.espectador_ativo()) {
        form.avisos.clear();
        return Ok(());
    }
    // Eventos urgentes têm prioridade absoluta; este aviso permanece na fila.
    if !ev.estado.pendentes.is_empty() {
        return Ok(());
    }
    let Some(e) = form.avisos.first().cloned() else { return Ok(()) };
    if !relogio.pausado {
        form.retomar = true;
        relogio.pausado = true;
    }
    let mut ok = false;
    ui_refinada::janela_estilo_menu("novo_pais")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([520.0, 0.0])
        .show(contextos.ctx_mut()?, |ui| {
            ui_refinada::decorar_janela(ui);
            ui.label(egui::RichText::new(format!("{:02}/{:02}/{}", e.data.dia, e.data.mes, e.data.ano)).color(ui_refinada::menu::TINTA_SUAVE));
            ui_refinada::cabecalho_painel(ui, &texto_noticia(&e));
            if !e.descricao.is_empty() {
                ui.label(egui::RichText::new(&e.descricao).color(ui_refinada::menu::TINTA));
            }
            if !e.independencia {
                ui.label(egui::RichText::new(format!("{} províncias, população {}.", e.provincias.len(), super::jogador::fmt_pop(e.populacao))).color(ui_refinada::menu::TINTA));
            }
            let faltam = form.avisos.len() - 1;
            if faltam > 0 {
                ui.label(egui::RichText::new(format!("(+{faltam} na fila)")).color(ui_refinada::menu::TINTA_SUAVE));
            }
            if ui_refinada::botao_estilo_menu(ui, "Entendido").clicked() {
                ok = true;
            }
        });
    if ok {
        form.avisos.remove(0);
        if form.avisos.is_empty() && form.retomar {
            relogio.pausado = false;
            form.retomar = false;
        }
    }
    Ok(())
}
