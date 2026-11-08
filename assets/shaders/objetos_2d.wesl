// Paleta comum para faces e sombras dos objetos 2.5D.
// Mantido como shader de referência para materiais 2D customizados do mapa.
import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

struct VolumeMaterial {
    color: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> material: VolumeMaterial;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Iluminação oblíqua vinda de noroeste no espaço UV local do quad (0..1),
    // independente da posição mundial para nunca saturar para branco no hemisfério norte.
    let topo = clamp(1.0 - in.uv.y, 0.0, 1.0);
    let oeste = clamp(1.0 - in.uv.x, 0.0, 1.0);
    let luz = 0.84 + 0.12 * topo + 0.04 * oeste;
    let rgb = clamp(material.color.rgb * luz, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(rgb, material.color.a);
}
