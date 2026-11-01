//! Diplomacia da Guerra Fria: relações (-100..+100), tensão mundial, esferas de influência,
//! ações diplomáticas com aceitação da IA, pactos e organizações com gatilho dinâmico.
//! Crate puro (sem Bevy). Fórmulas de `DIPLOMACIA.md`.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const RELACOES_1945: &str = include_str!("../dados/relacoes_iniciais_1945.ron");

#[derive(Deserialize)]
struct ParRelacao {
    a: String,
    b: String,
    valor: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Bloco {
    Ocidental,
    Sovietico,
    NaoAlinhado,
}

/// Esfera de influência de um país.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Esfera {
    Eua,
    Urss,
    NaoAlinhado,
    Disputada,
}

/// Dados mínimos de um país vindos do cenário.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfoPais {
    pub tag: String,
    pub bloco: Bloco,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Acao {
    MelhorarRelacoes,
    AjudaEconomica,
    AcordoComercial,
    PactoDefesa,
    Alianca,
    Embargo,
    Rivalizar,
}

impl Acao {
    pub const TODAS: [Acao; 7] = [
        Acao::MelhorarRelacoes,
        Acao::AjudaEconomica,
        Acao::AcordoComercial,
        Acao::PactoDefesa,
        Acao::Alianca,
        Acao::Embargo,
        Acao::Rivalizar,
    ];
    pub fn nome(self) -> &'static str {
        match self {
            Acao::MelhorarRelacoes => "Melhorar relações",
            Acao::AjudaEconomica => "Ajuda econômica",
            Acao::AcordoComercial => "Acordo comercial",
            Acao::PactoDefesa => "Pacto de defesa",
            Acao::Alianca => "Aliança",
            Acao::Embargo => "Embargo",
            Acao::Rivalizar => "Rivalidade",
        }
    }
    /// Custo em pontos de influência diplomática.
    pub fn custo_pontos(self) -> f32 {
        match self {
            Acao::MelhorarRelacoes => 10.0,
            Acao::AjudaEconomica => 35.0,
            Acao::AcordoComercial => 20.0,
            Acao::PactoDefesa => 40.0,
            Acao::Alianca => 60.0,
            Acao::Embargo => 25.0,
            Acao::Rivalizar => 15.0,
        }
    }
    /// Custo em tesouro (milhões).
    pub fn custo_tesouro(self) -> f64 {
        if self == Acao::AjudaEconomica { 150.0 } else { 0.0 }
    }
    fn faixa_relacao(self) -> (f32, f32) {
        match self {
            Acao::MelhorarRelacoes => (-60.0, 90.0),
            Acao::AjudaEconomica => (-30.0, 100.0),
            Acao::AcordoComercial => (-10.0, 100.0),
            Acao::PactoDefesa => (30.0, 100.0),
            Acao::Alianca => (50.0, 100.0),
            Acao::Embargo => (-100.0, 30.0),
            Acao::Rivalizar => (-100.0, 40.0),
        }
    }
    /// (base, k_relacao, k_alinhamento, k_ameaca, k_economia, k_ideologia); `None` = sem aceitação.
    fn aceitacao(self) -> Option<[f32; 6]> {
        match self {
            Acao::AjudaEconomica => Some([5.0, 0.20, 6.0, 0.0, 25.0, 0.1]),
            Acao::AcordoComercial => Some([-10.0, 0.35, 8.0, 0.1, 15.0, 0.1]),
            Acao::PactoDefesa => Some([-30.0, 0.45, 18.0, 0.6, 5.0, 0.2]),
            Acao::Alianca => Some([-45.0, 0.55, 22.0, 0.7, 5.0, 0.3]),
            _ => None,
        }
    }
    fn cooldown_dias(self) -> u32 {
        match self {
            Acao::MelhorarRelacoes => 30,
            Acao::AjudaEconomica => 60,
            Acao::AcordoComercial => 180,
            Acao::PactoDefesa => 730,
            Acao::Alianca => 1095,
            Acao::Embargo => 180,
            Acao::Rivalizar => 365,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TipoPacto {
    AcordoComercial,
    PactoDefesa,
    Alianca,
    Embargo,
    Rivalidade,
}

impl TipoPacto {
    pub fn nome(self) -> &'static str {
        match self {
            TipoPacto::AcordoComercial => "Acordo comercial",
            TipoPacto::PactoDefesa => "Pacto de defesa",
            TipoPacto::Alianca => "Aliança",
            TipoPacto::Embargo => "Embargo",
            TipoPacto::Rivalidade => "Rivalidade",
        }
    }
}

/// Pacto de `a` para `b` (simétrico, exceto embargo/rivalidade, que têm autor).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pacto {
    pub tipo: TipoPacto,
    pub a: String,
    pub b: String,
}

#[derive(Clone, Debug)]
pub struct Organizacao {
    pub id: &'static str,
    pub nome: &'static str,
    pub lider: Option<&'static str>,
    /// Data mínima (ano, mês) de formação.
    pub data_min: (i32, u8),
    pub tensao_min: f32,
    pub prob_mensal: f32,
    pub candidatos: Vec<&'static str>,
    pub formada: Option<(i32, u8)>,
    pub membros: BTreeSet<String>,
}

