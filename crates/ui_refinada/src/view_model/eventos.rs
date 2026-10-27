//! View-model para o Pop-up de Evento Histórico Modal (ex: Conferência de Yalta).

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcaoEventoViewModel {
    pub id: String,
    pub titulo: String,
    pub texto_efeito: String,
    pub delta_prestigio: i32,
    pub delta_tensao_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct EventoModalViewModel {
    pub aberto: bool,
    pub id: String,
    pub carimbo: String,
    pub titulo: String,
    pub subtitulo: String,
    pub data_evento: String,
    pub texto_narrativo: String,
    pub opcoes: Vec<OpcaoEventoViewModel>,
}

impl Default for EventoModalViewModel {
    fn default() -> Self {
        Self {
            aberto: false, // Pode ser acionado pelo jogador ou por evento
            id: "evento_yalta_1945".to_string(),
            carimbo: "ULTRASSECRETO".to_string(),
            titulo: "A Conferência de Yalta (Fevereiro de 1945)".to_string(),
            subtitulo: "Os Três Grandes: Roosevelt, Churchill e Stalin no Palácio de Livadia".to_string(),
            data_evento: "04 FEB 1945".to_string(),
            texto_narrativo: "Com a derrota da Alemanha nazista iminente no leste e no oeste, os líderes das três potências aliadas reúnem-se na Crimeia para definir a partilha do pós-guerra. Em pauta: as zonas de ocupação na Alemanha, o destino das fronteiras e do governo da Polônia, a fundação da Organização das Nações Unidas (ONU) e a entrada soviética na guerra contra o Japão imperial no Pacífico.".to_string(),
            opcoes: vec![
                OpcaoEventoViewModel {
                    id: "opcao_a_historica".to_string(),
                    titulo: "Opção A: Aceitar a Declaração sobre a Europa Libertada e a adesão da URSS à ONU".to_string(),
                    texto_efeito: "+150 Prestígio Internacional, -5% Tensão DEFCON. A URSS compromete-se a intervir contra o Japão três meses após a queda de Berlim.".to_string(),
                    delta_prestigio: 150,
                    delta_tensao_pct: -5.0,
                },
                OpcaoEventoViewModel {
                    id: "opcao_b_linhadura".to_string(),
                    titulo: "Opção B: Exigir eleições imediatas na Polônia e suspender remessas do Lend-Lease".to_string(),
                    texto_efeito: "+25% Tensão Global (DEFCON 3). O Exército Vermelho consolida o controle unilateral e acelera a corrida por Berlim.".to_string(),
                    delta_prestigio: 50,
                    delta_tensao_pct: 25.0,
                },
                OpcaoEventoViewModel {
                    id: "opcao_c_pacifico".to_string(),
                    titulo: "Opção C: Priorizar o Teatro do Pacífico e conceder concessões territoriais nas Ilhas Curilas".to_string(),
                    texto_efeito: "+80 Prestígio. Reduz as baixas projetadas das tropas americanas na invasão do Japão, ao custo de atrito com Londres.".to_string(),
                    delta_prestigio: 80,
                    delta_tensao_pct: 5.0,
                },
            ],
        }
    }
}
