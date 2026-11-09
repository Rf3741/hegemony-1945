//! Motor determinístico de divergência histórica e probabilidade geopolítica (1945-2045).
//!
//! Implementa os requisitos de:
//! 1. História como probabilidade: marcos após ~1960 dependem do estado do mundo (PIB, legitimidade,
//!    estabilidade, guerras, alianças) com janela de datas e chances calibradas por uma semente determinística.
//! 2. Consequências em cadeia: revoluções socialistas / pró-ocidentais alteram leis, esferas, alianças, comércio
//!    e lados em conflitos armados futuros.
//! 3. Condições de vitória e colapso na Guerra Fria: vitória ocidental (dissolução da URSS, reunificação alemã),
//!    vitória soviética (colapso da OTAN, sobrevivência da URSS, expansão do socialismo) ou impasse.
//! 4. Rastreamento de guerras extras e impacto demográfico realista.

use crate::mapa::AlinhamentoGeopolitico;
use crate::mundo::MundoJogo;
use crate::politica::CategoriaLei;
use crate::tempo::DataSimulacao;
use serde::{Deserialize, Serialize};

/// Gerador pseudoaleatório determinístico baseado em Xoshiro256** com expansão SplitMix64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngDeterministico {
    estado: [u64; 4],
}

impl Default for RngDeterministico {
    fn default() -> Self {
        Self::com_semente(19_450_815)
    }
}

impl RngDeterministico {
    pub fn com_semente(semente: u64) -> Self {
        let mut sm = semente;
        let mut proximo_splitmix = || {
            sm = sm.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };

        let mut estado = [
            proximo_splitmix(),
            proximo_splitmix(),
            proximo_splitmix(),
            proximo_splitmix(),
        ];

        if estado.iter().all(|&x| x == 0) {
            estado[0] = 0x853C49E6748FEA9B;
        }

        Self { estado }
    }

    #[inline]
    pub fn proximo_u64(&mut self) -> u64 {
        let resultado = (self.estado[1].wrapping_mul(5)).rotate_left(7).wrapping_mul(9);
        let t = self.estado[1] << 17;

        self.estado[2] ^= self.estado[0];
        self.estado[3] ^= self.estado[1];
        self.estado[1] ^= self.estado[2];
        self.estado[0] ^= self.estado[3];

        self.estado[2] ^= t;
        self.estado[3] = self.estado[3].rotate_left(45);

        resultado
    }

    #[inline]
    pub fn proximo_f64(&mut self) -> f64 {
        (self.proximo_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    #[inline]
    pub fn chance(&mut self, probabilidade: f64) -> bool {
        if probabilidade <= 0.0 {
            return false;
        }
        if probabilidade >= 1.0 {
            return true;
        }
        self.proximo_f64() < probabilidade
    }
}

/// Estado e desfecho da Guerra Fria bipolar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesfechoGuerraFria {
    EmAndamento,
    VitoriaOcidental,
    VitoriaSovietica,
    ImpasseProlongado,
}

impl DesfechoGuerraFria {
    pub fn rotulo(self) -> &'static str {
        match self {
            Self::EmAndamento => "Em andamento",
            Self::VitoriaOcidental => "Vitória Ocidental",
            Self::VitoriaSovietica => "Vitória Soviética",
            Self::ImpasseProlongado => "Impasse Prolongado",
        }
    }
}

/// Registro de um evento de divergência histórico-geopolítica.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventoDivergencia {
    RevolucaoSocialista { tag: String, nome: String, ano: i32 },
    GolpeProOcidental { tag: String, nome: String, ano: i32 },
    VitoriaGuerraFria { vencedor: DesfechoGuerraFria, ano: i32, descricao: String },
    ReunificacaoAlema { ano: i32, sob_rfa: bool },
}

/// Motor de simulação da história como probabilidade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorDivergencia {
    pub semente: u64,
    pub rng: RngDeterministico,
    pub desfecho: DesfechoGuerraFria,
    pub ano_desfecho: Option<i32>,
    pub dissolucao_urss_autorizada: bool,
    pub reunificacao_alema_autorizada: bool,
    pub dissolucao_otan_autorizada: bool,
    pub revolucoes_feitas: Vec<String>,
    pub golpes_feitos: Vec<String>,
    pub noticias_pendentes: Vec<(i32, u8, u8, String)>,
    pub guerras_extras_ativas: Vec<String>,
    pub baixas_civis_acumuladas: u64,
    pub baixas_militares_acumuladas: u64,
    ultimo_ano_avaliado: i32,
}