fn org(id: &'static str, nome: &'static str, lider: Option<&'static str>, data_min: (i32, u8), tensao_min: f32, prob: f32, cand: &[&'static str]) -> Organizacao {
    Organizacao { id, nome, lider, data_min, tensao_min, prob_mensal: prob, candidatos: cand.to_vec(), formada: None, membros: BTreeSet::new() }
}

fn catalogo_orgs() -> Vec<Organizacao> {
    vec![
        org("onu", "Organização das Nações Unidas", None, (1945, 10), 0.0, 1.0, &[
            "USA", "SOV", "GBR", "FRA", "CHI", "CAN", "AST", "NZL", "SAF", "IND", "BRA", "ARG", "MEX", "CHL", "COL", "VEN", "PER", "BOL", "PAR", "URU", "ECU", "CUB", "HAI", "DOM", "GUA", "SLV", "HON", "NIC", "CRI", "PAN", "BEL", "NLD", "LUX", "NOR", "DEN", "GRE", "TUR", "IRN", "IRQ", "SAU", "SYR", "LEB", "EGY", "ETH", "LBR", "YUG", "POL", "CZE", "PHI",
        ]),
        org("plano_marshall", "Plano Marshall", Some("USA"), (1948, 1), 15.0, 0.30, &[
            "GBR", "FRA", "BEL", "NLD", "LUX", "ITA", "GRE", "TUR", "AUT", "NOR", "DEN", "SWE", "ISL", "IRL", "POR", "CHE", "GER",
        ]),
        org("otan", "OTAN", Some("USA"), (1948, 7), 25.0, 0.40, &["USA", "GBR", "FRA", "CAN", "BEL", "NLD", "LUX", "NOR", "DEN", "ISL", "ITA", "POR"]),
        org("comecon", "COMECON", Some("SOV"), (1948, 6), 18.0, 0.30, &["SOV", "POL", "CZE", "HUN", "ROM", "BUL", "BGR", "ALB"]),
        org("pacto_varsovia", "Pacto de Varsóvia", Some("SOV"), (1955, 3), 22.0, 0.40, &["SOV", "POL", "CZE", "HUN", "ROM", "BUL", "BGR", "ALB"]),
        org("mna", "Movimento Não Alinhado", Some("IND"), (1961, 6), 30.0, 0.35, &["IND", "YUG", "EGY", "INS", "GHA", "IDN", "BUR", "CEY"]),
    ]
}

/// Resultado de uma ação diplomática.
#[derive(Clone, Debug)]
pub struct Resultado {
    pub aceita: bool,
    pub mensagem: String,
    pub custo_tesouro: f64,
    /// A UI deve assinar o acordo comercial na economia.
    pub acordo_comercial: bool,
    pub detalhe_aceitacao: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct Diplomacia {
    pub paises: Vec<InfoPais>,
    relacoes: HashMap<(String, String), f32>,
    /// influência (EUA, URSS) em cada país, 0..100.
    influencia: HashMap<String, (f32, f32)>,
    pub pontos: HashMap<String, f32>,
    pub infamia: HashMap<String, f32>,
    pub tesouro_ia: HashMap<String, f64>,
    pub tensao: f32,
    pub pactos: Vec<Pacto>,
    pub orgs: Vec<Organizacao>,
    pub eventos: BTreeSet<&'static str>,
    cooldowns: HashMap<(String, String, Acao), i64>,
    pub dia: i64,
    pub log: Vec<String>,
}

fn chave(a: &str, b: &str) -> (String, String) {
    if a <= b { (a.to_string(), b.to_string()) } else { (b.to_string(), a.to_string()) }
}

fn lado(b: Bloco) -> i32 {
    match b {
        Bloco::Ocidental => 1,
        Bloco::Sovietico => -1,
        Bloco::NaoAlinhado => 0,
    }
}

impl Diplomacia {
    pub fn novo(paises: Vec<InfoPais>) -> Self {
        let mut relacoes = HashMap::new();
        let pares: Vec<ParRelacao> = ron::from_str(RELACOES_1945).expect("relacoes_iniciais_1945.ron");
        for p in pares {
            relacoes.insert(chave(&p.a, &p.b), p.valor.clamp(-100.0, 100.0));
        }
        let mut influencia = HashMap::new();
        let mut pontos = HashMap::new();
        let mut tesouro_ia = HashMap::new();
        for p in &paises {
            let i = match (p.tag.as_str(), p.bloco) {
                ("USA", _) => (100.0, 0.0),
                ("SOV", _) => (0.0, 100.0),
                (_, Bloco::Ocidental) => (55.0, 0.0),
                (_, Bloco::Sovietico) => (0.0, 55.0),
                _ => (0.0, 0.0),
            };
            influencia.insert(p.tag.clone(), i);
            pontos.insert(p.tag.clone(), 60.0);
        }
        tesouro_ia.insert("USA".to_string(), 3000.0);
        tesouro_ia.insert("SOV".to_string(), 1500.0);
        Self {
            paises,
            relacoes,
            influencia,
            pontos,
            infamia: HashMap::new(),
            tesouro_ia,
            tensao: 18.0,
            pactos: vec![],
            orgs: catalogo_orgs(),
            eventos: BTreeSet::new(),
            cooldowns: HashMap::new(),
            dia: 0,
            log: vec![],
        }
    }

