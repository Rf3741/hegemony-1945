//! HUD estendido: botões de módulos (F4/F5/F6), barra de alertas e modos de mapa temáticos com legenda.
use super::diplomacia_ui::{DiploSim, PainelDiplomacia};
use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::jogador::{PaisJogador, fmt_pop};
use super::map::{CoresMapa, DadosMapa, ImagemMapa, colorir};
use super::politica_sim::PoliticaSim;
use super::tecnologia_ui::TecnologiaSim;
use super::time::{EventoTickDiario, RecursoRelogio};
use super::ui::ui_raiz;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use simulacao::mundo::EstadoPais;

/// Modo de mapa temático (além dos de construção, que ficam em `construcao_ui`).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModoTematico {
    #[default]
    Politico,
    Populacao,
    PibPerCapita,
    Ideologia,
    Alinhamento,
    PadraoVida,
}

impl ModoTematico {
    pub const TODOS: [ModoTematico; 6] = [Self::Politico, Self::Populacao, Self::PibPerCapita, Self::Ideologia, Self::Alinhamento, Self::PadraoVida];

    pub fn nome(self) -> &'static str {
        match self {
            Self::Politico => "Político",
            Self::Populacao => "População",
            Self::PibPerCapita => "PIB per capita",
            Self::Ideologia => "Ideologia",
            Self::Alinhamento => "Alinhamento/Esferas",
            Self::PadraoVida => "Padrão de vida",
        }
    }

    /// Próximo (ou anterior) modo, circular.
    pub fn vizinho(self, anterior: bool) -> ModoTematico {
        let n = Self::TODOS.len();
        let i = Self::TODOS.iter().position(|m| *m == self).unwrap_or(0);
        Self::TODOS[if anterior { (i + n - 1) % n } else { (i + 1) % n }]
    }

    pub fn tecla(self) -> KeyCode {
        match self {
            Self::Politico => KeyCode::KeyQ,
            Self::Populacao => KeyCode::KeyE,
            Self::PibPerCapita => KeyCode::KeyR,
            Self::Ideologia => KeyCode::KeyT,
            Self::Alinhamento => KeyCode::KeyY,
            Self::PadraoVida => KeyCode::KeyU,
        }
    }

    pub fn rotulo_tecla(self) -> &'static str {
        match self {
            Self::Politico => "Q",
            Self::Populacao => "E",
            Self::PibPerCapita => "R",
            Self::Ideologia => "T",
            Self::Alinhamento => "Y",
            Self::PadraoVida => "U",
        }
    }
}

/// Rampa sequencial de 0 a 1 (azul-escuro, verde, amarelo, vermelho-tijolo).
pub fn rampa(t: f32) -> [u8; 4] {
    const PONTOS: [[f32; 3]; 5] = [[40.0, 60.0, 110.0], [50.0, 130.0, 150.0], [110.0, 170.0, 90.0], [230.0, 200.0, 80.0], [190.0, 70.0, 50.0]];
    let t = t.clamp(0.0, 1.0) * 4.0;
    let i = (t.floor() as usize).min(3);
    let f = t - i as f32;
    let (a, b) = (PONTOS[i], PONTOS[i + 1]);
    [(a[0] + (b[0] - a[0]) * f) as u8, (a[1] + (b[1] - a[1]) * f) as u8, (a[2] + (b[2] - a[2]) * f) as u8, 255]
}

/// Escala logarítmica normalizada: `min` vira 0 e `max` vira 1.
pub fn escala_log(v: f64, min: f64, max: f64) -> f32 {
    if v <= min {
        return 0.0;
    }
    (((v / min).ln()) / ((max / min).ln())).clamp(0.0, 1.0) as f32
}

