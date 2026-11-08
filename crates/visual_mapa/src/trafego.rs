//! # Tráfego vivo: navios mercantes e aviões
//!
//! ## O que faz
//! - **Navios**: um navio por "unidade de volume" de cada fluxo comercial ([`FluxosComercio`]:
//!   exportações/importações entre mercados, vindas de `rotas_comerciais.ron` do `sim_economia`).
//!   Mais volume = mais navios na rota. Tipo pelo bem ([`TipoNavio`]: cargueiro, petroleiro,
//!   graneleiro) e faixa de bandeira na cor do país operador. As rotas vêm do [`Roteador`]
//!   (A* na máscara terra/mar de `provinces.png`, com os canais Suez, Panamá, Kiel, Corinto,
//!   Gibraltar e Bósforo como portais): navio nunca atravessa terra.
//! - **Aviões**: entre as maiores cidades; hélice em 1945, a jato a partir de 1958; a quantidade
//!   cresce com o [`AnoVisual`] (ver [`avioes_ativos_no_ano`] e [`fracao_jato_no_ano`]).
//! - **LOD**: Global (longe) mostra 1/4 dos navios e nenhum avião; Regional metade; Local tudo.
//!   Os ícones crescem com o zoom para continuarem legíveis. A tecla V
//!   ([`CamadaConstrucoesVisivel`]) oculta tudo. Limites em [`ConfigTrafego`].
//!
//! ## API para a economia (pedágio de canais)
//! O recurso [`RelatorioCanais`] acumula, por canal ([`Canal`]), a contagem de passagens e o
//! volume transportado (a viagem de volta segue em lastro: conta passagem, volume 0).
//! ```ignore
//! fn pedagio(mut rel: ResMut<RelatorioCanais>) {
//!     let suez = rel.canal(Canal::Suez);            // PassagensCanal { contagem, volume }
//!     for ev in rel.tomar_pendentes() {              // eventos desde a última chamada
//!         // ev.canal, ev.volume, ev.operador (tag do país, minúsculas, ex.: "gbr")
//!     }
//!     let por_pais = rel.do_operador(Canal::Suez, "gbr");
//! }
//! ```
//! Para alimentar com o comércio vivo (em vez do arquivo), sobrescreva [`FluxosComercio`]
//! (`ResMut<FluxosComercio>`, `fluxos = ...`): o plano de navios é recalculado em segundo plano.
//! A população de cidades/portos vem de [`EntradaVisual`]/[`EntradaEconomiaVisual`] (ou, sem
//! elas, de `assets/data/territorio/cidades.ron`).

use crate::cenario::{CIDADES_TERRITORIO_PADRAO, carregar_cidades_cenario};
use crate::config::{CamadaConstrucoesVisivel, VisualMapaConfig};
use crate::entrada::{AnoVisual, EntradaEconomiaVisual, EntradaVisual};
use crate::lod::{EstadoLod, NivelLod};
use crate::projecao::ProjecaoMapa;
pub use crate::rotas_maritimas::Canal;
use crate::rotas_maritimas::{MASCARA_PADRAO, Rota, Roteador};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, mpsc};

// ---------------------------------------------------------------------------------------------
// Configuração e entradas
// ---------------------------------------------------------------------------------------------

/// Configuração do tráfego (insira antes do plugin para sobrescrever).
#[derive(Resource, Debug, Clone)]
pub struct ConfigTrafego {
    pub ativo: bool,
    pub max_navios: usize,
    pub max_avioes: usize,
    /// Um navio por esta quantidade de volume de fluxo (mín. 1, máx. `max_navios_por_fluxo`).
    pub volume_por_navio: f32,
    pub max_navios_por_fluxo: usize,
    /// Velocidade dos navios (unidades de mundo por segundo).
    pub velocidade_navio: f32,
    pub velocidade_helice: f32,
    pub velocidade_jato: f32,
    /// Tamanho base dos ícones (multiplicador).
    pub escala_icones: f32,
    pub caminho_mascara: PathBuf,
    /// Pasta com `rotas_comerciais.ron`, `mercados.ron`.
    pub dir_economia: PathBuf,
    /// Quantas cidades (por população) servem de aeroportos.
    pub cidades_aeroporto: usize,
}

impl Default for ConfigTrafego {
    fn default() -> Self {
        Self {
            ativo: true,
            max_navios: 400,
            max_avioes: 160,
            volume_por_navio: 40.0,
            max_navios_por_fluxo: 8,
            velocidade_navio: 26.0,
            velocidade_helice: 70.0,
            velocidade_jato: 150.0,
            escala_icones: 1.0,
            caminho_mascara: PathBuf::from(MASCARA_PADRAO),
            dir_economia: PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data/economia")),
            cidades_aeroporto: 60,
        }
    }
}

/// Um fluxo de comércio exportador -> importador de um bem.
#[derive(Debug, Clone, PartialEq)]
pub struct FluxoComercio {
    /// País exportador (tag em minúsculas, ex.: "usa").
    pub origem: String,
    pub destino: String,
    pub bem: String,
    pub volume: f32,
    /// País que opera a rota (cor da bandeira do navio).
    pub operador: String,
}

/// Fluxos comerciais a desenhar. Padrão: lido de `rotas_comerciais.ron` na inicialização.
#[derive(Resource, Debug, Clone, Default)]
pub struct FluxosComercio {
    pub fluxos: Vec<FluxoComercio>,
}

#[derive(Deserialize)]
struct RotaRon {
    mercado_origem_id: String,
    mercado_destino_id: String,
    #[serde(default)]
    pais_operador_id: String,
    bem_id: String,
    volume_atual: f64,
}

#[derive(Deserialize)]
struct MercadoRon {
    id: String,
    pais_lider_id: String,
}

impl FluxosComercio {
    /// Lê `rotas_comerciais.ron` + `mercados.ron` (mercado -> país líder).
    pub fn carregar_diretorio(dir: &Path) -> Self {
        let rotas: Vec<RotaRon> = std::fs::read_to_string(dir.join("rotas_comerciais.ron"))
            .ok()
            .and_then(|t| ron::from_str(&t).ok())
            .unwrap_or_default();
        let mercados: Vec<MercadoRon> = std::fs::read_to_string(dir.join("mercados.ron"))
            .ok()
            .and_then(|t| ron::from_str(&t).ok())
            .unwrap_or_default();
        let lider: HashMap<&str, &str> =
            mercados.iter().map(|m| (m.id.as_str(), m.pais_lider_id.as_str())).collect();
        let pais_de = |m: &str| -> String {
            lider.get(m).map(|s| s.to_string()).unwrap_or_else(|| m.trim_start_matches("mercado_").to_string())
        };
        let fluxos = rotas
            .iter()
            .map(|r| FluxoComercio {
                origem: pais_de(&r.mercado_origem_id),
                destino: pais_de(&r.mercado_destino_id),
                bem: r.bem_id.clone(),
                volume: r.volume_atual as f32,
                operador: if r.pais_operador_id.is_empty() { pais_de(&r.mercado_origem_id) } else { r.pais_operador_id.clone() },
            })
            .collect();
        Self { fluxos }
    }
}

