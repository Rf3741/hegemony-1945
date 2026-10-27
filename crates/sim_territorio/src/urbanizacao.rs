//! Mecânicas de desenvolvimento urbano, crescimento de cidades e níveis de industrialização.

use crate::catalogo::CatalogoTerritorio;
use crate::infraestrutura::RedeInfraestruturaEstado;
use crate::tipos::{EraTecnologicaVisual, EstiloArquitetonico, NivelCidade, NivelIndustrial};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Entidade de cidade individual dentro de um estado.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cidade {
    pub id: String,
    pub nome: String,
    pub regiao_adm1: String,
    #[serde(default)]
    pub eh_capital: bool,
    #[serde(default)]
    pub eh_portuaria: bool,
    #[serde(default = "padrao_peso_atracao")]
    pub peso_atracao_base: f64,
    #[serde(default)]
    pub populacao_urbana: u64,
    #[serde(default)]
    pub empregos_industriais: u64,
    #[serde(default)]
    pub empregos_servicos: u64,
    #[serde(default = "padrao_tamanho_visual")]
    pub tamanho_visual: f64,
    #[serde(default)]
    pub nivel: NivelCidade,
    #[serde(default)]
    pub era_visual: EraTecnologicaVisual,
    #[serde(default)]
    pub coordenadas: Option<(f32, f32)>,
}

const fn padrao_peso_atracao() -> f64 {
    1.0
}

const fn padrao_tamanho_visual() -> f64 {
    1.0
}

impl Cidade {
    pub fn nova(
        id: impl Into<String>,
        nome: impl Into<String>,
        regiao_adm1: impl Into<String>,
        eh_capital: bool,
    ) -> Self {
        Self {
            id: id.into(),
            nome: nome.into(),
            regiao_adm1: regiao_adm1.into(),
            eh_capital,
            eh_portuaria: false,
            peso_atracao_base: if eh_capital { 1.5 } else { 1.0 },
            populacao_urbana: 0,
            empregos_industriais: 0,
            empregos_servicos: 0,
            tamanho_visual: 1.0,
            nivel: NivelCidade::PovoadoRural,
            era_visual: EraTecnologicaVisual::Era1945,
            coordenadas: None,
        }
    }
}

/// Estado das métricas de urbanização, empregos e industrialização de um estado.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstadoUrbano {
    /// Taxa efetiva de urbanização suavemente atualizada (0.05 a 0.98).
    #[serde(default = "padrao_taxa_urb")]
    pub taxa_urbanizacao: f64,
    /// Taxa de urbanização estrutural de equilíbrio dada pela soma dos prédios.
    #[serde(default = "padrao_taxa_urb")]
    pub taxa_urbanizacao_alvo: f64,
    /// Pontos de urbanização somados de todos os prédios construídos no estado.
    #[serde(default)]
    pub pontos_urbanizacao_predios: f64,
    /// Total de vagas/postos de trabalho industriais ocupados.
    #[serde(default)]
    pub empregos_industriais_totais: u64,
    /// Total de postos de trabalho no setor de serviços ocupados.
    #[serde(default)]
    pub empregos_servicos_totais: u64,
    /// Postos de trabalho rurais e de subsistência ocupados.
    #[serde(default)]
    pub empregos_rurais_totais: u64,
    /// Habitantes residentes em áreas urbanas no estado.
    #[serde(default)]
    pub populacao_urbana_total: u64,
    /// Habitantes residentes em áreas rurais no estado.
    #[serde(default)]
    pub populacao_rural_total: u64,
    /// População residente total do estado.
    #[serde(default)]
    pub populacao_total: u64,

    /// Índice de urbanização suave (0.0 a 100.0) sem saltos bruscos.
    #[serde(default)]
    pub indice_urbano_suavizado: f64,
    /// Índice de urbanização teórico alvo (0.0 a 100.0).
    #[serde(default)]
    pub indice_urbano_alvo: f64,
    /// Nível de cidade dominante do estado (escala de 1 a 8).
    #[serde(default)]
    pub nivel_cidade: NivelCidade,
    /// Contador de dias desde a última transição de nível de cidade (para histerese e estabilidade).
    #[serde(default)]
    pub dias_no_nivel_cidade: u32,

    /// Índice industrial suave (0.0 a 100.0).
    #[serde(default)]
    pub indice_industrial_suavizado: f64,
    /// Índice industrial teórico alvo (0.0 a 100.0).
    #[serde(default)]
    pub indice_industrial_alvo: f64,
    /// Nível industrial do estado (escala de 0 a 8).
    #[serde(default)]
    pub nivel_industrial: NivelIndustrial,
    /// Contador de dias no nível industrial atual.
    #[serde(default)]
    pub dias_no_nivel_industrial: u32,
}

