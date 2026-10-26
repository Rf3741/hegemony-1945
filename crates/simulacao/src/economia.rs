use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Identificadores tipados dos 26 bens da economia da Guerra Fria (conforme `DESIGN.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BemId {
    Graos,
    CarnePescado,
    Tecidos,
    Madeira,
    Borracha,
    Carvao,
    Ferro,
    Cobre,
    Bauxita,
    Petroleo,
    Uranio,
    Aco,
    Aluminio,
    Quimicos,
    Combustivel,
    Ferramentas,
    Cimento,
    Eletricidade,
    Transporte,
    AlimentosProc,
    Roupas,
    Moveis,
    Eletrodomesticos,
    Automoveis,
    Servicos,
    ArmasMunicoes,
}

impl BemId {
    pub const TODOS: [Self; 26] = [
        Self::Graos,
        Self::CarnePescado,
        Self::Tecidos,
        Self::Madeira,
        Self::Borracha,
        Self::Carvao,
        Self::Ferro,
        Self::Cobre,
        Self::Bauxita,
        Self::Petroleo,
        Self::Uranio,
        Self::Aco,
        Self::Aluminio,
        Self::Quimicos,
        Self::Combustivel,
        Self::Ferramentas,
        Self::Cimento,
        Self::Eletricidade,
        Self::Transporte,
        Self::AlimentosProc,
        Self::Roupas,
        Self::Moveis,
        Self::Eletrodomesticos,
        Self::Automoveis,
        Self::Servicos,
        Self::ArmasMunicoes,
    ];

    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::Graos => "Grãos e Cereais",
            Self::CarnePescado => "Carne e Pescado",
            Self::Tecidos => "Algodão e Fibras",
            Self::Madeira => "Madeira",
            Self::Borracha => "Borracha",
            Self::Carvao => "Carvão Mineral",
            Self::Ferro => "Minério de Ferro",
            Self::Cobre => "Cobre",
            Self::Bauxita => "Bauxita",
            Self::Petroleo => "Petróleo Cru",
            Self::Uranio => "Minério de Urânio",
            Self::Aco => "Aço",
            Self::Aluminio => "Alumínio e Ligas",
            Self::Quimicos => "Produtos Químicos",
            Self::Combustivel => "Combustível Refinado",
            Self::Ferramentas => "Ferramentas e Máquinas",
            Self::Cimento => "Cimento e Materiais",
            Self::Eletricidade => "Eletricidade",
            Self::Transporte => "Transporte",
            Self::AlimentosProc => "Alimentos Processados",
            Self::Roupas => "Roupas e Calçados",
            Self::Moveis => "Móveis e Utensílios",
            Self::Eletrodomesticos => "Rádios e Eletrodomésticos",
            Self::Automoveis => "Automóveis e Caminhões",
            Self::Servicos => "Serviços Urbanos",
            Self::ArmasMunicoes => "Armamentos e Munições",
        }
    }

    /// Preço base de referência ($P_0$) conforme `DESIGN.md`.
    pub const fn preco_base(self) -> f64 {
        match self {
            Self::Graos => 20.0,
            Self::CarnePescado => 30.0,
            Self::Tecidos => 20.0,
            Self::Madeira => 20.0,
            Self::Borracha => 40.0,
            Self::Carvao => 30.0,
            Self::Ferro => 40.0,
            Self::Cobre => 45.0,
            Self::Bauxita => 45.0,
            Self::Petroleo => 50.0,
            Self::Uranio => 100.0,
            Self::Aco => 50.0,
            Self::Aluminio => 65.0,
            Self::Quimicos => 50.0,
            Self::Combustivel => 60.0,
            Self::Ferramentas => 55.0,
            Self::Cimento => 35.0,
            Self::Eletricidade => 30.0,
            Self::Transporte => 30.0,
            Self::AlimentosProc => 35.0,
            Self::Roupas => 35.0,
            Self::Moveis => 40.0,
            Self::Eletrodomesticos => 70.0,
            Self::Automoveis => 100.0,
            Self::Servicos => 30.0,
            Self::ArmasMunicoes => 65.0,
        }
    }
}

/// Estrato socioeconômico das profissões.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Estrato {
    Baixo,
    Medio,
    Alto,
}

/// As 12 profissões estratificadas conforme `DESIGN.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profissao {
    Camponeses,
    TrabalhadoresRurais,
    Operarios,
    Soldados,
    Maquinistas,
    Escriturarios,
    Burocratas,
    Academicos,
    Clerigos,
    Oficiais,
    Aristocratas,
    Capitalistas,
}

impl Profissao {
    pub const fn estrato(self) -> Estrato {
        match self {
            Self::Camponeses
            | Self::TrabalhadoresRurais
            | Self::Operarios
            | Self::Soldados => Estrato::Baixo,
            Self::Maquinistas
            | Self::Escriturarios
            | Self::Burocratas
            | Self::Academicos
            | Self::Clerigos
            | Self::Oficiais => Estrato::Medio,
            Self::Aristocratas | Self::Capitalistas => Estrato::Alto,
        }
    }

    pub const fn multiplicador_salarial(self) -> f64 {
        match self {
            Self::Camponeses => 0.4,
            Self::TrabalhadoresRurais => 1.0,
            Self::Operarios => 1.3,
            Self::Soldados => 1.2,
            Self::Maquinistas => 2.0,
            Self::Escriturarios => 1.8,
            Self::Burocratas => 2.4,
            Self::Academicos => 3.0,
            Self::Clerigos => 1.8,
            Self::Oficiais => 3.2,
            Self::Aristocratas => 5.0,
            Self::Capitalistas => 6.0,
        }
    }

    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::Camponeses => "Camponeses",
            Self::TrabalhadoresRurais => "Trabalhadores Rurais",
            Self::Operarios => "Operários",
            Self::Soldados => "Soldados",
            Self::Maquinistas => "Técnicos e Maquinistas",
            Self::Escriturarios => "Escriturários",
            Self::Burocratas => "Burocratas",
            Self::Academicos => "Acadêmicos e Engenheiros",
            Self::Clerigos => "Clérigos",
            Self::Oficiais => "Oficiais Militares",
            Self::Aristocratas => "Latifundiários",
            Self::Capitalistas => "Capitalistas / Diretores",
        }
    }
}