// ---------------------------------------------------------------------------------------------
// Relatório de passagens por canal
// ---------------------------------------------------------------------------------------------

/// Acumulado de um canal.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PassagensCanal {
    pub contagem: u32,
    pub volume: f32,
}

/// Uma passagem individual (para pedágio).
#[derive(Debug, Clone, PartialEq)]
pub struct EventoPassagem {
    pub canal: Canal,
    pub volume: f32,
    /// Tag do país operador (minúsculas).
    pub operador: String,
}

/// Relatório de passagens por canal. Consumido pela economia (pedágio).
#[derive(Resource, Debug, Clone, Default)]
pub struct RelatorioCanais {
    total: BTreeMap<Canal, PassagensCanal>,
    por_operador: BTreeMap<(Canal, String), PassagensCanal>,
    pendentes: Vec<EventoPassagem>,
}

impl RelatorioCanais {
    const MAX_PENDENTES: usize = 20_000;

    pub fn registrar(&mut self, canal: Canal, volume: f32, operador: &str) {
        let t = self.total.entry(canal).or_default();
        t.contagem += 1;
        t.volume += volume;
        let o = self.por_operador.entry((canal, operador.to_string())).or_default();
        o.contagem += 1;
        o.volume += volume;
        if self.pendentes.len() < Self::MAX_PENDENTES {
            self.pendentes.push(EventoPassagem { canal, volume, operador: operador.to_string() });
        }
    }

    /// Acumulado total do canal desde o início (ou desde [`Self::zerar`]).
    pub fn canal(&self, canal: Canal) -> PassagensCanal {
        self.total.get(&canal).copied().unwrap_or_default()
    }

    /// Acumulado do canal para um operador (tag em minúsculas).
    pub fn do_operador(&self, canal: Canal, operador: &str) -> PassagensCanal {
        self.por_operador.get(&(canal, operador.to_string())).copied().unwrap_or_default()
    }

    /// Todos os canais com seus acumulados (inclui zeros), em ordem fixa.
    pub fn todos(&self) -> Vec<(Canal, PassagensCanal)> {
        Canal::TODOS.iter().map(|&c| (c, self.canal(c))).collect()
    }

    /// Entrega e limpa os eventos ocorridos desde a última chamada.
    pub fn tomar_pendentes(&mut self) -> Vec<EventoPassagem> {
        std::mem::take(&mut self.pendentes)
    }

    pub fn zerar(&mut self) {
        *self = Self::default();
    }
}

// ---------------------------------------------------------------------------------------------
// Tipos e cores
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoNavio {
    Cargueiro,
    Petroleiro,
    Graneleiro,
}

/// Tipo de navio pelo bem transportado.
pub fn tipo_navio_por_bem(bem: &str) -> TipoNavio {
    match bem {
        "petroleo" | "combustiveis_refinados" | "gas_natural" | "produtos_quimicos" => TipoNavio::Petroleiro,
        "graos" | "carvao" | "minerio_ferro" | "bauxita" | "cobre" | "chumbo_zinco" | "madeira" | "acucar"
        | "material_construcao" | "algodao" | "la" => TipoNavio::Graneleiro,
        _ => TipoNavio::Cargueiro,
    }
}

/// Cor da bandeira (aproximada) do país, por tag em qualquer caixa; hash estável como reserva.
pub fn cor_pais(tag: &str) -> Color {
    let t = tag.to_ascii_lowercase();
    let (r, g, b) = match t.as_str() {
        "usa" => (0.15, 0.30, 0.75),
        "gbr" => (0.75, 0.10, 0.15),
        "sov" => (0.80, 0.05, 0.05),
        "fra" => (0.20, 0.30, 0.80),
        "ger" => (0.10, 0.10, 0.10),
        "jap" => (0.95, 0.95, 0.95),
        "bra" => (0.10, 0.60, 0.20),
        "arg" => (0.45, 0.70, 0.95),
        "ven" => (0.95, 0.80, 0.10),
        "irn" => (0.15, 0.60, 0.30),
        "sau" => (0.05, 0.50, 0.20),
        "chl" => (0.80, 0.15, 0.15),
        "swe" => (0.95, 0.80, 0.10),
        "mex" => (0.05, 0.50, 0.25),
        "chi" | "ccp" => (0.85, 0.15, 0.10),
        "ita" => (0.20, 0.65, 0.30),
        "can" => (0.90, 0.10, 0.10),
        _ => {
            let mut h: u32 = 2166136261;
            for b in t.bytes() {
                h = (h ^ b as u32).wrapping_mul(16777619);
            }
            return Color::hsl((h % 360) as f32, 0.65, 0.5);
        }
    };
    Color::srgb(r, g, b)
}

/// Aviões ativos no ano (cresce com as eras: ~20 em 1945 até o máximo por volta de 1990).
pub fn avioes_ativos_no_ano(ano: f32, max: usize) -> usize {
    let t = ((ano - 1945.0) / 45.0).clamp(0.0, 1.0);
    let n = 20.0 + (max as f32 - 20.0) * t.powf(0.8);
    (n.round() as usize).min(max)
}

/// Fração da frota aérea que é a jato: 0 antes de 1958, 90% em 1972.
pub fn fracao_jato_no_ano(ano: f32) -> f32 {
    if ano < 1958.0 { 0.0 } else { (0.05 + 0.85 * ((ano - 1958.0) / 14.0)).clamp(0.0, 0.9) }
}

fn rng(estado: &mut u64) -> f32 {
    let mut x = *estado;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *estado = x;
    ((x >> 40) as f32) / (1u64 << 24) as f32
}

// ---------------------------------------------------------------------------------------------
// Cidades de interesse (portos e aeroportos)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CidadeTrafego {
    pub id: String,
    pub pais: String, // minúsculas
    pub mundo: Vec2,
    pub populacao: u64,
    pub porto: bool,
}

/// Cidades (portos/aeroportos) conhecidas pelo tráfego.
#[derive(Resource, Debug, Clone, Default)]
pub struct CidadesTrafego {
    pub cidades: Vec<CidadeTrafego>,
    versao: u32,
}