impl MotorDivergencia {
    pub fn novo(semente: u64) -> Self {
        Self {
            semente,
            rng: RngDeterministico::com_semente(semente),
            desfecho: DesfechoGuerraFria::EmAndamento,
            ano_desfecho: None,
            dissolucao_urss_autorizada: false,
            reunificacao_alema_autorizada: false,
            dissolucao_otan_autorizada: false,
            revolucoes_feitas: Vec::new(),
            golpes_feitos: Vec::new(),
            noticias_pendentes: Vec::new(),
            guerras_extras_ativas: Vec::new(),
            baixas_civis_acumuladas: 0,
            baixas_militares_acumuladas: 0,
            ultimo_ano_avaliado: 1944,
        }
    }

    /// Conta quantos países do mundo estão alinhados ao socialismo / comunismo.
    pub fn contar_paises_socialistas(&self, mundo: &MundoJogo) -> usize {
        mundo
            .paises
            .values()
            .filter(|p| {
                p.alinhamento_inicial == AlinhamentoGeopolitico::BlocoSovietico
                    || p.ideologia.contains("comunista")
                    || p.ideologia.contains("marxista")
                    || p.ideologia.contains("socialista")
                    || p.politica.lei_ativa(CategoriaLei::Governo) == "partido_unico"
            })
            .count()
    }

    /// Conta quantos países do mundo estão alinhados ao bloco ocidental / liberal.
    pub fn contar_paises_ocidentais(&self, mundo: &MundoJogo) -> usize {
        mundo
            .paises
            .values()
            .filter(|p| {
                p.alinhamento_inicial == AlinhamentoGeopolitico::BlocoOcidental
                    || p.ideologia.contains("liberal")
            })
            .count()
    }

