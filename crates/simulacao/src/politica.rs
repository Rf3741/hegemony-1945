use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// As 7 categorias institucionais de leis do Estado (Seção 5.1 do `DESIGN.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoriaLei {
    Governo,
    Economia,
    Trabalho,
    DireitosCivis,
    SistemaTributario,
    SaudeEEducacao,
    Exercito,
}

impl CategoriaLei {
    pub const TODAS: [Self; 7] = [
        Self::Governo,
        Self::Economia,
        Self::Trabalho,
        Self::DireitosCivis,
        Self::SistemaTributario,
        Self::SaudeEEducacao,
        Self::Exercito,
    ];

    pub const fn chave(self) -> &'static str {
        match self {
            Self::Governo => "governo",
            Self::Economia => "economia",
            Self::Trabalho => "trabalho",
            Self::DireitosCivis => "direitos_civis",
            Self::SistemaTributario => "sistema_tributario",
            Self::SaudeEEducacao => "saude_e_educacao",
            Self::Exercito => "exercito",
        }
    }

    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::Governo => "Distribuição de Poder",
            Self::Economia => "Sistema Econômico",
            Self::Trabalho => "Relações Trabalhistas",
            Self::DireitosCivis => "Direitos Civis e Segurança",
            Self::SistemaTributario => "Sistema Tributário",
            Self::SaudeEEducacao => "Saúde e Educação",
            Self::Exercito => "Organização Militar",
        }
    }
}

/// Os 11 Grupos de Interesse sociopolíticos (Seção 5.2 do `DESIGN.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrupoInteresseId {
    ProprietariosRurais,
    Militares,
    Industriais,
    Burocratas,
    TrabalhadoresSindicalizados,
    Intelectuais,
    Clero,
    PopulacaoRural,
    PequenaBurguesia,
    NomenklaturaPartidaria,
    AdministracaoColonial,
}

impl GrupoInteresseId {
    pub const TODOS: [Self; 11] = [
        Self::ProprietariosRurais,
        Self::Militares,
        Self::Industriais,
        Self::Burocratas,
        Self::TrabalhadoresSindicalizados,
        Self::Intelectuais,
        Self::Clero,
        Self::PopulacaoRural,
        Self::PequenaBurguesia,
        Self::NomenklaturaPartidaria,
        Self::AdministracaoColonial,
    ];

    pub const fn nome_pt(self) -> &'static str {
        match self {
            Self::ProprietariosRurais => "Proprietários Rurais",
            Self::Militares => "Forças Armadas",
            Self::Industriais => "Industriais",
            Self::Burocratas => "Burocratas de Estado",
            Self::TrabalhadoresSindicalizados => "Trabalhadores Sindicalizados",
            Self::Intelectuais => "Intelectuais",
            Self::Clero => "Clero e Instituições Religiosas",
            Self::PopulacaoRural => "População Rural",
            Self::PequenaBurguesia => "Pequena Burguesia",
            Self::NomenklaturaPartidaria => "Nomenklatura Partidária",
            Self::AdministracaoColonial => "Administração Colonial",
        }
    }

    pub fn de_str_id(s: &str) -> Option<Self> {
        match s {
            "proprietarios_rurais" => Some(Self::ProprietariosRurais),
            "militares" => Some(Self::Militares),
            "industriais" => Some(Self::Industriais),
            "burocratas" => Some(Self::Burocratas),
            "trabalhadores_sindicalizados" => Some(Self::TrabalhadoresSindicalizados),
            "intelectuais" => Some(Self::Intelectuais),
            "clero" => Some(Self::Clero),
            "populacao_rural" => Some(Self::PopulacaoRural),
            "pequena_burguesia" => Some(Self::PequenaBurguesia),
            "nomenklatura_partidaria" => Some(Self::NomenklaturaPartidaria),
            "administracao_colonial" => Some(Self::AdministracaoColonial),
            _ => None,
        }
    }

    /// Posição ideológica média no eixo Capitalismo (-1.0) <-> Socialismo (+1.0).
    pub const fn inclinacao_bipolar(self) -> f32 {
        match self {
            Self::Industriais => -0.85,
            Self::ProprietariosRurais => -0.60,
            Self::Clero => -0.55,
            Self::AdministracaoColonial => -0.65,
            Self::PequenaBurguesia => -0.35,
            Self::Militares => -0.15,
            Self::Intelectuais => 0.05,
            Self::PopulacaoRural => 0.10,
            Self::Burocratas => 0.15,
            Self::TrabalhadoresSindicalizados => 0.55,
            Self::NomenklaturaPartidaria => 0.95,
        }
    }
}