fn coletar_cidades_sistema(
    cfg: Res<VisualMapaConfig>,
    vis: Res<EntradaVisual>,
    eco: Res<EntradaEconomiaVisual>,
    time: Res<Time>,
    mut cidades: ResMut<CidadesTrafego>,
    mut tentou_fallback: Local<bool>,
) {
    let proj: ProjecaoMapa = cfg.projecao;
    let mut mapa: HashMap<String, CidadeTrafego> = HashMap::new();
    for c in &vis.cidades {
        mapa.insert(
            c.id.clone(),
            CidadeTrafego {
                id: c.id.clone(),
                pais: c.pais_tag.to_ascii_lowercase(),
                mundo: proj.geo_para_mundo(c.posicao.0, c.posicao.1),
                populacao: c.populacao as u64,
                porto: c.eh_porto,
            },
        );
    }
    for c in &eco.cidades {
        mapa.entry(c.id.clone()).or_insert_with(|| CidadeTrafego {
            id: c.id.clone(),
            pais: c.pais_tag.to_ascii_lowercase(),
            mundo: proj.geo_para_mundo(c.posicao.0, c.posicao.1),
            populacao: c.populacao_urbana,
            porto: c.eh_porto,
        });
    }
    // Sem entradas depois de 2 s: lê as cidades reais diretamente.
    if mapa.is_empty() && !*tentou_fallback && time.elapsed_secs() > 2.0 {
        *tentou_fallback = true;
        for c in carregar_cidades_cenario(Path::new(CIDADES_TERRITORIO_PADRAO)) {
            mapa.insert(
                c.id.clone(),
                CidadeTrafego {
                    id: c.id,
                    pais: c.pais_tag.to_ascii_lowercase(),
                    mundo: proj.geo_para_mundo(c.posicao.0, c.posicao.1),
                    populacao: c.populacao_urbana,
                    porto: c.eh_porto,
                },
            );
        }
    }
    if mapa.is_empty() {
        return;
    }
    let mut v: Vec<CidadeTrafego> = mapa.into_values().collect();
    v.sort_by(|a, b| b.populacao.cmp(&a.populacao).then(a.id.cmp(&b.id)));
    cidades.cidades = v;
    cidades.versao += 1;
}

// ---------------------------------------------------------------------------------------------
// Plano de rotas (em segundo plano)
// ---------------------------------------------------------------------------------------------

/// Rota planejada para um fluxo.
#[derive(Clone)]
pub struct RotaNavios {
    pub rota: Arc<Rota>,
    pub tipo: TipoNavio,
    pub operador: String,
    pub origem: String,
    pub destino: String,
    pub bem: String,
    pub navios: usize,
    pub carga: f32,
}

static ROTEADOR: OnceLock<Option<Arc<Roteador>>> = OnceLock::new();

fn roteador(caminho: &Path) -> Option<Arc<Roteador>> {
    ROTEADOR
        .get_or_init(|| match Roteador::carregar(caminho) {
            Ok(r) => Some(Arc::new(r)),
            Err(e) => {
                warn!("trafego: máscara indisponível ({e}); sem navios.");
                None
            }
        })
        .clone()
}

/// Calcula as rotas (A*) de todos os fluxos. Pesado: rode fora da thread principal.
pub fn calcular_plano(
    caminho_mascara: &Path,
    cidades: &[CidadeTrafego],
    fluxos: &[FluxoComercio],
    volume_por_navio: f32,
    max_por_fluxo: usize,
    max_total: usize,
) -> Vec<RotaNavios> {
    calcular_plano_com(caminho_mascara, cidades, fluxos, volume_por_navio, max_por_fluxo, max_total, &[])
}

/// Canais fechados na data atual (o cliente preenche a partir de `sim_economia::canal_disponivel`).
/// Quando muda, o plano de rotas é refeito e os navios contornam (Suez fechado -> Cabo da Boa Esperança).
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct CanaisFechados(pub Vec<Canal>);

/// Como [`calcular_plano`], evitando os canais `fechados`.
pub fn calcular_plano_com(
    caminho_mascara: &Path,
    cidades: &[CidadeTrafego],
    fluxos: &[FluxoComercio],
    volume_por_navio: f32,
    max_por_fluxo: usize,
    max_total: usize,
    fechados: &[Canal],
) -> Vec<RotaNavios> {
    let Some(r) = roteador(caminho_mascara) else { return Vec::new() };
    // portos ancorados por país (top 4 por população)
    let mut portos: HashMap<&str, Vec<(u32, Vec2)>> = HashMap::new();
    for c in cidades.iter().filter(|c| c.porto) {
        let v = portos.entry(c.pais.as_str()).or_default();
        if v.len() < 4 {
            if let Some(cel) = r.ancorar(c.mundo, 80) {
                v.push((cel, r.celula_para_mundo(cel)));
            }
        }
    }
    let mut cache: HashMap<(u32, u32), Option<Arc<Rota>>> = HashMap::new();
    let mut plano = Vec::new();
    for f in fluxos {
        let (Some(po), Some(pd)) = (portos.get(f.origem.as_str()), portos.get(f.destino.as_str())) else { continue };
        let mut pares: Vec<(f32, u32, u32)> = po
            .iter()
            .flat_map(|a| pd.iter().map(move |b| (a.1.distance(b.1), a.0, b.0)))
            .filter(|p| p.1 != p.2)
            .collect();
        pares.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut melhor: Option<(f32, Arc<Rota>)> = None;
        for &(_, a, b) in pares.iter().take(3) {
            let rota = cache
                .entry((a, b))
                .or_insert_with(|| r.caminho_sem(a, b, fechados).map(|p| Arc::new(p.rota)))
                .clone();
            if let Some(rota) = rota {
                let c = rota.comprimento();
                if melhor.as_ref().is_none_or(|(bc, _)| c < *bc) {
                    melhor = Some((c, rota));
                }
            }
        }
        let Some((_, rota)) = melhor else { continue };
        let navios = ((f.volume / volume_por_navio).ceil() as usize).clamp(1, max_por_fluxo.max(1));
        plano.push(RotaNavios {
            rota,
            tipo: tipo_navio_por_bem(&f.bem),
            operador: f.operador.clone(),
            origem: f.origem.clone(),
            destino: f.destino.clone(),
            bem: f.bem.clone(),
            navios,
            carga: f.volume / navios as f32,
        });
    }
    // respeita o teto global reduzindo proporcionalmente
    let total: usize = plano.iter().map(|p| p.navios).sum();
    if total > max_total && total > 0 {
        let k = max_total as f32 / total as f32;
        for p in &mut plano {
            let novo = ((p.navios as f32 * k).round() as usize).max(1);
            p.carga *= p.navios as f32 / novo as f32;
            p.navios = novo;
        }
    }
    plano
}

