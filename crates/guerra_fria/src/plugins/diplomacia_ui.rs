//! Tela de Diplomacia (4) e modo de mapa "Esferas": liga o crate puro `sim_diplomacia` ao cliente.

use super::economia_sim::{Comando, EconomiaSim};
use super::economy::RecursoMundo;
use super::guerra_ia::{aliados_com_obrigacao_defesa, todos_os_ocupantes};
use super::guerra_sim::{declarar_com_aliados, normalizar_tag};
use super::jogador::PaisJogador;
use super::map::{CoresMapa, DadosMapa, ImagemMapa, pintar_paleta};
use super::time::EventoTickDiario;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use sim_diplomacia::{Acao, Bloco, Diplomacia, Esfera, InfoPais};
use simulacao::mapa::AlinhamentoGeopolitico;

#[derive(Resource)]
pub struct DiploSim(pub Diplomacia);

#[derive(Clone, Debug)]
pub struct ItemDiploCache {
    pub tag: String,
    pub nome: String,
    pub relacao: f32,
    pub esfera: Esfera,
    pub tag_legado: String,
}

#[derive(Resource, Default)]
pub struct PainelDiplomacia {
    pub aberto: bool,
    pub alvo: String,
    pub busca: String,
    pub mensagem: String,
    pub cb_idx: usize,
    /// Modo de mapa "Esferas" ligado.
    pub mapa_esferas: bool,
    /// Pede repintura do mapa.
    pub repintar: bool,
    pub dias_desde_pintura: u32,
    pub cache_lista: Vec<ItemDiploCache>,
    pub cache_busca: String,
    _versao_diplo: u64,
}

pub struct DiplomaciaUiPlugin;

impl Plugin for DiplomaciaUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PainelDiplomacia>()
            .add_systems(
                Update,
                (criar_diplomacia, alternar, tick_diplomacia, repintar_mapa)
                    .chain()
                    .run_if(resource_exists::<PaisJogador>)
                    .run_if(crate::estado::em_partida),
            )
            .add_systems(
                EguiPrimaryContextPass,
                tela_diplomacia
                    .run_if(resource_exists::<PaisJogador>)
                    .run_if(resource_exists::<DiploSim>)
                    .run_if(crate::estado::em_partida),
            );
    }
}

fn criar_diplomacia(mut commands: Commands, mundo: Res<RecursoMundo>, existente: Option<Res<DiploSim>>) {
    if existente.is_some() {
        return;
    }
    let mut paises: Vec<InfoPais> = mundo
        .0
        .paises
        .values()
        .map(|p| InfoPais {
            tag: normalizar(&p.tag),
            bloco: match p.alinhamento_inicial {
                AlinhamentoGeopolitico::BlocoOcidental => Bloco::Ocidental,
                AlinhamentoGeopolitico::BlocoSovietico => Bloco::Sovietico,
                _ => Bloco::NaoAlinhado,
            },
        })
        .collect();
    paises.sort_by(|a, b| a.tag.cmp(&b.tag));
    commands.insert_resource(DiploSim(Diplomacia::novo(paises)));
}

/// Tags do cenário legado -> tags dos dados diplomáticos.
fn normalizar(t: &str) -> String {
    match t {
        "BGR" => "BUL".into(),
        "GRC" => "GRE".into(),
        _ => t.to_string(),
    }
}

/// Tag diplomática -> tag do cenário legado (inverso de `normalizar`).
fn tag_diplo_para_legado(t: &str) -> String {
    match t {
        "BUL" => "BGR".into(),
        "GRE" => "GRC".into(),
        _ => t.to_string(),
    }
}

fn alternar(teclas: Res<ButtonInput<KeyCode>>, mut p: ResMut<PainelDiplomacia>) {
    if teclas.just_pressed(KeyCode::Digit4) || teclas.just_pressed(KeyCode::Numpad4) {
        p.aberto = !p.aberto;
    }
}

