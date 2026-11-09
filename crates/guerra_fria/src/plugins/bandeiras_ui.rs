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
use bandeiras_resolver::{
    EstadoPais as EstadoBandeira, Sobreposicao, Status, VarianteBandeira, catalogo_embutido,
    flags_do_pais,
};
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
    /// O clero domina o governo (coalizão de governo liderada pelo grupo religioso).
    pub grupo_religioso_no_poder: bool,
    /// O regime difere do que o país tinha quando entrou na partida (ou há golpe/revolução em curso).
    pub mudanca_regime: bool,
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
            .insert_resource(UltimaVersao(0, (0, 0, 0)))
            .add_systems(Update, atualizar_bandeiras.run_if(resource_exists::<PoliticaSim>).run_if(resource_exists::<PaisJogador>).run_if(crate::estado::em_partida))
            .add_systems(EguiPrimaryContextPass, publicar.run_if(crate::estado::em_partida_ou_menu));
    }
}

#[derive(Resource)]
struct UltimaVersao(u64, (i32, u8, u8));

fn publicar(mut ctxs: EguiContexts, b: Res<Bandeiras>) -> Result {
    let ctx = ctxs.ctx_mut()?;
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("bandeiras_jogo"), b.clone()));
    Ok(())
}

/// Ocupações aliadas que dão nome ao país no cenário ("Alemanha Ocupada", "Japão Ocupado",
/// "Áustria Ocupada"): (tag, ocupante principal, de, até). Enquanto valem, o nome mantém a legenda.
const OCUPACOES_DATADAS: &[(&str, &str, (i32, u8, u8), (i32, u8, u8))] = &[
    // Até a Lei Fundamental e a criação da RFA/RDA.
    ("GER", "USA", (1945, 1, 1), (1949, 5, 23)),
    // Até o Tratado de São Francisco.
    ("JAP", "USA", (1945, 1, 1), (1952, 4, 28)),
    // Até o Tratado de Estado austríaco.
    ("AUT", "USA", (1945, 1, 1), (1955, 7, 27)),
];

/// Ocupante histórico do país na data, se ainda ocupado.
pub fn ocupante_historico(tag: &str, d: (i32, u8, u8)) -> Option<&'static str> {
    OCUPACOES_DATADAS.iter().find(|(t, _, de, ate)| *t == tag && d >= *de && d <= *ate).map(|(_, o, _, _)| *o)
}

/// Doze países têm outra grafia no catálogo de bandeiras que no legado `simulacao` (BGR→BUL,
/// HTI→HAI, THA→SIA...). Sem o alias o resolvedor não acha o país e devolve a tag crua como nome.
fn tag_do_catalogo(tag: &str) -> &str {
    super::economia_sim::ALIAS_TAGS
        .iter()
        .find(|(legado, _)| *legado == tag)
        .map_or(tag, |(_, cat)| *cat)
}