/// Coorte populacional homogênea por `(provincia_id, profissao, cultura, religiao)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pop {
    pub provincia_id: u32,
    pub profissao: Profissao,
    pub cultura: String,
    pub religiao: String,
    pub ativos: u64,
    pub dependentes: u64,
    pub alfabetizacao: f32,
    pub padrao_vida: f32,
    pub riqueza: f64,
    pub radicais: u64,
    pub leais: u64,
}

impl Pop {
    pub const fn total_habitantes(&self) -> u64 {
        self.ativos.saturating_add(self.dependentes)
    }

    /// Taxa anual de crescimento vegetativo baseada na transição demográfica (Seção 4.1 do `DESIGN.md`).
    pub fn taxa_crescimento_anual(&self, bonus_saude: f64) -> f64 {
        let sol = self.padrao_vida as f64;
        if sol < 8.0 {
            // Crise alimentar / miséria aguda
            -0.020 + 0.0025 * sol + bonus_saude * 0.5
        } else if sol <= 22.0 {
            // Transição demográfica e Baby Boom pós-1945
            let base = 0.014 + 0.0008 * (22.0 - (sol - 16.0).abs());
            (base + bonus_saude - 0.004 * (self.alfabetizacao as f64)).clamp(0.002, 0.032)
        } else {
            // Sociedade urbano-industrial madura
            let base = 0.012 - 0.0002 * (sol - 22.0) - 0.004 * (self.alfabetizacao as f64);
            (base + bonus_saude * 0.5).clamp(0.001, 0.020)
        }
    }
}

/// Tipos de prédios produtivos, governamentais e de infraestrutura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoPredio {
    FazendaSubsistencia,
    Fazenda,
    Plantacao,
    Serraria,
    PortoPesqueiro,
    MinaCarvao,
    MinaFerro,
    MinaMetais,
    MinaUranio,
    CampoPetrolifero,
    Siderurgica,
    RefinariaQuimica,
    FabricaMaquinas,
    IndustriaAlimentos,
    IndustriaConsumo,
    IndustriaEletroAuto,
    IndustriaBelica,
    UsinaEletrica,
    FerroviaPorto,
    CentroUrbano,
    Administracao,
    Quartel,
    SetorConstrucao,
}

