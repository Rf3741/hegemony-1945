//! IA economica: postura fiscal, obras estatais e reacao a crises.

use sim_economia::entidades::{Pais, Provincia};
use sim_economia::{calcular_lucratividade_esperada, CategoriaPredio, ContextoLucratividade, MundoEconomico, TipoPredio};
use std::collections::BTreeMap;

/// Histerese: apos mexer em impostos, a IA espera este numero de dias.
const COOLDOWN_FISCAL: u64 = 90;
const COOLDOWN_EMERGENCIA: u64 = 30;
/// Maximo (minimo garantido) de obras estatais simultaneas na fila; cresce com a capacidade de construcao.
const MAX_OBRAS_ESTATAIS: usize = 6;
/// Meta de niveis do setor de construcao como fracao do total de niveis de predios do pais.
/// Cada nivel gera ~22 pontos/dia (~70 niveis de predio por ano), logo 0,02% sustenta ~1,5%/ano de expansao.
const META_CONSTRUCAO: f64 = 0.0002;
/// Reconstrucao pos-guerra (tag, multiplicador inicial da meta de construcao). O efeito decai
/// linearmente ate 1,0 em `ANOS_RECONSTRUCAO` anos (Plano Marshall, milagres alemao e japones etc.).
const RECONSTRUCAO: [(&str, f64); 14] = [
    ("GER", 2.2), ("JAP", 2.2), ("ITA", 1.6), ("FRA", 1.5), ("GBR", 1.2), ("SOV", 1.4), ("POL", 1.5),
    ("CZE", 1.4), ("HUN", 1.5), ("NLD", 1.5), ("BEL", 1.5), ("AUT", 1.5), ("FIN", 1.4), ("GRE", 1.3),
];
const ANOS_RECONSTRUCAO: f64 = 10.0;
/// Maximo de obras estatais novas por passo semanal.
const NOVAS_POR_SEMANA: usize = 3;

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
    /// PIB per capita medio do mundo (atualizado mensalmente); base da convergencia entre paises.
    pib_pc_mundial: f64,
}

impl IaEconomia {
    pub fn novo() -> Self {
        Self::default()
    }

