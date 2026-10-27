//! Sistema de infraestrutura estadual com os 5 modais:
//! estradas, ferrovias, portos, aeroportos e rede elétrica.

use crate::tipos::{EraTecnologicaVisual, TipoInfraestrutura, TipoTerreno};
use serde::{Deserialize, Serialize};

/// Capacidades nominais de projeto por nível em cada modal em condições normais.
pub const CAPACIDADE_BASE_ESTRADAS: f64 = 25.0;
pub const CAPACIDADE_BASE_FERROVIAS: f64 = 50.0;
pub const CAPACIDADE_BASE_PORTOS: f64 = 60.0;
pub const CAPACIDADE_BASE_AEROPORTOS: f64 = 30.0;
pub const CAPACIDADE_BASE_REDE_ELETRICA: f64 = 40.0;

/// Estado operacional de um dos 5 modais de infraestrutura em um estado.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstadoModalInfra {
    pub modal: TipoInfraestrutura,
    /// Nível construído de infraestrutura (>= 0).
    pub nivel: u32,
    /// Capacidade de transporte/energia fornecida por nível.
    pub capacidade_base_por_nivel: f64,
    /// Capacidade total operacional disponível (já afetada por era, terreno e dano).
    #[serde(default)]
    pub capacidade_efetiva: f64,
    /// Demanda / uso total atual imposto por populações, cidades e indústrias.
    #[serde(default)]
    pub uso_atual: f64,
    /// Fator de destruição física ou desgaste da infraestrutura (0.00 = intacto, 1.00 = destruição total).
    #[serde(default)]
    pub dano: f64,
}

impl EstadoModalInfra {
    pub fn novo(modal: TipoInfraestrutura, nivel: u32) -> Self {
        let capacidade_base = match modal {
            TipoInfraestrutura::Estradas => CAPACIDADE_BASE_ESTRADAS,
            TipoInfraestrutura::Ferrovias => CAPACIDADE_BASE_FERROVIAS,
            TipoInfraestrutura::Portos => CAPACIDADE_BASE_PORTOS,
            TipoInfraestrutura::Aeroportos => CAPACIDADE_BASE_AEROPORTOS,
            TipoInfraestrutura::RedeEletrica => CAPACIDADE_BASE_REDE_ELETRICA,
        };
        let mut inst = Self {
            modal,
            nivel,
            capacidade_base_por_nivel: capacidade_base,
            capacidade_efetiva: 0.0,
            uso_atual: 0.0,
            dano: 0.0,
        };
        inst.recalcular_capacidade(1.0, EraTecnologicaVisual::Era1945, 0.0);
        inst
    }

    /// Taxa de utilização da infraestrutura (0.0 a 1.0 = normal, > 1.0 = sobrecarga/gargalo).
    #[inline]
    pub fn taxa_utilizacao(&self) -> f64 {
        if self.capacidade_efetiva <= 0.0 {
            if self.uso_atual > 0.0 { 2.0 } else { 0.0 }
        } else {
            self.uso_atual / self.capacidade_efetiva
        }
    }

    /// Retorna `true` se o uso supera a capacidade operacional.
    #[inline]
    pub fn sobrecarregado(&self) -> bool {
        self.uso_atual > self.capacidade_efetiva && self.uso_atual > 0.0
    }

    /// Saldo líquido de capacidade (positivo = folga, negativo = déficit).
    #[inline]
    pub fn saldo(&self) -> f64 {
        self.capacidade_efetiva - self.uso_atual
    }

