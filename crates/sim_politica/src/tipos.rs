//! Enumerações de domínio, calendário da Guerra Fria (1945+) e tipos fundamentais de política.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Data do calendário histórico a partir de 01/01/1945.
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
    /// Cria uma nova data gregoriana validada.
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

    /// Retorna verdadeiro se o ano for bissexto no calendário gregoriano.
    #[inline]
    pub const fn eh_bissexto(ano: u16) -> bool {
        (ano.is_multiple_of(4) && !ano.is_multiple_of(100)) || ano.is_multiple_of(400)
    }

    /// Retorna a quantidade de dias no mês do ano fornecido.
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

    /// Avança exatamente 1 tick diário no calendário.
    #[inline]
    pub fn avancar_dia(&mut self) {
        let max_dias = Self::dias_no_mes(self.ano, self.mes);
        if self.dia < max_dias {
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

/// Estrato socioeconômico de um grupo populacional (Pop), no estilo Victoria 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Estrato {
    /// Camponeses, operários, mineiros e soldados rasos.
    Baixo,
    /// Engenheiros, burocratas, oficiais, intelectuais, clero e pequena burguesia.
    Medio,
    /// Capitalistas, latifundiários e aristocratas.
    Alto,
}

impl Estrato {
    /// Retorna o índice numérico `0..3`.
    #[inline]
    pub const fn indice(self) -> usize {
        match self {
            Self::Baixo => 0,
            Self::Medio => 1,
            Self::Alto => 2,
        }
    }
}

/// Papel institucional de um Grupo de Interesse (GI) no governo da nação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PapelGrupoInteresse {
    /// Faz parte formal do gabinete ministerial e da coalizão de governo.
    NoGoverno,
    /// Fora do governo, influente ou poderoso na oposição parlamentar.
    NaOposicao,
    /// Excluído do processo decisório formal (`clout < 5%`).
    Marginalizado,
}

/// Classificação do poder político relativo (`clout`) de um Grupo de Interesse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusClout {
    /// `clout >= 20%`: poder de veto informal e modificadores em dobro.
    Poderoso,
    /// `5% <= clout < 20%`: voz ativa padrão e modificadores normais.
    Influente,
    /// `clout < 5%`: excluído de cargos e modificadores inativos.
    Marginalizado,
}

impl StatusClout {
    /// Avalia o status a partir do valor percentual de clout `0.0..=1.0`.
    #[inline]
    pub fn de_clout(clout: f64) -> Self {
        if clout >= 0.20 {
            Self::Poderoso
        } else if clout >= 0.05 {
            Self::Influente
        } else {
            Self::Marginalizado
        }
    }
}

/// Humor e nível de satisfação de um Grupo de Interesse (`-20.0..=+20.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HumorGrupoInteresse {
    /// `aprovacao <= -10.0`: insurreição armada, greve geral, boicote ou golpe militar.
    Radical,
    /// `-10.0 < aprovacao <= -5.0`: descontentamento ativo e penalidades moderadas.
    Irritado,
    /// `-5.0 < aprovacao < +5.0`: sem bônus nem penalidades críticas.
    Neutro,
    /// `+5.0 <= aprovacao < +10.0`: apoio governamental e bônus moderados.
    Feliz,
    /// `aprovacao >= +10.0`: máxima lealdade e bônus patrióticos plenos.
    Leal,
}

impl HumorGrupoInteresse {
    /// Determina o humor a partir da pontuação contínua de aprovação.
    #[inline]
    pub fn de_aprovacao(aprovacao: f64) -> Self {
        if aprovacao <= -10.0 {
            Self::Radical
        } else if aprovacao <= -5.0 {
            Self::Irritado
        } else if aprovacao < 5.0 {
            Self::Neutro
        } else if aprovacao < 10.0 {
            Self::Feliz
        } else {
            Self::Leal
        }
    }

    /// Retorna se o grupo está com humor favorável ao governo (`Feliz` ou `Leal`).
    #[inline]
    pub fn eh_feliz(&self) -> bool {
        matches!(self, Self::Feliz | Self::Leal)
    }

    /// Retorna se o grupo está com humor hostil moderado (`Irritado`).
    #[inline]
    pub fn eh_irritado(&self) -> bool {
        matches!(self, Self::Irritado)
    }

    /// Retorna se o grupo está em humor insurrecto radical (`Radical`).
    #[inline]
    pub fn eh_radical(&self) -> bool {
        matches!(self, Self::Radical)
    }

    /// Retorna se o grupo ativa modificadores insurrectos negativos.
    #[inline]
    pub fn eh_insurrecto(&self) -> bool {
        matches!(self, Self::Radical)
    }
}

/// Faixa de Legitimidade Governamental (`0.0..=100.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaixaLegitimidade {
    /// `0..24`: governo ilegítimo, paralisia legislativa total, radicalização acelerada.
    Ilegitimo,
    /// `25..49`: governo inaceitável, tramitação de leis com dobro do tempo.
    Inaceitavel,
    /// `50..74`: governo aceitável, funcionamento regular da máquina do Estado.
    Aceitavel,
    /// `75..89`: governo legítimo, aceleração legislativa e redução de radicais.
    Legitimo,
    /// `90..100`: governo hegemônico, máxima velocidade legislativa e coesão nacional.
    Hegemonico,
}

impl FaixaLegitimidade {
    /// Determina a faixa a partir da pontuação de legitimidade.
    #[inline]
    pub fn de_pontos(pontos: f64) -> Self {
        let p = pontos.clamp(0.0, 100.0);
        if p < 25.0 {
            Self::Ilegitimo
        } else if p < 50.0 {
            Self::Inaceitavel
        } else if p < 75.0 {
            Self::Aceitavel
        } else if p < 90.0 {
            Self::Legitimo
        } else {
            Self::Hegemonico
        }
    }

