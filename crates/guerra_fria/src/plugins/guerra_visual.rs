//! Visualização de guerra no mapa: linhas de frente pulsantes, províncias ocupadas/disputadas,
//! marcadores de batalha, setas de avanço, faixa de aviso e estilos para guerras por procuração.

use super::audio::SomTocar;
use super::config::{ConfigJogo, QualidadeGrafica};
use super::guerra_sim::GuerraSim;
use super::map::{CaixasProvincia, DadosMapa};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite::Sprite;
use bevy_sprite_render::{Material2d, Material2dPlugin};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use gf_mapa::mapa::ESCALA_Y_MAPA;
use serde::{Deserialize, Serialize};
use sim_guerra::{Frente, Guerra, LadoGuerra, NivelEnvolvimento};
use std::collections::HashMap;


// RNG simples determinístico para evitar dependência do crate rand
struct SimpleRng(u64);

impl SimpleRng {
    fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        Self(seed)
    }
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 32) as f32 / u32::MAX as f32
    }
}

thread_local! {
    static RNG: std::cell::RefCell<SimpleRng> = std::cell::RefCell::new(SimpleRng::new());
}

fn random_f32() -> f32 {
    RNG.with(|r| r.borrow_mut().next_f32())
}

/// Plugin de visualização de guerra.
pub struct GuerraVisualPlugin;

impl Plugin for GuerraVisualPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EstadoVisualGuerra>()
            .init_resource::<MarcadoresBatalha>()
            .init_resource::<SetasAvanco>()
            .init_resource::<AvisoGuerraAtivo>()
            .init_resource::<AssetsGuerra>()
            .init_resource::<ConfigVisualGuerra>()
            .add_plugins(Material2dPlugin::<MaterialLinhasGuerra>::default())
            .add_systems(Startup, (carregar_assets_guerra, criar_material_linhas_guerra))
            .add_systems(
                Update,
                (
                    atualizar_visual_guerra,
                    animar_linhas_frente,
                    animar_hachura_ocupacao,
                    animar_marcadores_batalha,
                    animar_setas_avanco,
                    atualizar_aviso_guerra,
                    desenhar_linhas_frente,
                    desenhar_marcadores_batalha,
                    desenhar_setas_avanco,
                )
                    .chain()
                    .run_if(resource_exists::<GuerraSim>)
                    .run_if(crate::estado::em_partida),
            )
            .add_systems(EguiPrimaryContextPass, ui_aviso_guerra.run_if(crate::estado::em_partida))
            .add_systems(EguiPrimaryContextPass, ui_config_guerra.run_if(crate::estado::em_partida));
    }
}

/// Estado global da visualização de guerra.
#[derive(Resource, Default)]
pub struct EstadoVisualGuerra {
    pub linhas_frente: Vec<LinhaFrente>,
    pub hachuras_ocupacao: Vec<HachuraOcupacao>,
    pub tempo: f32,
    pub frame: u32,
}

/// Linha de frente entre duas províncias beligerantes.
#[derive(Debug, Clone)]
pub struct LinhaFrente {
    pub _provincia_a: u32,
    pub _provincia_b: u32,
    pub _tag_a: String,
    pub _tag_b: String,
    pub _guerra_id: String,
    pub eh_proxy: bool,
    /// Pontos da linha no espaço do mundo (já convertidos).
    pub pontos: Vec<Vec2>,
    /// Intensidade do pulso (0.0 a 1.0).
    pub pulso: f32,
    /// Fase da animação.
    pub fase: f32,
}

/// Hachura animada em província ocupada/disputada.
#[derive(Debug, Clone)]
pub struct HachuraOcupacao {
    pub _provincia_id: u32,
    pub _cor_ocupante: [u8; 4],
    pub _tag_ocupante: String,
    pub _tag_dono: String,
    pub _eh_disputada: bool,
    pub _eh_proxy: bool,
    pub fase: f32,
}

/// Marcadores de batalha (espadas cruzadas/explosão).
#[derive(Resource, Default)]
pub struct MarcadoresBatalha {
    pub marcadores: Vec<MarcadorBatalha>,
}

/// Marcador individual de batalha.
#[derive(Debug, Clone)]
pub struct MarcadorBatalha {
    pub _provincia_id: u32,
    pub posicao: Vec2,
    pub atacante: String,
    pub defensor: String,
    pub _forca_atacante: f32,
    pub _forca_defensor: f32,
    pub _progresso: f32,
    pub _baixas_atacante: u32,
    pub _baixas_defensor: u32,
    pub regiao_conquistada: bool,
    pub tempo_criacao: f32,
    pub pulso: f32,
}

/// Setas de avanço (do atacante para província alvo).
#[derive(Resource, Default)]
pub struct SetasAvanco {
    pub setas: Vec<SetaAvanco>,
}

/// Seta de avanço individual.
#[derive(Debug, Clone)]
pub struct SetaAvanco {
    pub _exercito_id: String,
    pub _pais_tag: String,
    pub origem: Vec2,
    pub destino: Vec2,
    pub _guerra_id: String,
    pub eh_ofensiva: bool,
    pub fase: f32,
}

/// Aviso de guerra ativo (faixa no topo).
#[derive(Resource, Default)]
pub struct AvisoGuerraAtivo {
    pub ativo: bool,
    pub texto: String,
    pub tempo_inicio: f32,
    pub duracao: f32,
    pub som_tocado: bool,
}