/// Categoria política da ideologia: (rótulo, cor).
pub fn categoria_ideologia(ideologia: &str) -> (&'static str, [u8; 4]) {
    let i = ideologia;
    let tem = |ks: &[&str]| ks.iter().any(|k| i.contains(k));
    if tem(&["comunis", "socialis", "sovietic", "popular", "marxis"]) {
        ("Comunista/socialista", [190, 50, 45, 255])
    } else if tem(&["fascis", "nazis", "autorit", "militar", "ditadura", "junta", "falang", "corporativ"]) {
        ("Autoritário/militar", [70, 60, 60, 255])
    } else if tem(&["colon", "protetorado", "mandato", "condominio", "commonwealth", "ocupa"]) {
        ("Colônia/ocupação", [175, 160, 110, 255])
    } else if tem(&["monarq", "imperi", "reino", "sultan", "emirado", "xeque"]) {
        ("Monarquia", [140, 90, 160, 255])
    } else if tem(&["democra", "liberal", "republic", "parlament", "trabalh"]) {
        ("Democracia", [60, 120, 200, 255])
    } else {
        ("Outros", [140, 140, 140, 255])
    }
}

pub const LEGENDA_CATEGORIAS: [&str; 6] = ["Comunista/socialista", "Autoritário/militar", "Colônia/ocupação", "Monarquia", "Democracia", "Outros"];

/// Valor temático de um país (para as rampas) e cor final.
pub fn cor_pais_modo(modo: ModoTematico, p: &EstadoPais) -> [u8; 4] {
    match modo {
        ModoTematico::PibPerCapita => {
            let pop = p.populacao_total().max(1) as f64;
            rampa(escala_log(p.pib_anual_estimado / pop, 20.0, 20_000.0))
        }
        ModoTematico::Ideologia => categoria_ideologia(&p.ideologia).1,
        ModoTematico::PadraoVida => rampa((p.padrao_vida_medio() / 20.0).clamp(0.0, 1.0)),
        _ => [p.cor.0, p.cor.1, p.cor.2, 255],
    }
}

/// Sistema: troca o modo por tecla.
fn teclas_modos(teclas: Res<ButtonInput<KeyCode>>, mut modo: ResMut<ModoTematico>) {
    // M cicla para frente; Shift+M volta.
    if teclas.just_pressed(KeyCode::KeyM) {
        let recuar = teclas.pressed(KeyCode::ShiftLeft) || teclas.pressed(KeyCode::ShiftRight);
        *modo = modo.vizinho(recuar);
    }
    for m in ModoTematico::TODOS {
        if teclas.just_pressed(m.tecla()) {
            *modo = m;
        }
    }
}