const fn padrao_taxa_urb() -> f64 {
    0.15
}

impl Default for EstadoUrbano {
    fn default() -> Self {
        Self {
            taxa_urbanizacao: 0.15,
            taxa_urbanizacao_alvo: 0.15,
            pontos_urbanizacao_predios: 0.0,
            empregos_industriais_totais: 0,
            empregos_servicos_totais: 0,
            empregos_rurais_totais: 0,
            populacao_urbana_total: 0,
            populacao_rural_total: 0,
            populacao_total: 0,
            indice_urbano_suavizado: 10.0,
            indice_urbano_alvo: 10.0,
            nivel_cidade: NivelCidade::PovoadoRural,
            dias_no_nivel_cidade: 0,
            indice_industrial_suavizado: 0.0,
            indice_industrial_alvo: 0.0,
            nivel_industrial: NivelIndustrial::SemIndustria,
            dias_no_nivel_industrial: 0,
        }
    }
}

/// Descritor completo pronto para a camada de renderização Bevy (visual do mapa).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DescritorVisualEstado {
    pub nivel_cidade: u8,
    pub nivel_industrial: u8,
    pub era_visual: EraTecnologicaVisual,
    pub estilo_arquitetonico: EstiloArquitetonico,
    /// Densidade do tecido urbano para texturização (0.0 a 1.0).
    pub densidade_urbana: f32,
    /// Densidade de galpões e chaminés fabris (0.0 a 1.0).
    pub densidade_fabricas: f32,
    /// Cobertura de iluminação noturna (0.0 a 1.0).
    pub iluminacao_noturna: f32,
    /// Fumaça e fuligem visível (0.0 a 1.0).
    pub polucao_fumaca: f32,
}