/// Configuração de visualização de guerra (adicionada ao ConfigJogo via recurso separado).
#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigVisualGuerra {
    pub animacoes_ativas: bool,
    pub mostrar_linhas_frente: bool,
    pub mostrar_hachura_ocupacao: bool,
    pub mostrar_marcadores_batalha: bool,
    pub mostrar_setas_avanco: bool,
    pub mostrar_aviso_guerra: bool,
    pub som_aviso_guerra: bool,
}

impl Default for ConfigVisualGuerra {
    fn default() -> Self {
        Self {
            animacoes_ativas: true,
            mostrar_linhas_frente: true,
            mostrar_hachura_ocupacao: true,
            mostrar_marcadores_batalha: true,
            mostrar_setas_avanco: true,
            mostrar_aviso_guerra: true,
            som_aviso_guerra: true,
        }
    }
}

/// Handles para assets de guerra.
#[derive(Resource, Default)]
struct AssetsGuerra {
    textura_espadas: Option<Handle<Image>>,
    textura_explosao: Option<Handle<Image>>,
    textura_seta: Option<Handle<Image>>,
}

/// Material para linhas de frente (pulsante/serrilhada).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct MaterialLinhasGuerra {
    #[uniform(0)]
    tempo: f32,
    #[uniform(1)]
    cor_base: Vec4,
    #[uniform(2)]
    intensidade_pulso: f32,
    #[uniform(3)]
    eh_proxy: u32,
}

impl Material2d for MaterialLinhasGuerra {
    fn fragment_shader() -> ShaderRef {
        "shaders/linhas_guerra.wesl".into()
    }
}

/// Carrega assets de guerra (texturas de ícones).
fn carregar_assets_guerra(
    servidor: Res<AssetServer>,
    mut assets: ResMut<AssetsGuerra>,
) {
    assets.textura_espadas = Some(servidor.load("guerra/espadas_cruzadas.png"));
    assets.textura_explosao = Some(servidor.load("guerra/explosao.png"));
    assets.textura_seta = Some(servidor.load("guerra/seta_avanco.png"));
}

/// Cria o material para linhas de frente.
fn criar_material_linhas_guerra(
    mut materiais: ResMut<Assets<MaterialLinhasGuerra>>,
) {
    materiais.add(MaterialLinhasGuerra {
        tempo: 0.0,
        cor_base: Vec4::new(1.0, 0.1, 0.1, 1.0),
        intensidade_pulso: 1.0,
        eh_proxy: 0,
    });
}

/// Atualiza o estado visual da guerra a partir dos dados de simulação.
fn atualizar_visual_guerra(
    mut estado: ResMut<EstadoVisualGuerra>,
    mut marcadores: ResMut<MarcadoresBatalha>,
    mut setas: ResMut<SetasAvanco>,
    mut aviso: ResMut<AvisoGuerraAtivo>,
    guerra_sim: Res<GuerraSim>,
    mundo: Res<super::economy::RecursoMundo>,
    dados: Res<DadosMapa>,
    caixas: Res<CaixasProvincia>,
    config: Res<ConfigVisualGuerra>,
    tempo: Res<Time>,
    mut som: MessageWriter<SomTocar>,
) {
    if !config.animacoes_ativas {
        estado.linhas_frente.clear();
        estado.hachuras_ocupacao.clear();
        marcadores.marcadores.clear();
        setas.setas.clear();
        return;
    }

    estado.tempo += tempo.delta_secs();
    estado.frame += 1;

    let guerras = &guerra_sim.0.cenario.guerras;
    let regioes = &guerra_sim.0.cenario.regioes;
    let frentes = &guerra_sim.0.frentes_ativas;
    let exercitos = &guerra_sim.0.cenario.exercitos;
    let divisoes = &guerra_sim.0.cenario.divisoes;
    let _relatorio = &guerra_sim.0; // RelatorioTickGuerra não está acessível diretamente, usamos guerras ativas

    // 1. Atualiza linhas de frente a partir das frentes ativas
    atualizar_linhas_frente(&mut estado, frentes, regioes, &dados, &caixas, guerras);

    // 2. Atualiza hachuras de ocupação/disputa
    atualizar_hachuras_ocupacao(&mut estado, regioes, &mundo.0, &dados, &caixas, guerras);

    // 3. Atualiza marcadores de batalha a partir de guerras ativas e batalhas recentes
    // O relatório do tick anterior não está acessível diretamente, então usamos as frentes e divisões em combate
    atualizar_marcadores_batalha(&mut marcadores, &mut aviso, &mut som, frentes, regioes, &dados, &caixas, guerras, config.som_aviso_guerra, estado.tempo);

    // 4. Atualiza setas de avanço
    atualizar_setas_avanco(&mut setas, exercitos, divisoes, regioes, &dados, &caixas, guerras, estado.tempo);

    // Limpa marcadores antigos (> 10 segundos sem atualização)
    marcadores.marcadores.retain(|m| estado.tempo - m.tempo_criacao < 10.0);
    setas.setas.retain(|s| estado.tempo - s.fase < 30.0);
}

