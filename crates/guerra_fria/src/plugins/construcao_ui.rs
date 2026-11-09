//! Construção clara e regional: seção "Construir aqui" no painel da província, tela de construção (tecla B),
//! modos de mapa "Potencial" e "Infraestrutura" e tutorial curto em pt-BR.
//!
//! Toda a lógica de decisão (quais prédios cabem, ranking de estados, cores do mapa) fica em funções puras
//! sobre `VistaEconomia`, para ser testada sem janela. A interface só desenha e registra *pedidos*
//! (`ConstrucaoUi::pedidos`), que o sistema `aplicar_pedidos` converte em `Comando::Construir`.
use super::economia_sim::{Comando, EconomiaSim, VistaEconomia};
use super::jogador::{calcular_custo_materiais_obra, fmt_din};
use super::map::{CoresMapa, DadosMapa, ImagemMapa, ProvinciaSelecionada, RecursoCatalogo, pintar_paleta};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use sim_economia::{calcular_com_perfil, precos_por_indice, PerfilPredio, CategoriaPredio, ContextoLucratividade, Estado, EstimativaLucratividade, Provincia, TipoPredio};
use std::collections::HashMap;

/// Modo de pintura do mapa.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ModoMapa {
    #[default]
    Normal,
    /// Potencial livre do recurso do prédio escolhido (por estado).
    Potencial,
    /// Uso / capacidade de infraestrutura (por estado).
    Infraestrutura,
}

/// Estado da interface de construção.
#[derive(Resource)]
pub struct ConstrucaoUi {
    pub aberta: bool,
    /// Índice do tipo de prédio escolhido (no catálogo).
    pub tipo_idx: usize,
    /// Estado destacado na tela de construção.
    pub estado_id: String,
    pub filtro_cat: Option<CategoriaPredio>,
    pub mostrar_bloqueados: bool,
    pub modo_mapa: ModoMapa,
    /// Passo atual do tutorial (None = oculto).
    pub tutorial: Option<usize>,
    /// Pedidos de obra feitos pela interface: (província, tipo).
    pub pedidos: Vec<(String, String)>,
    /// Ações diretas sobre prédios existentes (fechar, reabrir, demolir).
    pub acoes_predios: Vec<Comando>,
    /// Confirmação pendente de demolição: `(provincia_id, tipo_predio_id)`.
    pub confirmar_demolicao: Option<(String, String)>,
    /// Última província do mapa processada (para sincronizar o estado destacado).
    ultima_selecao: u32,
}

impl Default for ConstrucaoUi {
    fn default() -> Self {
        Self {
            aberta: false,
            tipo_idx: 0,
            estado_id: String::new(),
            filtro_cat: None,
            mostrar_bloqueados: false,
            modo_mapa: ModoMapa::Normal,
            tutorial: None,
            pedidos: Vec::new(),
            acoes_predios: Vec::new(),
            confirmar_demolicao: None,
            ultima_selecao: 0,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Lógica pura
// ---------------------------------------------------------------------------------------------

/// Uma opção de prédio para uma província.
#[derive(Debug, Clone)]
pub struct OpcaoConstrucao {
    pub tipo_idx: usize,
    pub tipo_id: String,
    pub nome: String,
    pub categoria: CategoriaPredio,
    pub comporta: bool,
    pub motivo: String,
    pub pontos: f64,
    pub dias: f64,
    pub custo_bens: f64,
    pub itens: Vec<(String, f64)>,
    pub lucro_dia: f64,
    pub detalhe_lucro: String,
    pub governamental: bool,
    pub potencial: String,
    pub na_fila: usize,
}

pub fn nome_categoria(c: CategoriaPredio) -> &'static str {
    match c {
        CategoriaPredio::RuralAgricola => "Rural / Agrícola",
        CategoriaPredio::Extrativo => "Extrativo / Minas",
        CategoriaPredio::Industrial => "Industrial",
        CategoriaPredio::Infraestrutura => "Infraestrutura",
        CategoriaPredio::Construcao => "Construção",
        CategoriaPredio::Governamental => "Governamental",
        CategoriaPredio::Militar => "Militar",
    }
}

const CATEGORIAS: [CategoriaPredio; 7] = [
    CategoriaPredio::RuralAgricola,
    CategoriaPredio::Extrativo,
    CategoriaPredio::Industrial,
    CategoriaPredio::Infraestrutura,
    CategoriaPredio::Construcao,
    CategoriaPredio::Governamental,
    CategoriaPredio::Militar,
];

/// Província da economia (do país do jogador) que corresponde ao `adm1_code` do mapa.
pub fn provincia_por_adm1<'a>(v: &'a VistaEconomia, adm1: &str) -> Option<&'a Provincia> {
    v.provincias.iter().find(|p| p.codigo_externo.as_deref() == Some(adm1))
}

fn estado_de<'a>(v: &'a VistaEconomia, prov: &Provincia) -> Option<&'a Estado> {
    v.estados.iter().find(|e| e.id == prov.estado_id)
}

/// Níveis do tipo já construídos no estado (índice diário do `Estado`, O(1)).
fn niveis_do_tipo_no_estado(v: &VistaEconomia, estado_id: &str, tipo_id: &str) -> u32 {
    let Some(cat) = v.catalogo.as_ref() else { return 0 };
    let Some(ti) = cat.tipos_predio.iter().position(|t| t.id == tipo_id) else { return 0 };
    v.estados.iter().find(|e| e.id == estado_id).map_or(0, |e| e.niveis_do_tipo(ti))
}

fn contexto_lucratividade(v: &VistaEconomia, prov: &Provincia, tipo_idx: usize, t: &TipoPredio) -> ContextoLucratividade {
    let Some(est) = estado_de(v, prov) else { return ContextoLucratividade::default() };
    let populacao = prov.populacao_total();
    let sol = prov.padrao_de_vida_medio();
    let desempregados: f64 = prov.pops.iter().map(|p| p.desempregados()).sum();
    let mao_de_obra_total: f64 = prov.pops.iter().map(|p| p.populacao).sum();
    let salario_base = prov.predios.iter().map(|p| p.salario_base).sum::<f64>() / prov.predios.len().max(1) as f64;
    let niveis = est.niveis_do_tipo(tipo_idx) as f64;
    let (potencial_livre, potencial_total) = if t.usa_terra_aravel {
        if !est.cultura_permitida(&t.id) {
            (Some(0.0), Some(0.0))
        } else {
            (Some(est.terra_aravel_livre() as f64), Some(est.terra_aravel.max(1) as f64))
        }
    } else if let Some(rec) = &t.recurso_natural_requerido {
        let total = est.limite_recurso(rec) as f64;
        (Some((total - niveis).max(0.0)), Some(total))
    } else {
        (None, None)
    };
    ContextoLucratividade {
        salario_base: if salario_base > 0.0 { salario_base } else { 4.0 * (sol / 10.0).clamp(0.55, 2.5) },
        padrao_vida: sol,
        desempregados,
        mao_de_obra_total: mao_de_obra_total.max(populacao),
        infraestrutura_uso: est.infraestrutura_uso,
        infraestrutura_capacidade: est.infraestrutura_capacidade,
        acesso_mercado: est.acesso_mercado,
        qualidade_recurso: est.rendimento_recurso(t),
        potencial_livre,
        potencial_total,
        niveis_iguais: niveis,
        bonus_cadeia: est.bonus_cadeia(tipo_idx),
        urbanizacao: est.urbanizacao,
    }
}

