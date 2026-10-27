//! Dinâmica de descolonização, fervor anticolonial e secessão de estados
//! (independência negociada, partição pós-colonial e guerras de libertação).

use crate::catalogo::CatalogoTerritorio;
use crate::entidades::{Estado, MovimentoIndependencia, PaisTerritorial, Regiao};
use crate::tipos::{ModoTransferenciaTerritorial, ViaDescolonizacao};
use crate::transferencia::transferir_estado;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Relatório com os resultados políticos e demográficos de uma descolonização ou secessão.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelatorioDescolonizacao {
    pub via: ViaDescolonizacao,
    pub pais_emancipado_tag: String,
    pub metropole_tag: String,
    pub estados_emancipados: Vec<String>,
    pub populacao_total_emancipada: u64,
    pub refugiados_ou_repatriados: u64,
    pub descricao: String,
}

/// Atualiza o crescimento diário do fervor de independência de um movimento anticolonial.
pub fn atualizar_fervor_diario(
    movimento: &mut MovimentoIndependencia,
    metropole: &PaisTerritorial,
    media_alfabetizacao: f64,
    media_agitacao: f64,
    colonias_independentes_no_continente: u32,
) {
    if !movimento.ativo || movimento.fervor_independencia >= 100.0 {
        return;
    }

    // 1. Fatores internos (alfabetização das elites e agitação popular)
    let fator_alfa = media_alfabetizacao * 0.035;
    let fator_agitacao = (media_agitacao / 10.0) * 0.025;

    // 2. Política da metrópole (se for estrita, acelera o descontentamento)
    let fator_politica = match metropole.lei_cidadania {
        crate::tipos::LeiCidadania::DominioColonialEstrito
        | crate::tipos::LeiCidadania::SegregacaoApartheid => 0.040,
        crate::tipos::LeiCidadania::CidadaniaEtnicaNacional => 0.025,
        crate::tipos::LeiCidadania::CidadaniaCulturalAmpla => 0.012,
        crate::tipos::LeiCidadania::CidadaniaCivicaJusSoli => 0.006,
        crate::tipos::LeiCidadania::InternacionalismoProletario => 0.005,
    };

    // 3. Apoio financeiro/armado de superpotência externa
    let fator_apoio = if movimento.apoio_externo_tag.is_some() {
        0.030
    } else {
        0.0
    };

    // 4. Efeito dominó continental (outras colônias que já conquistaram soberania)
    let fator_domino = (colonias_independentes_no_continente as f64) * 0.008;

    let delta = 0.010 + fator_alfa + fator_agitacao + fator_politica + fator_apoio + fator_domino;
    movimento.fervor_independencia = (movimento.fervor_independencia + delta).clamp(0.0, 100.0);
}

/// Calcula o custo de manutenção imperial de um território colonial para o tesouro metropolitano.
/// Cresce quadraticamente quando o fervor anticolonial sobe (estilo Victoria 3 / Guerra Fria).
pub fn custo_manutencao_colonial(fervor: f64, custo_base: f64) -> f64 {
    let f = fervor.clamp(0.0, 100.0);
    custo_base * (1.0 + 0.04 * f + 0.0012 * f * f)
}

/// Calcula a eficiência extrativa de recursos e tributação colonial.
/// Cai progressivamente com greves, sabotagens e boicotes nacionalistas.
pub fn eficiencia_extrativa_colonial(fervor: f64) -> f64 {
    let excesso = (fervor - 30.0).max(0.0);
    (1.0 - 0.008 * excesso).clamp(0.15, 1.0)
}