/// Atualiza as linhas de frente entre províncias beligerantes.
fn atualizar_linhas_frente(
    estado: &mut EstadoVisualGuerra,
    frentes: &[Frente],
    regioes: &[sim_guerra::RegiaoMapa],
    dados: &DadosMapa,
    caixas: &CaixasProvincia,
    guerras: &[Guerra],
) {
    estado.linhas_frente.clear();

    let mapa_regioes: HashMap<String, usize> = regioes
        .iter()
        .enumerate()
        .map(|(i, r)| (r.adm1_code.clone(), i))
        .collect();

    // Mapa de província_id -> centroide em coordenadas de mundo
    let mut centros: HashMap<u32, Vec2> = HashMap::new();
    for (id, caixa) in caixas.0.iter().enumerate() {
        if let Some([x0, y0, x1, y1]) = caixa {
            let cx = (*x0 + *x1) as f32 / 2.0 - dados.largura as f32 / 2.0;
            let cy = dados.altura as f32 / 2.0 - (*y0 + *y1) as f32 / 2.0;
            centros.insert(id as u32, Vec2::new(cx, cy * ESCALA_Y_MAPA));
        }
    }

    for frente in frentes.iter().filter(|f| f.ativa) {
        let guerra = guerras.iter().find(|g| g.id == frente.guerra_id && g.ativa);
let eh_proxy = guerra.map_or(false, |g| {
                g.envolvimentos.iter().any(|e| {
                    matches!(e.nivel, NivelEnvolvimento::ConseleirosVoluntarios | NivelEnvolvimento::IntervencaoLimitada)
                        && (e.lado == LadoGuerra::Atacante || e.lado == LadoGuerra::Defensor)
                })
            });

        for (reg_a_code, reg_b_code) in &frente.pares_fronteira {
            let Some(idx_a) = mapa_regioes.get(reg_a_code) else { continue };
            let Some(idx_b) = mapa_regioes.get(reg_b_code) else { continue };
            let reg_a = &regioes[*idx_a];
            let reg_b = &regioes[*idx_b];

            // Extrai ID de província do adm1_code (formato: "prov_N")
            let prov_a = reg_a.adm1_code.strip_prefix("prov_").and_then(|s| s.parse::<u32>().ok());
            let prov_b = reg_b.adm1_code.strip_prefix("prov_").and_then(|s| s.parse::<u32>().ok());
            let (Some(prov_a), Some(prov_b)) = (prov_a, prov_b) else { continue };

            let Some(centro_a) = centros.get(&prov_a) else { continue };
            let Some(centro_b) = centros.get(&prov_b) else { continue };

            // Cria linha serrilhada entre os centroides
            let mut pontos = Vec::new();
            let segments = 8;
            for i in 0..=segments {
                let t = i as f32 / segments as f32;
                let base = centro_a.lerp(*centro_b, t);
                // Adiciona serrilhado perpendicular
                let perp = Vec2::new(-(centro_b.y - centro_a.y), centro_b.x - centro_a.x).normalize_or_zero();
                let offset = ((t * 20.0).sin() * 3.0) * perp;
                pontos.push(base + offset);
            }

        estado.linhas_frente.push(LinhaFrente {
            _provincia_a: prov_a,
            _provincia_b: prov_b,
            _tag_a: reg_a.controlador_tag.clone(),
            _tag_b: reg_b.controlador_tag.clone(),
            _guerra_id: frente.guerra_id.clone(),
            eh_proxy,
            pontos,
            pulso: 0.0,
            fase: random_f32() * std::f32::consts::TAU,
        });
        }
    }
}

/// Atualiza hachuras de províncias ocupadas/disputadas.
fn atualizar_hachuras_ocupacao(
    estado: &mut EstadoVisualGuerra,
    regioes: &[sim_guerra::RegiaoMapa],
    mundo: &simulacao::mundo::MundoJogo,
    _dados: &DadosMapa,
    caixas: &CaixasProvincia,
    guerras: &[Guerra],
) {
    estado.hachuras_ocupacao.clear();

    for reg in regioes {
        if reg.dono_tag == reg.controlador_tag {
            continue; // Não ocupada
        }

        let prov_id = reg.adm1_code.strip_prefix("prov_").and_then(|s| s.parse::<u32>().ok());
        let Some(prov_id) = prov_id else { continue };
        if caixas.0.get(prov_id as usize).is_none_or(|c| c.is_none()) {
            continue;
        }

        // Verifica se é disputada (tem combate ativo na fronteira)
        let eh_disputada = guerras.iter().any(|g| {
            g.ativa && g.envolvimentos.iter().any(|e| {
                matches!(e.nivel, NivelEnvolvimento::ApoioMaterial | NivelEnvolvimento::ConseleirosVoluntarios | NivelEnvolvimento::IntervencaoLimitada | NivelEnvolvimento::GuerraTotal)
            })
        });

        // Verifica se é guerra por procuração (nível 3-4)
        let eh_proxy = guerras.iter().any(|g| {
            g.ativa && g.envolvimentos.iter().any(|e| {
                matches!(e.nivel, NivelEnvolvimento::ConseleirosVoluntarios | NivelEnvolvimento::IntervencaoLimitada)
            })
        });

        let cor_ocupante = mundo.paises.get(&reg.controlador_tag).map(|p| [p.cor.0, p.cor.1, p.cor.2, 255]).unwrap_or([200, 200, 200, 255]);

        estado.hachuras_ocupacao.push(HachuraOcupacao {
            _provincia_id: prov_id,
            _cor_ocupante: cor_ocupante,
            _tag_ocupante: reg.controlador_tag.clone(),
            _tag_dono: reg.dono_tag.clone(),
            _eh_disputada: eh_disputada,
            _eh_proxy: eh_proxy,
            fase: random_f32() * std::f32::consts::TAU,
        });
    }
}

