//! IA economica: postura fiscal, obras estatais e reacao a crises.

use sim_economia::entidades::Pais;
use sim_economia::{CategoriaPredio, MundoEconomico};
use std::collections::BTreeMap;

/// Histerese: apos mexer em impostos, a IA espera este numero de dias.
const COOLDOWN_FISCAL: u64 = 90;
const COOLDOWN_EMERGENCIA: u64 = 30;
/// Maximo de obras estatais simultaneas na fila.
const MAX_OBRAS_ESTATAIS: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosturaFiscal {
    ExpansaoAgressiva,
    EquilibrioSustentado,
    AjusteModerado,
    AusteridadeSevera,
    EmergenciaAntiCalote,
}

impl PosturaFiscal {
    /// Pode iniciar obras que nao sejam necessidades (burocracia/infraestrutura)?
    fn permite_obras_de_retorno(self) -> bool {
        matches!(self, Self::ExpansaoAgressiva | Self::EquilibrioSustentado)
    }
    fn permite_necessidades(self) -> bool {
        !matches!(self, Self::AusteridadeSevera | Self::EmergenciaAntiCalote)
    }
}

/// Memoria da IA por pais.
#[derive(Debug, Clone)]
pub struct EstadoFiscalIa {
    pub postura: PosturaFiscal,
    pub ultimo_ajuste: Option<u64>,
}

impl Default for EstadoFiscalIa {
    fn default() -> Self {
        Self { postura: PosturaFiscal::EquilibrioSustentado, ultimo_ajuste: None }
    }
}

/// Classifica a saude fiscal (IA.md 2.1).
pub fn classificar(p: &Pais) -> PosturaFiscal {
    let o = &p.orcamento;
    let r_div = o.divida / o.limite_credito.max(1.0);
    let caixa_dias = o.tesouro / o.despesa_total().max(1.0);
    let receita = o.receita_total();
    if o.em_calote || r_div >= 0.85 || (o.tesouro < 0.0 && o.saldo_diario < 0.0 && r_div >= 0.6) {
        PosturaFiscal::EmergenciaAntiCalote
    } else if r_div >= 0.60 {
        PosturaFiscal::AusteridadeSevera
    } else if r_div >= 0.25 || (o.saldo_diario < -0.05 * receita && caixa_dias < 30.0) {
        PosturaFiscal::AjusteModerado
    } else if r_div == 0.0 && o.divida <= 0.0 && caixa_dias > 90.0 && o.saldo_diario > 0.0 {
        PosturaFiscal::ExpansaoAgressiva
    } else {
        PosturaFiscal::EquilibrioSustentado
    }
}

/// IA economica de todos os paises (exceto os controlados por jogadores).
#[derive(Debug, Clone, Default)]
pub struct IaEconomia {
    pub estados: BTreeMap<String, EstadoFiscalIa>,
    /// Obras estatais enfileiradas pela IA (total, para estatisticas/testes).
    pub obras_enfileiradas: u64,
    /// Ajustes de imposto realizados (total).
    pub ajustes_fiscais: u64,
    /// Provincias por pais (indices em `provincias()`), montado sob demanda.
    indice_provincias: BTreeMap<String, Vec<usize>>,
    n_provincias: usize,
}

impl IaEconomia {
    pub fn novo() -> Self {
        Self::default()
    }

    /// Executa os passos da IA que vencem hoje. Chamar uma vez por dia, depois do tick da economia.
    pub fn passo_diario(&mut self, mundo: &mut MundoEconomico, ignorar: &[String]) {
        let dia = mundo.dias_decorridos();
        let ids: Vec<String> = mundo.paises().iter().map(|p| p.id.clone()).collect();
        for (i, id) in ids.iter().enumerate() {
            if ignorar.iter().any(|x| x == id) {
                continue;
            }
            let fase = dia + i as u64;
            if fase % 7 == 0 {
                self.passo_semanal(mundo, id, dia);
            }
            if fase % 30 == 0 {
                self.passo_mensal(mundo, id, dia);
            }
        }
    }

    // ---------------------------------------------------------------- fiscal (mensal)

