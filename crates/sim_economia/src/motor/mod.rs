//! Motor principal de simulação econômica (`MundoEconomico`), buffers livres de alocação
//! e auditoria de invariantes numéricas.

pub mod consumo;
pub mod demografia;
pub mod distribuicao;
pub mod global;
pub mod governo;
pub mod mercado;
pub mod producao;
pub mod trabalho;

use crate::catalogo::CatalogoEconomia;
use crate::cenario::CenarioEconomia;
use crate::entidades::{
    BlocoComercial, ConfigSimulacao, DetalheRotaComercial, Estado, ItemConstrucao, Mercado,
    OpcaoParceiroComercial, Pais, Provincia, ResumoComercioPais, RotaComercial,
};
use crate::erro::{ErroEconomia, ResultadoEconomia, validar_id_snake_case};
use crate::motor::consumo::CestaNecessidadeCompilada;
use crate::motor::demografia::LoteMigrante;
use crate::motor::governo::{
    atualizar_terra_aravel_ocupada, compilar_cache_predio,
};
use crate::motor::producao::listar_bens_alimento_basico;
use crate::motor::trabalho::{LotePromocao, reconciliar_empregados_provincia};
use crate::tipos::{DataJogo, GrupoLeiEconomica, ModoCadencia, NUM_NECESSIDADES};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Medição detalhada em milissegundos de cada etapa interna de um tick diário.
#[derive(Debug, Clone, Copy, Default)]
pub struct TemposFasesTick {
    pub producao_ms: f64,
    pub distribuicao_ms: f64,
    pub consumo_ms: f64,
    pub mercado_ms: f64,
    pub demografia_ms: f64,
    pub trabalho_ms: f64,
    pub governo_ms: f64,
}

/// Resumo executivo retornado ao final de um tick diário ou período simulado.
#[derive(Debug, Clone, Default)]
pub struct RelatorioTick {
    pub data: DataJogo,
    pub dias_decorridos: u64,
    pub pib_mundial_diario: f64,
    pub populacao_mundial: f64,
    pub obras_concluidas: usize,
}

/// Buffers pré-alocados reutilizados a cada tick para evitar alocações no heap no caminho crítico.
#[derive(Debug, Clone, Default)]
struct BuffersTick {
    proxima_oferta: Vec<f64>,
    proxima_demanda: Vec<f64>,
    importacoes_liquidas: Vec<f64>,
    pontos_construcao_setores_pais: Vec<f64>,
    infra_gerada_estado: Vec<f64>,
    infra_usada_estado: Vec<f64>,
    pop_incorporada_pais: Vec<f64>,
    reservas_predios_pais: Vec<f64>,
    salarios_prov_tipo: Vec<f64>,
    dividendos_cap_prov: Vec<f64>,
    dividendos_ari_prov: Vec<f64>,
    dividendos_cap_pais: Vec<f64>,
    dividendos_ari_pais: Vec<f64>,
    pop_cap_pais: Vec<f64>,
    pop_ari_pais: Vec<f64>,
    cestas_pais: Vec<[CestaNecessidadeCompilada; NUM_NECESSIDADES]>,
    demanda_necessidade_pais: Vec<[f64; NUM_NECESSIDADES]>,
    scratch_tipo_a: Vec<f64>,
    scratch_tipo_b: Vec<f64>,
    scratch_tipo_c: Vec<f64>,
    lotes_migrantes: Vec<LoteMigrante>,
    lotes_promocao: Vec<LotePromocao>,
}

/// Instância indexada e determinística do mundo econômico em execução.
#[derive(Debug, Clone)]
pub struct MundoEconomico {
    data_atual: DataJogo,
    dias_decorridos: u64,
    pub config: ConfigSimulacao,
    catalogo: CatalogoEconomia,
    mercados: Vec<Mercado>,
    paises: Vec<Pais>,
    estados: Vec<Estado>,
    provincias: Vec<Provincia>,
    rotas_comerciais: Vec<RotaComercial>,
    pub blocos_comerciais: Vec<BlocoComercial>,

    indice_mercados: BTreeMap<String, usize>,
    indice_paises: BTreeMap<String, usize>,
    indice_estados: BTreeMap<String, usize>,
    indice_provincias: BTreeMap<String, usize>,
    nomes_culturas: Vec<String>,
    nomes_religioes: Vec<String>,
    bens_alimento_basico: Vec<usize>,
    buffers: BuffersTick,
    /// Fator (população histórica de 1945 / população do cenário) só para exibição, por país.
    fator_pop_exibicao: BTreeMap<String, f64>,
}

