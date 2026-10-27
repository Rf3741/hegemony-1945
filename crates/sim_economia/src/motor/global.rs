//! Mercado global: preço mundial por bem, formado pela oferta e demanda agregadas de todos os
//! mercados, que "puxa" os preços nacionais conforme a abertura comercial de cada mercado
//! (tarifa de importação e lei de comércio do país líder). Guarda o histórico diário de preços.

use super::MundoEconomico;
use crate::motor::mercado::calcular_preco_alvo;
use std::collections::VecDeque;

/// Parâmetros do mercado global.
#[derive(Debug, Clone)]
pub struct ConfigMercadoGlobal {
    /// Fração diária da diferença (preço mundial - preço nacional) absorvida por arbitragem, com abertura 1.0.
    pub forca_arbitragem: f64,
    /// Suavização exponencial diária do preço mundial (0..1).
    pub suavizacao_mundial: f64,
    /// Máximo de dias guardados no histórico de cada bem.
    pub historico_max_dias: usize,
}

impl Default for ConfigMercadoGlobal {
    fn default() -> Self {
        Self { forca_arbitragem: 0.08, suavizacao_mundial: 0.15, historico_max_dias: 1100 }
    }
}

/// Estado do mercado global (vetores indexados pela ordem de `catalogo().bens`).
#[derive(Debug, Clone)]
pub struct MercadoGlobal {
    pub config: ConfigMercadoGlobal,
    pub preco_mundial: Vec<f64>,
    pub oferta_mundial: Vec<f64>,
    pub demanda_mundial: Vec<f64>,
    /// Histórico diário do preço mundial por bem (mais antigo primeiro).
    pub historico_mundial: Vec<VecDeque<f32>>,
    /// Índice do mercado acompanhado (o do jogador) no histórico nacional.
    pub mercado_acompanhado: Option<usize>,
    /// Histórico diário do preço no mercado acompanhado.
    pub historico_nacional: Vec<VecDeque<f32>>,
    /// Dias registrados no histórico desde o início.
    pub dias_registrados: u64,
}

impl MercadoGlobal {
    /// Inicializa o preço mundial como média dos preços dos mercados, ponderada pelo volume.
    pub fn novo(mundo: &MundoEconomico) -> Self {
        let n = mundo.catalogo.bens.len();
        let mut g = Self {
            config: ConfigMercadoGlobal::default(),
            preco_mundial: vec![0.0; n],
            oferta_mundial: vec![0.0; n],
            demanda_mundial: vec![0.0; n],
            historico_mundial: vec![VecDeque::new(); n],
            mercado_acompanhado: None,
            historico_nacional: vec![VecDeque::new(); n],
            dias_registrados: 0,
        };
        for b in 0..n {
            let (mut soma, mut peso) = (0.0, 0.0);
            for m in &mundo.mercados {
                let w = m.oferta_idx[b] + m.demanda_idx[b] + 1.0;
                soma += m.precos_idx[b] * w;
                peso += w;
            }
            g.preco_mundial[b] =
                if peso > 0.0 { soma / peso } else { mundo.catalogo.bens[b].preco_base };
        }
        g
    }

    /// Define qual mercado nacional terá histórico (reinicia o histórico nacional se mudar).
    pub fn acompanhar_mercado(&mut self, mercado_idx: Option<usize>) {
        if self.mercado_acompanhado != mercado_idx {
            self.mercado_acompanhado = mercado_idx;
            for h in &mut self.historico_nacional {
                h.clear();
            }
        }
    }

    /// Variação percentual (fração) do último preço em relação a `dias` atrás.
    pub fn variacao(&self, bem_idx: usize, dias: usize, nacional: bool) -> f64 {
        let h = if nacional { &self.historico_nacional[bem_idx] } else { &self.historico_mundial[bem_idx] };
        let n = h.len();
        if n < 2 {
            return 0.0;
        }
        let ref_i = n.saturating_sub(1 + dias);
        let (a, b) = (h[ref_i] as f64, h[n - 1] as f64);
        if a.abs() < 1e-9 { 0.0 } else { b / a - 1.0 }
    }
}