    /// Avalia a probabilidade dos grandes marcos conforme a data e o estado do mundo.
    pub fn avaliar_passo(
        &mut self,
        mundo: &mut MundoJogo,
        data: DataSimulacao,
        tensao_mundial: f32,
        influencias: &[(String, (f32, f32))],
    ) -> Vec<EventoDivergencia> {
        let mut eventos = Vec::new();

        // 1. Antes de 1960: preserva estritamente o rumo histórico padrão.
        if data.ano < 1960 {
            return eventos;
        }

        // Avaliação anual principal (apenas uma vez por ano, no início do ano)
        if data.ano == self.ultimo_ano_avaliado {
            return eventos;
        }
        self.ultimo_ano_avaliado = data.ano;

        let ano = data.ano;
        let n_socialistas = self.contar_paises_socialistas(mundo);
        let n_ocidentais = self.contar_paises_ocidentais(mundo);

        // 2. Revoluções e Golpes (1960 a 2010): dependem da estabilidade, influência e semente.
        if ano >= 1960 && ano <= 2010 {
            let candidatos_socialistas = [
                ("CUB", 1960, 0.85),
                ("VNM", 1975, 0.90),
                ("LAO", 1975, 0.75),
                ("KHM", 1975, 0.70),
                ("AGO", 1975, 0.75),
                ("MOZ", 1975, 0.75),
                ("ETH", 1974, 0.70),
                ("NIC", 1979, 0.70),
                ("AFG", 1978, 0.75),
                ("YMD", 1967, 0.65),
                ("CHL", 1970, 0.40),
                ("PER", 1968, 0.35),
                ("BOL", 1970, 0.30),
                ("COG", 1969, 0.60),
                ("BEN", 1972, 0.55),
                ("MDG", 1975, 0.50),
            ];

            for &(tag, ano_janela, base_prob) in &candidatos_socialistas {
                if self.revolucoes_feitas.iter().any(|t| t == tag) {
                    continue;
                }
                if ano >= ano_janela - 2 && ano <= ano_janela + 8 {
                    let inf_sov = influencias
                        .iter()
                        .find(|(t, _)| t == tag)
                        .map(|(_, (_, s))| *s)
                        .unwrap_or(20.0);
                    let mult_inf = (inf_sov / 40.0).clamp(0.6, 1.8) as f64;
                    // Se o bloco soviético estiver forte, probabilidade de revolução aumenta
                    let mult_bloco = if n_socialistas >= 15 { 1.25 } else { 0.95 };
                    let prob = (base_prob * mult_inf * mult_bloco).clamp(0.15, 0.95);

                    if self.rng.chance(prob) {
                        if self.aplicar_revolucao_socialista(mundo, tag, data) {
                            let nome_pais = mundo.paises.get(tag).map_or(tag.to_string(), |p| p.nome.clone());
                            eventos.push(EventoDivergencia::RevolucaoSocialista {
                                tag: tag.to_string(),
                                nome: nome_pais,
                                ano,
                            });
                        }
                    }
                }
            }

            // Golpes pró-ocidentais em nações vulneráveis
            let candidatos_golpes = [
                ("BRA", 1964, 0.85),
                ("IDN", 1965, 0.85),
                ("GRC", 1967, 0.75),
                ("CHL", 1973, 0.75),
                ("ARG", 1976, 0.80),
                ("GHA", 1966, 0.65),
            ];

            for &(tag, ano_janela, base_prob) in &candidatos_golpes {
                if self.golpes_feitos.iter().any(|t| t == tag) {
                    continue;
                }
                if ano >= ano_janela - 2 && ano <= ano_janela + 4 {
                    let inf_eua = influencias
                        .iter()
                        .find(|(t, _)| t == tag)
                        .map(|(_, (u, _))| *u)
                        .unwrap_or(30.0);
                    let mult_inf = (inf_eua / 40.0).clamp(0.6, 1.6) as f64;
                    let prob = (base_prob * mult_inf).clamp(0.20, 0.95);

                    if self.rng.chance(prob) {
                        if self.aplicar_golpe_pro_ocidente(mundo, tag, data) {
                            let nome_pais = mundo.paises.get(tag).map_or(tag.to_string(), |p| p.nome.clone());
                            eventos.push(EventoDivergencia::GolpeProOcidental {
                                tag: tag.to_string(),
                                nome: nome_pais,
                                ano,
                            });
                        }
                    }
                }
            }
        }

        // 3. Fim da Guerra Fria e Vitória de Bloco (Janela 1985 a 2005)
        if self.desfecho == DesfechoGuerraFria::EmAndamento && ano >= 1985 {
            let pib_sov = mundo.paises.get("SOV").map_or(500e9, |p| p.pib_anual_estimado);
            let pib_usa = mundo.paises.get("USA").map_or(2000e9, |p| p.pib_anual_estimado);
            let leg_sov = mundo.paises.get("SOV").map_or(45.0, |p| p.politica.legitimidade);
            let leg_usa = mundo.paises.get("USA").map_or(70.0, |p| p.politica.legitimidade);
            let razao_eco = (pib_sov / pib_usa.max(1.0)).clamp(0.1, 2.0);

            // Condição de Vitória Soviética vs Ocidental:
            // Pondera a força relativa dos blocos pelo número de aliados, legitimidade e economia.
            let peso_sov = (n_socialistas as f64 * 3.0) + (leg_sov as f64 * 0.8) + (razao_eco * 30.0) + 10.0;
            let peso_eua = (n_ocidentais as f64 * 3.0) + (leg_usa as f64 * 0.8) + 40.0;
            let total_peso = (peso_sov + peso_eua).max(1.0);

            let prob_vitoria_sov = (peso_sov / total_peso) * 0.35;
            let prob_vitoria_eua = if ano >= 1991 {
                ((peso_eua / total_peso) * 0.70 + 0.15).min(0.85)
            } else {
                (peso_eua / total_peso) * 0.50
            };

            // Chance de Vitória Soviética vs Ocidental
            if ano >= 1988 {
                let roll = self.rng.proximo_f64();
                if roll < prob_vitoria_sov {
                    self.aplicar_vitoria_sovietica(mundo, data);
                    eventos.push(EventoDivergencia::VitoriaGuerraFria {
                        vencedor: DesfechoGuerraFria::VitoriaSovietica,
                        ano,
                        descricao: "VITÓRIA SOVIÉTICA NA GUERRA FRIA: Colapso da hegemonia ocidental e dissolução da OTAN. A União Soviética lidera a nova ordem mundial socialista.".to_string(),
                    });
                } else if roll < prob_vitoria_sov + prob_vitoria_eua {
                    self.aplicar_vitoria_ocidental(mundo, data);
                    eventos.push(EventoDivergencia::VitoriaGuerraFria {
                        vencedor: DesfechoGuerraFria::VitoriaOcidental,
                        ano,
                        descricao: "VITÓRIA OCIDENTAL NA GUERRA FRIA: Dissolução da União Soviética e fim do Pacto de Varsóvia. A democracia liberal e a economia de mercado triunfam.".to_string(),
                    });
                    eventos.push(EventoDivergencia::ReunificacaoAlema { ano, sob_rfa: true });
                }
            }
        }

        // Se chegou em 2005 sem vencedor declarado, marca como Impasse Prolongado
        if self.desfecho == DesfechoGuerraFria::EmAndamento && ano >= 2005 {
            self.desfecho = DesfechoGuerraFria::ImpasseProlongado;
            self.ano_desfecho = Some(ano);
            let desc = "IMPASSE DA GUERRA FRIA: Nenhuma superpotência capitulou; rivalidade bipolar prossegue no século XXI.".to_string();
            self.noticias_pendentes.push((ano, data.mes, data.dia, desc.clone()));
            eventos.push(EventoDivergencia::VitoriaGuerraFria {
                vencedor: DesfechoGuerraFria::ImpasseProlongado,
                ano,
                descricao: desc,
            });
        }

        let _ = tensao_mundial;
        eventos
    }