/// Repinta a textura do mapa quando o modo muda, e a cada 30 dias enquanto o modo está ativo.
fn repintar_tematico(
    modo: Res<ModoTematico>,
    mut ticks: MessageReader<EventoTickDiario>,
    mut dias: Local<u32>,
    mut ultimo: Local<Option<ModoTematico>>,
    mundo: Res<RecursoMundo>,
    cores: Option<Res<CoresMapa>>,
    dados: Option<Res<DadosMapa>>,
    imagem: Option<Res<ImagemMapa>>,
    diplo: Option<Res<DiploSim>>,
    painel: Option<ResMut<PainelDiplomacia>>,
    mut imagens: ResMut<Assets<Image>>,
) {
    *dias += ticks.read().count() as u32;
    let mudou = *ultimo != Some(*modo);
    if !mudou && !(*modo != ModoTematico::Politico && *dias >= 30) {
        return;
    }
    let anterior = *ultimo;
    *ultimo = Some(*modo);
    *dias = 0;
    // Alinhamento reaproveita o modo "Esferas" do módulo de diplomacia.
    if let Some(mut p) = painel {
        let quer = *modo == ModoTematico::Alinhamento;
        if p.mapa_esferas != quer {
            p.mapa_esferas = quer;
            p.repintar = true;
        }
    }
    if *modo == ModoTematico::Alinhamento {
        return;
    }
    // Primeira passada em modo Político: o mapa já está pintado.
    if anterior.is_none() && *modo == ModoTematico::Politico {
        return;
    }
    let (Some(cores), Some(dados), Some(imagem)) = (cores, dados, imagem) else { return };
    let _ = diplo;
    let mut cor_prov = cores.cor_prov.clone();
    if *modo != ModoTematico::Politico {
        let (mut min, mut max) = (u64::MAX, 0u64);
        for v in mundo.populacao_por_provincia.values() {
            if *v > 0 {
                min = min.min(*v);
                max = max.max(*v);
            }
        }
        let (min, max) = (min.max(1) as f64, (max.max(2)) as f64);
        for (id, tag) in &mundo.dono_por_provincia {
            let Some(slot) = cor_prov.get_mut(*id as usize) else { continue };
            *slot = match *modo {
                ModoTematico::Populacao => rampa(escala_log(mundo.populacao_por_provincia.get(id).copied().unwrap_or(0) as f64, min, max)),
                _ => mundo.paises.get(tag).map_or([60, 60, 60, 255], |p| cor_pais_modo(*modo, p)),
            };
        }
    }
    let (pixels, _) = colorir(&dados.ids, dados.largura, dados.altura, &cor_prov, &cores.dono_prov, &cores.estado_prov);
    if let Some(mut img) = imagens.get_mut(&imagem.0) {
        img.data = Some(pixels);
    } else {
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
}

/// Um alerta da barra (estilo Victoria 3).
#[derive(Debug, Clone, PartialEq)]
pub struct Alerta {
    pub icone: &'static str,
    pub texto: String,
    pub gravidade: u8, // 0 = info, 1 = aviso, 2 = crítico
}

/// Insumos puros dos alertas (testável sem ECS).
#[derive(Debug, Clone, Default)]
pub struct InsumosAlertas {
    pub tesouro: Option<f64>,
    pub saldo_diario: f64,
    pub em_calote: bool,
    pub desemprego: f64,
    pub lei_pronta: Option<String>,
    pub dias_eleicao: Option<u32>,
    pub em_campanha: bool,
    pub pesquisa_concluida: Option<String>,
}

pub fn calcular_alertas(i: &InsumosAlertas) -> Vec<Alerta> {
    let mut v = Vec::new();
    if i.em_calote {
        v.push(Alerta { icone: "💥", texto: "Calote: o governo não paga suas dívidas".into(), gravidade: 2 });
    } else if let Some(t) = i.tesouro
        && i.saldo_diario < 0.0
    {
        let dias = t / -i.saldo_diario;
        if dias < 90.0 {
            v.push(Alerta { icone: "💸", texto: format!("Falência próxima: tesouro acaba em {:.0} dias", dias.max(0.0)), gravidade: if dias < 30.0 { 2 } else { 1 } });
        }
    }
    if i.desemprego > 0.12 {
        v.push(Alerta { icone: "👷", texto: format!("Desemprego alto: {:.1}%", i.desemprego * 100.0), gravidade: if i.desemprego > 0.2 { 2 } else { 1 } });
    }
    if let Some(l) = &i.lei_pronta {
        v.push(Alerta { icone: "📜", texto: format!("Lei pronta para votação final: {l}"), gravidade: 0 });
    }
    if i.em_campanha {
        v.push(Alerta { icone: "🗳", texto: "Campanha eleitoral em andamento".into(), gravidade: 0 });
    } else if let Some(d) = i.dias_eleicao
        && d <= 60
    {
        v.push(Alerta { icone: "🗳", texto: format!("Eleição em {d} dias"), gravidade: 0 });
    }
    if let Some(t) = &i.pesquisa_concluida {
        v.push(Alerta { icone: "🔬", texto: format!("Pesquisa concluída: {t}"), gravidade: 0 });
    }
    v
}

fn cor_gravidade(g: u8) -> egui::Color32 {
    match g {
        2 => egui::Color32::from_rgb(220, 80, 70),
        1 => egui::Color32::from_rgb(230, 190, 70),
        _ => egui::Color32::from_rgb(170, 190, 210),
    }
}

/// Barra de módulos + modos de mapa + alertas (logo abaixo do HUD do país).
#[allow(clippy::too_many_arguments)]
pub fn barra_extra(
    mut contextos: EguiContexts,
    mut modo: ResMut<ModoTematico>,
    eco: Option<Res<EconomiaSim>>,
    pol: Option<Res<PoliticaSim>>,
    mut tec: Option<ResMut<TecnologiaSim>>,
    mut diplo: Option<ResMut<PainelDiplomacia>>,
    jogador: Option<Res<PaisJogador>>,
    relogio: Res<RecursoRelogio>,
) -> Result {
    let ctx = contextos.ctx_mut()?.clone();
    let mut raiz = ui_raiz(&ctx);
    let mut alertas_in = InsumosAlertas::default();
    if let Some(eco) = &eco
        && let Some(p) = &eco.vista.pais
        && !eco.vista.pais_id.is_empty()
    {
        alertas_in.tesouro = Some(p.orcamento.tesouro);
        alertas_in.saldo_diario = p.orcamento.saldo_diario;
        alertas_in.em_calote = p.orcamento.em_calote;
        alertas_in.desemprego = eco.vista.desemprego;
    }
    if let Some(pp) = pol.as_ref().and_then(|p| p.vista.pais.as_ref()) {
        if let Some(l) = &pp.lei_em_tramitacao
            && l.fase == sim_politica::FaseLegislativa::VotacaoFinal
        {
            alertas_in.lei_pronta = Some(l.lei_alvo_id.replace('_', " "));
        }
        alertas_in.dias_eleicao = Some(pp.dias_para_proxima_eleicao);
        alertas_in.em_campanha = pp.campanha_eleitoral.is_some();
    }
    if let Some(t) = &tec {
        alertas_in.pesquisa_concluida = t.concluidas_jogador.last().cloned();
    }
    let alertas = calcular_alertas(&alertas_in);
    let _ = (&jogador, &relogio);
    egui::Panel::top("barra_modulos").show(&mut raiz, |ui| {
        ui.horizontal_wrapped(|ui| {
            if let Some(d) = diplo.as_mut()
                && ui.selectable_label(d.aberto, "Diplomacia (F4)").clicked()
            {
                d.aberto = !d.aberto;
            }
            if let Some(t) = tec.as_mut()
                && ui.selectable_label(t.aberto, "Tecnologia (F5)").clicked()
            {
                t.aberto = !t.aberto;
            }
            ui.add_enabled(false, egui::Button::new("Guerra (F6)")).on_disabled_hover_text("A interface de guerra ainda não está integrada.");
            ui.separator();
            ui.label("Mapa:");
            for m in ModoTematico::TODOS {
                if ui.selectable_label(*modo == m, format!("{} ({})", m.nome(), m.rotulo_tecla())).clicked() {
                    *modo = m;
                }
            }
        });
        if !alertas.is_empty() {
            ui.horizontal_wrapped(|ui| {
                for a in &alertas {
                    ui.label(egui::RichText::new(format!("{} {}", a.icone, a.texto)).color(cor_gravidade(a.gravidade)).strong());
                    ui.add_space(8.0);
                }
            });
        }
    });
    Ok(())
}

/// Seletor de modo no canto do mapa (canto inferior direito).
fn seletor_canto(mut contextos: EguiContexts, mut modo: ResMut<ModoTematico>) -> Result {
    let ctx = contextos.ctx_mut()?;
    egui::Area::new("seletor_modo_mapa".into()).anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0]).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.small_button("<").on_hover_text("Modo anterior (Shift+M)").clicked() {
                    *modo = modo.vizinho(true);
                }
                egui::ComboBox::from_id_salt("combo_modo_mapa").selected_text(format!("Mapa: {} (M)", modo.nome())).show_ui(ui, |ui| {
                    for m in ModoTematico::TODOS {
                        ui.selectable_value(&mut *modo, m, m.nome());
                    }
                });
                if ui.small_button(">").on_hover_text("Próximo modo (M)").clicked() {
                    *modo = modo.vizinho(false);
                }
            });
        });
    });
    Ok(())
}