/// Estado dinâmico de um Grupo de Interesse dentro de um país.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoGrupoInteresse {
    pub id: GrupoInteresseId,
    /// Força política normalizada (`0.0..=1.0`, soma nacional = `1.0`).
    pub clout: f32,
    /// Parcela de votos na última eleição (`0.0..=1.0`).
    pub parcela_votos: f32,
    /// Aprovação em relação ao governo e leis vigentes (`-20.0..=+20.0`).
    pub aprovacao: f32,
    pub em_governo: bool,
}

impl EstadoGrupoInteresse {
    pub const fn marginalizado(&self) -> bool {
        self.clout < 0.04
    }

    pub const fn influente(&self) -> bool {
        self.clout >= 0.20
    }
}

/// Processo legislativo em andamento para promulgar uma nova lei em 3 marcos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReformaLeiAtiva {
    pub categoria: CategoriaLei,
    pub nova_lei: String,
    /// Marco atual (`0`, `1` ou `2`; ao atingir `3` a lei é promulgada).
    pub marcos_concluidos: u8,
    pub dias_no_marco: u16,
    pub chance_avanco: f32,
}

/// Classificação legível da posição de um país nas Esferas de Influência EUA vs. URSS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EsferaInfluencia {
    EsferaOcidentalConsolidada,
    InclinadoOcidente,
    NaoAlinhado,
    InclinadoBlocoOriental,
    EsferaSovieticaConsolidada,
}

impl EsferaInfluencia {
    pub fn de_pontuacao(score: f32) -> Self {
        if score <= -60.0 {
            Self::EsferaOcidentalConsolidada
        } else if score <= -20.0 {
            Self::InclinadoOcidente
        } else if score < 20.0 {
            Self::NaoAlinhado
        } else if score < 60.0 {
            Self::InclinadoBlocoOriental
        } else {
            Self::EsferaSovieticaConsolidada
        }
    }

    pub const fn rotulo_pt(self) -> &'static str {
        match self {
            Self::EsferaOcidentalConsolidada => "Esfera Ocidental (EUA)",
            Self::InclinadoOcidente => "Inclinado ao Ocidente",
            Self::NaoAlinhado => "Não Alinhado / Neutro",
            Self::InclinadoBlocoOriental => "Inclinado à URSS",
            Self::EsferaSovieticaConsolidada => "Esfera Soviética (URSS)",
        }
    }
}

/// Estado político completo de uma nação ou administração colonial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoPolitico {
    pub leis_ativas: BTreeMap<CategoriaLei, String>,
    pub grupos: BTreeMap<GrupoInteresseId, EstadoGrupoInteresse>,
    pub legitimidade: f32,
    /// Dias restantes para o próximo pleito eleitoral (`None` em autocracias, colônias ou partido único).
    pub dias_ate_eleicao: Option<u32>,
    pub reforma_ativa: Option<ReformaLeiAtiva>,
    /// Pontuação bipolar (`-100.0` = Washington / Capitalismo, `+100.0` = Moscou / Socialismo).
    pub influencia_bipolar: f32,
}

impl EstadoPolitico {
    /// Verifica se a lei de governo atual realiza eleições periódicas.
    pub fn possui_eleicoes(&self) -> bool {
        matches!(
            self.lei_ativa(CategoriaLei::Governo),
            "sufragio_universal" | "voto_censitario" | "voto_capacitario"
        )
    }

    pub fn lei_ativa(&self, categoria: CategoriaLei) -> &str {
        self.leis_ativas
            .get(&categoria)
            .map(String::as_str)
            .unwrap_or(match categoria {
                CategoriaLei::Governo => "oligarquia",
                CategoriaLei::Economia => "capitalismo_de_intervencao",
                CategoriaLei::Trabalho => "regulamentacao_basica",
                CategoriaLei::DireitosCivis => "liberdades_restringidas",
                CategoriaLei::SistemaTributario => "tributacao_proporcional",
                CategoriaLei::SaudeEEducacao => "sistema_publico_basico",
                CategoriaLei::Exercito => "alistamento_obrigatorio",
            })
    }