    /// Executa uma revolução socialista em um país, alterando leis, mercado, ideologia e alinhamento.
    pub fn aplicar_revolucao_socialista(&mut self, mundo: &mut MundoJogo, tag: &str, data: DataSimulacao) -> bool {
        let Some(pais) = mundo.paises.get_mut(tag) else { return false };
        if self.revolucoes_feitas.iter().any(|t| t == tag) {
            return false;
        }
        self.revolucoes_feitas.push(tag.to_string());

        pais.ideologia = "comunismo_marxista_leninista".to_string();
        pais.alinhamento_inicial = AlinhamentoGeopolitico::BlocoSovietico;
        pais.mercado_id = "MKT_SOV".to_string();

        pais.politica.leis_ativas.insert(CategoriaLei::Governo, "partido_unico".to_string());
        pais.politica.leis_ativas.insert(CategoriaLei::Economia, "economia_planificada".to_string());
        pais.politica.leis_ativas.insert(CategoriaLei::DireitosCivis, "estado_policial".to_string());
        pais.politica.leis_ativas.insert(CategoriaLei::Trabalho, "trabalhismo_estatal".to_string());
        pais.politica.leis_ativas.insert(CategoriaLei::SistemaTributario, "requisicao_estatal".to_string());
        pais.politica.leis_ativas.insert(CategoriaLei::Exercito, "alistamento_obrigatorio".to_string());
        pais.politica.influencia_bipolar = 85.0;
        pais.politica.legitimidade = 65.0;

        let msg = format!("Revolução Socialista em {}: novo regime popular assume o poder e alinha-se a Moscou.", pais.nome);
        self.noticias_pendentes.push((data.ano, data.mes, data.dia, msg));
        true
    }

    /// Executa um golpe pró-ocidente ou restauração liberal.
    pub fn aplicar_golpe_pro_ocidente(&mut self, mundo: &mut MundoJogo, tag: &str, data: DataSimulacao) -> bool {
        let Some(pais) = mundo.paises.get_mut(tag) else { return false };
        if self.golpes_feitos.iter().any(|t| t == tag) {
            return false;
        }
        self.golpes_feitos.push(tag.to_string());

        pais.ideologia = "democracia_liberal".to_string();
        pais.alinhamento_inicial = AlinhamentoGeopolitico::BlocoOcidental;
        pais.mercado_id = "MKT_USA".to_string();

        pais.politica.leis_ativas.insert(CategoriaLei::Governo, "sufragio_universal".to_string());
        pais.politica.leis_ativas.insert(CategoriaLei::Economia, "capitalismo_de_mercado".to_string());
        pais.politica.leis_ativas.insert(CategoriaLei::DireitosCivis, "liberdade_de_expressao".to_string());
        pais.politica.leis_ativas.insert(CategoriaLei::Trabalho, "regulamentacao_basica".to_string());
        pais.politica.leis_ativas.insert(CategoriaLei::SistemaTributario, "tributacao_progressiva".to_string());
        pais.politica.influencia_bipolar = -85.0;
        pais.politica.legitimidade = 60.0;

        let msg = format!("Golpe e Transição Liberal em {}: governo alinha-se aos Estados Unidos e adota economia de mercado.", pais.nome);
        self.noticias_pendentes.push((data.ano, data.mes, data.dia, msg));
        true
    }