/// Legenda do modo ativo (canto inferior esquerdo).
fn legenda(mut contextos: EguiContexts, modo: Res<ModoTematico>, mundo: Res<RecursoMundo>) -> Result {
    if matches!(*modo, ModoTematico::Politico) {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?;
    egui::Area::new("legenda_mapa".into()).anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0]).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.strong(format!("Mapa: {}", modo.nome()));
            let amostra = |ui: &mut egui::Ui, c: [u8; 4], txt: &str| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
                    ui.label(txt);
                });
            };
            match *modo {
                ModoTematico::Populacao | ModoTematico::PibPerCapita | ModoTematico::PadraoVida => {
                    let (baixo, alto) = match *modo {
                        ModoTematico::Populacao => ("poucos habitantes", "muitos habitantes"),
                        ModoTematico::PibPerCapita => ("PIB per capita baixo", "PIB per capita alto"),
                        _ => ("padrão de vida baixo", "padrão de vida alto"),
                    };
                    for k in 0..=4 {
                        let t = k as f32 / 4.0;
                        let txt = match k {
                            0 => baixo.to_string(),
                            4 => alto.to_string(),
                            _ => String::new(),
                        };
                        amostra(ui, rampa(t), &txt);
                    }
                    if *modo == ModoTematico::Populacao {
                        let max = mundo.populacao_por_provincia.values().copied().max().unwrap_or(0);
                        ui.weak(format!("Maior província: {} hab.", fmt_pop(max)));
                    }
                }
                ModoTematico::Ideologia => {
                    for nome in LEGENDA_CATEGORIAS {
                        let cor = match nome {
                            "Comunista/socialista" => [190, 50, 45, 255],
                            "Autoritário/militar" => [70, 60, 60, 255],
                            "Colônia/ocupação" => [175, 160, 110, 255],
                            "Monarquia" => [140, 90, 160, 255],
                            "Democracia" => [60, 120, 200, 255],
                            _ => [140, 140, 140, 255],
                        };
                        amostra(ui, cor, nome);
                    }
                }
                ModoTematico::Alinhamento => {
                    use sim_diplomacia::{Diplomacia, Esfera};
                    for (e, n) in [(Esfera::Eua, "Esfera dos EUA"), (Esfera::Urss, "Esfera da URSS"), (Esfera::NaoAlinhado, "Não alinhado"), (Esfera::Disputada, "Disputada")] {
                        let c = Diplomacia::cor_esfera(e);
                        amostra(ui, [c[0], c[1], c[2], 255], n);
                    }
                }
                ModoTematico::Politico => {}
            }
        });
    });
    Ok(())
}