/// Comércio mundial automático (fase 2b): (1) mercados com excedente exportam para mercados
/// deficitários, e (2) o resto do mundo cobre parte do déficit residual a preço de mercado + tarifa.
/// Tudo limitado pela abertura comercial de cada mercado. Os fluxos entram como ordens de
/// compra (origem) e venda (destino), exatamente como as rotas comerciais explícitas.
pub(crate) fn processar_comercio_mundial(
    catalogo: &crate::catalogo::CatalogoEconomia,
    config: &crate::entidades::ConfigSimulacao,
    mercados: &[crate::entidades::Mercado],
    paises: &mut [crate::entidades::Pais],
    abertura: &[f64],
    proxima_oferta: &mut [f64],
    proxima_demanda: &mut [f64],
    importacoes_liquidas: &mut [f64],
) {
    let n = catalogo.bens.len();
    let nm = mercados.len();
    for g in 0..n {
        let bem = &catalogo.bens[g];
        if !bem.comercializavel {
            continue;
        }
        let (mut cap_exp, mut dem_imp) = (0.0, 0.0);
        for m in 0..nm {
            let net = proxima_oferta[m * n + g] - proxima_demanda[m * n + g];
            if net > 0.0 {
                cap_exp += 0.5 * net * abertura[m];
            } else {
                dem_imp += -net * abertura[m];
            }
        }
        let t = cap_exp.min(dem_imp);
        if t > 1e-9 {
            // Lê os saldos antes de alterar (os fluxos usam os saldos originais de cada mercado)
            for m in 0..nm {
                let i = m * n + g;
                let net = proxima_oferta[i] - proxima_demanda[i];
                if net > 0.0 {
                    let vol = t * (0.5 * net * abertura[m]) / cap_exp;
                    proxima_demanda[i] += vol;
                    importacoes_liquidas[i] -= vol;
                } else if net < 0.0 {
                    let vol = t * (-net * abertura[m]) / dem_imp;
                    proxima_oferta[i] += vol;
                    importacoes_liquidas[i] += vol;
                    let lider = mercados[m].pais_lider_idx;
                    paises[lider].orcamento.receita_tarifas += vol
                        * bem.preco_base
                        * paises[lider].politica_fiscal.tarifa_importacao.max(0.0)
                        * paises[lider].eficiencia_arrecadacao;
                }
            }
        }
        // Resto do mundo: cobre parte do déficit que sobrou
        for m in 0..nm {
            let i = m * n + g;
            let falta = proxima_demanda[i] - proxima_oferta[i];
            if falta > 1e-9 {
                let vol = falta * config.fracao_maxima_importacao_residual.clamp(0.0, 1.0) * abertura[m];
                proxima_oferta[i] += vol;
                importacoes_liquidas[i] += vol;
                let lider = mercados[m].pais_lider_idx;
                paises[lider].orcamento.receita_tarifas += vol
                    * bem.preco_base
                    * paises[lider].politica_fiscal.tarifa_importacao.max(0.0)
                    * paises[lider].eficiencia_arrecadacao;
            }
        }
    }
}

impl MundoEconomico {
    /// Abertura comercial (0.05..1.0) de um mercado: depende da lei de comércio e da tarifa de importação do país líder.
    pub fn abertura_comercial_mercado(&self, mercado_idx: usize) -> f64 {
        let lider = &self.paises[self.mercados[mercado_idx].pais_lider_idx];
        let lei = (lider.mult_volume_comercio_lei / 1.25).clamp(0.2, 1.0);
        let tarifa = (1.0 - 2.0 * lider.politica_fiscal.tarifa_importacao.max(0.0)).clamp(0.2, 1.0);
        (lei * tarifa).clamp(0.05, 1.0)
    }