    /// Aplica a vitória do bloco ocidental: autoriza a dissolução da URSS e a reunificação alemã.
    pub fn aplicar_vitoria_ocidental(&mut self, mundo: &mut MundoJogo, data: DataSimulacao) {
        self.desfecho = DesfechoGuerraFria::VitoriaOcidental;
        self.ano_desfecho = Some(data.ano);
        self.dissolucao_urss_autorizada = true;
        self.reunificacao_alema_autorizada = true;

        if let Some(sov) = mundo.paises.get_mut("SOV") {
            sov.ideologia = "democracia_liberal".to_string();
            sov.alinhamento_inicial = AlinhamentoGeopolitico::NaoAlinhado;
            sov.politica.leis_ativas.insert(CategoriaLei::Governo, "sufragio_universal".to_string());
            sov.politica.leis_ativas.insert(CategoriaLei::Economia, "capitalismo_de_intervencao".to_string());
        }

        let msg = "FIM DA GUERRA FRIA: Dissolução da União Soviética. A Guerra Fria termina com vitória e hegemonia do bloco ocidental.".to_string();
        self.noticias_pendentes.push((data.ano, data.mes, data.dia, msg));
    }

    /// Aplica a vitória do bloco soviético: a URSS sobrevive, a OTAN é dissolvida e o socialismo se consolida.
    pub fn aplicar_vitoria_sovietica(&mut self, mundo: &mut MundoJogo, data: DataSimulacao) {
        self.desfecho = DesfechoGuerraFria::VitoriaSovietica;
        self.ano_desfecho = Some(data.ano);
        self.dissolucao_urss_autorizada = false; // URSS NÃO DISSOLVE
        self.reunificacao_alema_autorizada = false; // RDA se mantém ou Alemanha é socialista
        self.dissolucao_otan_autorizada = true;

        if let Some(sov) = mundo.paises.get_mut("SOV") {
            sov.politica.legitimidade = 85.0;
            sov.pib_anual_estimado *= 1.25;
        }
        if let Some(usa) = mundo.paises.get_mut("USA") {
            usa.politica.legitimidade = 35.0;
            usa.alinhamento_inicial = AlinhamentoGeopolitico::NaoAlinhado;
        }

        let msg = "FIM DA GUERRA FRIA: Triunfo global da União Soviética. A OTAN e a hegemonia ocidental colapsam; o modelo socialista vence a Guerra Fria.".to_string();
        self.noticias_pendentes.push((data.ano, data.mes, data.dia, msg));
    }

