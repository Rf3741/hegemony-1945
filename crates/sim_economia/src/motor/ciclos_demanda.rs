//! Ciclos históricos de demanda, substituição tecnológica, estocagem estratégica e choques
//! de oferta/demanda de commodities ao longo da Guerra Fria (1945–1991+).
//!
//! Todos os efeitos atuam exclusivamente sobre quantidades demandadas, pesos de substituição
//! das famílias e absorção/oferta residual do comércio exterior — **sem jamais fixar preços
//! manualmente**. Os preços continuam sendo formados 100% por `fechar_mercados_e_atualizar_precos`.

use crate::catalogo::CatalogoEconomia;
use crate::entidades::{Pais, Provincia};
use crate::erro::{ErroEconomia, ResultadoEconomia, validar_id_snake_case};
use crate::tipos::DataJogo;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const SEMENTE_CICLOS_PADRAO: u64 = 0x4845_4745_3139_3435;

const fn semente_padrao() -> u64 {
    SEMENTE_CICLOS_PADRAO
}

const fn padrao_true() -> bool {
    true
}

const fn padrao_um() -> f64 {
    1.0
}

/// Ponto de ancoragem histórica da curva secular de um bem (`ciclos_demanda.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PontoEraDemanda {
    /// Ano decimal de referência (ex.: `1945.0`, `1954.5`, `1973.8`).
    pub ano: f64,
    /// Multiplicador estrutural sobre a demanda industrial/produtiva do bem (`1.0` = base 1945).
    #[serde(default = "padrao_um")]
    pub fator_demanda: f64,
    /// Multiplicador sobre o peso de preferência das famílias nas cestas de substituição (`1.0` = base).
    #[serde(default = "padrao_um")]
    pub fator_peso_pop: f64,
    /// Procura estratégica/governamental mundial expressa em múltiplos da capacidade instalada de 1945.
    #[serde(default)]
    pub fator_procura_estrategica: f64,
    /// Fração de absorção de excedentes pelo resto do mundo (`1.0` = apetite pleno, `< 0.25` = glut global).
    #[serde(default = "padrao_um")]
    pub fator_absorcao_excedente: f64,
}

/// Efeito cumulativo da difusão mundial de uma tecnologia sobre a demanda de um bem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatilhoTecnologiaDemanda {
    pub tecnologia_id: String,
    #[serde(default)]
    pub delta_demanda: f64,
    #[serde(default)]
    pub delta_peso_pop: f64,
    #[serde(default)]
    pub delta_procura_estrategica: f64,
    #[serde(default)]
    pub delta_absorcao_excedente: f64,
}

/// Choque transitório de oferta/demanda associado a um evento histórico (ex.: OPEP 1973, Chernobyl 1986).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoqueEventoDemanda {
    pub evento_id: String,
    /// Ano decimal histórico de referência para gatilho autônomo (com desvio por semente) caso o motor
    /// externo de eventos não esteja acoplado.
    pub ano_referencia: f64,
    /// Duração total do choque em dias (decai suavemente na metade final).
    pub duracao_dias: u32,
    #[serde(default = "padrao_um")]
    pub multiplicador_demanda: f64,
    #[serde(default = "padrao_um")]
    pub multiplicador_procura_estrategica: f64,
    #[serde(default = "padrao_um")]
    pub multiplicador_absorcao_excedente: f64,
    /// Restrição de oferta residual externa durante embargos/crises (`< 1.0` aperta importações residuais).
    #[serde(default = "padrao_um")]
    pub multiplicador_oferta_importacao_residual: f64,
}