fn estimar_lucro(v: &VistaEconomia, prov: &Provincia, t: &TipoPredio) -> EstimativaLucratividade {
    let Some(cat) = v.catalogo.as_ref() else { return EstimativaLucratividade::default() };
    let Some(ti) = cat.tipos_predio.iter().position(|x| x.id == t.id) else { return EstimativaLucratividade::default() };
    let acesso = estado_de(v, prov).map_or(1.0, |e| e.acesso_mercado);
    let mult = 0.85 + 0.15 * acesso;
    let precos: Vec<f64> = if v.bens.len() == cat.bens.len() && v.bens.iter().zip(&cat.bens).all(|(a, b)| a.id == b.id) {
        v.bens.iter().map(|b| b.preco_nacional * mult).collect()
    } else {
        let pares: Vec<(String, f64)> = v.bens.iter().map(|b| (b.id.clone(), b.preco_nacional * mult)).collect();
        precos_por_indice(cat, &pares)
    };
    let perfil = PerfilPredio::novo(cat, &t.metodos_padrao);
    calcular_com_perfil(t, &perfil, &precos, contexto_lucratividade(v, prov, ti, t))
}

/// Texto do detalhamento do lucro: recurso local, cadeia local e demais fatores.
fn detalhe_lucro(v: &VistaEconomia, prov: &Provincia, t: &TipoPredio, l: EstimativaLucratividade) -> String {
    let est = estado_de(v, prov);
    let recurso = match (&t.recurso_natural_requerido, est) {
        (Some(rec), Some(e)) => {
            let lim = e.limite_recurso(rec);
            if lim == 0 { format!("{rec}: nenhum depósito no estado") } else { format!("{rec} (depósito {lim}, qualidade x{:.2})", l.fator_recurso) }
        }
        _ if t.usa_terra_aravel => format!("terra arável (x{:.2})", l.fator_recurso),
        _ => "não depende".to_string(),
    };
    let cadeia = match (v.catalogo.as_ref(), est) {
        (Some(cat), Some(e)) => {
            let perfil = PerfilPredio::novo(cat, &t.metodos_padrao);
            let locais: Vec<String> = perfil
                .insumos
                .iter()
                .filter(|&&(g, _)| e.producao_local(g) > 1e-6)
                .map(|&(g, _)| cat.bens[g].nome.to_lowercase())
                .collect();
            if perfil.insumos.is_empty() {
                "sem insumos".to_string()
            } else if locais.is_empty() {
                "insumos vêm de fora".to_string()
            } else {
                format!("{} no estado", locais.join(" e "))
            }
        }
        _ => "-".to_string(),
    };
    format!(
        "Recurso local: {recurso}\nCadeia local: +{:.0}% ({cadeia})\nOutros fatores: salário x{:.2}; mão de obra x{:.2}; infra x{:.2}; acesso x{:.2}; concorrência x{:.2}; mercado local x{:.2}",
        (l.fator_cadeia_local - 1.0).max(0.0) * 100.0,
        l.fator_salario, l.fator_mao_de_obra, l.fator_infraestrutura, l.fator_acesso, l.fator_concorrencia, l.fator_mercado_local
    )
}

/// Níveis livres do recurso regional que o prédio usa no estado (`None` = prédio sem recurso regional).
pub fn potencial_livre(v: &VistaEconomia, est: &Estado, t: &TipoPredio) -> Option<u32> {
    if t.usa_terra_aravel {
        if !est.cultura_permitida(&t.id) {
            return Some(0);
        }
        Some(est.terra_aravel_livre())
    } else if let Some(rec) = &t.recurso_natural_requerido {
        let lim = est.limite_recurso(rec);
        Some(lim.saturating_sub(niveis_do_tipo_no_estado(v, &est.id, &t.id)))
    } else {
        None
    }
}

fn texto_potencial(v: &VistaEconomia, est: &Estado, t: &TipoPredio) -> String {
    match potencial_livre(v, est, t) {
        None => "Sem recurso regional (só precisa de espaço urbano)".into(),
        Some(0) if t.usa_terra_aravel && !est.cultura_permitida(&t.id) => "Clima/solo desfavorável (rendimento reduzido)".into(),
        Some(0) if t.usa_terra_aravel => "Sem terra arável livre (rendimento reduzido)".into(),
        Some(0) => format!("Sem jazida de {} (rendimento reduzido)", t.recurso_natural_requerido.as_deref().unwrap_or("recurso").replace('_', " ")),
        Some(n) => {
            let q = est.fator_qualidade_predio(t);
            let b = if q > 1.0 { format!(", qualidade +{:.0}%", (q - 1.0) * 100.0) } else { String::new() };
            if t.usa_terra_aravel {
                format!("{n} níveis de terra arável livres{b}")
            } else {
                format!("{n} níveis de jazida livres{b}")
            }
        }
    }
}

/// Prédios que podem (ou não) ser construídos numa província, ordenados: possíveis primeiro, por lucro esperado.
pub fn opcoes_para_provincia(v: &VistaEconomia, prov: &Provincia) -> Vec<OpcaoConstrucao> {
    let Some(cat) = v.catalogo.as_ref() else { return Vec::new() };
    let est = estado_de(v, prov);
    let pontos_dia = v.pontos_construcao_dia.clamp(1.0, 25.0);
    let fila = v.pais.as_ref().map(|p| &p.fila_construcao);
    let mut out: Vec<OpcaoConstrucao> = cat
        .tipos_predio
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let bloqueio_tec = v.bloqueio_predios.get(&t.id);
            let comporta = bloqueio_tec.is_none() && v.comporta(prov, i);
            let mat = calcular_custo_materiais_obra(cat, t, &v.bens);
            let luc = estimar_lucro(v, prov, t);
            let (potencial, motivo) = match est {
                Some(e) => {
                    let p = texto_potencial(v, e, t);
                    let m = if let Some(tec) = bloqueio_tec {
                        format!("Requer tecnologia: {tec}")
                    } else if !comporta {
                        "Porto requer província costeira".into()
                    } else {
                        String::new()
                    };
                    (p, m)
                }
                None => ("-".into(), String::new()),
            };
            OpcaoConstrucao {
                tipo_idx: i,
                tipo_id: t.id.clone(),
                nome: t.nome.clone(),
                categoria: t.categoria,
                comporta,
                motivo,
                pontos: t.custo_pontos_construcao,
                dias: t.custo_pontos_construcao / pontos_dia,
                custo_bens: mat.total_reais,
                itens: mat.itens.iter().map(|(n, q, _, _)| (n.clone(), *q)).collect(),
                lucro_dia: if t.financiado_pelo_governo { -(luc.custo_insumos + luc.custo_folha) } else { luc.lucro_liquido },
                detalhe_lucro: detalhe_lucro(v, prov, t, luc),
                governamental: t.financiado_pelo_governo,
                potencial,
                na_fila: fila.map_or(0, |f| f.iter().filter(|it| it.provincia_id == prov.id && it.tipo_predio_id == t.id).count()),
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.comporta
            .cmp(&a.comporta)
            .then(a.governamental.cmp(&b.governamental))
            .then(b.lucro_dia.total_cmp(&a.lucro_dia))
    });
    out
}

/// Linha do ranking de estados para um tipo de prédio.
#[derive(Debug, Clone)]
pub struct LinhaEstado {
    pub estado_id: String,
    pub nome: String,
    pub livre: Option<u32>,
    pub qualidade: f64,
    pub infra_uso: f64,
    pub infra_cap: f64,
    pub penalidade: f64,
    pub lucro_dia: f64,
    pub vantagem: f64,
    /// Primeira província do estado onde cabe +1 nível.
    pub provincia_alvo: Option<String>,
}

