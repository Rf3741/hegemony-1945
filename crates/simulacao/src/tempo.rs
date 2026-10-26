use serde::{Deserialize, Serialize};
use std::fmt;

/// Data do calendário gregoriano usada pela simulação (iniciando em 01/01/1945).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DataSimulacao {
    pub ano: i32,
    pub mes: u8,
    pub dia: u8,
}

impl DataSimulacao {
    /// Marco inicial oficial do cenário: 1º de janeiro de 1945.
    pub const INICIO_1945: Self = Self {
        ano: 1945,
        mes: 1,
        dia: 1,
    };

    pub const fn nova(ano: i32, mes: u8, dia: u8) -> Self {
        Self { ano, mes, dia }
    }

    /// Verifica se o ano é bissexto no calendário gregoriano.
    pub const fn ano_bissexto(ano: i32) -> bool {
        (ano % 4 == 0 && ano % 100 != 0) || (ano % 400 == 0)
    }

    /// Número de dias de um mês específico.
    pub const fn dias_no_mes(ano: i32, mes: u8) -> u8 {
        match mes {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::ano_bissexto(ano) {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    /// Avança exatamente 1 dia no calendário (tick diário).
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

    /// Nome do mês em português brasileiro (`pt-BR`).
    pub const fn nome_mes_pt(mes: u8) -> &'static str {
        match mes {
            1 => "janeiro",
            2 => "fevereiro",
            3 => "março",
            4 => "abril",
            5 => "maio",
            6 => "junho",
            7 => "julho",
            8 => "agosto",
            9 => "setembro",
            10 => "outubro",
            11 => "novembro",
            12 => "dezembro",
            _ => "mês inválido",
        }
    }

    /// Formata a data em português brasileiro extenso (ex: "1º de janeiro de 1945").
    pub fn formatar_pt_br(&self) -> String {
        let mes_str = Self::nome_mes_pt(self.mes);
        if self.dia == 1 {
            format!("1º de {mes_str} de {}", self.ano)
        } else {
            format!("{} de {mes_str} de {}", self.dia, self.ano)
        }
    }

    /// Formata de modo curto `DD/MM/AAAA` para tabelas e logs.
    pub fn formatar_curto(&self) -> String {
        format!("{:02}/{:02}/{:04}", self.dia, self.mes, self.ano)
    }

    /// Verdadeiro no primeiro dia de cada mês.
    pub const fn eh_inicio_mes(&self) -> bool {
        self.dia == 1
    }
}

impl Default for DataSimulacao {
    fn default() -> Self {
        Self::INICIO_1945
    }
}

impl fmt::Display for DataSimulacao {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.formatar_pt_br())
    }
}

/// Cinco níveis de velocidade em tempo real (1 a 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VelocidadeJogo {
    V1 = 1,
    V2 = 2,
    V3 = 3,
    V4 = 4,
    V5 = 5,
}

impl VelocidadeJogo {
    pub const TODAS: [Self; 5] = [Self::V1, Self::V2, Self::V3, Self::V4, Self::V5];

    pub const fn nivel(self) -> u8 {
        self as u8
    }

    pub const fn de_nivel(nivel: u8) -> Option<Self> {
        match nivel {
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            3 => Some(Self::V3),
            4 => Some(Self::V4),
            5 => Some(Self::V5),
            _ => None,
        }
    }

    /// Ticks diários executados por segundo real em cada velocidade.
    pub const fn dias_por_segundo(self) -> f64 {
        match self {
            Self::V1 => 1.0,
            Self::V2 => 3.0,
            Self::V3 => 7.0,
            Self::V4 => 15.0,
            Self::V5 => 30.0,
        }
    }

    pub const fn rotulo_pt(self) -> &'static str {
        match self {
            Self::V1 => "1x (Lento)",
            Self::V2 => "2x (Normal)",
            Self::V3 => "3x (Rápido)",
            Self::V4 => "4x (Muito Rápido)",
            Self::V5 => "5x (Máximo)",
        }
    }

    pub const fn aumentar(self) -> Self {
        match self {
            Self::V1 => Self::V2,
            Self::V2 => Self::V3,
            Self::V3 => Self::V4,
            Self::V4 | Self::V5 => Self::V5,
        }
    }

    pub const fn diminuir(self) -> Self {
        match self {
            Self::V1 | Self::V2 => Self::V1,
            Self::V3 => Self::V2,
            Self::V4 => Self::V3,
            Self::V5 => Self::V4,
        }
    }
}

/// Controlador determinístico de tempo real com pausa e acumulador de ticks diários.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelogioSimulacao {
    pub data: DataSimulacao,
    pub pausado: bool,
    pub velocidade: VelocidadeJogo,
    pub acumulador_segundos: f64,
    pub total_ticks: u64,
}

impl Default for RelogioSimulacao {
    fn default() -> Self {
        Self::novo()
    }
}

impl RelogioSimulacao {
    /// Limite de segurança por quadro para evitar travamento caso o SO congele a janela.
    pub const MAX_TICKS_POR_QUADRO: u32 = 10;

    pub const fn novo() -> Self {
        Self {
            data: DataSimulacao::INICIO_1945,
            pausado: true,
            velocidade: VelocidadeJogo::V2,
            acumulador_segundos: 0.0,
            total_ticks: 0,
        }
    }

    pub fn alternar_pausa(&mut self) {
        self.pausado = !self.pausado;
    }

    pub fn definir_velocidade(&mut self, velocidade: VelocidadeJogo) {
        self.velocidade = velocidade;
    }

    /// Consome `delta_segundos` de tempo real e devolve quantos ticks diários dispararam,
    /// avançando a data e `total_ticks` de forma consistente.
    pub fn atualizar(&mut self, delta_segundos: f64) -> u32 {
        if self.pausado || delta_segundos <= 0.0 {
            return 0;
        }
        let intervalo = 1.0 / self.velocidade.dias_por_segundo();
        self.acumulador_segundos += delta_segundos.min(2.0);

        let mut disparados = 0;
        while self.acumulador_segundos >= intervalo && disparados < Self::MAX_TICKS_POR_QUADRO {
            self.acumulador_segundos -= intervalo;
            self.data.avancar_dia();
            self.total_ticks += 1;
            disparados += 1;
        }
        if disparados == Self::MAX_TICKS_POR_QUADRO {
            self.acumulador_segundos = 0.0;
        }
        disparados
    }
}