    /// Retorna se a legitimidade permite avanço diário de projetos de lei.
    #[inline]
    pub fn permite_tramitacao_leis(self) -> bool {
        !matches!(self, Self::Ilegitimo)
    }

    /// Multiplicador de velocidade de progresso legislativo diário.
    #[inline]
    pub fn multiplicador_velocidade_leis(self) -> f64 {
        match self {
            Self::Ilegitimo => 0.0,
            Self::Inaceitavel => 0.50,
            Self::Aceitavel => 1.00,
            Self::Legitimo => 1.25,
            Self::Hegemonico => 1.50,
        }
    }
}

/// Posição de um Grupo de Interesse ou Ideologia perante uma mudança de lei.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PosturaLei {
    /// Rejeição extrema (-2): causa forte revolta se for aprovada.
    RejeitaFortemente,
    /// Oposição (-1): desaprova a promulgação.
    Opoe,
    /// Neutralidade (0): indiferente à alteração.
    Neutro,
    /// Apoio (+1): deseja a aprovação.
    Apoia,
    /// Endosso absoluto (+2): bandeira prioritária do grupo ou ideologia.
    EndossaFortemente,
}

impl PosturaLei {
    /// Retorna o valor numérico de atitude `-2..=+2`.
    #[inline]
    pub fn valor(self) -> i32 {
        match self {
            Self::RejeitaFortemente => -2,
            Self::Opoe => -1,
            Self::Neutro => 0,
            Self::Apoia => 1,
            Self::EndossaFortemente => 2,
        }
    }

    /// Retorna se a postura é favorável à lei.
    #[inline]
    pub fn eh_favoravel(self) -> bool {
        self.valor() > 0
    }

    /// Retorna se a postura é contrária à lei.
    #[inline]
    pub fn eh_contraria(self) -> bool {
        self.valor() < 0
    }
}

/// Fases da tramitação de um projeto de lei no parlamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaseLegislativa {
    /// 1ª Fase: Apresentação e leitura inicial no plenário.
    Introducao,
    /// 2ª Fase: Deliberação em comitê e emendas de bancada.
    Comite,
    /// 3ª Fase: Votação final de redação antes da sanção.
    VotacaoFinal,
}

impl FaseLegislativa {
    /// Próxima fase após aprovação em checkpoint, ou `None` se sancionada.
    #[inline]
    pub fn proxima(self) -> Option<Self> {
        match self {
            Self::Introducao => Some(Self::Comite),
            Self::Comite => Some(Self::VotacaoFinal),
            Self::VotacaoFinal => None,
        }
    }

    /// Índice numérico `0..3`.
    #[inline]
    pub fn indice(self) -> usize {
        match self {
            Self::Introducao => 0,
            Self::Comite => 1,
            Self::VotacaoFinal => 2,
        }
    }
}

/// Resultado de um checkpoint de avaliação legislativa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResultadoCheckpointLei {
    /// O projeto avançou para a próxima fase legislativa ou sanção final.
    Avanco,
    /// O projeto gerou debate parlamentar, emendas e concessões.
    Debate,
    /// O projeto travou temporariamente por obstrução da oposição (gera 1 entrave).
    Travar,
    /// O projeto retrocedeu para a fase anterior.
    Recuo,
}

/// Sistema de representação para conversão de votos populares em cadeiras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TipoSistemaRepresentacao {
    /// Representação proporcional pelo método D'Hondt com cláusula de barreira.
    ProporcionalDHondt,
    /// Sistema distrital majoritário simples (first-past-the-post).
    MajoritarioDistrital,
    /// Plebiscito de lista única oficial do partido de vanguarda.
    PlebiscitarioPartidoUnico,
}

/// Alinhamento geopolítico formal no cenário da Guerra Fria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlinhamentoBloco {
    BlocoOcidental,
    BlocoSovietico,
    NaoAlinhado,
    EixoDerrotado,
}

/// Nível de integração de um país na esfera de influência de uma superpotência.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NivelEsfera {
    /// `0.0..19.9`: relações diplomáticas protocolares.
    ContatoDiplomatico,
    /// `20.0..49.9`: parceiro comercial prioritário e trocas culturais.
    ParceiroComercial,
    /// `50.0..74.9`: alinhado com cooperação militar e votos alinhados na ONU.
    AlinhadoBloco,
    /// `75.0..89.9`: integrado à esfera, com bases e união aduaneira.
    IntegradoEsfera,
    /// `90.0..100.0`: estado cliente / satélite sob tutela política estrita.
    EstadoSatelite,
}

impl NivelEsfera {
    /// Determina o nível a partir da pontuação `0.0..=100.0`.
    #[inline]
    pub fn de_pontos(pontos: f64) -> Self {
        let p = pontos.clamp(0.0, 100.0);
        if p < 20.0 {
            Self::ContatoDiplomatico
        } else if p < 50.0 {
            Self::ParceiroComercial
        } else if p < 75.0 {
            Self::AlinhadoBloco
        } else if p < 90.0 {
            Self::IntegradoEsfera
        } else {
            Self::EstadoSatelite
        }
    }
}

/// Superpotência patrona da Guerra Fria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PotenciaBloco {
    Eua,
    Urss,
}

/// Tipo de crise interna aguda deflagrada por radicalismo, descontentamento ou conspiração.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TipoCrisePolitica {
    Greve,
    Revolta,
    GolpeMilitar,
    Revolucao,
}

/// Causa originária de eclosão de uma guerra civil (evento para o futuro `sim_guerra`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrigemGuerraCivil {
    RevolucaoIdeologica,
    GolpeMilitarImpasse,
    SecessaoAnticolonialOuEtnica,
}