/// Estados do jogador ordenados pela vantagem de construir o tipo (potencial livre x qualidade x folga de infra).
pub fn ranking_estados(v: &VistaEconomia, tipo_idx: usize) -> Vec<LinhaEstado> {
    let Some(cat) = v.catalogo.as_ref() else { return Vec::new() };
    let Some(t) = cat.tipos_predio.get(tipo_idx) else { return Vec::new() };
    let mut linhas: Vec<LinhaEstado> = v
        .estados
        .iter()
        .map(|e| {
            let livre = potencial_livre(v, e, t);
            // Província sugerida: a que cabe +1 nível e dá o maior lucro esperado.
            let melhor = v
                .provincias
                .iter()
                .filter(|p| p.estado_id == e.id && v.comporta(p, tipo_idx))
                .map(|p| {
                    let l = estimar_lucro(v, p, t);
                    let ls = if t.financiado_pelo_governo { -(l.custo_insumos + l.custo_folha) } else { l.lucro_liquido };
                    (ls, p, l)
                })
                .max_by(|a, b| a.0.total_cmp(&b.0));
            let alvo = melhor.as_ref().map(|(_, p, _)| p.id.clone());
            let luc = melhor.map_or_else(EstimativaLucratividade::default, |(_, _, l)| l);
            let lucro = if t.financiado_pelo_governo { -(luc.custo_insumos + luc.custo_folha) } else { luc.lucro_liquido };
            let pot = livre.map_or(1.0, |n| if n > 0 { 1.0 + (n as f64).ln_1p() } else { 0.25 });
            let folga = (1.0 - e.penalidade_infraestrutura).clamp(0.05, 1.0);
            let vantagem = if alvo.is_some() { luc.fator_recurso * folga * pot * if lucro > 0.0 { 1.0 + lucro.max(0.0).ln_1p() } else { 0.5 } } else { 0.0 };
            LinhaEstado {
                estado_id: e.id.clone(),
                nome: e.nome.clone(),
                livre,
                qualidade: luc.fator_recurso,
                infra_uso: e.infraestrutura_uso,
                infra_cap: e.infraestrutura_capacidade,
                penalidade: e.penalidade_infraestrutura,
                lucro_dia: lucro,
                vantagem,
                provincia_alvo: alvo,
            }
        })
        .collect();
    linhas.sort_by(|a, b| b.vantagem.total_cmp(&a.vantagem).then(a.nome.cmp(&b.nome)));
    linhas
}

/// Cor RGBA do mapa para um estado no modo escolhido. `max_livre` normaliza o modo Potencial.
pub fn cor_modo(modo: ModoMapa, livre: Option<u32>, max_livre: u32, est: &Estado) -> [u8; 4] {
    match modo {
        ModoMapa::Normal => [128, 128, 128, 255],
        ModoMapa::Potencial => match livre {
            Some(0) => [120, 55, 55, 255],
            Some(n) => {
                let f = (n as f32 / max_livre.max(1) as f32).clamp(0.0, 1.0).sqrt();
                [(70.0 - 40.0 * f) as u8, (110.0 + 130.0 * f) as u8, (70.0 - 30.0 * f) as u8, 255]
            }
            None => [90, 90, 110, 255],
        },
        ModoMapa::Infraestrutura => {
            let u = est.taxa_utilizacao_infraestrutura() as f32;
            if u > 1.0 {
                [220, 50, 50, 255]
            } else if u > 0.7 {
                let f = (u - 0.7) / 0.3;
                [(120.0 + 100.0 * f) as u8, (200.0 - 90.0 * f) as u8, 60, 255]
            } else {
                [60, (200.0 - 80.0 * (u / 0.7)) as u8, 80, 255]
            }
        }
    }
}