impl MundoEconomico {
    /// Constrói, valida todas as referências cruzadas e pré-indexa um `MundoEconomico`
    /// a partir de um `CenarioEconomia`.
    pub fn novo(mut cenario: CenarioEconomia) -> ResultadoEconomia<Self> {
        cenario.catalogo.validar_e_indexar()?;
        let catalogo = cenario.catalogo;
        let num_bens = catalogo.bens.len();
        let num_tipos_pop = catalogo.tipos_pop.len();

        if num_bens == 0 || num_tipos_pop == 0 {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "CatalogoEconomia",
                id: "catalogo".into(),
                detalhe: "catálogo precisa conter ao menos 1 Bem e 1 TipoPop".into(),
            });
        }

        // 1. Indexar mercados (validação de pais_lider ocorre após indexar países)
        let mut indice_mercados = BTreeMap::new();
        for (idx, m) in cenario.mercados.iter_mut().enumerate() {
            validar_id_snake_case("Mercado", &m.id)?;
            if indice_mercados.insert(m.id.clone(), idx).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "Mercado",
                    id: m.id.clone(),
                });
            }
            m.precos_idx = vec![0.0; num_bens];
            m.oferta_idx = vec![0.0; num_bens];
            m.demanda_idx = vec![0.0; num_bens];
            m.fator_suprimento_idx = vec![1.0; num_bens];
            m.importacoes_liquidas_idx = vec![0.0; num_bens];

            for (g_idx, bem) in catalogo.bens.iter().enumerate() {
                let p_ini = m
                    .precos_atuais
                    .get(&bem.id)
                    .copied()
                    .unwrap_or(bem.preco_base);
                if !p_ini.is_finite() || p_ini <= 0.0 {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "Mercado",
                        id: m.id.clone(),
                        detalhe: format!("preço inicial inválido para '{}'", bem.id),
                    });
                }
                m.precos_idx[g_idx] = p_ini.clamp(
                    bem.preco_base * cenario.config.multiplicador_preco_minimo,
                    bem.preco_base * cenario.config.multiplicador_preco_maximo,
                );
                m.oferta_idx[g_idx] = m.ordens_venda.get(&bem.id).copied().unwrap_or(0.0).max(0.0);
                m.demanda_idx[g_idx] = m
                    .ordens_compra
                    .get(&bem.id)
                    .copied()
                    .unwrap_or(0.0)
                    .max(0.0);
            }
        }

        // 2. Indexar países e validar leis e mercados
        let mut indice_paises = BTreeMap::new();
        for (idx, p) in cenario.paises.iter_mut().enumerate() {
            validar_id_snake_case("Pais", &p.id)?;
            if indice_paises.insert(p.id.clone(), idx).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "Pais",
                    id: p.id.clone(),
                });
            }
            let &m_idx = indice_mercados.get(&p.mercado_id).ok_or_else(|| {
                ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "Pais",
                    origem_id: p.id.clone(),
                    alvo_entidade: "Mercado",
                    alvo_id: p.mercado_id.clone(),
                }
            })?;
            p.mercado_idx = m_idx;
            p.provincias_indices.clear();
            p.estados_indices.clear();
            if !p.orcamento.tesouro.is_finite()
                || !p.orcamento.divida.is_finite()
                || p.orcamento.divida < 0.0
            {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "Pais",
                    id: p.id.clone(),
                    detalhe: "tesouro ou dívida inicial inválida".into(),
                });
            }
            if p.orcamento.tesouro < 0.0 {
                p.orcamento.divida += -p.orcamento.tesouro;
                p.orcamento.tesouro = 0.0;
            }
            compilar_cache_fiscal_pais(&catalogo, p)?;
        }

        // Vincular pais_lider_idx dos mercados
        for m in &mut cenario.mercados {
            let &lider_idx = indice_paises.get(&m.pais_lider_id).ok_or_else(|| {
                ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "Mercado",
                    origem_id: m.id.clone(),
                    alvo_entidade: "Pais",
                    alvo_id: m.pais_lider_id.clone(),
                }
            })?;
            m.pais_lider_idx = lider_idx;
        }

        // 3. Indexar estados
        let mut indice_estados = BTreeMap::new();
        for (idx, e) in cenario.estados.iter_mut().enumerate() {
            validar_id_snake_case("Estado", &e.id)?;
            if indice_estados.insert(e.id.clone(), idx).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "Estado",
                    id: e.id.clone(),
                });
            }
            let &c_idx = indice_paises.get(&e.pais_id).ok_or_else(|| {
                ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "Estado",
                    origem_id: e.id.clone(),
                    alvo_entidade: "Pais",
                    alvo_id: e.pais_id.clone(),
                }
            })?;
            e.pais_idx = c_idx;
            e.provincias_indices.clear();
            cenario.paises[c_idx].estados_indices.push(idx);
        }

        // 4. Indexar províncias, pops e prédios
        let mut indice_provincias = BTreeMap::new();
        let mut indice_culturas: BTreeMap<String, u16> = BTreeMap::new();
        let mut indice_religioes: BTreeMap<String, u16> = BTreeMap::new();
        let mut nomes_culturas: Vec<String> = Vec::new();
        let mut nomes_religioes: Vec<String> = Vec::new();
        let mut scratch_emp = vec![0.0; num_tipos_pop];
        let mut scratch_pop = vec![0.0; num_tipos_pop];

        for (p_idx, prov) in cenario.provincias.iter_mut().enumerate() {
            validar_id_snake_case("Provincia", &prov.id)?;
            if indice_provincias.insert(prov.id.clone(), p_idx).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "Provincia",
                    id: prov.id.clone(),
                });
            }
            let &e_idx = indice_estados.get(&prov.estado_id).ok_or_else(|| {
                ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "Provincia",
                    origem_id: prov.id.clone(),
                    alvo_entidade: "Estado",
                    alvo_id: prov.estado_id.clone(),
                }
            })?;
            let &c_idx = indice_paises.get(&prov.pais_id).ok_or_else(|| {
                ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "Provincia",
                    origem_id: prov.id.clone(),
                    alvo_entidade: "Pais",
                    alvo_id: prov.pais_id.clone(),
                }
            })?;
            if cenario.estados[e_idx].pais_idx != c_idx {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "Provincia",
                    id: prov.id.clone(),
                    detalhe: format!(
                        "pais_id '{}' diverge do pais_id do estado '{}'",
                        prov.pais_id, prov.estado_id
                    ),
                });
            }
            prov.estado_idx = e_idx;
            prov.pais_idx = c_idx;
            prov.mercado_idx = cenario.paises[c_idx].mercado_idx;
            cenario.estados[e_idx].provincias_indices.push(p_idx);
            cenario.paises[c_idx].provincias_indices.push(p_idx);

            for rec_id in prov.limites_recursos.keys() {
                validar_id_snake_case("limites_recursos", rec_id)?;
            }

            // Validar Pops
            for pop in &mut prov.pops {
                validar_id_snake_case("cultura", &pop.cultura)?;
                validar_id_snake_case("religiao", &pop.religiao)?;
                let &tp_idx = catalogo
                    .indice_tipos_pop
                    .get(&pop.tipo_pop_id)
                    .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                        origem_entidade: "Pop",
                        origem_id: prov.id.clone(),
                        alvo_entidade: "TipoPop",
                        alvo_id: pop.tipo_pop_id.clone(),
                    })?;
                if !pop.populacao.is_finite() || pop.populacao < 0.0 {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "Pop",
                        id: prov.id.clone(),
                        detalhe: "população inicial não pode ser negativa ou NaN".into(),
                    });
                }
                pop.tipo_idx = tp_idx;
                pop.cultura_idx = if let Some(&idx) = indice_culturas.get(&pop.cultura) {
                    idx
                } else {
                    let idx = nomes_culturas.len() as u16;
                    nomes_culturas.push(pop.cultura.clone());
                    indice_culturas.insert(pop.cultura.clone(), idx);
                    idx
                };
                pop.religiao_idx = if let Some(&idx) = indice_religioes.get(&pop.religiao) {
                    idx
                } else {
                    let idx = nomes_religioes.len() as u16;
                    nomes_religioes.push(pop.religiao.clone());
                    indice_religioes.insert(pop.religiao.clone(), idx);
                    idx
                };
                pop.padrao_de_vida = pop.padrao_de_vida.clamp(1.0, 99.0);
                pop.alfabetizacao = pop.alfabetizacao.clamp(0.0, 1.0);
                pop.riqueza_reservas = pop.riqueza_reservas.max(0.0);
            }

            // Validar e compilar Prédios
            for b in &mut prov.predios {
                let &tp_pred_idx = catalogo
                    .indice_tipos_predio
                    .get(&b.tipo_predio_id)
                    .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                        origem_entidade: "Predio",
                        origem_id: prov.id.clone(),
                        alvo_entidade: "TipoPredio",
                        alvo_id: b.tipo_predio_id.clone(),
                    })?;
                let def = &catalogo.tipos_predio[tp_pred_idx];
                if b.metodos_ativos.is_empty() {
                    b.metodos_ativos = def.metodos_padrao.clone();
                }
                for m_id in &b.metodos_ativos {
                    if !def.metodos_permitidos.contains(m_id) {
                        return Err(ErroEconomia::ValorInvalido {
                            entidade: "Predio",
                            id: b.tipo_predio_id.clone(),
                            detalhe: format!(
                                "método ativo '{m_id}' não é permitido neste tipo de prédio"
                            ),
                        });
                    }
                }
                catalogo.validar_slots_metodos("Predio", &b.tipo_predio_id, &b.metodos_ativos)?;
                if !b.salario_base.is_finite() || b.salario_base <= 0.0 {
                    b.salario_base = 4.0;
                }
                b.reservas_caixa = b.reservas_caixa.max(0.0);
                compilar_cache_predio(&catalogo, b).ok_or_else(|| ErroEconomia::ValorInvalido {
                    entidade: "Predio",
                    id: b.tipo_predio_id.clone(),
                    detalhe: "falha ao compilar cache do prédio".into(),
                })?;
            }

            atualizar_terra_aravel_ocupada(&catalogo, prov);
            reconciliar_empregados_provincia(prov, &mut scratch_emp, &mut scratch_pop);
        }
        governo::atualizar_terra_aravel_estados(&mut cenario.estados, &cenario.provincias);
        producao::atualizar_infraestrutura_estados(&catalogo, &mut cenario.estados, &cenario.provincias);
        producao::calibrar_infraestrutura_inicial(
            &catalogo,
            &mut cenario.estados,
            &mut cenario.provincias,
            &mut scratch_emp,
            &mut scratch_pop,
        );

        // 5. Validar filas de construção iniciais dos países
        for pais in &mut cenario.paises {
            for item in &mut pais.fila_construcao {
                let &p_idx = indice_provincias.get(&item.provincia_id).ok_or_else(|| {
                    ErroEconomia::ReferenciaInexistente {
                        origem_entidade: "ItemConstrucao",
                        origem_id: pais.id.clone(),
                        alvo_entidade: "Provincia",
                        alvo_id: item.provincia_id.clone(),
                    }
                })?;
                let &tp_idx = catalogo
                    .indice_tipos_predio
                    .get(&item.tipo_predio_id)
                    .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                        origem_entidade: "ItemConstrucao",
                        origem_id: pais.id.clone(),
                        alvo_entidade: "TipoPredio",
                        alvo_id: item.tipo_predio_id.clone(),
                    })?;
                item.provincia_idx = p_idx;
                item.tipo_predio_idx = tp_idx;
                if item.custo_total_pontos <= 0.0 {
                    item.custo_total_pontos = catalogo.tipos_predio[tp_idx].custo_pontos_construcao;
                }
                item.progresso_pontos = item.progresso_pontos.clamp(0.0, item.custo_total_pontos);
            }
        }

        // 6. Validar rotas comerciais
        let mut ids_rotas = BTreeMap::new();
        for (r_idx, rota) in cenario.rotas_comerciais.iter_mut().enumerate() {
            validar_id_snake_case("RotaComercial", &rota.id)?;
            if ids_rotas.insert(rota.id.clone(), r_idx).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "RotaComercial",
                    id: rota.id.clone(),
                });
            }
            rota.origem_idx = *indice_mercados
                .get(&rota.mercado_origem_id)
                .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "RotaComercial",
                    origem_id: rota.id.clone(),
                    alvo_entidade: "Mercado",
                    alvo_id: rota.mercado_origem_id.clone(),
                })?;
            rota.destino_idx = *indice_mercados
                .get(&rota.mercado_destino_id)
                .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "RotaComercial",
                    origem_id: rota.id.clone(),
                    alvo_entidade: "Mercado",
                    alvo_id: rota.mercado_destino_id.clone(),
                })?;
            rota.pais_operador_idx =
                *indice_paises.get(&rota.pais_operador_id).ok_or_else(|| {
                    ErroEconomia::ReferenciaInexistente {
                        origem_entidade: "RotaComercial",
                        origem_id: rota.id.clone(),
                        alvo_entidade: "Pais",
                        alvo_id: rota.pais_operador_id.clone(),
                    }
                })?;
            rota.bem_idx = *catalogo.indice_bens.get(&rota.bem_id).ok_or_else(|| {
                ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "RotaComercial",
                    origem_id: rota.id.clone(),
                    alvo_entidade: "Bem",
                    alvo_id: rota.bem_id.clone(),
                }
            })?;
            if !catalogo.bens[rota.bem_idx].comercializavel {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "RotaComercial",
                    id: rota.id.clone(),
                    detalhe: format!(
                        "o bem '{}' é local e não comercializável entre mercados",
                        rota.bem_id
                    ),
                });
            }
            if !rota.volume_maximo.is_finite() || rota.volume_maximo <= 0.0 {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "RotaComercial",
                    id: rota.id.clone(),
                    detalhe: "volume_maximo deve ser > 0.0".into(),
                });
            }
            rota.volume_atual = rota.volume_atual.clamp(0.0, rota.volume_maximo);
        }

        let num_mercados = cenario.mercados.len();
        let num_paises = cenario.paises.len();
        let num_estados = cenario.estados.len();
        let num_provincias = cenario.provincias.len();
        let bens_alimento_basico = listar_bens_alimento_basico(&catalogo);

        let buffers = BuffersTick {
            proxima_oferta: vec![0.0; num_mercados * num_bens],
            proxima_demanda: vec![0.0; num_mercados * num_bens],
            importacoes_liquidas: vec![0.0; num_mercados * num_bens],
            pontos_construcao_setores_pais: vec![0.0; num_paises],
            infra_gerada_estado: vec![0.0; num_estados],
            infra_usada_estado: vec![0.0; num_estados],
            pop_incorporada_pais: vec![0.0; num_paises],
            reservas_predios_pais: vec![0.0; num_paises],
            salarios_prov_tipo: vec![0.0; num_provincias * num_tipos_pop],
            dividendos_cap_prov: vec![0.0; num_provincias],
            dividendos_ari_prov: vec![0.0; num_provincias],
            dividendos_cap_pais: vec![0.0; num_paises],
            dividendos_ari_pais: vec![0.0; num_paises],
            pop_cap_pais: vec![0.0; num_paises],
            pop_ari_pais: vec![0.0; num_paises],
            cestas_pais: vec![Default::default(); num_paises],
            demanda_necessidade_pais: vec![[0.0; NUM_NECESSIDADES]; num_paises],
            scratch_tipo_a: vec![0.0; num_tipos_pop],
            scratch_tipo_b: vec![0.0; num_tipos_pop],
            scratch_tipo_c: vec![0.0; num_tipos_pop],
            lotes_migrantes: Vec::with_capacity(16),
            lotes_promocao: Vec::with_capacity(8),
        };

        let mut blocos_comerciais = cenario.blocos_comerciais;
        if blocos_comerciais.is_empty() {
            // Bloco Soviético: URSS e satélites do Leste Europeu fechados até COMECON
            let sov_candidatos = [
                "sov", "pol", "cze", "hun", "rom", "bul", "alb", "ddr", "uniao_sovietica",
            ];
            let sov_membros: Vec<String> = sov_candidatos
                .iter()
                .filter(|&id| indice_paises.contains_key(*id))
                .map(|&s| s.to_string())
                .collect();
            if !sov_membros.is_empty() {
                let lider = if indice_paises.contains_key("sov") {
                    "sov"
                } else if indice_paises.contains_key("uniao_sovietica") {
                    "uniao_sovietica"
                } else {
                    &sov_membros[0]
                };
                blocos_comerciais.push(BlocoComercial {
                    id: "bloco_sovietico".to_string(),
                    nome: "Bloco Soviético (Leste Europeu)".to_string(),
                    lider_pais_id: lider.to_string(),
                    membros: sov_membros,
                    fechado: true,
                    comecon_ativo: false,
                });
            }

            // Bloco Ocidental: Acordo de Bretton Woods / GATT
            let oci_candidatos = [
                "usa", "gbr", "fra", "can", "bel", "nld", "lux", "ita", "ger", "estados_unidos", "belgica",
            ];
            let oci_membros: Vec<String> = oci_candidatos
                .iter()
                .filter(|&id| indice_paises.contains_key(*id))
                .map(|&s| s.to_string())
                .collect();
            if !oci_membros.is_empty() {
                let lider = if indice_paises.contains_key("usa") {
                    "usa"
                } else if indice_paises.contains_key("estados_unidos") {
                    "estados_unidos"
                } else {
                    &oci_membros[0]
                };
                blocos_comerciais.push(BlocoComercial {
                    id: "bloco_ocidental".to_string(),
                    nome: "Bloco Ocidental (Bretton Woods / GATT)".to_string(),
                    lider_pais_id: lider.to_string(),
                    membros: oci_membros,
                    fechado: false,
                    comecon_ativo: false,
                });
            }
        }

        // População histórica de 1945 (milhões) para exibição; o cenário usa contagens internas maiores.
        const POP_REAL_1945_MI: [(&str, f64); 14] = [
            ("usa", 140.0), ("sov", 170.0), ("chi", 500.0), ("ind", 400.0), ("bra", 46.0),
            ("gbr", 48.0), ("ger", 65.0), ("jap", 72.0), ("fra", 40.0), ("ita", 45.0),
            ("can", 12.0), ("arg", 15.0), ("pol", 24.0), ("tur", 19.0),
        ];
        let mut fator_pop_exibicao = BTreeMap::new();
        for (id, mi) in POP_REAL_1945_MI {
            if let Some(&pi) = indice_paises.get(id) {
                let soma: f64 = cenario.paises[pi]
                    .provincias_indices
                    .iter()
                    .map(|&i| cenario.provincias[i].populacao_total())
                    .sum();
                if soma > 0.0 {
                    fator_pop_exibicao.insert(id.to_string(), mi * 1e6 / soma);
                }
            }
        }

        Ok(Self {
            fator_pop_exibicao,
            data_atual: cenario.data_inicial,
            dias_decorridos: 0,
            config: cenario.config,
            catalogo,
            mercados: cenario.mercados,
            paises: cenario.paises,
            estados: cenario.estados,
            provincias: cenario.provincias,
            rotas_comerciais: cenario.rotas_comerciais,
            blocos_comerciais,
            indice_mercados,
            indice_paises,
            indice_estados,
            indice_provincias,
            nomes_culturas,
            nomes_religioes,
            bens_alimento_basico,
            buffers,
        })
    }

    /// Pedágio diário dos canais (calendário histórico): credita o tesouro do país dono.
    fn cobrar_pedagios_canais(&mut self) {
        for c in crate::canais::canais() {
            if let Some((dono, valor)) = crate::canais::pedagio_diario(&c.id, self.data_atual)
                && let Some(&i) = self.indice_paises.get(dono)
            {
                self.paises[i].orcamento.tesouro += valor;
            }
        }
    }

    /// Executa 1 tick diário completo do motor econômico, avançando o calendário em 1 dia.
    pub fn executar_tick_diario(&mut self) -> RelatorioTick {
        self.buffers.importacoes_liquidas.fill(0.0);

        // (1) Produção dos prédios e subsistência
        producao::executar_producao_diaria(
            &self.catalogo,
            &self.config,
            &self.mercados,
            &mut self.estados,
            &mut self.paises,
            &mut self.provincias,
            &self.bens_alimento_basico,
            &mut self.buffers.proxima_oferta,
            &mut self.buffers.proxima_demanda,
            &mut self.buffers.pontos_construcao_setores_pais,
            &mut self.buffers.infra_gerada_estado,
            &mut self.buffers.infra_usada_estado,
            &mut self.buffers.pop_incorporada_pais,
        );

        // (3) Salários, lucros, caixas e dividendos
        distribuicao::executar_distribuicao_rendas(
            &self.catalogo,
            &self.mercados,
            &mut self.paises,
            &mut self.provincias,
            &mut self.buffers.salarios_prov_tipo,
            &mut self.buffers.dividendos_cap_prov,
            &mut self.buffers.dividendos_ari_prov,
            &mut self.buffers.dividendos_cap_pais,
            &mut self.buffers.dividendos_ari_pais,
            &mut self.buffers.pop_cap_pais,
            &mut self.buffers.pop_ari_pais,
            &mut self.buffers.reservas_predios_pais,
        );

        // (4) Consumo por estrato com substituição de bens, (5) Padrão de vida / alfabetização
        // e impostos sobre renda, consumo e terra
        consumo::executar_consumo_tributos_e_bem_estar(
            &self.catalogo,
            &self.config,
            &self.mercados,
            &self.estados,
            &mut self.paises,
            &mut self.provincias,
            &self.buffers.salarios_prov_tipo,
            &self.buffers.dividendos_cap_prov,
            &self.buffers.dividendos_ari_prov,
            &self.buffers.dividendos_cap_pais,
            &self.buffers.dividendos_ari_pais,
            &self.buffers.pop_cap_pais,
            &self.buffers.pop_ari_pais,
            &mut self.buffers.cestas_pais,
            &mut self.buffers.demanda_necessidade_pais,
            &mut self.buffers.scratch_tipo_a,
            &mut self.buffers.scratch_tipo_b,
            &mut self.buffers.proxima_demanda,
        );

        // (2) Comércio exterior entre mercados e fechamento de preços por oferta e demanda
        mercado::processar_rotas_comerciais(
            &self.catalogo,
            &self.mercados,
            &mut self.paises,
            &mut self.rotas_comerciais,
            &self.blocos_comerciais,
            &mut self.buffers.proxima_oferta,
            &mut self.buffers.proxima_demanda,
            &mut self.buffers.importacoes_liquidas,
        );
        if self.config.comercio_mundial_automatico {
            let abertura: Vec<f64> =
                (0..self.mercados.len()).map(|m| self.abertura_comercial_mercado(m).max(0.8)).collect();
            global::processar_comercio_mundial(
                &self.catalogo,
                &self.config,
                &self.mercados,
                &mut self.paises,
                &abertura,
                &mut self.buffers.proxima_oferta,
                &mut self.buffers.proxima_demanda,
                &mut self.buffers.importacoes_liquidas,
            );
            global::processar_comercio_intra_bloco_comecon(
                &self.catalogo,
                &self.mercados,
                &self.paises,
                &self.blocos_comerciais,
                &mut self.buffers.proxima_oferta,
                &mut self.buffers.proxima_demanda,
                &mut self.buffers.importacoes_liquidas,
            );
        }
        mercado::fechar_mercados_e_atualizar_precos(
            &self.catalogo,
            &self.config,
            &mut self.mercados,
            &self.buffers.proxima_oferta,
            &self.buffers.proxima_demanda,
            &self.buffers.importacoes_liquidas,
        );

        let proximo_dia_num = self.dias_decorridos + 1;
        let rodar_estrutural = match self.config.modo_cadencia {
            ModoCadencia::DiarioCompleto => Some(1u32),
            ModoCadencia::DiarioComAjusteSemanal => {
                if proximo_dia_num.is_multiple_of(7) {
                    Some(7u32)
                } else {
                    None
                }
            }
        };

        if let Some(dias_passo) = rodar_estrutural {
            // (6) Crescimento vegetativo e migração interna
            demografia::executar_crescimento_e_migracao(
                &self.catalogo,
                &self.nomes_culturas,
                &self.nomes_religioes,
                &self.paises,
                &mut self.provincias,
                dias_passo,
                &mut self.buffers.scratch_tipo_a,
                &mut self.buffers.lotes_migrantes,
            );

            // (7) Contratação, promoção/qualificação, demissão e ajuste salarial
            trabalho::executar_contratacao_e_demissao(
                &self.catalogo,
                &self.nomes_culturas,
                &self.nomes_religioes,
                &self.config,
                &self.mercados,
                &self.estados,
                &self.paises,
                &mut self.provincias,
                dias_passo,
                &mut self.buffers.scratch_tipo_a,
                &mut self.buffers.scratch_tipo_b,
                &mut self.buffers.scratch_tipo_c,
                &mut self.buffers.lotes_promocao,
            );
        }

        // (8) Fechamento do orçamento, dívida e juros + (9) Fila de construção civil
        let obras_concluidas = governo::executar_fechamento_fiscal_e_construcao(
            &self.catalogo,
            &self.config,
            &self.mercados,
            &self.estados,
            &mut self.paises,
            &mut self.provincias,
            self.dias_decorridos,
            &self.buffers.pontos_construcao_setores_pais,
            &self.buffers.reservas_predios_pais,
        );

        self.dias_decorridos = proximo_dia_num;
        self.cobrar_pedagios_canais();
        self.data_atual.avancar_dia();

        let pib_mundial_diario = self.paises.iter().map(|p| p.pib_diario).sum();
        let populacao_mundial = self.provincias.iter().map(|p| p.populacao_total()).sum();

        RelatorioTick {
            data: self.data_atual,
            dias_decorridos: self.dias_decorridos,
            pib_mundial_diario,
            populacao_mundial,
            obras_concluidas,
        }
    }

    /// Executa 1 tick diário medindo o tempo individual (em ms) de cada subsistema.
    pub fn executar_tick_diario_perfilado(&mut self) -> (RelatorioTick, TemposFasesTick) {
        use std::time::Instant;
        let mut t = TemposFasesTick::default();
        self.buffers.importacoes_liquidas.fill(0.0);

        let i0 = Instant::now();
        producao::executar_producao_diaria(
            &self.catalogo,
            &self.config,
            &self.mercados,
            &mut self.estados,
            &mut self.paises,
            &mut self.provincias,
            &self.bens_alimento_basico,
            &mut self.buffers.proxima_oferta,
            &mut self.buffers.proxima_demanda,
            &mut self.buffers.pontos_construcao_setores_pais,
            &mut self.buffers.infra_gerada_estado,
            &mut self.buffers.infra_usada_estado,
            &mut self.buffers.pop_incorporada_pais,
        );
        t.producao_ms = i0.elapsed().as_secs_f64() * 1_000.0;

        let i1 = Instant::now();
        distribuicao::executar_distribuicao_rendas(
            &self.catalogo,
            &self.mercados,
            &mut self.paises,
            &mut self.provincias,
            &mut self.buffers.salarios_prov_tipo,
            &mut self.buffers.dividendos_cap_prov,
            &mut self.buffers.dividendos_ari_prov,
            &mut self.buffers.dividendos_cap_pais,
            &mut self.buffers.dividendos_ari_pais,
            &mut self.buffers.pop_cap_pais,
            &mut self.buffers.pop_ari_pais,
            &mut self.buffers.reservas_predios_pais,
        );
        t.distribuicao_ms = i1.elapsed().as_secs_f64() * 1_000.0;

        let i2 = Instant::now();
        consumo::executar_consumo_tributos_e_bem_estar(
            &self.catalogo,
            &self.config,
            &self.mercados,
            &self.estados,
            &mut self.paises,
            &mut self.provincias,
            &self.buffers.salarios_prov_tipo,
            &self.buffers.dividendos_cap_prov,
            &self.buffers.dividendos_ari_prov,
            &self.buffers.dividendos_cap_pais,
            &self.buffers.dividendos_ari_pais,
            &self.buffers.pop_cap_pais,
            &self.buffers.pop_ari_pais,
            &mut self.buffers.cestas_pais,
            &mut self.buffers.demanda_necessidade_pais,
            &mut self.buffers.scratch_tipo_a,
            &mut self.buffers.scratch_tipo_b,
            &mut self.buffers.proxima_demanda,
        );
        t.consumo_ms = i2.elapsed().as_secs_f64() * 1_000.0;

        let i3 = Instant::now();
        mercado::processar_rotas_comerciais(
            &self.catalogo,
            &self.mercados,
            &mut self.paises,
            &mut self.rotas_comerciais,
            &self.blocos_comerciais,
            &mut self.buffers.proxima_oferta,
            &mut self.buffers.proxima_demanda,
            &mut self.buffers.importacoes_liquidas,
        );
        if self.config.comercio_mundial_automatico {
            let abertura: Vec<f64> =
                (0..self.mercados.len()).map(|m| self.abertura_comercial_mercado(m).max(0.8)).collect();
            global::processar_comercio_mundial(
                &self.catalogo,
                &self.config,
                &self.mercados,
                &mut self.paises,
                &abertura,
                &mut self.buffers.proxima_oferta,
                &mut self.buffers.proxima_demanda,
                &mut self.buffers.importacoes_liquidas,
            );
            global::processar_comercio_intra_bloco_comecon(
                &self.catalogo,
                &self.mercados,
                &self.paises,
                &self.blocos_comerciais,
                &mut self.buffers.proxima_oferta,
                &mut self.buffers.proxima_demanda,
                &mut self.buffers.importacoes_liquidas,
            );
        }
        mercado::fechar_mercados_e_atualizar_precos(
            &self.catalogo,
            &self.config,
            &mut self.mercados,
            &self.buffers.proxima_oferta,
            &self.buffers.proxima_demanda,
            &self.buffers.importacoes_liquidas,
        );
        t.mercado_ms = i3.elapsed().as_secs_f64() * 1_000.0;

        let proximo_dia_num = self.dias_decorridos + 1;
        let rodar_estrutural = match self.config.modo_cadencia {
            ModoCadencia::DiarioCompleto => Some(1u32),
            ModoCadencia::DiarioComAjusteSemanal => {
                if proximo_dia_num.is_multiple_of(7) {
                    Some(7u32)
                } else {
                    None
                }
            }
        };

        if let Some(dias_passo) = rodar_estrutural {
            let i4 = Instant::now();
            demografia::executar_crescimento_e_migracao(
                &self.catalogo,
                &self.nomes_culturas,
                &self.nomes_religioes,
                &self.paises,
                &mut self.provincias,
                dias_passo,
                &mut self.buffers.scratch_tipo_a,
                &mut self.buffers.lotes_migrantes,
            );
            t.demografia_ms = i4.elapsed().as_secs_f64() * 1_000.0;

            let i5 = Instant::now();
            trabalho::executar_contratacao_e_demissao(
                &self.catalogo,
                &self.nomes_culturas,
                &self.nomes_religioes,
                &self.config,
                &self.mercados,
                &self.estados,
                &self.paises,
                &mut self.provincias,
                dias_passo,
                &mut self.buffers.scratch_tipo_a,
                &mut self.buffers.scratch_tipo_b,
                &mut self.buffers.scratch_tipo_c,
                &mut self.buffers.lotes_promocao,
            );
            t.trabalho_ms = i5.elapsed().as_secs_f64() * 1_000.0;
        }

        let i6 = Instant::now();
        let obras_concluidas = governo::executar_fechamento_fiscal_e_construcao(
            &self.catalogo,
            &self.config,
            &self.mercados,
            &self.estados,
            &mut self.paises,
            &mut self.provincias,
            self.dias_decorridos,
            &self.buffers.pontos_construcao_setores_pais,
            &self.buffers.reservas_predios_pais,
        );
        t.governo_ms = i6.elapsed().as_secs_f64() * 1_000.0;

        self.dias_decorridos = proximo_dia_num;
        self.cobrar_pedagios_canais();
        self.data_atual.avancar_dia();

        let pib_mundial_diario = self.paises.iter().map(|p| p.pib_diario).sum();
        let populacao_mundial = self.provincias.iter().map(|p| p.populacao_total()).sum();

        (
            RelatorioTick {
                data: self.data_atual,
                dias_decorridos: self.dias_decorridos,
                pib_mundial_diario,
                populacao_mundial,
                obras_concluidas,
            },
            t,
        )
    }

    /// Executa 1 semana completa (7 ticks diários consecutivos) e devolve o relatório consolidado.
    pub fn executar_tick_semanal(&mut self) -> RelatorioTick {
        self.avancar_dias(7)
    }

    /// Avança `dias` ticks diários e retorna o relatório consolidado ao término do período.
    pub fn avancar_dias(&mut self, dias: u32) -> RelatorioTick {
        let mut ultimo = RelatorioTick {
            data: self.data_atual,
            dias_decorridos: self.dias_decorridos,
            pib_mundial_diario: 0.0,
            populacao_mundial: 0.0,
            obras_concluidas: 0,
        };
        let mut total_obras = 0usize;
        for _ in 0..dias {
            ultimo = self.executar_tick_diario();
            total_obras += ultimo.obras_concluidas;
        }
        ultimo.obras_concluidas = total_obras;
        ultimo
    }

    /// Enfileira uma nova obra na fila de construção de um país, validando capacidade física da província.
    pub fn enfileirar_construcao(
        &mut self,
        pais_id: &str,
        provincia_id: &str,
        tipo_predio_id: &str,
        privado: bool,
    ) -> ResultadoEconomia<()> {
        let &c_idx =
            self.indice_paises
                .get(pais_id)
                .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "enfileirar_construcao",
                    origem_id: pais_id.to_string(),
                    alvo_entidade: "Pais",
                    alvo_id: pais_id.to_string(),
                })?;
        let &p_idx = self.indice_provincias.get(provincia_id).ok_or_else(|| {
            ErroEconomia::ReferenciaInexistente {
                origem_entidade: "enfileirar_construcao",
                origem_id: pais_id.to_string(),
                alvo_entidade: "Provincia",
                alvo_id: provincia_id.to_string(),
            }
        })?;
        let &tp_idx = self
            .catalogo
            .indice_tipos_predio
            .get(tipo_predio_id)
            .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                origem_entidade: "enfileirar_construcao",
                origem_id: pais_id.to_string(),
                alvo_entidade: "TipoPredio",
                alvo_id: tipo_predio_id.to_string(),
            })?;

        if self.provincias[p_idx].pais_idx != c_idx {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "ItemConstrucao",
                id: provincia_id.to_string(),
                detalhe: format!("a província não pertence ao país '{pais_id}'"),
            });
        }

        let def = &self.catalogo.tipos_predio[tp_idx];
        let prov = &self.provincias[p_idx];
        let estado = &self.estados[prov.estado_idx];
        if !governo::estado_comporta_novo_nivel(&self.catalogo, estado, prov, tp_idx) {
            return Err(ErroEconomia::CapacidadeExcedida {
                provincia_id: provincia_id.to_string(),
                recurso: def
                    .recurso_natural_requerido
                    .clone()
                    .unwrap_or_else(|| "terra_aravel".into()),
                disponivel: 0,
                necessario: 1,
            });
        }

        self.paises[c_idx].fila_construcao.push(ItemConstrucao {
            provincia_id: provincia_id.to_string(),
            tipo_predio_id: tipo_predio_id.to_string(),
            progresso_pontos: 0.0,
            custo_total_pontos: def.custo_pontos_construcao,
            privado,
            retorno_esperado: 0.0,
            justificativa: String::new(),
            provincia_idx: p_idx,
            tipo_predio_idx: tp_idx,
        });
        Ok(())
    }

    /// Altera os métodos de produção ativos de um prédio existente em uma província e recompila seu cache.
    pub fn alterar_metodos_predio(
        &mut self,
        provincia_id: &str,
        tipo_predio_id: &str,
        novos_metodos: Vec<String>,
    ) -> ResultadoEconomia<()> {
        let &p_idx = self.indice_provincias.get(provincia_id).ok_or_else(|| {
            ErroEconomia::ReferenciaInexistente {
                origem_entidade: "alterar_metodos_predio",
                origem_id: provincia_id.to_string(),
                alvo_entidade: "Provincia",
                alvo_id: provincia_id.to_string(),
            }
        })?;
        let &tp_idx = self
            .catalogo
            .indice_tipos_predio
            .get(tipo_predio_id)
            .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                origem_entidade: "alterar_metodos_predio",
                origem_id: provincia_id.to_string(),
                alvo_entidade: "TipoPredio",
                alvo_id: tipo_predio_id.to_string(),
            })?;

        let def = &self.catalogo.tipos_predio[tp_idx];
        if novos_metodos.is_empty() {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "Predio",
                id: tipo_predio_id.to_string(),
                detalhe: "lista de novos_metodos não pode ser vazia".into(),
            });
        }
        for m_id in &novos_metodos {
            if !def.metodos_permitidos.contains(m_id) {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "Predio",
                    id: tipo_predio_id.to_string(),
                    detalhe: format!("método '{m_id}' não permitido para '{tipo_predio_id}'"),
                });
            }
        }

        self.catalogo
            .validar_slots_metodos("Predio", tipo_predio_id, &novos_metodos)?;

        let prov = &mut self.provincias[p_idx];
        let predio = prov
            .predios
            .iter_mut()
            .find(|b| b.cache.tipo_idx == tp_idx)
            .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                origem_entidade: "Provincia",
                origem_id: provincia_id.to_string(),
                alvo_entidade: "Predio",
                alvo_id: tipo_predio_id.to_string(),
            })?;

        predio.metodos_ativos = novos_metodos;
        compilar_cache_predio(&self.catalogo, predio).ok_or_else(|| {
            ErroEconomia::ValorInvalido {
                entidade: "Predio",
                id: tipo_predio_id.to_string(),
                detalhe: "falha ao compilar cache dos novos métodos".into(),
            }
        })?;
        reconciliar_empregados_provincia(
            prov,
            &mut self.buffers.scratch_tipo_a,
            &mut self.buffers.scratch_tipo_b,
        );
        Ok(())
    }

    /// Promulga uma `LeiEconomica` em um `Pais`, aplicando suas alíquotas sugeridas e modificadores.
    pub fn promulgar_lei(&mut self, pais_id: &str, lei_id: &str) -> ResultadoEconomia<()> {
        let &c_idx =
            self.indice_paises
                .get(pais_id)
                .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "promulgar_lei",
                    origem_id: pais_id.to_string(),
                    alvo_entidade: "Pais",
                    alvo_id: pais_id.to_string(),
                })?;
        let &l_idx = self.catalogo.indice_leis.get(lei_id).ok_or_else(|| {
            ErroEconomia::ReferenciaInexistente {
                origem_entidade: "promulgar_lei",
                origem_id: pais_id.to_string(),
                alvo_entidade: "LeiEconomica",
                alvo_id: lei_id.to_string(),
            }
        })?;

        let lei = &self.catalogo.leis[l_idx];
        let pais = &mut self.paises[c_idx];
        match lei.grupo {
            GrupoLeiEconomica::SistemaEconomico => {
                pais.politica_fiscal.lei_sistema_economico_id = lei.id.clone();
            }
            GrupoLeiEconomica::Tributacao => {
                pais.politica_fiscal.lei_tributaria_id = lei.id.clone();
            }
            GrupoLeiEconomica::Comercio => {
                pais.politica_fiscal.lei_comercial_id = lei.id.clone();
            }
        }
        if let Some(v) = lei.aliquota_renda_baixo_sugerida {
            pais.politica_fiscal.aliquota_imposto_renda_baixo = v;
        }
        if let Some(v) = lei.aliquota_renda_medio_sugerida {
            pais.politica_fiscal.aliquota_imposto_renda_medio = v;
        }
        if let Some(v) = lei.aliquota_renda_alto_sugerida {
            pais.politica_fiscal.aliquota_imposto_renda_alto = v;
        }
        if let Some(v) = lei.aliquota_dividendos_sugerida {
            pais.politica_fiscal.aliquota_imposto_dividendos = v;
        }
        if let Some(v) = lei.imposto_terra_por_campones_sugerido {
            pais.politica_fiscal.imposto_sobre_terra_por_campones = v;
        }
        if let Some(v) = lei.tarifa_importacao_sugerida {
            pais.politica_fiscal.tarifa_importacao = v;
        }
        if let Some(v) = lei.tarifa_exportacao_sugerida {
            pais.politica_fiscal.tarifa_exportacao = v;
        }

        compilar_cache_fiscal_pais(&self.catalogo, pais)
    }

    /// Recompila o vetor rápido de impostos de consumo de um país após alteração direta em `politica_fiscal`.
    pub fn atualizar_cache_fiscal_pais(&mut self, pais_id: &str) -> ResultadoEconomia<()> {
        let &c_idx =
            self.indice_paises
                .get(pais_id)
                .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "atualizar_cache_fiscal_pais",
                    origem_id: pais_id.to_string(),
                    alvo_entidade: "Pais",
                    alvo_id: pais_id.to_string(),
                })?;
        compilar_cache_fiscal_pais(&self.catalogo, &mut self.paises[c_idx])
    }

    /// Data corrente do calendário da simulação.
    #[inline]
    pub fn data_atual(&self) -> DataJogo {
        self.data_atual
    }

    /// Total de dias simulados desde a inicialização do mundo.
    #[inline]
    pub fn dias_decorridos(&self) -> u64 {
        self.dias_decorridos
    }

    /// Referência ao catálogo estático de bens, profissões, leis, métodos e tipos de prédio.
    #[inline]
    pub fn catalogo(&self) -> &CatalogoEconomia {
        &self.catalogo
    }

    #[inline]
    pub fn mercados(&self) -> &[Mercado] {
        &self.mercados
    }

    #[inline]
    pub fn obter_mercado(&self, id: &str) -> Option<&Mercado> {
        self.indice_mercados.get(id).map(|&i| &self.mercados[i])
    }

    /// Consulta O(1) do preço atual de um bem em um mercado.
    pub fn preco_atual(&self, mercado_id: &str, bem_id: &str) -> Option<f64> {
        let &m_idx = self.indice_mercados.get(mercado_id)?;
        let &g_idx = self.catalogo.indice_bens.get(bem_id)?;
        Some(self.mercados[m_idx].precos_idx[g_idx])
    }

    /// Consulta O(1) das ordens de venda (oferta) de um bem em um mercado no último tick.
    pub fn oferta_atual(&self, mercado_id: &str, bem_id: &str) -> Option<f64> {
        let &m_idx = self.indice_mercados.get(mercado_id)?;
        let &g_idx = self.catalogo.indice_bens.get(bem_id)?;
        Some(self.mercados[m_idx].oferta_idx[g_idx])
    }

    /// Consulta O(1) das ordens de compra (demanda) de um bem em um mercado no último tick.
    pub fn demanda_atual(&self, mercado_id: &str, bem_id: &str) -> Option<f64> {
        let &m_idx = self.indice_mercados.get(mercado_id)?;
        let &g_idx = self.catalogo.indice_bens.get(bem_id)?;
        Some(self.mercados[m_idx].demanda_idx[g_idx])
    }

    #[inline]
    pub fn paises(&self) -> &[Pais] {
        &self.paises
    }

    #[inline]
    pub fn obter_pais(&self, id: &str) -> Option<&Pais> {
        self.indice_paises.get(id).map(|&i| &self.paises[i])
    }

    /// Localiza o bloco comercial ao qual um determinado país pertence, se houver.
    #[inline]
    pub fn bloco_do_pais(&self, pais_id: &str) -> Option<&BlocoComercial> {
        self.blocos_comerciais.iter().find(|b| b.membros.iter().any(|m| m == pais_id))
    }

    #[inline]
    pub fn obter_pais_mut(&mut self, id: &str) -> Option<&mut Pais> {
        self.indice_paises.get(id).map(|&i| &mut self.paises[i])
    }

    #[inline]
    pub fn estados(&self) -> &[Estado] {
        &self.estados
    }

    #[inline]
    pub fn obter_estado(&self, id: &str) -> Option<&Estado> {
        self.indice_estados.get(id).map(|&i| &self.estados[i])
    }

    /// Níveis ocupados de um determinado recurso natural/mineral em todas as províncias do estado.
    pub fn estado_recursos_ocupados(&self, estado_id: &str, recurso: &str) -> u32 {
        if let Some(estado) = self.obter_estado(estado_id) {
            governo::estado_niveis_recurso(&self.catalogo, estado, &self.provincias, recurso)
        } else {
            0
        }
    }

    #[inline]
    pub fn provincias(&self) -> &[Provincia] {
        &self.provincias
    }

    #[inline]
    pub fn obter_provincia(&self, id: &str) -> Option<&Provincia> {
        self.indice_provincias.get(id).map(|&i| &self.provincias[i])
    }

    #[inline]
    pub fn rotas_comerciais(&self) -> &[RotaComercial] {
        &self.rotas_comerciais
    }

    /// População total somada de todas as províncias de um país.
    pub fn populacao_total_pais(&self, pais_id: &str) -> Option<f64> {
        let pais = self.obter_pais(pais_id)?;
        let soma = pais
            .provincias_indices
            .iter()
            .map(|&i| self.provincias[i].populacao_total())
            .sum();
        Some(soma)
    }

    /// População para exibição na interface: ajustada à escala histórica de 1945 nos países
    /// grandes (o cenário usa contagens internas maiores, p.ex. EUA 684 mi contra ~140 mi reais).
    pub fn populacao_exibida_pais(&self, pais_id: &str) -> Option<f64> {
        let bruta = self.populacao_total_pais(pais_id)?;
        Some(bruta * self.fator_pop_exibicao.get(pais_id).copied().unwrap_or(1.0))
    }

    /// Padrão de vida médio ponderado pela população nacional.
    pub fn padrao_de_vida_medio_pais(&self, pais_id: &str) -> Option<f64> {
        let pais = self.obter_pais(pais_id)?;
        let mut pop_total = 0.0;
        let mut soma_sol = 0.0;
        for &p_idx in &pais.provincias_indices {
            for pop in &self.provincias[p_idx].pops {
                pop_total += pop.populacao;
                soma_sol += pop.populacao * pop.padrao_de_vida;
            }
        }
        Some(if pop_total > 0.0 {
            soma_sol / pop_total
        } else {
            10.0
        })
    }

    /// Taxa média nacional de desemprego (0.0..=1.0).
    pub fn taxa_desemprego_media_pais(&self, pais_id: &str) -> Option<f64> {
        let pais = self.obter_pais(pais_id)?;
        let mut pop_total = 0.0;
        let mut desemp_total = 0.0;
        for &p_idx in &pais.provincias_indices {
            for pop in &self.provincias[p_idx].pops {
                // Camponeses de subsistência são autônomos (produzem para si): não entram na força
                // de trabalho assalariada, como no Victoria 3.
                if self.catalogo.tipos_pop[pop.tipo_idx].subsistencia {
                    continue;
                }
                pop_total += pop.populacao;
                desemp_total += pop.desempregados();
            }
        }
        Some(if pop_total > 0.0 {
            (desemp_total / pop_total).clamp(0.0, 1.0)
        } else {
            0.0
        })
    }

    /// Taxa média nacional de alfabetização (0.0..=1.0).
    pub fn alfabetizacao_media_pais(&self, pais_id: &str) -> Option<f64> {
        let pais = self.obter_pais(pais_id)?;
        let mut pop_total = 0.0;
        let mut soma_alfa = 0.0;
        for &p_idx in &pais.provincias_indices {
            for pop in &self.provincias[p_idx].pops {
                pop_total += pop.populacao;
                soma_alfa += pop.populacao * pop.alfabetizacao;
            }
        }
        Some(if pop_total > 0.0 {
            soma_alfa / pop_total
        } else {
            0.0
        })
    }

    // -----------------------------------------------------------------------------------------
    // COMÉRCIO INTERNACIONAL, TARIFAS, ACORDOS E BLOCOS ECONÔMICOS
    // -----------------------------------------------------------------------------------------

    #[inline]
    pub fn blocos_comerciais(&self) -> &[BlocoComercial] {
        &self.blocos_comerciais
    }

    #[inline]
    pub fn blocos_comerciais_mut(&mut self) -> &mut [BlocoComercial] {
        &mut self.blocos_comerciais
    }

    /// Localiza mutavelmente o bloco comercial ao qual um determinado país pertence.
    #[inline]
    pub fn bloco_do_pais_mut(&mut self, pais_id: &str) -> Option<&mut BlocoComercial> {
        self.blocos_comerciais.iter_mut().find(|b| b.membros.iter().any(|m| m == pais_id))
    }

    /// Verifica se dois países possuem acordo comercial bilateral ativo.
    pub fn tem_acordo_comercial(&self, pais_a: &str, pais_b: &str) -> bool {
        let a = self.obter_pais(pais_a);
        let b = self.obter_pais(pais_b);
        match (a, b) {
            (Some(pa), Some(pb)) => {
                pa.acordos_comerciais.iter().any(|id| id == pais_b)
                    || pb.acordos_comerciais.iter().any(|id| id == pais_a)
            }
            _ => false,
        }
    }

    /// Firma um acordo bilateral de livre comércio / redução tarifária mútua entre dois países.
    pub fn assinar_acordo_comercial(&mut self, pais_a: &str, pais_b: &str) -> ResultadoEconomia<()> {
        if pais_a == pais_b {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "AcordoComercial",
                id: pais_a.into(),
                detalhe: "um país não pode firmar acordo comercial consigo mesmo".into(),
            });
        }
        let idx_a = *self.indice_paises.get(pais_a).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "AcordoComercial",
            origem_id: pais_a.into(),
            alvo_entidade: "Pais",
            alvo_id: pais_a.into(),
        })?;
        let idx_b = *self.indice_paises.get(pais_b).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "AcordoComercial",
            origem_id: pais_b.into(),
            alvo_entidade: "Pais",
            alvo_id: pais_b.into(),
        })?;

        if !self.paises[idx_a].acordos_comerciais.contains(&pais_b.to_string()) {
            self.paises[idx_a].acordos_comerciais.push(pais_b.to_string());
        }
        if !self.paises[idx_b].acordos_comerciais.contains(&pais_a.to_string()) {
            self.paises[idx_b].acordos_comerciais.push(pais_a.to_string());
        }
        Ok(())
    }

    /// Revoga um acordo comercial bilateral entre dois países.
    pub fn cancelar_acordo_comercial(&mut self, pais_a: &str, pais_b: &str) -> ResultadoEconomia<()> {
        if let Some(&idx_a) = self.indice_paises.get(pais_a) {
            self.paises[idx_a].acordos_comerciais.retain(|id| id != pais_b);
        }
        if let Some(&idx_b) = self.indice_paises.get(pais_b) {
            self.paises[idx_b].acordos_comerciais.retain(|id| id != pais_a);
        }
        Ok(())
    }

    /// Alterna a ativação do COMECON no Bloco Soviético, integrando os mercados socialistas com tarifas zeradas.
    pub fn alternar_comecon(&mut self) -> ResultadoEconomia<bool> {
        let bloco = self.blocos_comerciais
            .iter_mut()
            .find(|b| b.id == "bloco_sovietico")
            .ok_or_else(|| ErroEconomia::ReferenciaInexistente {
                origem_entidade: "Comando",
                origem_id: "alternar_comecon".into(),
                alvo_entidade: "BlocoComercial",
                alvo_id: "bloco_sovietico".into(),
            })?;
        bloco.comecon_ativo = !bloco.comecon_ativo;
        Ok(bloco.comecon_ativo)
    }

    /// Verifica se um país possui ao menos um porto comercial ativo (nível > 0).
    /// Caso o catálogo não defina o tipo de prédio "porto_comercial" (ex.: fixtures de teste mínimas),
    /// retorna `true` para manter compatibilidade.
    pub fn pais_tem_porto_ativo(&self, pais_id: &str) -> bool {
        let Some(&c_idx) = self.indice_paises.get(pais_id) else { return false };
        let Some(&porto_idx) = self.catalogo.indice_tipos_predio.get("porto_comercial") else {
            return true;
        };
        for &p_idx in &self.paises[c_idx].provincias_indices {
            if let Some(prov) = self.provincias.get(p_idx) {
                for b in &prov.predios {
                    if b.cache.tipo_idx == porto_idx && b.nivel > 0 {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Cria uma nova rota de importação ou exportação para um determinado bem e parceiro comercial.
    pub fn criar_rota_comercial(
        &mut self,
        pais_id: &str,
        parceiro_pais_id: &str,
        bem_id: &str,
        exportacao: bool,
        volume_maximo: f64,
    ) -> ResultadoEconomia<String> {
        if pais_id == parceiro_pais_id {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "RotaComercial",
                id: pais_id.into(),
                detalhe: "não é possível criar rota comercial com o próprio país".into(),
            });
        }
        if !self.pais_tem_porto_ativo(pais_id) {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "RotaComercial",
                id: format!("{pais_id}_{parceiro_pais_id}"),
                detalhe: "Comércio marítimo requer ao menos um porto comercial ativo no país operador.".into(),
            });
        }
        let idx_p = *self.indice_paises.get(pais_id).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "RotaComercial",
            origem_id: pais_id.into(),
            alvo_entidade: "Pais",
            alvo_id: pais_id.into(),
        })?;
        let idx_parc = *self.indice_paises.get(parceiro_pais_id).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "RotaComercial",
            origem_id: parceiro_pais_id.into(),
            alvo_entidade: "Pais",
            alvo_id: parceiro_pais_id.into(),
        })?;
        let bem_idx = *self.catalogo.indice_bens.get(bem_id).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "RotaComercial",
            origem_id: bem_id.into(),
            alvo_entidade: "Bem",
            alvo_id: bem_id.into(),
        })?;
        if !self.catalogo.bens[bem_idx].comercializavel {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "RotaComercial",
                id: bem_id.into(),
                detalhe: "o bem especificado não é comercializável no comércio exterior".into(),
            });
        }

        // Restrição de bloco fechado (URSS e satélites até COMECON)
        let bloco_p = self.bloco_do_pais(pais_id);
        let bloco_parc = self.bloco_do_pais(parceiro_pais_id);
        let p_fechado = bloco_p.map_or(false, |b| b.fechado && bloco_parc.map_or(true, |bp| bp.id != b.id));
        let parc_fechado = bloco_parc.map_or(false, |b| b.fechado && bloco_p.map_or(true, |bp| bp.id != b.id));
        if (p_fechado || parc_fechado) && !self.tem_acordo_comercial(pais_id, parceiro_pais_id) {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "RotaComercial",
                id: format!("{pais_id}_{parceiro_pais_id}"),
                detalhe: "País em bloco econômico fechado requer acordo comercial prévio para negociar com o exterior.".into(),
            });
        }

        let m_orig_id = if exportacao {
            self.paises[idx_p].mercado_id.clone()
        } else {
            self.paises[idx_parc].mercado_id.clone()
        };
        let m_dest_id = if exportacao {
            self.paises[idx_parc].mercado_id.clone()
        } else {
            self.paises[idx_p].mercado_id.clone()
        };

        if m_orig_id == m_dest_id {
            return Err(ErroEconomia::ValorInvalido {
                entidade: "RotaComercial",
                id: m_orig_id,
                detalhe: "ambos os países já pertencem ao mesmo mercado comum".into(),
            });
        }

        let m_orig_idx = *self.indice_mercados.get(&m_orig_id).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "RotaComercial",
            origem_id: m_orig_id.clone(),
            alvo_entidade: "Mercado",
            alvo_id: m_orig_id.clone(),
        })?;
        let m_dest_idx = *self.indice_mercados.get(&m_dest_id).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "RotaComercial",
            origem_id: m_dest_id.clone(),
            alvo_entidade: "Mercado",
            alvo_id: m_dest_id.clone(),
        })?;

        let id_rota = format!(
            "rota_{}_{}_{}_{}",
            pais_id,
            parceiro_pais_id,
            bem_id,
            if exportacao { "exp" } else { "imp" }
        );

        if let Some(r) = self.rotas_comerciais.iter_mut().find(|r| r.id == id_rota) {
            r.volume_maximo = volume_maximo.max(1.0);
            return Ok(id_rota);
        }

        validar_id_snake_case("RotaComercial", &id_rota)?;

        self.rotas_comerciais.push(RotaComercial {
            id: id_rota.clone(),
            mercado_origem_id: m_orig_id,
            mercado_destino_id: m_dest_id,
            pais_operador_id: pais_id.to_string(),
            bem_id: bem_id.to_string(),
            volume_atual: 0.0,
            volume_maximo: volume_maximo.max(1.0),
            automatica: true,
            lucro_ultimo_tick: 0.0,
            origem_idx: m_orig_idx,
            destino_idx: m_dest_idx,
            pais_operador_idx: idx_p,
            bem_idx,
        });

        Ok(id_rota)
    }

    /// Remove uma rota comercial pelo identificador.
    pub fn remover_rota_comercial(&mut self, rota_id: &str) -> ResultadoEconomia<()> {
        let pos = self.rotas_comerciais.iter().position(|r| r.id == rota_id).ok_or_else(|| {
            ErroEconomia::ReferenciaInexistente {
                origem_entidade: "MundoEconomico",
                origem_id: rota_id.into(),
                alvo_entidade: "RotaComercial",
                alvo_id: rota_id.into(),
            }
        })?;
        self.rotas_comerciais.remove(pos);
        Ok(())
    }

    /// Alterna se a rota ajusta seu volume automaticamente conforme margem de lucro.
    pub fn alternar_rota_automatica(&mut self, rota_id: &str) -> ResultadoEconomia<bool> {
        let rota = self.rotas_comerciais.iter_mut().find(|r| r.id == rota_id).ok_or_else(|| {
            ErroEconomia::ReferenciaInexistente {
                origem_entidade: "MundoEconomico",
                origem_id: rota_id.into(),
                alvo_entidade: "RotaComercial",
                alvo_id: rota_id.into(),
            }
        })?;
        rota.automatica = !rota.automatica;
        Ok(rota.automatica)
    }

    /// Ajusta o teto de volume operacional máximo de uma rota comercial.
    pub fn ajustar_volume_rota(&mut self, rota_id: &str, novo_volume_maximo: f64) -> ResultadoEconomia<()> {
        let rota = self.rotas_comerciais.iter_mut().find(|r| r.id == rota_id).ok_or_else(|| {
            ErroEconomia::ReferenciaInexistente {
                origem_entidade: "MundoEconomico",
                origem_id: rota_id.into(),
                alvo_entidade: "RotaComercial",
                alvo_id: rota_id.into(),
            }
        })?;
        rota.volume_maximo = novo_volume_maximo.max(1.0);
        Ok(())
    }

    /// Define tarifas gerais de importação e exportação de um país.
    pub fn definir_tarifa_geral(&mut self, pais_id: &str, importacao: f64, exportacao: f64) -> ResultadoEconomia<()> {
        let p = self.obter_pais_mut(pais_id).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "MundoEconomico",
            origem_id: pais_id.into(),
            alvo_entidade: "Pais",
            alvo_id: pais_id.into(),
        })?;
        p.politica_fiscal.tarifa_importacao = importacao.clamp(0.0, 0.80);
        p.politica_fiscal.tarifa_exportacao = exportacao.clamp(0.0, 0.80);
        p.politica_fiscal.nivel_abertura_mercado = Some(p.politica_fiscal.nivel_abertura());
        Ok(())
    }

    /// Define a tarifa específica de um bem (substituindo a tarifa geral para esse item).
    pub fn definir_tarifa_bem(
        &mut self,
        pais_id: &str,
        bem_id: &str,
        tarifa_importacao: f64,
        tarifa_exportacao: f64,
    ) -> ResultadoEconomia<()> {
        let p = self.obter_pais_mut(pais_id).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "MundoEconomico",
            origem_id: pais_id.into(),
            alvo_entidade: "Pais",
            alvo_id: pais_id.into(),
        })?;
        p.politica_fiscal.tarifas_importacao_bens.insert(bem_id.to_string(), tarifa_importacao.clamp(0.0, 0.80));
        p.politica_fiscal.tarifas_exportacao_bens.insert(bem_id.to_string(), tarifa_exportacao.clamp(0.0, 0.80));
        Ok(())
    }

    /// Ajusta o nível de abertura de mercado do país (0.0 a 1.0), recalculando tarifas gerais.
    pub fn definir_nivel_abertura(&mut self, pais_id: &str, nivel: f64) -> ResultadoEconomia<()> {
        let p = self.obter_pais_mut(pais_id).ok_or_else(|| ErroEconomia::ReferenciaInexistente {
            origem_entidade: "MundoEconomico",
            origem_id: pais_id.into(),
            alvo_entidade: "Pais",
            alvo_id: pais_id.into(),
        })?;
        p.politica_fiscal.definir_nivel_abertura(nivel);
        Ok(())
    }

    /// Constrói um resumo comercial completo do país para a interface (F3).
    pub fn resumo_comercio_pais(&self, pais_id: &str) -> ResumoComercioPais {
        let Some(p) = self.obter_pais(pais_id) else {
            return ResumoComercioPais::default();
        };
        let m_idx = self.paises.iter().find(|c| c.id == pais_id).map(|c| c.mercado_idx).unwrap_or(0);
        let m = &self.mercados[m_idx];

        let mut exp_total = 0.0;
        let mut imp_total = 0.0;
        for (g_idx, _bem) in self.catalogo.bens.iter().enumerate() {
            let saldo = m.importacoes_liquidas_idx[g_idx];
            let preco = m.precos_idx[g_idx];
            if saldo > 0.0 {
                imp_total += saldo * preco;
            } else if saldo < 0.0 {
                exp_total += (-saldo) * preco;
            }
        }

        let bloco = self.bloco_do_pais(pais_id);
        ResumoComercioPais {
            total_exportacoes_valor: exp_total,
            total_importacoes_valor: imp_total,
            saldo_comercial: exp_total - imp_total,
            receita_tarifas_total: p.orcamento.receita_tarifas,
            nivel_abertura: p.politica_fiscal.nivel_abertura(),
            tarifa_geral_importacao: p.politica_fiscal.tarifa_importacao,
            tarifa_geral_exportacao: p.politica_fiscal.tarifa_exportacao,
            acordos_comerciais: p.acordos_comerciais.clone(),
            bloco_id: bloco.map(|b| b.id.clone()),
            bloco_nome: bloco.map(|b| b.nome.clone()),
            bloco_fechado: bloco.map_or(false, |b| b.fechado),
            comecon_ativo: bloco.map_or(false, |b| b.comecon_ativo),
            eh_lider_bloco: bloco.map_or(false, |b| b.lider_pais_id == pais_id),
            membros_bloco: bloco.map(|b| b.membros.clone()).unwrap_or_default(),
        }
    }

    /// Retorna detalhes de todas as rotas comerciais conectadas ao país do jogador.
    pub fn detalhes_rotas_pais(&self, pais_id: &str) -> Vec<DetalheRotaComercial> {
        let Some(pais) = self.obter_pais(pais_id) else {
            return Vec::new();
        };
        let mercado_id = &pais.mercado_id;
        let mut lista = Vec::new();

        for rota in &self.rotas_comerciais {
            let eh_origem = &rota.mercado_origem_id == mercado_id;
            let eh_destino = &rota.mercado_destino_id == mercado_id;
            let eh_operador = &rota.pais_operador_id == pais_id;

            if !eh_origem && !eh_destino && !eh_operador {
                continue;
            }

            let eh_exportacao = eh_origem;
            let parceiro_mercado_id = if eh_exportacao {
                &rota.mercado_destino_id
            } else {
                &rota.mercado_origem_id
            };
            let parceiro = self.mercados.iter().find(|m| &m.id == parceiro_mercado_id).map(|m| {
                let p = &self.paises[m.pais_lider_idx];
                (p.id.clone(), p.nome.clone())
            }).unwrap_or_else(|| ("desconhecido".into(), "Parceiro Comercial".into()));

            let g = rota.bem_idx;
            let bem = &self.catalogo.bens[g];
            let p_orig = self.mercados[rota.origem_idx].precos_idx[g];
            let p_dest = self.mercados[rota.destino_idx].precos_idx[g];

            let l_orig = self.mercados[rota.origem_idx].pais_lider_idx;
            let l_dest = self.mercados[rota.destino_idx].pais_lider_idx;
            let tarifa_orig = self.paises[l_orig].politica_fiscal.tarifa_exportacao_bem(&bem.id);
            let tarifa_dest = self.paises[l_dest].politica_fiscal.tarifa_importacao_bem(&bem.id);
            let tem_acordo = self.tem_acordo_comercial(pais_id, &parceiro.0);

            lista.push(DetalheRotaComercial {
                id: rota.id.clone(),
                bem_id: bem.id.clone(),
                bem_nome: bem.nome.clone(),
                eh_exportacao,
                parceiro_pais_id: parceiro.0,
                parceiro_pais_nome: parceiro.1,
                volume_atual: rota.volume_atual,
                volume_maximo: rota.volume_maximo,
                automatica: rota.automatica,
                preco_origem: p_orig,
                preco_destino: p_dest,
                lucro_ultimo_tick: rota.lucro_ultimo_tick,
                tarifa_origem: tarifa_orig,
                tarifa_destino: tarifa_dest,
                tem_acordo,
            });
        }

        lista.sort_by(|a, b| b.lucro_ultimo_tick.total_cmp(&a.lucro_ultimo_tick));
        lista
    }

    /// Lista todos os outros países disponíveis como parceiros comerciais com suas permissões.
    pub fn opcoes_parceiros_pais(&self, pais_id: &str) -> Vec<OpcaoParceiroComercial> {
        let bloco_p = self.bloco_do_pais(pais_id);
        let tem_porto_operador = self.pais_tem_porto_ativo(pais_id);
        let mut opcoes = Vec::new();

        for p in &self.paises {
            if p.id == pais_id {
                continue;
            }
            let tem_acordo = self.tem_acordo_comercial(pais_id, &p.id);
            let bloco_outro = self.bloco_do_pais(&p.id);
            let mesmo_bloco = match (bloco_p, bloco_outro) {
                (Some(bp), Some(bo)) => bp.id == bo.id,
                _ => false,
            };

            let p_fechado = bloco_p.map_or(false, |b| b.fechado && !mesmo_bloco);
            let outro_fechado = bloco_outro.map_or(false, |b| b.fechado && !mesmo_bloco);

            let (permitido, razao) = if !tem_porto_operador {
                (false, Some("Comércio marítimo bloqueado: país requer ao menos um porto comercial ativo".into()))
            } else if (p_fechado || outro_fechado) && !tem_acordo {
                (false, Some("País em bloco fechado: requer acordo comercial prévio".into()))
            } else {
                (true, None)
            };

            opcoes.push(OpcaoParceiroComercial {
                pais_id: p.id.clone(),
                nome: p.nome.clone(),
                mercado_id: p.mercado_id.clone(),
                tem_acordo,
                mesmo_bloco,
                permitido_comercio: permitido,
                razao_bloqueio: razao,
            });
        }

        opcoes.sort_by(|a, b| a.nome.cmp(&b.nome));
        opcoes
    }

    /// Sincroniza os vetores numéricos rápidos de volta para os `BTreeMap<String, f64>` legíveis em RON.
    pub fn sincronizar_mapas_ron(&mut self) {
        for m in &mut self.mercados {
            m.precos_atuais.clear();
            m.ordens_venda.clear();
            m.ordens_compra.clear();
            m.importacoes_liquidas.clear();
            for (g_idx, bem) in self.catalogo.bens.iter().enumerate() {
                m.precos_atuais.insert(bem.id.clone(), m.precos_idx[g_idx]);
                m.ordens_venda.insert(bem.id.clone(), m.oferta_idx[g_idx]);
                m.ordens_compra.insert(bem.id.clone(), m.demanda_idx[g_idx]);
                let imp = m.importacoes_liquidas_idx[g_idx];
                if imp.abs() > 1e-6 {
                    m.importacoes_liquidas.insert(bem.id.clone(), imp);
                }
            }
        }

        for prov in &mut self.provincias {
            for b in &mut prov.predios {
                b.empregados_por_tipo.clear();
                for slot in b.cache.vagas() {
                    let tp_id = &self.catalogo.tipos_pop[slot.tipo_pop_idx as usize].id;
                    b.empregados_por_tipo.insert(tp_id.clone(), slot.empregados);
                }
            }
        }
    }

    /// Exporta o estado corrente completo como um `CenarioEconomia` pronto para gravação RON.
    pub fn exportar_cenario(&mut self) -> CenarioEconomia {
        self.sincronizar_mapas_ron();
        CenarioEconomia {
            data_inicial: self.data_atual,
            config: self.config.clone(),
            catalogo: self.catalogo.clone(),
            mercados: self.mercados.clone(),
            paises: self.paises.clone(),
            estados: self.estados.clone(),
            provincias: self.provincias.clone(),
            rotas_comerciais: self.rotas_comerciais.clone(),
            blocos_comerciais: self.blocos_comerciais.clone(),
        }
    }

    /// Verifica rigorosamente todas as invariantes numéricas e econômicas do mundo:
    /// ausência total de `NaN`/`Infinity`, preços dentro dos limites `[min, max]`,
    /// populações `>= 0`, `empregados <= populacao` e orçamentos finitos.
    pub fn auditar_estabilidade(&self) -> ResultadoEconomia<()> {
        for m in &self.mercados {
            for (g, bem) in self.catalogo.bens.iter().enumerate() {
                let p = m.precos_idx[g];
                let o = m.oferta_idx[g];
                let d = m.demanda_idx[g];
                if !p.is_finite() || !o.is_finite() || !d.is_finite() {
                    return Err(ErroEconomia::InvarianteViolada {
                        contexto: format!("Mercado '{}' / Bem '{}'", m.id, bem.id),
                        detalhe: "preço, oferta ou demanda não finito (NaN/Inf)".into(),
                    });
                }
                let p_min = bem.preco_base * self.config.multiplicador_preco_minimo - 1e-6;
                let p_max = bem.preco_base * self.config.multiplicador_preco_maximo + 1e-6;
                if p < p_min || p > p_max {
                    return Err(ErroEconomia::InvarianteViolada {
                        contexto: format!("Mercado '{}' / Bem '{}'", m.id, bem.id),
                        detalhe: format!(
                            "preço {p:.4} fora da faixa permitida [{p_min:.4}, {p_max:.4}]"
                        ),
                    });
                }
                if o < 0.0 || d < 0.0 {
                    return Err(ErroEconomia::InvarianteViolada {
                        contexto: format!("Mercado '{}' / Bem '{}'", m.id, bem.id),
                        detalhe: "oferta ou demanda negativa".into(),
                    });
                }
            }
        }

        for pais in &self.paises {
            let orc = &pais.orcamento;
            let valores = [
                orc.tesouro,
                orc.divida,
                orc.limite_credito,
                orc.saldo_diario,
                orc.receita_total(),
                orc.despesa_total(),
                pais.pib_diario,
                pais.fundo_investimento_privado,
            ];
            if valores.iter().any(|v| !v.is_finite()) {
                return Err(ErroEconomia::InvarianteViolada {
                    contexto: format!("Pais '{}'", pais.id),
                    detalhe: "valor fiscal/orçamentário NaN ou infinito".into(),
                });
            }
            if orc.tesouro < -1e-6 || orc.divida < -1e-6 || pais.fundo_investimento_privado < -1e-6
            {
                return Err(ErroEconomia::InvarianteViolada {
                    contexto: format!("Pais '{}'", pais.id),
                    detalhe: "tesouro, dívida ou fundo privado negativo".into(),
                });
            }
        }

        for prov in &self.provincias {
            for pop in &prov.pops {
                let vals = [
                    pop.populacao,
                    pop.empregados,
                    pop.riqueza_reservas,
                    pop.padrao_de_vida,
                    pop.alfabetizacao,
                ];
                if vals.iter().any(|v| !v.is_finite()) {
                    return Err(ErroEconomia::InvarianteViolada {
                        contexto: format!("Provincia '{}' / Pop '{}'", prov.id, pop.tipo_pop_id),
                        detalhe: "atributo de pop NaN ou infinito".into(),
                    });
                }
                if pop.populacao < 0.0
                    || pop.empregados < 0.0
                    || pop.empregados > pop.populacao + 1e-4
                    || pop.riqueza_reservas < 0.0
                {
                    return Err(ErroEconomia::InvarianteViolada {
                        contexto: format!("Provincia '{}' / Pop '{}'", prov.id, pop.tipo_pop_id),
                        detalhe: format!(
                            "população ({}) ou emprego ({}) ou riqueza ({}) inválida",
                            pop.populacao, pop.empregados, pop.riqueza_reservas
                        ),
                    });
                }
            }

            for b in &prov.predios {
                let vals = [
                    b.salario_base,
                    b.reservas_caixa,
                    b.receita_ultimo_tick,
                    b.custo_insumos_ultimo_tick,
                    b.folha_salarial_ultimo_tick,
                    b.lucro_ultimo_tick,
                ];
                if vals.iter().any(|v| !v.is_finite()) || b.reservas_caixa < -1e-6 {
                    return Err(ErroEconomia::InvarianteViolada {
                        contexto: format!(
                            "Provincia '{}' / Predio '{}'",
                            prov.id, b.tipo_predio_id
                        ),
                        detalhe: "contabilidade de prédio NaN/Inf ou reserva negativa".into(),
                    });
                }
            }
        }

        Ok(())
    }
}

