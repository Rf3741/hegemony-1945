//! Ponte entre o cliente e o motor econômico `sim_economia` (estilo Victoria 3).
//!
//! * O cenário de 1945 (`assets/data/economia`) é carregado em segundo plano na tela de carregamento
//!   (com 90 dias de aquecimento, para o modelo sair do transitório de calibração antes do jogo começar).
//! * O tick diário roda numa TAREFA em outra thread (a economia leva cerca de 25 a 30 ms por dia): o
//!   Bevy nunca fica bloqueado. A interface só lê um instantâneo (`VistaEconomia`), reconstruído ao fim
//!   de cada lote de dias; ações do jogador viram `Comando`s aplicados antes do próximo lote.
//! * O legado `simulacao` continua só para política e dono das províncias.
//!
//! Mapa de tags (legado -> economia): quase todas coincidem; as exceções estão em `ALIAS_TAGS`.

use super::time::EventoTickDiario;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, TaskPool, block_on, futures_lite::future};
use sim_economia::{
    CatalogoEconomia, CenarioEconomia, Estado, MercadoGlobal, MundoEconomico, Pais, Provincia,
    estado_comporta_novo_nivel, provincia_comporta_novo_nivel,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Tags do legado `simulacao` que têm outra grafia no cenário econômico: (legado, economia).
pub const ALIAS_TAGS: [(&str, &str); 12] = [
    ("BGR", "BUL"),
    ("BTN", "BHU"),
    ("GRC", "GRE"),
    ("GTM", "GUA"),
    ("HND", "HON"),
    ("HTI", "HAI"),
    ("MNG", "MON"),
    ("NPL", "NEP"),
    ("PRY", "PAR"),
    ("ROU", "ROM"),
    ("THA", "SIA"),
    ("URY", "URU"),
];

/// Dias de aquecimento do modelo antes de entregar o jogo (esconde o transitório inicial).
pub const DIAS_AQUECIMENTO: u32 = 90;
/// Máximo de dias pendentes; acima disso a economia "perde" dias (aparece no HUD).
const MAX_ATRASO_DIAS: u32 = 60;
/// Máximo de dias por lote enviado à thread.
const DIAS_POR_LOTE: u32 = 30;

// ---------------------------------------------------------------------------------------------
// Núcleo (sem Bevy): mundo + mercado global + catálogo
// ---------------------------------------------------------------------------------------------

pub struct Nucleo {
    pub mundo: MundoEconomico,
    pub global: MercadoGlobal,
    pub catalogo: Arc<CatalogoEconomia>,
    pub tags: BTreeMap<String, String>,
}

/// Ação do jogador sobre a economia (aplicada pela thread antes do próximo lote).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Comando {
    Construir { provincia_id: String, tipo_id: String },
    CancelarObra(usize),
    MoverObra { de: usize, para: usize },
    /// Troca, em todos os prédios do tipo no país, o método do slot do `metodo_id`.
    TrocarMetodoTodos { tipo_id: String, metodo_id: String },
    TrocarMetodo { provincia_id: String, tipo_id: String, metodo_id: String },
    /// Multiplica as alíquotas de imposto de renda e dividendos por este fator.
    EscalarImposto(f64),
    /// Soma `delta` à tarifa de importação (limitada a 0..0,6).
    AjustarTarifa(f64),
    /// Cria uma nova rota de importação ou exportação.
    CriarRota { bem_id: String, parceiro_id: String, exportacao: bool, volume: f64 },
    /// Remove rota comercial existente.
    RemoverRota(String),
    /// Alterna se a rota ajusta volume dinamicamente por rentabilidade.
    AlternarRotaAuto(String),
    /// Ajusta o volume máximo de uma rota comercial.
    AjustarVolumeRota { rota_id: String, volume: f64 },
    /// Define as tarifas gerais de importação e exportação.
    DefinirTarifaGeral { importacao: f64, exportacao: f64 },
    /// Define tarifa seletiva de importação/exportação sobre um bem específico.
    DefinirTarifaBem { bem_id: String, tarifa_imp: f64, tarifa_exp: f64 },
    /// Define o nível geral de abertura de mercado (0.0 = autarquia, 1.0 = livre-câmbio).
    DefinirAbertura(f64),
    /// Firma acordo comercial bilateral com outro país.
    AssinarAcordo(String),
    /// Revoga acordo comercial bilateral.
    CancelarAcordo(String),
    /// Alterna a ativação da integração comercial COMECON no Bloco Soviético.
    AlternarComecon,
}