/// Mapa `adm1_code -> cor` para repintar o mapa; províncias sem estado do jogador ficam cinza escuro.
pub fn cores_por_adm1(v: &VistaEconomia, modo: ModoMapa, tipo_idx: usize) -> HashMap<String, [u8; 4]> {
    let mut out = HashMap::new();
    let Some(cat) = v.catalogo.as_ref() else { return out };
    let Some(t) = cat.tipos_predio.get(tipo_idx) else { return out };
    let livres: HashMap<&str, Option<u32>> = v.estados.iter().map(|e| (e.id.as_str(), potencial_livre(v, e, t))).collect();
    let max_livre = livres.values().filter_map(|x| *x).max().unwrap_or(1);
    for p in &v.provincias {
        let (Some(code), Some(e)) = (p.codigo_externo.as_ref(), estado_de(v, p)) else { continue };
        let livre = livres.get(e.id.as_str()).copied().flatten();
        out.insert(code.clone(), cor_modo(modo, livre, max_livre, e));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Tutorial
// ---------------------------------------------------------------------------------------------

pub const PASSOS_TUTORIAL: [(&str, &str); 5] = [
    ("1/5  Como construir", "Aperte B (ou o botão \"Construir\" no topo) para abrir a tela de construção. Clique numa província do seu país e use a seção \"Construir aqui\" do painel da direita: escolha o prédio e aperte \"Construir\"."),
    ("2/5  Recursos regionais", "Qualquer prédio pode ser erguido em qualquer província, mas os recursos locais definem o rendimento: terras férteis e jazidas ricas rendem muito mais, enquanto regiões sem recursos rendem pouco e podem dar prejuízo."),
    ("3/5  Custo e lucro", "Cada obra gasta bens (aço, madeira, ferramentas...) e pontos de construção. O lucro esperado por dia ajuda a escolher. Prédios do governo dão serviço, não lucro."),
    ("4/5  Infraestrutura", "Prédios usam infraestrutura do estado (ferrovias, portos, urbanização). Passando da capacidade, a produção cai. Veja o modo de mapa \"Infraestrutura\" na tela de construção."),
    ("5/5  Setor privado e fila", "Capitalistas também investem sozinhos, conforme a lei econômica, usando o fundo privado. A fila de obras (na tela B) mostra o progresso de cada obra, estatal ou privada."),
];


// ---------------------------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------------------------

fn cor_lucro(v: f64) -> egui::Color32 {
    if v >= 0.0 { ui_refinada::menu::SUCESSO } else { ui_refinada::menu::PERIGO }
}

/// Calcula `(rotulo_tendencia, cor_tendencia, custo_reabrir, reembolso_demolir_1nv)` para um prédio existente.
pub fn resumo_gestao_predio(
    v: &VistaEconomia,
    b: &sim_economia::Predio,
) -> (String, egui::Color32, f64, f64) {
    let Some(cat) = v.catalogo.as_ref() else {
        return ("→ estável".into(), ui_refinada::menu::TINTA_SUAVE, 0.0, 0.0);
    };
    let Some(tipo) = cat.obter_tipo_predio(&b.tipo_predio_id) else {
        return ("→ estável".into(), ui_refinada::menu::TINTA_SUAVE, 0.0, 0.0);
    };
    let preco_bem = |id: &str| {
        v.bens
            .iter()
            .find(|x| x.id == id)
            .map_or(20.0, |x| x.preco_nacional)
    };
    let custo_reabrir =
        sim_economia::calcular_custo_reabertura_por_precos(cat, tipo, b.nivel, preco_bem);
    let reembolso_demolir =
        sim_economia::calcular_reembolso_demolicao_por_precos(cat, tipo, 1, preco_bem);

    let var30 = b
        .produtos_por_nivel()
        .first()
        .and_then(|&(idx, _)| v.bens.get(idx as usize))
        .map_or(0.0, |bem| bem.var30_nacional);

    let (seta, cor) = if b.tendencia_lucro > 3.0 || var30 > 0.04 {
        ("↗ em alta", ui_refinada::menu::SUCESSO)
    } else if b.tendencia_lucro < -3.0 || var30 < -0.04 || b.dias_prejuizo >= 15 {
        ("↘ em queda", ui_refinada::menu::PERIGO)
    } else {
        ("→ estável", ui_refinada::menu::TINTA_SUAVE)
    };
    let rotulo = if var30.abs() >= 0.005 {
        format!("{seta} ({:+.0}%/30d)", var30 * 100.0)
    } else {
        seta.to_string()
    };
    (rotulo, cor, custo_reabrir, reembolso_demolir)
}

/// Desenha os controles de Fechar / Reabrir e Demolir (com confirmação) para um prédio instalado.
pub fn botoes_gestao_predio_inline(
    ui: &mut egui::Ui,
    v: &VistaEconomia,
    provincia_id: &str,
    b: &sim_economia::Predio,
    confirmar_demolicao: &mut Option<(String, String)>,
    acoes: &mut Vec<Comando>,
) {
    let (rotulo_tend, cor_tend, custo_reabrir, reembolso_demolir) = resumo_gestao_predio(v, b);
    ui.colored_label(cor_tend, &rotulo_tend)
        .on_hover_text("Tendência recente de rentabilidade do prédio e do preço de venda do produto");

    if b.fechado {
        let txt_reabrir = format!("Reabrir ({})", fmt_din(custo_reabrir));
        if ui
            .small_button(
                egui::RichText::new(txt_reabrir)
                    .color(ui_refinada::menu::SUCESSO)
                    .strong(),
            )
            .on_hover_text(format!(
                "Reabre o prédio pagando {} de remobilização. A produção retoma gradualmente em {} dias.",
                fmt_din(custo_reabrir),
                sim_economia::Predio::TEMPO_RETOMADA_DIAS
            ))
            .clicked()
        {
            acoes.push(Comando::ReabrirPredio {
                provincia_id: provincia_id.to_string(),
                tipo_id: b.tipo_predio_id.clone(),
            });
        }
    } else if ui
        .small_button(egui::RichText::new("Fechar").color(ui_refinada::menu::AVISO))
        .on_hover_text(format!(
            "Fecha temporariamente: cessa produção e consumo de insumos, dispensa {:.0} trabalhadores e mantém custo de conservação baixo (${:.0}/dia).",
            b.total_empregados(),
            (b.nivel as f64) * sim_economia::CUSTO_MANUTENCAO_FECHADO_POR_NIVEL
        ))
        .clicked()
    {
        acoes.push(Comando::FecharPredio {
            provincia_id: provincia_id.to_string(),
            tipo_id: b.tipo_predio_id.clone(),
        });
    }

    let chave = (provincia_id.to_string(), b.tipo_predio_id.clone());
    if confirmar_demolicao.as_ref() == Some(&chave) {
        ui.colored_label(ui_refinada::menu::PERIGO, "Confirmar?");
        if ui
            .small_button(
                egui::RichText::new(format!("Demolir (+{})", fmt_din(reembolso_demolir)))
                    .color(ui_refinada::menu::PERIGO)
                    .strong(),
            )
            .on_hover_text(
                "Confirma a demolição de 1 nível, liberando espaço regional e recuperando parte dos materiais.",
            )
            .clicked()
        {
            acoes.push(Comando::DemolirPredio {
                provincia_id: provincia_id.to_string(),
                tipo_id: b.tipo_predio_id.clone(),
                niveis: 1,
            });
            *confirmar_demolicao = None;
        }
        if ui.small_button("Cancelar").clicked() {
            *confirmar_demolicao = None;
        }
    } else if ui
        .small_button(egui::RichText::new("Demolir (-1 nv)").color(ui_refinada::menu::PERIGO))
        .on_hover_text(format!(
            "Reduz 1 nível do prédio, libera capacidade regional e devolve {} em materiais recuperados.",
            fmt_din(reembolso_demolir)
        ))
        .clicked()
    {
        *confirmar_demolicao = Some(chave);
    }
}

/// Seção "Construir aqui" do painel de província/estado. `adm1` é o código da província clicada no mapa.
pub fn secao_construir_aqui(ui: &mut egui::Ui, v: &VistaEconomia, adm1: &str, c: &mut ConstrucaoUi) {
    ui.add_space(6.0);
    ui_refinada::titulo_secao(ui, "Construir aqui");
    if v.catalogo.is_none() {
        ui.label("A economia ainda está calculando...");
        return;
    }
    let Some(prov) = provincia_por_adm1(v, adm1) else {
        ui.label("Esta província não é do seu país: só é possível construir no seu território.");
        return;
    };
    if let Some(e) = estado_de(v, prov) {
        ui.label(format!("Estado: {} | Província: {}", e.nome, prov.nome));
        let pct = if e.infraestrutura_capacidade > 1e-6 {
            (e.infraestrutura_uso / e.infraestrutura_capacidade).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        let cor_infra = if e.penalidade_infraestrutura > 0.0 { ui_refinada::menu::PERIGO } else { ui_refinada::menu::SUCESSO };
        ui_refinada::barra_status(
            ui,
            Some("Infraestrutura"),
            pct,
            1.0,
            cor_infra,
            170.0,
            Some(&format!(
                "{:.0}/{:.0}{}",
                e.infraestrutura_uso,
                e.infraestrutura_capacidade,
                if e.penalidade_infraestrutura > 0.0 {
                    format!(" · penalidade -{:.0}%", e.penalidade_infraestrutura * 100.0)
                } else {
                    String::new()
                }
            )),
        );
        ui.small(format!("Terra arável livre: {}", e.terra_aravel_livre()));
    }

    // Prédios já instalados nesta província: lucro atual, tendência, Fechar/Reabrir e Demolir com confirmação
    if let Some(cat) = v.catalogo.as_ref()
        && prov.predios.iter().any(|b| b.nivel > 0)
    {
        ui.add_space(4.0);
        ui_refinada::titulo_secao(ui, "Prédios instalados nesta província");
        egui::ScrollArea::vertical()
            .id_salt("scroll_instalados_prov")
            .max_height(190.0)
            .show(ui, |ui| {
                for b in prov.predios.iter().filter(|b| b.nivel > 0) {
                    let nome_b = cat
                        .obter_tipo_predio(&b.tipo_predio_id)
                        .map_or(b.tipo_predio_id.as_str(), |t| t.nome.as_str());
                    let eh_gov = cat
                        .obter_tipo_predio(&b.tipo_predio_id)
                        .is_some_and(|t| t.financiado_pelo_governo);
                    ui_refinada::moldura_cartao().show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui_refinada::glifo_ui(ui, ui_refinada::chave_para(&b.tipo_predio_id), 14.0, None);
                            ui.label(egui::RichText::new(format!("{nome_b} (Nv. {})", b.nivel)).strong());
                            if b.fechado {
                                ui.colored_label(
                                    ui_refinada::menu::PERIGO,
                                    format!("FECHADO ({}d)", b.dias_fechado),
                                );
                            } else if b.esta_em_retomada() {
                                ui.colored_label(
                                    ui_refinada::menu::AVISO,
                                    format!(
                                        "Retomando ({}d · {:.0}%)",
                                        b.dias_retomada,
                                        b.fator_retomada() * 100.0
                                    ),
                                );
                            } else {
                                ui.weak(format!(
                                    "{:.0}/{:.0} trab.",
                                    b.total_empregados(),
                                    b.total_vagas()
                                ));
                            }
                            if eh_gov {
                                ui.colored_label(
                                    ui_refinada::menu::INFO,
                                    format!("Custo {}/d", fmt_din(b.folha_salarial_ultimo_tick + b.custo_insumos_ultimo_tick)),
                                );
                            } else {
                                ui.colored_label(
                                    cor_lucro(b.lucro_ultimo_tick),
                                    format!("{}/d", fmt_din(b.lucro_ultimo_tick)),
                                );
                                if b.dias_prejuizo > 0 && !b.fechado {
                                    ui.colored_label(
                                        ui_refinada::menu::PERIGO,
                                        format!("({}d prejuízo)", b.dias_prejuizo),
                                    );
                                }
                            }
                        });
                        ui.horizontal_wrapped(|ui| {
                            botoes_gestao_predio_inline(
                                ui,
                                v,
                                &prov.id,
                                b,
                                &mut c.confirmar_demolicao,
                                &mut c.acoes_predios,
                            );
                        });
                    });
                }
            });
        ui.add_space(4.0);
    }
    ui.horizontal_wrapped(|ui| {
        if ui_refinada::aba_glifo(ui, c.filtro_cat.is_none(), Some("lupa"), "Todas") {
            c.filtro_cat = None;
        }
        for cp in CATEGORIAS {
            if ui_refinada::aba_glifo(ui, c.filtro_cat == Some(cp), Some(ui_refinada::chave_para(nome_categoria(cp).to_lowercase().as_str())), nome_categoria(cp)) {
                c.filtro_cat = Some(cp);
            }
        }
    });
    ui.checkbox(&mut c.mostrar_bloqueados, "Mostrar também os que não cabem");
    let opcoes = opcoes_para_provincia(v, prov);
    let filtro = c.filtro_cat;
    let mut mostradas = 0;
    egui::ScrollArea::vertical().id_salt("scroll_construir_aqui").max_height(380.0).show(ui, |ui| {
        for o in opcoes.iter().filter(|o| filtro.is_none_or(|f| o.categoria == f)) {
            if !o.comporta && !c.mostrar_bloqueados {
                continue;
            }
            mostradas += 1;
            ui_refinada::moldura_cartao()
                .stroke(egui::Stroke::new(
                    1.0,
                    if o.comporta { ui_refinada::menu::PAPEL_BORDA } else { ui_refinada::menu::PERIGO },
                ))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui_refinada::glifo_ui(ui, ui_refinada::chave_para(&o.tipo_id), 16.0, None);
                        ui.label(
                            egui::RichText::new(&o.nome)
                                .family(ui_refinada::familia_oswald(ui.ctx()))
                                .size(15.0)
                                .strong()
                                .color(ui_refinada::menu::AZUL_MARINHO),
                        );
                        ui.weak(nome_categoria(o.categoria));
                        if o.na_fila > 0 {
                            ui.colored_label(ui_refinada::menu::AVISO, format!("({} na fila)", o.na_fila));
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui_refinada::chapa(ui, "picareta", "Pontos", &format!("{:.0}", o.pontos), None);
                        ui_refinada::chapa(ui, "relogio", "Prazo", &format!("~{:.0} dias", o.dias), None);
                        ui_refinada::chapa(ui, "caixa", "Custo em bens", &fmt_din(o.custo_bens), None);
                        ui_refinada::chapa(ui, "grafico", "Potencial", &o.potencial.to_string(), None);
                    });
                    if !o.itens.is_empty() {
                        let bens: Vec<String> = o.itens.iter().map(|(n, q)| format!("{q:.0}x {n}")).collect();
                        ui.small(format!("Bens: {}", bens.join(", ")));
                    }
                    ui.horizontal(|ui| {
                        if o.governamental {
                            ui.label(format!("Serviço público, custo ao tesouro: {}/dia", fmt_din(-o.lucro_dia)));
                        } else {
                            ui.label("Lucro esperado:");
                            ui.colored_label(cor_lucro(o.lucro_dia), format!("{}/dia por nível", fmt_din(o.lucro_dia))).on_hover_text(&o.detalhe_lucro);
                        }
                    });
                    ui.horizontal(|ui| {
                        let btn = if o.comporta {
                            ui_refinada::botao_estilo_menu(ui, "Construir")
                        } else {
                            ui.add_enabled(false, egui::Button::new("Não cabe"))
                        };
                        if btn.on_disabled_hover_text(&o.motivo).clicked() {
                            c.pedidos.push((prov.id.clone(), o.tipo_id.clone()));
                        }
                        if !o.comporta {
                            ui.colored_label(ui_refinada::menu::PERIGO, &o.motivo);
                        }
                    });
                });
        }
        if mostradas == 0 {
            ui.label("Nenhum prédio dessa categoria cabe aqui. Veja o potencial regional na tela de construção (B).");
        }
    });
}