#[derive(Resource, Default)]
struct EstadoPlano {
    rx: Option<Mutex<mpsc::Receiver<Vec<RotaNavios>>>>,
    versao_cidades: u32,
    refazer: bool,
    /// Plano atual (para consulta/diagnóstico).
    atual: Vec<RotaNavios>,
}

impl EstadoPlano {
    /// Rotas atualmente em uso (diagnóstico).
    pub fn rotas(&self) -> &[RotaNavios] {
        &self.atual
    }
}

// ---------------------------------------------------------------------------------------------
// Componentes e malhas
// ---------------------------------------------------------------------------------------------

#[derive(Component)]
pub struct NavioVisual {
    pub rota: Arc<Rota>,
    pub tipo: TipoNavio,
    pub operador: String,
    pub dist: f32,
    pub sentido: f32,
    pub espera: f32,
    pub lateral: f32,
    pub vel: f32,
    pub idx: u32,
    pub carga: f32,
    angulo: f32,
}

#[derive(Component)]
pub struct AviaoVisual {
    pub ordem: u32,
    pub origem: Vec2,
    pub destino: Vec2,
    pub t: f32,
    pub vel: f32,
    pub jato: bool,
    seed: u64,
    angulo: f32,
}

#[derive(Component)]
struct Esteira;
#[derive(Component)]
struct Sombra;

#[derive(Resource)]
struct RecursosTrafego {
    cascos: [Handle<Mesh>; 3],
    aviao_helice: Handle<Mesh>,
    aviao_jato: Handle<Mesh>,
    mat_navio: [Handle<ColorMaterial>; 3],
    mat_vertice: Handle<ColorMaterial>,
    mat_helice: Handle<ColorMaterial>,
    mat_jato: Handle<ColorMaterial>,
}

/// Malha de polígonos convexos (cada um em leque).
fn malha_poligonos(polis: &[&[[f32; 2]]]) -> Mesh {
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    for p in polis {
        let base = pos.len() as u32;
        pos.extend(p.iter().map(|v| [v[0], v[1], 0.0]));
        for i in 1..(p.len() as u32 - 1) {
            idx.extend([base, base + i, base + i + 1]);
        }
    }
    let n = pos.len();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n])
        .with_inserted_indices(Indices::U32(idx))
}

/// Malha com cor por vértice: cada peça é (polígono convexo, cor); camadas posteriores ficam por cima.
fn malha_colorida(pecas: &[(&[[f32; 2]], [f32; 4])]) -> Mesh {
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut cores: Vec<[f32; 4]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    for (camada, (p, cor)) in pecas.iter().enumerate() {
        let base = pos.len() as u32;
        pos.extend(p.iter().map(|v| [v[0], v[1], 0.02 * camada as f32]));
        cores.extend(std::iter::repeat(*cor).take(p.len()));
        for i in 1..(p.len() as u32 - 1) {
            idx.extend([base, base + i, base + i + 1]);
        }
    }
    let n = pos.len();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n])
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, cores)
        .with_inserted_indices(Indices::U32(idx))
}

