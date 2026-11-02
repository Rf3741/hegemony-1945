//! Bandeiras dinâmicas: a bandeira, o nome e a cor do país no mapa mudam com o governo e a ideologia
//! (Itália vira república em 1946 etc.). Liga o crate puro `bandeiras_resolver` ao cliente.
//!
//! * `politica_sim` entrega, a cada lote, o estado político de todos os países (`EntradaBandeira`).
//! * Aqui resolvemos a variante, atualizamos nome/cor do país (e o mapa) e damos a textura às telas
//!   (HUD, painel do país, diplomacia) por [`bandeira_ui`].

use super::diplomacia_ui::PainelDiplomacia;
use super::economy::RecursoMundo;
use super::eventos_ui::EstadoEv;
use super::jogador::PaisJogador;
use super::map::{CoresMapa, RaizProjeto};
use super::politica_sim::PoliticaSim;
use super::rotulos_pais::{RotuloPais, RotulosPaisCriados};
use bandeiras_resolver::{EstadoPais as EstadoBandeira, Sobreposicao, Status, VarianteBandeira, catalogo_embutido};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use eventos::Noticia;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

/// Estado político de um país relevante para a aparência (montado pela thread da política).
#[derive(Debug, Clone, Default)]
pub struct EntradaBandeira {
    /// Tag no grafia do legado `simulacao`.
    pub tag: String,
    pub forma_governo: String,
    pub religiao_estatal: String,
    pub ideologia_partido: Option<String>,
    pub ideologia_chefe: Option<String>,
    pub partido: Option<String>,
    pub guerra_civil: bool,
}

struct Interno {
    dir: PathBuf,
    atual: RwLock<HashMap<String, VarianteBandeira>>,
    texturas: Mutex<HashMap<String, Option<egui::TextureHandle>>>,
}

/// Acesso compartilhado às bandeiras (recurso do Bevy e dado do egui).
#[derive(Resource, Clone)]
pub struct Bandeiras(Arc<Interno>);

impl Bandeiras {
    pub fn novo(raiz: &std::path::Path) -> Self {
        Self(Arc::new(Interno { dir: raiz.join("assets/bandeiras"), atual: RwLock::default(), texturas: Mutex::default() }))
    }

    /// Variante atual do país (se já resolvida).
    pub fn variante(&self, tag: &str) -> Option<VarianteBandeira> {
        self.0.atual.read().ok()?.get(tag).cloned()
    }

    /// Textura (PNG 96x64) de um arquivo do catálogo, ex.: `svg/ITA_republica.svg`.
    fn textura(&self, ctx: &egui::Context, arquivo: &str) -> Option<egui::TextureHandle> {
        let nome = arquivo.rsplit('/').next()?.trim_end_matches(".svg").to_string();
        let mut cache = self.0.texturas.lock().ok()?;
        if let Some(t) = cache.get(&nome) {
            return t.clone();
        }
        let t = image::open(self.0.dir.join(format!("{nome}.png"))).ok().map(|img| {
            let img = img.to_rgba8();
            let (w, h) = img.dimensions();
            let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
            ctx.load_texture(format!("bandeira_{nome}"), ci, egui::TextureOptions::LINEAR)
        });
        cache.insert(nome, t.clone());
        t
    }