fn tela_construcao(mut contextos: EguiContexts, mut c: ResMut<ConstrucaoUi>, eco: Option<Res<EconomiaSim>>) -> Result {
    if !c.aberta {
        return Ok(());
    }
    let Some(eco) = eco else { return Ok(()) };
    let ctx = contextos.ctx_mut()?;
    let mut aberta = true;
    ui_refinada::janela_dossie("CONSTRUÇÃO (B)", None)
        .default_pos([20.0, 96.0])
        .default_size([980.0, 660.0])
        .show(ctx, |ui| {
            ui_refinada::faixa_titulo(ui, "Construção e Edificação", Some("B"), Some(&mut aberta));
            corpo_tela_construcao(ui, &eco.vista, &mut c);
        });
    c.aberta = aberta;
    Ok(())
}

/// Conteúdo da tela de construção (separado para testes).
pub fn corpo_tela_construcao(ui: &mut egui::Ui, v: &VistaEconomia, c: &mut ConstrucaoUi) {
    let (Some(cat), Some(pais)) = (v.catalogo.as_ref(), v.pais.as_ref()) else {
        ui.label("A economia ainda está calculando...");
        return;
    };
    if c.tipo_idx >= cat.tipos_predio.len() {
        c.tipo_idx = 0;
    }
    ui.horizontal_wrapped(|ui| {
        ui_refinada::chapa(ui, "picareta", "Capacidade", &format!("{:.0} pts/dia", v.pontos_construcao_dia), Some(ui_refinada::menu::OURO));
        ui.label(egui::RichText::new("MODO DO MAPA:").size(12.0).strong().color(ui_refinada::menu::TINTA_SUAVE));
        if ui_refinada::aba_glifo(ui, c.modo_mapa == ModoMapa::Normal, Some("caixa"), "Normal") { c.modo_mapa = ModoMapa::Normal; }
        if ui_refinada::aba_glifo(ui, c.modo_mapa == ModoMapa::Potencial, Some("lupa"), "Potencial") { c.modo_mapa = ModoMapa::Potencial; }
        if ui_refinada::aba_glifo(ui, c.modo_mapa == ModoMapa::Infraestrutura, Some("trem"), "Infraestrutura") { c.modo_mapa = ModoMapa::Infraestrutura; }
        if ui_refinada::botao_estilo_secundario(ui, "Ver tutorial").clicked() {
            c.tutorial = Some(0);
        }
    });
    match c.modo_mapa {
        ModoMapa::Potencial => {
            ui.weak("Mapa: verde = muito potencial livre do prédio escolhido; vermelho escuro = nenhum; cinza = fora do seu país.");
        }
        ModoMapa::Infraestrutura => {
            ui.weak("Mapa: verde = folga de infraestrutura; amarelo = quase cheia; vermelho = sobrecarga (produção cai).");
        }
        ModoMapa::Normal => {}
    }
    ui.add_space(4.0);
    ui_refinada::linha_decorativa(ui);
    ui.columns(2, |cols| {
        // Coluna esquerda: categoria e tipo
        let ui = &mut cols[0];
        ui_refinada::titulo_secao(ui, "1 · Escolha o prédio");
        ui.horizontal_wrapped(|ui| {
            if ui_refinada::aba_glifo(ui, c.filtro_cat.is_none(), Some("lupa"), "Todas") {
                c.filtro_cat = None;
            }
            for cp in CATEGORIAS {
                if ui_refinada::aba_glifo(ui, c.filtro_cat == Some(cp), Some(ui_refinada::chave_para(nome_categoria(cp).to_lowercase().as_str())), nome_categoria(cp)) {
                    c.filtro_cat = Some(cp);
                }
            }
        });
        egui::ScrollArea::vertical().id_salt("scroll_tipos").max_height(230.0).show(ui, |ui| {
            for (i, t) in cat.tipos_predio.iter().enumerate() {
                if c.filtro_cat.is_some_and(|f| f != t.categoria) {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui_refinada::glifo_ui(ui, ui_refinada::chave_para(&t.id), 14.0, None);
                    if ui.selectable_label(c.tipo_idx == i, &t.nome).clicked() {
                        c.tipo_idx = i;
                    }
                });
            }
        });
        let t = &cat.tipos_predio[c.tipo_idx];
        let mat = calcular_custo_materiais_obra(cat, t, &v.bens);
        let luc = v.provincias.iter()
            .find(|p| p.estado_id == c.estado_id && v.comporta(p, c.tipo_idx))
            .map_or_else(|| EstimativaLucratividade::default(), |p| estimar_lucro(v, p, t));
        ui.add_space(6.0);
        ui_refinada::moldura_cartao().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui_refinada::glifo_ui(ui, ui_refinada::chave_para(&t.id), 18.0, None);
                ui.label(
                    egui::RichText::new(&t.nome)
                        .family(ui_refinada::familia_oswald(ui.ctx()))
                        .size(17.0)
                        .strong()
                        .color(ui_refinada::menu::AZUL_MARINHO),
                );
            });
            ui_refinada::linha_decorativa(ui);
            ui.horizontal(|ui| {
                ui_refinada::chapa(ui, "caixa", "Custo em bens", &fmt_din(mat.total_reais), None);
                ui_refinada::chapa(ui, "picareta", "Pontos", &format!("{:.0}", t.custo_pontos_construcao), None);
                ui_refinada::chapa(ui, "relogio", "Prazo", &format!("~{:.0} dias", t.custo_pontos_construcao / v.pontos_construcao_dia.clamp(1.0, 25.0)), None);
            });
            if t.financiado_pelo_governo {
                ui.colored_label(ui_refinada::menu::INFO, format!("Serviço público: custa {}/dia ao tesouro", fmt_din(luc.custo_insumos + luc.custo_folha)));
            } else {
                ui.horizontal(|ui| {
                    ui.label("Lucro esperado (por nível, base):");
                    ui.colored_label(cor_lucro(luc.lucro_liquido), format!("{}/dia", fmt_din(luc.lucro_liquido)));
                });
            }
        });

        // Coluna direita: ranking de estados
        let ui = &mut cols[1];
        let ranking = ranking_estados(v, c.tipo_idx);
        ui_refinada::titulo_secao(ui, "2 · Estados com mais vantagem");
        egui::ScrollArea::vertical().id_salt("scroll_estados").max_height(300.0).show(ui, |ui| {
            egui::Grid::new("grid_ranking_estados").striped(true).spacing([10.0, 3.0]).show(ui, |ui| {
                for h in ["Estado", "Potencial", "Infra", "Infra %", "Lucro/dia", ""] {
                    ui.label(egui::RichText::new(h.to_uppercase()).size(12.0).strong().color(ui_refinada::menu::TINTA_SUAVE));
                }
                ui.end_row();
                for l in ranking.iter().take(60) {
                    let sel = c.estado_id == l.estado_id;
                    if ui.selectable_label(sel, &l.nome).clicked() {
                        c.estado_id = l.estado_id.clone();
                    }
                    match l.livre {
                        Some(n) => ui.label(format!("{n} livres (x{:.2})", l.qualidade)),
                        None => ui.weak("urbano"),
                    };
                    let pct = if l.infra_cap > 1e-6 { l.infra_uso / l.infra_cap * 100.0 } else { 0.0 };
                    let cor = if l.penalidade > 0.0 { ui_refinada::menu::PERIGO } else { ui_refinada::menu::SUCESSO };
                    ui_refinada::barra_trilho(ui, (pct / 100.0) as f32, cor, 54.0, 9.0);
                    ui.colored_label(cor, format!("{pct:.0}%"));
                    ui.colored_label(cor_lucro(l.lucro_dia), fmt_din(l.lucro_dia));
                    let ok = l.provincia_alvo.is_some();
                    if ui.add_enabled(ok, egui::Button::new("Construir")).on_disabled_hover_text("Não cabe neste estado").clicked() {
                        if let Some(pid) = &l.provincia_alvo {
                            c.pedidos.push((pid.clone(), t.id.clone()));
                        }
                    }
                    ui.end_row();
                }
            });
        });
    });
    ui.add_space(6.0);
    ui_refinada::titulo_secao(ui, &format!("3 · Fila de obras ({})", pais.fila_construcao.len()));
    egui::ScrollArea::vertical().id_salt("scroll_fila").max_height(180.0).show(ui, |ui| {
        if pais.fila_construcao.is_empty() {
            ui.weak("Nenhuma obra em andamento.");
        }
        for it in &pais.fila_construcao {
            let nome_t = cat.tipos_predio.iter().find(|t| t.id == it.tipo_predio_id).map_or(it.tipo_predio_id.as_str(), |t| t.nome.as_str());
            let nome_p = v.provincias.iter().find(|p| p.id == it.provincia_id).map_or(it.provincia_id.as_str(), |p| p.nome.as_str());
            let frac = (it.progresso_pontos / it.custo_total_pontos.max(1.0)).clamp(0.0, 1.0) as f32;
            ui_refinada::barra_status(
                ui,
                Some(if it.privado { "privada" } else { "estatal" }),
                frac,
                1.0,
                ui_refinada::menu::AZUL_MARINHO,
                160.0,
                Some(&format!("{nome_t} em {nome_p} · {:.0}/{:.0} pts", it.progresso_pontos, it.custo_total_pontos)),
            );
        }
    });
}