    pub fn esfera_atual(&self) -> EsferaInfluencia {
        EsferaInfluencia::de_pontuacao(self.influencia_bipolar)
    }

    /// Normaliza os *clouts* para que somem exatamente `1.0`.
    pub fn normalizar_clouts(&mut self) {
        let soma: f32 = self.grupos.values().map(|g| g.clout.max(0.001)).sum();
        if soma > 0.0 {
            for g in self.grupos.values_mut() {
                g.clout = g.clout.max(0.001) / soma;
            }
        }
    }

    /// Recalcula a Legitimidade (`0..=100`) conforme a lei de `governo` ativa e a coalizão (Seção 5.3 do `DESIGN.md`).
    pub fn recalcular_legitimidade(&mut self) {
        let lei_gov = self.lei_ativa(CategoriaLei::Governo);
        let (base, peso_clout, peso_votos) = match lei_gov {
            "sufragio_universal" => (20.0_f32, 45.0_f32, 55.0_f32),
            "voto_capacitario" | "voto_censitario" => (25.0, 55.0, 40.0),
            "partido_unico" => (40.0, 65.0, 0.0),
            "autocracia" => (45.0, 60.0, 0.0),
            "oligarquia" => (35.0, 65.0, 0.0),
            "governador_colonial" | "ocupacao_militar" => (38.0, 55.0, 0.0),
            _ => (30.0, 60.0, 0.0),
        };

        let mut soma_clout = 0.0_f32;
        let mut soma_votos = 0.0_f32;
        let mut num_gov = 0_usize;
        let mut min_eixo = f32::MAX;
        let mut max_eixo = f32::MIN;

        for g in self.grupos.values().filter(|g| g.em_governo) {
            soma_clout += g.clout;
            soma_votos += g.parcela_votos;
            num_gov += 1;
            let eixo = g.id.inclinacao_bipolar();
            min_eixo = min_eixo.min(eixo);
            max_eixo = max_eixo.max(eixo);
        }

        let penalidade_tamanho = (num_gov.saturating_sub(2) as f32) * 6.0;
        let divergencia = if num_gov >= 2 {
            (max_eixo - min_eixo).max(0.0) * 10.0
        } else {
            0.0
        };

        let bruta = base + peso_clout * soma_clout + peso_votos * soma_votos
            - penalidade_tamanho
            - divergencia;
        self.legitimidade = bruta.clamp(0.0, 100.0);
    }

    /// Define uma nova coalizão de governo e atualiza imediatamente a legitimidade.
    pub fn definir_coalizao(&mut self, membros: &BTreeSet<GrupoInteresseId>) {
        for g in self.grupos.values_mut() {
            g.em_governo = membros.contains(&g.id);
        }
        self.recalcular_legitimidade();
    }

    /// Realiza eleição nacional recalculando a parcela de votos de cada Grupo de Interesse.
    pub fn realizar_eleicao(&mut self, semente_deterministica: u64) {
        let mut pesos = BTreeMap::new();
        let mut soma = 0.0_f32;

        for (&id, g) in &self.grupos {
            if matches!(
                id,
                GrupoInteresseId::AdministracaoColonial | GrupoInteresseId::NomenklaturaPartidaria
            ) && self.lei_ativa(CategoriaLei::Governo) != "partido_unico"
            {
                pesos.insert(id, 0.01);
                soma += 0.01;
                continue;
            }
            // Momentum determinístico leve derivado da semente e aprovação
            let hash = semente_deterministica
                .wrapping_mul(6364136223846793005)
                .wrapping_add(id as u64 * 97);
            let variacao = (((hash >> 32) % 21) as f32 - 10.0) * 0.01; // -0.10 .. +0.10
            let fator_aprovacao = 1.0 + (g.aprovacao * 0.015);
            let peso = (g.clout * fator_aprovacao * (1.0 + variacao)).max(0.005);
            pesos.insert(id, peso);
            soma += peso;
        }

        if soma > 0.0 {
            for (&id, g) in &mut self.grupos {
                g.parcela_votos = pesos.get(&id).copied().unwrap_or(0.0) / soma;
            }
        }
        self.dias_ate_eleicao = Some(1460);
        self.recalcular_legitimidade();
    }

