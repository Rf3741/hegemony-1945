//! Mecânicas de transferência territorial, ocupação militar e efeitos demográficos nas pops
//! (recálculo de cidadania, discriminação cultural, migração voluntária e expulsões forçadas).

use crate::catalogo::{CatalogoTerritorio, Cultura};
use crate::entidades::{Estado, PaisTerritorial, PopRegional, Regiao};
use crate::tipos::{ModoTransferenciaTerritorial, StatusCulturalPop};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Relatório com métricas completas do impacto demográfico de uma transferência territorial.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelatorioTransferencia {
    pub estado_id: String,
    pub antigo_dono: String,
    pub novo_dono: String,
    pub antigo_controlador: String,
    pub novo_controlador: String,
    pub modo: ModoTransferenciaTerritorial,
    pub populacao_inicial: u64,
    pub populacao_final: u64,
    pub total_migrantes_ou_expulsos: u64,
    pub destruicao_guerra_adicionada: f64,
    /// Quantidade de indivíduos deslocados por cultura e tag de destino.
    pub fluxos_por_cultura: BTreeMap<String, u64>,
}

/// Executa a transferência de soberania de um estado inteiro e de todas as suas regiões,
/// aplicando os efeitos demográficos e culturais estritos nas populações residentes.
pub fn transferir_estado(
    estado: &mut Estado,
    regioes: &mut [&mut Regiao],
    novo_dono: &PaisTerritorial,
    antigo_dono: &PaisTerritorial,
    catalogo: &CatalogoTerritorio,
    modo: ModoTransferenciaTerritorial,
    culturas_alvo_expulsao: Option<&[String]>,
) -> RelatorioTransferencia {
    let pop_inicial: u64 = regioes.iter().map(|r| r.populacao).sum();
    let antigo_dono_tag = estado.dono_tag.clone();
    let antigo_ctrl_tag = estado.controlador_tag.clone();

    // 1. Atualiza titularidade soberana e controle
    estado.dono_tag = novo_dono.tag.clone();
    estado.controlador_tag = novo_dono.tag.clone();

    // Se o novo dono não tiver núcleo prévio e não for integração pacífica, o estado torna-se não-incorporado
    if !estado.tem_nucleo(&novo_dono.tag)
        && modo != ModoTransferenciaTerritorial::IntegracaoPacifica
    {
        estado.incorporado = false;
    }

    // 2. Coleta definições das culturas primárias do novo dono
    let defs_primarias: Vec<&Cultura> = novo_dono
        .culturas_primarias
        .iter()
        .filter_map(|cp| catalogo.buscar_cultura(cp))
        .collect();

    let mut total_deslocados: u64 = 0;
    let mut fluxos: BTreeMap<String, u64> = BTreeMap::new();
    let mut dano_adicionado = 0.0;

    // 3. Aplica danos e parâmetros de guerra conforme o modo
    match modo {
        ModoTransferenciaTerritorial::IntegracaoPacifica => {
            // Sem danos estruturais extras
        }
        ModoTransferenciaTerritorial::ConquistaMilitar => {
            dano_adicionado = 0.18;
            estado.rede_infraestrutura.aplicar_dano_geral(0.20);
            estado.agitacao_social = (estado.agitacao_social + 25.0).min(100.0);
        }
        ModoTransferenciaTerritorial::ParticaoComTrocaPopulacional => {
            dano_adicionado = 0.05;
            estado.rede_infraestrutura.aplicar_dano_geral(0.08);
            estado.agitacao_social = (estado.agitacao_social + 15.0).min(100.0);
        }
        ModoTransferenciaTerritorial::ExpulsaoForcadaMinorias => {
            dano_adicionado = 0.12;
            estado.rede_infraestrutura.aplicar_dano_geral(0.15);
            estado.agitacao_social = (estado.agitacao_social + 35.0).min(100.0);
        }
    }

    // 4. Processa cada região do estado
    for regiao in regioes.iter_mut() {
        regiao.pais_tag = novo_dono.tag.clone();
        regiao.controlador_tag = novo_dono.tag.clone();
        regiao.destruicao_guerra = (regiao.destruicao_guerra + dano_adicionado).clamp(0.0, 1.0);

        let mut pops_restantes: Vec<PopRegional> = Vec::with_capacity(regiao.pops.len());

        for mut pop in regiao.pops.drain(..) {
            // Determina novo status cultural perante as leis do novo soberano
            if let Some(def_cultura) = catalogo.buscar_cultura(&pop.cultura) {
                pop.status_cultural = novo_dono.determinar_status_cultural(
                    def_cultura,
                    &defs_primarias,
                    estado.incorporado,
                );
            }

            // Calcula proporção de deslocamento conforme o modo de transferência
            let fracao_saida = match modo {
                ModoTransferenciaTerritorial::IntegracaoPacifica => {
                    // Se a cultura era primária no antigo dono e agora foi discriminada, sai fração voluntária
                    if antigo_dono.culturas_primarias.contains(&pop.cultura)
                        && pop.status_cultural == StatusCulturalPop::Discriminada
                    {
                        0.10
                    } else {
                        0.0
                    }
                }
                ModoTransferenciaTerritorial::ConquistaMilitar => {
                    if antigo_dono.culturas_primarias.contains(&pop.cultura) {
                        0.30 // Fuga de elites administrativas e funcionários
                    } else if pop.status_cultural == StatusCulturalPop::Discriminada {
                        0.15
                    } else if pop.status_cultural == StatusCulturalPop::Perseguida {
                        0.50
                    } else {
                        0.02
                    }
                }
                ModoTransferenciaTerritorial::ParticaoComTrocaPopulacional => {
                    // Troca de populações: minorias alinhadas ao antigo soberano migram
                    if antigo_dono.culturas_primarias.contains(&pop.cultura)
                        || pop.status_cultural == StatusCulturalPop::Discriminada
                    {
                        0.65
                    } else {
                        0.0
                    }
                }
                ModoTransferenciaTerritorial::ExpulsaoForcadaMinorias => {
                    let eh_alvo = if let Some(alvos) = culturas_alvo_expulsao {
                        alvos.contains(&pop.cultura)
                    } else {
                        antigo_dono.culturas_primarias.contains(&pop.cultura)
                            || pop.status_cultural == StatusCulturalPop::Perseguida
                    };

                    if eh_alvo {
                        0.85 // Expulsão compulsória da grande maioria da população-alvo
                    } else {
                        0.0
                    }
                }
            };

            let qtd_saida = ((pop.populacao as f64) * fracao_saida).round() as u64;
            if qtd_saida > 0 {
                let real_saida = qtd_saida.min(pop.populacao);
                total_deslocados += real_saida;
                *fluxos.entry(pop.cultura.clone()).or_insert(0) += real_saida;
                pop.populacao -= real_saida;
            }

            // Populações discriminadas ganham agitação
            pop.agitacao = (pop.agitacao * pop.status_cultural.multiplicador_agitacao()).min(10.0);

            if pop.populacao > 0 {
                pops_restantes.push(pop);
            }
        }

        regiao.pops = pops_restantes;
        regiao.recalcular_populacao();
    }

    let pop_final: u64 = regioes.iter().map(|r| r.populacao).sum();

    RelatorioTransferencia {
        estado_id: estado.id.clone(),
        antigo_dono: antigo_dono_tag,
        novo_dono: novo_dono.tag.clone(),
        antigo_controlador: antigo_ctrl_tag,
        novo_controlador: novo_dono.tag.clone(),
        modo,
        populacao_inicial: pop_inicial,
        populacao_final: pop_final,
        total_migrantes_ou_expulsos: total_deslocados,
        destruicao_guerra_adicionada: dano_adicionado,
        fluxos_por_cultura: fluxos,
    }
}

