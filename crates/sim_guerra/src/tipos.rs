//! Enumerações de domínio, tipos fundamentais e tipos de valor de `sim_guerra`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Data histórica do calendário de jogo (iniciando em 01/01/1945).
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

/// Postura operacional atribuída a um exército em uma frente ou teatro de operações.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PosturaOperacional {
    /// Maximiza bônus defensivo e entrincheiramento; não inicia ofensivas por iniciativa própria.
    #[default]
    DefesaEmProfundidade,
    /// Ataca quando tiver vantagem tática razoável e organização suficiente; avanço cauteloso.
    OfensivaEquilibrada,
    /// Ataca agressivamente para romper linhas inimigas com foco em velocidade de avanço.
    AssaltoAgressivo,
    /// Destinado a policiar retaguarda, portos, capitais e suprimir resistência de ocupação.
    ContencaoGuarnicao,
}

impl PosturaOperacional {
    /// Multiplicador sobre o poder de ataque.
    #[inline]
    pub const fn modificador_ataque(self) -> f64 {
        match self {
            Self::DefesaEmProfundidade => 0.70,
            Self::OfensivaEquilibrada => 1.05,
            Self::AssaltoAgressivo => 1.30,
            Self::ContencaoGuarnicao => 0.50,
        }
    }

    /// Multiplicador sobre o poder de defesa.
    #[inline]
    pub const fn modificador_defesa(self) -> f64 {
        match self {
            Self::DefesaEmProfundidade => 1.35,
            Self::OfensivaEquilibrada => 1.00,
            Self::AssaltoAgressivo => 0.80,
            Self::ContencaoGuarnicao => 1.15,
        }
    }

    /// Taxa máxima de entrincheiramento que a postura permite acumular.
    #[inline]
    pub const fn multiplicador_entrincheiramento(self) -> f64 {
        match self {
            Self::DefesaEmProfundidade => 1.50,
            Self::OfensivaEquilibrada => 1.00,
            Self::AssaltoAgressivo => 0.50,
            Self::ContencaoGuarnicao => 1.20,
        }
    }

    /// Multiplicador de consumo de suprimentos diários.
    #[inline]
    pub const fn multiplicador_consumo_suprimento(self) -> f64 {
        match self {
            Self::DefesaEmProfundidade => 0.85,
            Self::OfensivaEquilibrada => 1.15,
            Self::AssaltoAgressivo => 1.60,
            Self::ContencaoGuarnicao => 0.75,
        }
    }

    /// Indica se a postura autoriza início proativo de combates ofensivos.
    #[inline]
    pub const fn autoriza_ataque(self) -> bool {
        matches!(self, Self::OfensivaEquilibrada | Self::AssaltoAgressivo)
    }
}

/// Classificação militar e doutrinária dos batalhões e unidades.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClasseUnidade {
    Infantaria,
    Motorizada,
    Blindada,
    Artilharia,
    ForcasEspeciais,
    Engenharia,
    ApoioLogistico,
}

/// Estado de uma batalha em andamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TipoResultadoBatalha {
    VitoriaAtacante,
    VitoriaDefensor,
    EmpateEmAndamento,
}

/// Cláusula específica de um tratado de paz.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClausulaPaz {
    /// Restauração do status quo ante bellum: territórios ocupados voltam aos donos legítimos.
    PazBranca,
    /// Anexação territorial definitiva de uma ou mais regiões pelo vencedor.
    AnexarRegioes {
        vencedor_tag: String,
        perdedor_tag: String,
        regioes_adm1: Vec<String>,
    },
    /// Alteração da ideologia ou alinhamento geopolítico do país derrotado (Guerra Fria).
    MudancaRegime {
        alvo_tag: String,
        patrocinador_tag: String,
        nova_ideologia: String,
    },
    /// Subordinação do país derrotado como estado satélite / protetorado do vencedor.
    EstadoSatelite {
        suserano_tag: String,
        vassalo_tag: String,
    },
    /// Estabelecimento de zonas de ocupação militar temporárias (ex: Alemanha/Áustria 1945).
    ZonaOcupacao {
        ocupante_tag: String,
        derrotado_tag: String,
        regioes_adm1: Vec<String>,
        duracao_dias: u32,
    },
    /// Pagamento periódico de reparações de guerra do perdedor ao vencedor.
    ReparacoesGuerra {
        pagador_tag: String,
        recebedor_tag: String,
        fracao_receita: f64,
        duracao_dias: u32,
    },
    /// Limitação legal imposta sobre o número máximo de divisões que o país pode manter.
    Desmilitarizacao {
        alvo_tag: String,
        limite_divisoes: usize,
        duracao_dias: u32,
    },
}