fn tick_diplomacia(
    mut eventos: MessageReader<EventoTickDiario>,
    mut d: Option<ResMut<DiploSim>>,
    mut eco: Option<ResMut<EconomiaSim>>,
    mut ev: Option<ResMut<super::eventos_ui::EstadoEv>>,
    jogador: Res<PaisJogador>,
    opcoes: Option<Res<super::smoke::OpcoesCli>>,
    mut painel: ResMut<PainelDiplomacia>,
) {
    let Some(d) = d.as_deref_mut() else { return };
    let jog = normalizar(&jogador.0);
    let eh_espectador = opcoes.as_ref().is_some_and(|o| o.espectador_ativo());
    let jog_arg = if eh_espectador { None } else { Some(jog.as_str()) };
    for e in eventos.read() {
        let hoje = (e.nova_data.ano, e.nova_data.mes, e.nova_data.dia);
        d.0.tick_dia(hoje.0, hoje.1, hoje.2, jog_arg);
        if eh_espectador {
            if hoje.2 == 1 {
                let t = d.0.tesouro_ia.entry(jog.clone()).or_insert(500.0);
                *t = (*t + 60.0).min(1500.0);
            }
            if hoje.2 == 15 {
                ia_diplomatica_jogador(&mut d.0, &jog, hoje, eco.as_deref_mut(), ev.as_deref_mut());
            }
        }
        painel.dias_desde_pintura += 1;
    }
    if d.0.log.len() > 300 {
        d.0.log.drain(..150);
    }
    if painel.mapa_esferas && painel.dias_desde_pintura >= 30 {
        painel.repintar = true;
    }
}

pub(crate) fn ia_diplomatica_jogador(
    d: &mut Diplomacia,
    eu: &str,
    hoje: (i32, u8, u8),
    eco: Option<&mut EconomiaSim>,
    ev: Option<&mut super::eventos_ui::EstadoEv>,
) {
    if d.pontos.get(eu).copied().unwrap_or(0.0) < 15.0 {
        return;
    }
    let tes = d.tesouro_ia.get(eu).copied().unwrap_or(500.0);
    let minha_esfera = d.esfera(eu);
    let parceiros_prioritarios = ["USA", "ARG", "GBR", "FRA", "MEX", "CHL", "COL", "POR", "ITA", "FRG", "GER", "CAN", "VEN", "URY", "JAP", "SOV", "IND", "CHN"];
    let mut candidatos: Vec<String> = d
        .paises
        .iter()
        .map(|p| p.tag.clone())
        .filter(|t| t != eu)
        .collect();
    if candidatos.is_empty() {
        return;
    }
    let rotacao = ((hoje.0 as usize).wrapping_mul(12) + hoje.1 as usize) % candidatos.len();
    candidatos.rotate_left(rotacao);
    candidatos.sort_by_key(|alvo| {
        let prio = if parceiros_prioritarios.contains(&alvo.as_str()) { 0 } else { 1 };
        let mesma_esfera = if d.esfera(alvo) == minha_esfera { 0 } else { 1 };
        (prio, mesma_esfera)
    });
    // Rotaciona entre os melhores parceiros a cada mês para diversificar tratados ao longo das décadas.
    let topo = candidatos.len().min(12);
    if topo > 1 {
        let r = ((hoje.0 as usize).wrapping_mul(12) + hoje.1 as usize) % topo;
        candidatos[..topo].rotate_left(r);
    }

    let seq = [
        Acao::AcordoComercial,
        Acao::PactoDefesa,
        Acao::Alianca,
        Acao::MelhorarRelacoes,
        Acao::AjudaEconomica,
    ];
    for alvo in &candidatos {
        for acao in seq {
            if d.pode(eu, alvo, acao, tes).is_err() {
                continue;
            }
            if d.aceitacao(eu, alvo, acao).is_some_and(|a| a < 0.0) {
                continue;
            }
            if let Ok(r) = d.executar(eu, alvo, acao, tes) {
                if r.custo_tesouro > 0.0 {
                    *d.tesouro_ia.entry(eu.into()).or_insert(500.0) -= r.custo_tesouro;
                }
                d.log.push(format!(
                    "Diplomacia {eu} -> {alvo}: {} ({})",
                    acao.nome(),
                    if r.aceita { "aceita" } else { "recusada" }
                ));
                if r.acordo_comercial
                    && let Some(eco) = eco
                    && let Some(id) = eco.id_pais(&tag_diplo_para_legado(alvo))
                {
                    eco.enviar(Comando::AssinarAcordo(id));
                }
                if r.aceita
                    && matches!(acao, Acao::AcordoComercial | Acao::PactoDefesa | Acao::Alianca)
                    && let Some(ev) = ev
                {
                    ev.estado.jornal.push(eventos::Noticia {
                        data: hoje,
                        texto: format!("Diplomacia: {eu} firma {} com {alvo}.", acao.nome()),
                    });
                    if ev.estado.jornal.len() > 200 {
                        ev.estado.jornal.remove(0);
                    }
                }
                return;
            }
        }
    }
}

