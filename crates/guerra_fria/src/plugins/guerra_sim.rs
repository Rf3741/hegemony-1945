//! Plugin de simulação militar e integração do `sim_guerra` ao cliente gráfico.
//!
//! Gerencia:
//! - Criação e sincronização do `MundoGuerra` a partir das províncias e países do jogo.
//! - Exércitos por país dimensionados por PIB e mobilização.
//! - Atribuição de generais históricos (`generais.ron`).
//! - Ticks diários de combate, suprimento e frentes automáticas na fronteira.
//! - Ocupação província por província sincronizada com `MundoJogo` e hachura no mapa.
//! - Declaração de guerra e acordos de paz (anexação e reparações).

use super::economia_sim::EconomiaSim;
use super::economy::RecursoMundo;
use super::guerra_dados::CatalogoDesignGuerra;
use super::jogador::PaisJogador;
use super::map::{CoresMapa, DadosMapa, ImagemMapa, RaizProjeto, RecursoCatalogo, colorir_com_hachura};
use super::time::EventoTickDiario;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use sim_guerra::{
    ArestaRegiao, CatalogoGuerra, ClausulaPaz, Comandante, ConfigGuerra,
    Divisao, Exercito, Guerra, MundoGuerra, PaisMilitar, PosturaOperacional,
    PropostaPaz, RegiaoMapa, TratadoPazAssinado,
};
use std::collections::HashMap;

/// Recurso ECS que encapsula o motor determinístico `sim_guerra`.
#[derive(Resource)]
pub struct GuerraSim(pub MundoGuerra);

/// Recurso ECS com os dados estáticos de design (CBs, generais, guerras históricas).
#[derive(Resource)]
pub struct DadosGuerra(pub CatalogoDesignGuerra);

/// Estado do painel e modo visual da guerra.
#[derive(Resource, Default)]
pub struct PainelGuerra {
    pub aberto: bool,
    pub guerra_selecionada: Option<String>,
    pub aba: u8, // 0 = Guerras, 1 = Frentes, 2 = Exércitos, 3 = Tratados
    pub proposta_anexar: bool,
    pub proposta_reparacoes: bool,
    pub casus_belli_idx: usize,
    pub mensagem: String,
    pub repintar: bool,
}

pub struct GuerraSimPlugin;

impl Plugin for GuerraSimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PainelGuerra>()
            .add_systems(
                Update,
                (
                    inicializar_guerra_sim,
                    tick_guerra,
                    tecla_atalho_guerra,
                    repintar_mapa_guerra,
                )
                    .chain()
                    .run_if(resource_exists::<PaisJogador>)
                    .run_if(crate::estado::em_partida),
            );
    }
}

/// Tecla 6 para alternar o painel militar.
fn tecla_atalho_guerra(teclas: Res<ButtonInput<KeyCode>>, mut painel: ResMut<PainelGuerra>) {
    if teclas.just_pressed(KeyCode::Digit6) || teclas.just_pressed(KeyCode::Numpad6) {
        painel.aberto = !painel.aberto;
    }
}

/// Normaliza tags para 3 caracteres ASCII maiúsculos compatíveis com `sim_guerra`.
pub fn normalizar_tag(tag: &str) -> String {
    let mut t = tag.trim().to_uppercase();
    if t.len() < 3 {
        while t.len() < 3 {
            t.push('X');
        }
    } else if t.len() > 3 {
        t.truncate(3);
    }
    t
}

/// Inicializa o motor militar `sim_guerra` a partir do mapa, economia e dados de 1945.
fn inicializar_guerra_sim(
    mut commands: Commands,
    raiz: Res<RaizProjeto>,
    catalogo_mapa: Res<RecursoCatalogo>,
    mundo: Res<RecursoMundo>,
    existente: Option<Res<GuerraSim>>,
) {
    if existente.is_some() {
        return;
    }

    match construir_guerra_1945(&raiz.0, &catalogo_mapa.0, &mundo.0) {
        Ok((mundo_guerra, catalogo_design)) => {
            commands.insert_resource(GuerraSim(mundo_guerra));
            commands.insert_resource(DadosGuerra(catalogo_design));
            info!("Mundo de guerra inicializado com sucesso.");
        }
        Err(e) => {
            error!("Falha crítica ao instanciar MundoGuerra: {}", e);
        }
    }
}