    /// Inicia a tramitação de uma nova lei institucional.
    pub fn iniciar_reforma(&mut self, categoria: CategoriaLei, nova_lei: impl Into<String>) {
        let nova = nova_lei.into();
        if self.lei_ativa(categoria) == nova {
            return;
        }
        let apoio_gov: f32 = self
            .grupos
            .values()
            .filter(|g| g.em_governo)
            .map(|g| g.clout)
            .sum();
        let bonus_leg = (self.legitimidade - 50.0) / 200.0;
        let chance = (0.20 + 0.55 * apoio_gov + bonus_leg).clamp(0.05, 0.85);

        self.reforma_ativa = Some(ReformaLeiAtiva {
            categoria,
            nova_lei: nova,
            marcos_concluidos: 0,
            dias_no_marco: 0,
            chance_avanco: chance,
        });
    }

    /// Avança um dia de política interna e deriva diplomática EUA/URSS.
    pub fn avancar_dia(&mut self, tag_pais: &str, semente_dia: u64) {
        // 1. Ciclo eleitoral
        if self.possui_eleicoes() {
            let atual = self.dias_ate_eleicao.unwrap_or(1460);
            if atual <= 1 {
                self.realizar_eleicao(semente_dia);
            } else {
                self.dias_ate_eleicao = Some(atual - 1);
            }
        } else {
            self.dias_ate_eleicao = None;
        }

        // 2. Tramitação de reforma de lei a cada 30 dias
        if let Some(mut ref_lei) = self.reforma_ativa.take() {
            ref_lei.dias_no_marco += 1;
            if ref_lei.dias_no_marco >= 30 {
                ref_lei.dias_no_marco = 0;
                let rolo = ((semente_dia.wrapping_mul(2862933555777941757) >> 32) % 1000) as f32
                    / 1000.0;
                if rolo <= ref_lei.chance_avanco {
                    ref_lei.marcos_concluidos += 1;
                }
            }
            if ref_lei.marcos_concluidos >= 3 {
                self.leis_ativas.insert(ref_lei.categoria, ref_lei.nova_lei);
                self.recalcular_legitimidade();
            } else {
                self.reforma_ativa = Some(ref_lei);
            }
        }

        // 3. Deriva diária das Esferas de Influência (Seção 5.4 do DESIGN.md)
        if tag_pais == "USA" {
            self.influencia_bipolar = -100.0;
            return;
        }
        if tag_pais == "SOV" {
            self.influencia_bipolar = 100.0;
            return;
        }

        let afinidade_econ = match self.lei_ativa(CategoriaLei::Economia) {
            "economia_planificada" => 45.0_f32,
            "livre_mercado" => -45.0,
            "capitalismo_de_intervencao" => -20.0,
            _ => 0.0,
        };
        let afinidade_gov = match self.lei_ativa(CategoriaLei::Governo) {
            "partido_unico" => 45.0_f32,
            "sufragio_universal" => -25.0,
            _ => 0.0,
        };
        let alvo = (afinidade_econ + afinidade_gov).clamp(-95.0, 95.0);
        let delta = 0.004 * (alvo - self.influencia_bipolar);
        self.influencia_bipolar = (self.influencia_bipolar + delta).clamp(-100.0, 100.0);
    }
}

