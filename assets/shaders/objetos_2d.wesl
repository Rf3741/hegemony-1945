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
    let luz = 0.86 + 0.14 * max(in.world_position.y, 0.0);
    return vec4<f32>(material.color.rgb * luz, material.color.a);
}
