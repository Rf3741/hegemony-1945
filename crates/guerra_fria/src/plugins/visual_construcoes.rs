//! Plugin de integração entre o motor econômico (`sim_economia`) e o mapa visual de construções (`visual_mapa`).
//!
//! - Inicializa o visual do mapa com os 40.884 prédios reais de 1945 e as 1.517 cidades mundiais.
//! - Mantém os prédios e cidades sincronizados com o estado de `sim_economia`.
//! - Quando novas obras são concluídas na simulação, atualiza `EntradaEconomiaVisual`, disparando
//!   animações de surgimento e subida de nível.
//! - Sincroniza o `AnoVisual` com a data do relógio para evolução de eras no mapa.
//! - Oferece atalho (tecla V) para ligar e desligar a camada visual com máximo desempenho.

use super::economia_sim::{EconomiaSim, EconomiaSimSet};
use super::map::{RaizProjeto, RecursoCatalogo};
use super::time::RecursoRelogio;
use bevy::prelude::*;
use std::collections::HashMap;
use visual_mapa::cenario::carregar_cidades_cenario;
use visual_mapa::entrada::{
    AnoVisual, BlocoVisual, CidadeEntrada, EntradaEconomiaVisual,
    EntradaVisual, PredioEconomiaEntrada, populacao_urbana_para_nivel,
};
use visual_mapa::VisualMapaPlugin;

/// Estado interno da ponte entre a economia e o mapa visual.
#[derive(Resource, Default)]
pub struct EstadoVisualConstrucoes {
    pub inicializado: bool,
    pub ultima_versao_construcao: u64,
    pub total_predios: usize,
    pub total_cidades: usize,
    /// Instante (s, tempo real) da última extração; limita a no máximo 1 a cada 2 s.
    pub ultima_extracao: f32,
}

pub struct VisualConstrucoesPlugin;

impl Plugin for VisualConstrucoesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EstadoVisualConstrucoes>()
            .add_plugins(VisualMapaPlugin)
            .add_systems(
                Update,
                (
                    sincronizar_dados_construcoes_sistema
                        .in_set(EconomiaSimSet::SincronizarConstrucoes)
                        .after(EconomiaSimSet::Recolher)
                        .before(EconomiaSimSet::Despachar),
                    sincronizar_relogio_visual_sistema,
                )
                    .run_if(resource_exists::<EconomiaSim>),
            );
    }
}

/// Sincroniza o ano contínuo do visual do mapa a partir do relógio do jogo.
fn sincronizar_relogio_visual_sistema(
    relogio: Option<Res<RecursoRelogio>>,
    tec: Option<Res<super::tecnologia_ui::TecnologiaSim>>,
    mut ano: ResMut<AnoVisual>,
) {
    if let Some(r) = relogio {
        let mut ano_calc = r.data.ano as f32 + (r.data.mes as f32 - 1.0) / 12.0;
        // Era visual do país do jogador (tecnologia): limita o ano à faixa da era alcançada (sem transição antes da hora).
        if let Some(t) = tec {
            let era = t.era_jogador as f32;
            ano_calc = ano_calc.clamp(era, era + 7.99);
        }
        if (ano.0 - ano_calc).abs() > 0.005 {
            ano.0 = ano_calc;
        }
    }
}

