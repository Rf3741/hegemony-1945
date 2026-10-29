//! Desempenho: 50 mil prédios viram poucas entidades e a atualização incremental é rápida.
use bevy::prelude::*;
use std::time::Instant;
use visual_mapa::predio::PredioVisual;
use visual_mapa::*;

#[test]
fn cinquenta_mil_predios_poucas_entidades() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<ColorMaterial>()
        .init_asset::<Image>()
        .init_asset::<TextureAtlasLayout>()
        .insert_resource(VisualMapaConfig { usar_atlas: false, ..default() })
        .add_plugins(VisualMapaPlugin);
    app.world_mut().spawn((Camera2d, Projection::Orthographic(OrthographicProjection { scale: 1.5, ..OrthographicProjection::default_2d() })));
    let tipos = ["fazenda", "mina", "siderurgica", "fabrica", "porto", "usina", "refinaria"];
    let mut e = EntradaEconomiaVisual::new();
    for i in 0..50_000usize {
        e.predios.push(PredioPlaceholder::novo(i, tipos[i % tipos.len()]));
    }
    *app.world_mut().resource_mut::<EntradaEconomiaVisual>() = e;
    let t = Instant::now();
    for _ in 0..3 {
        app.update();
    }
    let primeira = t.elapsed();
    let n = app.world().iter_entities().count();
    let w = app.world_mut();
    let predios = w.query::<&PredioVisual>().iter(w).count();
    println!("50k predios -> {n} entidades ({predios} PredioVisual); carga inicial {primeira:?}");
    assert!(predios <= 500 * 3, "{predios}");
    assert!(n < 3000, "{n}");
    // Frames sem mudança são baratos.
    let t = Instant::now();
    for _ in 0..60 {
        app.update();
    }
    println!("60 frames parados: {:?}", t.elapsed());
    // Muda 1 prédio: atualização incremental.
    app.world_mut().resource_mut::<EntradaEconomiaVisual>().predios[0].nivel += 1;
    let t = Instant::now();
    app.update();
    println!("atualizacao incremental: {:?}", t.elapsed());
    assert_eq!(app.world().iter_entities().count(), n, "sem recriar entidades");
}

struct PredioPlaceholder;
impl PredioPlaceholder {
    fn novo(i: usize, tipo: &str) -> PredioEconomiaEntrada {
        PredioEconomiaEntrada {
            id: format!("p{i}"),
            tipo: tipo.into(),
            estado_id: format!("estado{}", i % 500),
            pais_tag: "USA".into(),
            nivel: 1 + (i % 5) as u32,
            ativa: true,
            posicao: Some(((i % 360) as f32 - 180.0, ((i / 360) % 120) as f32 - 60.0)),
            ..default()
        }
    }
}
