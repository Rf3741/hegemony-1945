//! Smoke headless do plugin: sem janela/GPU, só ECS + assets.
use bevy::prelude::*;
use visual_mapa::cidade::CidadeVisual;
use visual_mapa::industria::InstalacaoIndustrial;
use visual_mapa::lod::{DetalheLod, MarcadorLod};
use visual_mapa::transporte::LinhaTransporte;
use visual_mapa::*;

fn app(usar_atlas: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Mesh>()
        .init_asset::<ColorMaterial>()
        .init_asset::<Image>()
        .init_asset::<TextureAtlasLayout>()
        .insert_resource(VisualMapaConfig { usar_atlas, ..default() })
        .add_plugins(VisualMapaPlugin);
    app.world_mut().spawn((Camera2d, Projection::Orthographic(OrthographicProjection { scale: 0.5, ..OrthographicProjection::default_2d() })));
    app
}

fn dados() -> EntradaVisual {
    EntradaVisual {
        cidades: vec![
            CidadeEntrada { id: "a".into(), nome: "A".into(), pais_tag: "USA".into(), posicao: (-74.0, 40.7), nivel: 5.0, ..default() },
            CidadeEntrada { id: "b".into(), nome: "B".into(), pais_tag: "SOV".into(), posicao: (37.6, 55.7), nivel: 3.0, ..default() },
        ],
        instalacoes: vec![InstalacaoEntrada { id: "i".into(), tipo: "siderurgica".into(), posicao: (-80.0, 40.0), ..default() }],
        conexoes: vec![ConexaoEntrada { id: "c".into(), origem_id: "a".into(), destino_id: "b".into(), tipo: "ferrovia".into(), nivel: 2, ativa: true }],
    }
}

fn rodar(usar_atlas: bool) -> App {
    let mut a = app(usar_atlas);
    *a.world_mut().resource_mut::<EntradaVisual>() = dados();
    a.world_mut().resource_mut::<AnoVisual>().0 = 1970.0;
    for _ in 0..4 {
        a.update();
    }
    a
}

#[test]
fn cria_entidades_com_atlas() {
    let mut a = rodar(true);
    assert!(a.world().resource::<AtlasVisual>().disponivel());
    let w = a.world_mut();
    assert_eq!(w.query::<&CidadeVisual>().iter(w).count(), 2);
    assert_eq!(w.query::<&InstalacaoIndustrial>().iter(w).count(), 1);
    assert_eq!(w.query::<&LinhaTransporte>().iter(w).count(), 1);
    // um sprite de atlas por cidade
    assert_eq!(w.query::<(&DetalheLod, &Sprite)>().iter(w).count(), 2);
    let era = w.query::<&CidadeVisual>().iter(w).next().unwrap().era_idx;
    assert_eq!(era, 2, "1970 = era 2");
}

#[test]
fn fallback_procedural_sem_atlas() {
    let mut a = rodar(false);
    let w = a.world_mut();
    assert!(!w.resource::<AtlasVisual>().disponivel());
    assert_eq!(w.query::<&CidadeVisual>().iter(w).count(), 2);
    assert!(w.query::<&DetalheLod>().iter(w).count() > 10, "predios procedurais");
    assert_eq!(w.query::<&MarcadorLod>().iter(w).count(), 2);
}

#[test]
fn nivel_sobe_e_era_local_vence_a_global() {
    let mut a = rodar(true);
    {
        let mut e = a.world_mut().resource_mut::<EntradaVisual>();
        e.cidades[0].nivel = 7.5;
        e.cidades[1].era = Some(4);
    }
    for _ in 0..3 {
        a.update();
    }
    let w = a.world_mut();
    let mut q = w.query::<&CidadeVisual>();
    let cs: Vec<_> = q.iter(w).collect();
    let ca = cs.iter().find(|c| c.id == "a").unwrap();
    let cb = cs.iter().find(|c| c.id == "b").unwrap();
    assert_eq!(ca.nivel_atual_inteiro, 7);
    assert!(ca.timer_animacao_nivel > 0.0, "pop ao subir de nivel");
    assert_eq!(cb.era_idx, 4);
}

#[test]
fn posicao_usa_projecao_do_gf_mapa() {
    let mut a = rodar(true);
    let w = a.world_mut();
    let c = w.query::<&CidadeVisual>().iter(w).find(|c| c.id == "a").unwrap();
    assert!((c.posicao_mundo - geo_para_mundo(-74.0, 40.7)).length() < 0.01);
}