    fn passo_mensal(&mut self, mundo: &mut MundoEconomico, id: &str, dia: u64) {
        let Some(p) = mundo.obter_pais(id) else { return };
        let postura = classificar(p);
        let est = self.estados.entry(id.to_string()).or_default();
        est.postura = postura;
        let cd = if postura == PosturaFiscal::EmergenciaAntiCalote { COOLDOWN_EMERGENCIA } else { COOLDOWN_FISCAL };
        if est.ultimo_ajuste.is_some_and(|u| dia < u + cd) {
            return;
        }
        let Some(p) = mundo.obter_pais_mut(id) else { return };
        let pf = &mut p.politica_fiscal;
        let antes = (pf.aliquota_imposto_renda_baixo, pf.aliquota_imposto_renda_medio, pf.aliquota_imposto_renda_alto, pf.aliquota_imposto_dividendos, pf.aliquota_consumo_geral);
        match postura {
            PosturaFiscal::ExpansaoAgressiva => {
                pf.aliquota_consumo_geral = (pf.aliquota_consumo_geral - 0.02).max(0.0);
                pf.aliquota_imposto_renda_baixo = (pf.aliquota_imposto_renda_baixo - 0.02).max(0.0);
            }
            PosturaFiscal::EquilibrioSustentado => {}
            PosturaFiscal::AjusteModerado => {
                pf.aliquota_imposto_renda_alto = (pf.aliquota_imposto_renda_alto + 0.02).min(0.70);
                pf.aliquota_imposto_dividendos = (pf.aliquota_imposto_dividendos + 0.02).min(0.60);
                pf.aliquota_consumo_geral = (pf.aliquota_consumo_geral + 0.01).min(0.30);
            }
            PosturaFiscal::AusteridadeSevera | PosturaFiscal::EmergenciaAntiCalote => {
                for a in [&mut pf.aliquota_imposto_renda_baixo, &mut pf.aliquota_imposto_renda_medio, &mut pf.aliquota_imposto_renda_alto, &mut pf.aliquota_imposto_dividendos] {
                    *a = (*a + 0.04).min(0.75);
                }
                pf.aliquota_consumo_geral = (pf.aliquota_consumo_geral + 0.02).min(0.35);
            }
        }
        let depois = (pf.aliquota_imposto_renda_baixo, pf.aliquota_imposto_renda_medio, pf.aliquota_imposto_renda_alto, pf.aliquota_imposto_dividendos, pf.aliquota_consumo_geral);
        // Sem obras estatais em austeridade/emergencia (evita a espiral do setor de construcao).
        if matches!(postura, PosturaFiscal::AusteridadeSevera | PosturaFiscal::EmergenciaAntiCalote) {
            p.fila_construcao.retain(|i| i.privado || i.progresso_pontos > 0.5 * i.custo_total_pontos);
        }
        if antes != depois {
            let _ = mundo.atualizar_cache_fiscal_pais(id);
            self.ajustes_fiscais += 1;
            self.estados.get_mut(id).unwrap().ultimo_ajuste = Some(dia);
        }
    }

    // ---------------------------------------------------------------- construcao (semanal)

    fn montar_indice(&mut self, mundo: &MundoEconomico) {
        let n = mundo.provincias().len();
        if self.n_provincias == n && !self.indice_provincias.is_empty() {
            return;
        }
        self.indice_provincias.clear();
        for (i, pr) in mundo.provincias().iter().enumerate() {
            self.indice_provincias.entry(pr.pais_id.clone()).or_default().push(i);
        }
        self.n_provincias = n;
    }

