//! Estimativa regional de rentabilidade de um novo nível de prédio.

use crate::{CategoriaPredio, CatalogoEconomia, TipoPredio};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default)]
pub struct ContextoLucratividade {
    pub salario_base: f64,
    pub padrao_vida: f64,
    pub desempregados: f64,
    pub mao_de_obra_total: f64,
    pub infraestrutura_uso: f64,
    pub infraestrutura_capacidade: f64,
    pub acesso_mercado: f64,
    pub qualidade_recurso: f64,
    pub potencial_livre: Option<f64>,
    pub potencial_total: Option<f64>,
    pub niveis_iguais: f64,
    /// Bônus de cadeia local (0.0 = nenhum; 0.2 = +20%), vindo de `Estado::bonus_cadeia`.
    pub bonus_cadeia: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EstimativaLucratividade {
    pub receita: f64,
    pub custo_insumos: f64,
    pub custo_folha: f64,
    pub lucro_liquido: f64,
    pub margem: f64,
    pub vagas_totais: f64,
    pub produtividade: f64,
    pub fator_salario: f64,
    pub fator_mao_de_obra: f64,
    pub fator_infraestrutura: f64,
    pub fator_recurso: f64,
    pub fator_acesso: f64,
    pub fator_concorrencia: f64,
    /// 1.0 + bônus de cadeia local.
    pub fator_cadeia_local: f64,
}

/// Insumos, produtos e vagas de um tipo de prédio com seus métodos, pré-agregados uma única vez
/// (a IA e a UI estimam o mesmo tipo em centenas de províncias).
#[derive(Debug, Clone, Default)]
pub struct PerfilPredio {
    /// (índice do bem no catálogo, quantidade por nível)
    pub produtos: Vec<(usize, f64)>,
    pub insumos: Vec<(usize, f64)>,
    /// (vagas, multiplicador salarial do tipo de pop)
    pub vagas: Vec<(f64, f64)>,
}

impl PerfilPredio {
    pub fn novo(cat: &CatalogoEconomia, metodos: &[String]) -> Self {
        let mut produtos: BTreeMap<usize, f64> = BTreeMap::new();
        let mut insumos: BTreeMap<usize, f64> = BTreeMap::new();
        let mut empregos: BTreeMap<&str, i32> = BTreeMap::new();
        let bem_idx = |id: &str| cat.bens.iter().position(|b| b.id == id);
        for mid in metodos {
            if let Some(m) = cat.metodos_producao.iter().find(|m| &m.id == mid) {
                for (id, qtd) in &m.produtos {
                    if let Some(i) = bem_idx(id) {
                        *produtos.entry(i).or_default() += *qtd;
                    }
                }
                for (id, qtd) in &m.insumos {
                    if let Some(i) = bem_idx(id) {
                        *insumos.entry(i).or_default() += *qtd;
                    }
                }
                for (id, qtd) in &m.empregos {
                    *empregos.entry(id.as_str()).or_default() += *qtd;
                }
            }
        }
        let vagas = empregos
            .into_iter()
            .filter(|(_, d)| *d > 0)
            .map(|(id, d)| (d as f64, cat.tipos_pop.iter().find(|tp| tp.id == id).map_or(1.0, |tp| tp.multiplicador_salario)))
            .collect();
        Self { produtos: produtos.into_iter().collect(), insumos: insumos.into_iter().collect(), vagas }
    }
}

/// Preços indexados pelo catálogo de bens (preço de mercado quando existe, senão o preço base).
pub fn precos_por_indice(cat: &CatalogoEconomia, precos: &[(String, f64)]) -> Vec<f64> {
    cat.bens
        .iter()
        .map(|b| precos.iter().find(|(id, _)| id == &b.id).map_or(b.preco_base, |(_, p)| *p))
        .collect()
}

/// Calcula a rentabilidade de um nível usando preços do mercado e condições locais.
pub fn calcular_lucratividade_esperada(
    cat: &CatalogoEconomia,
    tipo: &TipoPredio,
    metodos: &[String],
    precos: &[(String, f64)],
    contexto: ContextoLucratividade,
) -> EstimativaLucratividade {
    let perfil = PerfilPredio::novo(cat, metodos);
    calcular_com_perfil(tipo, &perfil, &precos_por_indice(cat, precos), contexto)
}

/// Versão barata: perfil do tipo e preços (por índice de bem) pré-calculados pelo chamador.
pub fn calcular_com_perfil(
    tipo: &TipoPredio,
    perfil: &PerfilPredio,
    precos: &[f64],
    contexto: ContextoLucratividade,
) -> EstimativaLucratividade {
    let preco_de = |i: usize| precos.get(i).copied().unwrap_or(20.0);

    let acesso = contexto.acesso_mercado.clamp(0.0, 1.0);
    let fator_acesso = (0.65 + 0.35 * acesso).clamp(0.35, 1.0);
    // Recurso local domina a extração: sem depósito (ou sem espaço livre) o rendimento é zero.
    let fator_recurso = match (contexto.potencial_livre, contexto.potencial_total) {
        (Some(livre), Some(total)) if total > 0.0 && livre > 0.0 => {
            (0.35 + 0.65 * (livre / total).clamp(0.0, 1.0)) * contexto.qualidade_recurso.max(0.0)
        }
        (Some(_), Some(_)) => 0.0,
        _ => contexto.qualidade_recurso.max(0.0),
    };
    let peso_infra = match tipo.categoria {
        CategoriaPredio::Industrial | CategoriaPredio::Extrativo => 2.0,
        CategoriaPredio::Infraestrutura => 0.0,
        _ => 1.0,
    };
    let uso_projetado = contexto.infraestrutura_uso + peso_infra;
    let fator_infra = if contexto.infraestrutura_capacidade > 1e-6 {
        (1.0 - ((uso_projetado / contexto.infraestrutura_capacidade) - 1.0).max(0.0) * 0.65).clamp(0.25, 1.0)
    } else if peso_infra > 0.0 {
        0.35
    } else {
        1.0
    };
    let fator_concorrencia = 1.0 / (1.0 + contexto.niveis_iguais.max(0.0) * 0.12);
    let salario_referencia = (4.0 * (contexto.padrao_vida / 10.0).clamp(0.55, 2.5)).max(0.5);
    let salario = contexto.salario_base.max(salario_referencia * 0.75);
    let fator_salario = salario / 4.0;

    let mut receita = perfil.produtos.iter().map(|&(i, qtd)| qtd * preco_de(i)).sum::<f64>();
    let mut custo_insumos = perfil.insumos.iter().map(|&(i, qtd)| qtd * preco_de(i)).sum::<f64>();
    let mut vagas_totais = 0.0;
    let mut custo_folha = 0.0;
    for &(vagas, mult) in &perfil.vagas {
        vagas_totais += vagas;
        custo_folha += vagas * salario * mult;
    }
    let fator_mao_de_obra = if vagas_totais > 0.0 {
        (contexto.desempregados.max(0.0) / vagas_totais.max(1.0)).clamp(0.35, 1.0)
    } else {
        1.0
    };
    let fator_cadeia_local = 1.0 + contexto.bonus_cadeia.max(0.0);
    let fator_producao = fator_acesso * fator_recurso * fator_infra * fator_concorrencia * fator_mao_de_obra * fator_cadeia_local;
    receita *= fator_producao;
    custo_insumos *= fator_producao;
    custo_folha *= fator_salario;
    let lucro_liquido = if tipo.financiado_pelo_governo {
        -(custo_insumos + custo_folha)
    } else {
        receita - custo_insumos - custo_folha
    };
    let margem = if receita > 1e-6 { lucro_liquido / receita * 100.0 } else { 0.0 };
    let produtividade = if vagas_totais > 0.0 { (receita - custo_insumos) / vagas_totais } else { 0.0 };
    EstimativaLucratividade {
        receita,
        custo_insumos,
        custo_folha,
        lucro_liquido,
        margem,
        vagas_totais,
        produtividade,
        fator_salario,
        fator_mao_de_obra,
        fator_infraestrutura: fator_infra,
        fator_recurso,
        fator_acesso,
        fator_concorrencia,
        fator_cadeia_local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Bem, CategoriaBem, CategoriaPredio, MetodoProducao, TipoPop};
    use crate::tipos::{Estrato, RegimePropriedade, SlotMetodo};

    fn catalogo() -> CatalogoEconomia {
        CatalogoEconomia {
            bens: vec![Bem {
                id: "ferro".into(), nome: "Ferro".into(), categoria: CategoriaBem::MateriaPrima,
                comercializavel: true, preco_base: 10.0, substituicoes: Vec::new(),
            }],
            tipos_pop: vec![TipoPop {
                id: "operarios".into(), nome: "Operários".into(), estrato: Estrato::Baixo,
                multiplicador_salario: 1.0, alfabetizacao_minima: 0.0, recebe_dividendos_capitalista: false,
                recebe_dividendos_aristocrata: false, subsistencia: false, fator_consumo_mercado: 1.0,
                forca_politica_base: 1.0,
            }],
            metodos_producao: vec![MetodoProducao {
                id: "mina".into(), nome: "Mina".into(), slot: SlotMetodo::ProducaoPrincipal,
                insumos: BTreeMap::new(), produtos: [("ferro".into(), 10.0)].into_iter().collect(),
                empregos: [("operarios".into(), 10)].into_iter().collect(),
                regime_propriedade: Some(RegimePropriedade::PrivadoCapitalista),
                pontos_construcao_gerados: 0.0, burocracia_gerada: 0.0,
                forca_militar_gerada: 0.0, infraestrutura_gerada: 0.0,
            }],
            tipos_predio: vec![TipoPredio {
                id: "mina_ferro".into(), nome: "Mina".into(), categoria: CategoriaPredio::Extrativo,
                custo_pontos_construcao: 10.0, usa_terra_aravel: false,
                recurso_natural_requerido: Some("ferro".into()), financiado_pelo_governo: false,
                metodos_permitidos: vec!["mina".into()], metodos_padrao: vec!["mina".into()],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn mesma_construcao_varia_por_provincia_e_recurso() {
        let cat = catalogo();
        let tipo = &cat.tipos_predio[0];
        let base = ContextoLucratividade {
            salario_base: 4.0, padrao_vida: 10.0, desempregados: 100.0, mao_de_obra_total: 100.0,
            infraestrutura_uso: 5.0, infraestrutura_capacidade: 20.0, acesso_mercado: 1.0,
            qualidade_recurso: 1.0, potencial_livre: Some(10.0), potencial_total: Some(10.0),
            niveis_iguais: 0.0, bonus_cadeia: 0.0,
        };
        let rica = calcular_lucratividade_esperada(&cat, tipo, &tipo.metodos_padrao, &[("ferro".into(), 10.0)], base);
        let pobre = calcular_lucratividade_esperada(&cat, tipo, &tipo.metodos_padrao, &[("ferro".into(), 10.0)], ContextoLucratividade {
            salario_base: 8.0, padrao_vida: 20.0, desempregados: 5.0, infraestrutura_uso: 19.0,
            acesso_mercado: 0.5, potencial_livre: Some(2.0), ..base
        });
        assert_ne!(rica.lucro_liquido, pobre.lucro_liquido);
        let sem_recurso = calcular_lucratividade_esperada(&cat, tipo, &tipo.metodos_padrao, &[("ferro".into(), 10.0)], ContextoLucratividade {
            potencial_livre: Some(0.0), ..base
        });
        assert!(rica.lucro_liquido > sem_recurso.lucro_liquido);
    }

    #[test]
    fn mina_so_rende_com_deposito_e_cadeia_local_aumenta_o_lucro() {
        let cat = catalogo();
        let tipo = &cat.tipos_predio[0];
        let base = ContextoLucratividade {
            salario_base: 4.0, padrao_vida: 10.0, desempregados: 100.0, mao_de_obra_total: 100.0,
            infraestrutura_uso: 5.0, infraestrutura_capacidade: 20.0, acesso_mercado: 1.0,
            qualidade_recurso: 1.0, potencial_livre: Some(10.0), potencial_total: Some(10.0),
            niveis_iguais: 0.0, bonus_cadeia: 0.0,
        };
        let precos = [("ferro".to_string(), 10.0)];
        let com = calcular_lucratividade_esperada(&cat, tipo, &tipo.metodos_padrao, &precos, base);
        assert!(com.lucro_liquido > 0.0, "mina com depósito deve dar lucro");
        let sem = calcular_lucratividade_esperada(&cat, tipo, &tipo.metodos_padrao, &precos, ContextoLucratividade {
            potencial_livre: Some(0.0), potencial_total: Some(0.0), qualidade_recurso: 0.0, ..base
        });
        assert!(sem.lucro_liquido < 0.0 && sem.fator_recurso == 0.0, "sem depósito não há receita");
        let cadeia = calcular_lucratividade_esperada(&cat, tipo, &tipo.metodos_padrao, &precos, ContextoLucratividade {
            bonus_cadeia: 0.2, ..base
        });
        assert!(cadeia.lucro_liquido > com.lucro_liquido && (cadeia.fator_cadeia_local - 1.2).abs() < 1e-9);
    }
}