    /// Recalcula a capacidade efetiva considerando terreno, era visual e dano.
    pub fn recalcular_capacidade(
        &mut self,
        modificador_terreno: f64,
        era: EraTecnologicaVisual,
        bonus_predios: f64,
    ) {
        // Multiplicador tecnológico de eficiência por era
        let mult_era = match (self.modal, era) {
            (TipoInfraestrutura::Estradas, EraTecnologicaVisual::Era1945) => 1.0,
            (TipoInfraestrutura::Estradas, EraTecnologicaVisual::Era1955) => 1.25, // Asfalto e autoestradas
            (TipoInfraestrutura::Estradas, EraTecnologicaVisual::Era1965) => 1.50, // Rodovias interestaduais
            (TipoInfraestrutura::Estradas, EraTecnologicaVisual::Era1975) => 1.70,
            (TipoInfraestrutura::Estradas, EraTecnologicaVisual::Era1985) => 1.85,

            (TipoInfraestrutura::Ferrovias, EraTecnologicaVisual::Era1945) => 1.0,
            (TipoInfraestrutura::Ferrovias, EraTecnologicaVisual::Era1955) => 1.20, // Dieselização
            (TipoInfraestrutura::Ferrovias, EraTecnologicaVisual::Era1965) => 1.40, // Eletrificação de linhas
            (TipoInfraestrutura::Ferrovias, EraTecnologicaVisual::Era1975) => 1.60,
            (TipoInfraestrutura::Ferrovias, EraTecnologicaVisual::Era1985) => 1.80, // Trens de alta capacidade

            (TipoInfraestrutura::Portos, EraTecnologicaVisual::Era1945) => 1.0,
            (TipoInfraestrutura::Portos, EraTecnologicaVisual::Era1955) => 1.15,
            (TipoInfraestrutura::Portos, EraTecnologicaVisual::Era1965) => 1.60, // Revolução da conteinerização
            (TipoInfraestrutura::Portos, EraTecnologicaVisual::Era1975) => 1.85,
            (TipoInfraestrutura::Portos, EraTecnologicaVisual::Era1985) => 2.10,

            (TipoInfraestrutura::Aeroportos, EraTecnologicaVisual::Era1945) => 1.0,
            (TipoInfraestrutura::Aeroportos, EraTecnologicaVisual::Era1955) => 1.40, // Quadrimotores comerciais
            (TipoInfraestrutura::Aeroportos, EraTecnologicaVisual::Era1965) => 2.00, // Jatos comerciais (Boeing 707)
            (TipoInfraestrutura::Aeroportos, EraTecnologicaVisual::Era1975) => 2.50, // Wide-bodies (747)
            (TipoInfraestrutura::Aeroportos, EraTecnologicaVisual::Era1985) => 2.90,

            (TipoInfraestrutura::RedeEletrica, EraTecnologicaVisual::Era1945) => 1.0,
            (TipoInfraestrutura::RedeEletrica, EraTecnologicaVisual::Era1955) => 1.25,
            (TipoInfraestrutura::RedeEletrica, EraTecnologicaVisual::Era1965) => 1.55, // Alta tensão regional
            (TipoInfraestrutura::RedeEletrica, EraTecnologicaVisual::Era1975) => 1.85, // Integração e nuclear
            (TipoInfraestrutura::RedeEletrica, EraTecnologicaVisual::Era1985) => 2.15,
        };

        // Terreno adverso reduz ligeiramente a capacidade de estradas e ferrovias
        let mult_terreno = if matches!(
            self.modal,
            TipoInfraestrutura::Estradas | TipoInfraestrutura::Ferrovias
        ) {
            1.0 / modificador_terreno.max(0.5)
        } else {
            1.0
        };

        let cap_base = (self.nivel as f64) * self.capacidade_base_por_nivel;
        let fator_integridade = (1.0 - self.dano.clamp(0.0, 1.0)).max(0.0);
        self.capacidade_efetiva =
            (cap_base * mult_era * mult_terreno + bonus_predios) * fator_integridade;
    }

    /// Repara dano diário gradualmente.
    pub fn reparar_dano(&mut self, taxa_diaria: f64) {
        if self.dano > 0.0 {
            self.dano = (self.dano - taxa_diaria).max(0.0);
        }
    }
}

/// Conjunto integrado dos 5 modais de infraestrutura de um estado.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RedeInfraestruturaEstado {
    pub estradas: EstadoModalInfra,
    pub ferrovias: EstadoModalInfra,
    pub portos: EstadoModalInfra,
    pub aeroportos: EstadoModalInfra,
    pub rede_eletrica: EstadoModalInfra,
    /// Indica se o estado tem acesso à costa marítima ou via navegável oceânica.
    pub eh_costeiro: bool,
}

