//! View-model para o Painel de Notificações e Despachos por Telegrama.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoTelegrama {
    Critico,     // Borda vermelha (invasão, ultimato, default)
    Importante,  // Borda amarela (veto na ONU, queda de legitimidade)
    Oportunidade,// Borda verde (tratado comercial, avanço de pesquisa)
    Informativo, // Borda cinza/azul (telex de rotina)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramaViewModel {
    pub id: u64,
    pub data_hora: String,
    pub tag_origem: String,
    pub remetente: String,
    pub carimbo: String,
    pub assunto: String,
    pub mensagem_completa: String,
    pub tipo: TipoTelegrama,
    pub lido: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct NotificacoesViewModel {
    pub telegramas: Vec<TelegramaViewModel>,
    pub gaveta_expandida: bool,
}

impl Default for NotificacoesViewModel {
    fn default() -> Self {
        Self {
            gaveta_expandida: true,
            telegramas: vec![
                TelegramaViewModel {
                    id: 1,
                    data_hora: "01/01/1945 - 08:30".to_string(),
                    tag_origem: "SOV".to_string(),
                    remetente: "Comissariado do Povo para Relações Exteriores (Moscou)".to_string(),
                    carimbo: "CONFIDENCIAL".to_string(),
                    assunto: "Despacho do Kremlin: Preparativos para Yalta".to_string(),
                    mensagem_completa: "Stalin manifesta concordância prévia com a data de fevereiro para a reunião dos Três Grandes na Crimeia, mas reitera a exigência de reconhecimento do Comitê de Lublin na Polônia.".to_string(),
                    tipo: TipoTelegrama::Importante,
                    lido: false,
                },
                TelegramaViewModel {
                    id: 2,
                    data_hora: "01/01/1945 - 11:15".to_string(),
                    tag_origem: "GBR".to_string(),
                    remetente: "Gabinete de Guerra de Churchill (Londres)".to_string(),
                    carimbo: "URGENTE".to_string(),
                    assunto: "Situação Operacional nas Ardenas e Suprimentos".to_string(),
                    mensagem_completa: "O avanço aliado no front ocidental estabilizou a Linha Siegfried após a contraofensiva alemã nas Ardenas. Londres solicita remessa acelerada de 100.000 t de combustível de aviação.".to_string(),
                    tipo: TipoTelegrama::Oportunidade,
                    lido: false,
                },
                TelegramaViewModel {
                    id: 3,
                    data_hora: "01/01/1945 - 14:00".to_string(),
                    tag_origem: "USA".to_string(),
                    remetente: "War Production Board & Gabinete Fiscal (Washington)".to_string(),
                    carimbo: "ULTRASSECRETO".to_string(),
                    assunto: "Relatório de Produção Bélica e Alocação do Tesouro".to_string(),
                    mensagem_completa: "As siderúrgicas de Pittsburgh atingiram novo recorde semanal de produção. O Tesouro confirma superávit operacional com a arrecadação da 7ª Série de Títulos de Guerra.".to_string(),
                    tipo: TipoTelegrama::Critico,
                    lido: false,
                },
            ],
        }
    }
}
