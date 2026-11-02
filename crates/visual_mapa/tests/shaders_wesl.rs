//! Valida os shaders WESL do mapa sem criar janela nem inicializar GPU.

use naga::{front::wgsl, valid::{Capabilities, ValidationFlags, Validator}};
use std::{borrow::Cow, fs, path::Path};
use wesl::{resolver::VirtualResolver, CompileOptions};

const VERTEX_OUTPUT: &str = r#"
public struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(5) @interpolate(flat) instance_index: u32,
}
"#;

const SPRITE_FUNCTIONS: &str = r#"
public fn sample_final_color(uv: vec2<f32>, instance_index: u32) -> vec4<f32> {
    return vec4<f32>(uv, f32(instance_index), 1.0);
}
"#;

const CONSTANTS: &str = "public const MATERIAL_BIND_GROUP: u32 = 2u;";

fn resolver(source_path: &Path) -> (VirtualResolver<'static>, wesl::syntax::ModulePath) {
    let mut resolver = VirtualResolver::new();
    resolver.add_module(
        "bevy_sprite_render::mesh2d::vertex_output".parse().unwrap(),
        Cow::Borrowed(VERTEX_OUTPUT),
    );
    resolver.add_module(
        "bevy_sprite_render::sprite_mesh::functions".parse().unwrap(),
        Cow::Borrowed(SPRITE_FUNCTIONS),
    );
    resolver.add_module("constants".parse().unwrap(), Cow::Borrowed(CONSTANTS));
    let prefix = format!(
        "/assets/shaders/{}/bevy_sprite_render",
        source_path.file_stem().unwrap().to_string_lossy()
    );
    resolver.add_module(
        format!("{prefix}/mesh2d/vertex_output").parse().unwrap(),
        Cow::Borrowed(VERTEX_OUTPUT),
    );
    resolver.add_module(
        format!("{prefix}/sprite_mesh/functions").parse().unwrap(),
        Cow::Borrowed(SPRITE_FUNCTIONS),
    );
    resolver.add_module(
        format!(
            "/assets/shaders/{}/constants",
            source_path.file_stem().unwrap().to_string_lossy()
        )
        .parse()
        .unwrap(),
        Cow::Borrowed(CONSTANTS),
    );
    resolver.add_module(
        format!(
            "/assets/shaders/{}/bevy_sprite_render::mesh2d::vertex_output",
            source_path.file_stem().unwrap().to_string_lossy()
        )
        .parse()
        .unwrap(),
        Cow::Borrowed(VERTEX_OUTPUT),
    );
    resolver.add_module(
        format!(
            "/assets/shaders/{}/bevy_sprite_render::sprite_mesh::functions",
            source_path.file_stem().unwrap().to_string_lossy()
        )
        .parse()
        .unwrap(),
        Cow::Borrowed(SPRITE_FUNCTIONS),
    );
    let module: wesl::syntax::ModulePath = format!(
        "/assets/shaders/{}",
        source_path.file_stem().unwrap().to_string_lossy()
    )
    .parse()
    .unwrap();
    (resolver, module)
}

#[test]
fn shaders_do_bevy_wesl_ate_naga() {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for nome in ["terreno", "atmosfera", "objetos_2d"] {
        let caminho = raiz.join("assets/shaders").join(format!("{nome}.wesl"));
        let fonte = fs::read_to_string(&caminho).unwrap();
        let (mut resolver, modulo) = resolver(&caminho);
        resolver.add_module(modulo.clone(), Cow::Owned(fonte));
        let resultado = wesl::compile(&modulo, &CompileOptions::default(), &resolver)
            .unwrap_or_else(|erro| panic!("{nome}.wesl falhou no WESL: {erro}"));
        let wgsl = resultado.to_string();
        let modulo_naga = wgsl::parse_str(&wgsl)
            .unwrap_or_else(|erro| panic!("{nome}.wesl gerou WGSL inválido: {erro}"));
        Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&modulo_naga)
            .unwrap_or_else(|erro| panic!("{nome}.wesl falhou na validação naga: {erro}"));
    }
}

/// O shader do terreno lê a textura de IDs e a paleta: os bindings têm de bater com o
/// `AsBindGroup` de `TerrainMaterial` (crates/guerra_fria/src/plugins/map.rs).
#[test]
fn terreno_declara_ids_paleta_e_parametros_batendo_com_o_material_rust() {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wesl = fs::read_to_string(raiz.join("assets/shaders/terreno.wesl")).unwrap();
    let rust = fs::read_to_string(raiz.join("crates/guerra_fria/src/plugins/map.rs")).unwrap();
    for (binding, nome, atributo) in [
        (20, "terrain_params", "#[uniform(20)]"),
        (21, "relief_texture", "#[texture(21)]"),
        (22, "relief_sampler", "#[sampler(22)]"),
        (23, "ids_texture", "#[texture(23)]"),
        (24, "palette_texture", "#[texture(24)]"),
        (25, "paleta_params", "#[uniform(25)]"),
    ] {
        let decl = format!("@binding({binding}) var");
        let linha = wesl
            .lines()
            .find(|l| l.contains(&decl))
            .unwrap_or_else(|| panic!("terreno.wesl sem @binding({binding})"));
        assert!(linha.contains(nome), "binding {binding} deveria ser `{nome}`: {linha}");
        assert!(rust.contains(atributo), "TerrainMaterial sem {atributo}");
    }
    // A cor política não pode voltar a depender da textura do sprite (repintura de 11,5 M pixels).
    assert!(wesl.contains("textureLoad(palette_texture"));
    assert!(wesl.contains("textureLoad(ids_texture"));
}