/// Estado de aparência de um país, tal como o resolvedor de bandeiras o recebe: a política
/// (`EntradaBandeira`) mais o que o cenário sabe (ideologia, suserano e nome histórico). A
/// ocupação militar não entra no nome: ela aparece pela hachura e pela legenda do mapa (`plugins::map`).
pub fn estado_de_aparencia(
    tag: &str,
    ideologia: &str,
    suserano: Option<&str>,
    nome_cenario: Option<&str>,
    e: &EntradaBandeira,
    data: (i32, u8, u8),
) -> EstadoBandeira {
    EstadoBandeira {
        ano: data.0,
        mes: data.1,
        forma_governo: e.forma_governo.clone(),
        religiao_estatal: e.religiao_estatal.clone(),
        ideologia_dominante: ideologia.to_string(),
        ideologia_partido: e.ideologia_partido.clone(),
        ideologia_chefe: e.ideologia_chefe.clone(),
        partido: e.partido.clone(),
        status: if suserano.is_some() { Status::Colonia } else { Status::Independente },
        suserano: suserano.map(str::to_string),
        ocupante: ocupante_historico(tag, data).map(str::to_string),
        guerra_civil: e.guerra_civil,
        grupo_religioso_no_poder: e.grupo_religioso_no_poder,
        mudanca_regime: e.mudanca_regime,
        nome_cenario: nome_cenario.unwrap_or("").to_string(),
        flags: {
            let mut f = flags_do_pais(tag, data);
            if tag == "SOV" && (ideologia.contains("comunista") || ideologia.contains("socialista")) && !e.mudanca_regime {
                f.retain(|flag| flag != "rus_federacao");
            }
            f
        },
    }
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
    let d = mundo.0.data;
    let data = (d.ano, d.mes, d.dia);
    if pol.versao_bandeiras == ultima.0 && ultima.1 == data {
        return;
    }
    ultima.0 = pol.versao_bandeiras;
    ultima.1 = data;
    let cat = catalogo_embutido();
    let entradas = pol.bandeiras.clone();
    let mut mudou_nome = false;
    let mut repintar: Vec<(String, (u8, u8, u8))> = Vec::new();
    let mut noticias: Vec<String> = Vec::new();
    for e in &entradas {
        let Some(pais) = mundo.0.paises.get(&e.tag) else { continue };
        let st = estado_de_aparencia(&e.tag, &pais.ideologia, pais.suserano.as_deref(), pais.nome_cenario.as_deref(), e, data);
        let nova = cat.resolver_bandeira(tag_do_catalogo(&e.tag), &st);
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

    use super::super::politica_sim::NucleoPol;
    use simulacao::formacao::Formacoes;
    use simulacao::mundo::MundoJogo;
    use simulacao::tempo::DataSimulacao;
    use std::collections::HashMap;

    /// Sufixo que só poderia aparecer com uma ocupação militar em curso.
    const SUFIXO_OCUPACAO: &str = "(Sob Ocupação";

    /// Cenário real de 1945: mundo do núcleo + política + formáveis/desocupações.
    struct Cenario {
        mundo: MundoJogo,
        entradas: HashMap<String, EntradaBandeira>,
        form: Formacoes,
    }

    fn raiz_projeto() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn cenario_1945() -> Cenario {
        let raiz = raiz_projeto();
        let (catalogo, mundo) = MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenário 1945");
        let (mut pol, _) = NucleoPol::carregar(&raiz.join("assets/data/politica")).expect("política de 1945");
        let entradas = pol.entradas_bandeira().into_iter().map(|e| (e.tag.clone(), e)).collect();
        let form = Formacoes::carregar(raiz.join("data/formaveis"), &catalogo).expect("formáveis de 1945");
        Cenario { mundo, entradas, form }
    }

    /// Nome exibido (variante vencedora) e se há ocupação ativa sobre cada país na data dada.
    fn nomes_em(cen: &Cenario, data: (i32, u8, u8)) -> Vec<(String, String, bool)> {
        nomes_com_politica(&cen.mundo, &cen.entradas, data)
    }

    /// Nomes exibidos por país, com as entradas políticas dadas (permite rodar a simulação antes).
    fn nomes_com_politica(mundo: &MundoJogo, entradas: &HashMap<String, EntradaBandeira>, data: (i32, u8, u8)) -> Vec<(String, String, bool)> {
        let cat = catalogo_embutido();
        let mut ocupadas: Vec<&String> = Vec::new();
        for (id, dono) in &mundo.dono_por_provincia {
            if mundo.controlador_por_provincia.get(id).is_some_and(|c| c != dono) {
                ocupadas.push(dono);
            }
        }
        mundo
            .paises
            .iter()
            .map(|(tag, pais)| {
                let e = entradas.get(tag).cloned().unwrap_or_default();
                let st = estado_de_aparencia(tag, &pais.ideologia, pais.suserano.as_deref(), pais.nome_cenario.as_deref(), &e, data);
                (tag.clone(), cat.resolver_bandeira(tag_do_catalogo(tag), &st).nome, ocupadas.contains(&tag))
            })
            .collect()
    }

    /// Roda desocupações e formações até a data (o que o cliente faz dia a dia).
    fn avancar_ate(cen: &mut Cenario, data: (i32, u8, u8)) {
        cen.mundo.data = DataSimulacao::nova(data.0, data.1, data.2);
        cen.form.processar(&mut cen.mundo);
    }

    /// O nome nunca traz a ocupação: ela aparece pela hachura e pela legenda do mapa.
    #[test]
    fn catalogo_de_nomes_nao_tem_sufixo_de_ocupacao() {
        let cat = catalogo_embutido();
        for p in &cat.dados().paises {
            for v in std::iter::once(&p.base).chain(&p.variantes) {
                assert!(!v.nome.contains(SUFIXO_OCUPACAO), "{} / {}: {:?}", p.tag, v.id, v.nome);
            }
        }
    }

    /// Em 01/01/1946 e 01/06/1946 só quem ainda está sob ocupação histórica (Alemanha, Japão,
    /// Áustria; ver OCUPACOES_DATADAS) pode mostrar a ocupação no nome; os libertados, não.
    #[test]
    fn nenhum_nome_de_pais_tem_ocupacao_em_1946() {
        let mut cen = cenario_1945();
        for data in [(1946, 1, 1), (1946, 6, 1)] {
            avancar_ate(&mut cen, data);
            for (tag, nome, ocupada) in nomes_em(&cen, data) {
                if ocupante_historico(&tag, data).is_some() {
                    continue;
                }
                assert!(!nome.contains(SUFIXO_OCUPACAO), "{tag} (ocupação ativa: {ocupada}): o nome traz {SUFIXO_OCUPACAO:?} -> {nome}");
                assert!(!nome.contains("Ocupad"), "{tag}: o nome traz a ocupação, mas ela já acabou -> {nome}");
            }
        }
        // Irã e Dinamarca: libertados, nome sem sufixo.
        let nomes: HashMap<String, String> = nomes_em(&cen, (1946, 6, 1)).into_iter().map(|(t, n, _)| (t, n)).collect();
        assert_eq!(nomes.get("IRN").map(String::as_str), Some("Império do Irã"));
        assert_eq!(nomes.get("DEN").map(String::as_str), Some("Reino da Dinamarca"));
    }

    /// Toda entrada política tem variante no catálogo (pela tag do cenário ou pelo alias): sem isso
    /// o resolvedor devolve a tag crua e o mapa escreve "BGR", "HTI", "THA" no lugar do nome.
    #[test]
    fn toda_entrada_politica_tem_variante_no_catalogo() {
        let cen = cenario_1945();
        let cat = catalogo_embutido();
        let sem_flag: Vec<String> = cen
            .entradas
            .keys()
            .filter(|t| cat.pais(tag_do_catalogo(t)).is_none())
            .cloned()
            .collect();
        assert!(sem_flag.is_empty(), "países sem catálogo de bandeiras (o nome vira a tag crua): {sem_flag:?}");
    }

    /// Nomes de `data/scenario_1945/countries_1945.ron`, lidos direto do arquivo (a referência do teste).
    fn nomes_do_cenario() -> HashMap<String, String> {
        let texto = std::fs::read_to_string(raiz_projeto().join("data/scenario_1945/countries_1945.ron")).expect("countries_1945.ron");
        let defs: Vec<simulacao::mapa::DefinicaoPais1945> = ron::from_str(&texto).expect("countries_1945.ron válido");
        defs.into_iter().map(|d| (d.tag, d.nome)).collect()
    }

    /// Em 01/01/1945 nenhum país mudou de regime, então todos mostram o nome histórico do cenário:
    /// as regras genéricas de ideologia e os nomes religiosos não podem renomear nada aqui. Os
    /// ocupados (Alemanha, Japão, Áustria) mostram o nome do cenário com a legenda de ocupação.
    #[test]
    fn em_1945_todos_os_paises_mostram_o_nome_do_cenario() {
        let cen = cenario_1945();
        let cenario = nomes_do_cenario();
        let nomes = nomes_em(&cen, (1945, 1, 1));
        let ruins: Vec<String> = nomes
            .iter()
            .filter_map(|(tag, nome, _)| {
                let do_cenario = cenario.get(tag)?;
                let esperado = if ocupante_historico(tag, (1945, 1, 1)).is_some() {
                    do_cenario.clone()
                } else {
                    bandeiras_resolver::sem_ocupacao(do_cenario)
                };
                if *nome == esperado { None } else { Some(format!("{tag}: cenário={esperado:?} exibido={nome:?}")) }
            })
            .collect();
        assert!(ruins.is_empty(), "{} país(es) com nome diferente do cenário em 01/01/1945:\n{}", ruins.len(), ruins.join("\n"));
        assert_eq!(nomes.len(), cenario.len(), "o teste cobriu todos os países do cenário");
    }

    /// Dois anos de simulação: Japão, Portugal, Bolívia, França e Uruguai não podem virar
    /// "Estado Teocrático" sem a lei de religião de estado obrigatória com o clero no governo.
    #[test]
    fn dois_anos_sem_teorico_sem_lei_religiosa() {
        let raiz = raiz_projeto();
        let (_catalogo, mundo) = MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenário 1945");
        let (mut pol, _) = NucleoPol::carregar(&raiz.join("assets/data/politica")).expect("política de 1945");
        let observados = ["JAP", "POR", "BOL", "FRA", "URY"];
        let mut ruins: Vec<String> = Vec::new();
        for _ in 0..(2 * 12) {
            let _ = pol.avancar("USA", 30);
            let entradas: HashMap<String, EntradaBandeira> = pol.entradas_bandeira().into_iter().map(|e| (e.tag.clone(), e)).collect();
            for (tag, nome, _) in nomes_com_politica(&mundo, &entradas, (1945, 1, 1)) {
                if !observados.contains(&tag.as_str()) || !nome.contains("Teocrático") {
                    continue;
                }
                let e = entradas.get(&tag).cloned().unwrap_or_default();
                ruins.push(format!("{tag}: {nome} com lei de religião {:?} e clero no governo: {}", e.religiao_estatal, e.grupo_religioso_no_poder));
            }
        }
        assert!(ruins.is_empty(), "\"Estado Teocrático\" sem a lei de religião de estado + clero no governo:\n{}", ruins.join("\n"));
    }

    /// Nomes de 01/01/1945: enquanto nada mudou, vale o nome do cenário (é dele que o mapa parte);
    /// os títulos do catálogo ("Reino da Noruega") entram quando o regime do país muda.
    #[test]
    fn nomes_oficiais_dos_governos_de_1945() {
        let cen = cenario_1945();
        let nomes: HashMap<String, String> = nomes_em(&cen, (1945, 1, 1)).into_iter().map(|(t, n, _)| (t, n)).collect();
        assert_eq!(nomes.get("CHI").map(String::as_str), Some("República da China"));
        assert_eq!(nomes.get("BRA").map(String::as_str), Some("Estados Unidos do Brasil"));
        assert_eq!(nomes.get("DEN").map(String::as_str), Some("Reino da Dinamarca"));
        assert_eq!(nomes.get("IRN").map(String::as_str), Some("Império do Irã"));
        assert_eq!(nomes.get("NOR").map(String::as_str), Some("Noruega"));
        assert_eq!(nomes.get("SOV").map(String::as_str), Some("União Soviética"));
        assert_eq!(nomes.get("FRA").map(String::as_str), Some("França (Governo Provisório)"));
        let juntas: Vec<&String> = nomes.iter().filter(|(_, n)| n.contains("Junta Militar")).map(|(_, n)| n).collect();
        assert!(juntas.is_empty(), "governos de 1945 escritos como junta militar: {juntas:?}");
    }
}
