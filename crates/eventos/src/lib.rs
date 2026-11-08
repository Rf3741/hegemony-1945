//! Sistema de eventos históricos e políticos sistêmicos (1945-1991):
//! carrega os `.ron`, avalia gatilhos diariamente, aplica efeitos nos motores
//! `simulacao`, `sim_politica`, `sim_economia` e `sim_diplomacia`, e escolhe opções para a IA.

pub mod condicoes;
pub mod contexto;
pub mod efeitos;
pub mod modificadores;
pub mod politicos;
pub mod rotulos;

pub use condicoes::{Condicao, Op};
pub use contexto::ContextoEfeitos;
pub use efeitos::{aplicar_efeito_contextual, categoria_da_lei};
pub use modificadores::{ModificadorAtivo, pertence_a_categoria};
pub use politicos::{CondicoesEvento, EventoPolitico, OpcaoEventoPolitico};

use serde::{Deserialize, Serialize};
use sim_politica::MundoPolitico;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// ---------------------------------------------------------------- formato dos arquivos

#[derive(Debug, Clone, Deserialize)]
pub struct DataGatilho {
    pub ano: i32,
    pub mes: u8,
    pub dia: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Modificador {
    pub id: String,
    pub valor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcaoEvento {
    pub id: String,
    pub texto: String,
    #[serde(default)]
    pub descricao_resultado: String,
    #[serde(default)]
    pub efeitos: Vec<Modificador>,
    #[serde(default)]
    pub ia_peso: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct EventoHistorico {
    id: String,
    titulo: String,
    descricao: String,
    data_gatilho: DataGatilho,
    #[serde(default)]
    paises_alvo: Vec<String>,
    #[serde(default)]
    condicoes: Vec<String>,
    opcoes: Vec<OpcaoEvento>,
    /// Probabilidade (0..1) de disparar quando as condições batem; padrão 1.
    #[serde(default)]
    chance: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct GatilhoMarco {
    ano_minimo: i32,
    #[serde(default)]
    paises_envolvidos: Vec<String>,
    #[serde(default)]
    condicoes_sistemicas: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpcaoDecisao {
    id: String,
    texto: String,
    #[serde(default)]
    resultado_historico: String,
    #[serde(default)]
    efeitos: Vec<Modificador>,
    #[serde(default)]
    ia_peso: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct MarcoDeEra {
    id: String,
    titulo: String,
    descricao: String,
    gatilho_sistemico: GatilhoMarco,
    #[serde(default)]
    efeitos_globais: Vec<Modificador>,
    #[serde(default)]
    opcoes_decisao: Vec<OpcaoDecisao>,
}

// ---------------------------------------------------------------- modelo interno

#[derive(Debug, Clone)]
pub struct Evento {
    pub id: String,
    pub titulo: String,
    pub descricao: String,
    pub data: (i32, u8, u8),
    pub paises: Vec<String>,
    pub condicoes: Vec<Condicao>,
    pub opcoes: Vec<OpcaoEvento>,
    pub chance: f32,
    pub generico: bool,
}

impl Evento {
    fn de_historico(e: EventoHistorico) -> Self {
        Evento {
            id: e.id,
            titulo: e.titulo,
            descricao: e.descricao,
            data: (e.data_gatilho.ano, e.data_gatilho.mes, e.data_gatilho.dia),
            paises: e.paises_alvo,
            condicoes: e.condicoes.iter().map(|c| Condicao::parse(c)).collect(),
            opcoes: e.opcoes,
            chance: e.chance.unwrap_or(1.0),
            generico: false,
        }
    }

    fn de_marco(m: MarcoDeEra) -> Self {
        let g = m.gatilho_sistemico;
        let opcoes = m
            .opcoes_decisao
            .into_iter()
            .map(|o| {
                let mut efeitos = m.efeitos_globais.clone();
                efeitos.extend(o.efeitos);
                OpcaoEvento {
                    id: o.id,
                    texto: o.texto,
                    descricao_resultado: o.resultado_historico,
                    efeitos,
                    ia_peso: o.ia_peso,
                }
            })
            .collect();
        Evento {
            id: m.id,
            titulo: m.titulo,
            descricao: m.descricao,
            data: (g.ano_minimo, 1, 1),
            paises: g.paises_envolvidos,
            condicoes: g.condicoes_sistemicas.iter().map(|c| Condicao::parse(c)).collect(),
            opcoes,
            chance: 1.0,
            generico: false,
        }
    }
}

/// Todos os eventos carregados, ordenados por data.
#[derive(Debug, Default, Clone)]
pub struct CatalogoEventos {
    pub eventos: Vec<Evento>,
}

const ARQ_HISTORICOS: [&str; 3] = ["eventos_1945_1950.ron", "eventos_1950_1969.ron", "eventos_1970_1991.ron"];
const ARQ_MARCOS: &str = "mudancas_de_era.ron";
const ARQ_POLITICOS: &str = "eventos_politicos.ron";

impl CatalogoEventos {
    /// Carrega os arquivos de `dir` (normalmente `assets/data/eventos`).
    pub fn carregar(dir: &Path) -> Result<Self, String> {
        let mut eventos = Vec::new();
        for arq in ARQ_HISTORICOS {
            let p = dir.join(arq);
            if p.exists() {
                let txt = std::fs::read_to_string(&p).map_err(|e| format!("{arq}: {e}"))?;
                let v: Vec<EventoHistorico> = ron::from_str(&txt).map_err(|e| format!("{arq}: {e}"))?;
                eventos.extend(v.into_iter().map(Evento::de_historico));
            }
        }
        let p_marcos = dir.join(ARQ_MARCOS);
        if p_marcos.exists() {
            let txt = std::fs::read_to_string(&p_marcos).map_err(|e| format!("{ARQ_MARCOS}: {e}"))?;
            let v: Vec<MarcoDeEra> = ron::from_str(&txt).map_err(|e| format!("{ARQ_MARCOS}: {e}"))?;
            eventos.extend(v.into_iter().map(Evento::de_marco));
        }

        // Carga dos eventos políticos sistêmicos (com fallback de caminhos do workspace/staging)
        let caminhos_politicos = [
            dir.join(ARQ_POLITICOS),
            dir.join("../politica/evolucao/eventos_politicos.ron"),
            Path::new("/home/rafael/claude/guerra-fria-staging/evolucao_ideologica/eventos_politicos.ron").to_path_buf(),
        ];
        for cp in &caminhos_politicos {
            if cp.exists() {
                if let Ok(txt) = std::fs::read_to_string(cp) {
                    match ron::from_str::<Vec<EventoPolitico>>(&txt) {
                        Ok(v) => {
                            eventos.extend(v.into_iter().map(EventoPolitico::em_evento));
                            break;
                        }
                        Err(e) => {
                            eprintln!("[eventos] erro ao carregar {}: {e}", cp.display());
                        }
                    }
                }
            }
        }

        eventos.retain(|e| !e.opcoes.is_empty());
        eventos.sort_by_key(|e| e.data);
        let mut vistos = BTreeSet::new();
        eventos.retain(|e| {
            let novo = vistos.insert(e.id.clone());
            if !novo {
                eprintln!("[eventos] aviso: id duplicado '{}' ignorado", e.id);
            }
            novo
        });
        Ok(Self { eventos })
    }

    pub fn por_id(&self, id: &str) -> Option<&Evento> {
        self.eventos.iter().find(|e| e.id == id)
    }
}

// ---------------------------------------------------------------- estado em jogo

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Noticia {
    pub data: (i32, u8, u8),
    pub texto: String,
}

/// Evento aguardando decisão do jogador (o cliente pausa o tempo enquanto houver um).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pendente {
    pub evento_id: String,
    pub pais: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EstadoEventos {
    /// Evento -> opção escolhida (`None` enquanto o jogador não decidiu).
    pub disparados: BTreeMap<String, Option<String>>,
    pub pendentes: Vec<Pendente>,
    pub jornal: Vec<Noticia>,
    /// Modificadores acumulados por país.
    pub modificadores: BTreeMap<(String, String), f32>,
    /// Modificadores temporários com duração ativa.
    pub modificadores_temporarios: Vec<ModificadorAtivo>,
    /// Flags booleanas por país.
    pub flags_pais: BTreeMap<String, BTreeSet<String>>,
    /// Variáveis numéricas customizadas por país.
    pub variaveis_pais: BTreeMap<(String, String), f32>,
    /// Relações diplomáticas bilaterais (tagA, tagB) -> -100.0..=100.0.
    pub relacoes: BTreeMap<(String, String), f32>,
    /// Registro de leis alteradas (pais, categoria, nova_lei).
    pub leis_trocadas: Vec<(String, String, String)>,
    /// Semana em que cada país recebeu o último pop-up do jogador.
    #[serde(default)]
    pub ultimo_popup_por_pais: BTreeMap<String, i64>,
    /// Quantidade de pop-ups exibidos ao jogador por semana.
    #[serde(default)]
    pub popups_por_semana: BTreeMap<i64, u8>,
    avisados: BTreeSet<String>,
}

fn chave_relacao(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

impl EstadoEventos {
    pub fn tem_flag(&self, pais: &str, flag: &str) -> bool {
        self.flags_pais.get(pais).is_some_and(|s| s.contains(flag))
    }

    pub fn definir_flag(&mut self, pais: &str, flag: &str) {
        self.flags_pais.entry(pais.to_string()).or_default().insert(flag.to_string());
    }

    pub fn remover_flag(&mut self, pais: &str, flag: &str) {
        if let Some(s) = self.flags_pais.get_mut(pais) {
            s.remove(flag);
        }
    }

    pub fn obter_variavel(&self, pais: &str, var: &str) -> f32 {
        self.variaveis_pais
            .get(&(pais.to_string(), var.to_string()))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn ajustar_variavel(&mut self, pais: &str, var: &str, delta: f32) {
        *self
            .variaveis_pais
            .entry((pais.to_string(), var.to_string()))
            .or_default() += delta;
    }

    pub fn obter_relacao(&self, a: &str, b: &str) -> f32 {
        if a == b {
            return 100.0;
        }
        self.relacoes.get(&chave_relacao(a, b)).copied().unwrap_or(0.0)
    }

    pub fn ajustar_relacao(&mut self, a: &str, b: &str, delta: f32) {
        if a == b {
            return;
        }
        let k = chave_relacao(a, b);
        let val = self.relacoes.entry(k).or_insert(0.0);
        *val = (*val + delta).clamp(-100.0, 100.0);
    }

    pub fn registrar_mudanca_lei(&mut self, pais: &str, cat: &str, lei: &str) {
        self.leis_trocadas
            .push((pais.to_string(), cat.to_string(), lei.to_string()));
    }

    pub fn adicionar_modificador_temporario(&mut self, pais: &str, id: &str, valor: f32, duracao_dias: u32) {
        self.modificadores_temporarios
            .push(ModificadorAtivo::novo(pais, id, valor, duracao_dias));
    }

    /// Avança a duração de todos os modificadores temporários e expira os concluídos.
    pub fn avancar_dias(&mut self, dias: u32) {
        for m in &mut self.modificadores_temporarios {
            m.dias_restantes = m.dias_restantes.saturating_sub(dias);
        }
        self.modificadores_temporarios.retain(|m| !m.expirado());
    }

    /// Retorna o valor agregado de modificadores temporários para um país por ID ou categoria conceitual.
    pub fn modificador_total(&self, pais: &str, id_ou_cat: &str) -> f32 {
        self.modificadores_temporarios
            .iter()
            .filter(|m| m.pais == pais && (m.id == id_ou_cat || pertence_a_categoria(&m.id, id_ou_cat)))
            .map(|m| m.valor)
            .sum()
    }

    pub fn avaliar(&mut self, mundo: &MundoJogo, alvo: &str, c: &Condicao) -> bool {
        match c {
            Condicao::AnoMin(a) => mundo.data.ano >= *a,
            Condicao::PosEvento(id) => self.disparados.contains_key(id),
            Condicao::Soberano(t) => mundo.paises.get(t).is_some_and(|p| !p.provincias_ids.is_empty()),
            Condicao::TemLei(l) => {
                mundo.paises.get(alvo).is_some_and(|p| p.politica.leis_ativas.values().any(|v| v == l))
            }
            Condicao::NaoTemLei(l) => {
                mundo.paises.get(alvo).map_or(true, |p| !p.politica.leis_ativas.values().any(|v| v == l))
            }
            Condicao::LegitimidadeMax(max_val) => {
                mundo.paises.get(alvo).map_or(0.0, |p| p.politica.legitimidade) <= *max_val
            }
            Condicao::LegitimidadeMin(min_val) => {
                mundo.paises.get(alvo).map_or(0.0, |p| p.politica.legitimidade) >= *min_val
            }
            Condicao::TemFlag(f) => self.tem_flag(alvo, f),
            Condicao::NaoTemFlag(f) => !self.tem_flag(alvo, f),
            Condicao::RelacaoMin(tag, min_val) => self.obter_relacao(alvo, tag) >= *min_val,
            Condicao::RelacaoMax(tag, max_val) => self.obter_relacao(alvo, tag) <= *max_val,
            Condicao::Alinhamento(bloco) => mundo
                .paises
                .get(alvo)
                .is_some_and(|p| chave_alinhamento(p.alinhamento_inicial) == *bloco),
            Condicao::Compara(nome, op, n) => match variavel(mundo, self, alvo, nome) {
                Some(v) => match op {
                    Op::Ge => v >= *n,
                    Op::Le => v <= *n,
                    Op::Gt => v > *n,
                    Op::Lt => v < *n,
                    Op::Eq => (v - n).abs() < 1e-3,
                },
                None => self.avisar_uma_vez(&format!("variavel:{nome}")),
            },
            Condicao::AberturaCortinaDeFerro => {
                // Abertura da Cortina de Ferro: tensão mundial baixa OU flag de desgelo definida
                mundo.tensao_mundial < 0.3 || self.tem_flag(alvo, "desgelo_cortina_ferro")
            }
            Condicao::AberturaSocialistaEmCurso => {
                // Abertura socialista em curso: país do bloco soviético com flag de reforma
                self.tem_flag(alvo, "reforma_socialista")
                    || (mundo.paises.get(alvo).is_some_and(|p| chave_alinhamento(p.alinhamento_inicial) == "bloco_sovietico")
                        && self.tem_flag(alvo, "desestalinizacao"))
            }
            Condicao::LiderAtivo(id_lider) => {
                // Líder ativo: verifica flag específica ou assume true para líderes históricos conhecidos
                self.tem_flag(alvo, &format!("lider_ativo:{id_lider}"))
                    || matches!(
                        id_lider.as_str(),
                        // Líderes que começam ativos em 1945 ou são sucessões históricas certas
                        "iosif_stalin" | "mao_zedong" | "harry_s_truman" | "clement_attlee" |
                        "charles_de_gaulle" | "konrad_adenauer" | "shigeru_yoshida" |
                        "winston_churchill" | "dwight_d_eisenhower" | "john_f_kennedy" |
                        "nikita_khrushchev" | "leonid_brezhnev" | "fidel_castro" |
                        "gamal_abdel_nasser" | "jomo_kenyatta" | "kwame_nkrumah" |
                        "sukarno" | "jawaharlal_nehru" | "getulio_vargas" |
                        "juan_domingo_peron" | "jacobo_arbenz" | "fulgencio_batista" |
                        "syngman_rhee" | "kim_il_sung" | "ho_chi_minh" |
                        "chiang_kai_shek" | "mohammad_mossadegh" | "mohammad_reza_pahlavi" |
                        "ngo_dinh_diem" | "imre_nagy" | "wladyslaw_gomulka" |
                        "carlos_castillo_armas" | "josip_broz_tito" | "zhou_enlai" |
                        "vo_nguyen_giap" | "douglas_macarthur"
                    )
            }
            Condicao::Desconhecida(s) => self.avisar_uma_vez(&format!("condicao:{s}")),
        }
    }

    /// Registra um aviso (uma vez por chave) e devolve `true` (condição ignorada = satisfeita).
    pub fn avisar_uma_vez(&mut self, chave: &str) -> bool {
        if self.avisados.insert(chave.to_string()) {
            eprintln!("[eventos] aviso: '{chave}' sem suporte no cliente, ignorado");
        }
        true
    }
}

/// Dias de tolerância depois da data do gatilho para as condições ainda poderem ser atendidas.
/// Com tolerância zero, um evento cujas condições não batessem exatamente no dia ficava perdido
/// para sempre (a história do cenário não se repete).
const JANELA_DIAS: i64 = 15;

/// Dia absoluto do calendário gregoriano (contagem de dias civis, com anos bissextos reais).
/// A aproximação anterior de "mês de 30 dias" fazia dois dias reais terem o mesmo número
/// (dia 31 e dia 1 do mês seguinte) e dois números nunca aparecerem (29 e 30 de fevereiro),
/// adiantando ou tornando inatingível o disparo de eventos datados.
fn dias(d: (i32, u8, u8)) -> i64 {
    let mut ano = i64::from(d.0);
    let (mes, dia) = (i64::from(d.1), i64::from(d.2));
    if mes <= 2 {
        ano -= 1;
    }
    let era = if ano >= 0 { ano } else { ano - 399 }.div_euclid(400);
    let yoe = ano - era * 400;
    let m = if mes > 2 { mes - 3 } else { mes + 9 };
    let doy = (153 * m + 2) / 5 + dia - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn rolar(id: &str, d: (i32, u8, u8)) -> f32 {
    let mut h: u64 = 1469598103934665603;
    for b in id.bytes().chain(d.0.to_le_bytes()).chain([d.1, d.2]) {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Chave usada nas condições `alinhamento:<chave>` do catálogo de eventos.
fn chave_alinhamento(alinhamento: simulacao::AlinhamentoGeopolitico) -> &'static str {
    match alinhamento {
        simulacao::AlinhamentoGeopolitico::BlocoOcidental => "bloco_ocidental",
        simulacao::AlinhamentoGeopolitico::BlocoSovietico => "bloco_sovietico",
        simulacao::AlinhamentoGeopolitico::NaoAlinhado => "nao_alinhado",
        simulacao::AlinhamentoGeopolitico::OcupacaoAliada => "ocupacao_aliada",
    }
}

fn variavel(mundo: &MundoJogo, est: &EstadoEventos, pais: &str, nome: &str) -> Option<f32> {
    match nome {
        "tensao_guerra_fria" | "tensao_mundial" => Some(mundo.tensao_mundial),
        "legitimidade" | "estabilidade_politica" => mundo.paises.get(pais).map(|p| p.politica.legitimidade),
        "tesouro" => mundo.paises.get(pais).map(|p| p.orcamento.tesouro as f32),
        outro => {
            if est.variaveis_pais.contains_key(&(pais.to_string(), outro.to_string())) {
                Some(est.obter_variavel(pais, outro))
            } else {
                None
            }
        }
    }
}

fn pais_afetado(ev: &Evento, jogador: &str, mundo: &MundoJogo) -> String {
    if ev.paises.iter().any(|p| p == jogador) {
        return jogador.to_string();
    }
    ev.paises
        .iter()
        .find(|p| mundo.paises.contains_key(*p))
        .cloned()
        .unwrap_or_else(|| jogador.to_string())
}

/// Avalia os gatilhos para a data atual de `mundo`.
pub fn avaliar_dia(
    cat: &CatalogoEventos,
    est: &mut EstadoEventos,
    mundo: &mut MundoJogo,
    jogador: &str,
) -> Vec<String> {
    avaliar_dia_com_contexto(cat, est, &mut ContextoEfeitos::apenas_mundo(mundo), jogador)
}

/// Avaliação diária de eventos conectada ao contexto unificado dos motores de simulação.
pub fn avaliar_dia_com_contexto(
    cat: &CatalogoEventos,
    est: &mut EstadoEventos,
    ctx: &mut ContextoEfeitos,
    jogador: &str,
) -> Vec<String> {
    // Avança 1 dia nos modificadores temporários
    est.avancar_dias(1);

    let hoje = (ctx.mundo.data.ano, ctx.mundo.data.mes, ctx.mundo.data.dia);
    let mut disparados = Vec::new();
    for ev in &cat.eventos {
        if !ev.generico && ev.data > hoje {
            break; // eventos históricos fixos são ordenados por data
        }
        if est.disparados.contains_key(&ev.id) {
            continue;
        }
        if !ev.generico && dias(hoje) - dias(ev.data) > JANELA_DIAS {
            continue;
        }
        let alvo = pais_afetado(ev, jogador, ctx.mundo);
        if !ev.paises.is_empty() && !ev.paises.iter().any(|p| ctx.mundo.paises.contains_key(p)) {
            continue;
        }
        if !ev.condicoes.iter().all(|c| est.avaliar(ctx.mundo, ctx.politica.as_deref_mut(), &alvo, c)) {
            continue;
        }
        if ev.chance < 1.0 && rolar(&ev.id, hoje) > ev.chance {
            continue;
        }
        disparados.push(ev.id.clone());
        if alvo == jogador {
            let semana = dias(hoje).div_euclid(7);
            let exibidos = est.popups_por_semana.get(&semana).copied().unwrap_or(0);
            let cooldown_liberado = est.ultimo_popup_por_pais.get(&alvo).copied() != Some(semana);
            if exibidos < 2 && cooldown_liberado {
                est.disparados.insert(ev.id.clone(), None);
                est.pendentes.push(Pendente { evento_id: ev.id.clone(), pais: alvo.clone() });
                est.popups_por_semana.insert(semana, exibidos + 1);
                est.ultimo_popup_por_pais.insert(alvo, semana);
            } else {
                // Eventos excedentes continuam históricos, mas entram no Jornal sem interromper o jogo.
                let i = escolha_ia(ev, hoje);
                aplicar_opcao_com_contexto(ctx, est, ev, i, &alvo);
            }
        } else {
            let i = escolha_ia(ev, hoje);
            aplicar_opcao_com_contexto(ctx, est, ev, i, &alvo);
        }
    }
    disparados
}

/// Escolha ponderada (determinística) da IA entre as opções, usando `ia_peso`.
pub fn escolha_ia(ev: &Evento, data: (i32, u8, u8)) -> usize {
    let total: u32 = ev.opcoes.iter().map(|o| o.ia_peso).sum();
    if total == 0 {
        return 0;
    }
    let mut alvo = (rolar(&ev.id, data) * total as f32) as u32;
    for (i, o) in ev.opcoes.iter().enumerate() {
        if alvo < o.ia_peso {
            return i;
        }
        alvo -= o.ia_peso;
    }
    0
}

/// Aplica a opção `i` do evento ao `pais`, registra a escolha e publica a notícia.
pub fn aplicar_opcao(mundo: &mut MundoJogo, est: &mut EstadoEventos, ev: &Evento, i: usize, pais: &str) {
    aplicar_opcao_com_contexto(&mut ContextoEfeitos::apenas_mundo(mundo), est, ev, i, pais);
}

/// Aplica a opção `i` do evento enriquecida com o contexto de todos os subsistemas integrados.
pub fn aplicar_opcao_com_contexto(
    ctx: &mut ContextoEfeitos,
    est: &mut EstadoEventos,
    ev: &Evento,
    i: usize,
    pais: &str,
) {
    let Some(op) = ev.opcoes.get(i) else { return };
    for m in &op.efeitos {
        aplicar_efeito_contextual(ctx, est, pais, m);
    }
    est.disparados.insert(ev.id.clone(), Some(op.id.clone()));
    est.pendentes.retain(|p| p.evento_id != ev.id);
    let nome = ctx
        .mundo
        .paises
        .get(pais)
        .map(|p| p.nome.clone())
        .unwrap_or_else(|| pais.to_string());
    est.jornal.push(Noticia {
        data: (ctx.mundo.data.ano, ctx.mundo.data.mes, ctx.mundo.data.dia),
        texto: format!("{nome}: {} ({})", op.texto, ev.titulo),
    });
    if est.jornal.len() > 200 {
        est.jornal.remove(0);
    }
}

/// Texto de tooltip para um efeito (ex.: "Tensão da Guerra Fria +3,5").
pub fn descrever_efeito(m: &Modificador) -> String {
    rotulos::descrever(m)
}
