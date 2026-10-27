//! View-model para a Tela de Política, Leis e Grupos de Interesse (F6).
//! 7 categorias institucionais de leis, tramitação parlamentar e 11 IGs.

use bevy_egui::egui::Color32;
use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeiAtivaViewModel {
    pub categoria_id: String,
    pub categoria_nome: String,
    pub lei_id: String,
    pub lei_nome: String,
    pub descricao_efeito: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeiEmTramitacaoViewModel {
    pub nome_projeto: String,
    pub fase_atual: String, // ex: "Debate no Senado (Fase 2 de 3)"
    pub progresso_pct: f32,
    pub dias_para_proxima_votacao: u32,
    pub probabilidade_aprovacao_pct: f32,
    pub probabilidade_emendas_pct: f32,
    pub probabilidade_bloqueio_pct: f32,
    pub apoiadores: Vec<String>,
    pub opositores: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrupoInteresseViewModel {
    pub id: String,
    pub nome: String,
    pub forca_politica_clout_pct: f32,
    pub aprovacao: i32, // -20 a +20
    pub estado_lealdade: String, // "Leal", "Contente", "Neutro", "Descontente", "Insurrecto"
    pub eh_influente: bool,
    pub esta_no_governo: bool,
    pub base_social_descricao: String,
    pub pautas_principais: String,
}

impl GrupoInteresseViewModel {
    pub fn cor_aprovacao(&self) -> Color32 {
        if self.aprovacao >= 5 {
            Color32::from_rgb(76, 138, 77)
        } else if self.aprovacao <= -5 {
            Color32::from_rgb(184, 50, 40)
        } else {
            Color32::from_rgb(200, 134, 39)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Resource)]
pub struct PoliticaViewModel {
    pub legitimidade_governo_pct: f32,
    pub classificacao_governo: String,
    pub leis_ativas: Vec<LeiAtivaViewModel>,
    pub lei_em_tramitacao: Option<LeiEmTramitacaoViewModel>,
    pub grupos_interesse: Vec<GrupoInteresseViewModel>,
    pub bancadas_congresso: Vec<(String, f32, (u8, u8, u8))>,
}

impl Default for PoliticaViewModel {
    fn default() -> Self {
        Self {
            legitimidade_governo_pct: 86.0,
            classificacao_governo: "Governo Firme e Legítimo".to_string(),
            leis_ativas: vec![
                LeiAtivaViewModel {
                    categoria_id: "forma_governo".to_string(),
                    categoria_nome: "Forma de Governo".to_string(),
                    lei_id: "republica_presidencialista".to_string(),
                    lei_nome: "República Presidencialista Constitucional".to_string(),
                    descricao_efeito: "+10 Legitimidade, eleições a cada 4 anos, separação rígida de poderes.".to_string(),
                },
                LeiAtivaViewModel {
                    categoria_id: "economia".to_string(),
                    categoria_nome: "Sistema Econômico".to_string(),
                    lei_id: "capitalismo_de_intervencao".to_string(),
                    lei_nome: "Capitalismo de Intervenção (New Deal)".to_string(),
                    descricao_efeito: "+25% Reinvestimento capitalista, permite subsídios e obras de infraestrutura.".to_string(),
                },
                LeiAtivaViewModel {
                    categoria_id: "trabalho".to_string(),
                    categoria_nome: "Relações Trabalhistas".to_string(),
                    lei_id: "regulamentacao_basica".to_string(),
                    lei_nome: "Regulamentação Básica & Ato Wagner".to_string(),
                    descricao_efeito: "Piso salarial de 60% da média, -20% mortalidade de operários, sindicatos protegidos.".to_string(),
                },
                LeiAtivaViewModel {
                    categoria_id: "direitos_civis".to_string(),
                    categoria_nome: "Direitos Civis e Segurança".to_string(),
                    lei_id: "liberdade_de_expressao".to_string(),
                    lei_nome: "Liberdade de Expressão e Imprensa Livre".to_string(),
                    descricao_efeito: "+25% Inovação e difusão de ideias; proíbe supressão forçada de partidos legais.".to_string(),
                },
                LeiAtivaViewModel {
                    categoria_id: "sistema_tributario".to_string(),
                    categoria_nome: "Sistema Tributário".to_string(),
                    lei_id: "tributacao_progressiva".to_string(),
                    lei_nome: "Tributação Progressiva de Renda de Guerra".to_string(),
                    descricao_efeito: "Alíquota de até 94% sobre lucros corporativos e grandes rendas; alivia o consumo.".to_string(),
                },
                LeiAtivaViewModel {
                    categoria_id: "saude_e_educacao".to_string(),
                    categoria_nome: "Saúde e Educação".to_string(),
                    lei_id: "sistema_publico_basico".to_string(),
                    lei_nome: "Sistema Público Básico & Escolas Comunitárias".to_string(),
                    descricao_efeito: "+30% Velocidade de alfabetização e -2.0% mortalidade infantil.".to_string(),
                },
                LeiAtivaViewModel {
                    categoria_id: "exercito".to_string(),
                    categoria_nome: "Organização Militar".to_string(),
                    lei_id: "mobilizacao_total_de_guerra".to_string(),
                    lei_nome: "Mobilização Geral de Guerra (Selective Service)".to_string(),
                    descricao_efeito: "Conscrição ampla; alto consumo de suprimentos com capacidade de mobilização recorde.".to_string(),
                },
            ],
            lei_em_tramitacao: Some(LeiEmTramitacaoViewModel {
                nome_projeto: "Expansão do GI Bill e Reconversão Econômica".to_string(),
                fase_atual: "Debate no Senado (Fase 2 de 3)".to_string(),
                progresso_pct: 68.0,
                dias_para_proxima_votacao: 42,
                probabilidade_aprovacao_pct: 74.0,
                probabilidade_emendas_pct: 18.0,
                probabilidade_bloqueio_pct: 8.0,
                apoiadores: vec!["Sindicatos Operários".to_string(), "Intelectuais".to_string(), "Veteranos de Guerra".to_string()],
                opositores: vec!["Industriais Conservadores".to_string()],
            }),
            grupos_interesse: vec![
                GrupoInteresseViewModel {
                    id: "industriais".to_string(),
                    nome: "Burguesia Industrial e Finanças".to_string(),
                    forca_politica_clout_pct: 28.0,
                    aprovacao: 4,
                    estado_lealdade: "Contente".to_string(),
                    eh_influente: true,
                    esta_no_governo: true,
                    base_social_descricao: "Capitalistas, grandes industriais e armadores navais".to_string(),
                    pautas_principais: "Livre comércio, incentivos corporativos e alinhamento ocidental".to_string(),
                },
                GrupoInteresseViewModel {
                    id: "trabalhadores_sindicalizados".to_string(),
                    nome: "Sindicatos Operários (AFL-CIO)".to_string(),
                    forca_politica_clout_pct: 24.0,
                    aprovacao: -1,
                    estado_lealdade: "Tensão por Teto Salarial".to_string(),
                    eh_influente: true,
                    esta_no_governo: true,
                    base_social_descricao: "Operários metalúrgicos, maquinistas e portuários".to_string(),
                    pautas_principais: "Fim do congelamento salarial de guerra e direitos trabalhistas plenos".to_string(),
                },
                GrupoInteresseViewModel {
                    id: "militares".to_string(),
                    nome: "Estado-Maior das Forças Armadas".to_string(),
                    forca_politica_clout_pct: 22.0,
                    aprovacao: 8,
                    estado_lealdade: "Entusiasta".to_string(),
                    eh_influente: true,
                    esta_no_governo: true,
                    base_social_descricao: "Oficiais generais, almirantes e corpos veteranos".to_string(),
                    pautas_principais: "Vitória incondicional no Eixo e investimento no Projeto Manhattan".to_string(),
                },
                GrupoInteresseViewModel {
                    id: "intelectuais".to_string(),
                    nome: "Intelectuais e Cientistas".to_string(),
                    forca_politica_clout_pct: 14.0,
                    aprovacao: 3,
                    estado_lealdade: "Apoiador".to_string(),
                    eh_influente: false,
                    esta_no_governo: false,
                    base_social_descricao: "Acadêmicos, cientistas e jornalistas de renome".to_string(),
                    pautas_principais: "Criação da ONU, liberdade acadêmica e cooperação científica global".to_string(),
                },
                GrupoInteresseViewModel {
                    id: "populacao_rural".to_string(),
                    nome: "Produtores Agrícolas & Fazendeiros".to_string(),
                    forca_politica_clout_pct: 12.0,
                    aprovacao: 2,
                    estado_lealdade: "Satisfeito".to_string(),
                    eh_influente: false,
                    esta_no_governo: false,
                    base_social_descricao: "Fazendeiros do Meio-Oeste e pequenos proprietários rurais".to_string(),
                    pautas_principais: "Preços mínimos garantidos para grãos e eletrificação rural".to_string(),
                },
            ],
            bancadas_congresso: vec![
                ("Partido Democrata (Maioria FDR)".to_string(), 53.4, (41, 92, 143)),
                ("Partido Republicano (Oposição Leal)".to_string(), 45.9, (170, 41, 36)),
                ("Independentes".to_string(), 0.7, (108, 127, 95)),
            ],
        }
    }
}