impl Nucleo {
    pub fn carregar(dir: &Path, aquecimento: u32) -> Result<Self, String> {
        let mut cen = CenarioEconomia::carregar_diretorio(dir).map_err(|e| e.to_string())?;
        // Demografia e mercado de trabalho só a cada 7 dias: ~25% mais rápido, sem perder a produção/preços diários.
        cen.config.modo_cadencia = sim_economia::ModoCadencia::DiarioComAjusteSemanal;
        let mut mundo = MundoEconomico::novo(cen).map_err(|e| e.to_string())?;
        for _ in 0..aquecimento {
            mundo.executar_tick_diario();
        }
        let global = MercadoGlobal::novo(&mundo);
        let catalogo = Arc::new(mundo.catalogo().clone());
        let tags = mundo
            .paises()
            .iter()
            .filter_map(|p| p.tag.clone().map(|t| (t, p.id.clone())))
            .collect();
        Ok(Self { mundo, global, catalogo, tags })
    }

    /// Id econômico do país a partir da tag do legado.
    #[allow(dead_code)]
    pub fn id_pais(&self, tag_legado: &str) -> Option<String> {
        let tag = ALIAS_TAGS.iter().find(|(l, _)| *l == tag_legado).map_or(tag_legado, |(_, e)| *e);
        self.tags.get(tag).cloned()
    }

    pub fn avancar(&mut self, dias: u32) -> usize {
        let mut obras = 0;
        for _ in 0..dias {
            let rel = self.mundo.executar_tick_diario();
            obras += rel.obras_concluidas;
            self.mundo.atualizar_mercado_global(&mut self.global);
        }
        obras
    }

    pub fn aplicar(&mut self, pais_id: &str, cmd: Comando) -> Result<String, String> {
        match cmd {
            Comando::Construir { provincia_id, tipo_id } => {
                self.mundo.enfileirar_construcao(pais_id, &provincia_id, &tipo_id, false).map_err(|e| e.to_string())?;
                Ok("Obra enfileirada.".into())
            }
            Comando::CancelarObra(i) => {
                let p = self.mundo.obter_pais_mut(pais_id).ok_or("país inexistente")?;
                if i < p.fila_construcao.len() {
                    p.fila_construcao.remove(i);
                }
                Ok("Obra cancelada.".into())
            }
            Comando::MoverObra { de, para } => {
                let p = self.mundo.obter_pais_mut(pais_id).ok_or("país inexistente")?;
                let n = p.fila_construcao.len();
                if de < n && para < n && de != para {
                    let item = p.fila_construcao.remove(de);
                    p.fila_construcao.insert(para, item);
                    Ok("Fila de construção reordenada.".into())
                } else {
                    Ok("Posição mantida na fila.".into())
                }
            }
            Comando::TrocarMetodo { provincia_id, tipo_id, metodo_id } => {
                let atuais = self
                    .mundo
                    .obter_provincia(&provincia_id)
                    .and_then(|p| p.predios.iter().find(|b| b.tipo_predio_id == tipo_id))
                    .map(|b| b.metodos_ativos.clone())
                    .ok_or("prédio inexistente")?;
                let novos = self.substituir_metodo(&atuais, &metodo_id);
                self.mundo.alterar_metodos_predio(&provincia_id, &tipo_id, novos).map_err(|e| e.to_string())?;
                Ok("Método de produção trocado.".into())
            }
            Comando::TrocarMetodoTodos { tipo_id, metodo_id } => {
                let alvos: Vec<(String, Vec<String>)> = self
                    .mundo
                    .provincias()
                    .iter()
                    .filter(|p| p.pais_id == pais_id)
                    .filter_map(|p| p.predios.iter().find(|b| b.tipo_predio_id == tipo_id).map(|b| (p.id.clone(), b.metodos_ativos.clone())))
                    .collect();
                let mut ok = 0;
                for (pid, atuais) in alvos {
                    let novos = self.substituir_metodo(&atuais, &metodo_id);
                    if self.mundo.alterar_metodos_predio(&pid, &tipo_id, novos).is_ok() {
                        ok += 1;
                    }
                }
                Ok(format!("Método trocado em {ok} prédios."))
            }
            Comando::EscalarImposto(f) => {
                let p = self.mundo.obter_pais_mut(pais_id).ok_or("país inexistente")?;
                let pf = &mut p.politica_fiscal;
                for a in [&mut pf.aliquota_imposto_renda_baixo, &mut pf.aliquota_imposto_renda_medio, &mut pf.aliquota_imposto_renda_alto, &mut pf.aliquota_imposto_dividendos] {
                    *a = (*a * f).clamp(0.0, 0.8);
                }
                self.mundo.atualizar_cache_fiscal_pais(pais_id).map_err(|e| e.to_string())?;
                Ok("Impostos ajustados.".into())
            }
            Comando::AjustarTarifa(d) => {
                let p = self.mundo.obter_pais_mut(pais_id).ok_or("país inexistente")?;
                p.politica_fiscal.tarifa_importacao = (p.politica_fiscal.tarifa_importacao + d).clamp(0.0, 0.6);
                self.mundo.atualizar_cache_fiscal_pais(pais_id).map_err(|e| e.to_string())?;
                Ok("Tarifa de importação ajustada.".into())
            }
            Comando::CriarRota { bem_id, parceiro_id, exportacao, volume } => {
                let id = self.mundo.criar_rota_comercial(pais_id, &parceiro_id, &bem_id, exportacao, volume)
                    .map_err(|e| e.to_string())?;
                Ok(format!("Rota comercial criada: {id}"))
            }
            Comando::RemoverRota(id) => {
                self.mundo.remover_rota_comercial(&id).map_err(|e| e.to_string())?;
                Ok("Rota comercial removida.".into())
            }
            Comando::AlternarRotaAuto(id) => {
                let auto = self.mundo.alternar_rota_automatica(&id).map_err(|e| e.to_string())?;
                Ok(if auto { "Rota em modo automático." } else { "Rota em volume manual fixo." }.into())
            }
            Comando::AjustarVolumeRota { rota_id, volume } => {
                self.mundo.ajustar_volume_rota(&rota_id, volume).map_err(|e| e.to_string())?;
                Ok("Volume máximo da rota ajustado.".into())
            }
            Comando::DefinirTarifaGeral { importacao, exportacao } => {
                self.mundo.definir_tarifa_geral(pais_id, importacao, exportacao).map_err(|e| e.to_string())?;
                self.mundo.atualizar_cache_fiscal_pais(pais_id).map_err(|e| e.to_string())?;
                Ok("Tarifas gerais atualizadas.".into())
            }
            Comando::DefinirTarifaBem { bem_id, tarifa_imp, tarifa_exp } => {
                self.mundo.definir_tarifa_bem(pais_id, &bem_id, tarifa_imp, tarifa_exp).map_err(|e| e.to_string())?;
                self.mundo.atualizar_cache_fiscal_pais(pais_id).map_err(|e| e.to_string())?;
                Ok(format!("Tarifa atualizada para {bem_id}."))
            }
            Comando::DefinirAbertura(nivel) => {
                self.mundo.definir_nivel_abertura(pais_id, nivel).map_err(|e| e.to_string())?;
                self.mundo.atualizar_cache_fiscal_pais(pais_id).map_err(|e| e.to_string())?;
                Ok("Nível de abertura de mercado atualizado.".into())
            }
            Comando::AssinarAcordo(parceiro_id) => {
                self.mundo.assinar_acordo_comercial(pais_id, &parceiro_id).map_err(|e| e.to_string())?;
                Ok("Acordo comercial bilateral firmado.".into())
            }
            Comando::CancelarAcordo(parceiro_id) => {
                self.mundo.cancelar_acordo_comercial(pais_id, &parceiro_id).map_err(|e| e.to_string())?;
                Ok("Acordo comercial bilateral revogado.".into())
            }
            Comando::AlternarComecon => {
                let ativo = self.mundo.alternar_comecon().map_err(|e| e.to_string())?;
                Ok(if ativo { "COMECON ativado: cooperação e livre comércio no bloco socialista." } else { "COMECON desativado." }.into())
            }
        }
    }