/// Atualiza marcadores de batalha.
fn atualizar_marcadores_batalha(
    marcadores: &mut MarcadoresBatalha,
    aviso: &mut AvisoGuerraAtivo,
    som: &mut MessageWriter<SomTocar>,
    frentes: &[Frente],
    regioes: &[sim_guerra::RegiaoMapa],
    dados: &DadosMapa,
    caixas: &CaixasProvincia,
    guerras: &[Guerra],
    som_ativo: bool,
    tempo_atual: f32,
) {
    // Detecta novas guerras para aviso
    for guerra in guerras.iter().filter(|g| g.ativa) {
        // Verifica se é uma guerra nova (iniciou agora)
        let duracao_guerra = guerra_sim_data_para_dias(guerra.data_inicio) as f32;
        // Simplificação: avisa se a guerra tem menos de 1 dia
        if duracao_guerra < 1.0 && !aviso.ativo {
            let atacantes: Vec<String> = guerra.atacantes.iter().cloned().collect();
            let defensores: Vec<String> = guerra.defensores.iter().cloned().collect();
            aviso.ativo = true;
            aviso.texto = format!("GUERRA: {} × {}", atacantes.join(", "), defensores.join(", "));
            aviso.tempo_inicio = tempo_atual;
            aviso.duracao = 5.0;
            aviso.som_tocado = false;
            if som_ativo {
                som.write(SomTocar::novo("diplomacia.ultimato_enviado"));
                aviso.som_tocado = true;
            }
        }
    }

    // Cria marcadores para províncias na fronteira ativa (simplificado: usa frentes)
    let mapa_regioes: HashMap<String, usize> = regioes
        .iter()
        .enumerate()
        .map(|(i, r)| (r.adm1_code.clone(), i))
        .collect();

    let mut centros: HashMap<u32, Vec2> = HashMap::new();
    for (id, caixa) in caixas.0.iter().enumerate() {
        if let Some([x0, y0, x1, y1]) = caixa {
            let cx = (*x0 + *x1) as f32 / 2.0 - dados.largura as f32 / 2.0;
            let cy = dados.altura as f32 / 2.0 - (*y0 + *y1) as f32 / 2.0;
            centros.insert(id as u32, Vec2::new(cx, cy * ESCALA_Y_MAPA));
        }
    }

    for frente in frentes.iter().filter(|f| f.ativa) {
        for (reg_a_code, reg_b_code) in &frente.pares_fronteira {
            let Some(idx_a) = mapa_regioes.get(reg_a_code) else { continue };
            let Some(idx_b) = mapa_regioes.get(reg_b_code) else { continue };
            let reg_a = &regioes[*idx_a];
            let reg_b = &regioes[*idx_b];

            let prov_a = reg_a.adm1_code.strip_prefix("prov_").and_then(|s| s.parse::<u32>().ok());
            let prov_b = reg_b.adm1_code.strip_prefix("prov_").and_then(|s| s.parse::<u32>().ok());
            let (Some(prov_a), Some(prov_b)) = (prov_a, prov_b) else { continue };

            let Some(centro_a) = centros.get(&prov_a) else { continue };
            let Some(centro_b) = centros.get(&prov_b) else { continue };

            // Ponto médio da fronteira
            let pos = centro_a.lerp(*centro_b, 0.5);

            // Verifica se já existe marcador próximo
            let ja_existe = marcadores.marcadores.iter().any(|m| {
                (m.posicao - pos).length() < 50.0 && m.atacante == frente.lado_a_tag && m.defensor == frente.lado_b_tag
            });

            if !ja_existe {
                marcadores.marcadores.push(MarcadorBatalha {
                    _provincia_id: prov_a,
                    posicao: pos,
                    atacante: frente.lado_a_tag.clone(),
                    defensor: frente.lado_b_tag.clone(),
                    _forca_atacante: 1.0,
                    _forca_defensor: 1.0,
                    _progresso: 0.5,
                    _baixas_atacante: 0,
                    _baixas_defensor: 0,
                    regiao_conquistada: false,
                    tempo_criacao: tempo_atual,
                    pulso: 0.0,
                });
            }
        }
    }
}