impl RedeInfraestruturaEstado {
    pub fn novo(eh_costeiro: bool, nivel_base: u32) -> Self {
        let nivel_portos = if eh_costeiro { nivel_base } else { 0 };
        Self {
            estradas: EstadoModalInfra::novo(TipoInfraestrutura::Estradas, nivel_base),
            ferrovias: EstadoModalInfra::novo(TipoInfraestrutura::Ferrovias, nivel_base),
            portos: EstadoModalInfra::novo(TipoInfraestrutura::Portos, nivel_portos),
            aeroportos: EstadoModalInfra::novo(
                TipoInfraestrutura::Aeroportos,
                nivel_base.saturating_sub(1).max(1),
            ),
            rede_eletrica: EstadoModalInfra::novo(TipoInfraestrutura::RedeEletrica, nivel_base),
            eh_costeiro,
        }
    }

    /// Retorna referência ao modal indicado.
    pub fn modal(&self, m: TipoInfraestrutura) -> &EstadoModalInfra {
        match m {
            TipoInfraestrutura::Estradas => &self.estradas,
            TipoInfraestrutura::Ferrovias => &self.ferrovias,
            TipoInfraestrutura::Portos => &self.portos,
            TipoInfraestrutura::Aeroportos => &self.aeroportos,
            TipoInfraestrutura::RedeEletrica => &self.rede_eletrica,
        }
    }

    /// Retorna referência mutável ao modal indicado.
    pub fn modal_mut(&mut self, m: TipoInfraestrutura) -> &mut EstadoModalInfra {
        match m {
            TipoInfraestrutura::Estradas => &mut self.estradas,
            TipoInfraestrutura::Ferrovias => &mut self.ferrovias,
            TipoInfraestrutura::Portos => &mut self.portos,
            TipoInfraestrutura::Aeroportos => &mut self.aeroportos,
            TipoInfraestrutura::RedeEletrica => &mut self.rede_eletrica,
        }
    }

    /// Zera o uso registrado em todos os modais para novo cálculo diário.
    pub fn zerar_uso(&mut self) {
        self.estradas.uso_atual = 0.0;
        self.ferrovias.uso_atual = 0.0;
        self.portos.uso_atual = 0.0;
        self.aeroportos.uso_atual = 0.0;
        self.rede_eletrica.uso_atual = 0.0;
    }

    /// Adiciona demanda de uso a um modal específico.
    pub fn adicionar_uso(&mut self, m: TipoInfraestrutura, qtd: f64) {
        if qtd > 0.0 {
            self.modal_mut(m).uso_atual += qtd;
        }
    }

    /// Recalcula todas as capacidades efetivas.
    pub fn recalcular_capacidades(
        &mut self,
        terreno: TipoTerreno,
        era: EraTecnologicaVisual,
        bonus_geracao_eletrica: f64,
        bonus_capacidade_portos: f64,
    ) {
        let mod_terreno = terreno.modificador_infraestrutura();
        self.estradas.recalcular_capacidade(mod_terreno, era, 0.0);
        self.ferrovias.recalcular_capacidade(mod_terreno, era, 0.0);

        let cap_portos_bonus = if self.eh_costeiro {
            bonus_capacidade_portos
        } else {
            0.0
        };
        self.portos
            .recalcular_capacidade(1.0, era, cap_portos_bonus);
        if !self.eh_costeiro {
            self.portos.capacidade_efetiva = 0.0;
        }

        self.aeroportos.recalcular_capacidade(1.0, era, 0.0);
        self.rede_eletrica
            .recalcular_capacidade(1.0, era, bonus_geracao_eletrica);
    }

    /// Cobertura e suficiência da rede elétrica no estado (0.0 a 1.0).
    /// Se o consumo superar a geração, cai suavemente, causando apagões e penalidade fabril.
    #[inline]
    pub fn cobertura_eletrica(&self) -> f64 {
        if self.rede_eletrica.uso_atual <= 0.0 {
            1.0
        } else if self.rede_eletrica.capacidade_efetiva <= 0.0 {
            0.0
        } else {
            (self.rede_eletrica.capacidade_efetiva / self.rede_eletrica.uso_atual).clamp(0.0, 1.0)
        }
    }

