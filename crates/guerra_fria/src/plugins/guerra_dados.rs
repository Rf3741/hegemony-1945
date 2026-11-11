//! Catálogos e definições orientados a dados para o sistema de guerra:
//! Casus Belli, unidades militares de design, generais históricos e guerras históricas.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Registro de um Casus Belli (`casus_belli.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasusBelliDef {
    pub id: String,
    pub nome: String,
    pub descricao: String,
    pub tensao_mundial_gerada: f32,
    pub custo_justificacao_dias: u16,
    pub exige_reivindicacao_territorial: bool,
    pub permite_intervencao_superpotencias: bool,
    pub objetivos_permitidos: Vec<String>,
    pub penalidade_infamia: f32,
}

/// Registro de um Arquétipo de Unidade Militar (`unidades_design.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnidadeDef {
    pub id: String,
    pub nome: String,
    pub ramo: String,
    pub categoria: String,
    pub era: String,
    pub efetivo_homens: u32,
    pub ataque: f32,
    pub defesa: f32,
    pub blindagem: f32,
    pub penetracao: f32,
    pub velocidade_kmh: f32,
    pub peso_suprimento: f32,
    pub consumo_diario: BTreeMap<String, f32>,
    pub custo_equipamento: BTreeMap<String, f32>,
    pub tempo_treinamento_dias: u16,
}

/// Registro de Comandante Militar Histórico (`generais.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralDef {
    pub id: String,
    pub nome: String,
    pub pais_tag: String,
    pub ramo: String,
    pub posto: String,
    pub habilidade_comando: u8,
    pub ataque: u8,
    pub defesa: u8,
    pub logistica: u8,
    pub planejamento: u8,
    pub tracos: Vec<String>,
    pub ano_inicio: u16,
    pub ano_fim: u16,
}

/// Registro de beligerante em conflito histórico.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeligeranteDef {
    pub tag: String,
    pub papel: String,
    pub lider_alianca: bool,
}

/// Patrocinador ou apoio de superpotência.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatrocinadorDef {
    pub potencia: String,
    pub tipo_ajuda: String,
    pub intensidade: f32,
}

/// Data simples para gatilhos de conflitos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSimples {
    pub ano: u16,
    pub mes: u8,
    pub dia: u8,
}

/// Gatilho de disparo de guerra histórica.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatilhoGuerraDef {
    pub tipo: String,
    pub condicao_data: DataSimples,
    pub condicao_tensao_minima: f32,
    pub descricao: String,
}

/// Registro de Conflito Histórico (`guerras_historicas.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuerraHistoricaDef {
    pub id: String,
    pub nome: String,
    pub ano_inicio: u16,
    pub mes_inicio: u8,
    pub dia_inicio: u8,
    pub ano_fim_historico: u16,
    pub mes_fim_historico: u8,
    pub dia_fim_historico: u8,
    pub tipo_conflito: String,
    pub casus_belli_id: String,
    pub beligerantes_lado_a: Vec<BeligeranteDef>,
    pub beligerantes_lado_b: Vec<BeligeranteDef>,
    pub patrocinadores_superpotencias: BTreeMap<String, PatrocinadorDef>,
    pub teatros_iniciais_provincias: Vec<String>,
    pub gatilho_disparo: GatilhoGuerraDef,
    pub condicoes_vitoria_lado_a: Vec<BTreeMap<String, ron::Value>>,
    pub condicoes_vitoria_lado_b: Vec<BTreeMap<String, ron::Value>>,
    pub impacto_tensao_mundial_vitoria: f32,
    pub consequencia_historica_texto: String,
}

/// Catálogo completo de dados de design de guerra carregados dos arquivos RON.
#[derive(Debug, Clone, Default)]
pub struct CatalogoDesignGuerra {
    pub casus_belli: Vec<CasusBelliDef>,
    _unidades: Vec<UnidadeDef>,
    pub generais: Vec<GeneralDef>,
    pub guerras_historicas: Vec<GuerraHistoricaDef>,
}

impl CatalogoDesignGuerra {
    pub fn carregar_de_diretorio(dir: &Path) -> Result<Self, String> {
        let cb_path = dir.join("casus_belli.ron");
        let gen_path = dir.join("generais.ron");
        let un_path = dir.join("unidades_design.ron");
        let gh_path = dir.join("guerras_historicas.ron");

        let casus_belli: Vec<CasusBelliDef> = if cb_path.exists() {
            let s = std::fs::read_to_string(&cb_path).map_err(|e| format!("Erro lendo casus_belli.ron: {e}"))?;
            ron::from_str(&s).map_err(|e| format!("Erro decodificando casus_belli.ron: {e}"))?
        } else {
            Vec::new()
        };

        let generais: Vec<GeneralDef> = if gen_path.exists() {
            let s = std::fs::read_to_string(&gen_path).map_err(|e| format!("Erro lendo generais.ron: {e}"))?;
            ron::from_str(&s).map_err(|e| format!("Erro decodificando generais.ron: {e}"))?
        } else {
            Vec::new()
        };

        let unidades: Vec<UnidadeDef> = if un_path.exists() {
            let s = std::fs::read_to_string(&un_path).map_err(|e| format!("Erro lendo unidades_design.ron: {e}"))?;
            ron::from_str(&s).map_err(|e| format!("Erro decodificando unidades_design.ron: {e}"))?
        } else {
            Vec::new()
        };

        let guerras_historicas: Vec<GuerraHistoricaDef> = if gh_path.exists() {
            let s = std::fs::read_to_string(&gh_path).map_err(|e| format!("Erro lendo guerras_historicas.ron: {e}"))?;
            ron::from_str(&s).map_err(|e| format!("Erro decodificando guerras_historicas.ron: {e}"))?
        } else {
            Vec::new()
        };

        Ok(Self {
            casus_belli,
            _unidades: unidades,
            generais,
            guerras_historicas,
        })
    }
}
