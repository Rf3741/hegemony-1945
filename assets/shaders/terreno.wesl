// MaterialExtension2d do mapa. Bevy 0.20 chama esta variante modular de WESL
// (WGSL com imports e shader defs).
import package::{
    mesh2d::vertex_output::VertexOutput,
    sprite_mesh::functions as sprite_functions,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(20) var<uniform> terrain_params: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(21) var relief_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(22) var relief_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let base = sprite_functions::sample_final_color(mesh.uv, mesh.instance_index);
    let h = textureSample(relief_texture, relief_sampler, mesh.uv).r;
    let dx = textureSample(relief_texture, relief_sampler, mesh.uv + vec2<f32>(0.0012, 0.0)).r - h;
    let dy = textureSample(relief_texture, relief_sampler, mesh.uv + vec2<f32>(0.0, 0.0012)).r - h;

    // Sol cartográfico vindo do noroeste: vales ficam ligeiramente mais fechados,
    // enquanto as encostas voltadas ao sol ganham leitura sem lavar a cor política.
    let normal = normalize(vec3<f32>(-dx * 8.0, -dy * 8.0, 1.0));
    let sun = max(dot(normal, normalize(vec3<f32>(-0.55, 0.72, 0.65))), 0.0);
    let vale = clamp(1.0 - h, 0.0, 1.0) * clamp(1.0 - sun, 0.0, 1.0);
    var shade = 1.0 + terrain_params.x * (sun * 0.34 - vale * 0.16);

    // Neve muito discreta apenas no alto relevo, sem transformar cores de países em branco.
    let neve = smoothstep(0.82, 0.98, h) * terrain_params.x * 0.12;
    var rgb = base.rgb * shade;
    rgb = mix(rgb, vec3<f32>(0.92, 0.94, 0.92), neve);

    // Reflexo de água barato e temporal: o alfa da textura-base mantém a seleção e
    // todas as fronteiras intactas; a costa recebe a sombra só no preset Ultra.
    let ondas = sin((mesh.uv.x * 170.0 + mesh.uv.y * 90.0) + terrain_params.w * 0.25);
    let agua = 1.0 - smoothstep(0.015, 0.09, h);
    let brilho_agua = max(ondas, 0.0) * terrain_params.z * agua * 0.045;
    let sombra_costa = agua * smoothstep(0.10, 0.02, h) * terrain_params.y * 0.035;
    rgb += vec3<f32>(0.10, 0.17, 0.24) * brilho_agua;
    rgb *= 1.0 - sombra_costa;
    return vec4<f32>(rgb, base.a);
}