    pub fn bloco(&self, tag: &str) -> Bloco {
        self.paises.iter().find(|p| p.tag == tag).map_or(Bloco::NaoAlinhado, |p| p.bloco)
    }

    /// Relação entre dois países; pares não listados usam a regra de blocos.
    pub fn relacao(&self, a: &str, b: &str) -> f32 {
        if a == b {
            return 100.0;
        }
        if let Some(v) = self.relacoes.get(&chave(a, b)) {
            return *v;
        }
        let (la, lb) = (lado(self.bloco(a)), lado(self.bloco(b)));
        if la != 0 && la == lb { 5.0 } else if la * lb < 0 { -10.0 } else { 0.0 }
    }

    pub fn definir_relacao(&mut self, a: &str, b: &str, v: f32) {
        self.relacoes.insert(chave(a, b), v.clamp(-100.0, 100.0));
    }

    fn mudar_relacao(&mut self, a: &str, b: &str, d: f32) {
        let v = self.relacao(a, b) + d;
        self.definir_relacao(a, b, v);
    }

    pub fn nivel_relacao(v: f32) -> &'static str {
        match v {
            v if v <= -61.0 => "Hostil",
            v if v <= -21.0 => "Frio",
            v if v <= 20.0 => "Neutro",
            v if v <= 60.0 => "Cordial",
            _ => "Amigável",
        }
    }

    pub fn influencia(&self, tag: &str) -> (f32, f32) {
        self.influencia.get(tag).copied().unwrap_or((0.0, 0.0))
    }

    /// Soma influência de `potencia` ("USA"/"SOV") em `tag` com cabo de guerra.
    pub fn dar_influencia(&mut self, tag: &str, potencia: &str, v: f32) {
        let e = self.influencia.entry(tag.to_string()).or_insert((0.0, 0.0));
        let (ganha, perde) = if potencia == "USA" { (&mut e.0, &mut e.1) } else { (&mut e.1, &mut e.0) };
        *ganha = (*ganha + v).clamp(0.0, 100.0);
        let excesso = *ganha + *perde - 100.0;
        if excesso > 0.0 {
            *perde = (*perde - excesso).max(0.0);
        }
    }

    fn aliado_de(&self, tag: &str, potencia: &str) -> bool {
        self.pactos.iter().any(|p| p.tipo == TipoPacto::Alianca && ((p.a == tag && p.b == potencia) || (p.b == tag && p.a == potencia)))
            || self.orgs.iter().any(|o| o.formada.is_some() && matches!(o.id, "otan" | "pacto_varsovia") && o.lider == Some(potencia) && o.membros.contains(tag))
    }

    pub fn esfera(&self, tag: &str) -> Esfera {
        if tag == "USA" || self.aliado_de(tag, "USA") {
            return Esfera::Eua;
        }
        if tag == "SOV" || self.aliado_de(tag, "SOV") {
            return Esfera::Urss;
        }
        if self.orgs.iter().any(|o| o.id == "mna" && o.formada.is_some() && o.membros.contains(tag)) {
            return Esfera::NaoAlinhado;
        }
        let (u, s) = self.influencia(tag);
        if u >= 40.0 && s >= 40.0 && (u - s).abs() < 15.0 {
            Esfera::Disputada
        } else if u >= 40.0 && u - s >= 15.0 {
            Esfera::Eua
        } else if s >= 40.0 && s - u >= 15.0 {
            Esfera::Urss
        } else {
            Esfera::NaoAlinhado
        }
    }

    pub fn tem_pacto(&self, a: &str, b: &str, t: TipoPacto) -> bool {
        self.pactos.iter().any(|p| p.tipo == t && ((p.a == a && p.b == b) || (p.a == b && p.b == a)))
    }