fn janela_tutorial(mut contextos: EguiContexts, mut c: ResMut<ConstrucaoUi>) -> Result {
    let Some(passo) = c.tutorial else { return Ok(()) };
    let ctx = contextos.ctx_mut()?;
    let (titulo, texto) = PASSOS_TUTORIAL[passo.min(PASSOS_TUTORIAL.len() - 1)];
    let mut novo = Some(passo);
    ui_refinada::janela_estilo_menu("TUTORIAL: CONSTRUÇÃO")
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(440.0)
        .show(ctx, |ui| {
            ui_refinada::decorar_janela(ui);
            ui_refinada::cabecalho_painel(ui, titulo);
            ui.label(texto);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(passo > 0, egui::Button::new("Voltar")).clicked() {
                    novo = Some(passo - 1);
                }
                if passo + 1 < PASSOS_TUTORIAL.len() {
                    if ui_refinada::botao_estilo_menu(ui, "Próximo").clicked() {
                        novo = Some(passo + 1);
                    }
                } else if ui_refinada::botao_estilo_menu(ui, "Entendi").clicked() {
                    novo = None;
                }
                if ui_refinada::botao_estilo_secundario(ui, "Pular").clicked() {
                    novo = None;
                }
            });
        });
    c.tutorial = novo;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Sistemas
// ---------------------------------------------------------------------------------------------

fn tecla_b(teclas: Res<ButtonInput<KeyCode>>, mut c: ResMut<ConstrucaoUi>, mut egui: EguiContexts) {
    if teclas.just_pressed(KeyCode::KeyB) && !egui.ctx_mut().is_ok_and(|ctx| ctx.egui_wants_keyboard_input()) {
        c.aberta = !c.aberta;
    }
}

/// Converte os pedidos da interface em comandos para a economia.
pub fn aplicar_pedidos(mut c: ResMut<ConstrucaoUi>, mut eco: ResMut<EconomiaSim>) {
    for (provincia_id, tipo_id) in std::mem::take(&mut c.pedidos) {
        eco.enviar(Comando::Construir { provincia_id, tipo_id });
    }
    for cmd in std::mem::take(&mut c.acoes_predios) {
        eco.enviar(cmd);
    }
}

/// Com a tela aberta, clicar numa província do mapa destaca o estado dela.
fn sincronizar_estado_clicado(
    mut c: ResMut<ConstrucaoUi>,
    sel: Res<ProvinciaSelecionada>,
    cat_mapa: Res<RecursoCatalogo>,
    eco: Res<EconomiaSim>,
) {
    if c.ultima_selecao == sel.0 {
        return;
    }
    c.ultima_selecao = sel.0;
    let Some(p) = cat_mapa.0.obter_por_id(sel.0) else { return };
    if let Some(pe) = provincia_por_adm1(&eco.vista, &p.adm1_code) {
        c.estado_id = pe.estado_id.clone();
    }
}