impl EstadoUrbano {
    /// Recalcula o equilíbrio estrutural urbano a partir dos prédios e população.
    /// Chamado tanto na inicialização quanto a cada tick diário.
    pub fn recalcular_alvos(
        &mut self,
        catalogo: &CatalogoTerritorio,
        predios_totais: &BTreeMap<String, u32>,
        populacao_total: u64,
        infra: &RedeInfraestruturaEstado,
    ) {
        self.populacao_total = populacao_total;

        let mut pontos_urb = 0.0;
        let mut vagas_ind: u64 = 0;
        let mut vagas_serv: u64 = 0;
        let mut vagas_rur: u64 = 0;
        let mut peso_ind = 0.0;

        for (pred_id, &nivel) in predios_totais {
            if nivel == 0 {
                continue;
            }
            let nivel_f = nivel as f64;
            if let Some(def) = catalogo.buscar_predio(pred_id) {
                pontos_urb += nivel_f * def.pontos_urbanizacao_por_nivel;
                vagas_ind += (nivel as u64) * (def.empregos_industriais_por_nivel as u64);
                vagas_serv += (nivel as u64) * (def.empregos_servicos_por_nivel as u64);
                vagas_rur += (nivel as u64) * (def.empregos_rurais_por_nivel as u64);
                peso_ind += nivel_f * def.peso_nivel_industrial;
            } else {
                // Heurística de fallback caso o prédio seja customizado
                pontos_urb += nivel_f * 20.0;
                vagas_ind += (nivel as u64) * 200;
                vagas_serv += (nivel as u64) * 100;
                peso_ind += nivel_f * 1.5;
            }
        }

        // Empregos de serviços induzidos pela aglomeração urbana (estilo Victoria 3)
        let servicos_induzidos = (pontos_urb * 18.0) as u64;
        let vagas_serv_totais = vagas_serv + servicos_induzidos;

        // Limita postos de trabalho à força de trabalho economicamente ativa (~48%)
        let forca_trabalho = ((populacao_total as f64) * 0.48).round() as u64;
        let vagas_urbanas_desejadas = vagas_ind + vagas_serv_totais;

        let (emp_ind, emp_serv) = if vagas_urbanas_desejadas <= forca_trabalho {
            (vagas_ind, vagas_serv_totais)
        } else if vagas_urbanas_desejadas > 0 {
            let fator = (forca_trabalho as f64) / (vagas_urbanas_desejadas as f64);
            (
                ((vagas_ind as f64) * fator).round() as u64,
                ((vagas_serv_totais as f64) * fator).round() as u64,
            )
        } else {
            (0, 0)
        };

        let emp_rur = vagas_rur.min(forca_trabalho.saturating_sub(emp_ind + emp_serv));

        self.pontos_urbanizacao_predios = pontos_urb;
        self.empregos_industriais_totais = emp_ind;
        self.empregos_servicos_totais = emp_serv;
        self.empregos_rurais_totais = emp_rur;

        // Fator de infraestrutura para sustentação urbana
        let fator_infra = 0.65 + 0.35 * infra.conectividade_logistica();

        // População urbana sustentada pelas vagas e pontos de urbanização dos prédios
        let emp_urb = emp_ind + emp_serv;
        let pop_urb_sustentada = ((emp_urb as f64) * 2.15 + pontos_urb * 380.0) * fator_infra;

        if populacao_total > 0 {
            let razao_bruta = pop_urb_sustentada / (populacao_total as f64);
            // Curva suave com saturação assintótica em 97%
            self.taxa_urbanizacao_alvo =
                (0.06 + 0.91 * (razao_bruta / (razao_bruta + 0.55))).clamp(0.05, 0.97);
        } else {
            self.taxa_urbanizacao_alvo = 0.05;
        }

        // Cálculo do Índice Urbano Alvo (0.0..=100.0)
        // 1. Componente População Urbana (escala logarítmica, até 38 pontos)
        let pop_urb_alvo = (populacao_total as f64) * self.taxa_urbanizacao_alvo;
        let comp_pop = (1.0 + pop_urb_alvo / 20_000.0).ln() / 6.0;
        let comp_pop_pontos = comp_pop.clamp(0.0, 1.0) * 38.0;

        // 2. Componente Empregos Industriais + Serviços (escala logarítmica, até 30 pontos)
        let comp_emp = (1.0 + (emp_urb as f64) / 5_000.0).ln() / 5.8;
        let comp_emp_pontos = comp_emp.clamp(0.0, 1.0) * 30.0;

        // 3. Componente Prédios Urbanos e Taxa de Urbanização (até 20 pontos)
        let comp_predios = self.taxa_urbanizacao_alvo * 12.0
            + ((1.0 + pontos_urb / 40.0).ln() / 4.8).clamp(0.0, 1.0) * 8.0;

        // 4. Componente Infraestrutura e Energia (até 12 pontos)
        let comp_infra = infra.conectividade_logistica() * 8.0 + infra.cobertura_eletrica() * 4.0;

        self.indice_urbano_alvo =
            (comp_pop_pontos + comp_emp_pontos + comp_predios + comp_infra).clamp(0.0, 100.0);

        // Cálculo do Índice Industrial Alvo (0.0..=100.0)
        let cobertura_energia = infra.cobertura_eletrica();
        let peso_ind_efetivo = peso_ind * (0.40 + 0.60 * cobertura_energia);
        self.indice_industrial_alvo =
            (((1.0 + peso_ind_efetivo / 8.0).ln() / 4.6).clamp(0.0, 1.0) * 100.0).clamp(0.0, 100.0);
    }