fn compilar_cache_fiscal_pais(
    catalogo: &CatalogoEconomia,
    pais: &mut Pais,
) -> ResultadoEconomia<()> {
    let mut mult_const = 1.0;
    let mut mult_com = 1.0;
    let mut exige_estatal = false;

    for (campo, lei_id) in [
        (
            "lei_sistema_economico_id",
            &pais.politica_fiscal.lei_sistema_economico_id,
        ),
        ("lei_tributaria_id", &pais.politica_fiscal.lei_tributaria_id),
        ("lei_comercial_id", &pais.politica_fiscal.lei_comercial_id),
    ] {
        if !lei_id.is_empty() {
            let &l_idx = catalogo.indice_leis.get(lei_id).ok_or_else(|| {
                ErroEconomia::ReferenciaInexistente {
                    origem_entidade: campo,
                    origem_id: pais.id.clone(),
                    alvo_entidade: "LeiEconomica",
                    alvo_id: lei_id.clone(),
                }
            })?;
            let lei = &catalogo.leis[l_idx];
            mult_const *= lei.multiplicador_eficiencia_construcao;
            mult_com *= lei.multiplicador_volume_comercio;
            if lei.exige_propriedade_estatal {
                exige_estatal = true;
            }
        }
    }

    pais.mult_eficiencia_construcao_lei = mult_const;
    pais.mult_volume_comercio_lei = mult_com;
    pais.exige_propriedade_estatal_lei = exige_estatal;

    let geral = pais.politica_fiscal.aliquota_consumo_geral.clamp(0.0, 1.0);
    pais.aliquota_consumo_por_bem = vec![geral; catalogo.bens.len()];
    for (bem_id, &aliq) in &pais.politica_fiscal.bens_tributados_consumo {
        let &g_idx = catalogo.indice_bens.get(bem_id).ok_or_else(|| {
            ErroEconomia::ReferenciaInexistente {
                origem_entidade: "bens_tributados_consumo",
                origem_id: pais.id.clone(),
                alvo_entidade: "Bem",
                alvo_id: bem_id.clone(),
            }
        })?;
        pais.aliquota_consumo_por_bem[g_idx] = (geral + aliq.max(0.0)).clamp(0.0, 1.5);
    }

    Ok(())
}