    /// Desenha a bandeira do país com a altura dada (proporção 3:2). Devolve `false` se não há bandeira.
    pub fn desenhar(&self, ui: &mut egui::Ui, tag: &str, altura: f32) -> bool {
        let Some(v) = self.variante(tag) else { return false };
        let Some(t) = self.textura(ui.ctx(), &v.arquivo) else { return false };
        let tam = egui::vec2(altura * 1.5, altura);
        let r = ui.add(egui::Image::new(egui::load::SizedTexture::new(t.id(), tam)));
        if let (Some(c), true) = (&v.tag_cantao, v.sobreposicao != Sobreposicao::Nenhuma)
            && let Some(vc) = self.variante(c)
            && let Some(tc) = self.textura(ui.ctx(), &vc.arquivo)
        {
            let menor = egui::Rect::from_min_size(r.rect.min, tam * 0.4);
            ui.painter().image(tc.id(), menor, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        r.on_hover_text(&v.nome);
        true
    }
}

/// Desenha a bandeira do país a partir de qualquer tela egui (usa o dado publicado pelo plugin).
pub fn bandeira_ui(ui: &mut egui::Ui, tag: &str, altura: f32) -> bool {
    let b = ui.ctx().data(|d| d.get_temp::<Bandeiras>(egui::Id::new("bandeiras_jogo")));
    b.is_some_and(|b| b.desenhar(ui, tag, altura))
}

/// Nome dinâmico do país (variante atual); `None` se desconhecido.
#[allow(dead_code)]
pub fn nome_dinamico(ui: &egui::Ui, tag: &str) -> Option<String> {
    ui.ctx().data(|d| d.get_temp::<Bandeiras>(egui::Id::new("bandeiras_jogo"))).and_then(|b| b.variante(tag)).map(|v| v.nome)
}

pub struct BandeirasPlugin;

impl Plugin for BandeirasPlugin {
    fn build(&self, app: &mut App) {
        let raiz = app
            .world()
            .get_resource::<RaizProjeto>()
            .map(|r| r.0.clone())
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        app.insert_resource(Bandeiras::novo(&raiz))
            .insert_resource(UltimaVersao(0))
            .add_systems(Update, atualizar_bandeiras.run_if(resource_exists::<PoliticaSim>).run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida))
            .add_systems(EguiPrimaryContextPass, publicar.run_if(crate::estado::em_partida_ou_menu));
    }
}

#[derive(Resource)]
struct UltimaVersao(u64);

fn publicar(mut ctxs: EguiContexts, b: Res<Bandeiras>) -> Result {
    let ctx = ctxs.ctx_mut()?;
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("bandeiras_jogo"), b.clone()));
    Ok(())
}

/// Flags de eventos por data (tag, flag, desde, até) para variantes que dependem da história.
const FLAGS_DATADAS: [(&str, &str, (i32, u8, u8), (i32, u8, u8)); 4] = [
    ("CAN", "bandeira_folha_de_bordo", (1965, 2, 15), (9999, 1, 1)),
    ("LBY", "jamahiriya", (1977, 9, 2), (9999, 1, 1)),
    ("EGY", "uniao_arabe", (1958, 2, 1), (1961, 9, 28)),
    ("SYR", "uniao_arabe", (1958, 2, 1), (1961, 9, 28)),
];

pub fn flags_do_pais(tag: &str, d: (i32, u8, u8)) -> Vec<String> {
    FLAGS_DATADAS.iter().filter(|(t, _, de, ate)| *t == tag && d >= *de && d <= *ate).map(|(_, f, _, _)| f.to_string()).collect()
}