pub struct HudMapasPlugin;

impl Plugin for HudMapasPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModoTematico>()
            .add_systems(Update, (teclas_modos, repintar_tematico).chain().run_if(crate::estado::em_partida))
            .add_systems(EguiPrimaryContextPass, (legenda, seletor_canto).run_if(crate::estado::em_partida));
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ciclo_de_modos() {
        assert_eq!(ModoTematico::Politico.vizinho(false), ModoTematico::Populacao);
        assert_eq!(ModoTematico::Politico.vizinho(true), ModoTematico::PadraoVida);
        assert_eq!(ModoTematico::PadraoVida.vizinho(false), ModoTematico::Politico);
    }

    #[test]
    fn rampa_e_escala() {
        assert_eq!(rampa(0.0), [40, 60, 110, 255]);
        assert_eq!(rampa(1.0), [190, 70, 50, 255]);
        assert_eq!(escala_log(10.0, 10.0, 1000.0), 0.0);
        assert!((escala_log(100.0, 10.0, 1000.0) - 0.5).abs() < 1e-6);
        assert_eq!(escala_log(1e9, 10.0, 1000.0), 1.0);
    }

    #[test]
    fn ideologias_classificadas() {
        assert_eq!(categoria_ideologia("comunismo").0, "Comunista/socialista");
        assert_eq!(categoria_ideologia("colonia_coroa").0, "Colônia/ocupação");
        assert_eq!(categoria_ideologia("democracia_batllista").0, "Democracia");
        assert_eq!(categoria_ideologia("autoritarismo_carias").0, "Autoritário/militar");
        for n in LEGENDA_CATEGORIAS {
            assert!(["Comunista/socialista", "Autoritário/militar", "Colônia/ocupação", "Monarquia", "Democracia", "Outros"].contains(&n));
        }
    }

    #[test]
    fn alertas_disparam() {
        let i = InsumosAlertas { tesouro: Some(100.0), saldo_diario: -10.0, desemprego: 0.25, lei_pronta: Some("voto".into()), dias_eleicao: Some(30), ..Default::default() };
        let a = calcular_alertas(&i);
        assert_eq!(a.len(), 4);
        assert_eq!(a[0].gravidade, 2); // falência em 10 dias
        assert!(calcular_alertas(&InsumosAlertas { tesouro: Some(1e6), saldo_diario: 5.0, ..Default::default() }).is_empty());
        assert!(calcular_alertas(&InsumosAlertas { em_calote: true, ..Default::default() })[0].texto.contains("Calote"));
    }
}