/// Popula inicialmente e atualiza os prédios e cidades do mapa a partir do `MundoEconomico`.
pub fn sincronizar_dados_construcoes_sistema(
    mut estado: ResMut<EstadoVisualConstrucoes>,
    eco: Res<EconomiaSim>,
    raiz: Option<Res<RaizProjeto>>,
    catalogo_mapa: Option<Res<RecursoCatalogo>>,
    mut entrada_eco: ResMut<EntradaEconomiaVisual>,
    mut entrada_vis: ResMut<EntradaVisual>,
    tempo: Res<Time<Real>>,
    mut coords_cache: Local<Option<HashMap<String, (f32, f32)>>>,
) {
    let Some(ref nucleo) = eco.nucleo else { return };
    let precisa = !estado.inicializado
        || (eco.versao_construcao != estado.ultima_versao_construcao && tempo.elapsed_secs() - estado.ultima_extracao >= 2.0);
    if !precisa {
        return;
    }

    // Tabela de coordenadas: código ADM1 ou ID de província -> centroide (lon, lat); montada uma só vez.
    if coords_cache.is_none() {
        *coords_cache = Some(
            catalogo_mapa
                .as_ref()
                .map(|cat| {
                    cat.0
                        .provincias
                        .iter()
                        .rev() // a 1ª província de cada estado (a do centroide original) prevalece
                        .map(|p| (p.adm1_code.clone(), p.centroide))
                        .collect()
                })
                .unwrap_or_default(),
        );
    }
    let coords = coords_cache.as_ref().unwrap();

    if !estado.inicializado {
        // 1. Carregar cidades demográficas de 1945
        let caminho_cidades = raiz
            .as_ref()
            .map(|r| r.0.join("assets/data/territorio/cidades.ron"))
            .unwrap_or_else(|| std::path::PathBuf::from("assets/data/territorio/cidades.ron"));
        let cidades = carregar_cidades_cenario(&caminho_cidades);

        for c in &cidades {
            let niv = populacao_urbana_para_nivel(c.populacao_urbana);
            entrada_vis.cidades.push(CidadeEntrada {
                id: c.id.clone(),
                nome: c.nome.clone(),
                pais_tag: c.pais_tag.clone(),
                posicao: c.posicao,
                nivel: niv as f32,
                populacao: c.populacao_urbana.min(u32::MAX as u64) as u32,
                eh_capital: c.eh_capital,
                bloco: c.bloco,
                era: Some(0),
                nivel_maximo: Some(8),
                ano_fundacao: c.ano_fundacao,
                mudancas_nome: c.mudancas_nome.clone(),
                eh_porto: c.eh_porto,
            });
            entrada_eco.cidades.push(c.clone());
        }

        // 2. Extrair construções de MundoEconomico
        atualizar_predios_de_mundo(&nucleo.mundo, coords, &mut entrada_eco);

        estado.total_predios = entrada_eco.total_predios();
        estado.total_cidades = entrada_eco.total_cidades();
        estado.ultima_versao_construcao = eco.versao_construcao;
        estado.inicializado = true;

        info!(
            "visual_construcoes: mapa vivo populado com {} construções e {} cidades da economia.",
            estado.total_predios, estado.total_cidades
        );
    } else if eco.versao_construcao != estado.ultima_versao_construcao
        && tempo.elapsed_secs() - estado.ultima_extracao >= 2.0
    {
        estado.ultima_extracao = tempo.elapsed_secs();
        estado.ultima_versao_construcao = eco.versao_construcao;
        atualizar_predios_de_mundo(&nucleo.mundo, coords, &mut entrada_eco);
        estado.total_predios = entrada_eco.total_predios();
        info!(
            "visual_construcoes: construções atualizadas no mapa vivo! Total prédios: {}",
            estado.total_predios
        );
    }
}