    /// Lista de métodos ativos com o método do mesmo slot de `metodo_id` substituído.
    fn substituir_metodo(&self, atuais: &[String], metodo_id: &str) -> Vec<String> {
        let slot_de = |id: &str| self.catalogo.metodos_producao.iter().find(|m| m.id == id).map(|m| m.slot);
        let Some(slot) = slot_de(metodo_id) else { return atuais.to_vec() };
        let mut novos: Vec<String> = Vec::with_capacity(atuais.len() + 1);
        let mut feito = false;
        for m in atuais {
            if slot_de(m) == Some(slot) {
                if !feito {
                    novos.push(metodo_id.to_string());
                    feito = true;
                }
            } else {
                novos.push(m.clone());
            }
        }
        if !feito {
            novos.push(metodo_id.to_string());
        }
        novos
    }

    /// Instantâneo para a interface do país `pais_id`.
    pub fn vista(&self, pais_id: &str) -> VistaEconomia {
        let mundo = &self.mundo;
        let cat = &self.catalogo;
        let m_idx = mundo.indice_mercado_do_pais(pais_id);
        let mut v = VistaEconomia { pais_id: pais_id.to_string(), catalogo: Some(self.catalogo.clone()), dias: mundo.dias_decorridos(), ..Default::default() };

        // Ranking de todos os países
        for p in mundo.paises() {
            v.ranking.push(ResumoPaisEco {
                id: p.id.clone(),
                nome: p.nome.clone(),
                pib_anual: p.pib_diario * 365.0,
                populacao: mundo.populacao_exibida_pais(&p.id).unwrap_or(0.0),
                padrao_vida: mundo.padrao_de_vida_medio_pais(&p.id).unwrap_or(0.0),
            });
        }
        v.pib_mundial_anual = v.ranking.iter().map(|r| r.pib_anual).sum();

        // Mercado: um item por bem
        if let Some(m) = m_idx {
            for (b, bem) in cat.bens.iter().enumerate() {
                let (p, o, d, imp) = mundo.dados_mercado_bem(m, b);
                let mut prod: Vec<(String, f64)> = Vec::new();
                let mut cons: Vec<(String, f64)> = Vec::new();
                for (mi, mk) in mundo.mercados().iter().enumerate() {
                    let (_, om, dm, _) = mundo.dados_mercado_bem(mi, b);
                    prod.push((mk.nome.clone(), om));
                    cons.push((mk.nome.clone(), dm));
                }
                prod.sort_by(|a, b| b.1.total_cmp(&a.1));
                cons.sort_by(|a, b| b.1.total_cmp(&a.1));
                prod.truncate(6);
                cons.truncate(6);
                v.bens.push(VistaBem {
                    idx: b,
                    id: bem.id.clone(),
                    nome: bem.nome.clone(),
                    categoria: bem.categoria,
                    comercializavel: bem.comercializavel,
                    preco_base: bem.preco_base,
                    preco_mundial: self.global.preco_mundial[b],
                    preco_nacional: p,
                    var30_mundial: self.global.variacao(b, 30, false),
                    var30_nacional: self.global.variacao(b, 30, true),
                    oferta_nacional: o,
                    demanda_nacional: d,
                    saldo_comercial: imp,
                    oferta_mundial: self.global.oferta_mundial[b],
                    demanda_mundial: self.global.demanda_mundial[b],
                    produtores: prod,
                    consumidores: cons,
                    hist_mundial: self.global.historico_mundial[b].iter().copied().collect(),
                    hist_nacional: self.global.historico_nacional[b].iter().copied().collect(),
                });
            }
        }

        // País do jogador
        let Some(pais) = mundo.obter_pais(pais_id) else { return v };
        v.pib_anual = pais.pib_diario * 365.0;
        v.padrao_vida = mundo.padrao_de_vida_medio_pais(pais_id).unwrap_or(0.0);
        v.populacao = mundo.populacao_exibida_pais(pais_id).unwrap_or(0.0);
        v.pontos_construcao_dia = mundo.pontos_construcao_dia(pais_id).unwrap_or(0.0);
        v.custo_materiais = cat.tipos_predio.iter().map(|t| mundo.custo_materiais_obra(pais_id, &t.id).unwrap_or(0.0)).collect();
        v.estados = mundo.estados().iter().filter(|e| e.pais_id == pais_id).cloned().collect();
        v.provincias = mundo.provincias().iter().filter(|p| p.pais_id == pais_id).cloned().collect();

        // Pops por tipo e desemprego
        let mut por_tipo: BTreeMap<String, VistaPop> = BTreeMap::new();
        let (mut pop_trab, mut desemp) = (0.0, 0.0);
        for pr in &v.provincias {
            for pop in &pr.pops {
                let tp = cat.tipos_pop.iter().find(|t| t.id == pop.tipo_pop_id);
                let e = por_tipo.entry(pop.tipo_pop_id.clone()).or_insert_with(|| VistaPop {
                    id: pop.tipo_pop_id.clone(),
                    nome: tp.map(|t| t.nome.clone()).unwrap_or_else(|| pop.tipo_pop_id.clone()),
                    ..Default::default()
                });
                e.populacao += pop.populacao;
                e.empregados += pop.empregados;
                e.renda += pop.renda_ultimo_tick;
                e.sol_ponderado += pop.padrao_de_vida * pop.populacao;
                if !tp.is_some_and(|t| t.subsistencia) {
                    pop_trab += pop.populacao;
                    desemp += pop.desempregados();
                }
            }
        }
        v.pops = por_tipo.into_values().collect();
        v.pops.sort_by(|a, b| b.populacao.total_cmp(&a.populacao));
        v.desemprego = if pop_trab > 0.0 { desemp / pop_trab } else { 0.0 };
        v.pais = Some(pais.clone());
        v.resumo_comercio = mundo.resumo_comercio_pais(pais_id);
        v.rotas_comerciais = mundo.detalhes_rotas_pais(pais_id);
        v.parceiros_comerciais = mundo.opcoes_parceiros_pais(pais_id);
        v
    }
}

