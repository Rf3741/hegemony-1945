//! Enumerações de domínio, calendário histórico (1/1/1945+) e tipos fundamentais.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Quantidade fixa de categorias universais de necessidade doméstica das pops.
pub const NUM_NECESSIDADES: usize = 6;

/// Data histórica do cenário (iniciando tipicamente em 01/01/1945).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DataJogo {
    pub ano: u16,
    pub mes: u8,
    pub dia: u8,
}

impl Default for DataJogo {
    fn default() -> Self {
        Self {
            ano: 1945,
            mes: 1,
            dia: 1,
        }
    }
}

impl DataJogo {
    /// Cria uma nova data validada do calendário gregoriano.
    pub fn nova(ano: u16, mes: u8, dia: u8) -> Option<Self> {
        if !(1..=12).contains(&mes) {
            return None;
        }
        let max_dias = Self::dias_no_mes(ano, mes);
        if !(1..=max_dias).contains(&dia) {
            return None;
        }
        Some(Self { ano, mes, dia })
    }

    /// Retorna se o ano é bissexto no calendário gregoriano.
    #[inline]
    pub const fn eh_bissexto(ano: u16) -> bool {
        (ano.is_multiple_of(4) && !ano.is_multiple_of(100)) || ano.is_multiple_of(400)
    }

    /// Quantidade de dias em um determinado mês e ano.
    #[inline]
    pub const fn dias_no_mes(ano: u16, mes: u8) -> u8 {
        match mes {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::eh_bissexto(ano) {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    /// Avança exatamente 1 dia no calendário.
    #[inline]
    pub fn avancar_dia(&mut self) {
        let limite = Self::dias_no_mes(self.ano, self.mes);
        if self.dia < limite {
            self.dia += 1;
        } else {
            self.dia = 1;
            if self.mes < 12 {
                self.mes += 1;
            } else {
                self.mes = 1;
                self.ano += 1;
            }
        }
    }
}

impl fmt::Display for DataJogo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}/{:02}/{:04}", self.dia, self.mes, self.ano)
    }
}

/// Estrato socioeconômico de uma profissão (`TipoPop`), ao estilo Victoria 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Estrato {
    /// Camponeses, operários, soldados rasos e trabalhadores braçais.
    Baixo,
    /// Técnicos, maquinistas, burocratas, engenheiros, agricultores comerciais e intelectuais.
    Medio,
    /// Capitalistas, grandes industriais e aristocratas/latifundiários.
    Alto,
}

impl Estrato {
    /// Índice compacto `0..3` para vetores estáticos sem alocação.
    #[inline]
    pub const fn indice(self) -> usize {
        match self {
            Self::Baixo => 0,
            Self::Medio => 1,
            Self::Alto => 2,
        }
    }
}

/// Classificação econômica macro de um `Bem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CategoriaBem {
    AlimentoBasico,
    MateriaPrima,
    Energia,
    Industrial,
    Consumo,
    Luxo,
    Militar,
    LocalNaoComercializavel,
}

/// Categorias de necessidade doméstica atendidas por cestas de bens substituíveis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CategoriaNecessidade {
    AlimentacaoBasica,
    VestuarioDomestico,
    EnergiaAquecimento,
    ServicosUrbanos,
    MobilidadeComunicacao,
    ArtigosLuxo,
}

impl CategoriaNecessidade {
    /// Lista ordenada de todas as categorias de necessidade.
    pub const TODAS: [Self; NUM_NECESSIDADES] = [
        Self::AlimentacaoBasica,
        Self::VestuarioDomestico,
        Self::EnergiaAquecimento,
        Self::ServicosUrbanos,
        Self::MobilidadeComunicacao,
        Self::ArtigosLuxo,
    ];

    /// Índice `0..NUM_NECESSIDADES` correspondente à categoria.
    #[inline]
    pub const fn indice(self) -> usize {
        match self {
            Self::AlimentacaoBasica => 0,
            Self::VestuarioDomestico => 1,
            Self::EnergiaAquecimento => 2,
            Self::ServicosUrbanos => 3,
            Self::MobilidadeComunicacao => 4,
            Self::ArtigosLuxo => 5,
        }
    }