/// Estado dinâmico completo da economia para salvar/carregar (o catálogo vem dos arquivos do jogo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoEconomiaSalvo {
    pub data: DataJogo,
    pub dias_decorridos: u64,
    pub config: ConfigSimulacao,
    pub mercados: Vec<Mercado>,
    pub paises: Vec<Pais>,
    pub estados: Vec<Estado>,
    pub provincias: Vec<Provincia>,
    pub rotas_comerciais: Vec<RotaComercial>,
    pub blocos_comerciais: Vec<BlocoComercial>,
    pub fator_pop_exibicao: BTreeMap<String, f64>,
}

impl MundoEconomico {
    /// Captura o estado dinâmico (sem o catálogo).
    pub fn salvar_estado(&mut self) -> EstadoEconomiaSalvo {
        let c = self.exportar_cenario();
        EstadoEconomiaSalvo {
            data: c.data_inicial,
            dias_decorridos: self.dias_decorridos,
            config: c.config,
            mercados: c.mercados,
            paises: c.paises,
            estados: c.estados,
            provincias: c.provincias,
            rotas_comerciais: c.rotas_comerciais,
            blocos_comerciais: c.blocos_comerciais,
            fator_pop_exibicao: self.fator_pop_exibicao.clone(),
        }
    }

    /// Reconstrói o mundo (reindexando tudo) a partir do catálogo e do estado salvo.
    pub fn restaurar_estado(catalogo: CatalogoEconomia, e: EstadoEconomiaSalvo) -> ResultadoEconomia<Self> {
        let mut m = Self::novo(CenarioEconomia {
            data_inicial: e.data,
            config: e.config,
            catalogo,
            mercados: e.mercados,
            paises: e.paises,
            estados: e.estados,
            provincias: e.provincias,
            rotas_comerciais: e.rotas_comerciais,
            blocos_comerciais: e.blocos_comerciais,
        })?;
        m.dias_decorridos = e.dias_decorridos;
        m.fator_pop_exibicao = e.fator_pop_exibicao;
        Ok(m)
    }
}