    /// Inicializa imediatamente no ponto de equilíbrio (ex: ao carregar cenário inicial de 1945).
    pub fn inicializar_equilibrio_imediato(
        &mut self,
        catalogo: &CatalogoTerritorio,
        predios_totais: &BTreeMap<String, u32>,
        populacao_total: u64,
        infra: &RedeInfraestruturaEstado,
    ) {
        self.recalcular_alvos(catalogo, predios_totais, populacao_total, infra);
        self.taxa_urbanizacao = self.taxa_urbanizacao_alvo;
        self.indice_urbano_suavizado = self.indice_urbano_alvo;
        self.indice_industrial_suavizado = self.indice_industrial_alvo;

        self.populacao_urbana_total =
            ((self.populacao_total as f64) * self.taxa_urbanizacao).round() as u64;
        self.populacao_rural_total = self
            .populacao_total
            .saturating_sub(self.populacao_urbana_total);

        self.nivel_cidade = NivelCidade::a_partir_u8(determinar_nivel_de_cidade(
            self.indice_urbano_suavizado,
            self.nivel_cidade.valor(),
            0,
        ));
        self.nivel_industrial = NivelIndustrial::a_partir_u8(determinar_nivel_industrial(
            self.indice_industrial_suavizado,
            self.nivel_industrial.valor(),
            0,
        ));
        self.dias_no_nivel_cidade = 0;
        self.dias_no_nivel_industrial = 0;
    }

    /// Avança 1 tick diário na evolução urbana e industrial suave do estado.
    /// Garante que `taxa_urbanizacao`, `indice_urbano_suavizado` e `nivel_cidade`
    /// evoluam de forma contínua e estável (sem saltos de múltiplos níveis em um único tick).
    pub fn avancar_tick_diario(
        &mut self,
        catalogo: &CatalogoTerritorio,
        predios_totais: &BTreeMap<String, u32>,
        populacao_total: u64,
        infra: &RedeInfraestruturaEstado,
    ) {
        self.recalcular_alvos(catalogo, predios_totais, populacao_total, infra);

        // 1. Convergência suave da taxa de urbanização (~180 dias de meia-vida)
        let delta_taxa = self.taxa_urbanizacao_alvo - self.taxa_urbanizacao;
        self.taxa_urbanizacao += delta_taxa * 0.004;
        self.taxa_urbanizacao = self.taxa_urbanizacao.clamp(0.05, 0.98);

        self.populacao_urbana_total =
            ((self.populacao_total as f64) * self.taxa_urbanizacao).round() as u64;
        self.populacao_rural_total = self
            .populacao_total
            .saturating_sub(self.populacao_urbana_total);

        // 2. Convergência suave do índice urbano (~170 dias de meia-vida)
        let delta_indice_urb = self.indice_urbano_alvo - self.indice_urbano_suavizado;
        self.indice_urbano_suavizado += delta_indice_urb * 0.0045;
        self.indice_urbano_suavizado = self.indice_urbano_suavizado.clamp(0.0, 100.0);

        // 3. Convergência suave do índice industrial (~150 dias de meia-vida)
        let delta_indice_ind = self.indice_industrial_alvo - self.indice_industrial_suavizado;
        self.indice_industrial_suavizado += delta_indice_ind * 0.005;
        self.indice_industrial_suavizado = self.indice_industrial_suavizado.clamp(0.0, 100.0);

        self.dias_no_nivel_cidade = self.dias_no_nivel_cidade.saturating_add(1);
        self.dias_no_nivel_industrial = self.dias_no_nivel_industrial.saturating_add(1);

        // 4. Determinação dos níveis discretos com histerese e transição suave
        let novo_nivel_cid = determinar_nivel_de_cidade(
            self.indice_urbano_suavizado,
            self.nivel_cidade.valor(),
            self.dias_no_nivel_cidade,
        );
        if novo_nivel_cid != self.nivel_cidade.valor() {
            self.nivel_cidade = NivelCidade::a_partir_u8(novo_nivel_cid);
            self.dias_no_nivel_cidade = 0;
        }

        let novo_nivel_ind = determinar_nivel_industrial(
            self.indice_industrial_suavizado,
            self.nivel_industrial.valor(),
            self.dias_no_nivel_industrial,
        );
        if novo_nivel_ind != self.nivel_industrial.valor() {
            self.nivel_industrial = NivelIndustrial::a_partir_u8(novo_nivel_ind);
            self.dias_no_nivel_industrial = 0;
        }
    }

