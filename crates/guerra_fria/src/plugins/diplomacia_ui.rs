//! Tela de Diplomacia (F4) e modo de mapa "Esferas": liga o crate puro `sim_diplomacia` ao cliente.

use super::economia_sim::{Comando, EconomiaSim};
use super::economy::RecursoMundo;
use super::jogador::PaisJogador;
use super::map::{CoresMapa, DadosMapa, ImagemMapa, colorir};
use super::time::EventoTickDiario;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use sim_diplomacia::{Acao, Bloco, Diplomacia, Esfera, InfoPais};
use simulacao::mapa::AlinhamentoGeopolitico;

#[derive(Resource)]
pub struct DiploSim(pub Diplomacia);

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

fn alternar(teclas: Res<ButtonInput<KeyCode>>, mut p: ResMut<PainelDiplomacia>) {
    if teclas.just_pressed(KeyCode::F4) {
        p.aberto = !p.aberto;
    }
}

fn tick_diplomacia(
    mut eventos: MessageReader<EventoTickDiario>,
    mut d: Option<ResMut<DiploSim>>,
    jogador: Res<PaisJogador>,
    mut painel: ResMut<PainelDiplomacia>,
) {
    let Some(d) = d.as_deref_mut() else { return };
    let jog = normalizar(&jogador.0);
    for e in eventos.read() {
        d.0.tick_dia(e.nova_data.ano, e.nova_data.mes, e.nova_data.dia, Some(&jog));
        painel.dias_desde_pintura += 1;
    }
    if painel.mapa_esferas && painel.dias_desde_pintura >= 30 {
        painel.repintar = true;
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
    let (Some(d), Some(cores), Some(dados), Some(imagem)) = (d, cores, dados, imagem) else { return };
    let mut cor_prov = cores.cor_prov.clone();
    if painel.mapa_esferas {
        for (id, tag) in &mundo.0.dono_por_provincia {
            if let Some(slot) = cor_prov.get_mut(*id as usize) {
                let [r, g, b] = Diplomacia::cor_esfera(d.0.esfera(&normalizar(tag)));
                *slot = [r, g, b, 255];
            }
        }
    }
    let (pixels, _) = colorir(&dados.ids, dados.largura, dados.altura, &cor_prov, &cores.dono_prov, &cores.estado_prov);
    let mut img = Image::new(
        Extent3d { width: dados.largura, height: dados.altura, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    let _ = imagens.insert(imagem.0.id(), img);
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

    egui::Window::new("Diplomacia (F4)").open(&mut aberto).default_size([900.0, 600.0]).show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.group(|ui| {
                ui.label("Tensão mundial:");
                let t = d.0.tensao;
                let cor = if t > 60.0 { egui::Color32::LIGHT_RED } else if t > 30.0 { egui::Color32::YELLOW } else { egui::Color32::LIGHT_GREEN };
                ui.colored_label(cor, format!("{t:.0}/100"));
            });
            ui.group(|ui| {
                ui.label("Pontos de influência:");
                ui.strong(format!("{:.0}", d.0.pontos.get(&eu).copied().unwrap_or(0.0)));
            });
            ui.group(|ui| {
                ui.label("Sua esfera:");
                let e = d.0.esfera(&eu);
                ui.colored_label(cor_egui(e), nome_esfera(e));
            });
            if ui.checkbox(&mut painel.mapa_esferas, "Mapa: Esferas").changed() {
                mudou_mapa = true;
            }
        });
        ui.separator();
        ui.columns(2, |cols| {
            // ---- Coluna esquerda: lista de países ----
            cols[0].horizontal(|ui| {
                ui.label("Buscar:");
                ui.text_edit_singleline(&mut painel.busca);
            });
            egui::ScrollArea::vertical().id_salt("diplo_lista").max_height(480.0).show(&mut cols[0], |ui| {
                let busca = painel.busca.to_lowercase();
                let tags: Vec<String> = d.0.paises.iter().map(|p| p.tag.clone()).collect();
                for t in tags {
                    if t == eu || (!busca.is_empty() && !nome(&t).to_lowercase().contains(&busca) && !t.to_lowercase().contains(&busca)) {
                        continue;
                    }
                    let r = d.0.relacao(&eu, &t);
                    let e = d.0.esfera(&t);
                    ui.horizontal(|ui| {
                        ui.colored_label(cor_egui(e), "■");
                        let sel = painel.alvo == t;
                        if ui.selectable_label(sel, format!("{} ({t})", nome(&t))).clicked() {
                            painel.alvo = t.clone();
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let c = if r >= 21.0 { egui::Color32::LIGHT_GREEN } else if r <= -21.0 { egui::Color32::LIGHT_RED } else { egui::Color32::GRAY };
                            ui.colored_label(c, format!("{r:+.0}"));
                        });
                    });
                }
            });
            // ---- Coluna direita: detalhes e ações ----
            let ui = &mut cols[1];
            if painel.alvo.is_empty() {
                ui.label("Selecione um país na lista.");
            } else {
                let alvo = painel.alvo.clone();
                let r = d.0.relacao(&eu, &alvo);
                ui.heading(nome(&alvo));
                ui.label(format!("Relação: {r:+.0} ({})", Diplomacia::nivel_relacao(r)));
                let e = d.0.esfera(&alvo);
                let (iu, is) = d.0.influencia(&alvo);
                ui.horizontal(|ui| {
                    ui.label("Esfera:");
                    ui.colored_label(cor_egui(e), nome_esfera(e));
                    ui.label(format!("(influência EUA {iu:.0} / URSS {is:.0})"));
                });
                for p in d.0.pactos_de(&alvo).filter(|p| p.a == eu || p.b == eu) {
                    ui.label(format!("• {} com você", p.tipo.nome()));
                }
                ui.separator();
                ui.strong("Ações");
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
                ui.separator();
                ui.strong("Operações Militares");
                if let Some(ref mut g_sim) = guerra_sim {
                    let guerra_ativa = g_sim.0.guerras().iter().find(|gw| gw.ativa && gw.sao_inimigos(&eu, &alvo)).cloned();
                    if let Some(gw) = guerra_ativa {
                        ui.colored_label(egui::Color32::from_rgb(240, 80, 80), "⚔ VOCÊ ESTÁ EM GUERRA COM ESTE PAÍS");
                        let score = if gw.eh_atacante(&eu) { gw.pontuacao_guerra } else { -gw.pontuacao_guerra };
                        let cor_s = if score > 0.0 { egui::Color32::LIGHT_GREEN } else if score < 0.0 { egui::Color32::LIGHT_RED } else { egui::Color32::GRAY };
                        ui.horizontal(|ui| {
                            ui.label("Pontuação de Guerra:");
                            ui.colored_label(cor_s, format!("{score:+.1}%"));
                        });
                        if ui.button("Abrir Painel de Guerra e Paz (F6)").clicked() {
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
                            ui.label(format!("Tensão gerada: +{:.0} | Infâmia: +{:.0}", cb_atual.tensao_mundial_gerada, cb_atual.penalidade_infamia));

                            let btn_decl = ui.button(egui::RichText::new("⚔ DECLARAR GUERRA").color(egui::Color32::from_rgb(255, 90, 90)));
                            if btn_decl.clicked() {
                                let nome_guerra = format!("{} de {} contra {}", cb_atual.nome, nome(&eu), nome(&alvo));
                                match super::guerra_sim::acao_declarar_guerra(g_sim, &eu, &alvo, &nome_guerra) {
                                    Ok(g_id) => {
                                        d.0.tensao = (d.0.tensao + cb_atual.tensao_mundial_gerada).min(100.0);
                                        painel.mensagem = format!("Guerra declarada contra {}! (ID: {})", nome(&alvo), g_id);
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
                if !painel.mensagem.is_empty() {
                    ui.colored_label(egui::Color32::YELLOW, &painel.mensagem);
                }
            }
            ui.separator();
            ui.strong("Organizações");
            for o in &d.0.orgs {
                match o.formada {
                    Some((a, m)) => {
                        let sou = o.membros.contains(&eu);
                        ui.label(format!("{} — {m:02}/{a} — {} membros{}", o.nome, o.membros.len(), if sou { " (você é membro)" } else { "" }));
                    }
                    None => {
                        ui.weak(format!("{} — não formada (a partir de {:02}/{})", o.nome, o.data_min.1, o.data_min.0));
                    }
                }
            }
            if let Some(l) = d.0.log.last() {
                ui.weak(l);
            }
        });
    });
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
