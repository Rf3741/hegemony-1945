//! Controle temporal determinístico para simulação e demonstração acelerada (1945..=1991).

use bevy::prelude::*;

/// Recurso global que controla a passagem do tempo na simulação.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct TempoSimulacao {
    /// Ano contínuo com precisão fracionária (1945.0 até 1991.0).
    pub ano_continuo: f32,
    /// Se o relógio está pausado pelo jogador.
    pub pausado: bool,
    /// Multiplicador de velocidade (1.0 = normal, 2.0, 5.0, 10.0, etc.).
    pub velocidade: f32,
    /// Se está em modo demonstração contínua (avança automaticamente até 1991).
    pub modo_demonstracao: bool,
}

impl Default for TempoSimulacao {
    fn default() -> Self {
        Self {
            ano_continuo: 1945.0,
            pausado: false,
            velocidade: 1.0,
            modo_demonstracao: false,
        }
    }
}

impl TempoSimulacao {
    /// Taxa base: 1 ano de tempo de jogo leva 10 segundos na velocidade 1.0.
    pub const SEGUNDOS_POR_ANO_BASE: f32 = 10.0;
    pub const ANO_INICIAL: f32 = 1945.0;
    pub const ANO_FINAL: f32 = 1991.0;

    /// Cria uma nova instância de controle temporal.
    pub fn novo(ano_inicial: f32) -> Self {
        Self {
            ano_continuo: ano_inicial.clamp(Self::ANO_INICIAL, Self::ANO_FINAL),
            pausado: false,
            velocidade: 1.0,
            modo_demonstracao: false,
        }
    }

    /// Avança o relógio com base no delta time do motor.
    pub fn avancar(&mut self, dt: f32) {
        if self.pausado {
            return;
        }

        let vel_efetiva = if self.modo_demonstracao {
            // Em modo demonstração acelerada, avança cerca de 1.2 anos por segundo (~38 segundos para 1945..1991)
            12.0
        } else {
            self.velocidade
        };

        let delta_anos = (dt / Self::SEGUNDOS_POR_ANO_BASE) * vel_efetiva;
        self.ano_continuo += delta_anos;

        if self.ano_continuo >= Self::ANO_FINAL {
            self.ano_continuo = Self::ANO_FINAL;
            if self.modo_demonstracao {
                // Ao atingir 1991 na demonstração, pausa para observação do resultado final
                self.pausado = true;
                self.modo_demonstracao = false;
            }
        }
    }

    /// Alterna estado de pausa.
    pub fn alternar_pausa(&mut self) {
        self.pausado = !self.pausado;
    }

    /// Ajusta o ano via scrub bar interativa (1945..=1991).
    pub fn definir_ano(&mut self, novo_ano: f32) {
        self.ano_continuo = novo_ano.clamp(Self::ANO_INICIAL, Self::ANO_FINAL);
    }

    /// Retorna o ano inteiro truncado.
    #[inline]
    pub fn ano_inteiro(&self) -> u16 {
        self.ano_continuo.floor() as u16
    }

    /// Retorna a fração decorrida do ano atual [0.0, 1.0).
    #[inline]
    pub fn fracao_ano(&self) -> f32 {
        self.ano_continuo.fract().clamp(0.0, 0.9999)
    }

    /// Retorna a data aproximada no formato pt-BR "DD/MM/AAAA".
    pub fn data_formatada(&self) -> String {
        let ano = self.ano_inteiro();
        let dia_do_ano = (self.fracao_ano() * 365.0).floor() as u16 + 1;
        
        let (mes, dia) = dia_do_ano_para_mes_dia(dia_do_ano);
        format!("{:02}/{:02}/{}", dia, mes, ano)
    }

    /// Determina o índice da era correspondente ao ano atual:
    /// 0 = Era 1945 (1945..1955)
    /// 1 = Era 1955 (1955..1965)
    /// 2 = Era 1965 (1965..1975)
    /// 3 = Era 1975 (1975..1985)
    /// 4 = Era 1985 (1985..1991+)
    pub fn indice_era(&self) -> usize {
        let ano = self.ano_inteiro();
        if ano < 1955 {
            0
        } else if ano < 1965 {
            1
        } else if ano < 1975 {
            2
        } else if ano < 1985 {
            3
        } else {
            4
        }
    }

    /// Fator de interpolação entre o cenário de 1945 (0.0) e 1991 (1.0).
    #[inline]
    pub fn fator_evolucao_global(&self) -> f32 {
        ((self.ano_continuo - Self::ANO_INICIAL) / (Self::ANO_FINAL - Self::ANO_INICIAL))
            .clamp(0.0, 1.0)
    }
}

/// Converte o dia do ano (1..=365) em mês e dia do mês.
fn dia_do_ano_para_mes_dia(dia_do_ano: u16) -> (u8, u8) {
    let dias_meses = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut restante = dia_do_ano.clamp(1, 365);
    for (i, &dias) in dias_meses.iter().enumerate() {
        if restante <= dias {
            return ((i + 1) as u8, restante as u8);
        }
        restante -= dias;
    }
    (12, 31)
}

/// Sistema Bevy que avança o tempo a cada tick do motor.
pub fn atualizar_tempo_sistema(time: Res<Time>, mut tempo: ResMut<TempoSimulacao>) {
    tempo.avancar(time.delta_secs());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tempo_inicial_e_avanco() {
        let mut t = TempoSimulacao::novo(1945.0);
        assert_eq!(t.ano_inteiro(), 1945);
        assert_eq!(t.indice_era(), 0);

        // Avança 10 segundos na velocidade 1.0 -> 1 ano
        t.avancar(10.0);
        assert_eq!(t.ano_inteiro(), 1946);
    }

    #[test]
    fn test_transicao_de_eras() {
        let mut t = TempoSimulacao::novo(1945.0);
        assert_eq!(t.indice_era(), 0); // 1945

        t.definir_ano(1955.5);
        assert_eq!(t.indice_era(), 1); // 1955

        t.definir_ano(1968.0);
        assert_eq!(t.indice_era(), 2); // 1965

        t.definir_ano(1977.2);
        assert_eq!(t.indice_era(), 3); // 1975

        t.definir_ano(1989.9);
        assert_eq!(t.indice_era(), 4); // 1985
    }

    #[test]
    fn test_limites_e_formato_data() {
        let mut t = TempoSimulacao::novo(1945.0);
        let data = t.data_formatada();
        assert!(data.starts_with("01/01/1945"));

        t.definir_ano(2050.0);
        assert_eq!(t.ano_inteiro(), 1991);
    }
}

/// Avança o tempo de demonstração e publica o [`AnoVisual`] (só escreve quando muda de fato).
pub fn publicar_ano_sistema(tempo: Res<TempoSimulacao>, mut ano: ResMut<crate::entrada::AnoVisual>) {
    if (ano.0 - tempo.ano_continuo).abs() > 1e-4 {
        ano.0 = tempo.ano_continuo;
    }
}