/// Ocupa militarmente um estado (altera apenas o `controlador_tag` de facto, mantendo `dono_tag` de jure).
pub fn ocupar_militarmente_estado(
    estado: &mut Estado,
    regioes: &mut [&mut Regiao],
    ocupante_tag: &str,
) {
    estado.controlador_tag = ocupante_tag.to_string();
    estado.rede_infraestrutura.aplicar_dano_geral(0.10);
    estado.agitacao_social = (estado.agitacao_social + 20.0).min(100.0);

    for r in regioes.iter_mut() {
        r.controlador_tag = ocupante_tag.to_string();
        r.destruicao_guerra = (r.destruicao_guerra + 0.08).clamp(0.0, 1.0);
    }
}

/// Libera a ocupação militar de um estado, restituindo o controle ao proprietário de jure.
pub fn desocupar_estado(estado: &mut Estado, regioes: &mut [&mut Regiao]) {
    let dono = estado.dono_tag.clone();
    estado.controlador_tag = dono.clone();
    for r in regioes.iter_mut() {
        r.controlador_tag = dono.clone();
    }
}

/// Processa a migração contínua espontânea de populações discriminadas para estados com cidadania plena.
pub fn processar_migracao_espontanea(
    estado_origem: &mut Estado,
    regioes_origem: &mut [&mut Regiao],
    regiao_destino: &mut Regiao,
    cultura_id: &str,
    taxa_anual: f64,
) -> u64 {
    let mut total_migrantes: u64 = 0;
    let taxa_diaria = taxa_anual / 365.0;

    for reg in regioes_origem.iter_mut() {
        for pop in reg.pops.iter_mut() {
            if pop.cultura == cultura_id
                && (pop.status_cultural == StatusCulturalPop::Discriminada
                    || pop.status_cultural == StatusCulturalPop::Perseguida)
            {
                let propensao = pop.status_cultural.propensao_emigracao();
                let migrantes = ((pop.populacao as f64) * taxa_diaria * propensao).round() as u64;
                let real = migrantes.min(pop.populacao);
                if real > 0 {
                    pop.populacao -= real;
                    total_migrantes += real;
                }
            }
        }
        reg.recalcular_populacao();
    }

    if total_migrantes > 0 {
        // Encontra ou cria a pop correspondente na região de destino
        if let Some(pop_dest) = regiao_destino
            .pops
            .iter_mut()
            .find(|p| p.cultura == cultura_id)
        {
            pop_dest.populacao += total_migrantes;
        } else {
            regiao_destino.pops.push(PopRegional::nova(
                cultura_id,
                "outros",
                crate::tipos::SetorEmpregoPop::Industrial,
                total_migrantes,
            ));
        }
        regiao_destino.recalcular_populacao();
    }

    let pop_atual_origem: u64 = regioes_origem.iter().map(|r| r.populacao).sum();
    estado_origem.desenvolvimento_urbano.populacao_total = pop_atual_origem;

    total_migrantes
}
