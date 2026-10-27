//! Instância dinâmica de um Grupo de Interesse em um país.

use crate::catalogo::CatalogoPolitica;
use crate::entidades::lider::LiderPolitico;
use crate::tipos::{HumorGrupoInteresse, PapelGrupoInteresse, PosturaLei, StatusClout};
use serde::{Deserialize, Serialize};

/// Modificador temporário de aprovação concedido por eventos ou decisões de curto prazo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModificadorTemporario {
    pub id: String,
    pub valor: f64,
    pub dias_restantes: u32,
}

/// Estado dinâmico de um Grupo de Interesse (GI) dentro de uma nação soberana.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanciaGrupoInteresse {
    pub id: String,
    pub nome: String,
    /// Líder atual do grupo.
    pub lider: Option<LiderPolitico>,
    /// Poder político relativo (`0.0..=1.0`, soma de todos os IGs do país = 1.0).
    pub clout: f64,
    /// Força política agregada bruta antes da normalização percentual.
    #[serde(default)]
    pub forca_politica_acumulada: f64,
    /// Total de indivíduos adultos de Pops que apoiam este grupo.
    #[serde(default)]
    pub populacao_apoiadora: f64,
    /// Pontuação contínua de aprovação (-20.0 a +20.0).
    #[serde(default)]
    pub aprovacao: f64,
    /// Valor de equilíbrio para o qual a aprovação converge no tick diário.
    #[serde(default)]
    pub aprovacao_alvo: f64,
    /// Papel institucional atual (no governo, na oposição ou marginalizado).
    pub papel: PapelGrupoInteresse,
    /// Status derivado da faixa de clout (Poderoso, Influente ou Marginalizado).
    pub status: StatusClout,
    /// Humor derivado da pontuação de aprovação (Feliz, Irritado, Radical, Neutro, Leal).
    pub humor: HumorGrupoInteresse,
    /// Bônus financeiro ou de inteligência concedido por superpotência estrangeira (CIA / KGB).
    #[serde(default)]
    pub bonus_financiamento_externo: f64,
    /// Modificadores temporários com prazo de expiração.
    #[serde(default)]
    pub modificadores_temporarios: Vec<ModificadorTemporario>,
}

impl InstanciaGrupoInteresse {
    /// Cria uma nova instância de Grupo de Interesse.
    pub fn novo(
        id: impl Into<String>,
        nome: impl Into<String>,
        papel: PapelGrupoInteresse,
    ) -> Self {
        let id_str = id.into();
        Self {
            id: id_str,
            nome: nome.into(),
            lider: None,
            clout: 0.10,
            forca_politica_acumulada: 0.0,
            populacao_apoiadora: 0.0,
            aprovacao: 0.0,
            aprovacao_alvo: 0.0,
            papel,
            status: StatusClout::Influente,
            humor: HumorGrupoInteresse::Neutro,
            bonus_financiamento_externo: 0.0,
            modificadores_temporarios: Vec::new(),
        }
    }

    /// Substitui o líder do grupo por um novo líder.
    ///
    /// Esta mudança afeta imediatamente a ideologia ativa do grupo e suas posturas legislativas.
    pub fn trocar_lider(&mut self, novo_lider: LiderPolitico) {
        self.lider = Some(novo_lider);
    }

    /// Avalia a postura do grupo diante da substituição da lei atual pela lei proposta.
    ///
    /// No Victoria 3, a ideologia pessoal do líder sobrepõe ou modula fortemente a postura
    /// tradicional do grupo.
    pub fn avaliar_postura_mudanca_lei(
        &self,
        catalogo: &CatalogoPolitica,
        lei_alvo_id: &str,
        lei_atual_id: &str,
    ) -> PosturaLei {
        let def_gi = catalogo.grupos_interesse.get(&self.id);

        // 1. Postura intrínseca da definição do grupo
        let mut pontos_atitude = 0i32;
        if let Some(gi) = def_gi {
            let postura_alvo = gi.postura_padrao_lei(lei_alvo_id);
            let postura_atual = gi.postura_padrao_lei(lei_atual_id);
            pontos_atitude += postura_alvo.valor() - postura_atual.valor();
        }

        // 2. Modulação ou sobreposição pela ideologia pessoal do líder
        if let Some(lider) = &self.lider
            && let Some(ideol_lider) = catalogo.ideologias.get(&lider.ideologia_id)
        {
            let postura_lider_alvo = ideol_lider.postura_lei(lei_alvo_id);
            let postura_lider_atual = ideol_lider.postura_lei(lei_atual_id);

            // O líder tem peso dominante sobre a postura do grupo
            let delta_lider = postura_lider_alvo.valor() - postura_lider_atual.valor();
            if delta_lider != 0 {
                pontos_atitude += 2 * delta_lider;
            }
        }

        match pontos_atitude {
            p if p >= 2 => PosturaLei::EndossaFortemente,
            1 => PosturaLei::Apoia,
            0 => PosturaLei::Neutro,
            -1 => PosturaLei::Opoe,
            _ => PosturaLei::RejeitaFortemente,
        }
    }

    /// Retorna verdadeiro se o grupo faz parte do governo civil.
    #[inline]
    pub fn eh_no_governo(&self) -> bool {
        matches!(self.papel, PapelGrupoInteresse::NoGoverno)
    }

    /// Retorna verdadeiro se o grupo está marginalizado.
    #[inline]
    pub fn eh_marginalizado(&self) -> bool {
        matches!(self.papel, PapelGrupoInteresse::Marginalizado)
    }
}