// ---------------------------------------------------------------------------------------------
// Instantâneo para a interface
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct VistaPop {
    pub id: String,
    pub nome: String,
    pub populacao: f64,
    pub empregados: f64,
    pub renda: f64,
    pub sol_ponderado: f64,
}

impl VistaPop {
    pub fn padrao_vida(&self) -> f64 {
        self.sol_ponderado / self.populacao.max(1.0)
    }
    /// Renda diária média por pessoa.
    pub fn renda_per_capita(&self) -> f64 {
        self.renda / self.populacao.max(1.0)
    }
}

#[derive(Debug, Clone)]
pub struct VistaBem {
    #[allow(dead_code)]
    pub idx: usize,
    pub id: String,
    pub nome: String,
    pub categoria: sim_economia::CategoriaBem,
    pub comercializavel: bool,
    pub preco_base: f64,
    pub preco_mundial: f64,
    pub preco_nacional: f64,
    pub var30_mundial: f64,
    pub var30_nacional: f64,
    pub oferta_nacional: f64,
    pub demanda_nacional: f64,
    /// Importações líquidas (+) ou exportações (-) do mercado do jogador.
    pub saldo_comercial: f64,
    pub oferta_mundial: f64,
    pub demanda_mundial: f64,
    pub produtores: Vec<(String, f64)>,
    pub consumidores: Vec<(String, f64)>,
    pub hist_mundial: Vec<f32>,
    pub hist_nacional: Vec<f32>,
}