/// Definição completa da dinâmica temporal de demanda de um `Bem` (`ciclos_demanda.ron`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvaDemandaBem {
    pub bem_id: String,
    #[serde(default)]
    pub referencia_historica: String,
    /// Amplitude máxima (em anos, ±) de deslocamento dos pontos de inflexão pós-1946 por semente.
    #[serde(default)]
    pub amplitude_desvio_anos: f64,
    /// Amplitude máxima (fração, ±) da intensidade estrutural sorteada por partida.
    #[serde(default)]
    pub amplitude_variacao_partida: f64,
    /// Amplitude máxima (fração, ±) de oscilação cíclica de médio prazo (estoques/conjuntura).
    #[serde(default)]
    pub amplitude_ciclo_curto: f64,
    pub pontos_era: Vec<PontoEraDemanda>,
    #[serde(default)]
    pub gatilhos_tecnologia: Vec<GatilhoTecnologiaDemanda>,
    #[serde(default)]
    pub choques_eventos: Vec<ChoqueEventoDemanda>,

    #[serde(skip)]
    pub(crate) bem_idx: usize,
}

/// Tabela estática carregada de `assets/data/economia/ciclos_demanda.ron`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TabelaCiclosDemanda {
    #[serde(default)]
    pub curvas: Vec<CurvaDemandaBem>,
}