    fn passo_semanal(&mut self, mundo: &mut MundoEconomico, id: &str, _dia: u64) {
        self.montar_indice(mundo);
        let Some(p) = mundo.obter_pais(id) else { return };
        let postura = classificar(p);
        self.estados.entry(id.to_string()).or_default().postura = postura;
        if !postura.permite_necessidades() {
            return;
        }
        let estatais = p.fila_construcao.iter().filter(|i| !i.privado).count();
        if estatais >= MAX_OBRAS_ESTATAIS {
            return;
        }
        // Teto de gasto com construcao estatal por postura (fracao da receita).
        let receita = p.orcamento.receita_total().max(1.0);
        let teto = match postura {
            PosturaFiscal::ExpansaoAgressiva => 0.45,
            PosturaFiscal::EquilibrioSustentado => 0.30,
            PosturaFiscal::AjusteModerado => 0.15,
            _ => 0.0,
        };
        if p.orcamento.gasto_construcao + p.orcamento.gasto_obras_materiais > teto * receita {
            return;
        }
        let Some(idx) = self.indice_provincias.get(id).cloned() else { return };
        let cat = mundo.catalogo();

        // Retorno medio por nivel de cada tipo de predio produtivo ja existente no pais.
        let mut retorno: BTreeMap<usize, (f64, f64)> = BTreeMap::new();
        for &pi in &idx {
            for b in &mundo.provincias()[pi].predios {
                if let Some(ti) = cat.tipos_predio.iter().position(|t| t.id == b.tipo_predio_id) {
                    let e = retorno.entry(ti).or_default();
                    e.0 += b.lucro_ultimo_tick;
                    e.1 += b.nivel as f64;
                }
            }
        }
        let burocracia_falta = (p.burocracia_necessaria - p.burocracia_produzida) / p.burocracia_necessaria.max(10.0);
        let burocracia_sobra = p.burocracia_produzida >= 1.10 * p.burocracia_necessaria;

        // Candidatos (score, tipo_idx, provincia_idx)
        let mut cands: Vec<(f64, usize, usize)> = Vec::new();
        for (ti, t) in cat.tipos_predio.iter().enumerate() {
            let base = match t.categoria {
                CategoriaPredio::Governamental if t.financiado_pelo_governo => {
                    if burocracia_sobra || burocracia_falta <= 0.0 { continue }
                    (250.0 * burocracia_falta).clamp(25.0, 150.0)
                }
                CategoriaPredio::Militar | CategoriaPredio::Construcao => continue,
                CategoriaPredio::Infraestrutura => 0.0, // decidido por estado, abaixo
                _ => {
                    if !postura.permite_obras_de_retorno() { continue }
                    // Setores já presentes são priorizados pelo lucro; setores novos recebem
                    // capital semente para evitar que a economia fique presa na composição de 1945.
                    let s = if let Some(&(l, n)) = retorno.get(&ti) {
                        if n <= 0.0 { continue }
                        (l / n / t.custo_pontos_construcao.max(1.0) * 500.0).clamp(-50.0, 100.0)
                    } else {
                        match t.categoria {
                            CategoriaPredio::Industrial | CategoriaPredio::Extrativo | CategoriaPredio::RuralAgricola => 18.0,
                            CategoriaPredio::Infraestrutura => 12.0,
                            _ => 0.0,
                        }
                    };
                    if s <= 5.0 { continue }
                    s
                }
            };
            // Escolhe a melhor provincia (pior acesso para infra; menos prediosa para o resto).
            let mut melhor: Option<(f64, usize)> = None;
            for &pi in &idx {
                let pr = &mundo.provincias()[pi];
                let acesso = mundo.obter_estado(&pr.estado_id).map_or(1.0, |e| e.acesso_mercado);
                let score = if t.categoria == CategoriaPredio::Infraestrutura {
                    if acesso >= 0.92 { continue }
                    180.0 * (1.0 - acesso)
                } else {
                    let m = if acesso < 0.75 { 0.2 } else { acesso };
                    base * m - pr.predios.len() as f64 * 0.5
                };
                if melhor.is_none_or(|(s, _)| score > s) {
                    melhor = Some((score, pi));
                }
            }
            if let Some((s, pi)) = melhor
                && s > 5.0
            {
                cands.push((s, ti, pi));
            }
        }
        cands.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        // Tenta os melhores candidatos ate um ser aceito (a provincia pode nao comportar).
        for (_, ti, pi) in cands.into_iter().take(6) {
            let tipo = mundo.catalogo().tipos_predio[ti].id.clone();
            let prov = mundo.provincias()[pi].id.clone();
            if mundo.enfileirar_construcao(id, &prov, &tipo, false).is_ok() {
                self.obras_enfileiradas += 1;
                break;
            }
        }
    }
}