/// Constrói o `MundoGuerra` a partir do mapa, da economia e dos dados de 1945 (sem Bevy).
pub fn construir_guerra_1945(
    raiz: &std::path::Path,
    catalogo_mapa: &gf_mapa::mapa::CatalogoProvincias,
    mundo: &simulacao::mundo::MundoJogo,
) -> Result<(MundoGuerra, CatalogoDesignGuerra), String> {
    let pasta_guerra = raiz.join("data/guerra");
    let catalogo_militar = match CatalogoGuerra::carregar_diretorio(&pasta_guerra) {
        Ok(c) => c,
        Err(e) => {
            warn!("Aviso ao carregar catálogo militar de {:?}: {}", pasta_guerra, e);
            CatalogoGuerra::default()
        }
    };

    let catalogo_design = CatalogoDesignGuerra::carregar_de_diretorio(&pasta_guerra).unwrap_or_default();

    // 1. Constrói regiões militares correspondentes às províncias
    let mut regioes = Vec::with_capacity(catalogo_mapa.provincias.len());
    let mut capitais_por_pais: HashMap<String, String> = HashMap::new();

    for prov in &catalogo_mapa.provincias {
        let adm1_code = format!("prov_{}", prov.id);
        let dono = mundo.dono_por_provincia.get(&prov.id).map(|t| normalizar_tag(t)).unwrap_or_else(|| "NEU".to_string());
        let controlador = mundo.controlador_por_provincia.get(&prov.id).map(|t| normalizar_tag(t)).unwrap_or_else(|| dono.clone());

        capitais_por_pais.entry(dono.clone()).or_insert_with(|| adm1_code.clone());

        let pop = mundo.populacao_por_provincia.get(&prov.id).copied().unwrap_or(15_000);
        let eh_capital = capitais_por_pais.get(&dono).map_or(false, |c| c == &adm1_code);
        let pv = if eh_capital { 10.0 } else { 1.0 };
        let porto = if prov.vizinhos.iter().any(|&v| v == 0) { 15 } else { 0 };

        regioes.push(RegiaoMapa {
            adm1_code,
            nome_pt: prov.nome.clone(),
            dono_tag: dono,
            controlador_tag: controlador,
            terreno_id: "planicie".to_string(),
            infraestrutura: 35,
            nivel_porto: porto,
            nivel_fortificacao: 0,
            pontos_vitoria: pv,
            populacao: pop,
            destruicao_guerra: 0.0,
            resistencia_ocupacao: 0.0,
        });
    }

    // 2. Constrói arestas territoriais entre províncias vizinhas
    let mut arestas = Vec::new();
    let max_id = catalogo_mapa.provincias.len() as u32;
    for prov in &catalogo_mapa.provincias {
        let de = format!("prov_{}", prov.id);
        for &viz in &prov.vizinhos {
            if viz > prov.id && viz <= max_id {
                let para = format!("prov_{}", viz);
                arestas.push(ArestaRegiao {
                    de: de.clone(),
                    para,
                    distancia_km: 60.0,
                    travessia_rio: false,
                    estreito_maritimo: false,
                });
            }
        }
    }

    // 3. Países militares com capacidade baseada em PIB e mobilização
    let mut paises_militares = Vec::new();
    let mut tags_conhecidas = std::collections::HashSet::new();

    for (tag_orig, pais) in &mundo.paises {
        let tag = normalizar_tag(tag_orig);
        if tags_conhecidas.contains(&tag) {
            continue;
        }
        tags_conhecidas.insert(tag.clone());

        let capital = capitais_por_pais.get(&tag).cloned().unwrap_or_else(|| {
            regioes.iter().find(|r| r.dono_tag == tag).map(|r| r.adm1_code.clone()).unwrap_or_else(|| "prov_1".to_string())
        });

        let pop_total = pais.populacao_total();
        let pop_mob = (pop_total as f64 * 0.025) as u64;

        paises_militares.push(PaisMilitar {
            tag,
            nome_pt: pais.nome.clone(),
            capital_adm1: capital,
            doutrina_id: "doutrina_padrao".to_string(),
            tecnologias_ativas: Vec::new(),
            cansaco_guerra: 0.0,
            baixas_totais: 0,
            populacao_mobilizavel: pop_mob,
            estoque_suprimento: 1.0,
            eficiencia_naval: 1.0,
        });
    }

    // 4. Comandantes históricos catalogados
    let mut comandantes = Vec::new();
    for g in &catalogo_design.generais {
        let tag = normalizar_tag(&g.pais_tag);
        if tags_conhecidas.contains(&tag) {
            comandantes.push(Comandante {
                id: g.id.clone(),
                nome_pt: g.nome.clone(),
                pais_tag: tag,
                habilidade_ataque: g.ataque.clamp(1, 10),
                habilidade_defesa: g.defesa.clamp(1, 10),
                habilidade_logistica: g.logistica.clamp(1, 10),
                habilidade_planejamento: g.planejamento.clamp(1, 10),
                experiencia: 0.50,
                tracos: g.tracos.clone(),
            });
        }
    }

    // 5. Exércitos e divisões por país
    let mut divisoes = Vec::new();
    let mut exercitos = Vec::new();

    for pais in &paises_militares {
        let tag = &pais.tag;
        let provs_pais: Vec<String> = regioes.iter().filter(|r| &r.dono_tag == tag).map(|r| r.adm1_code.clone()).collect();
        if provs_pais.is_empty() {
            continue;
        }

        let pib = mundo.paises.get(tag).map_or(5_000_000_000.0, |p| p.pib_anual_estimado.max(1_000_000.0));
        // Tamanho do exército em divisões determinado pelo PIB
        let n_divs = ((pib / 4_000_000_000.0).clamp(2.0, 40.0)) as usize;

        let coms_pais: Vec<String> = comandantes.iter().filter(|c| &c.pais_tag == tag).map(|c| c.id.clone()).collect();

        let divs_por_exercito = 6.max(n_divs / 3);
        let n_exercitos = (n_divs + divs_por_exercito - 1) / divs_por_exercito;

        for e_num in 0..n_exercitos {
            let e_id = format!("exe_{}_{}", tag.to_lowercase(), e_num + 1);
            let com_id = coms_pais.get(e_num).cloned();

            let mut exe_divs = Vec::new();
            let inicio = e_num * divs_por_exercito;
            let fim = (inicio + divs_por_exercito).min(n_divs);

            for d_num in inicio..fim {
                let d_id = format!("div_{}_{}", tag.to_lowercase(), d_num + 1);
                let reg_id = provs_pais[d_num % provs_pais.len()].clone();
                let tmpl = if d_num % 4 == 0 { "divisao_blindada_1945" } else { "divisao_infantaria_1945" };

                divisoes.push(Divisao {
                    id: d_id.clone(),
                    nome_pt: format!("{}ª Divisão ({})", d_num + 1, tag),
                    pais_tag: tag.clone(),
                    template_id: tmpl.to_string(),
                    regiao_adm1: reg_id,
                    efetivos_atuais: 10_000,
                    equipamento_fracao: 1.0,
                    organizacao: 45.0,
                    moral: 0.85,
                    entrincheiramento: 2.0,
                    suprimento_atual: 1.0,
                    experiencia: 0.30,
                });
                exe_divs.push(d_id);
            }

            exercitos.push(Exercito {
                id: e_id,
                nome_pt: format!("{}º Exército de {}", e_num + 1, pais.nome_pt),
                pais_tag: tag.clone(),
                comandante_id: com_id,
                divisoes_ids: exe_divs,
                postura: PosturaOperacional::OfensivaEquilibrada,
                frente_id: None,
                regiao_objetivo_adm1: None,
            });
        }
    }

    let cenario = sim_guerra::CenarioGuerra {
        data_inicial: sim_guerra::DataJogo {
            ano: mundo.data.ano as u16,
            mes: mundo.data.mes as u8,
            dia: mundo.data.dia as u8,
        },
        seed: 19450815,
        config: ConfigGuerra {
            resolver_paz_automatica: true,
            limiar_capitulacao_vitoria: 0.80,
            limiar_cansaco_armisticio: 85.0,
            ..ConfigGuerra::default()
        },
        catalogo: catalogo_militar,
        regioes,
        arestas,
        paises: paises_militares,
        comandantes,
        divisoes,
        exercitos,
        guerras: Vec::new(),
    };

    MundoGuerra::novo(cenario)
        .map(|m| (m, catalogo_design))
        .map_err(|e| e.to_string())
}