impl TipoPredio {
    pub const TRABALHADORES_POR_NIVEL: u64 = 5_000;

    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::FazendaSubsistencia => "Agricultura de Subsistência",
            Self::Fazenda => "Fazenda Comercial",
            Self::Plantacao => "Plantação (Fibras/Borracha)",
            Self::Serraria => "Serraria",
            Self::PortoPesqueiro => "Porto Pesqueiro",
            Self::MinaCarvao => "Mina de Carvão",
            Self::MinaFerro => "Mina de Ferro",
            Self::MinaMetais => "Mina de Cobre e Bauxita",
            Self::MinaUranio => "Mina de Urânio",
            Self::CampoPetrolifero => "Campo Petrolífero",
            Self::Siderurgica => "Siderúrgica e Metalurgia",
            Self::RefinariaQuimica => "Refinaria e Indústria Química",
            Self::FabricaMaquinas => "Fábrica de Ferramentas e Cimento",
            Self::IndustriaAlimentos => "Indústria Alimentícia",
            Self::IndustriaConsumo => "Indústria Têxtil e Moveleira",
            Self::IndustriaEletroAuto => "Indústria Automotiva e Eletrônica",
            Self::IndustriaBelica => "Indústria de Armamentos",
            Self::UsinaEletrica => "Usina Elétrica",
            Self::FerroviaPorto => "Ferrovia e Porto Comercial",
            Self::CentroUrbano => "Centro Urbano",
            Self::Administracao => "Administração Governamental",
            Self::Quartel => "Quartel e Base Militar",
            Self::SetorConstrucao => "Setor de Construção",
        }
    }

    pub const fn eh_governamental(self) -> bool {
        matches!(self, Self::Administracao | Self::Quartel | Self::SetorConstrucao)
    }

    /// Insumos consumidos por nível em plena ocupação.
    pub fn insumos_por_nivel(self) -> &'static [(BemId, f64)] {
        match self {
            Self::FazendaSubsistencia => &[],
            Self::Fazenda => &[(BemId::Ferramentas, 5.0), (BemId::Quimicos, 5.0)],
            Self::Plantacao => &[(BemId::Ferramentas, 5.0)],
            Self::Serraria => &[(BemId::Ferramentas, 5.0)],
            Self::PortoPesqueiro => &[(BemId::Madeira, 5.0), (BemId::Ferramentas, 3.0)],
            Self::MinaCarvao => &[(BemId::Ferramentas, 10.0)],
            Self::MinaFerro => &[(BemId::Ferramentas, 10.0), (BemId::Carvao, 5.0)],
            Self::MinaMetais => &[(BemId::Ferramentas, 12.0), (BemId::Carvao, 5.0)],
            Self::MinaUranio => &[
                (BemId::Ferramentas, 15.0),
                (BemId::Quimicos, 10.0),
                (BemId::Eletricidade, 10.0),
            ],
            Self::CampoPetrolifero => &[(BemId::Ferramentas, 15.0), (BemId::Aco, 5.0)],
            Self::Siderurgica => &[
                (BemId::Ferro, 30.0),
                (BemId::Carvao, 25.0),
                (BemId::Bauxita, 10.0),
                (BemId::Eletricidade, 10.0),
            ],
            Self::RefinariaQuimica => &[
                (BemId::Petroleo, 35.0),
                (BemId::Carvao, 15.0),
                (BemId::Eletricidade, 10.0),
            ],
            Self::FabricaMaquinas => &[
                (BemId::Aco, 25.0),
                (BemId::Madeira, 15.0),
                (BemId::Carvao, 10.0),
            ],
            Self::IndustriaAlimentos => &[
                (BemId::Graos, 25.0),
                (BemId::CarnePescado, 15.0),
                (BemId::Ferramentas, 5.0),
            ],
            Self::IndustriaConsumo => &[
                (BemId::Tecidos, 25.0),
                (BemId::Madeira, 20.0),
                (BemId::Ferramentas, 5.0),
            ],
            Self::IndustriaEletroAuto => &[
                (BemId::Aco, 20.0),
                (BemId::Aluminio, 10.0),
                (BemId::Borracha, 12.0),
                (BemId::Cobre, 12.0),
                (BemId::Combustivel, 8.0),
            ],
            Self::IndustriaBelica => &[
                (BemId::Aco, 22.0),
                (BemId::Quimicos, 18.0),
                (BemId::Cobre, 10.0),
                (BemId::Ferramentas, 8.0),
            ],
            Self::UsinaEletrica => &[(BemId::Carvao, 20.0), (BemId::Ferramentas, 8.0)],
            Self::FerroviaPorto => &[
                (BemId::Aco, 10.0),
                (BemId::Combustivel, 12.0),
                (BemId::Ferramentas, 6.0),
            ],
            Self::CentroUrbano => &[(BemId::Eletricidade, 8.0), (BemId::Carvao, 6.0)],
            Self::Administracao => &[(BemId::Madeira, 8.0), (BemId::Servicos, 5.0)],
            Self::Quartel => &[
                (BemId::ArmasMunicoes, 12.0),
                (BemId::Combustivel, 6.0),
                (BemId::AlimentosProc, 8.0),
            ],
            Self::SetorConstrucao => &[
                (BemId::Cimento, 20.0),
                (BemId::Aco, 15.0),
                (BemId::Madeira, 15.0),
                (BemId::Ferramentas, 10.0),
            ],
        }
    }

    /// Bens produzidos por nível em plena ocupação.
    pub fn saidas_por_nivel(self) -> &'static [(BemId, f64)] {
        match self {
            Self::FazendaSubsistencia => &[
                (BemId::Graos, 12.0),
                (BemId::Tecidos, 4.0),
                (BemId::Servicos, 2.0),
            ],
            Self::Fazenda => &[(BemId::Graos, 40.0), (BemId::CarnePescado, 15.0)],
            Self::Plantacao => &[(BemId::Tecidos, 30.0), (BemId::Borracha, 15.0)],
            Self::Serraria => &[(BemId::Madeira, 45.0)],
            Self::PortoPesqueiro => &[(BemId::CarnePescado, 35.0)],
            Self::MinaCarvao => &[(BemId::Carvao, 50.0)],
            Self::MinaFerro => &[(BemId::Ferro, 42.0)],
            Self::MinaMetais => &[(BemId::Cobre, 24.0), (BemId::Bauxita, 20.0)],
            Self::MinaUranio => &[(BemId::Uranio, 15.0)],
            Self::CampoPetrolifero => &[(BemId::Petroleo, 50.0)],
            Self::Siderurgica => &[(BemId::Aco, 50.0), (BemId::Aluminio, 18.0)],
            Self::RefinariaQuimica => &[(BemId::Combustivel, 42.0), (BemId::Quimicos, 35.0)],
            Self::FabricaMaquinas => &[(BemId::Ferramentas, 45.0), (BemId::Cimento, 35.0)],
            Self::IndustriaAlimentos => &[(BemId::AlimentosProc, 55.0)],
            Self::IndustriaConsumo => &[(BemId::Roupas, 40.0), (BemId::Moveis, 32.0)],
            Self::IndustriaEletroAuto => &[
                (BemId::Eletrodomesticos, 28.0),
                (BemId::Automoveis, 18.0),
            ],
            Self::IndustriaBelica => &[(BemId::ArmasMunicoes, 45.0)],
            Self::UsinaEletrica => &[(BemId::Eletricidade, 75.0)],
            Self::FerroviaPorto => &[(BemId::Transporte, 60.0)],
            Self::CentroUrbano => &[(BemId::Servicos, 50.0)],
            Self::Administracao | Self::Quartel | Self::SetorConstrucao => &[],
        }
    }
}