/// Constrói um `EstadoPolitico` inicial coerente a partir das leis e do alinhamento de 1945.
pub fn construir_politica_inicial(
    tag: &str,
    leis_str: &BTreeMap<String, String>,
    alinhamento_inicial_score: f32,
) -> EstadoPolitico {
    let mut leis_ativas = BTreeMap::new();
    for cat in CategoriaLei::TODAS {
        if let Some(v) = leis_str.get(cat.chave()) {
            leis_ativas.insert(cat, v.clone());
        } else {
            leis_ativas.insert(
                cat,
                match cat {
                    CategoriaLei::Governo => "oligarquia",
                    CategoriaLei::Economia => "capitalismo_de_intervencao",
                    CategoriaLei::Trabalho => "regulamentacao_basica",
                    CategoriaLei::DireitosCivis => "liberdades_restringidas",
                    CategoriaLei::SistemaTributario => "tributacao_proporcional",
                    CategoriaLei::SaudeEEducacao => "sistema_publico_basico",
                    CategoriaLei::Exercito => "alistamento_obrigatorio",
                }
                .to_string(),
            );
        }
    }

    let lei_gov = leis_ativas
        .get(&CategoriaLei::Governo)
        .map(String::as_str)
        .unwrap_or("oligarquia");
    let lei_econ = leis_ativas
        .get(&CategoriaLei::Economia)
        .map(String::as_str)
        .unwrap_or("capitalismo_de_intervencao");

    let mut grupos = BTreeMap::new();
    for id in GrupoInteresseId::TODOS {
        let mut peso = match id {
            GrupoInteresseId::Industriais => 0.14_f32,
            GrupoInteresseId::Militares => 0.15,
            GrupoInteresseId::TrabalhadoresSindicalizados => 0.13,
            GrupoInteresseId::ProprietariosRurais => 0.12,
            GrupoInteresseId::PopulacaoRural => 0.12,
            GrupoInteresseId::Burocratas => 0.10,
            GrupoInteresseId::Intelectuais => 0.08,
            GrupoInteresseId::PequenaBurguesia => 0.07,
            GrupoInteresseId::Clero => 0.06,
            GrupoInteresseId::NomenklaturaPartidaria => 0.02,
            GrupoInteresseId::AdministracaoColonial => 0.01,
        };

        if lei_gov == "partido_unico" || lei_econ == "economia_planificada" {
            if id == GrupoInteresseId::NomenklaturaPartidaria {
                peso = 0.42;
            } else if matches!(
                id,
                GrupoInteresseId::Industriais
                    | GrupoInteresseId::ProprietariosRurais
                    | GrupoInteresseId::Clero
            ) {
                peso = 0.01;
            }
        } else if lei_gov == "governador_colonial" {
            if id == GrupoInteresseId::AdministracaoColonial {
                peso = 0.38;
            }
        } else if lei_gov == "sufragio_universal"
            && matches!(
                id,
                GrupoInteresseId::Industriais
                    | GrupoInteresseId::TrabalhadoresSindicalizados
                    | GrupoInteresseId::Intelectuais
            )
        {
            peso += 0.07;
        }

        let em_governo = match lei_gov {
            "partido_unico" => matches!(
                id,
                GrupoInteresseId::NomenklaturaPartidaria | GrupoInteresseId::Militares
            ),
            "governador_colonial" => matches!(
                id,
                GrupoInteresseId::AdministracaoColonial | GrupoInteresseId::Militares
            ),
            "ocupacao_militar" => matches!(
                id,
                GrupoInteresseId::Militares | GrupoInteresseId::Burocratas
            ),
            "sufragio_universal" => matches!(
                id,
                GrupoInteresseId::Industriais
                    | GrupoInteresseId::Intelectuais
                    | GrupoInteresseId::TrabalhadoresSindicalizados
            ),
            _ => matches!(
                id,
                GrupoInteresseId::ProprietariosRurais
                    | GrupoInteresseId::Militares
                    | GrupoInteresseId::Burocratas
            ),
        };

        grupos.insert(
            id,
            EstadoGrupoInteresse {
                id,
                clout: peso,
                parcela_votos: peso,
                aprovacao: if em_governo { 4.5 } else { 0.0 },
                em_governo,
            },
        );
    }

    let mut estado = EstadoPolitico {
        leis_ativas,
        grupos,
        legitimidade: 60.0,
        dias_ate_eleicao: None,
        reforma_ativa: None,
        influencia_bipolar: if tag == "USA" {
            -100.0
        } else if tag == "SOV" {
            100.0
        } else {
            alinhamento_inicial_score.clamp(-100.0, 100.0)
        },
    };
    estado.normalizar_clouts();
    let soma_votos: f32 = estado.grupos.values().map(|g| g.parcela_votos).sum();
    if soma_votos > 0.0 {
        for g in estado.grupos.values_mut() {
            g.parcela_votos /= soma_votos;
        }
    }
    if estado.possui_eleicoes() {
        estado.dias_ate_eleicao = Some(1380); // Próxima grande rodada eleitoral (~nov/1948 nos EUA)
    }
    estado.recalcular_legitimidade();
    estado
}