#[derive(Debug, Clone, Default)]
pub struct ResumoPaisEco {
    pub id: String,
    pub nome: String,
    pub pib_anual: f64,
    pub populacao: f64,
    pub padrao_vida: f64,
}

#[derive(Debug, Clone, Default)]
pub struct VistaEconomia {
    pub pais_id: String,
    #[allow(dead_code)]
    pub dias: u64,
    pub catalogo: Option<Arc<CatalogoEconomia>>,
    pub pais: Option<Pais>,
    pub estados: Vec<Estado>,
    pub provincias: Vec<Provincia>,
    pub pib_anual: f64,
    pub pib_mundial_anual: f64,
    pub padrao_vida: f64,
    pub populacao: f64,
    pub desemprego: f64,
    pub pontos_construcao_dia: f64,
    /// Custo em materiais (a preços atuais) de 1 nível de cada tipo de prédio (ordem do catálogo).
    pub custo_materiais: Vec<f64>,
    pub pops: Vec<VistaPop>,
    pub bens: Vec<VistaBem>,
    pub ranking: Vec<ResumoPaisEco>,
    pub resumo_comercio: sim_economia::ResumoComercioPais,
    pub rotas_comerciais: Vec<sim_economia::DetalheRotaComercial>,
    pub parceiros_comerciais: Vec<sim_economia::OpcaoParceiroComercial>,
}

impl VistaEconomia {
    /// Se o prédio comporta +1 nível na província (terra arável, culturas permitidas, limites de recurso do estado).
    pub fn comporta(&self, prov: &Provincia, tipo_idx: usize) -> bool {
        let Some(cat) = self.catalogo.as_ref() else { return false };
        if let Some(estado) = self.estados.iter().find(|e| e.id == prov.estado_id) {
            estado_comporta_novo_nivel(cat, estado, prov, tipo_idx)
        } else {
            provincia_comporta_novo_nivel(cat, prov, tipo_idx)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Integração com o Bevy
// ---------------------------------------------------------------------------------------------

type SaidaLote = (Nucleo, VistaEconomia, Vec<String>, f64, usize);

/// Conjunto de sistemas para ordenação com outros plugins (ex: visual das construções).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EconomiaSimSet {
    /// Recolhe o resultado do lote simulado pela thread.
    Recolher,
    /// Sincroniza dados da simulação (como novas construções para o mapa).
    SincronizarConstrucoes,
    /// Despacha o próximo lote de dias para a thread.
    Despachar,
}

/// Estado da economia no ECS. `nucleo` fica `None` enquanto a thread processa um lote.
#[derive(Resource)]
pub struct EconomiaSim {
    pub nucleo: Option<Nucleo>,
    tarefa: Option<Task<SaidaLote>>,
    pub vista: VistaEconomia,
    pendentes: u32,
    comandos: Vec<Comando>,
    /// País econômico do jogador (None antes da escolha).
    pub pais_id: Option<String>,
    pub mensagens: Vec<String>,
    /// Média móvel do custo de um dia simulado (ms).
    pub ms_por_dia: f64,
    pub dias_perdidos: u64,
    tags: BTreeMap<String, String>,
    pub obras_novas_lote: usize,
    pub versao_construcao: u64,
}

impl EconomiaSim {
    pub fn enviar(&mut self, c: Comando) {
        self.comandos.push(c);
    }
    #[allow(dead_code)]
    pub fn ocupada(&self) -> bool {
        self.tarefa.is_some() || self.pendentes > 0
    }
    pub fn atraso_dias(&self) -> u32 {
        self.pendentes
    }
    /// Id econômico para a tag do legado.
    pub fn id_pais(&self, tag_legado: &str) -> Option<String> {
        let tag = ALIAS_TAGS.iter().find(|(l, _)| *l == tag_legado).map_or(tag_legado, |(_, e)| *e);
        self.tags.get(tag).cloned()
    }
}

#[derive(Resource)]
struct CargaEconomia(Task<Result<Nucleo, String>>);

pub struct EconomiaSimPlugin;

impl Plugin for EconomiaSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(crate::estado::AppState::Carregando), iniciar_carga_economia)
            .add_systems(Update, aguardar_economia.run_if(in_state(crate::estado::AppState::Carregando)))
            .add_systems(
                Update,
                (
                    acumular_dias,
                    definir_pais_economico,
                    conduzir_economia_recolher.in_set(EconomiaSimSet::Recolher),
                    conduzir_economia_despachar
                        .in_set(EconomiaSimSet::Despachar)
                        .after(EconomiaSimSet::Recolher),
                )
                    .chain()
                    .run_if(resource_exists::<EconomiaSim>),
            );
    }
}