/// Instância de um complexo produtivo ou governamental.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predio {
    pub tipo: TipoPredio,
    pub nivel: u32,
    pub empregados: u64,
    /// Fator de destruição bélica herdado da Segunda Guerra (`0.0..=0.85`).
    pub destruicao_guerra: f32,
    pub reserva_caixa: f64,
    pub ultimo_lucro_diario: f64,
}

impl Predio {
    pub fn novo(tipo: TipoPredio, nivel: u32, destruicao_guerra: f32) -> Self {
        let nivel_util = nivel.max(1);
        let empregados = (nivel_util as u64) * TipoPredio::TRABALHADORES_POR_NIVEL;
        Self {
            tipo,
            nivel: nivel_util,
            empregados,
            destruicao_guerra: destruicao_guerra.clamp(0.0, 0.85),
            reserva_caixa: (nivel_util as f64) * 500.0,
            ultimo_lucro_diario: 0.0,
        }
    }

    /// Fator de economia de escala: `+1%` por nível adicional até `+50%` (Seção 4.3 do `DESIGN.md`).
    pub fn fator_economia_escala(&self) -> f64 {
        let extra = self.nivel.saturating_sub(1).min(50) as f64;
        1.0 + 0.01 * extra
    }

    /// Razão de ocupação de mão de obra (`0.0..=1.0`).
    pub fn taxa_ocupacao(&self) -> f64 {
        if self.nivel == 0 {
            return 0.0;
        }
        let vagas = (self.nivel as u64) * TipoPredio::TRABALHADORES_POR_NIVEL;
        ((self.empregados as f64) / (vagas as f64)).clamp(0.0, 1.0)
    }

