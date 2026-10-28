//! Exemplo `economia_1945`: mapa vivo alimentado pelas construções e cidades REAIS de 1945.
//!
//! - Carrega 40.884 prédios de 4.594 províncias de `/home/rafael/claude/guerra-fria-staging/cenario_1945_economia`.
//! - Carrega 1.517 cidades com população de 1945 e nível 1 a 8.
//! - Layout em espiral sem sobreposição ao redor do centroide geográfico de cada província/estado.
//! - LOD: Longe (`Global`), badges estaduais com a contagem total de prédios; perto (`Local`), sprites individuais.
//! - Modo demonstração: crescimento contínuo ao longo do tempo (surgimento e subida de nível animados).
//!
//! Execução:
//! `cargo run -p visual_mapa --example economia_1945 --features demo --release`
//!
//! Controles: WASD/Setas (Pan), Q/E ou Scroll (Zoom), Espaço (Pausar/Retomar), N (Modo Noturno).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use std::path::Path;
use visual_mapa::cenario::{
    CIDADES_TERRITORIO_PADRAO, MAPA_PROVINCES_PADRAO, RAIZ_CENARIO_STAGING, carregar_cenario_1945,
};
use visual_mapa::demo::camera::{CameraMapa, controlar_camera_sistema};
use visual_mapa::entrada::{
    AnoVisual, CidadeEntrada, EntradaEconomiaVisual, EntradaVisual, PredioEconomiaEntrada,
    populacao_urbana_para_nivel,
};
use visual_mapa::era::GerenciadorEras;
use visual_mapa::lod::EstadoLod;
use visual_mapa::{VisualMapaConfig, VisualMapaPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Guerra Fria (1945) — Visual das Construções no Mapa".into(),
                resolution: (1600, 950).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.14, 0.24)))
        .insert_resource(VisualMapaConfig {
            limite_predios_por_estado: 32,
            espacamento_predios: 14.0,
            ..default()
        })
        .insert_resource(ControladorDemoCrescimento {
            ativo: true,
            timer: 0.0,
            intervalo: 0.8,
            velocidade: 1.0,
            predios_construidos_demo: 0,
            upgrades_realizados_demo: 0,
        })
        .add_plugins((VisualMapaPlugin, EguiPlugin::default()))
        .add_systems(Startup, (configurar_camera, criar_fundo_mapa, carregar_dados_iniciais_1945))
        .add_systems(
            Update,
            (
                controlar_camera_sistema,
                processar_crescimento_demo_sistema,
                hud_economia_sistema,
            ),
        )
        .run();
}

/// Estado do modo de demonstração de crescimento dinâmico.
#[derive(Resource)]
pub struct ControladorDemoCrescimento {
    pub ativo: bool,
    pub timer: f32,
    pub intervalo: f32,
    pub velocidade: f32,
    pub predios_construidos_demo: u32,
    pub upgrades_realizados_demo: u32,
}

fn configurar_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 3.5, // Começa em visão global/regional
            ..OrthographicProjection::default_2d()
        }),
        CameraMapa {
            velocidade_pan: 550.0,
            velocidade_zoom: 1.6,
            limite: Vec2::new(3000.0, 1200.0),
        },
    ));
}

/// Carrega o sprite do mapa real `provinces.png` do jogo.
fn criar_fundo_mapa(mut commands: Commands, mut imagens: ResMut<Assets<Image>>) {
    let caminho = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/map/provinces.png");
    let Ok(img) = image::open(caminho) else {
        warn!("provinces.png não encontrado em {caminho}; operando sem fundo.");
        return;
    };
    let img = img.to_rgba8();
    let (w, h) = img.dimensions();
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for p in img.pixels() {
        let [r, g, b, _] = p.0;
        if r == 0 && g == 0 && b == 0 {
            // Oceano
            px.extend_from_slice(&[20, 36, 60, 255]);
        } else {
            // Terra estilizada em tons sutis
            let t = 75 + ((r as u32 * 31 + g as u32 * 17 + b as u32 * 7) % 55) as u8;
            px.extend_from_slice(&[t, t.saturating_add(6), t.saturating_sub(6), 255]);
        }
    }
    let imagem = Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        px,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    commands.spawn((Sprite::from_image(imagens.add(imagem)), Transform::from_xyz(0.0, 0.0, 0.0)));
}