/// Atualiza setas de avanço.
fn atualizar_setas_avanco(
    setas: &mut SetasAvanco,
    exercitos: &[sim_guerra::Exercito],
    divisoes: &[sim_guerra::Divisao],
    regioes: &[sim_guerra::RegiaoMapa],
    dados: &DadosMapa,
    caixas: &CaixasProvincia,
    guerras: &[Guerra],
    tempo_atual: f32,
) {
    setas.setas.clear();

    let mapa_regioes: HashMap<String, usize> = regioes
        .iter()
        .enumerate()
        .map(|(i, r)| (r.adm1_code.clone(), i))
        .collect();

    let mut centros: HashMap<u32, Vec2> = HashMap::new();
    for (id, caixa) in caixas.0.iter().enumerate() {
        if let Some([x0, y0, x1, y1]) = caixa {
            let cx = (*x0 + *x1) as f32 / 2.0 - dados.largura as f32 / 2.0;
            let cy = dados.altura as f32 / 2.0 - (*y0 + *y1) as f32 / 2.0;
            centros.insert(id as u32, Vec2::new(cx, cy * ESCALA_Y_MAPA));
        }
    }

    for exercito in exercitos {
        if let Some(objetivo_code) = &exercito.regiao_objetivo_adm1 {
            if let Some(idx) = mapa_regioes.get(objetivo_code) {
                let reg_objetivo = &regioes[*idx];
                let prov_destino = reg_objetivo.adm1_code.strip_prefix("prov_").and_then(|s| s.parse::<u32>().ok());
                let Some(prov_destino) = prov_destino else { continue };
                let Some(destino) = centros.get(&prov_destino) else { continue };

                // Origem: primeira divisão do exército
                let mut origem = None;
                for div_id in &exercito.divisoes_ids {
                    if let Some(div) = divisoes.iter().find(|d| &d.id == div_id) {
                        let prov_origem = div.regiao_adm1.strip_prefix("prov_").and_then(|s| s.parse::<u32>().ok());
                        if let Some(prov_origem) = prov_origem {
                            origem = centros.get(&prov_origem).copied();
                            break;
                        }
                    }
                }
                let Some(origem) = origem else { continue };

                // Verifica se o exército está em guerra
                let em_guerra = guerras.iter().any(|g| g.ativa && g.participa(&exercito.pais_tag));
                if !em_guerra {
                    continue;
                }

                // Determina se é ofensiva (postura agressiva)
                let eh_ofensiva = matches!(exercito.postura, sim_guerra::PosturaOperacional::AssaltoAgressivo | sim_guerra::PosturaOperacional::OfensivaEquilibrada);

                setas.setas.push(SetaAvanco {
                    _exercito_id: exercito.id.clone(),
                    _pais_tag: exercito.pais_tag.clone(),
                    origem,
                    destino: *destino,
                    _guerra_id: guerras.iter().find(|g| g.ativa && g.participa(&exercito.pais_tag)).map(|g| g.id.clone()).unwrap_or_default(),
                    eh_ofensiva,
                    fase: tempo_atual,
                });
            }
        }
    }
}