    /// Distribui a população urbana e os empregos entre as cidades do estado.
    pub fn alocar_cidades(
        &self,
        cidades: &mut [Cidade],
        era_estado: EraTecnologicaVisual,
        infra: &RedeInfraestruturaEstado,
    ) {
        if cidades.is_empty() {
            return;
        }

        // Calcula pesos de atração dinâmicos para cada cidade
        let mut pesos: Vec<f64> = cidades
            .iter()
            .map(|c| {
                let mut p = c.peso_atracao_base;
                if c.eh_capital {
                    p *= 1.4;
                }
                if c.eh_portuaria && infra.eh_costeiro {
                    p *= 1.25 + 0.20 * infra.portos.taxa_utilizacao().min(1.0);
                }
                p.max(0.1)
            })
            .collect();

        let soma_pesos: f64 = pesos.iter().sum();
        if soma_pesos > 0.0 {
            for p in &mut pesos {
                *p /= soma_pesos;
            }
        } else {
            pesos.fill(1.0 / (cidades.len() as f64));
        }

        let pop_urb_total = self.populacao_urbana_total as f64;
        let emp_ind_total = self.empregos_industriais_totais as f64;
        let emp_serv_total = self.empregos_servicos_totais as f64;

        for (idx, cid) in cidades.iter_mut().enumerate() {
            let f = pesos[idx];
            cid.populacao_urbana = (pop_urb_total * f).round() as u64;
            cid.empregos_industriais = (emp_ind_total * f).round() as u64;
            cid.empregos_servicos = (emp_serv_total * f).round() as u64;
            cid.era_visual = era_estado;

            // Nível individual da cidade proporcional ao peso relativo e ao nível estadual
            let indice_cidade =
                (self.indice_urbano_suavizado * (f / pesos[0].max(0.001)).sqrt()).clamp(0.0, 100.0);
            cid.nivel = NivelCidade::a_partir_u8(determinar_nivel_de_cidade(
                indice_cidade,
                cid.nivel.valor(),
                0,
            ));

            // Tamanho visual contínuo (escala 1.0 a 100.0)
            cid.tamanho_visual = (1.0 + (cid.populacao_urbana as f64) / 40_000.0).clamp(1.0, 100.0);
        }
    }