/// Carrega os dados reais do cenário de 1945 e alimenta a [`EntradaEconomiaVisual`] e [`EntradaVisual`].
fn carregar_dados_iniciais_1945(
    mut entrada_eco: ResMut<EntradaEconomiaVisual>,
    mut entrada_vis: ResMut<EntradaVisual>,
) {
    let caminho_eco = Path::new(RAIZ_CENARIO_STAGING).join("provincias.ron");
    let caminho_mapa = Path::new(MAPA_PROVINCES_PADRAO);
    let caminho_cidades = Path::new(CIDADES_TERRITORIO_PADRAO);

    let mut eco = carregar_cenario_1945(&caminho_eco, caminho_mapa, caminho_cidades);

    // Se o staging não estiver disponível, gera dados ilustrativos para garantir visualização
    if eco.total_predios() == 0 {
        warn!("Gerando construções de demonstração (cenário staging não localizado)...");
        let polos = [
            ("afg_cabul_central", "AFG", (69.17, 34.53)),
            ("usa_new_york", "USA", (-74.0, 40.7)),
            ("sov_moscow", "SOV", (37.6, 55.7)),
            ("bra_sao_paulo", "BRA", (-46.6, -23.5)),
            ("gbr_london", "GBR", (-0.12, 51.5)),
            ("fra_paris", "FRA", (2.35, 48.85)),
            ("jap_tokyo", "JAP", (139.7, 35.7)),
            ("chn_shanghai", "CHI", (121.47, 31.23)),
        ];
        let tipos = ["siderurgica", "fabrica_textil", "fazenda_graos", "usina_termeletrica", "porto_comercial"];
        for &(est, tag, pos) in &polos {
            for (k, &t) in tipos.iter().enumerate() {
                eco.adicionar_predio(PredioEconomiaEntrada {
                    id: format!("{est}_{t}_{k}"),
                    tipo: t.into(),
                    estado_id: est.into(),
                    pais_tag: tag.into(),
                    nivel: (k as u32 % 5) + 1,
                    posicao: Some(pos),
                    producao: 1.0,
                    ..default()
                });
            }
        }
    }

    // Alimentar cidades também em EntradaVisual para renderização urbana completa
    for c in &eco.cidades {
        let niv = populacao_urbana_para_nivel(c.populacao_urbana);
        entrada_vis.cidades.push(CidadeEntrada {
            id: c.id.clone(),
            nome: c.nome.clone(),
            pais_tag: c.pais_tag.clone(),
            posicao: c.posicao,
            nivel: niv as f32,
            populacao: (c.populacao_urbana.min(u32::MAX as u64)) as u32,
            eh_capital: c.eh_capital,
            bloco: c.bloco,
            era: Some(0),
            nivel_maximo: Some(8),
        });
    }

    *entrada_eco = eco;
}

/// Sistema do modo de demonstração: simula o crescimento econômico adicionando e expandindo prédios.
fn processar_crescimento_demo_sistema(
    time: Res<Time>,
    mut demo: ResMut<ControladorDemoCrescimento>,
    mut entrada: ResMut<EntradaEconomiaVisual>,
    ano: Res<AnoVisual>,
) {
    if !demo.ativo {
        return;
    }

    demo.timer += time.delta_secs() * demo.velocidade;
    if demo.timer < demo.intervalo {
        return;
    }
    demo.timer = 0.0;

    let total = entrada.predios.len();
    if total == 0 {
        return;
    }

    // 1. Alterna entre subir de nível de um prédio existente ou criar um novo
    let tick = demo.predios_construidos_demo + demo.upgrades_realizados_demo;
    if tick % 2 == 0 {
        // Subir de nível de um prédio existente
        let idx = (tick as usize * 17) % total;
        if let Some(p) = entrada.predios.get_mut(idx) {
            p.nivel = (p.nivel + 1).min(30);
            demo.upgrades_realizados_demo += 1;
        }
    } else {
        // Adicionar novo prédio em um estado com alta atividade
        let idx_base = (tick as usize * 31) % total;
        let base = entrada.predios[idx_base].clone();
        let novo_tipo = match (tick % 6) as usize {
            0 => "fabrica_eletronicos",
            1 => "siderurgica",
            2 => "usina_termeletrica",
            3 => "industria_quimica",
            4 => "fabrica_automoveis",
            _ => "complexo_militar_quartel",
        };

        let novo_id = format!("{}_{}_{}", base.estado_id, novo_tipo, demo.predios_construidos_demo);
        entrada.predios.push(PredioEconomiaEntrada {
            id: novo_id,
            tipo: novo_tipo.to_string(),
            estado_id: base.estado_id,
            provincia_id: base.provincia_id,
            pais_tag: base.pais_tag,
            nivel: 1,
            posicao: base.posicao,
            bloco: base.bloco,
            era: Some(((ano.0 as i32 - 1945) / 10).clamp(0, 4) as usize),
            producao: 1.0,
            ..default()
        });
        demo.predios_construidos_demo += 1;
    }
}