    /// Multiplicador efetivo de throughput produtivo antes da checagem de escassez do mercado.
    pub fn multiplicador_efetivo(&self, penalidade_escassez: f64) -> f64 {
        if self.nivel == 0 {
            return 0.0;
        }
        let escala = self.fator_economia_escala();
        let integridade = (1.0 - self.destruicao_guerra as f64).clamp(0.15, 1.0);
        let sem_escassez = (1.0 - penalidade_escassez).clamp(0.25, 1.0);
        (self.nivel as f64) * self.taxa_ocupacao() * escala * integridade * sem_escassez
    }

    /// Recupera gradualmente danos da Segunda Guerra Mundial em tempo de paz.
    pub fn recuperar_dano_paz_diario(&mut self) {
        if self.destruicao_guerra > 0.0 {
            self.destruicao_guerra = (self.destruicao_guerra - 0.0008).max(0.0);
        }
    }
}

/// Calcula o preço-alvo estilo *Victoria 3* (Seção 4.4 do `DESIGN.md`):
/// `P* = P0 * clamp(1 + 0.75 * (D - O) / max(eps, min(D, O)), 0.25, 1.75)`.
pub fn calcular_preco_alvo_mercado(preco_base: f64, oferta: f64, demanda: f64) -> f64 {
    let o = oferta.max(0.0);
    let d = demanda.max(0.0);
    if (o - d).abs() < 1e-6 {
        return preco_base;
    }
    if o <= 1e-6 && d > 0.0 {
        return preco_base * 1.75;
    }
    if d <= 1e-6 && o > 0.0 {
        return preco_base * 0.25;
    }
    let denom = o.min(d).max(1.0);
    let fator = (1.0 + 0.75 * ((d - o) / denom)).clamp(0.25, 1.75);
    preco_base * fator
}

/// Calcula a penalidade física de escassez (`0.0..=0.75`) quando `oferta < 0.5 * demanda`.
pub fn calcular_penalidade_escassez(oferta: f64, demanda: f64) -> f64 {
    if demanda <= 1.0 {
        return 0.0;
    }
    let razao = oferta.max(0.0) / demanda;
    if razao >= 0.5 {
        0.0
    } else {
        ((0.5 - razao) * 1.5).clamp(0.0, 0.75)
    }
}

/// Mercado nacional ou união aduaneira de bloco (ex: Área do Dólar, Área da Libra, Bloco Soviético).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mercado {
    pub id: String,
    pub nome: String,
    pub lider_tag: String,
    pub oferta: BTreeMap<BemId, f64>,
    pub demanda: BTreeMap<BemId, f64>,
    pub precos: BTreeMap<BemId, f64>,
}

impl Mercado {
    pub fn novo(id: impl Into<String>, nome: impl Into<String>, lider_tag: impl Into<String>) -> Self {
        let mut oferta = BTreeMap::new();
        let mut demanda = BTreeMap::new();
        let mut precos = BTreeMap::new();
        for bem in BemId::TODOS {
            oferta.insert(bem, 10.0);
            demanda.insert(bem, 10.0);
            precos.insert(bem, bem.preco_base());
        }
        Self {
            id: id.into(),
            nome: nome.into(),
            lider_tag: lider_tag.into(),
            oferta,
            demanda,
            precos,
        }
    }

    pub fn zerar_ordens(&mut self) {
        for bem in BemId::TODOS {
            self.oferta.insert(bem, 0.0);
            self.demanda.insert(bem, 0.0);
        }
    }

    pub fn registrar_oferta(&mut self, bem: BemId, quantidade: f64) {
        if quantidade > 0.0 {
            *self.oferta.entry(bem).or_insert(0.0) += quantidade;
        }
    }

    pub fn registrar_demanda(&mut self, bem: BemId, quantidade: f64) {
        if quantidade > 0.0 {
            *self.demanda.entry(bem).or_insert(0.0) += quantidade;
        }
    }

    pub fn preco(&self, bem: BemId) -> f64 {
        self.precos.get(&bem).copied().unwrap_or_else(|| bem.preco_base())
    }

    /// Atualiza os preços correntes com suavização exponencial `alpha = 0.20` em direção ao preço-alvo.
    pub fn atualizar_precos_diarios(&mut self) {
        const ALPHA: f64 = 0.20;
        for bem in BemId::TODOS {
            let o = self.oferta.get(&bem).copied().unwrap_or(0.0);
            let d = self.demanda.get(&bem).copied().unwrap_or(0.0);
            let alvo = calcular_preco_alvo_mercado(bem.preco_base(), o, d);
            let atual = self.preco(bem);
            let novo = atual + ALPHA * (alvo - atual);
            self.precos.insert(bem, novo);
        }
    }
}