/// Concede independência negociada e pacífica aos estados visados pelo movimento anticolonial.
/// (ex: Descolonização britânica da Nigéria/Gana ou transição francesa para a Comunidade).
pub fn conceder_independencia_negociada(
    movimento: &mut MovimentoIndependencia,
    metropole: &PaisTerritorial,
    novo_pais: &mut PaisTerritorial,
    estados_map: &mut BTreeMap<String, Estado>,
    regioes_por_estado: &mut BTreeMap<String, Vec<Regiao>>,
    catalogo: &CatalogoTerritorio,
) -> RelatorioDescolonizacao {
    novo_pais.suserano_tag = None;
    novo_pais.lei_cidadania = movimento.lei_cidadania_inicial;
    novo_pais.culturas_primarias = movimento.culturas_primarias_pretendente.clone();

    let mut total_pop: u64 = 0;
    let mut total_repatriados: u64 = 0;
    let mut estados_concedidos = Vec::new();

    for estado_id in &movimento.estados_alvo {
        if let Some(estado) = estados_map.get_mut(estado_id) {
            if estado.dono_tag == metropole.tag {
                estado.incorporado = true;
                estado.adicionar_nucleo(novo_pais.tag.clone());

                if let Some(regioes) = regioes_por_estado.get_mut(estado_id) {
                    let mut refs: Vec<&mut Regiao> = regioes.iter_mut().collect();
                    let rel = transferir_estado(
                        estado,
                        &mut refs,
                        novo_pais,
                        metropole,
                        catalogo,
                        ModoTransferenciaTerritorial::IntegracaoPacifica,
                        None,
                    );
                    total_pop += rel.populacao_final;
                    total_repatriados += rel.total_migrantes_ou_expulsos;
                }
                estados_concedidos.push(estado_id.clone());
            }
        }
    }

    movimento.ativo = false;
    movimento.em_guerra = false;

    RelatorioDescolonizacao {
        via: ViaDescolonizacao::IndependenciaNegociada,
        pais_emancipado_tag: novo_pais.tag.clone(),
        metropole_tag: metropole.tag.clone(),
        estados_emancipados: estados_concedidos,
        populacao_total_emancipada: total_pop,
        refugiados_ou_repatriados: total_repatriados,
        descricao: format!(
            "Independência pacífica concedida a {} pela metrópole {}.",
            novo_pais.nome, metropole.nome
        ),
    }
}

/// Executa a partição colonial complexa de um território entre múltiplos países sucessores
/// (ex: Partição da Índia Britânica em Índia IND e Paquistão PAK em 1947).
pub fn executar_particao_colonial(
    metropole: &PaisTerritorial,
    particao_estados: &BTreeMap<String, String>, // estado_id -> novo_dono_tag
    paises_sucessores: &mut BTreeMap<String, PaisTerritorial>,
    estados_map: &mut BTreeMap<String, Estado>,
    regioes_por_estado: &mut BTreeMap<String, Vec<Regiao>>,
    catalogo: &CatalogoTerritorio,
) -> Vec<RelatorioDescolonizacao> {
    let mut relatorios = Vec::new();

    for (estado_id, novo_dono_tag) in particao_estados {
        if let Some(estado) = estados_map.get_mut(estado_id) {
            if let Some(novo_dono) = paises_sucessores.get(novo_dono_tag) {
                let novo_dono_clone = novo_dono.clone();
                estado.incorporado = true;
                estado.adicionar_nucleo(novo_dono_clone.tag.clone());

                let mut total_pop_estado = 0;
                let mut total_deslocados = 0;

                if let Some(regioes) = regioes_por_estado.get_mut(estado_id) {
                    let mut refs: Vec<&mut Regiao> = regioes.iter_mut().collect();
                    let rel = transferir_estado(
                        estado,
                        &mut refs,
                        &novo_dono_clone,
                        metropole,
                        catalogo,
                        ModoTransferenciaTerritorial::ParticaoComTrocaPopulacional,
                        None,
                    );
                    total_pop_estado = rel.populacao_final;
                    total_deslocados = rel.total_migrantes_ou_expulsos;
                }

                relatorios.push(RelatorioDescolonizacao {
                    via: ViaDescolonizacao::ParticaoPosColonial,
                    pais_emancipado_tag: novo_dono_clone.tag,
                    metropole_tag: metropole.tag.clone(),
                    estados_emancipados: vec![estado_id.clone()],
                    populacao_total_emancipada: total_pop_estado,
                    refugiados_ou_repatriados: total_deslocados,
                    descricao: format!(
                        "Partição territorial do estado {} atribuído a {}.",
                        estado.nome, novo_dono.nome
                    ),
                });
            }
        }
    }

    relatorios
}