/// HUD interativo egui com telemetria, controles de LOD e modo de crescimento.
fn hud_economia_sistema(
    mut contexts: EguiContexts,
    mut demo: ResMut<ControladorDemoCrescimento>,
    mut entrada: ResMut<EntradaEconomiaVisual>,
    mut eras: ResMut<GerenciadorEras>,
    lod: Res<EstadoLod>,
    time: Res<Time>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let fps = 1.0 / time.delta_secs().max(0.0001);

    egui::Window::new("☭ 1945 — Visual das Construções no Mapa")
        .default_pos([18.0, 18.0])
        .default_width(340.0)
        .show(ctx, |ui| {
            ui.heading("Economia Real & Mapa Vivo");
            ui.separator();

            // Telemetria de Desempenho
            ui.horizontal(|ui| {
                let cor_fps = if fps >= 55.0 {
                    egui::Color32::from_rgb(100, 220, 100)
                } else if fps >= 30.0 {
                    egui::Color32::from_rgb(220, 200, 60)
                } else {
                    egui::Color32::from_rgb(240, 80, 80)
                };
                ui.label("Desempenho:");
                ui.label(egui::RichText::new(format!("{:.1} FPS", fps)).strong().color(cor_fps));
            });

            // Contagens de elementos
            ui.label(format!("🏢 Construções no Mapa: {}", entrada.total_predios()));
            ui.label(format!("🏙 Cidades Carregadas: {}", entrada.total_cidades()));

            // Nível de Detalhe (LOD)
            ui.horizontal(|ui| {
                ui.label("LOD Ativo:");
                let (lod_txt, cor_lod) = match lod.nivel_atual {
                    visual_mapa::lod::NivelLod::Global => ("Global (Ícones Estaduais)", egui::Color32::from_rgb(120, 180, 240)),
                    visual_mapa::lod::NivelLod::Regional => ("Regional (Misto)", egui::Color32::from_rgb(220, 180, 100)),
                    visual_mapa::lod::NivelLod::Local => ("Local (Sprites Individuais)", egui::Color32::from_rgb(140, 230, 140)),
                };
                ui.label(egui::RichText::new(lod_txt).strong().color(cor_lod));
            });
            ui.label(format!("Escala Câmera: {:.2}x", lod.escala_camera));

            ui.separator();
            ui.heading("Modo Demonstração (Crescimento)");
            ui.checkbox(&mut demo.ativo, "Simular Crescimento Ativo");
            if demo.ativo {
                ui.add(egui::Slider::new(&mut demo.velocidade, 0.2..=5.0).text("Velocidade"));
                ui.label(format!("✨ Novos Prédios Criados: {}", demo.predios_construidos_demo));
                ui.label(format!("⬆ Níveis Expandidos: {}", demo.upgrades_realizados_demo));
            }

            ui.horizontal(|ui| {
                if ui.button("➕ Construir +10 Prédios").clicked() {
                    for i in 0..10 {
                        let total = entrada.predios.len();
                        if total > 0 {
                            let mut base = entrada.predios[i % total].clone();
                            base.id = format!("{}_extra_{}", base.estado_id, demo.predios_construidos_demo);
                            base.nivel = 1;
                            entrada.predios.push(base);
                            demo.predios_construidos_demo += 1;
                        }
                    }
                }

                if ui.button("🌙 Alternar Noite").clicked() {
                    eras.modo_noturno = !eras.modo_noturno;
                }
            });

            ui.separator();
            ui.collapsing("Guia de Controles", |ui| {
                ui.label("• WASD ou Setas: Mover câmera pelo globo");
                ui.label("• Q / E ou Roda do Mouse: Zoom (altera LOD)");
                ui.label("• Espaço: Pausar/Continuar crescimento");
                ui.label("• N: Alternar modo noturno com iluminação");
            });
        });
}