    /// Gera o descritor visual completo para Bevy.
    pub fn descritor_visual(
        &self,
        era: EraTecnologicaVisual,
        estilo: EstiloArquitetonico,
        infra: &RedeInfraestruturaEstado,
    ) -> DescritorVisualEstado {
        let dens_urb = (self.indice_urbano_suavizado / 100.0) as f32;
        let dens_fab = (self.indice_industrial_suavizado / 100.0) as f32;
        let ilum = (dens_urb * 0.70 + (infra.cobertura_eletrica() as f32) * 0.30).clamp(0.0, 1.0);
        let fumaca = match era {
            EraTecnologicaVisual::Era1945 => (dens_fab * 0.90).clamp(0.0, 1.0),
            EraTecnologicaVisual::Era1955 => (dens_fab * 0.75).clamp(0.0, 1.0),
            EraTecnologicaVisual::Era1965 => (dens_fab * 0.60).clamp(0.0, 1.0),
            EraTecnologicaVisual::Era1975 => (dens_fab * 0.45).clamp(0.0, 1.0),
            EraTecnologicaVisual::Era1985 => (dens_fab * 0.25).clamp(0.0, 1.0),
        };

        DescritorVisualEstado {
            nivel_cidade: self.nivel_cidade.valor(),
            nivel_industrial: self.nivel_industrial.valor(),
            era_visual: era,
            estilo_arquitetonico: estilo,
            densidade_urbana: dens_urb,
            densidade_fabricas: dens_fab,
            iluminacao_noturna: ilum,
            polucao_fumaca: fumaca,
        }
    }
}

/// Determina o nível de cidade (1 a 8) com histerese para evitar oscilações na fronteira.
/// Limita a transição a no máximo 1 nível por vez se dias_no_nivel for menor que o período de carência.
fn determinar_nivel_de_cidade(indice: f64, nivel_atual: u8, dias_no_nivel: u32) -> u8 {
    let nivel_alvo = if indice < 14.0 {
        1
    } else if indice < 26.0 {
        2
    } else if indice < 38.0 {
        3
    } else if indice < 50.0 {
        4
    } else if indice < 62.0 {
        5
    } else if indice < 74.0 {
        6
    } else if indice < 86.0 {
        7
    } else {
        8
    };

    // Aplica histerese: para rebaixar, exige margem de 1.8 pontos
    if nivel_alvo < nivel_atual {
        let limiar_rebaixamento = match nivel_atual {
            2 => 14.0 - 1.8,
            3 => 26.0 - 1.8,
            4 => 38.0 - 1.8,
            5 => 50.0 - 1.8,
            6 => 62.0 - 1.8,
            7 => 74.0 - 1.8,
            8 => 86.0 - 1.8,
            _ => 0.0,
        };
        if indice >= limiar_rebaixamento {
            return nivel_atual;
        }
    }

    // Passagem suave: transiciona no máximo 1 nível se houver contagem contínua de dias
    if dias_no_nivel > 0 {
        if nivel_alvo > nivel_atual && dias_no_nivel >= 45 {
            nivel_atual + 1
        } else if nivel_alvo < nivel_atual && dias_no_nivel >= 90 {
            nivel_atual - 1
        } else {
            nivel_atual
        }
    } else {
        nivel_alvo
    }
}

/// Determina o nível industrial (0 a 8) com histerese.
fn determinar_nivel_industrial(indice: f64, nivel_atual: u8, dias_no_nivel: u32) -> u8 {
    let nivel_alvo = if indice < 3.0 {
        0
    } else if indice < 14.0 {
        1
    } else if indice < 26.0 {
        2
    } else if indice < 38.0 {
        3
    } else if indice < 52.0 {
        4
    } else if indice < 66.0 {
        5
    } else if indice < 78.0 {
        6
    } else if indice < 90.0 {
        7
    } else {
        8
    };

    if nivel_alvo < nivel_atual {
        let limiar = match nivel_atual {
            1 => 3.0 - 1.2,
            2 => 14.0 - 1.5,
            3 => 26.0 - 1.5,
            4 => 38.0 - 1.5,
            5 => 52.0 - 1.5,
            6 => 66.0 - 1.5,
            7 => 78.0 - 1.5,
            8 => 90.0 - 1.5,
            _ => 0.0,
        };
        if indice >= limiar {
            return nivel_atual;
        }
    }

    if dias_no_nivel > 0 {
        if nivel_alvo > nivel_atual && dias_no_nivel >= 45 {
            nivel_atual + 1
        } else if nivel_alvo < nivel_atual && dias_no_nivel >= 90 {
            nivel_atual - 1
        } else {
            nivel_atual
        }
    } else {
        nivel_alvo
    }
}