pub fn dir_economia(raiz: &Path) -> PathBuf {
    raiz.join("assets/data/economia")
}

fn iniciar_carga_economia(mut commands: Commands, raiz: Res<super::map::RaizProjeto>) {
    let dir = dir_economia(&raiz.0);
    let tarefa = AsyncComputeTaskPool::get_or_init(TaskPool::default).spawn(async move { Nucleo::carregar(&dir, DIAS_AQUECIMENTO) });
    commands.insert_resource(CargaEconomia(tarefa));
}

fn aguardar_economia(mut commands: Commands, carga: Option<ResMut<CargaEconomia>>) {
    let Some(mut carga) = carga else { return };
    let Some(res) = block_on(future::poll_once(&mut carga.0)) else { return };
    commands.remove_resource::<CargaEconomia>();
    match res {
        Ok(nucleo) => {
            let tags = nucleo.tags.clone();
            commands.insert_resource(EconomiaSim {
                nucleo: Some(nucleo),
                tarefa: None,
                vista: VistaEconomia::default(),
                pendentes: 0,
                comandos: Vec::new(),
                pais_id: None,
                mensagens: Vec::new(),
                ms_por_dia: 0.0,
                dias_perdidos: 0,
                tags,
                obras_novas_lote: 0,
                versao_construcao: 0,
            });
        }
        Err(e) => error!("falha ao carregar a economia: {e}"),
    }
}

fn acumular_dias(mut leitor: MessageReader<EventoTickDiario>, mut eco: ResMut<EconomiaSim>) {
    for _ in leitor.read() {
        eco.pendentes += 1;
    }
    if eco.pendentes > MAX_ATRASO_DIAS {
        eco.dias_perdidos += (eco.pendentes - MAX_ATRASO_DIAS) as u64;
        eco.pendentes = MAX_ATRASO_DIAS;
    }
}

/// Mantém `EconomiaSim::pais_id` em sincronia com o país escolhido no menu.
fn definir_pais_economico(jogador: Option<Res<super::jogador::PaisJogador>>, mut eco: ResMut<EconomiaSim>) {
    let Some(j) = jogador else { return };
    if let Some(id) = eco.id_pais(&j.0)
        && eco.pais_id.as_deref() != Some(id.as_str())
    {
        eco.pais_id = Some(id);
        // Força um lote (mesmo com 0 dias) para montar a vista do novo país.
        eco.vista.pais_id.clear();
    }
}

fn recolher_interno(eco: &mut EconomiaSim) {
    if let Some(t) = eco.tarefa.as_mut()
        && let Some((nucleo, vista, msgs, ms, obras)) = block_on(future::poll_once(t))
    {
        eco.tarefa = None;
        eco.nucleo = Some(nucleo);
        eco.vista = vista;
        eco.mensagens.extend(msgs);
        eco.ms_por_dia = if eco.ms_por_dia == 0.0 { ms } else { 0.8 * eco.ms_por_dia + 0.2 * ms };
        if obras > 0 {
            eco.obras_novas_lote += obras;
            eco.versao_construcao = eco.versao_construcao.wrapping_add(1);
        }
    }
}

fn despachar_interno(eco: &mut EconomiaSim) {
    let precisa_vista = eco.pais_id.is_some() && eco.vista.pais_id.as_str() != eco.pais_id.as_deref().unwrap_or("");
    if eco.nucleo.is_none() || (eco.pendentes == 0 && eco.comandos.is_empty() && !precisa_vista) {
        return;
    }
    let dias = eco.pendentes.min(DIAS_POR_LOTE);
    eco.pendentes -= dias;
    let cmds = std::mem::take(&mut eco.comandos);
    let pais = eco.pais_id.clone();
    let mut nucleo = eco.nucleo.take().expect("núcleo presente");
    eco.tarefa = Some(AsyncComputeTaskPool::get_or_init(TaskPool::default).spawn(async move {
        let (vista, msgs, ms, obras) = executar_lote(&mut nucleo, pais.as_deref(), cmds, dias);
        (nucleo, vista, msgs, ms, obras)
    }));
}

pub fn conduzir_economia_recolher(mut eco: ResMut<EconomiaSim>) {
    recolher_interno(&mut *eco);
}

pub fn conduzir_economia_despachar(mut eco: ResMut<EconomiaSim>) {
    despachar_interno(&mut *eco);
}

#[allow(dead_code)]
pub fn conduzir_economia(mut eco: ResMut<EconomiaSim>) {
    recolher_interno(&mut *eco);
    despachar_interno(&mut *eco);
}