#[allow(clippy::too_many_arguments)]
fn atualizar_bandeiras(
    pol: Res<PoliticaSim>,
    b: Res<Bandeiras>,
    mut mundo: ResMut<RecursoMundo>,
    cores: Option<ResMut<CoresMapa>>,
    painel: Option<ResMut<PainelDiplomacia>>,
    mut ev: ResMut<EstadoEv>,
    jogador: Res<PaisJogador>,
    rotulos: Query<Entity, With<RotuloPais>>,
    mut commands: Commands,
    mut ultima: ResMut<UltimaVersao>,
) {
    if pol.versao_bandeiras == ultima.0 {
        return;
    }
    ultima.0 = pol.versao_bandeiras;
    let cat = catalogo_embutido();
    let d = mundo.0.data;
    let data = (d.ano, d.mes, d.dia);
    let entradas = pol.bandeiras.clone();
    let mut mudou_nome = false;
    let mut repintar: Vec<(String, (u8, u8, u8))> = Vec::new();
    let mut noticias: Vec<String> = Vec::new();
    for e in &entradas {
        let Some(pais) = mundo.0.paises.get(&e.tag) else { continue };
        let st = EstadoBandeira {
            ano: d.ano,
            mes: d.mes,
            forma_governo: e.forma_governo.clone(),
            religiao_estatal: e.religiao_estatal.clone(),
            ideologia_dominante: pais.ideologia.clone(),
            ideologia_partido: e.ideologia_partido.clone(),
            ideologia_chefe: e.ideologia_chefe.clone(),
            partido: e.partido.clone(),
            status: if pais.suserano.is_some() { Status::Colonia } else { Status::Independente },
            suserano: pais.suserano.clone(),
            ocupante: None,
            guerra_civil: e.guerra_civil,
            flags: flags_do_pais(&e.tag, data),
        };
        let nova = cat.resolver_bandeira(&e.tag, &st);
        let antiga = b.0.atual.read().ok().and_then(|m| m.get(&e.tag).cloned());
        if antiga.as_ref().is_some_and(|a| a == &nova) {
            continue;
        }
        if let Some(a) = &antiga
            && (e.tag == jogador.0 || nova.historica)
            && a.id != nova.id
        {
            noticias.push(format!("{} passa a ser {}.", a.nome, nova.nome));
        }
        if let Ok(mut m) = b.0.atual.write() {
            m.insert(e.tag.clone(), nova.clone());
        }
        let Some(pais) = mundo.0.paises.get_mut(&e.tag) else { continue };
        if pais.nome != nova.nome {
            pais.nome = nova.nome.clone();
            mudou_nome = true;
        }
        if pais.cor != nova.cor {
            pais.cor = nova.cor;
            repintar.push((e.tag.clone(), nova.cor));
        }
    }
    for t in noticias {
        ev.estado.jornal.push(Noticia { data, texto: t });
    }
    if let Some(mut c) = cores {
        for (tag, cor) in &repintar {
            for (pid, dono) in &mundo.0.dono_por_provincia {
                if dono == tag
                    && let Some(s) = c.cor_prov.get_mut(*pid as usize)
                {
                    *s = [cor.0, cor.1, cor.2, 255];
                }
            }
        }
    }
    if !repintar.is_empty()
        && let Some(mut p) = painel
    {
        p.repintar = true;
    }
    if mudou_nome {
        for ent in &rotulos {
            commands.entity(ent).despawn();
        }
        commands.remove_resource::<RotulosPaisCriados>();
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn flags_por_data() {
        assert_eq!(flags_do_pais("CAN", (1965, 3, 1)), vec!["bandeira_folha_de_bordo".to_string()]);
        assert!(flags_do_pais("CAN", (1960, 1, 1)).is_empty());
        assert!(flags_do_pais("SYR", (1962, 1, 1)).is_empty());
    }

    #[test]
    fn italia_vira_republica_e_muda_nome_e_cor() {
        let cat = catalogo_embutido();
        let mut e = EstadoBandeira { ano: 1945, mes: 1, forma_governo: "monarquia_constitucional".into(), ideologia_dominante: "democracia_liberal".into(), religiao_estatal: "estado_laico".into(), ..Default::default() };
        let antes = cat.resolver_bandeira("ITA", &e);
        e.ano = 1946;
        e.mes = 6;
        e.forma_governo = "republica_parlamentarista".into();
        let depois = cat.resolver_bandeira("ITA", &e);
        assert_ne!(antes, depois);
        assert_eq!(depois.nome, "República Italiana");
    }

    /// O Afeganistão de 1945 é uma monarquia: a variante base, não a islâmica de 1992.
    #[test]
    fn afeganiste_1945_e_o_reino() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/politica");
        let cenario = sim_politica::CenarioPolitica::carregar_diretorio(&dir).expect("cenário político");
        let pais = cenario.paises.iter().find(|p| p.tag == "AFG").expect("AFG em paises_politica.ron");
        let entrada = EstadoBandeira {
            ano: 1945,
            mes: 1,
            forma_governo: pais.leis_ativas.get("forma_governo").cloned().unwrap_or_default(),
            religiao_estatal: pais.leis_ativas.get("religiao_estatal").cloned().unwrap_or_default(),
            ideologia_dominante: "monarquia_tradicional".into(),
            ideologia_chefe: pais.chefe_de_estado.as_ref().map(|l| l.ideologia_id.clone()),
            ..Default::default()
        };
        let v = catalogo_embutido().resolver_bandeira("AFG", &entrada);
        assert_eq!(v.nome, "Reino do Afeganistão", "ideologia efetiva = {}", entrada.ideologia_efetiva());
    }

    #[test]
    fn pngs_das_variantes_existem() {
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/bandeiras");
        let cat = catalogo_embutido();
        for t in ["ITA", "PAK", "RUS", "USA"] {
            let p = cat.pais(t).expect(t);
            let nome = p.base.arquivo.rsplit('/').next().unwrap().trim_end_matches(".svg");
            assert!(raiz.join(format!("{nome}.png")).exists(), "{nome}");
        }
    }
}