/// Repinta a textura do mapa quando muda o modo, o prédio escolhido ou as obras.
fn repintar_mapa(
    c: Res<ConstrucaoUi>,
    eco: Res<EconomiaSim>,
    cores: Option<Res<CoresMapa>>,
    dados: Option<Res<DadosMapa>>,
    imagem: Option<Res<ImagemMapa>>,
    catalogo: Res<RecursoCatalogo>,
    mut imagens: ResMut<Assets<Image>>,
    mut ultimo: Local<Option<(ModoMapa, usize, u64, String)>>,
    tempo: Res<Time<Real>>,
    mut ultima_pintura: Local<f32>,
    tematico: Option<Res<super::hud_mapas::ModoTematico>>,
) {
    // Um modo temático (população, PIB...) ativo é dono da textura do mapa.
    if tematico.is_some_and(|t| *t != super::hud_mapas::ModoTematico::Politico) {
        return;
    }
    let (Some(cores), Some(dados), Some(imagem)) = (cores, dados, imagem) else { return };
    // No modo Normal as obras concluídas não mudam as cores: não repinta a textura inteira a cada obra.
    let versao = if c.modo_mapa == ModoMapa::Normal { 0 } else { eco.versao_construcao };
    let chave = (c.modo_mapa, c.tipo_idx, versao, eco.vista.pais_id.clone());
    if ultimo.as_ref() == Some(&chave) || eco.vista.catalogo.is_none() {
        return;
    }
    // Mudança só de versão (obra concluída) nos outros modos: adia (debounce de 3 s) a repintura pesada.
    let so_versao = ultimo.as_ref().is_some_and(|u| (u.0, u.1, &u.3) == (chave.0, chave.1, &chave.3));
    if so_versao && tempo.elapsed_secs() - *ultima_pintura < 3.0 {
        return;
    }
    *ultima_pintura = tempo.elapsed_secs();
    let primeira = ultimo.is_none();
    *ultimo = Some(chave);
    if primeira && c.modo_mapa == ModoMapa::Normal {
        return;
    }
    let mut cor_prov = cores.cor_prov.clone();
    if c.modo_mapa != ModoMapa::Normal {
        let por_adm1 = cores_por_adm1(&eco.vista, c.modo_mapa, c.tipo_idx);
        for p in &catalogo.0.provincias {
            if let Some(slot) = cor_prov.get_mut(p.id as usize) {
                *slot = por_adm1.get(&p.adm1_code).copied().unwrap_or([55, 55, 58, 255]);
            }
        }
    }
    let _ = dados;
    pintar_paleta(&mut imagens, &imagem, &cor_prov, &cores.dono_prov, &cores.estado_prov, None);
}

pub struct ConstrucaoUiPlugin;