    /// Executa os passos da IA que vencem hoje. Chamar uma vez por dia, depois do tick da economia.
    pub fn passo_diario(&mut self, mundo: &mut MundoEconomico, ignorar: &[String]) {
        let dia = mundo.dias_decorridos();
        if dia % 30 == 0 || self.pib_pc_mundial <= 0.0 {
            let pib: f64 = mundo.paises().iter().map(|p| p.pib_diario).sum();
            let pop: f64 = mundo.provincias().iter().map(|p| p.populacao_total()).sum();
            if pop > 0.0 {
                self.pib_pc_mundial = pib / pop;
            }
        }
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

    fn contexto_lucratividade(mundo: &MundoEconomico, prov: &Provincia, tipo: &TipoPredio) -> ContextoLucratividade {
            let Some(est) = mundo.obter_estado(&prov.estado_id) else { return ContextoLucratividade::default() };
            let pop = prov.populacao_total();
            let sol = prov.padrao_de_vida_medio();
            let desempregados: f64 = prov.pops.iter().map(|p| p.desempregados()).sum();
            let salario = prov.predios.iter().map(|p| p.salario_base).sum::<f64>() / prov.predios.len().max(1) as f64;
            let niveis = mundo.provincias().iter().filter(|p| p.estado_id == est.id).flat_map(|p| &p.predios)
                .filter(|b| b.tipo_predio_id == tipo.id).map(|b| b.nivel as f64).sum::<f64>();
            let (livre, total) = if tipo.usa_terra_aravel {
                (Some(est.terra_aravel_livre() as f64), Some(est.terra_aravel.max(1) as f64))
            } else if let Some(rec) = &tipo.recurso_natural_requerido {
                let total = est.limite_recurso(rec) as f64;
                (Some(total - niveis), Some(total.max(1.0)))
            } else {
                (None, None)
            };
            ContextoLucratividade {
                salario_base: if salario > 0.0 { salario } else { 4.0 * (sol / 10.0).clamp(0.55, 2.5) },
                padrao_vida: sol,
                desempregados,
                mao_de_obra_total: pop,
                infraestrutura_uso: est.infraestrutura_uso,
                infraestrutura_capacidade: est.infraestrutura_capacidade,
                acesso_mercado: est.acesso_mercado,
                qualidade_recurso: est.fator_qualidade_predio(tipo),
                potencial_livre: livre,
                potencial_total: total,
                niveis_iguais: niveis,
            }
        }

    fn estimar_lucro(mundo: &MundoEconomico, prov: &Provincia, tipo: &TipoPredio) -> sim_economia::EstimativaLucratividade {
            let cat = mundo.catalogo();
            let mercado = mundo.indice_mercado_do_pais(&prov.pais_id);
            let precos = cat.bens.iter().enumerate().map(|(i, b)| {
                let preco = mercado.map(|m| mundo.dados_mercado_bem(m, i).0).unwrap_or(b.preco_base);
                (b.id.clone(), preco)
            }).collect::<Vec<_>>();
        calcular_lucratividade_esperada(cat, tipo, &tipo.metodos_padrao, &precos, Self::contexto_lucratividade(mundo, prov, tipo))
    }

    fn passo_semanal(&mut self, mundo: &mut MundoEconomico, id: &str, dia_atual: u64) {
        self.montar_indice(mundo);
        let Some(p) = mundo.obter_pais(id) else { return };
        let postura = classificar(p);
        self.estados.entry(id.to_string()).or_default().postura = postura;
        if !postura.permite_necessidades() {
            return;
        }
        let estatais = p.fila_construcao.iter().filter(|i| !i.privado).count();
        let Some(idx) = self.indice_provincias.get(id).cloned() else { return };
        let lote = mundo.tamanho_lote_obra(id);
        // Capacidade teorica de construcao (pontos/dia): base + ~22 pontos por nivel do setor de construcao.
        let niveis_constr_atual: f64 = idx
            .iter()
            .flat_map(|&pi| mundo.provincias()[pi].predios.iter())
            .filter(|b| mundo.catalogo().tipos_predio.iter().any(|t| t.id == b.tipo_predio_id && t.categoria == CategoriaPredio::Construcao))
            .map(|b| b.nivel as f64)
            .sum();
        let cap_dia = p.pontos_construcao_base + 22.0 * niveis_constr_atual;
        let obras_paralelas = (cap_dia / (25.0 * lote as f64)).ceil() as usize;
        let max_estatais = MAX_OBRAS_ESTATAIS.max(2 * obras_paralelas).min(80);
        let novas_semana = NOVAS_POR_SEMANA.max((7.0 * cap_dia / (130.0 * lote as f64)).ceil() as usize).min(40);
        // Fila cheia: so o setor de construcao (gargalo do crescimento) ainda entra.
        let fila_cheia = estatais >= max_estatais;
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
        let cat = mundo.catalogo();
        // Ritmo de expansao do pais: convergencia (pobres crescem mais, ricos menos) e reconstrucao.
        let pop_pais: f64 = idx.iter().map(|&pi| mundo.provincias()[pi].populacao_total()).sum();
        let pc_rel = if pop_pais > 0.0 && self.pib_pc_mundial > 0.0 {
            (p.pib_diario / pop_pais / self.pib_pc_mundial).max(0.05)
        } else {
            1.0
        };
        let fator_convergencia = pc_rel.powf(-0.5).clamp(0.45, 1.5);
        let anos = dia_atual as f64 / 365.0;
        let fator_recon = p
            .tag
            .as_deref()
            .and_then(|t| RECONSTRUCAO.iter().find(|(x, _)| *x == t))
            .map_or(1.0, |&(_, m)| 1.0 + (m - 1.0) * (1.0 - anos / ANOS_RECONSTRUCAO).max(0.0));
        let meta = META_CONSTRUCAO * fator_convergencia * fator_recon;
        // Niveis do setor de construcao ja a caminho (na fila) contam: evita empilhar projetos que nunca terminam.
        let a_caminho_constr: f64 = p
            .fila_construcao
            .iter()
            .filter(|i| cat.tipos_predio.iter().any(|t| t.id == i.tipo_predio_id && t.categoria == CategoriaPredio::Construcao))
            .map(|i| i.niveis as f64)
            .sum();

        // Retorno medio por nivel de cada tipo de predio produtivo ja existente no pais.
        let mut retorno: BTreeMap<usize, (f64, f64)> = BTreeMap::new();
        let mut niveis_total = 0.0;
        let mut niveis_construcao = 0.0;
        for &pi in &idx {
            for b in &mundo.provincias()[pi].predios {
                if let Some(ti) = cat.tipos_predio.iter().position(|t| t.id == b.tipo_predio_id) {
                    let e = retorno.entry(ti).or_default();
                    e.0 += b.lucro_ultimo_tick;
                    e.1 += b.nivel as f64;
                    niveis_total += b.nivel as f64;
                    if cat.tipos_predio[ti].categoria == CategoriaPredio::Construcao {
                        niveis_construcao += b.nivel as f64;
                    }
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
                _ if fila_cheia && t.categoria != CategoriaPredio::Construcao => continue,
                CategoriaPredio::Militar => continue,
                CategoriaPredio::Construcao => {
                    // Capacidade de construcao proporcional ao tamanho da economia (gargalo do crescimento).
                    let falta = meta * niveis_total.max(100.0) - niveis_construcao;
                    if falta - a_caminho_constr <= 0.0 { continue }
                    500.0
                }
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
            // Escolhe a melhor provincia usando a mesma rentabilidade regional exibida ao jogador.
            let mut melhor: Option<(f64, usize)> = None;
            for &pi in &idx {
                let pr = &mundo.provincias()[pi];
                let acesso = mundo.obter_estado(&pr.estado_id).map_or(1.0, |e| e.acesso_mercado);
                let score = if t.categoria == CategoriaPredio::Infraestrutura {
                    if acesso >= 0.92 { continue }
                    180.0 * (1.0 - acesso)
                } else {
                    let lucro = Self::estimar_lucro(mundo, pr, t).lucro_liquido;
                    let regional = lucro / t.custo_pontos_construcao.max(1.0) * 500.0;
                    regional.max(base * 0.25) - pr.predios.len() as f64 * 0.5
                };
                if melhor.is_none_or(|(s, _)| score > s) {
                    melhor = Some((score, pi));
                }
            }
            if let Some((s, pi)) = melhor
                && s > 5.0
            {
                cands.push((s, ti, pi));
                if t.categoria == CategoriaPredio::Construcao {
                    // Varios niveis de uma vez (a mesma provincia comporta); nao contam no limite semanal.
                    let falta = meta * niveis_total.max(100.0) - niveis_construcao - a_caminho_constr;
                    for _ in 1..((falta / lote as f64).ceil() as usize).min(4) {
                        cands.push((s, ti, pi));
                    }
                }
            }
        }
        cands.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        // Tenta os melhores candidatos (a provincia pode nao comportar), ate `NOVAS_POR_SEMANA` aceitos.
        let mut aceitas = 0;
        for (_, ti, pi) in cands.into_iter().take(16 + novas_semana) {
            let tipo = mundo.catalogo().tipos_predio[ti].id.clone();
            let prov = mundo.provincias()[pi].id.clone();
            if mundo.enfileirar_construcao_lote(id, &prov, &tipo, false, lote).is_ok() {
                let ids_constr: Vec<String> = mundo
                    .catalogo()
                    .tipos_predio
                    .iter()
                    .filter(|t| t.categoria == CategoriaPredio::Construcao)
                    .map(|t| t.id.clone())
                    .collect();
                self.obras_enfileiradas += 1;
                // Obras do setor de construcao furam a fila: elas multiplicam a capacidade de todas as outras.
                if mundo.catalogo().tipos_predio[ti].categoria == CategoriaPredio::Construcao
                    && let Some(p) = mundo.obter_pais_mut(id)
                    && let Some(item) = p.fila_construcao.pop()
                {
                    // Na frente das demais obras, mas atras das obras de construcao ja enfileiradas (sem inanicao).
                    let pos = p.fila_construcao.iter().take_while(|i| ids_constr.contains(&i.tipo_predio_id)).count();
                    p.fila_construcao.insert(pos, item);
                    continue;
                }
                aceitas += 1;
                if aceitas >= novas_semana {
                    break;
                }
            }
        }
    }
}