/// Repinta a textura do mapa com a esfera de cada país (ou restaura as cores políticas).
fn repintar_mapa(
    mut painel: ResMut<PainelDiplomacia>,
    d: Option<Res<DiploSim>>,
    cores: Option<Res<CoresMapa>>,
    dados: Option<Res<DadosMapa>>,
    imagem: Option<Res<ImagemMapa>>,
    mundo: Res<RecursoMundo>,
    mut imagens: ResMut<Assets<Image>>,
) {
    if !painel.repintar {
        return;
    }
    painel.repintar = false;
    painel.dias_desde_pintura = 0;
    let (Some(d), Some(cores), Some(_), Some(imagem)) = (d, cores, dados, imagem) else { return };
    let mut cor_prov = cores.cor_prov.clone();
    if painel.mapa_esferas {
        for (id, tag) in &mundo.0.dono_por_provincia {
            if let Some(slot) = cor_prov.get_mut(*id as usize) {
                let [r, g, b] = Diplomacia::cor_esfera(d.0.esfera(&normalizar(tag)));
                *slot = [r, g, b, 255];
            }
        }
    }
    pintar_paleta(&mut imagens, &imagem, &cor_prov, &cores.dono_prov, &cores.estado_prov, None);
}

fn cor_egui(e: Esfera) -> egui::Color32 {
    let [r, g, b] = Diplomacia::cor_esfera(e);
    egui::Color32::from_rgb(r, g, b)
}

fn nome_esfera(e: Esfera) -> &'static str {
    match e {
        Esfera::Eua => "EUA",
        Esfera::Urss => "URSS",
        Esfera::NaoAlinhado => "Não alinhado",
        Esfera::Disputada => "Disputada",
    }
}