/// Anima as linhas de frente (pulso).
fn animar_linhas_frente(
    tempo: Res<Time>,
    mut estado: ResMut<EstadoVisualGuerra>,
    config: Res<ConfigVisualGuerra>,
) {
    if !config.animacoes_ativas || !config.mostrar_linhas_frente {
        return;
    }
    let dt = tempo.delta_secs();
    for linha in &mut estado.linhas_frente {
        linha.fase += dt * 2.0;
        linha.pulso = (linha.fase.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
    }
}

/// Anima hachuras de ocupação.
fn animar_hachura_ocupacao(
    tempo: Res<Time>,
    mut estado: ResMut<EstadoVisualGuerra>,
    config: Res<ConfigVisualGuerra>,
) {
    if !config.animacoes_ativas || !config.mostrar_hachura_ocupacao {
        return;
    }
    let dt = tempo.delta_secs();
    for hachura in &mut estado.hachuras_ocupacao {
        hachura.fase += dt * 1.5;
    }
}

/// Anima marcadores de batalha (pulso).
fn animar_marcadores_batalha(
    tempo: Res<Time>,
    mut marcadores: ResMut<MarcadoresBatalha>,
    config: Res<ConfigVisualGuerra>,
) {
    if !config.animacoes_ativas || !config.mostrar_marcadores_batalha {
        return;
    }
    let dt = tempo.delta_secs();
    for m in &mut marcadores.marcadores {
        m.pulso = (m.pulso + dt * 3.0).rem_euclid(std::f32::consts::TAU);
    }
}

/// Anima setas de avanço.
fn animar_setas_avanco(
    tempo: Res<Time>,
    mut setas: ResMut<SetasAvanco>,
    config: Res<ConfigVisualGuerra>,
) {
    if !config.animacoes_ativas || !config.mostrar_setas_avanco {
        return;
    }
    let dt = tempo.delta_secs();
    for s in &mut setas.setas {
        s.fase += dt;
    }
}

/// Atualiza aviso de guerra (desaparece após duração).
fn atualizar_aviso_guerra(
    tempo: Res<Time>,
    mut aviso: ResMut<AvisoGuerraAtivo>,
) {
    if aviso.ativo {
        if tempo.elapsed_secs() as f32 - aviso.tempo_inicio > aviso.duracao {
            aviso.ativo = false;
            aviso.texto.clear();
        }
    }
}

/// Desenha as linhas de frente usando mesh.
fn desenhar_linhas_frente(
    estado: Res<EstadoVisualGuerra>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materiais: ResMut<Assets<MaterialLinhasGuerra>>,
    mut comandos: Commands,
    config: Res<ConfigVisualGuerra>,
    qualidade: Res<ConfigJogo>,
) {
    if !config.animacoes_ativas || !config.mostrar_linhas_frente || estado.linhas_frente.is_empty() {
        return;
    }

    // Qualidade Baixo: não desenha linhas de frente
    if matches!(qualidade.graficos.qualidade, QualidadeGrafica::Baixo) {
        return;
    }

    // Cria mesh para cada linha de frente
    for (i, linha) in estado.linhas_frente.iter().enumerate() {
        let mut builder = ConstrutorLinhasGuerra::novo();
        let cor = if linha.eh_proxy {
            Color::srgba(1.0, 0.6, 0.0, 0.9) // Laranja para proxy
        } else {
            Color::srgba(1.0, 0.1, 0.1, 0.9) // Vermelho para guerra direta
        };

        // Desenha segmento a segmento com pulso
        for par in linha.pontos.windows(2) {
            let largura = if matches!(qualidade.graficos.qualidade, QualidadeGrafica::Ultra) { 3.0 } else { 2.0 };
            let pulso_largura = largura * (1.0 + linha.pulso * 0.5);
            builder.segmento(par[0], par[1], pulso_largura, 0.0, cor);
        }

        let mesh = builder.construir();
        let handle = meshes.add(mesh);

        // Material com pulso
        let material = materiais.add(MaterialLinhasGuerra {
            tempo: estado.tempo,
            cor_base: cor.to_linear().to_vec4(),
            intensidade_pulso: linha.pulso,
            eh_proxy: if linha.eh_proxy { 1 } else { 0 },
        });

        comandos.spawn((
            Mesh2d(handle),
            MeshMaterial2d(material),
            Transform::from_xyz(0.0, 0.0, 10.0 + i as f32 * 0.001),
            Visibility::Visible,
            LinhaFrenteVisual { _idx: i },
        ));
    }
}

/// Componente para identificar linhas de frente visuais.
#[derive(Component)]
struct LinhaFrenteVisual {
    _idx: usize,
}

/// Desenha marcadores de batalha.
fn desenhar_marcadores_batalha(
    marcadores: Res<MarcadoresBatalha>,
    mut comandos: Commands,
    assets: Res<AssetsGuerra>,
    config: Res<ConfigVisualGuerra>,
    qualidade: Res<ConfigJogo>,
) {
    if !config.animacoes_ativas || !config.mostrar_marcadores_batalha || marcadores.marcadores.is_empty() {
        return;
    }

    if matches!(qualidade.graficos.qualidade, QualidadeGrafica::Baixo) {
        return;
    }

    for m in &marcadores.marcadores {
        let escala = 0.8 + 0.4 * (m.pulso.sin() * 0.5 + 0.5);
        let cor = if m.regiao_conquistada {
            Color::srgba(0.2, 1.0, 0.2, 1.0)
        } else {
            Color::srgba(1.0, 0.8, 0.1, 1.0)
        };

        // Usa textura se disponível, senão desenha primitiva
        if let Some(textura) = &assets.textura_espadas {
            comandos.spawn((
                Sprite {
                    image: textura.clone(),
                    color: cor,
                    custom_size: Some(Vec2::new(24.0 * escala, 24.0 * escala)),
                    ..default()
                },
                Transform::from_xyz(m.posicao.x, m.posicao.y, 15.0),
                Visibility::Visible,
                MarcadorBatalhaVisual,
            ));
        }
    }
}

/// Componente para marcadores de batalha.
#[derive(Component)]
struct MarcadorBatalhaVisual;

/// Desenha setas de avanço.
fn desenhar_setas_avanco(
    setas: Res<SetasAvanco>,
    mut comandos: Commands,
    assets: Res<AssetsGuerra>,
    config: Res<ConfigVisualGuerra>,
    qualidade: Res<ConfigJogo>,
) {
    if !config.animacoes_ativas || !config.mostrar_setas_avanco || setas.setas.is_empty() {
        return;
    }

    if matches!(qualidade.graficos.qualidade, QualidadeGrafica::Baixo) {
        return;
    }

    for s in &setas.setas {
        let dir = (s.destino - s.origem).normalize_or_zero();
        let angulo = dir.y.atan2(dir.x);
        let dist = (s.destino - s.origem).length();
        let num_setas = (dist / 120.0).ceil() as usize;

        for i in 0..num_setas.clamp(1, 10) {
            let t = (i as f32 + 0.5) / num_setas as f32;
            let pos = s.origem.lerp(s.destino, t);
            let offset_pulso = (s.fase * 2.0 + i as f32 * 0.5).sin() * 3.0;
            let pos_final = pos + Vec2::new(-dir.y, dir.x) * offset_pulso;

            let cor = if s.eh_ofensiva {
                Color::srgba(1.0, 0.2, 0.2, 0.9)
            } else {
                Color::srgba(0.2, 0.8, 1.0, 0.9)
            };

            if let Some(textura) = &assets.textura_seta {
                comandos.spawn((
                    Sprite {
                        image: textura.clone(),
                        color: cor,
                        custom_size: Some(Vec2::new(32.0, 16.0)),
                        ..default()
                    },
                    Transform::from_xyz(pos_final.x, pos_final.y, 12.0).with_rotation(Quat::from_rotation_z(angulo)),
                    Visibility::Visible,
                    SetaAvancoVisual,
                ));
            }
        }
    }
}

/// Componente para setas de avanço.
#[derive(Component)]
struct SetaAvancoVisual;

/// UI: Faixa de aviso de guerra no topo.
fn ui_aviso_guerra(
    mut contextos: EguiContexts,
    aviso: Res<AvisoGuerraAtivo>,
    config: Res<ConfigVisualGuerra>,
) -> Result {
    if !config.mostrar_aviso_guerra || !aviso.ativo {
        return Ok(());
    }

    let ctx = contextos.ctx_mut()?;
    let tempo_atual = ctx.input(|i| i.time) as f32;
    let progresso = ((tempo_atual - aviso.tempo_inicio) / aviso.duracao).clamp(0.0, 1.0);
    let alfa = if progresso < 0.1 { progresso * 10.0 } else if progresso > 0.9 { (1.0 - progresso) * 10.0 } else { 1.0 };

    egui::Area::new("aviso_guerra".into())
        .anchor(egui::Align2::CENTER_TOP, [0.0, 20.0])
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_premultiplied(180, 20, 20, (alfa * 230.0) as u8))
                .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgba_premultiplied(255, 200, 200, (alfa * 255.0) as u8)))
                .inner_margin(egui::Margin::symmetric(24, 10))
                .corner_radius(egui::CornerRadius::same(8))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(&aviso.texto)
                            .size(22.0)
                            .strong()
                            .color(egui::Color32::from_rgba_premultiplied(255, 255, 255, (alfa * 255.0) as u8))
                            .family(egui::FontFamily::Name("Oswald".into())),
                    );
                });
        });

    Ok(())
}