    /// Registra baixas de guerras extras e deduz diretamente das populações beligerantes.
    pub fn registrar_baixas_guerra_extra(
        &mut self,
        mundo: &mut MundoJogo,
        tag_atacante: &str,
        tag_defensor: &str,
        baixas_atk: u32,
        baixas_def: u32,
    ) {
        self.baixas_militares_acumuladas += (baixas_atk + baixas_def) as u64;
        let civis = ((baixas_atk + baixas_def) as f64 * 1.5) as u64;
        self.baixas_civis_acumuladas += civis;

        // Deduz da população do atacante em MundoJogo
        if let Some(pais_atk) = mundo.paises.get_mut(tag_atacante) {
            let total_atk = pais_atk.populacao_total();
            let perda_atk = (baixas_atk as u64 + civis / 2).min(total_atk.saturating_sub(10_000));
            if total_atk > 0 && perda_atk > 0 {
                let frac = perda_atk as f64 / total_atk as f64;
                for pop in &mut pais_atk.pops {
                    pop.ativos = (pop.ativos as f64 * (1.0 - frac)).round() as u64;
                    pop.dependentes = (pop.dependentes as f64 * (1.0 - frac)).round() as u64;
                }
            }
        }

        // Deduz da população do defensor em MundoJogo
        if let Some(pais_def) = mundo.paises.get_mut(tag_defensor) {
            let total_def = pais_def.populacao_total();
            let perda_def = (baixas_def as u64 + civis / 2).min(total_def.saturating_sub(10_000));
            if total_def > 0 && perda_def > 0 {
                let frac = perda_def as f64 / total_def as f64;
                for pop in &mut pais_def.pops {
                    pop.ativos = (pop.ativos as f64 * (1.0 - frac)).round() as u64;
                    pop.dependentes = (pop.dependentes as f64 * (1.0 - frac)).round() as u64;
                }
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::mundo::EstadoPais;
    use crate::politica::construir_politica_inicial;
    use std::collections::BTreeMap;

    fn mundo_teste() -> MundoJogo {
        let mut paises = BTreeMap::new();
        let leis = BTreeMap::new();

        let criar = |tag: &str, nome: &str, pib: f64, alin: AlinhamentoGeopolitico, ideo: &str| {
            let pol = construir_politica_inicial(tag, &leis, 0.0);
            EstadoPais {
                tag: tag.to_string(),
                nome: nome.to_string(),
                nome_cenario: Some(nome.to_string()),
                cor: (100, 100, 100),
                capital: "Capital".to_string(),
                suserano: None,
                alinhamento_inicial: alin,
                ideologia: ideo.to_string(),
                mercado_id: format!("MKT_{tag}"),
                provincias_ids: vec![1, 2],
                pops: vec![crate::economia::Pop {
                    provincia_id: 1,
                    profissao: crate::economia::Profissao::Camponeses,
                    cultura: "padrao".to_string(),
                    religiao: "padrao".to_string(),
                    ativos: 10_000_000,
                    dependentes: 15_000_000,
                    padrao_vida: 12.0,
                    alfabetizacao: 0.7,
                    riqueza: 300.0,
                    radicais: 500_000,
                    leais: 1_000_000,
                }],
                predios: Vec::new(),
                orcamento: crate::economia::OrcamentoNacional::default(),
                politica: pol,
                pib_anual_estimado: pib,
            }
        };

        paises.insert("USA".to_string(), criar("USA", "Estados Unidos", 2500e9, AlinhamentoGeopolitico::BlocoOcidental, "democracia_liberal"));
        paises.insert("SOV".to_string(), criar("SOV", "União Soviética", 800e9, AlinhamentoGeopolitico::BlocoSovietico, "comunismo_marxista_leninista"));
        paises.insert("CUB".to_string(), criar("CUB", "Cuba", 10e9, AlinhamentoGeopolitico::NaoAlinhado, "autoritarismo_direita"));
        paises.insert("BRA".to_string(), criar("BRA", "Brasil", 80e9, AlinhamentoGeopolitico::NaoAlinhado, "nacionalismo_trabalhista"));

        MundoJogo {
            data: DataSimulacao::nova(1945, 1, 1),
            tensao_mundial: 20.0,
            paises,
            mercados: BTreeMap::new(),
            dono_por_provincia: std::collections::HashMap::new(),
            controlador_por_provincia: std::collections::HashMap::new(),
            populacao_por_provincia: std::collections::HashMap::new(),
            ticks_executados: 0,
        }
    }

    #[test]
    fn rng_deterministico_mesma_semente_mesma_sequencia() {
        let mut rng1 = RngDeterministico::com_semente(19450815);
        let mut rng2 = RngDeterministico::com_semente(19450815);
        for _ in 0..50 {
            assert_eq!(rng1.proximo_u64(), rng2.proximo_u64());
        }
    }

    #[test]
    fn antes_de_1960_mantem_caminho_sem_divergencias() {
        let mut motor = MotorDivergencia::novo(19450815);
        let mut mundo = mundo_teste();
        let evs = motor.avaliar_passo(&mut mundo, DataSimulacao::nova(1955, 6, 1), 25.0, &[]);
        assert!(evs.is_empty());
        assert_eq!(motor.desfecho, DesfechoGuerraFria::EmAndamento);
    }

    #[test]
    fn revolucao_socialista_muda_leis_mercado_e_ideologia() {
        let mut motor = MotorDivergencia::novo(19450815);
        let mut mundo = mundo_teste();
        let ok = motor.aplicar_revolucao_socialista(&mut mundo, "CUB", DataSimulacao::nova(1960, 1, 1));
        assert!(ok);
        let cub = mundo.paises.get("CUB").unwrap();
        assert_eq!(cub.ideologia, "comunismo_marxista_leninista");
        assert_eq!(cub.alinhamento_inicial, AlinhamentoGeopolitico::BlocoSovietico);
        assert_eq!(cub.mercado_id, "MKT_SOV");
        assert_eq!(cub.politica.lei_ativa(CategoriaLei::Governo), "partido_unico");
        assert_eq!(cub.politica.lei_ativa(CategoriaLei::Economia), "economia_planificada");
    }

    #[test]
    fn vitoria_ocidental_autoriza_dissolucao_urss() {
        let mut motor = MotorDivergencia::novo(19450815);
        let mut mundo = mundo_teste();
        motor.aplicar_vitoria_ocidental(&mut mundo, DataSimulacao::nova(1991, 12, 25));
        assert_eq!(motor.desfecho, DesfechoGuerraFria::VitoriaOcidental);
        assert!(motor.dissolucao_urss_autorizada);
        assert!(motor.reunificacao_alema_autorizada);
    }

    #[test]
    fn vitoria_sovietica_preserva_urss_e_dissolve_otan() {
        let mut motor = MotorDivergencia::novo(19171107);
        let mut mundo = mundo_teste();
        motor.aplicar_vitoria_sovietica(&mut mundo, DataSimulacao::nova(1990, 5, 1));
        assert_eq!(motor.desfecho, DesfechoGuerraFria::VitoriaSovietica);
        assert!(!motor.dissolucao_urss_autorizada);
        assert!(motor.dissolucao_otan_autorizada);
    }

    #[test]
    fn guerra_extra_reduz_populacao_realista() {
        let mut motor = MotorDivergencia::novo(19450815);
        let mut mundo = mundo_teste();
        let pop_inicial_usa = mundo.paises.get("USA").unwrap().populacao_total();
        let pop_inicial_sov = mundo.paises.get("SOV").unwrap().populacao_total();

        motor.registrar_baixas_guerra_extra(&mut mundo, "USA", "SOV", 500_000, 750_000);

        let pop_pos_usa = mundo.paises.get("USA").unwrap().populacao_total();
        let pop_pos_sov = mundo.paises.get("SOV").unwrap().populacao_total();

        assert!(pop_pos_usa < pop_inicial_usa);
        assert!(pop_pos_sov < pop_inicial_sov);
        assert!(pop_inicial_usa - pop_pos_usa >= 500_000);
        assert!(pop_inicial_sov - pop_pos_sov >= 750_000);
    }

    #[test]
    fn desfecho_divergente_com_sementes_diferentes() {
        let simular_ate_1995 = |semente: u64| -> DesfechoGuerraFria {
            let mut motor = MotorDivergencia::novo(semente);
            let mut mundo = mundo_teste();
            for ano in 1960..=1995 {
                motor.avaliar_passo(&mut mundo, DataSimulacao::nova(ano, 6, 1), 50.0, &[]);
            }
            motor.desfecho
        };

        let d_ocidental = simular_ate_1995(19171107);
        let d_sovietico = simular_ate_1995(19530305);

        assert_eq!(d_ocidental, DesfechoGuerraFria::VitoriaOcidental);
        assert_eq!(d_sovietico, DesfechoGuerraFria::VitoriaSovietica);
    }

    #[test]
    fn desfecho_divergente_cenario_real() {
        use std::path::PathBuf;
        let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (_, mundo_base) = MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenario");

        let simular = |semente: u64| -> DesfechoGuerraFria {
            let mut motor = MotorDivergencia::novo(semente);
            let mut mundo = mundo_base.clone();
            for ano in 1960..=1995 {
                motor.avaliar_passo(&mut mundo, DataSimulacao::nova(ano, 6, 1), 35.0, &[]);
            }
            motor.desfecho
        };

        assert_eq!(simular(19621027), DesfechoGuerraFria::VitoriaOcidental);
        assert_eq!(simular(19891109), DesfechoGuerraFria::VitoriaSovietica);
    }
}
