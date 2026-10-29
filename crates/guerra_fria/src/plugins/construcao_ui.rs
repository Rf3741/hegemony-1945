//! Construção clara e regional: seção "Construir aqui" no painel da província, tela de construção (tecla B),
//! modos de mapa "Potencial" e "Infraestrutura" e tutorial curto em pt-BR.
//!
//! Toda a lógica de decisão (quais prédios cabem, ranking de estados, cores do mapa) fica em funções puras
//! sobre `VistaEconomia`, para ser testada sem janela. A interface só desenha e registra *pedidos*
//! (`ConstrucaoUi::pedidos`), que o sistema `aplicar_pedidos` converte em `Comando::Construir`.
use super::economia_sim::{Comando, EconomiaSim, VistaEconomia};
use super::jogador::{calcular_custo_materiais_obra, calcular_lucratividade_esperada, fmt_din};
use super::map::{CoresMapa, DadosMapa, ImagemMapa, ProvinciaSelecionada, RecursoCatalogo, colorir};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use sim_economia::{CategoriaPredio, Estado, Provincia, TipoPredio};
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
            tutorial: if tutorial_ja_visto() { None } else { Some(0) },
            pedidos: Vec::new(),
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

fn niveis_do_tipo_no_estado(v: &VistaEconomia, estado_id: &str, tipo_id: &str) -> u32 {
    v.provincias
        .iter()
        .filter(|p| p.estado_id == estado_id)
        .flat_map(|p| p.predios.iter())
        .filter(|b| b.tipo_predio_id == tipo_id)
        .map(|b| b.nivel)
        .sum()
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
        Some(0) if t.usa_terra_aravel && !est.cultura_permitida(&t.id) => "Cultura não permitida neste estado".into(),
        Some(0) if t.usa_terra_aravel => "Sem terra arável livre".into(),
        Some(0) => format!("Sem jazida livre de {}", t.recurso_natural_requerido.as_deref().unwrap_or("recurso").replace('_', " ")),
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
            let comporta = v.comporta(prov, i);
            let mat = calcular_custo_materiais_obra(cat, t, &v.bens);
            let luc = calcular_lucratividade_esperada(cat, t, &t.metodos_padrao, &v.bens, 4.0);
            let q = est.map_or(1.0, |e| e.fator_qualidade_predio(t));
            let (potencial, motivo) = match est {
                Some(e) => {
                    let p = texto_potencial(v, e, t);
                    let m = if comporta { String::new() } else { p.clone() };
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
                motivo: if comporta || !motivo.is_empty() { motivo } else { "Capacidade esgotada".into() },
                pontos: t.custo_pontos_construcao,
                dias: t.custo_pontos_construcao / pontos_dia,
                custo_bens: mat.total_reais,
                itens: mat.itens.iter().map(|(n, q, _, _)| (n.clone(), *q)).collect(),
                lucro_dia: if t.financiado_pelo_governo { -(luc.custo_insumos + luc.custo_folha) } else { luc.lucro_liquido * q },
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
    let luc = calcular_lucratividade_esperada(cat, t, &t.metodos_padrao, &v.bens, 4.0);
    let mut linhas: Vec<LinhaEstado> = v
        .estados
        .iter()
        .map(|e| {
            let livre = potencial_livre(v, e, t);
            let q = e.fator_qualidade_predio(t);
            let alvo = v
                .provincias
                .iter()
                .filter(|p| p.estado_id == e.id)
                .find(|p| v.comporta(p, tipo_idx))
                .map(|p| p.id.clone());
            let lucro = if t.financiado_pelo_governo { -(luc.custo_insumos + luc.custo_folha) } else { luc.lucro_liquido * q };
            let pot = livre.map_or(1.0, |n| (1.0 + n as f64).ln());
            let folga = (1.0 - e.penalidade_infraestrutura).clamp(0.05, 1.0);
            let vantagem = if alvo.is_some() { q * folga * pot * if lucro > 0.0 { 1.0 + lucro.max(0.0).ln_1p() } else { 0.5 } } else { 0.0 };
            LinhaEstado {
                estado_id: e.id.clone(),
                nome: e.nome.clone(),
                livre,
                qualidade: q,
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
    ("2/5  Recursos regionais", "Cada estado tem recursos próprios: terra arável, culturas permitidas e jazidas. Café só onde há cultura de café; ferro e carvão só onde há jazida. A lista só mostra o que cabe na província."),
    ("3/5  Custo e lucro", "Cada obra gasta bens (aço, madeira, ferramentas...) e pontos de construção. O lucro esperado por dia ajuda a escolher. Prédios do governo dão serviço, não lucro."),
    ("4/5  Infraestrutura", "Prédios usam infraestrutura do estado (ferrovias, portos, urbanização). Passando da capacidade, a produção cai. Veja o modo de mapa \"Infraestrutura\" na tela de construção."),
    ("5/5  Setor privado e fila", "Capitalistas também investem sozinhos, conforme a lei econômica, usando o fundo privado. A fila de obras (na tela B) mostra o progresso de cada obra, estatal ou privada."),
];

fn arquivo_tutorial() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    Some(base.join("hegemony1945").join("tutorial_construcao_visto"))
}

fn tutorial_ja_visto() -> bool {
    // Testes nunca dependem/gravam o estado do usuário.
    if cfg!(test) {
        return true;
    }
    arquivo_tutorial().is_some_and(|p| p.exists())
}

fn marcar_tutorial_visto() {
    if cfg!(test) {
        return;
    }
    if let Some(p) = arquivo_tutorial() {
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, "1");
    }
}

// ---------------------------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------------------------

fn cor_lucro(v: f64) -> egui::Color32 {
    if v >= 0.0 { egui::Color32::LIGHT_GREEN } else { egui::Color32::LIGHT_RED }
}

/// Seção "Construir aqui" do painel de província/estado. `adm1` é o código da província clicada no mapa.
pub fn secao_construir_aqui(ui: &mut egui::Ui, v: &VistaEconomia, adm1: &str, c: &mut ConstrucaoUi) {
    ui.separator();
    ui.heading("Construir aqui");
    let Some(cat) = v.catalogo.as_ref() else {
        ui.label("A economia ainda está calculando...");
        return;
    };
    let Some(prov) = provincia_por_adm1(v, adm1) else {
        ui.label("Esta província não é do seu país: só é possível construir no seu território.");
        return;
    };
    if let Some(e) = estado_de(v, prov) {
        ui.label(format!("Estado: {} | Província: {}", e.nome, prov.nome));
        ui.label(format!(
            "Terra arável livre: {} | Infraestrutura: {:.0}/{:.0}{}",
            e.terra_aravel_livre(),
            e.infraestrutura_uso,
            e.infraestrutura_capacidade,
            if e.penalidade_infraestrutura > 0.0 { format!(" (penalidade -{:.0}%)", e.penalidade_infraestrutura * 100.0) } else { String::new() }
        ));
    }
    ui.horizontal_wrapped(|ui| {
        if ui.selectable_label(c.filtro_cat.is_none(), "Todas").clicked() {
            c.filtro_cat = None;
        }
        for cp in CATEGORIAS {
            if ui.selectable_label(c.filtro_cat == Some(cp), nome_categoria(cp)).clicked() {
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
            egui::Frame::new().stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(70))).inner_margin(6.0).corner_radius(4.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(&o.nome);
                    ui.weak(nome_categoria(o.categoria));
                    if o.na_fila > 0 {
                        ui.colored_label(egui::Color32::YELLOW, format!("({} na fila)", o.na_fila));
                    }
                });
                ui.label(format!("Potencial: {}", o.potencial));
                ui.label(format!("Custo: {} em bens + {:.0} pts de construção (~{:.0} dias)", fmt_din(o.custo_bens), o.pontos, o.dias));
                if !o.itens.is_empty() {
                    let bens: Vec<String> = o.itens.iter().map(|(n, q)| format!("{q:.0}x {n}")).collect();
                    ui.weak(format!("Bens: {}", bens.join(", ")));
                }
                ui.horizontal(|ui| {
                    if o.governamental {
                        ui.label(format!("Serviço público, custo ao tesouro: {}/dia", fmt_din(-o.lucro_dia)));
                    } else {
                        ui.label("Lucro esperado:");
                        ui.colored_label(cor_lucro(o.lucro_dia), format!("{}/dia por nível", fmt_din(o.lucro_dia)));
                    }
                    let b = ui.add_enabled(o.comporta, egui::Button::new("Construir"));
                    if b.on_disabled_hover_text(&o.motivo).clicked() {
                        c.pedidos.push((prov.id.clone(), o.tipo_id.clone()));
                    }
                });
                if !o.comporta {
                    ui.colored_label(egui::Color32::LIGHT_RED, format!("Não cabe: {}", o.motivo));
                }
            });
        }
        if mostradas == 0 {
            ui.label("Nenhum prédio dessa categoria cabe aqui. Veja o potencial regional na tela de construção (B).");
        }
    });
    let _ = cat;
}

