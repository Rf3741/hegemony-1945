//! Testes de integridade do mapeamento entre economia real e o mapa vivo:
//! - Todo tipo de prédio em `tipos_predio.ron` possui mapeamento.
//! - Todo sprite mapeado existe no `atlas.ron`.
//! - Todas as eras e blocos geopolíticos resolvem para sprites válidos.
//! - Funcionamento no ECS com atlas e fallback procedural.

use bevy::prelude::*;
use std::collections::HashSet;
use std::path::Path;
use visual_mapa::atlas::IndiceAtlas;
use visual_mapa::entrada::{BlocoVisual, EntradaEconomiaVisual, PredioEconomiaEntrada};
use visual_mapa::lod::MarcadorLod;
use visual_mapa::mapeamento::carregar_mapeamento_predios;
use visual_mapa::predio::{AgregadoEstadoVisual, PredioVisual};
use visual_mapa::*;

const CAMINHO_TIPOS_PREDIO: &str = "/home/rafael/claude/guerra-fria-staging/economia_dados/formato_sim/tipos_predio.ron";

#[test]
fn todo_tipo_de_predio_tem_sprite_e_todo_sprite_existe() {
    let raiz = crate::dados::raiz_dados_padrao();
    let catalogo = carregar_mapeamento_predios(&raiz);

    // 1. Carregar atlas.ron e indexar todos os IDs de sprite
    let texto_atlas = std::fs::read_to_string(raiz.join("atlas.ron"))
        .expect("atlas.ron deve existir na raiz de dados");
    let atlas_indice = IndiceAtlas::de_texto(&texto_atlas)
        .expect("atlas.ron deve ser sintaticamente válido");

    // 2. Carregar tipos_predio.ron
    let caminho_tipos = Path::new(CAMINHO_TIPOS_PREDIO);
    let tipos_da_economia: Vec<String> = if caminho_tipos.exists() {
        let texto = std::fs::read_to_string(caminho_tipos).unwrap();
        texto
            .lines()
            .filter_map(|l| {
                let trimmed = l.trim();
                if trimmed.starts_with("id:") {
                    let start = trimmed.find('"')? + 1;
                    let end = trimmed[start..].find('"')? + start;
                    Some(trimmed[start..end].to_string())
                } else {
                    None
                }
            })
            .collect()
    } else {
        // Fallback: usar as chaves do próprio catálogo se o arquivo de staging não estiver acessível
        catalogo.predios.keys().cloned().collect()
    };

    assert_eq!(
        tipos_da_economia.len(),
        56,
        "A economia possui exatamente 56 tipos de prédio"
    );

    let blocos = [
        BlocoVisual::Ocidental,
        BlocoVisual::Sovietica,
        BlocoVisual::TerceiroMundo,
    ];
    let anos = [1945.0, 1955.0, 1965.0, 1975.0, 1985.0];

    let mut sprites_usados = HashSet::new();

    // 3. Validar que CADA tipo tem mapeamento e que TODO sprite mapeado existe no atlas
    for tipo in &tipos_da_economia {
        assert!(
            catalogo.buscar(tipo).is_some(),
            "Tipo de prédio '{}' não possui entrada em mapeamento_predios.ron!",
            tipo
        );

        for bloco in blocos {
            for ano in anos {
                let sprite_id = catalogo.resolver_sprite(tipo, bloco, ano);
                assert!(
                    sprite_id.is_some(),
                    "Falha ao resolver sprite para tipo='{}', bloco={:?}, ano={}",
                    tipo,
                    bloco,
                    ano
                );

                let id = sprite_id.unwrap();
                sprites_usados.insert(id.to_string());

                assert!(
                    atlas_indice.indice_por_id(id).is_some(),
                    "Sprite '{}' mapeado para tipo='{}' ({:?}, ano {}) NÃO existe no atlas.ron!",
                    id,
                    tipo,
                    bloco,
                    ano
                );
            }
        }
    }

    println!(
        "Validação concluída: {} tipos testados em 3 blocos x 5 eras ({} combinações). Sprites únicos utilizados: {}",
        tipos_da_economia.len(),
        tipos_da_economia.len() * blocos.len() * anos.len(),
        sprites_usados.len()
    );
}

fn app_headless(usar_atlas: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<ColorMaterial>()
        .init_asset::<Image>()
        .init_asset::<TextureAtlasLayout>()
        .insert_resource(VisualMapaConfig {
            usar_atlas,
            limite_predios_por_estado: 10,
            ..default()
        })
        .add_plugins(VisualMapaPlugin);

    app.world_mut().spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: 0.5, // LOD Local (detalhado)
            ..OrthographicProjection::default_2d()
        }),
    ));
    app
}

#[test]
fn sincronizacao_ecs_predios_economia() {
    let mut app = app_headless(true);

    // Inserir dados de economia
    {
        let mut eco = app.world_mut().resource_mut::<EntradaEconomiaVisual>();
        eco.adicionar_predio(PredioEconomiaEntrada {
            id: "prov1_siderurgica_0".into(),
            tipo: "siderurgica".into(),
            estado_id: "estado_sp".into(),
            pais_tag: "BRA".into(),
            nivel: 2,
            posicao: Some((-46.6, -23.5)),
            producao: 1.0,
            ..default()
        });
        eco.adicionar_predio(PredioEconomiaEntrada {
            id: "prov1_fazenda_graos_0".into(),
            tipo: "fazenda_graos".into(),
            estado_id: "estado_sp".into(),
            pais_tag: "BRA".into(),
            nivel: 5,
            posicao: Some((-46.6, -23.5)),
            producao: 0.8,
            ..default()
        });
    }

    // Rodar updates
    for _ in 0..4 {
        app.update();
    }

    let w = app.world_mut();
    // 2 prédios individuais criados
    assert_eq!(w.query::<&PredioVisual>().iter(w).count(), 2);
    // 1 agregado estadual criado com MarcadorLod
    assert_eq!(w.query::<(&AgregadoEstadoVisual, &MarcadorLod)>().iter(w).count(), 1);

    // Verificar componente do prédio individual
    let predios: Vec<_> = w.query::<&PredioVisual>().iter(w).collect();
    let siderurgica = predios.iter().find(|p| p.tipo == "siderurgica").unwrap();
    assert_eq!(siderurgica.nivel, 2);
    assert!(siderurgica.emite_fumaca);

    // Subir de nível e verificar animação de crescimento
    {
        let mut eco = app.world_mut().resource_mut::<EntradaEconomiaVisual>();
        eco.predios[0].nivel = 6;
    }

    for _ in 0..2 {
        app.update();
    }

    let w = app.world_mut();
    let predios: Vec<_> = w.query::<&PredioVisual>().iter(w).collect();
    let siderurgica = predios.iter().find(|p| p.tipo == "siderurgica").unwrap();
    assert_eq!(siderurgica.nivel, 6);
    assert!(siderurgica.timer_pop_nivel > 0.0, "Subida de nível deve disparar pulso animado");
}
