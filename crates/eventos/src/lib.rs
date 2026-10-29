//! Sistema de eventos históricos (1945-1991): carrega os `.ron`, avalia gatilhos diariamente,
//! aplica efeitos no `MundoJogo` e escolhe opções para os países da IA.
use serde::Deserialize;
use simulacao::mundo::MundoJogo;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// ---------------------------------------------------------------- formato dos arquivos

#[derive(Debug, Clone, Deserialize)]
pub struct DataGatilho {
    pub ano: i32,
    pub mes: u8,
    pub dia: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Modificador {
    pub id: String,
    pub valor: f32,
}

#[derive(Debug, Clone, Deserialize)]
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

/// Condição já interpretada a partir do texto do `.ron`.
#[derive(Debug, Clone, PartialEq)]
pub enum Condicao {
    AnoMin(i32),
    PosEvento(String),
    Soberano(String),
    TemLei(String),
    /// Variável numérica do jogo comparada a um valor (`tensao_guerra_fria >= 40`).
    Compara(String, Op, f32),
    /// Sem dado no cliente para avaliar: assume verdadeira (e avisa uma vez no log).
    Desconhecida(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Ge,
    Le,
    Gt,
    Lt,
    Eq,
}

impl Condicao {
    pub fn parse(s: &str) -> Condicao {
        let s = s.trim();
        if let Some((k, v)) = s.split_once(':') {
            let v = v.trim().to_string();
            return match k.trim() {
                "pos_evento" => Condicao::PosEvento(v),
                "soberano" => Condicao::Soberano(v),
                "tem_lei" => Condicao::TemLei(v),
                _ => Condicao::Desconhecida(s.to_string()),
            };
        }
        for (txt, op) in [(">=", Op::Ge), ("<=", Op::Le), ("==", Op::Eq), (">", Op::Gt), ("<", Op::Lt)] {
            if let Some((a, b)) = s.split_once(txt)
                && let Ok(n) = b.trim().parse::<f32>()
            {
                let a = a.trim();
                if a == "ano" {
                    if op == Op::Ge {
                        return Condicao::AnoMin(n as i32);
                    }
                }
                return Condicao::Compara(a.to_string(), op, n);
            }
        }
        Condicao::Desconhecida(s.to_string())
    }
}

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
                OpcaoEvento { id: o.id, texto: o.texto, descricao_resultado: o.resultado_historico, efeitos, ia_peso: o.ia_peso }
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

impl CatalogoEventos {
    /// Carrega os arquivos de `dir` (normalmente `assets/data/eventos`).
    pub fn carregar(dir: &Path) -> Result<Self, String> {
        let mut eventos = Vec::new();
        for arq in ARQ_HISTORICOS {
            let txt = std::fs::read_to_string(dir.join(arq)).map_err(|e| format!("{arq}: {e}"))?;
            let v: Vec<EventoHistorico> = ron::from_str(&txt).map_err(|e| format!("{arq}: {e}"))?;
            eventos.extend(v.into_iter().map(Evento::de_historico));
        }
        let txt = std::fs::read_to_string(dir.join(ARQ_MARCOS)).map_err(|e| format!("{ARQ_MARCOS}: {e}"))?;
        let v: Vec<MarcoDeEra> = ron::from_str(&txt).map_err(|e| format!("{ARQ_MARCOS}: {e}"))?;
        eventos.extend(v.into_iter().map(Evento::de_marco));
        eventos.retain(|e| !e.opcoes.is_empty());
        eventos.sort_by_key(|e| e.data);
        Ok(Self { eventos })
    }