    pub fn pactos_de<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = &'a Pacto> {
        self.pactos.iter().filter(move |p| p.a == tag || p.b == tag)
    }

    pub fn em_org(&self, id: &str, tag: &str) -> bool {
        self.orgs.iter().any(|o| o.id == id && o.formada.is_some() && o.membros.contains(tag))
    }

    /// Aceitação da IA (fórmula 3.3). Retorna `None` se a ação não pede aceitação.
    pub fn aceitacao(&self, ator: &str, alvo: &str, acao: Acao) -> Option<f32> {
        let k = acao.aceitacao()?;
        let alin = {
            let (la, lb) = (lado(self.bloco(alvo)), lado(self.bloco(ator)));
            if la == 0 || lb == 0 { 0.0 } else { (la * lb) as f32 }
        };
        let mut a = k[0] + k[1] * self.relacao(alvo, ator) + k[2] * alin + k[3] * 2.0 + k[4] * 0.8 + k[5] * 0.0;
        a -= 0.5 * self.infamia.get(ator).copied().unwrap_or(0.0);
        let rival = if ator == "USA" || lado(self.bloco(ator)) > 0 { "SOV" } else { "USA" };
        if self.tem_pacto(alvo, rival, TipoPacto::Alianca) || self.tem_pacto(alvo, rival, TipoPacto::PactoDefesa) {
            a -= 20.0;
        }
        if self.bloco(alvo) == Bloco::NaoAlinhado && matches!(acao, Acao::Alianca | Acao::PactoDefesa) && matches!(ator, "USA" | "SOV") {
            a -= 20.0;
        }
        Some(a)
    }

    /// Verifica se a ação é possível (sem executar). Mensagem de erro em português.
    pub fn pode(&self, ator: &str, alvo: &str, acao: Acao, tesouro_ator: f64) -> Result<(), String> {
        if ator == alvo {
            return Err("Alvo inválido.".into());
        }
        if self.pontos.get(ator).copied().unwrap_or(0.0) < self.custo_efetivo(ator, acao) {
            return Err("Pontos de influência insuficientes.".into());
        }
        if tesouro_ator < acao.custo_tesouro() {
            return Err("Tesouro insuficiente.".into());
        }
        let r = self.relacao(ator, alvo);
        let (lo, hi) = acao.faixa_relacao();
        if r < lo || r > hi {
            return Err(format!("Relação ({r:.0}) fora da faixa {lo:.0}..{hi:.0}."));
        }
        if let Some(&ate) = self.cooldowns.get(&(ator.into(), alvo.into(), acao))
            && self.dia < ate
        {
            return Err(format!("Em recarga por mais {} dias.", ate - self.dia));
        }
        let tem = |t| self.tem_pacto(ator, alvo, t);
        match acao {
            Acao::AcordoComercial if tem(TipoPacto::AcordoComercial) => return Err("Acordo já existe.".into()),
            Acao::AcordoComercial if tem(TipoPacto::Embargo) || tem(TipoPacto::Rivalidade) => return Err("Embargo ou rivalidade em vigor.".into()),
            Acao::PactoDefesa | Acao::Alianca if tem(TipoPacto::Rivalidade) => return Err("Rivalidade em vigor.".into()),
            Acao::PactoDefesa if tem(TipoPacto::PactoDefesa) || tem(TipoPacto::Alianca) => return Err("Pacto já existe.".into()),
            Acao::PactoDefesa | Acao::Alianca if self.esfera(ator) != self.esfera(alvo) && !matches!(self.esfera(ator), Esfera::NaoAlinhado) => {
                return Err("Exige a mesma esfera de influência.".into());
            }
            Acao::Alianca if !tem(TipoPacto::PactoDefesa) => return Err("Exige pacto de defesa prévio.".into()),
            Acao::Alianca if tem(TipoPacto::Alianca) => return Err("Aliança já existe.".into()),
            Acao::Embargo if tem(TipoPacto::Embargo) => return Err("Embargo já imposto.".into()),
            Acao::Rivalizar if tem(TipoPacto::Rivalidade) => return Err("Já são rivais.".into()),
            _ => {}
        }
        Ok(())
    }

    pub fn custo_efetivo(&self, ator: &str, acao: Acao) -> f32 {
        acao.custo_pontos() * (1.0 + 0.01 * (self.infamia.get(ator).copied().unwrap_or(0.0) - 25.0).max(0.0))
    }

    /// Executa uma ação. O tesouro do ator é debitado pelo chamador (`custo_tesouro`).
    pub fn executar(&mut self, ator: &str, alvo: &str, acao: Acao, tesouro_ator: f64) -> Result<Resultado, String> {
        self.pode(ator, alvo, acao, tesouro_ator)?;
        let custo = self.custo_efetivo(ator, acao);
        let a = self.aceitacao(ator, alvo, acao);
        let aceita = a.is_none_or(|v| v >= 0.0);
        *self.pontos.entry(ator.into()).or_insert(0.0) -= custo;
        self.cooldowns.insert((ator.into(), alvo.into(), acao), self.dia + acao.cooldown_dias() as i64);
        let nome_alvo = alvo.to_string();
        if !aceita {
            return Ok(Resultado { aceita: false, mensagem: format!("{nome_alvo} recusou: {}.", acao.nome()), custo_tesouro: 0.0, acordo_comercial: false, detalhe_aceitacao: a });
        }
        let potencia = match ator {
            "USA" | "SOV" => Some(ator),
            _ => match self.bloco(ator) {
                Bloco::Ocidental => Some("USA"),
                Bloco::Sovietico => Some("SOV"),
                _ => None,
            },
        };
        let mut acordo = false;
        let pacto = |s: &mut Self, t| s.pactos.push(Pacto { tipo: t, a: ator.into(), b: alvo.into() });
        match acao {
            Acao::MelhorarRelacoes => self.mudar_relacao(ator, alvo, 10.0),
            Acao::AjudaEconomica => {
                self.mudar_relacao(ator, alvo, 18.0);
                if let Some(p) = potencia {
                    self.dar_influencia(alvo, p, 10.0);
                }
                *self.infamia.entry(ator.into()).or_insert(0.0) = (self.infamia.get(ator).copied().unwrap_or(0.0) - 0.5).max(0.0);
            }
            Acao::AcordoComercial => {
                self.mudar_relacao(ator, alvo, 8.0);
                if let Some(p) = potencia {
                    self.dar_influencia(alvo, p, 1.0);
                }
                pacto(self, TipoPacto::AcordoComercial);
                acordo = true;
            }
            Acao::PactoDefesa => {
                self.mudar_relacao(ator, alvo, 15.0);
                if let Some(p) = potencia {
                    self.dar_influencia(alvo, p, 8.0);
                }
                pacto(self, TipoPacto::PactoDefesa);
                self.tensao = (self.tensao + 2.0).min(100.0);
            }
            Acao::Alianca => {
                self.mudar_relacao(ator, alvo, 20.0);
                if let Some(p) = potencia {
                    self.dar_influencia(alvo, p, 12.0);
                }
                pacto(self, TipoPacto::Alianca);
                self.tensao = (self.tensao + 3.0).min(100.0);
            }
            Acao::Embargo => {
                self.mudar_relacao(ator, alvo, -30.0);
                *self.infamia.entry(ator.into()).or_insert(0.0) += 2.0;
                self.pactos.retain(|p| !(p.tipo == TipoPacto::AcordoComercial && ((p.a == ator && p.b == alvo) || (p.a == alvo && p.b == ator))));
                pacto(self, TipoPacto::Embargo);
            }
            Acao::Rivalizar => {
                self.mudar_relacao(ator, alvo, -25.0);
                pacto(self, TipoPacto::Rivalidade);
                self.tensao = (self.tensao + 1.0).min(100.0);
            }
        }
        Ok(Resultado { aceita: true, mensagem: format!("{}: {nome_alvo} aceitou/efetuado.", acao.nome()), custo_tesouro: acao.custo_tesouro(), acordo_comercial: acordo, detalhe_aceitacao: a })
    }

    fn rng(&self, sal: u64) -> f32 {
        // SplitMix64 determinístico a partir do dia.
        let mut z = (self.dia as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(sal.wrapping_mul(0xBF58_476D_1CE4_E5B9));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Avança um dia. `jogador` não é alvo da IA. Chame com a data corrente.
    pub fn tick_dia(&mut self, ano: i32, mes: u8, dia_mes: u8, jogador: Option<&str>) {
        self.dia += 1;
        // Tensão decai rumo à base 15.
        if self.tensao > 15.0 {
            self.tensao = (self.tensao - 0.02).max(15.0);
        }
        for v in self.pontos.values_mut() {
            *v = (*v + 0.5).min(150.0);
        }
        for v in self.infamia.values_mut() {
            *v = (*v - 0.05).max(0.0);
        }
        // Efeitos diários dos pactos: rivalidade e embargo corroem a relação; comércio/aliança ajudam.
        let ef: Vec<(String, String, f32)> = self
            .pactos
            .iter()
            .filter_map(|p| match p.tipo {
                TipoPacto::Rivalidade => Some((p.a.clone(), p.b.clone(), -0.1)),
                TipoPacto::Alianca => Some((p.a.clone(), p.b.clone(), 0.05)),
                TipoPacto::AcordoComercial => Some((p.a.clone(), p.b.clone(), 0.02)),
                _ => None,
            })
            .collect();
        for (a, b, d) in ef {
            self.mudar_relacao(&a, &b, d);
        }
        if dia_mes == 1 {
            self.tick_mensal(ano, mes, jogador);
        }
    }

    fn evento(&mut self, id: &'static str, quando: (i32, u8), ano: i32, mes: u8, tensao: f32) {
        if (ano, mes) >= quando && self.eventos.insert(id) {
            self.tensao = (self.tensao + tensao).min(100.0);
            self.log.push(format!("Evento: {id}"));
        }
    }

    pub fn tick_mensal(&mut self, ano: i32, mes: u8, jogador: Option<&str>) {
        self.evento("cortina_de_ferro", (1946, 3), ano, mes, 10.0);
        self.evento("crise_grecia_turquia", (1947, 3), ano, mes, 10.0);
        self.evento("bloqueio_de_berlim", (1948, 6), ano, mes, 15.0);
        self.evento("rearmamento_alemao", (1955, 3), ano, mes, 10.0);
        for t in self.tesouro_ia.values_mut() {
            *t = (*t + 200.0).min(6000.0);
        }
        self.verificar_orgs(ano, mes);
        self.ia_superpotencias(jogador);
    }

    fn verificar_orgs(&mut self, ano: i32, mes: u8) {
        for i in 0..self.orgs.len() {
            if self.orgs[i].formada.is_some() || (ano, mes) < self.orgs[i].data_min {
                continue;
            }
            let id = self.orgs[i].id;
            if self.tensao < self.orgs[i].tensao_min {
                continue;
            }
            let cond = match id {
                "onu" => true,
                "plano_marshall" => self.eventos.contains("crise_grecia_turquia"),
                "otan" => self.eventos.contains("bloqueio_de_berlim"),
                "comecon" => self.orgs.iter().any(|o| o.id == "plano_marshall" && o.formada.is_some()) && self.contagem_bloco(Bloco::Sovietico) >= 3,
                "pacto_varsovia" => self.orgs.iter().any(|o| o.id == "comecon" && o.formada.is_some()) && self.eventos.contains("rearmamento_alemao"),
                "mna" => self.contagem_bloco(Bloco::NaoAlinhado) >= 5,
                _ => false,
            };
            if !cond || self.rng(i as u64 + 1) >= self.orgs[i].prob_mensal {
                continue;
            }
            self.formar_org(i, ano, mes);
        }
    }

    fn contagem_bloco(&self, b: Bloco) -> usize {
        self.paises.iter().filter(|p| p.bloco == b).count()
    }

    fn formar_org(&mut self, i: usize, ano: i32, mes: u8) {
        let id = self.orgs[i].id;
        let lider = self.orgs[i].lider;
        let existentes: BTreeSet<&str> = self.paises.iter().map(|p| p.tag.as_str()).collect();
        let mut membros = BTreeSet::new();
        for &c in &self.orgs[i].candidatos {
            if !existentes.contains(c) {
                continue;
            }
            let b = self.bloco(c);
            let ok = match id {
                "otan" | "plano_marshall" => b != Bloco::Sovietico && lider.is_none_or(|l| c == l || self.relacao(c, l) >= 0.0 || id == "plano_marshall"),
                "comecon" | "pacto_varsovia" => b == Bloco::Sovietico || c == "SOV",
                "mna" => b == Bloco::NaoAlinhado,
                _ => true,
            };
            if ok {
                membros.insert(c.to_string());
            }
        }
        if let Some(l) = lider {
            membros.insert(l.to_string());
        }
        // Efeitos de membro: relações e influência do líder.
        let lista: Vec<String> = membros.iter().cloned().collect();
        let bonus = match id {
            "otan" | "pacto_varsovia" => 4.0,
            "onu" => 1.0,
            _ => 2.0,
        };
        for (x, a) in lista.iter().enumerate() {
            for b in &lista[x + 1..] {
                self.mudar_relacao(a, b, bonus);
            }
        }
        if let Some(l) = lider
            && (l == "USA" || l == "SOV")
        {
            let ganho = match id {
                "plano_marshall" => 8.0,
                "otan" | "pacto_varsovia" => 6.0,
                _ => 5.0,
            };
            for m in &lista {
                if m != l {
                    self.dar_influencia(m, l, ganho);
                }
            }
        }
        if matches!(id, "otan" | "pacto_varsovia" | "comecon" | "plano_marshall") {
            self.tensao = (self.tensao + 3.0).min(100.0);
        }
        if id == "comecon"
            && let Some(m) = self.orgs.iter_mut().find(|o| o.id == "plano_marshall")
        {
            m.membros.retain(|t| t != "SOV");
        }
        self.orgs[i].formada = Some((ano, mes));
        self.orgs[i].membros = membros;
        self.log.push(format!("{} formada em {mes:02}/{ano}", self.orgs[i].nome));
    }

    /// EUA e URSS disputam esferas: ajuda econômica em alvos disputados/neutros e pactos nos já próximos.
    fn ia_superpotencias(&mut self, jogador: Option<&str>) {
        for ator in ["USA", "SOV"] {
            if Some(ator) == jogador {
                continue;
            }
            let rival = if ator == "USA" { "SOV" } else { "USA" };
            let meu = |s: &Self, t: &str| {
                let (u, v) = s.influencia(t);
                if ator == "USA" { (u, v) } else { (v, u) }
            };
            // Candidatos: não aliados do rival, ainda não seguros na minha esfera.
            let mut melhor: Option<(f32, String)> = None;
            for p in &self.paises {
                let t = p.tag.as_str();
                if t == ator || t == rival || Some(t) == jogador || self.aliado_de(t, rival) || self.aliado_de(t, ator) {
                    continue;
                }
                let (m, r) = meu(self, t);
                if m >= 90.0 || self.relacao(ator, t) < -30.0 {
                    continue;
                }
                let score = r * 0.5 + (60.0 - m).max(0.0) + 0.2 * self.relacao(ator, t) + 20.0 * self.rng(t.len() as u64 + t.bytes().map(|b| b as u64).sum::<u64>());
                if melhor.as_ref().is_none_or(|(s, _)| score > *s) {
                    melhor = Some((score, t.to_string()));
                }
            }
            let Some((_, alvo)) = melhor else { continue };
            let tes = self.tesouro_ia.get(ator).copied().unwrap_or(0.0);
            // 1) Pacto/aliança quando já está na minha esfera.
            let seq = [Acao::Alianca, Acao::PactoDefesa, Acao::AjudaEconomica, Acao::AcordoComercial];
            for acao in seq {
                if self.pode(ator, &alvo, acao, tes).is_ok() {
                    if let Ok(r) = self.executar(ator, &alvo, acao, tes) {
                        if r.aceita {
                            *self.tesouro_ia.entry(ator.into()).or_insert(0.0) -= r.custo_tesouro;
                        }
                        self.log.push(format!("IA {ator} -> {alvo}: {} ({})", acao.nome(), if r.aceita { "aceita" } else { "recusada" }));
                    }
                    break;
                }
            }
        }
    }

    /// Cor/identidade para o mapa de esferas.
    pub fn cor_esfera(e: Esfera) -> [u8; 3] {
        match e {
            Esfera::Eua => [60, 120, 210],
            Esfera::Urss => [200, 50, 50],
            Esfera::NaoAlinhado => [150, 150, 150],
            Esfera::Disputada => [200, 170, 60],
        }
    }

    pub fn resumo_orgs(&self) -> BTreeMap<&'static str, Option<(i32, u8)>> {
        self.orgs.iter().map(|o| (o.id, o.formada)).collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn mundo() -> Diplomacia {
        let mut v: Vec<InfoPais> = vec![];
        let mut add = |t: &str, b| v.push(InfoPais { tag: t.into(), bloco: b });
        for t in ["USA", "GBR", "FRA", "CAN", "BEL", "NLD", "LUX", "NOR", "DEN", "ISL", "ITA", "POR", "BRA", "MEX"] {
            add(t, Bloco::Ocidental);
        }
        for t in ["SOV", "POL", "CZE", "HUN", "ROM", "BUL", "ALB"] {
            add(t, Bloco::Sovietico);
        }
        for t in ["IND", "YUG", "EGY", "SWE", "CHE", "IRL"] {
            add(t, Bloco::NaoAlinhado);
        }
        Diplomacia::novo(v)
    }

    fn simular(d: &mut Diplomacia, ate_ano: i32) {
        for ano in 1945..=ate_ano {
            for mes in 1..=12u8 {
                for dia in 1..=30u8 {
                    d.tick_dia(ano, mes, dia, Some("BRA"));
                }
            }
        }
    }

    #[test]
    fn relacoes_iniciais() {
        let d = mundo();
        assert_eq!(d.relacao("USA", "GBR"), 75.0);
        assert_eq!(d.relacao("GBR", "USA"), 75.0);
        assert_eq!(d.relacao("USA", "SOV"), 30.0);
        assert_eq!(Diplomacia::nivel_relacao(75.0), "Amigável");
        assert_eq!(Diplomacia::nivel_relacao(-70.0), "Hostil");
    }

    #[test]
    fn esferas_iniciais() {
        let d = mundo();
        assert_eq!(d.esfera("USA"), Esfera::Eua);
        assert_eq!(d.esfera("POL"), Esfera::Urss);
        assert_eq!(d.esfera("IND"), Esfera::NaoAlinhado);
    }

    #[test]
    fn aceitacao_e_ajuda() {
        let mut d = mundo();
        // Ajuda dos EUA à Índia: base 5 + 0.2*20 + 0 + 25*0.8 > 0.
        assert!(d.aceitacao("USA", "IND", Acao::AjudaEconomica).unwrap() > 0.0);
        let (u0, _) = d.influencia("IND");
        let r = d.executar("USA", "IND", Acao::AjudaEconomica, 1000.0).unwrap();
        assert!(r.aceita && r.custo_tesouro == 150.0);
        assert!(d.influencia("IND").0 > u0);
        assert!(d.relacao("USA", "IND") >= 38.0);
        // Cooldown e tesouro.
        assert!(d.pode("USA", "IND", Acao::AjudaEconomica, 1000.0).is_err());
        assert!(d.pode("USA", "GBR", Acao::AjudaEconomica, 10.0).is_err());
    }

    #[test]
    fn aliado_oposto_recusa() {
        let d = mundo();
        // URSS propondo pacto de defesa a Portugal: relação baixa, bloco oposto -> recusa.
        assert!(d.aceitacao("SOV", "POR", Acao::PactoDefesa).unwrap() < 0.0);
        // EUA a GBR aceita (relação 75, mesmo bloco).
        assert!(d.aceitacao("USA", "GBR", Acao::PactoDefesa).unwrap() >= 0.0);
    }

    #[test]
    fn escalada_de_pactos() {
        let mut d = mundo();
        assert!(d.executar("USA", "GBR", Acao::Alianca, 0.0).is_err(), "aliança exige pacto de defesa");
        assert!(d.executar("USA", "GBR", Acao::PactoDefesa, 0.0).unwrap().aceita);
        d.dia += 2000;
        for v in d.pontos.values_mut() {
            *v = 100.0;
        }
        assert!(d.executar("USA", "GBR", Acao::Alianca, 0.0).unwrap().aceita);
        assert!(d.tem_pacto("GBR", "USA", TipoPacto::Alianca));
    }

    #[test]
    fn embargo_e_rivalidade() {
        let mut d = mundo();
        let r0 = d.relacao("USA", "SOV");
        d.executar("USA", "SOV", Acao::Embargo, 0.0).unwrap();
        assert!(d.relacao("USA", "SOV") < r0);
        assert!(d.tem_pacto("USA", "SOV", TipoPacto::Embargo));
    }

    #[test]
    fn otan_e_comecon_nascem_nas_datas_plausiveis() {
        let mut d = mundo();
        simular(&mut d, 1951);
        let r = d.resumo_orgs();
        let otan = r["otan"].expect("OTAN formada");
        let com = r["comecon"].expect("COMECON formado");
        assert!((1948, 7) <= otan && otan <= (1950, 12), "otan {otan:?}");
        assert!((1948, 6) <= com && com <= (1950, 12), "comecon {com:?}");
        assert!(r["onu"].is_some() && r["onu"].unwrap().0 == 1945);
        assert!(d.em_org("otan", "GBR") && !d.em_org("otan", "POL"));
        assert!(d.em_org("comecon", "POL"));
        assert!(r["pacto_varsovia"].is_none());
    }

    #[test]
    fn varsovia_e_mna() {
        let mut d = mundo();
        simular(&mut d, 1963);
        let r = d.resumo_orgs();
        let v = r["pacto_varsovia"].expect("Varsóvia");
        assert!((1955, 3) <= v && v <= (1957, 12), "{v:?}");
        // MNA exige >= 5 não alinhados (fixture tem 6) e tensão >= 30.
        let m = r["mna"];
        assert!(m.is_none_or(|m| m.0 >= 1961));
    }

    #[test]
    fn ia_disputa_esferas() {
        let mut d = mundo();
        simular(&mut d, 1950);
        assert!(d.pactos.iter().any(|p| p.a == "USA" || p.a == "SOV"));
        let neutros: f32 = ["IND", "YUG", "EGY", "SWE", "CHE", "IRL"].iter().map(|t| d.influencia(t).0 + d.influencia(t).1).sum();
        assert!(neutros > 0.0, "a IA deve disputar países não alinhados");
    }
}

/// Parte dinâmica de uma organização internacional.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrgSalva {
    pub id: String,
    pub formada: Option<(i32, u8)>,
    pub membros: BTreeSet<String>,
}

