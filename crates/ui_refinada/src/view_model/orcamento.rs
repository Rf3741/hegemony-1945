//! View-model para a Tela de Orçamento e Tesouro (F5).
//! Sliders interativos com cálculo dinâmico de receitas, despesas e juros da dívida.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct OrcamentoViewModel {
    // Alíquotas Tributárias (Sliders)
    pub imposto_renda_alta_pct: f32,
    pub imposto_classe_media_pct: f32,
    pub imposto_trabalhadores_pct: f32,
    pub tarifa_importacao_pct: f32,
    pub imposto_consumo_pct: f32,

    // Despesas Públicas Semanais em Milhões (Sliders)
    pub despesa_forcas_armadas_milhoes: f32,
    pub despesa_pesquisa_manhattan_milhoes: f32,
    pub despesa_burocracia_federal_milhoes: f32,
    pub despesa_subsidios_industriais_milhoes: f32,
    pub despesa_veteranos_gi_bill_milhoes: f32,

    // Balanço de Dívida e Títulos de Guerra
    pub divida_publica_bilhoes: f64,
    pub titulos_guerra_inscritos_bilhoes: f64,
    pub teto_endividamento_bilhoes: f64,
    pub taxa_juros_base_pct: f32,
}

impl OrcamentoViewModel {
    /// Calcula a receita semanal estimada baseada nas alíquotas.
    pub fn receita_semanal_estimada(&self) -> f64 {
        let base_alta = 96.8 * (self.imposto_renda_alta_pct as f64 / 88.0);
        let base_media = 42.4 * (self.imposto_classe_media_pct as f64 / 26.0);
        let base_trabalhadores = 32.1 * (self.imposto_trabalhadores_pct as f64 / 18.0);
        let base_tarifas = 14.3 * (self.tarifa_importacao_pct as f64 / 15.0);
        let base_consumo = 18.2 * (self.imposto_consumo_pct as f64 / 12.0);
        base_alta + base_media + base_trabalhadores + base_tarifas + base_consumo
    }

    /// Calcula a despesa semanal total somando alocações governamentais e serviço da dívida.
    pub fn despesa_semanal_estimada(&self) -> f64 {
        let operacionais = self.despesa_forcas_armadas_milhoes as f64
            + self.despesa_pesquisa_manhattan_milhoes as f64
            + self.despesa_burocracia_federal_milhoes as f64
            + self.despesa_subsidios_industriais_milhoes as f64
            + self.despesa_veteranos_gi_bill_milhoes as f64;
        
        // Juros semanais da dívida: Dívida * taxa anual / 52
        let juros_semanais = (self.divida_publica_bilhoes * 1000.0) * (self.taxa_juros_efetiva_pct() as f64 / 100.0) / 52.0;
        operacionais + juros_semanais
    }

    /// Saldo líquido semanal (+/- milhoes).
    pub fn saldo_liquido_semanal(&self) -> f64 {
        self.receita_semanal_estimada() - self.despesa_semanal_estimada()
    }

    /// Taxa de juros efetiva da dívida considerando o risco fiscal.
    pub fn taxa_juros_efetiva_pct(&self) -> f32 {
        let razao_endividamento = (self.divida_publica_bilhoes / self.teto_endividamento_bilhoes).clamp(0.0, 1.5) as f32;
        self.taxa_juros_base_pct + 1.5 * (razao_endividamento * razao_endividamento)
    }
}

impl Default for OrcamentoViewModel {
    fn default() -> Self {
        Self {
            imposto_renda_alta_pct: 88.0,
            imposto_classe_media_pct: 26.0,
            imposto_trabalhadores_pct: 18.0,
            tarifa_importacao_pct: 15.0,
            imposto_consumo_pct: 12.0,
            despesa_forcas_armadas_milhoes: 82.5,
            despesa_pesquisa_manhattan_milhoes: 21.5,
            despesa_burocracia_federal_milhoes: 12.8,
            despesa_subsidios_industriais_milhoes: 11.2,
            despesa_veteranos_gi_bill_milhoes: 15.0,
            divida_publica_bilhoes: 258.7,
            titulos_guerra_inscritos_bilhoes: 185.0,
            teto_endividamento_bilhoes: 300.0,
            taxa_juros_base_pct: 2.0,
        }
    }
}