    /// Atualiza o preço mundial (oferta e demanda agregadas), aplica a arbitragem sobre os preços
    /// nacionais dos bens comercializáveis e registra o histórico do dia. Chamar após cada tick diário.
    pub fn atualizar_mercado_global(&mut self, global: &mut MercadoGlobal) {
        let n = self.catalogo.bens.len();
        let cfg = global.config.clone();
        for b in 0..n {
            let (mut o, mut d, mut soma, mut peso) = (0.0, 0.0, 0.0, 0.0);
            for m in &self.mercados {
                o += m.oferta_idx[b];
                d += m.demanda_idx[b];
                let w = m.oferta_idx[b] + m.demanda_idx[b] + 1.0;
                soma += m.precos_idx[b] * w;
                peso += w;
            }
            global.oferta_mundial[b] = o;
            global.demanda_mundial[b] = d;
            let bem = &self.catalogo.bens[b];
            let alvo = if bem.comercializavel {
                calcular_preco_alvo(bem.preco_base, o, d, &self.config)
            } else {
                soma / peso.max(1e-9)
            };
            let ant = global.preco_mundial[b];
            let novo = (ant + cfg.suavizacao_mundial * (alvo - ant)).clamp(
                bem.preco_base * self.config.multiplicador_preco_minimo,
                bem.preco_base * self.config.multiplicador_preco_maximo,
            );
            global.preco_mundial[b] = if novo.is_finite() { novo } else { bem.preco_base };
        }

        // Arbitragem: o preço mundial puxa os preços nacionais proporcionalmente à abertura comercial.
        for m_idx in 0..self.mercados.len() {
            let k = cfg.forca_arbitragem * self.abertura_comercial_mercado(m_idx);
            for b in 0..n {
                let bem = &self.catalogo.bens[b];
                if !bem.comercializavel {
                    continue;
                }
                let p = self.mercados[m_idx].precos_idx[b];
                let novo = (p + k * (global.preco_mundial[b] - p)).clamp(
                    bem.preco_base * self.config.multiplicador_preco_minimo,
                    bem.preco_base * self.config.multiplicador_preco_maximo,
                );
                self.mercados[m_idx].precos_idx[b] = novo;
            }
        }

        // Histórico diário
        let max = cfg.historico_max_dias.max(2);
        for b in 0..n {
            let h = &mut global.historico_mundial[b];
            h.push_back(global.preco_mundial[b] as f32);
            if h.len() > max {
                h.pop_front();
            }
            if let Some(mi) = global.mercado_acompanhado {
                let h = &mut global.historico_nacional[b];
                h.push_back(self.mercados[mi].precos_idx[b] as f32);
                if h.len() > max {
                    h.pop_front();
                }
            }
        }
        global.dias_registrados += 1;
    }

    // ---- Consultas auxiliares para interfaces ----

    /// Índice do mercado (em `mercados()`) de um país.
    pub fn indice_mercado_do_pais(&self, pais_id: &str) -> Option<usize> {
        self.indice_paises.get(pais_id).map(|&i| self.paises[i].mercado_idx)
    }

    /// Índice de um bem no catálogo.
    pub fn indice_bem(&self, bem_id: &str) -> Option<usize> {
        self.catalogo.indice_bens.get(bem_id).copied()
    }

    /// Preço, oferta, demanda e saldo líquido de importação de um bem em um mercado (por índices).
    pub fn dados_mercado_bem(&self, mercado_idx: usize, bem_idx: usize) -> (f64, f64, f64, f64) {
        let m = &self.mercados[mercado_idx];
        (m.precos_idx[bem_idx], m.oferta_idx[bem_idx], m.demanda_idx[bem_idx], m.importacoes_liquidas_idx[bem_idx])
    }

    /// Se a província do país comporta +1 nível do tipo de prédio (terra arável / recurso natural).
    pub fn pode_construir(&self, provincia_id: &str, tipo_predio_id: &str) -> bool {
        let (Some(&p), Some(&t)) =
            (self.indice_provincias.get(provincia_id), self.catalogo.indice_tipos_predio.get(tipo_predio_id))
        else {
            return false;
        };
        crate::motor::governo::provincia_comporta_novo_nivel(&self.catalogo, &self.provincias[p], t)
    }

    /// Custo estimado em materiais, a preços do mercado do país, de 1 nível do tipo de prédio.
    pub fn custo_materiais_obra(&self, pais_id: &str, tipo_predio_id: &str) -> Option<f64> {
        let &c = self.indice_paises.get(pais_id)?;
        let &t = self.catalogo.indice_tipos_predio.get(tipo_predio_id)?;
        let def = &self.catalogo.tipos_predio[t];
        Some(crate::motor::governo::custo_materiais_estimado(
            &self.catalogo,
            &self.mercados[self.paises[c].mercado_idx],
            def.categoria,
            def.custo_pontos_construcao,
        ))
    }

    /// Pontos de construção por dia disponíveis para o país (base + setor, com eficiência da lei).
    /// O valor de setor é o do último tick (aproximado por `pontos_aplicados_ultimo_tick` quando há fila).
    pub fn pontos_construcao_dia(&self, pais_id: &str) -> Option<f64> {
        let &c = self.indice_paises.get(pais_id)?;
        let p = &self.paises[c];
        Some((p.pontos_construcao_base + self.buffers.pontos_construcao_setores_pais.get(c).copied().unwrap_or(0.0)) * p.mult_eficiencia_construcao_lei.max(0.0))
    }
}