impl TabelaCiclosDemanda {
    /// Valida todos os IDs, intervalos numéricos e ordenação temporal dos pontos de era.
    pub fn validar_e_indexar(
        &mut self,
        indice_bens: &BTreeMap<String, usize>,
    ) -> ResultadoEconomia<()> {
        let mut vistos = BTreeMap::new();
        for (i, curva) in self.curvas.iter_mut().enumerate() {
            validar_id_snake_case("CurvaDemandaBem.bem_id", &curva.bem_id)?;
            let &bem_idx = indice_bens.get(&curva.bem_id).ok_or_else(|| {
                ErroEconomia::ReferenciaInexistente {
                    origem_entidade: "CurvaDemandaBem",
                    origem_id: curva.bem_id.clone(),
                    alvo_entidade: "Bem",
                    alvo_id: curva.bem_id.clone(),
                }
            })?;
            if vistos.insert(curva.bem_id.clone(), i).is_some() {
                return Err(ErroEconomia::IdentificadorDuplicado {
                    entidade: "CurvaDemandaBem",
                    id: curva.bem_id.clone(),
                });
            }
            curva.bem_idx = bem_idx;

            if curva.pontos_era.is_empty() {
                return Err(ErroEconomia::ValorInvalido {
                    entidade: "CurvaDemandaBem",
                    id: curva.bem_id.clone(),
                    detalhe: "pontos_era não pode ser vazio".into(),
                });
            }

            let mut ultimo_ano = f64::NEG_INFINITY;
            for p in &curva.pontos_era {
                if !p.ano.is_finite()
                    || !p.fator_demanda.is_finite()
                    || !p.fator_peso_pop.is_finite()
                    || !p.fator_procura_estrategica.is_finite()
                    || !p.fator_absorcao_excedente.is_finite()
                    || p.fator_demanda <= 0.0
                    || p.fator_peso_pop <= 0.0
                    || p.fator_procura_estrategica < 0.0
                    || p.fator_absorcao_excedente < 0.0
                {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "PontoEraDemanda",
                        id: curva.bem_id.clone(),
                        detalhe: "valores numéricos inválidos em pontos_era".into(),
                    });
                }
                if p.ano < ultimo_ano {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "PontoEraDemanda",
                        id: curva.bem_id.clone(),
                        detalhe: "pontos_era deve estar ordenado por ano crescente".into(),
                    });
                }
                ultimo_ano = p.ano;
            }

            for gt in &curva.gatilhos_tecnologia {
                validar_id_snake_case("GatilhoTecnologiaDemanda", &gt.tecnologia_id)?;
            }
            for ch in &curva.choques_eventos {
                validar_id_snake_case("ChoqueEventoDemanda", &ch.evento_id)?;
                if ch.duracao_dias == 0 || !ch.ano_referencia.is_finite() {
                    return Err(ErroEconomia::ValorInvalido {
                        entidade: "ChoqueEventoDemanda",
                        id: ch.evento_id.clone(),
                        detalhe: "duracao_dias e ano_referencia devem ser válidos".into(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Instância ativa de um choque histórico em curso na partida.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoqueEventoAtivo {
    pub evento_id: String,
    pub bem_id: String,
    pub dias_restantes: u32,
    pub duracao_total: u32,
    pub multiplicador_demanda: f64,
    pub multiplicador_procura_estrategica: f64,
    pub multiplicador_absorcao_excedente: f64,
    pub multiplicador_oferta_importacao_residual: f64,
}

/// Estado dinâmico serializável dos ciclos de demanda de uma partida.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoCiclosDemanda {
    #[serde(default = "semente_padrao")]
    pub semente_partida: u64,
    #[serde(default = "padrao_true")]
    pub usar_gatilhos_historicos_autonomos: bool,
    /// Capacidade física nominal mundial instalada em 1945 por bem (fixada no início do cenário).
    #[serde(default)]
    pub oferta_nominal_inicial_bem: Vec<f64>,
    /// Capacidade física nominal instalada em 1945 por `(mercado, bem)` (`m * num_bens + g`).
    #[serde(default)]
    pub oferta_nominal_inicial_mercado_bem: Vec<f64>,
    /// Tecnologias pesquisadas registradas no motor (`tecnologia_id -> lista ordenada de pais_id`).
    #[serde(default)]
    pub tecnologias_pesquisadas: BTreeMap<String, Vec<String>>,
    /// Eventos históricos já disparados (`evento_id -> dias_decorridos`).
    #[serde(default)]
    pub eventos_disparados: BTreeMap<String, u64>,
    /// Choques históricos atualmente em vigor.
    #[serde(default)]
    pub choques_ativos: Vec<ChoqueEventoAtivo>,

    #[serde(skip)]
    pub(crate) fator_demanda_bem: Vec<f64>,
    #[serde(skip)]
    pub(crate) fator_peso_pop_bem: Vec<f64>,
    #[serde(skip)]
    pub(crate) fator_procura_estrategica_bem: Vec<f64>,
    #[serde(skip)]
    pub(crate) fator_absorcao_excedente_bem: Vec<f64>,
    #[serde(skip)]
    pub(crate) fator_oferta_importacao_bem: Vec<f64>,
    #[serde(skip)]
    pub(crate) fator_choque_demanda_bem: Vec<f64>,
    #[serde(skip)]
    pub(crate) peso_militar_industrial_mercado: Vec<f64>,
}

impl Default for EstadoCiclosDemanda {
    fn default() -> Self {
        Self {
            semente_partida: SEMENTE_CICLOS_PADRAO,
            usar_gatilhos_historicos_autonomos: true,
            oferta_nominal_inicial_bem: Vec::new(),
            oferta_nominal_inicial_mercado_bem: Vec::new(),
            tecnologias_pesquisadas: BTreeMap::new(),
            eventos_disparados: BTreeMap::new(),
            choques_ativos: Vec::new(),
            fator_demanda_bem: Vec::new(),
            fator_peso_pop_bem: Vec::new(),
            fator_procura_estrategica_bem: Vec::new(),
            fator_absorcao_excedente_bem: Vec::new(),
            fator_oferta_importacao_bem: Vec::new(),
            fator_choque_demanda_bem: Vec::new(),
            peso_militar_industrial_mercado: Vec::new(),
        }
    }
}

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[inline]
fn hash_str_u64(s: &str) -> u64 {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in s.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01B3);
    }
    h
}

/// Converte um `u64` determinístico em um `f64` uniforme em `[-1.0, +1.0]`.
#[inline]
fn ruido_simetrico(semente: u64, chave: &str, sal: u64) -> f64 {
    let h = splitmix64(semente ^ hash_str_u64(chave) ^ sal);
    let u01 = ((h >> 11) as f64) * (1.0 / ((1u64 << 53) as f64));
    u01 * 2.0 - 1.0
}

#[inline]
pub(crate) fn ano_decimal(data: DataJogo) -> f64 {
    let dias_antes_mes = [0u32, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let mes_idx = (data.mes.clamp(1, 12) - 1) as usize;
    let dia_do_ano = dias_antes_mes[mes_idx] + (data.dia.saturating_sub(1) as u32);
    (data.ano as f64) + (dia_do_ano as f64) / 365.0
}

fn interpolar_pontos(pontos: &[PontoEraDemanda], ano_efetivo: f64) -> (f64, f64, f64, f64) {
    let primeiro = &pontos[0];
    if ano_efetivo <= primeiro.ano {
        return (
            primeiro.fator_demanda,
            primeiro.fator_peso_pop,
            primeiro.fator_procura_estrategica,
            primeiro.fator_absorcao_excedente,
        );
    }
    let ultimo = &pontos[pontos.len() - 1];
    if ano_efetivo >= ultimo.ano {
        return (
            ultimo.fator_demanda,
            ultimo.fator_peso_pop,
            ultimo.fator_procura_estrategica,
            ultimo.fator_absorcao_excedente,
        );
    }
    for par in pontos.windows(2) {
        let a = &par[0];
        let b = &par[1];
        if ano_efetivo >= a.ano && ano_efetivo <= b.ano {
            let span = (b.ano - a.ano).max(1e-6);
            let t = ((ano_efetivo - a.ano) / span).clamp(0.0, 1.0);
            // Interpolação suave (smoothstep) evita quinas artificiais nos preços
            let s = t * t * (3.0 - 2.0 * t);
            return (
                a.fator_demanda + s * (b.fator_demanda - a.fator_demanda),
                a.fator_peso_pop + s * (b.fator_peso_pop - a.fator_peso_pop),
                a.fator_procura_estrategica
                    + s * (b.fator_procura_estrategica - a.fator_procura_estrategica),
                a.fator_absorcao_excedente
                    + s * (b.fator_absorcao_excedente - a.fator_absorcao_excedente),
            );
        }
    }
    (
        ultimo.fator_demanda,
        ultimo.fator_peso_pop,
        ultimo.fator_procura_estrategica,
        ultimo.fator_absorcao_excedente,
    )
}

impl EstadoCiclosDemanda {
    /// Inicializa os buffers e captura a capacidade nominal instalada do cenário inicial (1945).
    pub fn inicializar_do_cenario(
        &mut self,
        catalogo: &CatalogoEconomia,
        num_mercados: usize,
        provincias: &[Provincia],
    ) {
        let num_bens = catalogo.bens.len();
        self.garantir_buffers_volateis(num_bens, num_mercados);

        if self.oferta_nominal_inicial_bem.len() != num_bens
            || self.oferta_nominal_inicial_mercado_bem.len() != num_mercados * num_bens
        {
            self.oferta_nominal_inicial_bem = vec![0.0; num_bens];
            self.oferta_nominal_inicial_mercado_bem = vec![0.0; num_mercados * num_bens];
            for prov in provincias {
                let m = prov.mercado_idx;
                if m >= num_mercados {
                    continue;
                }
                for b in &prov.predios {
                    if b.nivel == 0 {
                        continue;
                    }
                    let nivel = b.nivel as f64;
                    for &(g_u16, qtd_por_nivel) in b.cache.produtos() {
                        let g = g_u16 as usize;
                        if g < num_bens && qtd_por_nivel > 0.0 {
                            let cap = nivel * qtd_por_nivel;
                            self.oferta_nominal_inicial_bem[g] += cap;
                            self.oferta_nominal_inicial_mercado_bem[m * num_bens + g] += cap;
                        }
                    }
                }
            }
        }
    }

    /// Redimensiona vetores quando um novo país/mercado dinâmico é formado na partida.
    pub fn redimensionar_mercados(&mut self, num_bens: usize, num_mercados: usize) {
        self.oferta_nominal_inicial_mercado_bem
            .resize(num_mercados * num_bens, 0.0);
        self.garantir_buffers_volateis(num_bens, num_mercados);
    }

    pub(crate) fn garantir_buffers_volateis(&mut self, num_bens: usize, num_mercados: usize) {
        if self.fator_demanda_bem.len() != num_bens {
            self.fator_demanda_bem = vec![1.0; num_bens];
            self.fator_peso_pop_bem = vec![1.0; num_bens];
            self.fator_procura_estrategica_bem = vec![0.0; num_bens];
            self.fator_absorcao_excedente_bem = vec![1.0; num_bens];
            self.fator_oferta_importacao_bem = vec![1.0; num_bens];
            self.fator_choque_demanda_bem = vec![1.0; num_bens];
        }
        if self.peso_militar_industrial_mercado.len() != num_mercados {
            self.peso_militar_industrial_mercado = vec![0.0; num_mercados];
        }
    }

    /// Registra que `pais_id` concluiu a pesquisa de `tecnologia_id`. Retorna `true` se inédito para o país.
    pub fn registrar_tecnologia(&mut self, pais_id: &str, tecnologia_id: &str) -> bool {
        let lista = self
            .tecnologias_pesquisadas
            .entry(tecnologia_id.to_string())
            .or_default();
        if let Err(pos) = lista.binary_search_by(|p| p.as_str().cmp(pais_id)) {
            lista.insert(pos, pais_id.to_string());
            true
        } else {
            false
        }
    }

    /// Dispara os choques econômicos associados a `evento_id` (deduplicado por partida).
    pub fn disparar_evento(
        &mut self,
        catalogo: &CatalogoEconomia,
        evento_id: &str,
        dias_decorridos: u64,
    ) -> bool {
        if self.eventos_disparados.contains_key(evento_id) {
            return false;
        }
        let mut disparou_algum = false;
        for curva in &catalogo.ciclos_demanda.curvas {
            for ch in &curva.choques_eventos {
                if ch.evento_id == evento_id {
                    // Variação leve (±12%) na severidade e duração do choque conforme a semente
                    let r_int = ruido_simetrico(self.semente_partida, evento_id, 0xA11CE);
                    let r_dur = ruido_simetrico(self.semente_partida, evento_id, 0xB22DF);
                    let escala_var = 1.0 + 0.12 * r_int;
                    let ajustar_mult = |m: f64| -> f64 {
                        if (m - 1.0).abs() < 1e-6 {
                            1.0
                        } else {
                            (1.0 + (m - 1.0) * escala_var).clamp(0.08, 3.0)
                        }
                    };
                    let duracao = ((ch.duracao_dias as f64) * (1.0 + 0.12 * r_dur))
                        .round()
                        .max(30.0) as u32;

                    self.choques_ativos.push(ChoqueEventoAtivo {
                        evento_id: evento_id.to_string(),
                        bem_id: curva.bem_id.clone(),
                        dias_restantes: duracao,
                        duracao_total: duracao,
                        multiplicador_demanda: ajustar_mult(ch.multiplicador_demanda),
                        multiplicador_procura_estrategica: ajustar_mult(
                            ch.multiplicador_procura_estrategica,
                        ),
                        multiplicador_absorcao_excedente: ajustar_mult(
                            ch.multiplicador_absorcao_excedente,
                        ),
                        multiplicador_oferta_importacao_residual: ajustar_mult(
                            ch.multiplicador_oferta_importacao_residual,
                        ),
                    });
                    disparou_algum = true;
                }
            }
        }
        self.eventos_disparados
            .insert(evento_id.to_string(), dias_decorridos);
        disparou_algum
    }

    /// Avança em 1 dia os relógios de choques ativos e dispara choques autônomos cuja janela chegou.
    pub(crate) fn avancar_dia_choques(
        &mut self,
        catalogo: &CatalogoEconomia,
        data_atual: DataJogo,
        dias_decorridos: u64,
    ) {
        if catalogo.ciclos_demanda.curvas.is_empty() {
            return;
        }

        // Decrementar choques em vigor
        for ch in &mut self.choques_ativos {
            ch.dias_restantes = ch.dias_restantes.saturating_sub(1);
        }
        self.choques_ativos.retain(|ch| ch.dias_restantes > 0);

        // Gatilho autônomo com desvio por semente (±1.1 anos) para partidas sem EventosPlugin externo
        if self.usar_gatilhos_historicos_autonomos {
            let ano = ano_decimal(data_atual);
            let mut pendentes: Vec<String> = Vec::new();
            for curva in &catalogo.ciclos_demanda.curvas {
                for ch in &curva.choques_eventos {
                    if self.eventos_disparados.contains_key(&ch.evento_id) {
                        continue;
                    }
                    let desvio_ano =
                        1.1 * ruido_simetrico(self.semente_partida, &ch.evento_id, 0x5150);
                    let ano_gatilho = (ch.ano_referencia + desvio_ano).max(1946.5);
                    if ano >= ano_gatilho && !pendentes.iter().any(|e| e == &ch.evento_id) {
                        pendentes.push(ch.evento_id.clone());
                    }
                }
            }
            for ev_id in pendentes {
                self.disparar_evento(catalogo, &ev_id, dias_decorridos);
            }
        }
    }

    /// Recalcula em $O(\text{bens})$ os multiplicadores diários de demanda, peso de consumo,
    /// procura estratégica e absorção/oferta residual externa.
    pub(crate) fn recalcular_fatores_diarios(
        &mut self,
        catalogo: &CatalogoEconomia,
        paises: &[Pais],
        num_mercados: usize,
        data_atual: DataJogo,
        dias_decorridos: u64,
    ) {
        let num_bens = catalogo.bens.len();
        self.garantir_buffers_volateis(num_bens, num_mercados);

        self.fator_demanda_bem.fill(1.0);
        self.fator_peso_pop_bem.fill(1.0);
        self.fator_procura_estrategica_bem.fill(0.0);
        self.fator_absorcao_excedente_bem.fill(1.0);
        self.fator_oferta_importacao_bem.fill(1.0);
        self.fator_choque_demanda_bem.fill(1.0);

        if catalogo.ciclos_demanda.curvas.is_empty() {
            return;
        }

        let ano = ano_decimal(data_atual);
        // Rampa suave nos primeiros 18 meses para preservar o equilíbrio fino de partida em 1945
        let rampa_variacao = ((dias_decorridos as f64) / 540.0).clamp(0.0, 1.0);
        // Progresso tecnológico médio endógeno (produtividade dos países líderes) caso tecnologias
        // explícitas ainda não tenham sido notificadas
        let prod_media_lideres = if paises.is_empty() {
            1.0
        } else {
            let mut soma_prod = 0.0;
            let mut soma_peso = 0.0;
            for p in paises {
                let w = p.pib_diario.max(1.0);
                soma_prod += p.produtividade * w;
                soma_peso += w;
            }
            if soma_peso > 0.0 {
                soma_prod / soma_peso
            } else {
                1.0
            }
        };
        let difusao_endogena_global = ((prod_media_lideres - 1.0) / 0.85).clamp(0.0, 1.0);

        for curva in &catalogo.ciclos_demanda.curvas {
            let g = curva.bem_idx;
            if g >= num_bens {
                continue;
            }

            // 1. Variação determinística por semente da partida (impede decorar anos/picos exatos)
            let r_ano = ruido_simetrico(self.semente_partida, &curva.bem_id, 0x101);
            let r_amp = ruido_simetrico(self.semente_partida, &curva.bem_id, 0x202);
            let r_per = ruido_simetrico(self.semente_partida, &curva.bem_id, 0x303);
            let r_fas = ruido_simetrico(self.semente_partida, &curva.bem_id, 0x404);

            let desvio_anos = curva.amplitude_desvio_anos * r_ano * rampa_variacao;
            let ano_efetivo = (ano + desvio_anos).max(1945.0);

            let (base_dem, base_pop, base_est, base_abs) =
                interpolar_pontos(&curva.pontos_era, ano_efetivo);

            // Escala de intensidade estrutural da partida (atua sobre o afastamento frente a 1945)
            let ganho_partida = 1.0 + curva.amplitude_variacao_partida * r_amp * rampa_variacao;
            let periodo_anos = 7.2 + 2.1 * r_per; // 5.1 a 9.3 anos
            let fase = (r_fas + 1.0) * std::f64::consts::PI;
            let onda_curta = 1.0
                + curva.amplitude_ciclo_curto
                    * rampa_variacao
                    * (std::f64::consts::TAU * (ano - 1945.0) / periodo_anos + fase).sin();

            let mut f_dem = (1.0 + (base_dem - 1.0) * ganho_partida) * onda_curta;
            let mut f_pop = 1.0 + (base_pop - 1.0) * ganho_partida;
            let mut f_est = if base_est > 0.0 {
                let base0 = curva.pontos_era[0].fator_procura_estrategica;
                (base0 + (base_est - base0) * ganho_partida) * onda_curta
            } else {
                0.0
            };
            let mut f_abs = base_abs;

            // 2. Gatilhos tecnológicos (explícitos via árvore de pesquisa + difusão industrial gradual)
            for gt in &curva.gatilhos_tecnologia {
                let n_paises_tec = self
                    .tecnologias_pesquisadas
                    .get(&gt.tecnologia_id)
                    .map(|v| v.len())
                    .unwrap_or(0);
                // 6 potências adotantes já representam difusão mundial plena da tecnologia no mercado
                let adocao_explicita = ((n_paises_tec as f64) / 6.0).clamp(0.0, 1.0);
                let adocao = if self.tecnologias_pesquisadas.is_empty() {
                    0.35 * difusao_endogena_global
                } else {
                    adocao_explicita.max(0.20 * difusao_endogena_global)
                };
                if adocao > 0.0 {
                    f_dem += gt.delta_demanda * adocao;
                    f_pop += gt.delta_peso_pop * adocao;
                    f_est += gt.delta_procura_estrategica * adocao;
                    f_abs += gt.delta_absorcao_excedente * adocao;
                }
            }

            self.fator_demanda_bem[g] = f_dem.clamp(0.15, 4.5);
            self.fator_peso_pop_bem[g] = f_pop.clamp(0.08, 5.0);
            self.fator_procura_estrategica_bem[g] = f_est.clamp(0.0, 4.5);
            self.fator_absorcao_excedente_bem[g] = f_abs.clamp(0.02, 1.0);
            // Quando a demanda estrutural/estratégica global dispara acima de 1945, o resto do mundo
            // também disputa o mesmo recurso escasso e tem menos folga residual para exportar.
            let pressao_global = self.fator_demanda_bem[g]
                .max(self.fator_procura_estrategica_bem[g])
                .max(1.0);
            if pressao_global > 1.02 {
                let aperto_mundial = (1.0 / pressao_global.powf(1.35)).clamp(0.15, 1.0);
                self.fator_oferta_importacao_bem[g] *= aperto_mundial;
            }
        }

        // 3. Aplicar choques históricos ativos com envelope suave na metade final
        for ch in &self.choques_ativos {
            let Some(&g) = catalogo.indice_bens.get(&ch.bem_id) else {
                continue;
            };
            let frac_restante =
                (ch.dias_restantes as f64 / ch.duracao_total.max(1) as f64).clamp(0.0, 1.0);
            // Intensidade 1.0 na primeira metade do choque, decaindo linearmente até 0.0 ao expirar
            let peso = (frac_restante * 2.0).clamp(0.0, 1.0);
            let interp = |alvo: f64| 1.0 + peso * (alvo - 1.0);

            self.fator_choque_demanda_bem[g] =
                (self.fator_choque_demanda_bem[g] * interp(ch.multiplicador_demanda)).clamp(0.15, 3.5);
            self.fator_procura_estrategica_bem[g] = (self.fator_procura_estrategica_bem[g]
                * interp(ch.multiplicador_procura_estrategica))
            .clamp(0.0, 5.0);
            self.fator_absorcao_excedente_bem[g] = (self.fator_absorcao_excedente_bem[g]
                * interp(ch.multiplicador_absorcao_excedente))
            .clamp(0.02, 1.0);
            self.fator_oferta_importacao_bem[g] = (self.fator_oferta_importacao_bem[g]
                * interp(ch.multiplicador_oferta_importacao_residual))
            .clamp(0.05, 1.5);
        }

        // 4. Pesos estratégicos por mercado (capacidade militar + escala econômica dos países membros)
        self.peso_militar_industrial_mercado.fill(0.0);
        for p in paises {
            if p.mercado_idx < num_mercados {
                let w = p.forca_militar_total.max(0.0) * 10.0 + (p.pib_diario.max(0.0) * 1e-5);
                self.peso_militar_industrial_mercado[p.mercado_idx] += w;
            }
        }
    }

    /// Escala a demanda industrial/produtiva (`proxima_demanda` logo após `producao`) pelo fator
    /// estrutural da era + tecnologia. Não afeta o orçamento das famílias.
    pub(crate) fn aplicar_demanda_industrial_era(
        &self,
        catalogo: &CatalogoEconomia,
        num_mercados: usize,
        proxima_demanda: &mut [f64],
    ) {
        if catalogo.ciclos_demanda.curvas.is_empty() {
            return;
        }
        let num_bens = catalogo.bens.len();
        for m in 0..num_mercados {
            let base = m * num_bens;
            for curva in &catalogo.ciclos_demanda.curvas {
                let g = curva.bem_idx;
                let f = self.fator_demanda_bem[g];
                if (f - 1.0).abs() > 1e-6 {
                    proxima_demanda[base + g] *= f;
                }
            }
        }
    }

    /// Injeta a procura estratégica governamental/militar (ex.: corrida nuclear do urânio) e
    /// multiplicadores de choques históricos antes do fechamento comercial do mercado.
    pub(crate) fn aplicar_procura_estrategica_e_choques(
        &self,
        catalogo: &CatalogoEconomia,
        num_mercados: usize,
        proxima_demanda: &mut [f64],
    ) {
        if catalogo.ciclos_demanda.curvas.is_empty() {
            return;
        }
        let num_bens = catalogo.bens.len();
        let soma_peso_mil: f64 = self.peso_militar_industrial_mercado.iter().sum();

        for curva in &catalogo.ciclos_demanda.curvas {
            let g = curva.bem_idx;
            let f_choque = self.fator_choque_demanda_bem[g];
            if (f_choque - 1.0).abs() > 1e-6 {
                for m in 0..num_mercados {
                    proxima_demanda[m * num_bens + g] *= f_choque;
                }
            }

            let f_est = self.fator_procura_estrategica_bem[g];
            let cap_global_1945 = self.oferta_nominal_inicial_bem.get(g).copied().unwrap_or(0.0);
            if f_est > 1e-6 && cap_global_1945 > 1e-6 {
                let procura_total = f_est * cap_global_1945;
                // 65% via contratos diretos de compra nos centros produtores (ex.: CDA/AEC/Wismut)
                // + 35% concentrado nos mercados das potências militares/industriais.
                let parcela_contratos = 0.65 * procura_total;
                let parcela_potencias = 0.35 * procura_total;
                for m in 0..num_mercados {
                    let idx = m * num_bens + g;
                    let cap_m = self
                        .oferta_nominal_inicial_mercado_bem
                        .get(idx)
                        .copied()
                        .unwrap_or(0.0);
                    let via_contrato = parcela_contratos * (cap_m / cap_global_1945);
                    let via_potencia = if soma_peso_mil > 1e-9 {
                        parcela_potencias * (self.peso_militar_industrial_mercado[m] / soma_peso_mil)
                    } else {
                        parcela_potencias / (num_mercados.max(1) as f64)
                    };
                    proxima_demanda[idx] += via_contrato + via_potencia;
                }
            }
        }
    }
}