impl Plugin for ConstrucaoUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConstrucaoUi>()
            .add_systems(
                Update,
                (tecla_b, aplicar_pedidos.run_if(resource_exists::<EconomiaSim>), sincronizar_estado_clicado.run_if(resource_exists::<EconomiaSim>), repintar_mapa.run_if(resource_exists::<EconomiaSim>))
                    .run_if(crate::estado::em_partida),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (tela_construcao, janela_tutorial).run_if(crate::estado::em_partida),
            );
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::estado::AppState;
    use crate::plugins::economia_sim::EconomiaSimPlugin;
    use crate::plugins::time::EventoTickDiario;
    use crate::plugins::jogador::PaisJogador;
    use crate::plugins::map::RaizProjeto;
    use bevy::state::app::StatesPlugin;
    use gf_mapa::tempo::DataSimulacao;
    use std::path::Path;
    use std::time::Instant;

    fn app_headless() -> App {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin))
            .init_state::<AppState>()
            .add_message::<EventoTickDiario>()
            .insert_resource(RaizProjeto(raiz))
            .add_plugins(EconomiaSimPlugin);
        app.update();
        let t = Instant::now();
        while app.world().get_resource::<EconomiaSim>().is_none() {
            assert!(t.elapsed().as_secs() < 180, "carga demorou demais");
            std::thread::sleep(std::time::Duration::from_millis(50));
            app.update();
        }
        app.world_mut().insert_resource(PaisJogador("USA".into()));
        let t = Instant::now();
        loop {
            app.update();
            if app.world().resource::<EconomiaSim>().vista.pais.is_some() {
                break;
            }
            assert!(t.elapsed().as_secs() < 60, "vista não montada");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        app
    }

    /// Fluxo completo: província clicada -> opções -> pedido -> comando -> obra na fila.
    #[test]
    fn clicar_provincia_lista_opcoes_e_constroi() {
        let mut app = app_headless();
        app.init_resource::<ConstrucaoUi>().add_systems(Update, aplicar_pedidos);
        let (adm1, tipo_id, prov_id, n_fila) = {
            let v = &app.world().resource::<EconomiaSim>().vista;
            // Província do jogador onde cabe pelo menos um prédio privado e lucrativo.
            let prov = v.provincias.iter().find(|p| p.codigo_externo.is_some() && opcoes_para_provincia(v, p).iter().any(|o| o.comporta && !o.governamental)).expect("província com opções");
            let adm1 = prov.codigo_externo.clone().unwrap();
            let ops = opcoes_para_provincia(v, prov);
            let possiveis: Vec<_> = ops.iter().filter(|o| o.comporta).collect();
            assert!(!possiveis.is_empty(), "o painel deve listar prédios possíveis");
            assert!(possiveis.iter().all(|o| o.pontos > 0.0 && o.custo_bens >= 0.0 && o.lucro_dia.is_finite()));
            assert!(provincia_por_adm1(v, &adm1).is_some());
            let o = possiveis[0];
            (adm1, o.tipo_id.clone(), prov.id.clone(), v.pais.as_ref().unwrap().fila_construcao.len())
        };
        // A interface (egui) renderiza a seção sem pânico e com os dados.
        {
            let ctx = egui::Context::default();
            let mut c = ConstrucaoUi::default();
            let v = app.world().resource::<EconomiaSim>().vista.clone();
            let mut saida = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    secao_construir_aqui(ui, &v, &adm1, &mut c);
                    corpo_tela_construcao(ui, &v, &mut c);
                });
            });
            saida.textures_delta.clear();
        }
        // Clique em "Construir": registra o pedido.
        app.world_mut().resource_mut::<ConstrucaoUi>().pedidos.push((prov_id.clone(), tipo_id.clone()));
        let data = DataSimulacao::default();
        app.world_mut().write_message(EventoTickDiario { nova_data: data, total_ticks: 0 });
        let t = Instant::now();
        loop {
            app.update();
            let e = app.world().resource::<EconomiaSim>();
            let fila = e.vista.pais.as_ref().map_or(0, |p| p.fila_construcao.len());
            if !e.ocupada() && fila > n_fila {
                let p = e.vista.pais.as_ref().unwrap();
                assert!(p.fila_construcao.iter().any(|i| i.provincia_id == prov_id && i.tipo_predio_id == tipo_id));
                break;
            }
            assert!(t.elapsed().as_secs() < 60, "obra não entrou na fila: {:?}", e.mensagens);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn ranking_ordenado_e_modos_de_mapa_tem_dados() {
        let app = app_headless();
        let v = &app.world().resource::<EconomiaSim>().vista;
        let cat = v.catalogo.as_ref().unwrap();
        let idx = cat.tipos_predio.iter().position(|t| t.id == "plantacao_cafe_cha").unwrap_or(0);
        let r = ranking_estados(v, idx);
        assert!(!r.is_empty());
        assert!(r.windows(2).all(|w| w[0].vantagem >= w[1].vantagem), "ranking deve estar ordenado");
        for modo in [ModoMapa::Potencial, ModoMapa::Infraestrutura] {
            let cores = cores_por_adm1(v, modo, idx);
            assert!(!cores.is_empty(), "modo {modo:?} sem dados");
        }
    }

    #[test]
    fn tutorial_tem_de_3_a_5_passos() {
        assert!((3..=5).contains(&PASSOS_TUTORIAL.len()));
    }

    /// Diagnóstico e validação regional: lucro previsto do mesmo prédio em várias províncias dos EUA.
    #[test]
    fn diag_lucro_por_provincia() {
        let app = app_headless();
        let v = &app.world().resource::<EconomiaSim>().vista;
        println!("DIAG pais={} provincias na vista: {} estados: {}", v.pais_id, v.provincias.len(), v.estados.len());
        let provs: Vec<&Provincia> = v.provincias.iter().step_by((v.provincias.len() / 10).max(1)).take(10).collect();

        let buscar_tipo = |opcoes: &[OpcaoConstrucao], tipo: &str| -> Option<f64> {
            match tipo {
                "fazenda_trigo" => opcoes.iter().find(|o| o.tipo_id == "fazenda_graos").map(|o| o.lucro_dia),
                "fabrica_moveis" => opcoes.iter().find(|o| o.tipo_id == "fabrica_moveis").map(|o| o.lucro_dia),
                "siderurgica" => opcoes.iter().find(|o| o.tipo_id == "siderurgica").map(|o| o.lucro_dia),
                "mina_carvao" => {
                    opcoes
                        .iter()
                        .find(|o| o.tipo_id == "mina_carvao")
                        .or_else(|| opcoes.iter().find(|o| o.tipo_id.starts_with("mina_")))
                        .map(|o| o.lucro_dia)
                }
                outro => opcoes.iter().find(|o| o.tipo_id.contains(outro.split('_').next().unwrap())).map(|o| o.lucro_dia),
            }
        };

        for tipo in ["fabrica_moveis", "siderurgica", "mina_carvao", "fazenda_trigo"] {
            let linha: Vec<String> = provs.iter().map(|pr| {
                let ops = opcoes_para_provincia(v, pr);
                buscar_tipo(&ops, tipo).map_or("-".into(), |val| format!("{:.1}", val))
            }).collect();
            println!("DIAG {tipo}: {}", linha.join(" | "));
        }

        let prov_samoa = provs.iter().find(|p| p.estado_id == "usa_american_samoa").copied().expect("samoa");
        let prov_ohio = provs.iter().find(|p| p.estado_id == "usa_ohio").copied().expect("ohio");
        let prov_iowa = provs.iter().find(|p| p.estado_id == "usa_iowa").copied().expect("iowa");

        let ops_samoa = opcoes_para_provincia(v, prov_samoa);
        let ops_ohio = opcoes_para_provincia(v, prov_ohio);
        let ops_iowa = opcoes_para_provincia(v, prov_iowa);

        let lucro_moveis_samoa = buscar_tipo(&ops_samoa, "fabrica_moveis").expect("lucro moveis samoa");
        let lucro_moveis_ohio = buscar_tipo(&ops_ohio, "fabrica_moveis").expect("lucro moveis ohio");
        let lucro_siderurgica_ohio = buscar_tipo(&ops_ohio, "siderurgica").expect("lucro siderurgica ohio");
        let lucro_trigo_iowa = buscar_tipo(&ops_iowa, "fazenda_trigo").expect("lucro trigo iowa");

        // 1. Fazenda de trigo em Iowa > 0
        assert!(lucro_trigo_iowa > 0.0, "fazenda de trigo em Iowa deve ser > 0, deu {lucro_trigo_iowa:.1}");

        // 2. Siderúrgica em Ohio > 0
        assert!(lucro_siderurgica_ohio > 0.0, "siderurgica em Ohio deve ser > 0, deu {lucro_siderurgica_ohio:.1}");

        // 3. Lucro previsto da mesma fábrica deve diferir pelo menos 20% entre Samoa Americana e Ohio
        let dif_rel = (lucro_moveis_ohio - lucro_moveis_samoa).abs() / lucro_moveis_ohio.abs().max(lucro_moveis_samoa.abs()).max(1.0);
        assert!(
            dif_rel >= 0.20,
            "lucro previsto de fábrica deve diferir pelo menos 20% entre Samoa ({lucro_moveis_samoa:.1}) e Ohio ({lucro_moveis_ohio:.1}), dif={:.1}%",
            dif_rel * 100.0
        );

        for pr in &provs {
            let est = estado_de(v, pr);
            let desemp: f64 = pr.pops.iter().map(|p| p.desempregados()).sum();
            println!(
                "DIAG prov {} estado={} pop={:.0} desemp={:.0} urb={:?} infra={:?}/{:?} pen_infra={:?} acesso={:?} terra={:?}/{:?}",
                pr.nome,
                pr.estado_id,
                pr.populacao_total(),
                desemp,
                est.map(|e| e.urbanizacao),
                est.map(|e| e.infraestrutura_uso),
                est.map(|e| e.infraestrutura_capacidade),
                est.map(|e| e.penalidade_infraestrutura),
                est.map(|e| e.acesso_mercado),
                est.map(|e| e.terra_aravel_livre()),
                est.map(|e| e.terra_aravel),
            );
        }
    }

    /// Validação da regra do usuário: em Cuba o jogador tem dezenas de opções construíveis.
    #[test]
    fn cuba_tem_dezenas_de_opcoes_construiveis() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let nucleo = crate::plugins::economia_sim::Nucleo::carregar(&raiz.join("assets/data/economia"), 0).expect("nucleo");
        let v = nucleo.vista("cub");
        assert!(!v.provincias.is_empty(), "Cuba deve ter províncias");
        for prov in &v.provincias {
            let opcoes = opcoes_para_provincia(&v, prov);
            let construiveis = opcoes.iter().filter(|o| o.comporta).count();
            println!("Cuba prov {}: {} construíveis de {} opções totais", prov.nome, construiveis, opcoes.len());
            assert!(
                construiveis >= 20,
                "em Cuba o jogador deve ter dezenas de opções construíveis (deu {construiveis})"
            );
        }
    }

    #[test]
    fn fechar_reabrir_demolir_ui_e_consequencias() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut nucleo = crate::plugins::economia_sim::Nucleo::carregar(&raiz.join("assets/data/economia"), 5).expect("nucleo");
        let usa = nucleo.id_pais("USA").unwrap();
        let (prov_id, tipo_id, nivel_ini) = nucleo
            .mundo
            .provincias()
            .iter()
            .filter(|p| p.pais_id == usa)
            .find_map(|p| {
                p.predios
                    .iter()
                    .find(|b| b.nivel >= 2 && !b.fechado && b.total_empregados() >= 500.0)
                    .map(|b| (p.id.clone(), b.tipo_predio_id.clone(), b.nivel))
            })
            .expect("prédio grande nos EUA");

        // 1. Fechar via Comando::FecharPredio
        let msg_fechar = nucleo
            .aplicar(&usa, Comando::FecharPredio { provincia_id: prov_id.clone(), tipo_id: tipo_id.clone() })
            .expect("fechar");
        assert!(msg_fechar.contains("fechado"));
        assert!(!nucleo.acoes_predio_recentes.is_empty());
        assert!(nucleo.acoes_predio_recentes[0].trabalhadores_demitidos >= 500.0);

        let v_fechado = nucleo.vista(&usa);
        let b_fechado = v_fechado
            .provincias
            .iter()
            .find(|p| p.id == prov_id)
            .unwrap()
            .predios
            .iter()
            .find(|b| b.tipo_predio_id == tipo_id)
            .unwrap();
        assert!(b_fechado.fechado);
        let (rotulo_tend, _, custo_reabrir, reembolso_demolir) = resumo_gestao_predio(&v_fechado, b_fechado);
        assert!(!rotulo_tend.is_empty() && custo_reabrir > 0.0 && reembolso_demolir > 0.0);

        // 2. Reabrir via Comando::ReabrirPredio
        let msg_reabrir = nucleo
            .aplicar(&usa, Comando::ReabrirPredio { provincia_id: prov_id.clone(), tipo_id: tipo_id.clone() })
            .expect("reabrir");
        assert!(msg_reabrir.contains("reaberto"));

        // 3. Demolir 1 nível via Comando::DemolirPredio
        let msg_demolir = nucleo
            .aplicar(&usa, Comando::DemolirPredio { provincia_id: prov_id.clone(), tipo_id: tipo_id.clone(), niveis: 1 })
            .expect("demolir");
        assert!(msg_demolir.contains("demolido"));
        let b_demolido = nucleo
            .mundo
            .obter_provincia(&prov_id)
            .unwrap()
            .predios
            .iter()
            .find(|b| b.tipo_predio_id == tipo_id)
            .unwrap();
        assert_eq!(b_demolido.nivel, nivel_ini - 1);
    }
}