/// Tick diário de simulação da guerra sincronizado com o relógio do jogo.
fn tick_guerra(
    mut eventos: MessageReader<EventoTickDiario>,
    mut guerra_sim: Option<ResMut<GuerraSim>>,
    mut mundo: ResMut<RecursoMundo>,
    mut cores: Option<ResMut<CoresMapa>>,
    mut painel: ResMut<PainelGuerra>,
    eco: Option<ResMut<EconomiaSim>>,
    mut ids_prov: Local<Vec<Option<u32>>>,
) {
    let Some(guerra) = guerra_sim.as_deref_mut() else { return };
    for _ in eventos.read() {
        let mudou = aplicar_tick_guerra(
            &mut guerra.0,
            &mut mundo.0,
            cores.as_deref_mut(),
            eco.is_some(),
            &mut ids_prov,
        );
        if mudou {
            painel.repintar = true;
        }
    }
}

/// Executa um dia de guerra e sincroniza o resultado com o mundo do jogo.
/// Devolve `true` se algum território mudou de controlador (precisa repintar o mapa).
///
/// `ids_prov` é um cache (índice da região -> id da província) para não reparsear
/// `"prov_N"` das ~12 mil regiões todo dia.
pub fn aplicar_tick_guerra(
    guerra: &mut MundoGuerra,
    mundo: &mut simulacao::mundo::MundoJogo,
    mut cores: Option<&mut CoresMapa>,
    com_economia: bool,
    ids_prov: &mut Vec<Option<u32>>,
) -> bool {
    let rel = guerra.executar_tick_diario();

    // 1. Consumo simples de bens militares e impacto no tesouro
    if com_economia {
        // Divisões ativas por país numa única passada (antes: países x divisões com String)
        let mut divs_ativas = vec![0u32; guerra.cenario.paises.len()];
        for d in &guerra.cenario.divisoes {
            if d.efetivos_atuais > 0
                && let Some(&p) = guerra.mapa_paises.get(&d.pais_tag)
            {
                divs_ativas[p] += 1;
            }
        }
        for (p_idx, pais) in guerra.cenario.paises.iter_mut().enumerate() {
            let n = divs_ativas[p_idx];
            if n > 0 {
                // Manutenção diária simples: 0.05M por divisão
                let custo_manutencao = n as f64 * 0.05;
                if let Some(p_jogo) = mundo.paises.get_mut(&pais.tag) {
                    p_jogo.orcamento.tesouro -= custo_manutencao;
                    if p_jogo.orcamento.tesouro < 0.0 {
                        pais.estoque_suprimento = (pais.estoque_suprimento - 0.005).max(0.40);
                    } else {
                        pais.estoque_suprimento = (pais.estoque_suprimento + 0.01).min(1.0);
                    }
                }
            }
        }
    }

    // 2. Sincroniza ocupação territorial das províncias
    let mut mudou_territorio = false;
    let regioes = &guerra.cenario.regioes;
    if ids_prov.len() != regioes.len() {
        ids_prov.clear();
        ids_prov.extend(regioes.iter().map(|r| {
            r.adm1_code.strip_prefix("prov_").and_then(|s| s.parse::<u32>().ok())
        }));
    }
    for (reg, id) in regioes.iter().zip(ids_prov.iter()) {
        let Some(id) = *id else { continue };
        // Sem clonar a String do controlador antigo (antes: 1 alocação por região por dia)
        let igual = match mundo.controlador_por_provincia.get(&id) {
            Some(c) => *c == reg.controlador_tag,
            None => reg.controlador_tag.is_empty(),
        };
        if !igual {
            mundo.controlador_por_provincia.insert(id, reg.controlador_tag.clone());
            mudou_territorio = true;

            if let Some(c) = cores.as_deref_mut() {
                let cor_ocup = if reg.esta_ocupada() {
                    mundo.paises.get(&reg.controlador_tag).map(|p| [p.cor.0, p.cor.1, p.cor.2, 255])
                } else {
                    None
                };
                if (id as usize) < c.ocupante_cor.len() {
                    c.ocupante_cor[id as usize] = cor_ocup;
                }
            }
        }
    }

    // 3. Sincroniza transferências soberanas de tratados de paz assinados
    if rel.tratados_assinados > 0 {
        mudou_territorio = true;
        for tratado in &guerra.historico_tratados {
            for clausula in &tratado.clausulas_executadas {
                if let ClausulaPaz::AnexarRegioes { vencedor_tag, regioes_adm1, .. } = clausula {
                    for reg_code in regioes_adm1 {
                        if let Some(id_str) = reg_code.strip_prefix("prov_")
                            && let Ok(id) = id_str.parse::<u32>()
                        {
                            mundo.dono_por_provincia.insert(id, vencedor_tag.clone());
                            mundo.controlador_por_provincia.insert(id, vencedor_tag.clone());
                            if let Some(c) = cores.as_deref_mut() {
                                if let Some(p) = mundo.paises.get(vencedor_tag)
                                    && (id as usize) < c.cor_prov.len()
                                {
                                    c.cor_prov[id as usize] = [p.cor.0, p.cor.1, p.cor.2, 255];
                                    c.ocupante_cor[id as usize] = None;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    mudou_territorio
}

/// Repinta a textura do mapa aplicando hachura diagonal sobre províncias ocupadas.
fn repintar_mapa_guerra(
    mut painel: ResMut<PainelGuerra>,
    cores: Option<Res<CoresMapa>>,
    dados: Option<Res<DadosMapa>>,
    imagem: Option<Res<ImagemMapa>>,
    mut imagens: ResMut<Assets<Image>>,
) {
    if !painel.repintar {
        return;
    }
    painel.repintar = false;

    let (Some(cores), Some(dados), Some(imagem)) = (cores, dados, imagem) else { return };

    let (pixels, _) = colorir_com_hachura(
        &dados.ids,
        dados.largura,
        dados.altura,
        &cores.cor_prov,
        &cores.dono_prov,
        &cores.estado_prov,
        Some(&cores.ocupante_cor),
    );

    let mut img = Image::new(
        Extent3d {
            width: dados.largura,
            height: dados.altura,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::nearest();
    let _ = imagens.insert(imagem.0.id(), img);
}

/// Helper para declarar guerra entre atacante e defensor com um casus belli.
pub fn acao_declarar_guerra(
    guerra_sim: &mut GuerraSim,
    atacante: &str,
    defensor: &str,
    nome_guerra: &str,
) -> Result<String, String> {
    let atk = normalizar_tag(atacante);
    let def = normalizar_tag(defensor);
    guerra_sim
        .0
        .declarar_guerra(&atk, &def, nome_guerra)
        .map_err(|e| format!("Erro ao declarar guerra: {e}"))
}

/// Helper para propor e executar tratado de paz simples.
pub fn acao_propor_paz(
    guerra_sim: &mut GuerraSim,
    guerra_id: &str,
    proponente: &str,
    anexar_ocupadas: bool,
    reparacoes: bool,
) -> Result<TratadoPazAssinado, String> {
    let prop_tag = normalizar_tag(proponente);
    let mut clausulas = Vec::new();

    if anexar_ocupadas {
        let ocupadas: Vec<String> = guerra_sim
            .0
            .regioes()
            .iter()
            .filter(|r| r.controlador_tag == prop_tag && r.dono_tag != prop_tag)
            .map(|r| r.adm1_code.clone())
            .collect();

        if !ocupadas.is_empty() {
            let perdedor_tag = guerra_sim
                .0
                .guerras()
                .iter()
                .find(|g| g.id == guerra_id)
                .map(|g| if g.eh_atacante(&prop_tag) { g.defensores[0].clone() } else { g.atacantes[0].clone() })
                .unwrap_or_else(|| "INM".to_string());

            clausulas.push(ClausulaPaz::AnexarRegioes {
                vencedor_tag: prop_tag.clone(),
                perdedor_tag,
                regioes_adm1: ocupadas,
            });
        }
    }

    if reparacoes {
        let pagador_tag = guerra_sim
            .0
            .guerras()
            .iter()
            .find(|g| g.id == guerra_id)
            .map(|g| if g.eh_atacante(&prop_tag) { g.defensores[0].clone() } else { g.atacantes[0].clone() })
            .unwrap_or_else(|| "INM".to_string());

        clausulas.push(ClausulaPaz::ReparacoesGuerra {
            pagador_tag,
            recebedor_tag: prop_tag.clone(),
            fracao_receita: 0.15,
            duracao_dias: 720,
        });
    }

    if clausulas.is_empty() {
        clausulas.push(ClausulaPaz::PazBranca);
    }

    let proposta = PropostaPaz {
        guerra_id: guerra_id.to_string(),
        proponente_tag: prop_tag,
        clausulas,
    };

    guerra_sim
        .0
        .propor_paz(proposta)
        .map_err(|e| format!("Erro ao assinar paz: {e}"))
}

#[cfg(test)]
mod testes {
    use super::*;
    use sim_guerra::{ParametrosCenarioSintetico, gerar_cenario_sintetico};

    /// Guerra headless entre 2 países que termina (por colapso, armistício ou paz proposta).
    #[test]
    fn guerra_entre_dois_paises_termina() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sim_guerra/tests/fixtures");
        let cat = CatalogoGuerra::carregar_diretorio(&dir).expect("catalogo");
        let params = ParametrosCenarioSintetico { num_regioes: 50, num_paises: 2, num_guerras: 1, divisoes_por_pais: 8, seed: 19450815 };
        let mut g = GuerraSim(MundoGuerra::novo(gerar_cenario_sintetico(cat, params)).unwrap());
        for _ in 0..3000 {
            if g.0.guerras().iter().all(|w| !w.ativa) {
                break;
            }
            g.0.executar_tick_diario();
        }
        if let Some(w) = g.0.guerras().iter().find(|w| w.ativa).cloned() {
            let prop = w.atacantes[0].clone();
            acao_propor_paz(&mut g, &w.id, &prop, true, true).expect("paz");
        }
        assert!(g.0.guerras().iter().all(|w| !w.ativa));
        assert!(!g.0.frentes().iter().any(|f| f.ativa));
    }

    use simulacao::mundo::MundoJogo;
    use std::time::Instant;

    /// Versão ORIGINAL (antes da otimização) da sincronização do plugin, mantida só para
    /// provar que a nova produz exatamente o mesmo resultado e para medir o ganho.
    fn tick_legado(
        guerra: &mut MundoGuerra,
        mundo: &mut MundoJogo,
        mut cores: Option<&mut CoresMapa>,
        com_economia: bool,
    ) -> bool {
    let rel = guerra.executar_tick_diario();

    // 1. Consumo simples de bens militares e impacto no tesouro
    if com_economia {
        for pais in guerra.cenario.paises.iter_mut() {
            let divs_ativas = guerra.cenario.divisoes.iter().filter(|d| d.pais_tag == pais.tag && d.efetivos_atuais > 0).count();
            if divs_ativas > 0 {
                // Manutenção diária simples: 0.05M por divisão
                let custo_manutencao = divs_ativas as f64 * 0.05;
                if let Some(p_jogo) = mundo.paises.get_mut(&pais.tag) {
                    p_jogo.orcamento.tesouro -= custo_manutencao;
                    if p_jogo.orcamento.tesouro < 0.0 {
                        pais.estoque_suprimento = (pais.estoque_suprimento - 0.005).max(0.40);
                    } else {
                        pais.estoque_suprimento = (pais.estoque_suprimento + 0.01).min(1.0);
                    }
                }
            }
        }
    }

    // 2. Sincroniza ocupação territorial das províncias
    let mut mudou_territorio = false;
    for reg in &guerra.cenario.regioes {
        if let Some(id_str) = reg.adm1_code.strip_prefix("prov_")
            && let Ok(id) = id_str.parse::<u32>()
        {
            let ctrl_antigo = mundo.controlador_por_provincia.get(&id).cloned().unwrap_or_default();
            if ctrl_antigo != reg.controlador_tag {
                mundo.controlador_por_provincia.insert(id, reg.controlador_tag.clone());
                mudou_territorio = true;

                if let Some(ref mut c) = cores {
                    let cor_ocup = if reg.esta_ocupada() {
                        mundo.paises.get(&reg.controlador_tag).map(|p| [p.cor.0, p.cor.1, p.cor.2, 255])
                    } else {
                        None
                    };
                    if (id as usize) < c.ocupante_cor.len() {
                        c.ocupante_cor[id as usize] = cor_ocup;
                    }
                }
            }
        }
    }

    // 3. Sincroniza transferências soberanas de tratados de paz assinados
    if rel.tratados_assinados > 0 {
        mudou_territorio = true;
        for tratado in &guerra.historico_tratados {
            for clausula in &tratado.clausulas_executadas {
                if let ClausulaPaz::AnexarRegioes { vencedor_tag, regioes_adm1, .. } = clausula {
                    for reg_code in regioes_adm1 {
                        if let Some(id_str) = reg_code.strip_prefix("prov_")
                            && let Ok(id) = id_str.parse::<u32>()
                        {
                            mundo.dono_por_provincia.insert(id, vencedor_tag.clone());
                            mundo.controlador_por_provincia.insert(id, vencedor_tag.clone());
                            if let Some(ref mut c) = cores {
                                if let Some(p) = mundo.paises.get(vencedor_tag)
                                    && (id as usize) < c.cor_prov.len()
                                {
                                    c.cor_prov[id as usize] = [p.cor.0, p.cor.1, p.cor.2, 255];
                                    c.ocupante_cor[id as usize] = None;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

        mudou_territorio
    }

    fn carregar_1945() -> (MundoGuerra, MundoJogo) {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (catalogo, mundo) = MundoJogo::carregar_de_arquivos(
            raiz.join("assets/map/provinces.ron"),
            raiz.join("data/scenario_1945/countries_1945.ron"),
            raiz.join("data/scenario_1945/province_owners_1945.ron"),
        )
        .expect("cenário 1945");
        let catalogo_mapa: gf_mapa::mapa::CatalogoProvincias = ron::from_str(
            &std::fs::read_to_string(raiz.join("assets/map/provinces.ron")).expect("provinces.ron"),
        )
        .expect("provinces.ron inválido");
        drop(catalogo);
        let (g, _) = construir_guerra_1945(&raiz, &catalogo_mapa, &mundo).expect("guerra 1945");
        (g, mundo)
    }

    fn resumo(g: &MundoGuerra, m: &MundoJogo) -> String {
        let mut ctrl: Vec<_> = m.controlador_por_provincia.iter().collect();
        ctrl.sort();
        let mut dono: Vec<_> = m.dono_por_provincia.iter().collect();
        dono.sort();
        let tesouros: Vec<_> = m.paises.iter().map(|(t, p)| (t.clone(), p.orcamento.tesouro.to_bits())).collect();
        format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            ctrl,
            dono,
            tesouros,
            g.paises(),
            g.guerras(),
            g.divisoes()
        )
    }

    /// A sincronização otimizada deve dar o mesmo estado que a original, em paz e em guerra.
    #[test]
    fn tick_otimizado_igual_ao_legado_com_dados_reais() {
        let (mut g_novo, mut m_novo) = carregar_1945();
        let (mut g_old, mut m_old) = carregar_1945();
        // Guerra real entre duas potências vizinhas depois de alguns dias de paz
        let mut ids = Vec::new();
        for dia in 0..50 {
            if dia == 10 {
                for g in [&mut g_novo, &mut g_old] {
                    let _ = g.declarar_guerra("SOV", "JAP", "Guerra de teste");
                }
            }
            let a = aplicar_tick_guerra(&mut g_novo, &mut m_novo, None, true, &mut ids);
            let b = tick_legado(&mut g_old, &mut m_old, None, true);
            assert_eq!(a, b, "dia {dia}: flag de território divergiu");
        }
        assert_eq!(resumo(&g_novo, &m_novo), resumo(&g_old, &m_old));
    }

    /// Micro-benchmark do custo por dia do tick de guerra no cenário real de 1945 (paz).
    /// Rodar: `cargo test --release -p guerra_fria bench_tick_guerra_1945 -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_tick_guerra_1945() {
        let dias = 230;
        let (mut g, mut m) = carregar_1945();
        println!("regioes={} paises={} divisoes={}", g.regioes().len(), g.paises().len(), g.divisoes().len());
        let mut ids = Vec::new();
        for _ in 0..5 {
            aplicar_tick_guerra(&mut g, &mut m, None, true, &mut ids);
        }
        let t = Instant::now();
        for _ in 0..dias {
            aplicar_tick_guerra(&mut g, &mut m, None, true, &mut ids);
        }
        let novo = t.elapsed().as_secs_f64() * 1000.0 / dias as f64;

        let (mut g2, mut m2) = carregar_1945();
        for _ in 0..5 {
            tick_legado(&mut g2, &mut m2, None, true);
        }
        let t = Instant::now();
        for _ in 0..dias {
            tick_legado(&mut g2, &mut m2, None, true);
        }
        let legado = t.elapsed().as_secs_f64() * 1000.0 / dias as f64;
        println!("BENCH tick_guerra 1945 (paz): sincronizacao legada={legado:.3} ms/dia, otimizada={novo:.3} ms/dia");
    }
}