/// Aplica comandos, avança `dias` e monta a vista. Devolve (vista, mensagens, ms por dia, obras concluídas).
pub fn executar_lote(
    nucleo: &mut Nucleo,
    pais: Option<&str>,
    cmds: Vec<Comando>,
    dias: u32,
) -> (VistaEconomia, Vec<String>, f64, usize) {
    let mut msgs = Vec::new();
    if let Some(p) = pais {
        let idx = nucleo.mundo.indice_mercado_do_pais(p);
        nucleo.global.acompanhar_mercado(idx);
        for c in cmds {
            match nucleo.aplicar(p, c) {
                Ok(m) => msgs.push(m),
                Err(e) => msgs.push(format!("Não foi possível: {e}")),
            }
        }
    }
    let t = Instant::now();
    let obras = nucleo.avancar(dias);
    let ms = if dias > 0 { t.elapsed().as_secs_f64() * 1000.0 / dias as f64 } else { 0.0 };
    let vista = pais.map(|p| nucleo.vista(p)).unwrap_or_default();
    (vista, msgs, ms, obras)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia")
    }

    #[test]
    fn alias_de_tags_aponta_para_paises_existentes() {
        let n = Nucleo::carregar(&dir(), 0).expect("cenário real");
        for (leg, eco) in ALIAS_TAGS {
            assert!(n.tags.contains_key(eco), "tag econômica {eco} (legado {leg}) não existe");
        }
        for t in ["USA", "SOV", "GBR", "BRA", "GER", "JAP"] {
            assert!(n.id_pais(t).is_some(), "sem país econômico para {t}");
        }
        assert_eq!(n.id_pais("THA").as_deref(), Some("sia"));
    }

    #[test]
    fn mover_obra_reordena_fila_corretamente() {
        let mut n = Nucleo::carregar(&dir(), 0).expect("cenário real");
        let usa = n.id_pais("USA").unwrap();
        let prov1 = n.mundo.provincias().iter().find(|p| p.pais_id == usa).unwrap().id.clone();
        let prov2 = n.mundo.provincias().iter().filter(|p| p.pais_id == usa).nth(1).unwrap().id.clone();
        let fila_base = n.mundo.obter_pais(&usa).unwrap().fila_construcao.len();
        n.aplicar(&usa, Comando::Construir { provincia_id: prov1.clone(), tipo_id: "siderurgica".into() }).unwrap();
        n.aplicar(&usa, Comando::Construir { provincia_id: prov2.clone(), tipo_id: "fabrica_armas_leves".into() }).unwrap();
        let p = n.mundo.obter_pais(&usa).unwrap();
        assert_eq!(p.fila_construcao.len(), fila_base + 2);
        let idx1 = fila_base;
        let idx2 = fila_base + 1;
        assert_eq!(p.fila_construcao[idx1].tipo_predio_id, "siderurgica");
        assert_eq!(p.fila_construcao[idx2].tipo_predio_id, "fabrica_armas_leves");
        n.aplicar(&usa, Comando::MoverObra { de: idx2, para: idx1 }).unwrap();
        let p = n.mundo.obter_pais(&usa).unwrap();
        assert_eq!(p.fila_construcao[idx1].tipo_predio_id, "fabrica_armas_leves");
        assert_eq!(p.fila_construcao[idx2].tipo_predio_id, "siderurgica");
    }

    /// Teste headless: cenário real, 5 anos, jogador = EUA, com obra, troca de método e imposto.
    #[test]
    fn cinco_anos_headless_sem_nan_e_precos_na_faixa() {
        let mut n = Nucleo::carregar(&dir(), DIAS_AQUECIMENTO).expect("cenário real");
        let usa = n.id_pais("USA").unwrap();
        let (v0, _, _, _) = executar_lote(&mut n, Some(&usa), vec![], 0);
        assert!(!v0.bens.is_empty() && v0.pais.is_some());
        // Constrói uma siderúrgica na primeira província que comporta
        let tipo_idx = n.catalogo.tipos_predio.iter().position(|t| t.id == "siderurgica").unwrap();
        let prov = v0.provincias.iter().find(|p| v0.comporta(p, tipo_idx)).expect("província livre");
        let cmds = vec![
            Comando::Construir { provincia_id: prov.id.clone(), tipo_id: "siderurgica".into() },
            Comando::EscalarImposto(1.1),
            Comando::AjustarTarifa(0.02),
        ];
        let t = Instant::now();
        let mut total_dias = 0u32;
        let mut msgs_total = Vec::new();
        let mut vista = v0;
        let mut cmds = Some(cmds);
        for _ in 0..(1826 / 30 + 1) {
            let (v, m, _, _) = executar_lote(&mut n, Some(&usa), cmds.take().unwrap_or_default(), 30);
            msgs_total.extend(m);
            total_dias += 30;
            vista = v;
            n.mundo.auditar_estabilidade().expect("estabilidade");
        }
        let por_dia = t.elapsed().as_secs_f64() * 1000.0 / total_dias as f64;
        assert!(msgs_total.iter().all(|m| !m.starts_with("Não foi possível")), "{msgs_total:?}");
        for v in &vista.bens {
            for p in [v.preco_mundial, v.preco_nacional] {
                assert!(p.is_finite() && p >= v.preco_base * 0.25 - 1e-6 && p <= v.preco_base * 1.75 + 1e-6, "{}: {p}", v.id);
            }
            assert!(v.hist_mundial.iter().all(|x| x.is_finite() && *x > 0.0));
        }
        for tag in ["USA", "SOV"] {
            let id = n.id_pais(tag).unwrap();
            let p = n.mundo.obter_pais(&id).unwrap();
            assert!(p.pib_diario.is_finite() && p.pib_diario > 0.0, "PIB de {tag}");
        }
        assert!(vista.pib_anual > 0.0 && vista.padrao_vida.is_finite() && vista.desemprego.is_finite());
        eprintln!(
            "5 anos: tick medio {por_dia:.1} ms/dia (incl. vista a cada 30 dias); PIB EUA {:.2e}; SoL {:.1}; desemprego {:.1}%",
            vista.pib_anual,
            vista.padrao_vida,
            vista.desemprego * 100.0
        );
    }
}

