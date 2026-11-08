//! HUD estendido: botões de módulos (4/5/6), barra de alertas e modos de mapa temáticos com legenda.
use super::diplomacia_ui::{DiploSim, PainelDiplomacia};
use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::jogador::PaisJogador;
use super::map::{CoresMapa, DadosMapa, ImagemMapa, RaizProjeto, RecursoCatalogo, pintar_paleta};
use super::mapa_provincial::{Escala, Metrica, PesosProvincias};
use super::politica_sim::PoliticaSim;
use super::tecnologia_ui::TecnologiaSim;
use super::time::{EventoTickDiario, RecursoRelogio};
use super::ui::{area_livre_mapa, consumir_topo, ui_raiz};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use simulacao::mundo::EstadoPais;

/// Modo de mapa temático (além dos de construção, que ficam em `construcao_ui`).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModoTematico {
    #[default]
    Politico,
    Populacao,
    Densidade,
    PibTotal,
    PibPerCapita,
    PadraoVida,
    Urbanizacao,
    Industrializacao,
    Ideologia,
    Alinhamento,
}

impl ModoTematico {
    pub const TODOS: [ModoTematico; 10] = [
        Self::Politico,
        Self::Populacao,
        Self::Densidade,
        Self::PibTotal,
        Self::PibPerCapita,
        Self::PadraoVida,
        Self::Urbanizacao,
        Self::Industrializacao,
        Self::Ideologia,
        Self::Alinhamento,
    ];

    /// Métrica por província do modo (None nos modos políticos).
    pub fn metrica(self) -> Option<Metrica> {
        Some(match self {
            Self::Populacao => Metrica::Populacao,
            Self::Densidade => Metrica::Densidade,
            Self::PibTotal => Metrica::PibTotal,
            Self::PibPerCapita => Metrica::PibPerCapita,
            Self::PadraoVida => Metrica::PadraoVida,
            Self::Urbanizacao => Metrica::Urbanizacao,
            Self::Industrializacao => Metrica::Industrializacao,
            _ => return None,
        })
    }

    pub fn nome(self) -> &'static str {
        match self {
            Self::Politico => "Político",
            Self::Populacao => "População",
            Self::Densidade => "Densidade demográfica",
            Self::PibTotal => "PIB total",
            Self::PibPerCapita => "PIB per capita",
            Self::Urbanizacao => "Urbanização",
            Self::Industrializacao => "Industrialização",
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
    eco: Option<Res<EconomiaSim>>,
    catalogo: Option<Res<RecursoCatalogo>>,
    raiz: Option<Res<RaizProjeto>>,
    mut pesos: Local<Option<PesosProvincias>>,
    mut escala_ativa: ResMut<EscalaAtiva>,
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
        let valores = modo.metrica().and_then(|m| {
            let (cat, eco) = (catalogo.as_ref()?, eco.as_ref()?);
            if eco.vista.regioes.is_empty() {
                return None;
            }
            let p = pesos.get_or_insert_with(|| {
                let urbana = urbana_por_provincia(raiz.as_deref().map(|r| r.0.as_path()), &dados);
                PesosProvincias::novo(&cat.0.provincias, &urbana)
            });
            Some((m, p.valores(m, &eco.vista.regioes)))
        });
        let escala = valores.as_ref().map(|(m, v)| Escala::de_valores(v, m.logaritmica()));
        if let (Some(e), Some((m, _))) = (escala, valores.as_ref()) {
            *escala_ativa = EscalaAtiva { modo: Some(*modo), escala: e, metrica: Some(*m) };
        }
        for (id, tag) in &mundo.dono_por_provincia {
            let Some(slot) = cor_prov.get_mut(*id as usize) else { continue };
            let por_prov = match (&valores, escala) {
                (Some((_, v)), Some(e)) => v.get(*id as usize).copied().filter(|x| x.is_finite()).map(|x| rampa(e.normalizar(x))),
                _ => None,
            };
            *slot = por_prov.unwrap_or_else(|| match *modo {
                ModoTematico::Populacao | ModoTematico::Densidade | ModoTematico::PibTotal | ModoTematico::Urbanizacao | ModoTematico::Industrializacao => [60, 60, 60, 255],
                _ => mundo.paises.get(tag).map_or([60, 60, 60, 255], |p| cor_pais_modo(*modo, p)),
            });
        }
    }
    pintar_paleta(&mut imagens, &imagem, &cor_prov, &cores.dono_prov, &cores.estado_prov, None);
}