fn tela_diplomacia(
    mut contextos: EguiContexts,
    mut painel: ResMut<PainelDiplomacia>,
    mut d: ResMut<DiploSim>,
    mut mundo: ResMut<RecursoMundo>,
    mut eco: Option<ResMut<EconomiaSim>>,
    jogador: Res<PaisJogador>,
    elenco: Option<Res<super::personagens_sim::ElencoRes>>,
    mut retratos: Option<ResMut<super::personagens_ui::Retratos>>,
    mut guerra_sim: Option<ResMut<super::guerra_sim::GuerraSim>>,
    dados_guerra: Option<Res<super::guerra_sim::DadosGuerra>>,
    mut painel_guerra: Option<ResMut<super::guerra_sim::PainelGuerra>>,
) -> Result {
    if !painel.aberto {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?;
    let eu = normalizar(&jogador.0);
    let tag_legado = jogador.0.clone();
    let mut aberto = true;
    let mut mudou_mapa = false;
    let tesouro = mundo.0.paises.get(&tag_legado).map_or(0.0, |p| p.orcamento.tesouro);
    let mut gasto = 0.0;
    let mut cmd_acordo: Option<String> = None;
    let nomes: std::collections::HashMap<String, String> = mundo.0.paises.values().map(|p| (normalizar(&p.tag), p.nome.clone())).collect();
    let nome = |t: &str| nomes.get(t).cloned().unwrap_or_else(|| t.to_string());

    let resp_janela = ui_refinada::janela_dossie("DIPLOMACIA E RELAÇÕES EXTERIORES (4)", None)
        .default_pos([20.0, 96.0])
        .default_size([980.0, 660.0])
        .show(ctx, |ui| {
            ui_refinada::faixa_titulo(ui, "Diplomacia e Relações Exteriores", Some("4"), Some(&mut aberto));
            ui.horizontal_wrapped(|ui| {
                let t = d.0.tensao;
                let cor_t = if t > 60.0 {
                    ui_refinada::menu::PERIGO
                } else if t > 30.0 {
                    ui_refinada::menu::AVISO
                } else {
                    ui_refinada::menu::SUCESSO
                };
                ui_refinada::barra_status(
                    ui,
                    Some("Tensão mundial"),
                    t as f32,
                    100.0,
                    cor_t,
                    150.0,
                    Some(&format!("{t:.0}/100")),
                );
                ui_refinada::chapa(
                    ui,
                    "moeda",
                    "Pontos de influência",
                    &format!("{:.0}", d.0.pontos.get(&eu).copied().unwrap_or(0.0)),
                    Some(ui_refinada::menu::OURO),
                );
                let e = d.0.esfera(&eu);
                ui_refinada::chapa(ui, "estrela", "Sua esfera", nome_esfera(e), Some(cor_egui(e)));
                if ui.checkbox(&mut painel.mapa_esferas, "Mapa: Esferas").changed() {
                    mudou_mapa = true;
                }
            });
        ui.separator();
        ui.columns(2, |cols| {
            // ---- Coluna esquerda: lista de países ----
            ui_refinada::titulo_secao(&mut cols[0], "Nações Reconhecidas");
            cols[0].horizontal(|ui| {
                ui.small("Buscar:");
                ui.add(egui::TextEdit::singleline(&mut painel.busca).hint_text("nome ou sigla").desired_width(150.0));
            });
            cols[0].horizontal(|ui| {
                ui.weak("País");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak("Relação");
                });
            });
            cols[0].separator();

            // Cache da lista de países: só recalcula se a busca ou os dados diplomáticos mudarem
            if painel.cache_busca != painel.busca || painel.cache_lista.is_empty() || d.is_changed() {
                painel.cache_busca = painel.busca.clone();
                let busca = painel.busca.to_lowercase();
                let mut nova_lista = Vec::new();
                for p in &d.0.paises {
                    let t = &p.tag;
                    if *t == eu {
                        continue;
                    }
                    let n = nome(t);
                    if !busca.is_empty() && !n.to_lowercase().contains(&busca) && !t.to_lowercase().contains(&busca) {
                        continue;
                    }
                    let r = d.0.relacao(&eu, t);
                    let e = d.0.esfera(t);
                    nova_lista.push(ItemDiploCache {
                        tag: t.clone(),
                        nome: n,
                        relacao: r,
                        esfera: e,
                        tag_legado: tag_diplo_para_legado(t),
                    });
                }
                painel.cache_lista = nova_lista;
            }
            let total_linhas = painel.cache_lista.len();
            let mut novo_alvo = None;
            egui::ScrollArea::vertical().id_salt("diplo_lista").max_height(480.0).show_rows(&mut cols[0], 22.0, total_linhas, |ui, row_range| {
                for idx in row_range {
                    let item = &painel.cache_lista[idx];
                    ui.horizontal(|ui| {
                        if !super::bandeiras_ui::bandeira_ui(ui, &item.tag_legado, 14.0) {
                            ui.colored_label(cor_egui(item.esfera), "■");
                        }
                        let sel = painel.alvo == item.tag;
                        if ui.selectable_label(sel, format!("{} ({})", item.nome, item.tag)).clicked() {
                            novo_alvo = Some(item.tag.clone());
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let r = item.relacao;
                            let c = if r >= 21.0 { ui_refinada::menu::SUCESSO } else if r <= -21.0 { ui_refinada::menu::PERIGO } else { ui_refinada::menu::TINTA_SUAVE };
                            ui.colored_label(c, format!("{r:+.0}"));
                            ui_refinada::barra_relacao(ui, (r / 100.0) as f32, 58.0);
                        });
                    });
                }
            });
            if let Some(alvo) = novo_alvo {
                painel.alvo = alvo;
            }
            // ---- Coluna direita: detalhes e ações ----
            let ui = &mut cols[1];
            if painel.alvo.is_empty() {
                ui.centered_and_justified(|ui| ui.weak("Selecione um país na lista."));
            } else {
                let alvo = painel.alvo.clone();
                let r = d.0.relacao(&eu, &alvo);
                ui_refinada::moldura_cartao().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        super::bandeiras_ui::bandeira_ui(ui, &tag_diplo_para_legado(&alvo), 32.0);
                        ui.label(
                            egui::RichText::new(nome(&alvo))
                                .family(ui_refinada::familia_oswald(ui.ctx()))
                                .size(20.0)
                                .strong()
                                .color(ui_refinada::menu::AZUL_MARINHO),
                        );
                    });
                    if let (Some(el), Some(rt)) = (elenco.as_deref(), retratos.as_deref_mut()) {
                        super::personagens_ui::retrato_chefe(ui, el, rt, &alvo, 72.0);
                    }
                    ui.horizontal(|ui| {
                        ui_refinada::barra_relacao(ui, (r / 100.0) as f32, 130.0);
                        ui.label(format!("{r:+.0} · {}", Diplomacia::nivel_relacao(r)));
                    });
                    let e = d.0.esfera(&alvo);
                    let (iu, is) = d.0.influencia(&alvo);
                    ui.horizontal(|ui| {
                        ui_refinada::chapa(ui, "estrela", "Esfera", nome_esfera(e), Some(cor_egui(e)));
                        ui_refinada::chapa(ui, "navio", "EUA", &format!("{iu:.0}"), None);
                        ui_refinada::chapa(ui, "canhao", "URSS", &format!("{is:.0}"), None);
                    });
                    for p in d.0.pactos_de(&alvo).filter(|p| p.a == eu || p.b == eu) {
                        ui.horizontal(|ui| {
                            ui_refinada::glifo_ui(ui, "balanca", 14.0, Some(ui_refinada::menu::AZUL_MARINHO));
                            ui.label(format!("{} com você", p.tipo.nome()));
                        });
                    }
                });
                ui.add_space(6.0);
                ui_refinada::moldura_cartao().show(ui, |ui| {
                    ui_refinada::titulo_secao(ui, "Ações Diplomáticas");
                    for acao in Acao::TODAS {
                        let custo = d.0.custo_efetivo(&eu, acao);
                        let ct = acao.custo_tesouro();
                        let txt = if ct > 0.0 { format!("{} ({custo:.0} pts, {ct:.0} M)", acao.nome()) } else { format!("{} ({custo:.0} pts)", acao.nome()) };
                        let pode = d.0.pode(&eu, &alvo, acao, tesouro);
                        let mut dica = match &pode {
                            Ok(()) => "Disponível.".to_string(),
                            Err(m) => m.clone(),
                        };
                        if let Some(a) = d.0.aceitacao(&eu, &alvo, acao) {
                            dica += &format!("\nAceitação da IA: {a:.1} (precisa ser >= 0)");
                        }
                        if ui.add_enabled(pode.is_ok(), egui::Button::new(txt)).on_hover_text(dica).clicked() {
                            match d.0.executar(&eu, &alvo, acao, tesouro) {
                                Ok(res) => {
                                    gasto += res.custo_tesouro;
                                    if res.acordo_comercial {
                                        cmd_acordo = Some(alvo.clone());
                                    }
                                    painel.mensagem = res.mensagem;
                                    if painel.mapa_esferas {
                                        mudou_mapa = true;
                                    }
                                }
                                Err(m) => painel.mensagem = m,
                            }
                        }
                    }
                });
                ui.add_space(6.0);
                ui_refinada::moldura_cartao().show(ui, |ui| {
                    ui_refinada::titulo_secao(ui, "Operações Militares");
                    if let Some(ref mut g_sim) = guerra_sim {
                    let guerra_ativa = g_sim.0.guerras().iter().find(|gw| gw.ativa && gw.sao_inimigos(&eu, &alvo)).cloned();
                    if let Some(gw) = guerra_ativa {
                        ui.horizontal(|ui| {
                            ui_refinada::glifo_ui(ui, "canhao", 16.0, Some(ui_refinada::menu::VERMELHO));
                            ui.label(
                                egui::RichText::new("VOCÊ ESTÁ EM GUERRA COM ESTE PAÍS")
                                    .family(ui_refinada::familia_oswald(ui.ctx()))
                                    .size(15.0)
                                    .strong()
                                    .color(ui_refinada::menu::VERMELHO),
                            );
                        });
                        let score = if gw.eh_atacante(&eu) { gw.pontuacao_guerra } else { -gw.pontuacao_guerra };
                        let cor_s = if score > 0.0 { ui_refinada::menu::SUCESSO } else if score < 0.0 { ui_refinada::menu::PERIGO } else { ui_refinada::menu::TINTA_SUAVE };
                        ui_refinada::barra_status(
                            ui,
                            Some("Pontuação de guerra"),
                            (score + 100.0) as f32,
                            200.0,
                            cor_s,
                            150.0,
                            Some(&format!("{score:+.1}%")),
                        );
                        if ui_refinada::botao_estilo_menu(ui, "Abrir Painel de Guerra e Paz (6)").clicked() {
                            if let Some(ref mut pg) = painel_guerra {
                                pg.aberto = true;
                                pg.aba = 0;
                            }
                        }
                    } else {
                        let cbs = if let Some(ref dg) = dados_guerra {
                            dg.0.casus_belli.clone()
                        } else {
                            Vec::new()
                        };

                        if !cbs.is_empty() {
                            let idx = painel.cb_idx.min(cbs.len() - 1);
                            let cb_atual = &cbs[idx];
                            egui::ComboBox::from_label("Casus Belli")
                                .selected_text(&cb_atual.nome)
                                .show_ui(ui, |ui| {
                                    for (i, cb) in cbs.iter().enumerate() {
                                        ui.selectable_value(&mut painel.cb_idx, i, &cb.nome);
                                    }
                                });
                            ui.label(format!("Descrição: {}", cb_atual.descricao));
                            ui.horizontal(|ui| {
                                ui_refinada::chapa(ui, "raio", "Tensão gerada", &format!("+{:.0}", cb_atual.tensao_mundial_gerada), Some(ui_refinada::menu::AVISO));
                                ui_refinada::chapa(ui, "jornal", "Infâmia", &format!("+{:.0}", cb_atual.penalidade_infamia), Some(ui_refinada::menu::PERIGO));
                            });

                            // Calcula consequências da declaração de guerra
                            let data_tuple = (mundo.0.data.ano as i32, mundo.0.data.mes, mundo.0.data.dia);
                            let alvo_norm = normalizar_tag(&alvo);
                            let ocupantes: Vec<String> = todos_os_ocupantes(g_sim, &alvo_norm, data_tuple);
                            let aliados_def: Vec<String> = aliados_com_obrigacao_defesa(&d.0, &alvo_norm);
                            let mut consequencias: Vec<String> = Vec::new();
                            if !ocupantes.is_empty() {
                                consequencias.push(format!("Ocupantes: {}", ocupantes.join(", ")));
                            }
                            if !aliados_def.is_empty() {
                                consequencias.push(format!("Aliados de defesa: {}", aliados_def.join(", ")));
                            }

                            // Mostra aviso se houver consequências
                            if !consequencias.is_empty() {
                                ui.horizontal(|ui| {
                                    ui_refinada::glifo_ui(ui, "alerta", 14.0, Some(ui_refinada::menu::PERIGO));
                                    ui.colored_label(ui_refinada::menu::PERIGO, &format!("⚠ Isso também te põe em guerra com: {}", consequencias.join("; ")));
                                });
                                ui.add_space(4.0);
                            }

                            let btn_decl = ui_refinada::botao_estilo_menu(ui, "Declarar Guerra");
                            if btn_decl.clicked() {
                                let nome_guerra = format!("{} de {} contra {}", cb_atual.nome, nome(&eu), nome(&alvo));
                                let eu_norm = normalizar_tag(&eu);
                                match declarar_com_aliados(g_sim, &d.0, &eu_norm, &alvo_norm, &nome_guerra, true, data_tuple) {
                                    Ok((g_id, ocupantes_entraram, aliados_def_entraram)) => {
                                        d.0.tensao = (d.0.tensao + cb_atual.tensao_mundial_gerada).min(100.0);
                                        let mut msg = format!("Guerra declarada contra {}! (ID: {})", nome(&alvo), g_id);
                                        if !ocupantes_entraram.is_empty() {
                                            msg.push_str(&format!("; {} (potência ocupante) entra na guerra contra você", ocupantes_entraram.join(", ")));
                                        }
                                        if !aliados_def_entraram.is_empty() {
                                            msg.push_str(&format!("; aliados de defesa {} entram na guerra contra você", aliados_def_entraram.join(", ")));
                                        }
                                        painel.mensagem = msg;
                                        if let Some(ref mut pg) = painel_guerra {
                                            pg.aberto = true;
                                            pg.repintar = true;
                                        }
                                    }
                                    Err(e) => {
                                        painel.mensagem = e;
                                    }
                                }
                            }
                        }
                    }
                }
                });
                if !painel.mensagem.is_empty() {
                    ui.horizontal(|ui| {
                        ui_refinada::glifo_ui(ui, "jornal", 14.0, Some(ui_refinada::menu::AVISO));
                        ui.colored_label(ui_refinada::menu::AVISO, &painel.mensagem);
                    });
                }
            }
            ui.add_space(6.0);
            ui_refinada::moldura_cartao().show(ui, |ui| {
                ui_refinada::titulo_secao(ui, "Organizações Internacionais");
                for o in &d.0.orgs {
                    match o.formada {
                        Some((a, m)) => {
                            let sou = o.membros.contains(&eu);
                            ui.horizontal(|ui| {
                                ui_refinada::glifo_ui(ui, "urna", 14.0, Some(ui_refinada::menu::AZUL_MARINHO));
                                ui.label(format!("{} — {m:02}/{a} — {} membros{}", o.nome, o.membros.len(), if sou { " (você é membro)" } else { "" }));
                            });
                        }
                        None => {
                            ui.weak(format!("{} — não formada (a partir de {:02}/{})", o.nome, o.data_min.1, o.data_min.0));
                        }
                    }
                }
            });
            if let Some(l) = d.0.log.last() {
                ui.weak(l);
            }
        });
    });
    if let Some(r) = resp_janela {
        super::tutorial_guiado::marcar(&r.response, "janela_diplo");
    }
    if gasto > 0.0
        && let Some(p) = mundo.0.paises.get_mut(&tag_legado)
    {
        p.orcamento.tesouro -= gasto;
    }
    if let (Some(alvo), Some(eco)) = (cmd_acordo, eco.as_deref_mut())
        && let Some(id) = eco.id_pais(&alvo)
    {
        eco.enviar(Comando::AssinarAcordo(id));
    }
    if mudou_mapa {
        painel.repintar = true;
    }
    painel.aberto = aberto;
    Ok(())
}