#[cfg(test)]
mod testes_plugin {
    use super::*;
    use crate::estado::AppState;
    use crate::plugins::jogador::PaisJogador;
    use crate::plugins::map::RaizProjeto;
    use bevy::state::app::StatesPlugin;
    use gf_mapa::tempo::DataSimulacao;

    /// Fluxo completo no ECS sem janela: carga em background, escolha do país, ticks diários em thread e comandos.
    #[test]
    fn plugin_carrega_roda_lotes_e_aplica_comandos() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .add_message::<EventoTickDiario>()
            .insert_resource(RaizProjeto(raiz))
            .add_plugins(EconomiaSimPlugin);
        app.update(); // entra em Carregando -> inicia a carga
        let t = Instant::now();
        while app.world().get_resource::<EconomiaSim>().is_none() {
            assert!(t.elapsed().as_secs() < 120, "carga demorou demais");
            std::thread::sleep(std::time::Duration::from_millis(50));
            app.update();
        }
        eprintln!("carga + aquecimento: {:?}", t.elapsed());
        app.world_mut().insert_resource(PaisJogador("USA".into()));
        for _ in 0..3 {
            app.update();
        }
        // 45 dias de jogo disparados de uma vez (mais de um lote)
        let data = DataSimulacao::default();
        for i in 0..45u64 {
            app.world_mut().write_message(EventoTickDiario { nova_data: data, total_ticks: i });
        }
        app.world_mut().resource_mut::<EconomiaSim>().enviar(Comando::AjustarTarifa(0.02));
        let t = Instant::now();
        loop {
            app.update();
            let e = app.world().resource::<EconomiaSim>();
            if e.pendentes == 0 && e.tarefa.is_none() && e.vista.dias >= DIAS_AQUECIMENTO as u64 + 45 {
                break;
            }
            assert!(t.elapsed().as_secs() < 60, "economia não avançou");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let e = app.world().resource::<EconomiaSim>();
        assert_eq!(e.pais_id.as_deref(), Some("usa"));
        assert!(e.vista.pais.is_some() && !e.vista.bens.is_empty() && e.ms_por_dia > 0.0);
        assert!(e.mensagens.iter().any(|m| m.contains("Tarifa")), "{:?}", e.mensagens);
        eprintln!("45 dias em {:?}; {:.1} ms/dia", t.elapsed(), e.ms_por_dia);
    }

    /// Teste headless de carga inicial: carrega os arquivos de cenário de 1945 como no main() e valida que tudo carrega sem pânico.
    #[test]
    fn jogo_carga_inicial_cenario_real_sem_panico() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (_catalogo_legado, mundo) = simulacao::mundo::MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("falha ao carregar o cenário legado de 1945");
        assert!(!mundo.paises.is_empty(), "paises não podem ser vazios");

        let catalogo: gf_mapa::CatalogoProvincias = ron::from_str(
            &std::fs::read_to_string(raiz.join("assets/map/provinces.ron")).expect("provinces.ron"),
        )
        .expect("provinces.ron inválido");
        assert!(!catalogo.provincias.is_empty(), "catálogo de províncias vazio");
        // Dono e controlador por província, inicializados do dono do estado.
        assert_eq!(mundo.dono_por_provincia.len(), catalogo.provincias.len());
        assert_eq!(mundo.controlador_por_provincia, mundo.dono_por_provincia);

        // Carga da economia de 1945 pelo mesmo caminho do jogo
        let dir = dir_economia(&raiz);
        let nucleo = Nucleo::carregar(&dir, 0).expect("economia 1945 deve carregar sem pânico");
        assert!(!nucleo.mundo.paises().is_empty());
        assert!(!nucleo.mundo.provincias().is_empty());
    }
}