    pub fn por_id(&self, id: &str) -> Option<&Evento> {
        self.eventos.iter().find(|e| e.id == id)
    }
}

// ---------------------------------------------------------------- estado em jogo

#[derive(Debug, Clone)]
pub struct Noticia {
    pub data: (i32, u8, u8),
    pub texto: String,
}

/// Evento aguardando decisão do jogador (o cliente pausa o tempo enquanto houver um).
#[derive(Debug, Clone)]
pub struct Pendente {
    pub evento_id: String,
    pub pais: String,
}

#[derive(Debug, Default, Clone)]
pub struct EstadoEventos {
    /// Evento -> opção escolhida (`None` enquanto o jogador não decidiu).
    pub disparados: BTreeMap<String, Option<String>>,
    pub pendentes: Vec<Pendente>,
    pub jornal: Vec<Noticia>,
    /// Modificadores acumulados por país (inclui os que o cliente ainda não consome).
    pub modificadores: BTreeMap<(String, String), f32>,
    avisados: BTreeSet<String>,
}

/// Dias de tolerância depois da data do gatilho para as condições ainda poderem ser atendidas.
const JANELA_DIAS: i64 = 180;

fn dias(d: (i32, u8, u8)) -> i64 {
    // Aproximação suficiente para comparar janelas (mês de 30,4 dias).
    d.0 as i64 * 365 + (d.1 as i64 - 1) * 30 + d.2 as i64
}

fn rolar(id: &str, d: (i32, u8, u8)) -> f32 {
    let mut h: u64 = 1469598103934665603;
    for b in id.bytes().chain(d.0.to_le_bytes()).chain([d.1, d.2]) {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    (h >> 40) as f32 / (1u64 << 24) as f32
}

fn variavel(mundo: &MundoJogo, pais: &str, nome: &str) -> Option<f32> {
    match nome {
        "tensao_guerra_fria" | "tensao_mundial" => Some(mundo.tensao_mundial),
        "legitimidade" | "estabilidade_politica" => mundo.paises.get(pais).map(|p| p.politica.legitimidade),
        "tesouro" => mundo.paises.get(pais).map(|p| p.orcamento.tesouro as f32),
        _ => None,
    }
}

impl EstadoEventos {
    fn avaliar(&mut self, mundo: &MundoJogo, alvo: &str, c: &Condicao) -> bool {
        match c {
            Condicao::AnoMin(a) => mundo.data.ano >= *a,
            Condicao::PosEvento(id) => self.disparados.contains_key(id),
            Condicao::Soberano(t) => mundo.paises.get(t).is_some_and(|p| !p.provincias_ids.is_empty()),
            Condicao::TemLei(l) => mundo.paises.get(alvo).is_some_and(|p| p.politica.leis_ativas.values().any(|v| v == l)),
            Condicao::Compara(nome, op, n) => match variavel(mundo, alvo, nome) {
                Some(v) => match op {
                    Op::Ge => v >= *n,
                    Op::Le => v <= *n,
                    Op::Gt => v > *n,
                    Op::Lt => v < *n,
                    Op::Eq => (v - n).abs() < 1e-3,
                },
                None => self.avisar_uma_vez(&format!("variavel:{nome}")),
            },
            Condicao::Desconhecida(s) => self.avisar_uma_vez(&format!("condicao:{s}")),
        }
    }

    /// Registra um aviso (uma vez por chave) e devolve `true` (condição ignorada = satisfeita).
    fn avisar_uma_vez(&mut self, chave: &str) -> bool {
        if self.avisados.insert(chave.to_string()) {
            eprintln!("[eventos] aviso: '{chave}' sem suporte no cliente, ignorado");
        }
        true
    }
}

fn pais_afetado(ev: &Evento, jogador: &str, mundo: &MundoJogo) -> String {
    if ev.paises.iter().any(|p| p == jogador) {
        return jogador.to_string();
    }
    ev.paises.iter().find(|p| mundo.paises.contains_key(*p)).cloned().unwrap_or_else(|| jogador.to_string())
}

/// Avalia os gatilhos para a data atual de `mundo`. Eventos do jogador viram `pendentes`;
/// os demais são decididos pela IA, aplicados e anunciados no jornal. Devolve os ids disparados.
pub fn avaliar_dia(cat: &CatalogoEventos, est: &mut EstadoEventos, mundo: &mut MundoJogo, jogador: &str) -> Vec<String> {
    let hoje = (mundo.data.ano, mundo.data.mes, mundo.data.dia);
    let mut disparados = Vec::new();
    for ev in &cat.eventos {
        if ev.data > hoje {
            break; // ordenado por data
        }
        if est.disparados.contains_key(&ev.id) || dias(hoje) - dias(ev.data) > JANELA_DIAS {
            continue;
        }
        let alvo = pais_afetado(ev, jogador, mundo);
        if !ev.paises.is_empty() && !ev.paises.iter().any(|p| mundo.paises.contains_key(p)) {
            continue;
        }
        if !ev.condicoes.iter().all(|c| est.avaliar(mundo, &alvo, c)) {
            continue;
        }
        if ev.chance < 1.0 && rolar(&ev.id, hoje) > ev.chance {
            continue;
        }
        disparados.push(ev.id.clone());
        if alvo == jogador {
            est.disparados.insert(ev.id.clone(), None);
            est.pendentes.push(Pendente { evento_id: ev.id.clone(), pais: alvo });
        } else {
            let i = escolha_ia(ev, hoje);
            aplicar_opcao(mundo, est, ev, i, &alvo);
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
    let Some(op) = ev.opcoes.get(i) else { return };
    for m in &op.efeitos {
        aplicar_efeito(mundo, est, pais, m);
    }
    est.disparados.insert(ev.id.clone(), Some(op.id.clone()));
    est.pendentes.retain(|p| p.evento_id != ev.id);
    let nome = mundo.paises.get(pais).map(|p| p.nome.clone()).unwrap_or_else(|| pais.to_string());
    est.jornal.push(Noticia { data: (mundo.data.ano, mundo.data.mes, mundo.data.dia), texto: format!("{nome}: {} ({})", op.texto, ev.titulo) });
    if est.jornal.len() > 200 {
        est.jornal.remove(0);
    }
}

fn aplicar_efeito(mundo: &mut MundoJogo, est: &mut EstadoEventos, pais: &str, m: &Modificador) {
    *est.modificadores.entry((pais.to_string(), m.id.clone())).or_insert(0.0) += m.valor;
    match m.id.as_str() {
        "tensao_guerra_fria" => mundo.tensao_mundial = (mundo.tensao_mundial + m.valor * 10.0).clamp(0.0, 100.0),
        "legitimidade_base" | "estabilidade_politica" | "autoridade_estatal" | "legitimidade_internacional" => {
            let k = if m.id == "legitimidade_internacional" { 0.25 } else { 1.0 };
            if let Some(p) = mundo.paises.get_mut(pais) {
                p.politica.legitimidade = (p.politica.legitimidade + m.valor * k).clamp(0.0, 100.0);
            }
        }
        "crescimento_pib" => {
            if let Some(p) = mundo.paises.get_mut(pais) {
                p.orcamento.tesouro += p.pib_anual_estimado * m.valor as f64 * 0.1;
            }
        }
        _ => {
            if est.avisar_uma_vez(&format!("efeito:{}", m.id)) { /* guardado apenas em `modificadores` */ }
        }
    }
}

/// Texto de tooltip para um efeito (ex.: "Tensão mundial +3.5").
pub fn descrever_efeito(m: &Modificador) -> String {
    let nome = match m.id.as_str() {
        "tensao_guerra_fria" => "Tensão mundial".to_string(),
        "legitimidade_base" | "estabilidade_politica" => "Estabilidade".to_string(),
        "legitimidade_internacional" => "Legitimidade internacional".to_string(),
        "crescimento_pib" => "Tesouro (crescimento do PIB)".to_string(),
        outro => {
            let s = outro.replace('_', " ");
            let mut c = s.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        }
    };
    let v = if m.id == "tensao_guerra_fria" { m.valor * 10.0 } else { m.valor };
    format!("{nome} {v:+.2}")
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::path::PathBuf;

    fn raiz() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn catalogo() -> CatalogoEventos {
        CatalogoEventos::carregar(&raiz().join("assets/data/eventos")).expect("eventos")
    }

    fn mundo() -> MundoJogo {
        let r = raiz();
        MundoJogo::carregar_de_arquivos(
            r.join("assets/map/provinces.ron"),
            r.join("data/scenario_1945/countries_1945.ron"),
            r.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("mundo")
        .1
    }

    #[test]
    fn carrega_todos_sem_erro() {
        let c = catalogo();
        assert!(c.eventos.len() >= 300, "{}", c.eventos.len());
        let ids: BTreeSet<_> = c.eventos.iter().map(|e| &e.id).collect();
        assert_eq!(ids.len(), c.eventos.len(), "ids duplicados");
    }

    #[test]
    fn evento_dispara_na_data_certa() {
        let c = catalogo();
        let mut m = mundo();
        let mut est = EstadoEventos::default();
        m.data.ano = 1945;
        m.data.mes = 2;
        m.data.dia = 10;
        assert!(!avaliar_dia(&c, &mut est, &mut m, "USA").contains(&"conferencia_yalta_1945".to_string()));
        m.data.dia = 11;
        let d = avaliar_dia(&c, &mut est, &mut m, "USA");
        assert!(d.contains(&"conferencia_yalta_1945".to_string()));
        assert_eq!(est.pendentes.len(), 1);
        // Não repete.
        m.data.dia = 12;
        assert!(!avaliar_dia(&c, &mut est, &mut m, "USA").contains(&"conferencia_yalta_1945".to_string()));
    }

    #[test]
    fn evento_de_outro_pais_vira_noticia() {
        let c = catalogo();
        let mut m = mundo();
        let mut est = EstadoEventos::default();
        m.data.ano = 1945;
        m.data.mes = 2;
        m.data.dia = 11;
        avaliar_dia(&c, &mut est, &mut m, "BRA");
        assert!(est.pendentes.is_empty());
        assert!(!est.jornal.is_empty());
    }

    #[test]
    fn escolha_aplica_efeito() {
        let c = catalogo();
        let mut m = mundo();
        let mut est = EstadoEventos::default();
        let ev = c.por_id("conferencia_yalta_1945").unwrap();
        let t0 = m.tensao_mundial;
        let l0 = m.paises["USA"].politica.legitimidade;
        aplicar_opcao(&mut m, &mut est, ev, 1, "USA"); // intransigente: tensão +0.35 (x10) e +0.40 hostilidade
        assert!((m.tensao_mundial - (t0 + 3.5)).abs() < 1e-3);
        assert!(m.paises["USA"].politica.legitimidade <= l0 + 1e-3);
        assert_eq!(est.disparados["conferencia_yalta_1945"].as_deref(), Some("yalta_posicao_intransigente"));
        assert_eq!(est.jornal.len(), 1);
    }

    #[test]
    fn condicoes_interpretadas() {
        assert_eq!(Condicao::parse("ano >= 1950"), Condicao::AnoMin(1950));
        assert_eq!(Condicao::parse("pos_evento:x"), Condicao::PosEvento("x".into()));
        assert!(matches!(Condicao::parse("coisa_estranha"), Condicao::Desconhecida(_)));
    }
}