fn tela_construcao(mut contextos: EguiContexts, mut c: ResMut<ConstrucaoUi>, eco: Option<Res<EconomiaSim>>) -> Result {
    if !c.aberta {
        return Ok(());
    }
    let Some(eco) = eco else { return Ok(()) };
    let ctx = contextos.ctx_mut()?;
    let mut aberta = true;
    egui::Window::new("Construção (B)").open(&mut aberta).default_size([860.0, 640.0]).show(ctx, |ui| {
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
        ui.label(format!("Capacidade de construção: {:.0} pts/dia", v.pontos_construcao_dia));
        ui.separator();
        ui.label("Modo do mapa:");
        ui.selectable_value(&mut c.modo_mapa, ModoMapa::Normal, "Normal");
        ui.selectable_value(&mut c.modo_mapa, ModoMapa::Potencial, "Potencial");
        ui.selectable_value(&mut c.modo_mapa, ModoMapa::Infraestrutura, "Infraestrutura");
        ui.separator();
        if ui.button("Ver tutorial").clicked() {
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
    ui.separator();
    ui.columns(2, |cols| {
        // Coluna esquerda: categoria e tipo
        let ui = &mut cols[0];
        ui.strong("1. Escolha o prédio");
        ui.horizontal_wrapped(|ui| {
            if ui.selectable_label(c.filtro_cat.is_none(), "Todas").clicked() {
                c.filtro_cat = None;
            }
            for cp in CATEGORIAS {
                if ui.selectable_label(c.filtro_cat == Some(cp), nome_categoria(cp)).clicked() {
                    c.filtro_cat = Some(cp);
                }
            }
        });
        egui::ScrollArea::vertical().id_salt("scroll_tipos").max_height(230.0).show(ui, |ui| {
            for (i, t) in cat.tipos_predio.iter().enumerate() {
                if c.filtro_cat.is_some_and(|f| f != t.categoria) {
                    continue;
                }
                if ui.selectable_label(c.tipo_idx == i, &t.nome).clicked() {
                    c.tipo_idx = i;
                }
            }
        });
        let t = &cat.tipos_predio[c.tipo_idx];
        let mat = calcular_custo_materiais_obra(cat, t, &v.bens);
        let luc = calcular_lucratividade_esperada(cat, t, &t.metodos_padrao, &v.bens, 4.0);
        ui.separator();
        ui.strong(&t.nome);
        ui.label(format!("Custo: {} em bens + {:.0} pts de construção (~{:.0} dias)", fmt_din(mat.total_reais), t.custo_pontos_construcao, t.custo_pontos_construcao / v.pontos_construcao_dia.clamp(1.0, 25.0)));
        if t.financiado_pelo_governo {
            ui.label(format!("Serviço público: custa {}/dia ao tesouro", fmt_din(luc.custo_insumos + luc.custo_folha)));
        } else {
            ui.horizontal(|ui| {
                ui.label("Lucro esperado (por nível, base):");
                ui.colored_label(cor_lucro(luc.lucro_liquido), format!("{}/dia", fmt_din(luc.lucro_liquido)));
            });
        }

        // Coluna direita: ranking de estados
        let ui = &mut cols[1];
        ui.strong("2. Estados com mais vantagem");
        let ranking = ranking_estados(v, c.tipo_idx);
        egui::ScrollArea::vertical().id_salt("scroll_estados").max_height(300.0).show(ui, |ui| {
            egui::Grid::new("grid_ranking_estados").striped(true).show(ui, |ui| {
                ui.strong("Estado");
                ui.strong("Potencial");
                ui.strong("Infra");
                ui.strong("Lucro/dia");
                ui.strong("");
                ui.end_row();
                for l in ranking.iter().take(60) {
                    let sel = c.estado_id == l.estado_id;
                    if ui.selectable_label(sel, &l.nome).clicked() {
                        c.estado_id = l.estado_id.clone();
                    }
                    match l.livre {
                        Some(n) => ui.label(format!("{n} livres (x{:.2})", l.qualidade)),
                        None => ui.label("urbano"),
                    };
                    let pct = if l.infra_cap > 1e-6 { l.infra_uso / l.infra_cap * 100.0 } else { 0.0 };
                    let cor = if l.penalidade > 0.0 { egui::Color32::LIGHT_RED } else { egui::Color32::LIGHT_GREEN };
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
    ui.separator();
    ui.strong(format!("3. Fila de obras ({})", pais.fila_construcao.len()));
    egui::ScrollArea::vertical().id_salt("scroll_fila").max_height(180.0).show(ui, |ui| {
        if pais.fila_construcao.is_empty() {
            ui.label("Nenhuma obra em andamento.");
        }
        for it in &pais.fila_construcao {
            let nome_t = cat.tipos_predio.iter().find(|t| t.id == it.tipo_predio_id).map_or(it.tipo_predio_id.as_str(), |t| t.nome.as_str());
            let nome_p = v.provincias.iter().find(|p| p.id == it.provincia_id).map_or(it.provincia_id.as_str(), |p| p.nome.as_str());
            let frac = (it.progresso_pontos / it.custo_total_pontos.max(1.0)).clamp(0.0, 1.0) as f32;
            ui.horizontal(|ui| {
                ui.label(format!("{nome_t} em {nome_p} [{}]", if it.privado { "privada" } else { "estatal" }));
                ui.add(egui::ProgressBar::new(frac).desired_width(160.0).text(format!("{:.0}/{:.0} pts", it.progresso_pontos, it.custo_total_pontos)));
            });
        }
    });
}

fn janela_tutorial(mut contextos: EguiContexts, mut c: ResMut<ConstrucaoUi>) -> Result {
    let Some(passo) = c.tutorial else { return Ok(()) };
    let ctx = contextos.ctx_mut()?;
    let (titulo, texto) = PASSOS_TUTORIAL[passo.min(PASSOS_TUTORIAL.len() - 1)];
    let mut novo = Some(passo);
    egui::Window::new("Tutorial: construção").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).default_width(440.0).show(ctx, |ui| {
        ui.heading(titulo);
        ui.label(texto);
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(passo > 0, egui::Button::new("Voltar")).clicked() {
                novo = Some(passo - 1);
            }
            if passo + 1 < PASSOS_TUTORIAL.len() {
                if ui.button("Próximo").clicked() {
                    novo = Some(passo + 1);
                }
            } else if ui.button("Entendi").clicked() {
                novo = None;
            }
            if ui.button("Pular").clicked() {
                novo = None;
            }
        });
    });
    if novo.is_none() {
        marcar_tutorial_visto();
    }
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
    if c.pedidos.is_empty() {
        return;
    }
    for (provincia_id, tipo_id) in std::mem::take(&mut c.pedidos) {
        eco.enviar(Comando::Construir { provincia_id, tipo_id });
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
) {
    let (Some(cores), Some(dados), Some(imagem)) = (cores, dados, imagem) else { return };
    let chave = (c.modo_mapa, c.tipo_idx, eco.versao_construcao, eco.vista.pais_id.clone());
    if ultimo.as_ref() == Some(&chave) || eco.vista.catalogo.is_none() {
        return;
    }
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
    let (pixels, _) = colorir(&dados.ids, dados.largura, dados.altura, &cor_prov, &cores.dono_prov);
    if let Some(mut img) = imagens.get_mut(&imagem.0) {
        img.data = Some(pixels);
    }
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
}