fn caixa(x0: f32, x1: f32, y0: f32, y1: f32) -> [[f32; 2]; 4] {
    [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

/// Silhuetas de navio com comprimento 1 (proa em +x): cargueiro, petroleiro, graneleiro.
fn malhas_navios() -> [Mesh; 3] {
    let ponte = [0.90, 0.90, 0.86, 1.0];
    let chamine = [0.75, 0.22, 0.18, 1.0];
    // Cargueiro: casco 3,3:1, ponte e chaminé à ré, contêineres coloridos à vante.
    let h = [
        [-0.5, -0.15], [0.28, -0.15], [0.5, 0.0], [0.28, 0.15], [-0.5, 0.15],
    ];
    let (b1, b2, b3, b4) = (caixa(-0.12, 0.0, -0.10, 0.10), caixa(0.02, 0.14, -0.10, 0.10), caixa(0.16, 0.26, -0.08, 0.08), caixa(-0.46, -0.30, -0.10, 0.10));
    let (cb, ch) = (caixa(-0.27, -0.20, -0.04, 0.04), caixa(-0.20, -0.15, -0.04, 0.04));
    let cargueiro = malha_colorida(&[
        (&h, [0.20, 0.22, 0.27, 1.0]),
        (&b1, [0.80, 0.30, 0.18, 1.0]),
        (&b2, [0.22, 0.45, 0.75, 1.0]),
        (&b3, [0.85, 0.70, 0.20, 1.0]),
        (&b4, ponte),
        (&cb, [0.97, 0.97, 0.95, 1.0]),
        (&ch, chamine),
    ]);
    // Petroleiro: mais comprido e baixo, tubulação no convés.
    let h = [[-0.5, -0.11], [0.34, -0.11], [0.5, 0.0], [0.34, 0.11], [-0.5, 0.11]];
    let (dutos, p, c) = (caixa(-0.20, 0.36, -0.015, 0.015), caixa(-0.47, -0.34, -0.085, 0.085), caixa(-0.40, -0.35, -0.04, 0.04));
    let tanques = caixa(-0.22, 0.34, -0.07, 0.07);
    let petroleiro = malha_colorida(&[
        (&h, [0.10, 0.11, 0.14, 1.0]),
        (&tanques, [0.30, 0.20, 0.17, 1.0]),
        (&dutos, [0.75, 0.75, 0.70, 1.0]),
        (&p, ponte),
        (&c, chamine),
    ]);
    // Graneleiro: largo, proa curta, escotilhas.
    let h = [[-0.5, -0.18], [0.36, -0.18], [0.5, -0.06], [0.5, 0.06], [0.36, 0.18], [-0.5, 0.18]];
    let (e1, e2, e3, e4) = (caixa(-0.22, -0.08, -0.12, 0.12), caixa(-0.05, 0.09, -0.12, 0.12), caixa(0.12, 0.26, -0.12, 0.12), caixa(0.29, 0.38, -0.09, 0.09));
    let (p, c) = (caixa(-0.46, -0.30, -0.13, 0.13), caixa(-0.38, -0.32, -0.05, 0.05));
    let esc = [0.45, 0.36, 0.24, 1.0];
    let graneleiro = malha_colorida(&[
        (&h, [0.36, 0.28, 0.20, 1.0]),
        (&e1, esc), (&e2, esc), (&e3, esc), (&e4, esc),
        (&p, ponte),
        (&c, chamine),
    ]);
    [cargueiro, petroleiro, graneleiro]
}

/// Tamanho de ícone em pixels de TELA -> unidades de mundo (compensa a escala da câmera,
/// como os marcadores de cidade). Pequeno de longe, maior de perto.
fn px_para_mundo(px: f32, escala_camera: f32) -> f32 {
    px * escala_camera.max(0.02)
}

/// Comprimento do navio em pixels de tela (6 px longe, ~16 px perto).
fn navio_px(escala_camera: f32) -> f32 {
    (10.0 * escala_camera.max(0.05).powf(-0.25)).clamp(6.0, 16.0)
}

/// Envergadura/comprimento do avião em pixels de tela (5 px longe, ~14 px perto).
fn aviao_px(escala_camera: f32) -> f32 {
    (9.0 * escala_camera.max(0.05).powf(-0.25)).clamp(5.0, 14.0)
}

fn criar_recursos(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<ColorMaterial>>) {
    let [m0, m1, m2] = malhas_navios();
    let cascos = [meshes.add(m0), meshes.add(m1), meshes.add(m2)];
    let helice = meshes.add(malha_poligonos(&[
        &[[-0.5, -0.05], [0.45, -0.05], [0.5, 0.0], [0.45, 0.05], [-0.5, 0.05]],
        &[[-0.08, -0.5], [0.12, -0.5], [0.12, 0.5], [-0.08, 0.5]],
        &[[-0.5, -0.18], [-0.4, -0.18], [-0.4, 0.18], [-0.5, 0.18]],
    ]));
    let jato = meshes.add(malha_poligonos(&[
        &[[-0.6, -0.04], [0.45, -0.04], [0.6, 0.0], [0.45, 0.04], [-0.6, 0.04]],
        &[[0.15, 0.04], [-0.25, 0.5], [-0.42, 0.5], [-0.18, 0.04]],
        &[[0.15, -0.04], [-0.25, -0.5], [-0.42, -0.5], [-0.18, -0.04]],
        &[[-0.5, 0.03], [-0.62, 0.2], [-0.68, 0.2], [-0.6, 0.03]],
        &[[-0.5, -0.03], [-0.62, -0.2], [-0.68, -0.2], [-0.6, -0.03]],
    ]));
    let mut cm = |r: f32, g: f32, b: f32| mats.add(ColorMaterial::from_color(Color::srgb(r, g, b)));
    commands.insert_resource(RecursosTrafego {
        cascos,
        aviao_helice: helice,
        aviao_jato: jato,
        mat_navio: [cm(0.22, 0.24, 0.28), cm(0.08, 0.08, 0.10), cm(0.42, 0.30, 0.20)],
        mat_vertice: cm(1.0, 1.0, 1.0),
        mat_helice: cm(0.78, 0.75, 0.62),
        mat_jato: cm(0.95, 0.96, 0.98),
    });
}

// ---------------------------------------------------------------------------------------------
// Sistemas
// ---------------------------------------------------------------------------------------------

const Z_NAVIO: f32 = 9.0;
const Z_AVIAO: f32 = 30.0;

fn carregar_fluxos_padrao(cfg: Res<ConfigTrafego>, mut fluxos: ResMut<FluxosComercio>) {
    if fluxos.fluxos.is_empty() {
        *fluxos = FluxosComercio::carregar_diretorio(&cfg.dir_economia);
        info!("trafego: {} fluxos comerciais carregados.", fluxos.fluxos.len());
    }
}

fn gerenciar_plano_sistema(
    mut commands: Commands,
    cfg: Res<ConfigTrafego>,
    fluxos: Res<FluxosComercio>,
    cidades: Res<CidadesTrafego>,
    fechados: Res<CanaisFechados>,
    rec: Option<Res<RecursosTrafego>>,
    mut estado: ResMut<EstadoPlano>,
    navios: Query<Entity, With<NavioVisual>>,
) {
    if !cfg.ativo {
        return;
    }
    if fluxos.is_changed() || (fechados.is_changed() && !fechados.is_added()) {
        estado.refazer = true;
    }
    // resultado pronto?
    let pronto = estado.rx.as_ref().and_then(|rx| rx.lock().ok().and_then(|r| r.try_recv().ok()));
    if let (Some(plano), Some(rec)) = (pronto, rec.as_ref()) {
        estado.rx = None;
        for e in &navios {
            commands.entity(e).despawn();
        }
        let _ = rec;
        estado.atual = plano;
        spawnar_navios(&mut commands, &estado.atual, rec);
        info!(
            "trafego: {} rotas, {} navios.",
            estado.atual.len(),
            estado.atual.iter().map(|p| p.navios).sum::<usize>()
        );
        return;
    }
    if estado.rx.is_some() || cidades.cidades.is_empty() || fluxos.fluxos.is_empty() {
        return;
    }
    if estado.refazer || estado.versao_cidades != cidades.versao && estado.atual.is_empty() {
        estado.refazer = false;
        estado.versao_cidades = cidades.versao;
        let (tx, rx) = mpsc::channel();
        estado.rx = Some(Mutex::new(rx));
        let (cam, cs, fl) = (cfg.caminho_mascara.clone(), cidades.cidades.clone(), fluxos.fluxos.clone());
        let fechados = fechados.0.clone();
        let (vpn, mpf, mt) = (cfg.volume_por_navio, cfg.max_navios_por_fluxo, cfg.max_navios);
        std::thread::spawn(move || {
            let _ = tx.send(calcular_plano_com(&cam, &cs, &fl, vpn, mpf, mt, &fechados));
        });
    }
}

fn spawnar_navios(commands: &mut Commands, plano: &[RotaNavios], rec: &RecursosTrafego) {
    let mut idx = 0u32;
    for p in plano {
        let mat = match p.tipo {
            TipoNavio::Cargueiro => 0,
            TipoNavio::Petroleiro => 1,
            TipoNavio::Graneleiro => 2,
        };
        let larg_faixa = [0.26, 0.19, 0.32][mat];
        let cor = cor_pais(&p.operador);
        let len = p.rota.comprimento();
        for k in 0..p.navios {
            let frac = (k as f32 + 0.5) / p.navios as f32;
            let sentido = if k % 2 == 0 { 1.0 } else { -1.0 };
            let (pos, _) = p.rota.amostrar(len * frac);
            commands
                .spawn((
                    Mesh2d(rec.cascos[mat].clone()),
                    MeshMaterial2d(rec.mat_vertice.clone()),
                    Transform::from_translation(pos.extend(Z_NAVIO)).with_scale(Vec3::splat(8.0)),
                    Visibility::Hidden,
                    NavioVisual {
                        rota: p.rota.clone(),
                        tipo: p.tipo,
                        operador: p.operador.clone(),
                        dist: len * frac,
                        sentido,
                        espera: 0.0,
                        lateral: ((k % 3) as f32 - 1.0) * 2.0,
                        vel: 0.9 + 0.2 * (((idx * 37) % 10) as f32 / 10.0),
                        idx,
                        carga: p.carga,
                        angulo: 0.0,
                    },
                ))
                .with_children(|c| {
                    // faixa na cor do país, na popa (coordenadas locais do casco de comprimento 1)
                    c.spawn((
                        Sprite::from_color(cor, Vec2::new(0.05, larg_faixa)),
                        Transform::from_xyz(-0.48, 0.0, 0.3),
                    ));
                });
            idx += 1;
        }
    }
}

/// Fator de LOD: fração de entidades mostradas e escala do ícone.
fn lod_trafego(lod: &EstadoLod) -> (u32, f32) {
    let esc = lod.escala_camera.max(0.8).min(2.4);
    match lod.nivel_atual {
        NivelLod::Global => (0, esc),
        NivelLod::Regional => (2, esc),
        NivelLod::Local => (1, esc),
    }
}

fn mover_navios_sistema(
    time: Res<Time>,
    cfg: Res<ConfigTrafego>,
    camada: Res<CamadaConstrucoesVisivel>,
    lod: Res<EstadoLod>,
    camera: Query<(&Transform, &Projection), (With<Camera2d>, Without<NavioVisual>)>,
    janela: Query<&Window, With<PrimaryWindow>>,
    mut rel: ResMut<RelatorioCanais>,
    mut q: Query<(&mut NavioVisual, &mut Transform, &mut Visibility), Without<Camera2d>>,
) {
    let dt = time.delta_secs().min(0.1);
    let (passo, _) = lod_trafego(&lod);
    let esc = lod.escala_camera;
    let ver_base = cfg.ativo && camada.0 && lod.nivel_atual != NivelLod::Global && passo > 0;
    let tela_rect = if ver_base {
        camera.single().ok().and_then(|(ctf, proj)| {
            let s = match proj {
                Projection::Orthographic(o) => o.scale,
                _ => 1.0,
            };
            let (w, h) = janela
                .single()
                .map(|win| (win.width(), win.height()))
                .unwrap_or((1600.0, 900.0));
            let meia = Vec2::new(w, h) * 0.5 * s + Vec2::splat(50.0);
            Some((ctf.translation.truncate(), meia))
        })
    } else {
        None
    };

    for (mut n, mut tr, mut vis) in &mut q {
        let comp_rota = n.rota.comprimento();
        if n.espera > 0.0 {
            n.espera -= dt;
        } else {
            let antes = n.dist;
            let depois = (antes + n.sentido * n.vel * cfg.velocidade_navio * dt).clamp(0.0, comp_rota);
            for &(m, canal) in &n.rota.marcas {
                if (antes < m && depois >= m) || (antes > m && depois <= m) {
                    let carga = if n.sentido > 0.0 { n.carga } else { 0.0 };
                    let op = n.operador.clone();
                    rel.registrar(canal, carga, &op);
                }
            }
            n.dist = depois;
            if depois <= 0.0 || depois >= comp_rota {
                n.sentido = -n.sentido;
                n.espera = 2.5; // atraca
            }
        }
        let (pos, dir) = n.rota.amostrar(n.dist);
        let mut visivel = false;
        if ver_base && n.idx % passo == 0 {
            if let Some((c, meia)) = tela_rect {
                if (pos.y - c.y).abs() <= meia.y {
                    let dx = (pos.x - c.x).abs();
                    let dx_wrap = dx.min((pos.x + 5632.0 - c.x).abs()).min((pos.x - 5632.0 - c.x).abs());
                    if dx_wrap <= meia.x {
                        visivel = true;
                    }
                }
            } else {
                visivel = true;
            }
        }
        let novo = if visivel { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != novo {
            *vis = novo;
        }
        if !visivel {
            continue;
        }

        let dir = dir * n.sentido;
        let normal = Vec2::new(-dir.y, dir.x);
        let alvo = dir.y.atan2(dir.x);
        let mut dif = alvo - n.angulo;
        while dif > std::f32::consts::PI {
            dif -= std::f32::consts::TAU;
        }
        while dif < -std::f32::consts::PI {
            dif += std::f32::consts::TAU;
        }
        n.angulo += dif * (1.0 - (-5.0 * dt).exp());
        tr.translation = (pos + normal * n.lateral).extend(Z_NAVIO);
        tr.rotation = Quat::from_rotation_z(n.angulo);
        let mult = match n.tipo {
            TipoNavio::Cargueiro => 1.0,
            TipoNavio::Petroleiro => 1.2,
            TipoNavio::Graneleiro => 0.95,
        };
        let l = px_para_mundo(navio_px(esc) * mult * cfg.escala_icones, esc);
        tr.scale = Vec3::new(l, l, 1.0);
    }
}

fn gerenciar_avioes_sistema(
    mut commands: Commands,
    cfg: Res<ConfigTrafego>,
    cidades: Res<CidadesTrafego>,
    rec: Option<Res<RecursosTrafego>>,
    avioes: Query<(), With<AviaoVisual>>,
) {
    let Some(rec) = rec else { return };
    if !cfg.ativo || !avioes.is_empty() || cidades.cidades.len() < 2 {
        return;
    }
    let aero: Vec<Vec2> = cidades.cidades.iter().take(cfg.cidades_aeroporto).map(|c| c.mundo).collect();
    for i in 0..cfg.max_avioes {
        let mut seed = 0x9E3779B97F4A7C15u64 ^ ((i as u64 + 1).wrapping_mul(0xD1B54A32D192ED03));
        let a = aero[(rng(&mut seed) * aero.len() as f32) as usize % aero.len()];
        let b = escolher_destino(a, &aero, &mut seed);
        let t0 = rng(&mut seed);
        commands
            .spawn((
                Mesh2d(rec.aviao_helice.clone()),
                MeshMaterial2d(rec.mat_helice.clone()),
                Transform::from_translation(a.lerp(b, t0).extend(Z_AVIAO)),
                Visibility::Hidden,
                AviaoVisual {
                    ordem: i as u32,
                    origem: a,
                    destino: b,
                    t: t0,
                    vel: 0.85 + 0.3 * rng(&mut seed),
                    jato: false,
                    seed,
                    angulo: (b - a).to_angle(),
                },
            ))
            .with_children(|c| {
                c.spawn((
                    Mesh2d(rec.aviao_helice.clone()),
                    MeshMaterial2d(rec.mat_navio[1].clone()),
                    Transform::from_xyz(0.12, -0.35, -0.5),
                    Sombra,
                ));
            });
    }
}

fn escolher_destino(a: Vec2, aero: &[Vec2], seed: &mut u64) -> Vec2 {
    for _ in 0..12 {
        let b = aero[(rng(seed) * aero.len() as f32) as usize % aero.len()];
        if a.distance(b) > 250.0 {
            return b;
        }
    }
    aero[(rng(seed) * aero.len() as f32) as usize % aero.len()]
}

fn mover_avioes_sistema(
    time: Res<Time>,
    cfg: Res<ConfigTrafego>,
    ano: Res<AnoVisual>,
    camada: Res<CamadaConstrucoesVisivel>,
    lod: Res<EstadoLod>,
    camera: Query<(&Transform, &Projection), (With<Camera2d>, Without<AviaoVisual>, Without<Sombra>)>,
    janela: Query<&Window, With<PrimaryWindow>>,
    cidades: Res<CidadesTrafego>,
    rec: Option<Res<RecursosTrafego>>,
    mut q: Query<(&mut AviaoVisual, &mut Transform, &mut Visibility, &mut Mesh2d, &mut MeshMaterial2d<ColorMaterial>, &Children), (Without<Camera2d>, Without<Sombra>)>,
    mut sombras: Query<(&mut Transform, &mut Mesh2d), (With<Sombra>, Without<AviaoVisual>, Without<Camera2d>)>,
) {
    let Some(rec) = rec else { return };
    let dt = time.delta_secs().min(0.1);
    let ativos = avioes_ativos_no_ano(ano.0, cfg.max_avioes);
    let jato_frac = fracao_jato_no_ano(ano.0);
    let aero: Vec<Vec2> = cidades.cidades.iter().take(cfg.cidades_aeroporto).map(|c| c.mundo).collect();
    let esc = lod.escala_camera;
    let tela_rect = if cfg.ativo && camada.0 && lod.nivel_atual != NivelLod::Global {
        camera.single().ok().and_then(|(ctf, proj)| {
            let s = match proj {
                Projection::Orthographic(o) => o.scale,
                _ => 1.0,
            };
            let (w, h) = janela
                .single()
                .map(|win| (win.width(), win.height()))
                .unwrap_or((1600.0, 900.0));
            let meia = Vec2::new(w, h) * 0.5 * s + Vec2::splat(60.0);
            Some((ctf.translation.truncate(), meia))
        })
    } else {
        None
    };

    for (mut a, mut tr, mut vis, mut mesh, mut mat, filhos) in &mut q {
        let comp = a.origem.distance(a.destino).max(1.0);
        let v = if a.jato { cfg.velocidade_jato } else { cfg.velocidade_helice } * a.vel;
        // acelera na decolagem e freia no pouso
        let fator = (0.3 + 5.0 * a.t.min(1.0 - a.t).max(0.0)).min(1.0);
        a.t += v * fator * dt / comp;
        if a.t >= 1.0 {
            let (o, d) = (a.destino, if aero.is_empty() { a.origem } else { escolher_destino(a.destino, &aero, &mut a.seed) });
            a.origem = o;
            a.destino = d;
            a.t = 0.0;
        }

        let ativo = cfg.ativo && camada.0 && (a.ordem as usize) < ativos
            && match lod.nivel_atual {
                NivelLod::Global => false,
                NivelLod::Regional => a.ordem % 2 == 0,
                NivelLod::Local => true,
            };
        let pos = a.origem.lerp(a.destino, a.t);
        let mut visivel = false;
        if ativo {
            if let Some((c, meia)) = tela_rect {
                if (pos.y - c.y).abs() <= meia.y {
                    let dx = (pos.x - c.x).abs();
                    let dx_wrap = dx.min((pos.x + 5632.0 - c.x).abs()).min((pos.x - 5632.0 - c.x).abs());
                    if dx_wrap <= meia.x {
                        visivel = true;
                    }
                }
            } else {
                visivel = true;
            }
        }
        let novo = if visivel { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != novo {
            *vis = novo;
        }
        if !visivel {
            continue;
        }

        // tipo (hélice/jato) estável por avião: o i-ésimo mais "à esquerda" vira jato conforme o ano
        let u = ((a.ordem.wrapping_mul(2654435761)) >> 8) as f32 / (1u32 << 24) as f32;
        let jato = u < jato_frac;
        if jato != a.jato {
            a.jato = jato;
            mesh.0 = if jato { rec.aviao_jato.clone() } else { rec.aviao_helice.clone() };
            mat.0 = if jato { rec.mat_jato.clone() } else { rec.mat_helice.clone() };
            for &c in filhos {
                if let Ok((_, mut m)) = sombras.get_mut(c) {
                    m.0 = mesh.0.clone();
                }
            }
        }

        let alvo = (a.destino - a.origem).to_angle();
        let mut dif = alvo - a.angulo;
        while dif > std::f32::consts::PI {
            dif -= std::f32::consts::TAU;
        }
        while dif < -std::f32::consts::PI {
            dif += std::f32::consts::TAU;
        }
        a.angulo += dif * (1.0 - (-6.0 * dt).exp());
        let alt = (a.t.min(1.0 - a.t) * 6.0).clamp(0.0, 1.0); // 0 no solo, 1 em cruzeiro
        tr.translation = pos.extend(Z_AVIAO);
        tr.rotation = Quat::from_rotation_z(a.angulo);
        let tam = px_para_mundo(aviao_px(esc) * if a.jato { 1.25 } else { 1.0 } * cfg.escala_icones, esc) * (0.7 + 0.3 * alt);
        tr.scale = Vec3::new(tam, tam, 1.0);
        for &c in filhos {
            if let Ok((mut s, _)) = sombras.get_mut(c) {
                s.translation = Vec3::new(0.0, -0.1 - 0.5 * alt, -0.5);
            }
        }
    }
}

/// Plugin do tráfego (adicionado por [`crate::VisualMapaPlugin`]).
#[derive(Default)]
pub struct TrafegoPlugin;

impl Plugin for TrafegoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConfigTrafego>()
            .init_resource::<FluxosComercio>()
            .init_resource::<RelatorioCanais>()
            .init_resource::<CidadesTrafego>()
            .init_resource::<EstadoPlano>()
            .init_resource::<CanaisFechados>()
            .add_systems(Startup, (criar_recursos, carregar_fluxos_padrao))
            .add_systems(
                Update,
                (
                    coletar_cidades_sistema.run_if(
                        |c: Res<CidadesTrafego>, v: Res<EntradaVisual>, e: Res<EntradaEconomiaVisual>| {
                            c.cidades.is_empty() || v.is_changed() || e.is_changed()
                        },
                    ),
                    gerenciar_plano_sistema,
                    gerenciar_avioes_sistema,
                    mover_navios_sistema,
                    mover_avioes_sistema,
                )
                    .chain()
                    .after(crate::lod::atualizar_lod_sistema),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avioes_crescem_com_a_era() {
        let a = avioes_ativos_no_ano(1945.0, 160);
        let b = avioes_ativos_no_ano(1965.0, 160);
        let c = avioes_ativos_no_ano(1990.0, 160);
        assert!(a < b && b < c && c <= 160);
    }

    #[test]
    fn jato_so_a_partir_de_1958() {
        assert_eq!(fracao_jato_no_ano(1945.0), 0.0);
        assert_eq!(fracao_jato_no_ano(1957.9), 0.0);
        assert!(fracao_jato_no_ano(1958.0) > 0.0);
        assert!(fracao_jato_no_ano(1975.0) > fracao_jato_no_ano(1960.0));
    }

    #[test]
    fn tipos_por_bem() {
        assert_eq!(tipo_navio_por_bem("petroleo"), TipoNavio::Petroleiro);
        assert_eq!(tipo_navio_por_bem("graos"), TipoNavio::Graneleiro);
        assert_eq!(tipo_navio_por_bem("maquinas"), TipoNavio::Cargueiro);
    }

    #[test]
    fn relatorio_conta_e_entrega_pendentes() {
        let mut r = RelatorioCanais::default();
        r.registrar(Canal::Suez, 50.0, "gbr");
        r.registrar(Canal::Suez, 0.0, "gbr");
        r.registrar(Canal::Panama, 10.0, "usa");
        assert_eq!(r.canal(Canal::Suez), PassagensCanal { contagem: 2, volume: 50.0 });
        assert_eq!(r.do_operador(Canal::Panama, "usa").contagem, 1);
        assert_eq!(r.tomar_pendentes().len(), 3);
        assert!(r.tomar_pendentes().is_empty());
        assert_eq!(r.canal(Canal::Suez).contagem, 2);
    }

    #[test]
    fn carrega_fluxos_reais_e_planeja_sem_terra() {
        let cfg = ConfigTrafego::default();
        let f = FluxosComercio::carregar_diretorio(&cfg.dir_economia);
        assert!(!f.fluxos.is_empty());
        assert_eq!(f.fluxos[0].origem, "usa");
        let cid = carregar_cidades_cenario(Path::new(CIDADES_TERRITORIO_PADRAO));
        if cid.is_empty() {
            return;
        }
        let proj = ProjecaoMapa::default();
        let cidades: Vec<CidadeTrafego> = cid
            .into_iter()
            .map(|c| CidadeTrafego {
                id: c.id,
                pais: c.pais_tag.to_ascii_lowercase(),
                mundo: proj.geo_para_mundo(c.posicao.0, c.posicao.1),
                populacao: c.populacao_urbana,
                porto: c.eh_porto,
            })
            .collect();
        let plano = calcular_plano(Path::new(MASCARA_PADRAO), &cidades, &f.fluxos, 40.0, 8, 400);
        assert!(plano.len() >= f.fluxos.len() / 2, "só {} de {} rotas", plano.len(), f.fluxos.len());
        let r = roteador(Path::new(MASCARA_PADRAO)).unwrap();
        for p in &plano {
            let portais: Vec<f32> = p.rota.marcas.iter().map(|m| m.0).collect();
            for (i, w) in p.rota.pontos.windows(2).enumerate() {
                let meio = (p.rota.acumulado[i] + p.rota.acumulado[i + 1]) * 0.5;
                if portais.iter().any(|d| (d - meio).abs() < 1.0) {
                    continue;
                }
                assert!(
                    r.mascara.linha_livre(w[0], w[1]) || w[0].distance(w[1]) < 60.0,
                    "{}->{} atravessa terra",
                    p.origem,
                    p.destino
                );
            }
        }
        // Venezuela -> EUA: Caribe -> Golfo, sem canais; GBR <- IRN: via Suez
        let tem = |o: &str, d: &str| plano.iter().find(|p| p.origem == o && p.destino == d);
        if let Some(p) = tem("irn", "gbr") {
            assert!(p.rota.marcas.iter().any(|(_, c)| *c == Canal::Suez), "irn->gbr sem Suez");
        }
        if let Some(p) = tem("chl", "usa") {
            eprintln!("chl->usa canais: {:?} comp {}", p.rota.marcas, p.rota.comprimento());
        }
    }

    #[test]
    fn app_headless_gera_navios_avioes_e_relatorio() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<ColorMaterial>()
            .init_resource::<VisualMapaConfig>()
            .init_resource::<EntradaVisual>()
            .init_resource::<EntradaEconomiaVisual>()
            .init_resource::<EstadoLod>()
            .init_resource::<AnoVisual>()
            .init_resource::<CamadaConstrucoesVisivel>()
            .add_plugins(TrafegoPlugin);
        {
            let mut v = app.world_mut().resource_mut::<EntradaVisual>();
            for c in carregar_cidades_cenario(Path::new(CIDADES_TERRITORIO_PADRAO)).into_iter().take(1500) {
                v.cidades.push(crate::entrada::CidadeEntrada {
                    id: c.id,
                    pais_tag: c.pais_tag,
                    posicao: c.posicao,
                    populacao: c.populacao_urbana as u32,
                    eh_porto: c.eh_porto,
                    ..Default::default()
                });
            }
        }
        app.world_mut().resource_mut::<EstadoLod>().nivel_atual = NivelLod::Local;
        app.world_mut().resource_mut::<AnoVisual>().0 = 1965.0;
        let t0 = std::time::Instant::now();
        loop {
            app.update();
            let n = app.world_mut().query::<&NavioVisual>().iter(app.world()).count();
            if n > 0 || t0.elapsed().as_secs() > 60 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let navios = app.world_mut().query::<&NavioVisual>().iter(app.world()).count();
        assert!(navios > 20, "navios: {navios}");
        for _ in 0..5 {
            app.update();
        }
        let avioes = app.world_mut().query::<&AviaoVisual>().iter(app.world()).count();
        assert_eq!(avioes, 160);
        let jatos = app.world_mut().query::<&AviaoVisual>().iter(app.world()).filter(|a| a.jato).count();
        assert!(jatos > 0, "sem jatos em 1965");
        // avança navios artificialmente para forçar passagens
        for _ in 0..4000 {
            app.world_mut().resource_mut::<ConfigTrafego>().velocidade_navio = 4000.0;
            std::thread::sleep(std::time::Duration::from_micros(200));
            app.update();
        }
        let rel = app.world().resource::<RelatorioCanais>();
        eprintln!("navios {navios} avioes {avioes} jatos {jatos} relatorio {:?}", rel.todos());
        assert!(rel.todos().iter().any(|(_, p)| p.contagem > 0), "nenhuma passagem registrada");
    }
}