    /// Índice de acesso ao mercado (0.00 a 1.00) diretamente compatível com `sim_economia`.
    /// Determinado pela capacidade de escoamento terrestre e marítimo.
    pub fn acesso_mercado(&self) -> f64 {
        // Transporte terrestre somado
        let cap_terrestre = self.estradas.capacidade_efetiva + self.ferrovias.capacidade_efetiva;
        let uso_terrestre = self.estradas.uso_atual + self.ferrovias.uso_atual;

        let razao_terrestre = if uso_terrestre <= 0.0 {
            1.0
        } else if cap_terrestre <= 0.0 {
            0.15
        } else {
            (cap_terrestre / uso_terrestre).clamp(0.15, 1.0)
        };

        // Se for costeiro, o porto amortece gargalos terrestres
        if self.eh_costeiro && self.portos.capacidade_efetiva > 0.0 {
            let razao_porto = if self.portos.uso_atual <= 0.0 {
                1.0
            } else {
                (self.portos.capacidade_efetiva / self.portos.uso_atual).clamp(0.20, 1.0)
            };
            (0.70 * razao_terrestre + 0.30 * razao_porto).clamp(0.20, 1.0)
        } else {
            razao_terrestre
        }
    }

    /// Conectividade logística integrada (0.0 a 1.0) para expansão de cidades e suprimento militar.
    pub fn conectividade_logistica(&self) -> f64 {
        let tx_est = (1.0 - (self.estradas.taxa_utilizacao() - 1.0).max(0.0)).clamp(0.2, 1.0);
        let tx_fer = (1.0 - (self.ferrovias.taxa_utilizacao() - 1.0).max(0.0)).clamp(0.2, 1.0);
        let tx_ele = self.cobertura_eletrica();
        let tx_aero = (1.0 - (self.aeroportos.taxa_utilizacao() - 1.0).max(0.0)).clamp(0.2, 1.0);

        0.35 * tx_est + 0.30 * tx_fer + 0.20 * tx_ele + 0.15 * tx_aero
    }

    /// Capacidade máxima diária de suprimento militar antes de atrição logística (estilo HoI4).
    pub fn capacidade_suprimento_militar(&self) -> f64 {
        let base_terrestre =
            self.estradas.capacidade_efetiva * 0.4 + self.ferrovias.capacidade_efetiva * 0.8;
        let base_portuaria = if self.eh_costeiro {
            self.portos.capacidade_efetiva * 1.2
        } else {
            0.0
        };
        let base_aerea = self.aeroportos.capacidade_efetiva * 0.5;

        (base_terrestre + base_portuaria + base_aerea).max(10.0)
    }

    /// Aplica dano percentual (ex: bombardeio de guerra) a toda a malha.
    pub fn aplicar_dano_geral(&mut self, delta: f64) {
        let d = delta.clamp(0.0, 1.0);
        self.estradas.dano = (self.estradas.dano + d).clamp(0.0, 1.0);
        self.ferrovias.dano = (self.ferrovias.dano + d).clamp(0.0, 1.0);
        self.portos.dano = (self.portos.dano + d).clamp(0.0, 1.0);
        self.aeroportos.dano = (self.aeroportos.dano + d).clamp(0.0, 1.0);
        self.rede_eletrica.dano = (self.rede_eletrica.dano + d).clamp(0.0, 1.0);
    }

    /// Repara gradualmente todos os modais da malha.
    pub fn reparar_geral(&mut self, taxa_diaria: f64) {
        self.estradas.reparar_dano(taxa_diaria);
        self.ferrovias.reparar_dano(taxa_diaria);
        self.portos.reparar_dano(taxa_diaria);
        self.aeroportos.reparar_dano(taxa_diaria);
        self.rede_eletrica.reparar_dano(taxa_diaria);
    }
}