/// Cinco patamares fiscais ajustáveis pelo gabinete ministerial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NivelImposto {
    MuitoBaixo,
    Baixo,
    Medio,
    Alto,
    MuitoAlto,
}

impl NivelImposto {
    pub const fn multiplicador_aliquota(self) -> f64 {
        match self {
            Self::MuitoBaixo => 0.60,
            Self::Baixo => 0.80,
            Self::Medio => 1.00,
            Self::Alto => 1.25,
            Self::MuitoAlto => 1.50,
        }
    }

    pub const fn impacto_aprovacao(self) -> f32 {
        match self {
            Self::MuitoBaixo => 3.0,
            Self::Baixo => 1.5,
            Self::Medio => 0.0,
            Self::Alto => -2.5,
            Self::MuitoAlto => -5.5,
        }
    }
}

/// Estado fiscal, tesouro, fundo privado de investimento e dívida pública de um país.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrcamentoNacional {
    pub nivel_imposto: NivelImposto,
    pub tesouro: f64,
    pub divida_soberana: f64,
    pub limite_credito: f64,
    pub fundo_investimento: f64,
    pub eficiencia_arrecadacao: f64,
    pub receita_impostos_diaria: f64,
    pub receita_estatal_diaria: f64,
    pub transferencias_externas_diaria: f64,
    pub despesa_administrativa_diaria: f64,
    pub despesa_militar_diaria: f64,
    pub despesa_construcao_diaria: f64,
    pub juros_divida_diaria: f64,
    pub em_moratoria: bool,
}

impl Default for OrcamentoNacional {
    fn default() -> Self {
        Self {
            nivel_imposto: NivelImposto::Medio,
            tesouro: 10_000.0,
            divida_soberana: 0.0,
            limite_credito: 100_000.0,
            fundo_investimento: 2_500.0,
            eficiencia_arrecadacao: 1.0,
            receita_impostos_diaria: 0.0,
            receita_estatal_diaria: 0.0,
            transferencias_externas_diaria: 0.0,
            despesa_administrativa_diaria: 0.0,
            despesa_militar_diaria: 0.0,
            despesa_construcao_diaria: 0.0,
            juros_divida_diaria: 0.0,
            em_moratoria: false,
        }
    }
}

impl OrcamentoNacional {
    pub fn receita_total_diaria(&self) -> f64 {
        self.receita_impostos_diaria
            + self.receita_estatal_diaria
            + self.transferencias_externas_diaria.max(0.0)
    }

    pub fn despesa_total_diaria(&self) -> f64 {
        self.despesa_administrativa_diaria
            + self.despesa_militar_diaria
            + self.despesa_construcao_diaria
            + self.juros_divida_diaria
            + (-self.transferencias_externas_diaria).max(0.0)
    }

    pub fn saldo_diario(&self) -> f64 {
        self.receita_total_diaria() - self.despesa_total_diaria()
    }

    /// Consolida o saldo diário no tesouro ou na dívida soberana e calcula juros progressivos.
    pub fn consolidar_caixa_diario(&mut self, grande_potencia: bool) {
        let taxa_base = if grande_potencia { 0.04 } else { 0.075 };
        let razao_divida = if self.limite_credito > 1.0 {
            (self.divida_soberana / self.limite_credito).clamp(0.0, 2.0)
        } else {
            0.0
        };
        let taxa_anual = taxa_base + 0.06 * razao_divida * razao_divida;
        self.juros_divida_diaria = self.divida_soberana * (taxa_anual / 365.0);

        let saldo = self.saldo_diario();
        if saldo >= 0.0 {
            if self.divida_soberana > 0.0 {
                let amortizado = saldo.min(self.divida_soberana);
                self.divida_soberana -= amortizado;
                self.tesouro += saldo - amortizado;
            } else {
                self.tesouro += saldo;
            }
        } else {
            let deficit = -saldo;
            if self.tesouro >= deficit {
                self.tesouro -= deficit;
            } else {
                let restante = deficit - self.tesouro;
                self.tesouro = 0.0;
                self.divida_soberana += restante;
            }
        }

        self.em_moratoria = self.limite_credito > 0.0 && self.divida_soberana > self.limite_credito;
    }
}