    /// Calcula a distribuição orçamentária das necessidades (soma = 1.0)
    /// combinando o perfil do `Estrato` com a Lei de Engel conforme o `padrao_de_vida`.
    #[inline]
    pub fn pesos_orcamento(estrato: Estrato, padrao_de_vida: f64) -> [f64; NUM_NECESSIDADES] {
        let sol = padrao_de_vida.clamp(1.0, 99.0);
        let mut pesos = match estrato {
            Estrato::Baixo => [0.46, 0.22, 0.14, 0.09, 0.06, 0.03],
            Estrato::Medio => [0.25, 0.20, 0.12, 0.18, 0.13, 0.12],
            Estrato::Alto => [0.10, 0.15, 0.08, 0.22, 0.15, 0.30],
        };

        // Modulação contínua pela Lei de Engel:
        // Padrão de vida baixo concentra gasto em alimentos e aquecimento;
        // padrão de vida elevado desloca orçamento para serviços, mobilidade e luxo.
        if sol < 10.0 {
            let urgencia = (10.0 - sol) / 10.0;
            pesos[0] += 0.25 * urgencia;
            pesos[2] += 0.06 * urgencia;
            pesos[4] *= 1.0 - 0.60 * urgencia;
            pesos[5] *= 1.0 - 0.85 * urgencia;
        } else if sol > 15.0 {
            let afluencia = ((sol - 15.0) / 40.0).min(1.0);
            pesos[0] *= 1.0 - 0.45 * afluencia;
            pesos[3] += 0.08 * afluencia;
            pesos[4] += 0.07 * afluencia;
            pesos[5] += 0.22 * afluencia;
        }

        let soma: f64 = pesos.iter().sum();
        if soma > 0.0 {
            for p in &mut pesos {
                *p /= soma;
            }
        } else {
            pesos = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        }
        pesos
    }
}

/// Slot funcional ocupado por um `MetodoProducao` dentro de um prédio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SlotMetodo {
    /// Técnica produtiva principal (ex.: agricultura tradicional vs. mecanizada).
    ProducaoPrincipal,
    /// Automação poupadora de mão de obra (ex.: transporte ferroviário interno, esteiras).
    Automacao,
    /// Estrutura societária e destino dos dividendos (privado, aristocrático, estatal, cooperativo).
    Propriedade,
    /// Produção complementar ou subproduto secundário.
    Secundario,
}

/// Regime de propriedade que determina quem recebe os lucros/dividendos excedentes do prédio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegimePropriedade {
    /// Dividendos distribuídos aos `capitalistas` locais/nacionais.
    PrivadoCapitalista,
    /// Dividendos distribuídos aos `aristocratas`/latifundiários locais/nacionais.
    Aristocratico,
    /// Lucros recolhidos ao tesouro do Estado (empresas estatais / economia planificada).
    Estatal,
    /// Excedente repartido diretamente entre os trabalhadores empregados no próprio prédio.
    Cooperativo,
}

/// Categoria funcional de um `TipoPredio`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CategoriaPredio {
    RuralAgricola,
    Extrativo,
    Industrial,
    Infraestrutura,
    Construcao,
    Governamental,
    Militar,
}

/// Eixo jurídico-econômico de uma `LeiEconomica`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrupoLeiEconomica {
    SistemaEconomico,
    Tributacao,
    Comercio,
}

/// Fórmula de formação de preços nos mercados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormulaPreco {
    /// Fórmula direta: `base * clamp(1 + (D - O) / max(O, D), 0.25, 1.75)` (com proteção `0/0 -> 1.0`).
    Classica,
    /// Fórmula canônica estilo Victoria 3 aprimorada:
    /// `alvo = base * clamp(1 + 0.75 * (D - O) / max(O, D, volume_minimo), 0.25, 1.75)`
    /// combinada com amortecimento exponencial configurável para evitar serrilhamento diário.
    VictoriaSuavizada,
}

/// Cadência de execução das fases estruturais dentro do tick diário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModoCadencia {
    /// Executa todas as 9 fases suavemente todos os dias em taxas diárias (sem picos semanais).
    DiarioCompleto,
    /// Executa produção, mercado, salários, consumo, impostos e construção diariamente,
    /// e executa demografia/migração e contratação/demissão a cada 7 dias.
    DiarioComAjusteSemanal,
}
