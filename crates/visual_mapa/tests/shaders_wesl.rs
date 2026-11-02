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