/// Proclama secessão armada / guerra de libertação nacional quando a metrópole recusa concessões.
pub fn proclamar_secessao_armada(
    movimento: &mut MovimentoIndependencia,
    estados_map: &mut BTreeMap<String, Estado>,
    regioes_por_estado: &mut BTreeMap<String, Vec<Regiao>>,
) {
    movimento.em_guerra = true;
    for estado_id in &movimento.estados_alvo {
        if let Some(estado) = estados_map.get_mut(estado_id) {
            // Os rebeldes assumem o controle de facto das regiões
            estado.controlador_tag = movimento.pais_pretendente_tag.clone();
            estado.rede_infraestrutura.aplicar_dano_geral(0.20);
            estado.agitacao_social = (estado.agitacao_social + 40.0).min(100.0);

            if let Some(regioes) = regioes_por_estado.get_mut(estado_id) {
                for r in regioes.iter_mut() {
                    r.controlador_tag = movimento.pais_pretendente_tag.clone();
                    r.destruicao_guerra = (r.destruicao_guerra + 0.15).clamp(0.0, 1.0);
                }
            }
        }
    }
}

/// Vitória militar de uma guerra de libertação nacional (ex: Argélia 1962, Vietnã 1954).
/// Culmina na expulsão/fuga dos colonos metropolitanos e estabelecimento da soberania plena.
pub fn vitoria_guerra_libertacao(
    movimento: &mut MovimentoIndependencia,
    metropole: &PaisTerritorial,
    novo_pais: &mut PaisTerritorial,
    estados_map: &mut BTreeMap<String, Estado>,
    regioes_por_estado: &mut BTreeMap<String, Vec<Regiao>>,
    catalogo: &CatalogoTerritorio,
) -> RelatorioDescolonizacao {
    novo_pais.suserano_tag = None;
    novo_pais.lei_cidadania = movimento.lei_cidadania_inicial;
    novo_pais.culturas_primarias = movimento.culturas_primarias_pretendente.clone();

    let mut total_pop = 0;
    let mut total_expulsos = 0;
    let mut estados_libertados = Vec::new();

    for estado_id in &movimento.estados_alvo {
        if let Some(estado) = estados_map.get_mut(estado_id) {
            estado.incorporado = true;
            estado.adicionar_nucleo(novo_pais.tag.clone());

            if let Some(regioes) = regioes_por_estado.get_mut(estado_id) {
                let mut refs: Vec<&mut Regiao> = regioes.iter_mut().collect();
                // Guerra de libertação resulta em expulsão ou fuga das elites coloniais
                let rel = transferir_estado(
                    estado,
                    &mut refs,
                    novo_pais,
                    metropole,
                    catalogo,
                    ModoTransferenciaTerritorial::ExpulsaoForcadaMinorias,
                    Some(&metropole.culturas_primarias),
                );
                total_pop += rel.populacao_final;
                total_expulsos += rel.total_migrantes_ou_expulsos;
            }
            estados_libertados.push(estado_id.clone());
        }
    }

    movimento.ativo = false;
    movimento.em_guerra = false;

    RelatorioDescolonizacao {
        via: ViaDescolonizacao::GuerraLibertacaoSecessao,
        pais_emancipado_tag: novo_pais.tag.clone(),
        metropole_tag: metropole.tag.clone(),
        estados_emancipados: estados_libertados,
        populacao_total_emancipada: total_pop,
        refugiados_ou_repatriados: total_expulsos,
        descricao: format!(
            "Vitória revolucionária na Guerra de Libertação: {} conquistou independência contra a metrópole {}.",
            novo_pais.nome, metropole.nome
        ),
    }
}