/// Extrai e atualiza construções de `MundoEconomico` para `EntradaEconomiaVisual`.
pub fn atualizar_predios_de_mundo(
    mundo: &sim_economia::MundoEconomico,
    coords: &HashMap<String, (f32, f32)>,
    entrada: &mut EntradaEconomiaVisual,
) {
    let mut mapa_existentes: HashMap<String, usize> = entrada
        .predios
        .iter()
        .enumerate()
        .map(|(idx, p)| (p.id.clone(), idx))
        .collect();

    let tags: HashMap<&str, String> = mundo
        .paises()
        .iter()
        .map(|p| (p.id.as_str(), p.tag.clone().unwrap_or_else(|| p.id.to_uppercase())))
        .collect();

    for prov in mundo.provincias() {
        let pais_tag = tags
            .get(prov.pais_id.as_str())
            .cloned()
            .unwrap_or_else(|| prov.pais_id.to_uppercase());
        let bloco = Some(BlocoVisual::de_pais(&pais_tag));
        let pos = prov
            .codigo_externo
            .as_deref()
            .and_then(|adm1| coords.get(adm1).copied())
            .or_else(|| coords.get(&prov.id).copied());

        for (p_idx, predio) in prov.predios.iter().enumerate() {
            let p_id = format!("{}_{}_{}", prov.id, predio.tipo_predio_id, p_idx);
            let ocupacao = predio.taxa_ocupacao() as f32;
            let insumos = predio.fator_insumos() as f32;
            let producao = (ocupacao * insumos).clamp(0.0, 1.0);
            let producao = if producao > 0.0 { producao } else { 1.0 };

            if let Some(&idx) = mapa_existentes.get(&p_id) {
                let p = &mut entrada.predios[idx];
                p.nivel = predio.nivel;
                p.producao = producao;
                p.metodos_ativos = predio.metodos_ativos.clone();
            } else {
                let novo = PredioEconomiaEntrada {
                    id: p_id.clone(),
                    tipo: predio.tipo_predio_id.clone(),
                    estado_id: prov.estado_id.clone(),
                    provincia_id: Some(prov.id.clone()),
                    pais_tag: pais_tag.clone(),
                    nivel: predio.nivel,
                    metodos_ativos: predio.metodos_ativos.clone(),
                    producao,
                    posicao: pos,
                    bloco,
                    era: Some(0),
                    ativa: true,
                };
                mapa_existentes.insert(p_id, entrada.predios.len());
                entrada.predios.push(novo);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::economia_sim::{Nucleo, dir_economia};
    use std::path::Path;
    use visual_mapa::CamadaConstrucoesVisivel;

    #[test]
    fn extracao_de_predios_da_economia_1945() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let dir = dir_economia(&raiz);
        let nucleo = Nucleo::carregar(&dir, 0).expect("cenário econômico 1945");
        let mut entrada = EntradaEconomiaVisual::new();
        let coords = HashMap::new();

        atualizar_predios_de_mundo(&nucleo.mundo, &coords, &mut entrada);
        assert!(
            entrada.total_predios() >= 40_000,
            "Cenário econômico deve ter mais de 40.000 prédios, obteve: {}",
            entrada.total_predios()
        );
        let algum_predio = &entrada.predios[0];
        assert!(!algum_predio.id.is_empty());
        assert!(!algum_predio.tipo.is_empty());
        assert!(algum_predio.nivel >= 1);
    }

    #[test]
    fn alternancia_tecla_v_camada_visivel() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<ColorMaterial>()
            .init_asset::<Image>()
            .init_asset::<TextureAtlasLayout>()
            .init_resource::<ButtonInput<KeyCode>>()
            .insert_resource(visual_mapa::VisualMapaConfig {
                usar_atlas: false,
                ..default()
            })
            .add_plugins(VisualMapaPlugin);

        app.world_mut().spawn((
            Camera2d,
            Projection::Orthographic(OrthographicProjection {
                scale: 0.5,
                ..OrthographicProjection::default_2d()
            }),
        ));

        app.update();
        let vis = app.world().resource::<CamadaConstrucoesVisivel>();
        assert!(vis.0, "Camada deve iniciar ativa por padrão");

        // Simula pressão da tecla V
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyV);
        app.update();
        let vis = app.world().resource::<CamadaConstrucoesVisivel>();
        assert!(!vis.0, "Tecla V deve desativar a camada visual");

        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyV);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyV);
        app.update();
        let vis = app.world().resource::<CamadaConstrucoesVisivel>();
        assert!(vis.0, "Tecla V novamente deve reativar a camada visual");
    }
}