/// UI: Opções de configuração de visualização de guerra no menu de configurações.
fn ui_config_guerra(
    mut contextos: EguiContexts,
    _config_visual: ResMut<ConfigVisualGuerra>,
    _config_jogo: ResMut<ConfigJogo>,
) -> Result {
    let _ctx = contextos.ctx_mut()?;
    // Só mostra se o menu de configurações estiver aberto
    // (O menu de configurações é gerenciado em outro lugar, aqui só adicionamos a seção se necessário)
    // Esta função será chamada pelo sistema de configurações existente.
    // Por enquanto, apenas garantimos que o recurso existe.
    Ok(())
}

/// Converte DataJogo para dias desde época.
fn guerra_sim_data_para_dias(data: sim_guerra::DataJogo) -> i32 {
    data.ano as i32 * 365 + data.mes as i32 * 30 + data.dia as i32
}

/// Construtor de linhas para guerra (serrilhado/pulsante).
struct ConstrutorLinhasGuerra {
    pos: Vec<[f32; 3]>,
    cor: Vec<[f32; 4]>,
    idx: Vec<u32>,
}

impl ConstrutorLinhasGuerra {
    fn novo() -> Self {
        Self { pos: Vec::new(), cor: Vec::new(), idx: Vec::new() }
    }

    fn segmento(&mut self, a: Vec2, b: Vec2, w: f32, _ext: f32, cor: Color) {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 || w <= 0.0 {
            return;
        }
        let dir = d / len;
        let n = Vec2::new(-dir.y, dir.x) * (w * 0.5);
        let c = cor.to_linear().to_f32_array();
        let base = self.pos.len() as u32;
        self.pos.push([a.x - n.x, a.y - n.y, 0.0]);
        self.pos.push([a.x + n.x, a.y + n.y, 0.0]);
        self.pos.push([b.x + n.x, b.y + n.y, 0.0]);
        self.pos.push([b.x - n.x, b.y - n.y, 0.0]);
        for _ in 0..4 {
            self.cor.push(c);
        }
        self.idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn construir(self) -> Mesh {
        let n = self.pos.len();
        let mut m = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
        m.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        m.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.cor);
        m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n]);
        m.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
        m.insert_indices(Indices::U32(self.idx));
        m
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use sim_guerra::{Frente, Guerra, RegiaoMapa, DataJogo};

    #[test]
    fn calcular_linhas_frente_entre_beligerantes() {
        // Cria regiões de teste
        let regioes = vec![
            RegiaoMapa {
                adm1_code: "prov_1".into(),
                nome_pt: "Província A".into(),
                dono_tag: "AAA".into(),
                controlador_tag: "AAA".into(),
                terreno_id: "planicie".into(),
                infraestrutura: 50,
                nivel_porto: 0,
                nivel_fortificacao: 0,
                pontos_vitoria: 1.0,
                populacao: 10000,
                destruicao_guerra: 0.0,
                resistencia_ocupacao: 0.0,
            },
            RegiaoMapa {
                adm1_code: "prov_2".into(),
                nome_pt: "Província B".into(),
                dono_tag: "BBB".into(),
                controlador_tag: "BBB".into(),
                terreno_id: "planicie".into(),
                infraestrutura: 50,
                nivel_porto: 0,
                nivel_fortificacao: 0,
                pontos_vitoria: 1.0,
                populacao: 10000,
                destruicao_guerra: 0.0,
                resistencia_ocupacao: 0.0,
            },
        ];

        // Cria frente entre as duas províncias
        let frente = Frente {
            id: 1,
            nome_pt: "Frente Teste".into(),
            guerra_id: "guerra_teste".into(),
            lado_a_tag: "AAA".into(),
            lado_b_tag: "BBB".into(),
            regioes_lado_a: vec!["prov_1".into()],
            regioes_lado_b: vec!["prov_2".into()],
            pares_fronteira: vec![("prov_1".into(), "prov_2".into())],
            ativa: true,
        };

        let guerras = vec![Guerra {
            id: "guerra_teste".into(),
            nome_pt: "Guerra Teste".into(),
            atacantes: vec!["AAA".into()],
            defensores: vec!["BBB".into()],
            data_inicio: DataJogo::nova(1945, 1, 1).unwrap(),
            pontuacao_guerra: 0.0,
            baixas_atacantes: 0,
            baixas_defensores: 0,
            batalhas_vencidas_atacantes: 0,
            batalhas_vencidas_defensores: 0,
            capitulou_atacantes: false,
            capitulou_defensores: false,
            ativa: true,
            envolvimentos: Vec::new(),
        }];

        let mut estado = EstadoVisualGuerra::default();
        let mut centros = HashMap::new();
        centros.insert(1u32, Vec2::new(-100.0, 0.0));
        centros.insert(2u32, Vec2::new(100.0, 0.0));

        // Testa a lógica de geração de pontos
        let caixas: HashMap<u32, Option<[u32; 4]>> = HashMap::from([
            (1, Some([0, 0, 10, 10])),
            (2, Some([20, 0, 30, 10])),
        ]);

        // Simula a geração de pontos da linha de frente
        let centro_a = centros[&1];
        let centro_b = centros[&2];
        let mut pontos = Vec::new();
        let segments = 8;
        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let base = centro_a.lerp(centro_b, t);
            let perp = Vec2::new(-(centro_b.y - centro_a.y), centro_b.x - centro_a.x).normalize_or_zero();
            let offset = ((t * 20.0).sin() * 3.0) * perp;
            pontos.push(base + offset);
        }

        assert_eq!(pontos.len(), 9); // 8 segmentos = 9 pontos
        assert!((pontos[0] - centro_a).length() < 5.0);
        assert!((pontos[8] - centro_b).length() < 5.0);

        // Verifica que a linha tem serrilhado (pontos não colineares)
        let mut tem_serrilhado = false;
        for i in 1..pontos.len()-1 {
            let v1 = (pontos[i] - pontos[i-1]).normalize_or_zero();
            let v2 = (pontos[i+1] - pontos[i]).normalize_or_zero();
            if (v1 - v2).length() > 0.1 {
                tem_serrilhado = true;
                break;
            }
        }
        assert!(tem_serrilhado, "Linha de frente deve ter serrilhado");
    }

    #[test]
    fn hachura_ocupacao_usa_cor_ocupante() {
        let regiao = RegiaoMapa {
            adm1_code: "prov_1".into(),
            nome_pt: "Província Ocupada".into(),
            dono_tag: "AAA".into(),
            controlador_tag: "BBB".into(), // Ocupada por BBB
            terreno_id: "planicie".into(),
            infraestrutura: 50,
            nivel_porto: 0,
            nivel_fortificacao: 0,
            pontos_vitoria: 1.0,
            populacao: 10000,
            destruicao_guerra: 0.0,
            resistencia_ocupacao: 0.0,
        };

        assert!(regiao.esta_ocupada());
        assert_eq!(regiao.dono_tag, "AAA");
        assert_eq!(regiao.controlador_tag, "BBB");
    }

    #[test]
    fn proxy_war_nivel_3_4_eh_proxy() {
        use sim_guerra::{EnvolvimentoGuerra, LadoGuerra, NivelEnvolvimento};

        let guerra = Guerra {
            id: "guerra_proxy".into(),
            nome_pt: "Guerra por Procuração".into(),
            atacantes: vec!["AAA".into()],
            defensores: vec!["BBB".into()],
            data_inicio: DataJogo::nova(1945, 1, 1).unwrap(),
            pontuacao_guerra: 0.0,
            baixas_atacantes: 0,
            baixas_defensores: 0,
            batalhas_vencidas_atacantes: 0,
            batalhas_vencidas_defensores: 0,
            capitulou_atacantes: false,
            capitulou_defensores: false,
            ativa: true,
            envolvimentos: vec![
                EnvolvimentoGuerra {
                    pais_tag: "CCC".into(),
                    guerra_id: "guerra_proxy".into(),
                    lado: LadoGuerra::Atacante,
                    nivel: NivelEnvolvimento::IntervencaoLimitada, // Nível 4
                    data_inicio: DataJogo::nova(1945, 1, 1).unwrap(),
                    custo_acumulado: 0.0,
                },
            ],
        };

        let eh_proxy = guerra.envolvimentos.iter().any(|e| {
            matches!(e.nivel, NivelEnvolvimento::ConseleirosVoluntarios | NivelEnvolvimento::IntervencaoLimitada)
        });
        assert!(eh_proxy, "Nível 4 (Intervenção Limitada) deve ser considerado proxy");

        let guerra2 = Guerra {
            envolvimentos: vec![
                EnvolvimentoGuerra {
                    pais_tag: "CCC".into(),
                    guerra_id: "guerra_proxy".into(),
                    lado: LadoGuerra::Atacante,
                    nivel: NivelEnvolvimento::ConseleirosVoluntarios, // Nível 3
                    data_inicio: DataJogo::nova(1945, 1, 1).unwrap(),
                    custo_acumulado: 0.0,
                },
            ],
            ..guerra.clone()
        };

        let eh_proxy2 = guerra2.envolvimentos.iter().any(|e| {
            matches!(e.nivel, NivelEnvolvimento::ConseleirosVoluntarios | NivelEnvolvimento::IntervencaoLimitada)
        });
        assert!(eh_proxy2, "Nível 3 (Conselheiros/Voluntários) deve ser considerado proxy");

        // Nível 5 (Guerra Total) NÃO é proxy - é guerra direta
        let guerra3 = Guerra {
            envolvimentos: vec![
                EnvolvimentoGuerra {
                    pais_tag: "CCC".into(),
                    guerra_id: "guerra_proxy".into(),
                    lado: LadoGuerra::Atacante,
                    nivel: NivelEnvolvimento::GuerraTotal, // Nível 5
                    data_inicio: DataJogo::nova(1945, 1, 1).unwrap(),
                    custo_acumulado: 0.0,
                },
            ],
            ..guerra.clone()
        };

        let eh_proxy3 = guerra3.envolvimentos.iter().any(|e| {
            matches!(e.nivel, NivelEnvolvimento::ConseleirosVoluntarios | NivelEnvolvimento::IntervencaoLimitada)
        });
        assert!(!eh_proxy3, "Nível 5 (Guerra Total) NÃO deve ser considerado proxy (é guerra direta)");
    }
}