//! View-model para a Tela de Diplomacia e Esferas de Influência (F7).

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcaoDiplomaticaViewModel {
    pub id: String,
    pub nome: String,
    pub custo_prestigio: u32,
    pub descricao: String,
    pub disponivel: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct DiplomaciaViewModel {
    pub pib_bloco_ocidental_bilhoes: f64,
    pub divisoes_bloco_ocidental: u32,
    pub pib_bloco_sovietico_bilhoes: f64,
    pub divisoes_bloco_sovietico: u32,
    pub nivel_defcon: u8,
    pub relacao_bilateral_urss: i32, // -100 a +100 (ex: -35)
    pub status_relacao_urss: String,
    pub capitais_em_disputa: Vec<String>,
    pub acoes_disponiveis: Vec<AcaoDiplomaticaViewModel>,
}

impl Default for DiplomaciaViewModel {
    fn default() -> Self {
        Self {
            pib_bloco_ocidental_bilhoes: 410.5,
            divisoes_bloco_ocidental: 320,
            pib_bloco_sovietico_bilhoes: 145.2,
            divisoes_bloco_sovietico: 540,
            nivel_defcon: 4,
            relacao_bilateral_urss: -35,
            status_relacao_urss: "Desconfiança Mútua & Aliança Antifascista Tensa".to_string(),
            capitais_em_disputa: vec![
                "Berlim (Divisão Quadripartite pendente)".to_string(),
                "Varsóvia (Governo pró-soviético de Lublin)".to_string(),
                "Viena (Zonas de Ocupação da Áustria)".to_string(),
                "Atenas (Guerra Civil iminente)".to_string(),
            ],
            acoes_disponiveis: vec![
                AcaoDiplomaticaViewModel {
                    id: "cupula_yalta".to_string(),
                    nome: "Convocar Cúpula dos Três Grandes em Yalta".to_string(),
                    custo_prestigio: 150,
                    descricao: "Reúne Roosevelt, Churchill e Stalin para definir as esferas de influência pós-guerra.".to_string(),
                    disponivel: true,
                },
                AcaoDiplomaticaViewModel {
                    id: "ajuste_lend_lease".to_string(),
                    nome: "Negociar Extensão do Lend-Lease com Moscou".to_string(),
                    custo_prestigio: 75,
                    descricao: "Oferece caminhões e suprimentos em troca de cooperação na fundação da ONU.".to_string(),
                    disponivel: true,
                },
                AcaoDiplomaticaViewModel {
                    id: "operacoes_oss".to_string(),
                    nome: "Infiltrar Operativos da OSS na Europa Central".to_string(),
                    custo_prestigio: 100,
                    descricao: "Mapeia movimentações de divisões blindadas do Exército Vermelho na Polônia.".to_string(),
                    disponivel: true,
                },
                AcaoDiplomaticaViewModel {
                    id: "pacto_nao_agressao".to_string(),
                    nome: "Acordo de Delimitação de Fronteiras Marítimas".to_string(),
                    custo_prestigio: 50,
                    descricao: "Evita incidentes navais entre a Frota do Pacífico dos EUA e submarinos soviéticos.".to_string(),
                    disponivel: true,
                },
            ],
        }
    }
}
