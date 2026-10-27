//! Dinâmica de radicalização e lealdade dos Pops.
//!
//! Cada Pop possui frações-alvo de radicais e de leais que dependem do déficit de padrão de vida,
//! do desemprego, da discriminação legal (cultura fora das `culturas_primarias` sob leis de
//! segregação) e da legitimidade do governo. As frações reais convergem diariamente para o alvo,
//! de modo que o radicalismo sobe e desce (não é um acumulador sem retorno).

use crate::entidades::pais::PaisPolitica;

/// Velocidade diária de convergência da fração de radicais/leais ao alvo.
const TAXA_CONVERGENCIA: f64 = 0.010;

/// Calcula a fração-alvo de radicais de um Pop (0.0..=0.90).
pub fn fracao_alvo_radicais(
    deficit_padrao_vida: f64,
    taxa_desemprego: f64,
    discriminacao: f64,
    legitimidade: f64,
) -> f64 {
    let mut alvo = 0.0;
    alvo += 0.06 * deficit_padrao_vida.clamp(0.0, 10.0);
    alvo += 0.60 * (taxa_desemprego - 0.08).max(0.0);
    alvo += discriminacao;
    if legitimidade < 25.0 {
        alvo += 0.15 + 0.004 * (25.0 - legitimidade);
    }
    alvo.clamp(0.0, 0.90)
}

/// Calcula a fração-alvo de leais de um Pop (0.0..=0.70).
pub fn fracao_alvo_leais(excedente_padrao_vida: f64, legitimidade: f64) -> f64 {
    let mut alvo = 0.0;
    if excedente_padrao_vida > 2.0 {
        alvo += 0.04 * (excedente_padrao_vida - 2.0).min(10.0);
    }
    if legitimidade >= 75.0 {
        alvo += 0.20;
    } else if legitimidade >= 50.0 {
        alvo += 0.05;
    }
    alvo.clamp(0.0, 0.70)
}

/// Atualiza o contingente de radicais e leais de cada Pop em um tick diário.
pub fn atualizar_radicalismo_e_lealdade_pops(pais: &mut PaisPolitica) {
    let lei_cidadania = pais
        .leis_ativas
        .get("cidadania")
        .cloned()
        .unwrap_or_default();
    let legitimidade = pais.legitimidade;
    let culturas = pais.culturas_primarias.clone();

    for pop in &mut pais.pops {
        if pop.populacao <= 0.0 {
            continue;
        }

        let deficit = pop.padrao_de_vida_esperado - pop.padrao_de_vida;
        let excedente = -deficit;

        // Discriminação: apenas culturas fora do grupo nacional titular são afetadas.
        let minoria = !culturas.is_empty() && !culturas.contains(&pop.cultura);
        let discriminacao = match lei_cidadania.as_str() {
            "segregacao_apartheid" if minoria => 0.35,
            "segregacao_apartheid" => 0.02,
            "cidadania_etnica_nacional" if minoria => 0.15,
            _ => 0.0,
        };

        let alvo_rad =
            fracao_alvo_radicais(deficit, pop.taxa_desemprego, discriminacao, legitimidade);
        let alvo_lea = fracao_alvo_leais(excedente, legitimidade).min(1.0 - alvo_rad);

        let fr_rad = pop.fracao_radicais();
        let fr_lea = pop.fracao_leais();

        let nova_rad = (fr_rad + (alvo_rad - fr_rad) * TAXA_CONVERGENCIA).clamp(0.0, 1.0);
        let mut nova_lea = (fr_lea + (alvo_lea - fr_lea) * TAXA_CONVERGENCIA).clamp(0.0, 1.0);
        if nova_rad + nova_lea > 1.0 {
            nova_lea = (1.0 - nova_rad).max(0.0);
        }

        pop.radicais = pop.populacao * nova_rad;
        pop.leais = pop.populacao * nova_lea;
    }
}