/// Estado dinâmico da diplomacia para salvar/carregar.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiplomaciaSalva {
    pub paises: Vec<InfoPais>,
    pub relacoes: Vec<((String, String), f32)>,
    pub influencia: Vec<(String, (f32, f32))>,
    pub pontos: Vec<(String, f32)>,
    pub infamia: Vec<(String, f32)>,
    pub tesouro_ia: Vec<(String, f64)>,
    pub tensao: f32,
    pub pactos: Vec<Pacto>,
    pub orgs: Vec<OrgSalva>,
    pub eventos: Vec<String>,
    pub cooldowns: Vec<((String, String, Acao), i64)>,
    pub dia: i64,
    pub log: Vec<String>,
}

fn ordenado<K: Ord + Clone, V: Clone>(m: &HashMap<K, V>) -> Vec<(K, V)> {
    let mut v: Vec<(K, V)> = m.iter().map(|(k, x)| (k.clone(), x.clone())).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

impl Diplomacia {
    pub fn salvar_estado(&self) -> DiplomaciaSalva {
        DiplomaciaSalva {
            paises: self.paises.clone(),
            relacoes: ordenado(&self.relacoes),
            influencia: ordenado(&self.influencia),
            pontos: ordenado(&self.pontos),
            infamia: ordenado(&self.infamia),
            tesouro_ia: ordenado(&self.tesouro_ia),
            tensao: self.tensao,
            pactos: self.pactos.clone(),
            orgs: self.orgs.iter().map(|o| OrgSalva { id: o.id.to_string(), formada: o.formada, membros: o.membros.clone() }).collect(),
            eventos: self.eventos.iter().map(|e| e.to_string()).collect(),
            cooldowns: ordenado(&self.cooldowns),
            dia: self.dia,
            log: self.log.clone(),
        }
    }

    pub fn restaurar_estado(&mut self, s: DiplomaciaSalva) {
        self.paises = s.paises;
        self.relacoes = s.relacoes.into_iter().collect();
        self.influencia = s.influencia.into_iter().collect();
        self.pontos = s.pontos.into_iter().collect();
        self.infamia = s.infamia.into_iter().collect();
        self.tesouro_ia = s.tesouro_ia.into_iter().collect();
        self.tensao = s.tensao;
        self.pactos = s.pactos;
        for o in s.orgs {
            if let Some(d) = self.orgs.iter_mut().find(|x| x.id == o.id) {
                d.formada = o.formada;
                d.membros = o.membros;
            }
        }
        // Os ids de eventos são literais estáticos; reaproveita-os pelo valor (vazamento mínimo e único por carga).
        self.eventos = s.eventos.into_iter().map(|e| &*Box::leak(e.into_boxed_str())).collect();
        self.cooldowns = s.cooldowns.into_iter().collect();
        self.dia = s.dia;
        self.log = s.log;
    }
}