/// Escala em uso no mapa (para a legenda).
#[derive(Resource, Default)]
pub struct EscalaAtiva {
    pub modo: Option<ModoTematico>,
    pub escala: Escala,
    pub metrica: Option<Metrica>,
}

/// População urbana (cidades de 1945) atribuída à província do mapa onde cada cidade cai.
fn urbana_por_provincia(raiz: Option<&std::path::Path>, dados: &DadosMapa) -> std::collections::HashMap<u32, f64> {
    let caminho = raiz.map_or_else(|| std::path::PathBuf::from("assets/data/territorio/cidades.ron"), |r| r.join("assets/data/territorio/cidades.ron"));
    let mut out = std::collections::HashMap::new();
    let (w, h) = (dados.largura as i64, dados.altura as i64);
    for c in visual_mapa::cenario::carregar_cidades_cenario(&caminho) {
        let (px, py) = gf_mapa::mapa::projetar_miller(c.posicao.0 as f64, c.posicao.1 as f64, dados.largura, dados.altura);
        let (px, py) = (px as i64, py as i64);
        // A cidade pode cair num pixel de mar/borda: procura o pixel de terra mais próximo.
        'busca: for r in 0..=8i64 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let (x, y) = (px + dx, py + dy);
                    if x < 0 || y < 0 || x >= w || y >= h {
                        continue;
                    }
                    let id = dados.ids[(y * w + x) as usize];
                    if id != 0 {
                        *out.entry(id).or_insert(0.0) += c.populacao_urbana as f64;
                        break 'busca;
                    }
                }
            }
        }
    }
    out
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
        2 => ui_refinada::menu::PERIGO,
        1 => ui_refinada::menu::AVISO,
        _ => ui_refinada::menu::INFO,
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
    mut guerra: Option<ResMut<super::guerra_sim::PainelGuerra>>,
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
    let resp = egui::Panel::top("barra_modulos")
        .frame(egui::Frame::NONE.fill(ui_refinada::menu::PAPEL).stroke(egui::Stroke::new(1.0, ui_refinada::menu::VERMELHO)).inner_margin(4.0))
        .show(&mut raiz, |ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(d) = diplo.as_mut() {
                    let r = ui.selectable_label(d.aberto, "Diplomacia (4)");
                    super::tutorial_guiado::marcar(&r, "btn_diplo");
                    if r.clicked() {
                        d.aberto = !d.aberto;
                    }
                }
                if let Some(t) = tec.as_mut() {
                    let r = ui.selectable_label(t.aberto, "Tecnologia (5)");
                    super::tutorial_guiado::marcar(&r, "btn_tec");
                    if r.clicked() {
                        t.aberto = !t.aberto;
                    }
                }
                if let Some(g) = guerra.as_mut()
                    && ui.selectable_label(g.aberto, "Guerra (6)").clicked()
                {
                    g.aberto = !g.aberto;
                }
                ui.separator();
                ui.label(egui::RichText::new("Mapa:").strong().color(ui_refinada::menu::TINTA));
                for m in ModoTematico::TODOS {
                    if ui.selectable_label(*modo == m, m.nome()).clicked() {
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
    consumir_topo(&ctx, resp.response.rect.max.y);
    Ok(())
}

/// Seletor de modo no canto do mapa (canto inferior direito da área livre do mapa).
fn seletor_canto(mut contextos: EguiContexts, mut modo: ResMut<ModoTematico>) -> Result {
    let ctx = contextos.ctx_mut()?;
    let tela = ctx.viewport_rect();
    let livre = area_livre_mapa(ctx);
    let dx = -(tela.max.x - livre.max.x).max(0.0) - 10.0;
    egui::Area::new("seletor_modo_mapa".into()).anchor(egui::Align2::RIGHT_BOTTOM, [dx, -10.0]).show(ctx, |ui| {
        let quadro = egui::Frame::popup(ui.style())
            .fill(ui_refinada::menu::PAPEL)
            .stroke(egui::Stroke::new(1.5, ui_refinada::menu::VERMELHO))
            .inner_margin(6.0)
            .show(ui, |ui| {
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
        super::tutorial_guiado::alvo(ui.ctx(), "modos", quadro.response.rect);
    });
    Ok(())
}

/// Legenda do modo ativo (canto inferior esquerdo; sobe durante o tutorial para não colidir com a caixa).
fn legenda(
    mut contextos: EguiContexts,
    modo: Res<ModoTematico>,
    escala: Res<EscalaAtiva>,
    tutorial: Option<Res<super::tutorial_guiado::TutorialGuiado>>,
) -> Result {
    if matches!(*modo, ModoTematico::Politico) {
        return Ok(());
    }
    let ctx = contextos.ctx_mut()?;
    let em_tutorial = tutorial.as_ref().is_some_and(|t| t.ativo);
    let pos = if em_tutorial { [16.0, -340.0] } else { [10.0, -10.0] };
    egui::Area::new("legenda_mapa".into()).anchor(egui::Align2::LEFT_BOTTOM, pos).show(ctx, |ui| {
        egui::Frame::popup(ui.style())
            .fill(ui_refinada::menu::PAPEL)
            .stroke(egui::Stroke::new(1.5, ui_refinada::menu::VERMELHO))
            .inner_margin(8.0)
            .show(ui, |ui| {
            ui.strong(format!("Mapa: {}", modo.nome()));
            let amostra = |ui: &mut egui::Ui, c: [u8; 4], txt: &str| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(18.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 2.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
                    ui.label(txt);
                });
            };
            match *modo {
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
                _ => {
                    let Some(m) = modo.metrica() else { return };
                    let ativa = escala.modo == Some(*modo) && escala.metrica == Some(m);
                    for k in (0..=4).rev() {
                        let t = k as f32 / 4.0;
                        let txt = if !ativa || !matches!(k, 0 | 2 | 4) {
                            String::new()
                        } else {
                            let e = escala.escala;
                            let v = if e.log { e.min * (e.max / e.min).powf(t as f64) } else { e.min + (e.max - e.min) * t as f64 };
                            let pref = match k {
                                0 => "≤ ",
                                4 => "≥ ",
                                _ => "",
                            };
                            format!("{pref}{}", m.formatar(v))
                        };
                        amostra(ui, rampa(t), &txt);
                    }
                    ui.weak(if m.logaritmica() { "Escala logarítmica, por província" } else { "Escala linear, por província" });
                }
            }
        });
    });
    Ok(())
}

pub struct HudMapasPlugin;

impl Plugin for HudMapasPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ModoTematico>()
            .init_resource::<EscalaAtiva>()
            .add_systems(Update, (teclas_modos, repintar_tematico).chain().run_if(crate::estado::em_partida))
            .add_systems(
                EguiPrimaryContextPass,
                (legenda, seletor_canto)
                    .after(super::ui::painel_provincia)
                    .run_if(crate::estado::em_partida),
            );
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ciclo_de_modos() {
        assert_eq!(ModoTematico::Politico.vizinho(false), ModoTematico::Populacao);
        assert_eq!(ModoTematico::Politico.vizinho(true), ModoTematico::Alinhamento);
        assert_eq!(ModoTematico::Alinhamento.vizinho(false), ModoTematico::Politico);
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
