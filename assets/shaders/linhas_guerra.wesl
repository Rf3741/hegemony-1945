// Shader para linhas de frente de guerra (pulsante/serrilhada).
// Recebe cor base, intensidade de pulso e flag de proxy (laranja tracejado).

import bevy_sprite_render::{
    mesh2d::vertex_output::VertexOutput,
    sprite_mesh::functions as sprite_functions,
};

struct LinhasGuerraParams {
    tempo: f32,
    cor_base: vec4<f32>,
    intensidade_pulso: f32,
    eh_proxy: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> params: LinhasGuerraParams;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let tint = sprite_functions::sample_final_color(mesh.uv, mesh.instance_index);
    
    var cor = params.cor_base.rgb;
    var alfa = params.cor_base.a;
    
    if params.eh_proxy == 1u {
        // Guerra por procuração: laranja tracejado animado
        let dash = sin(mesh.uv.x * 50.0 + params.tempo * 3.0);
        let mask = step(0.0, dash);
        alfa *= mask * (0.7 + 0.3 * params.intensidade_pulso);
        cor = vec3<f32>(1.0, 0.6, 0.0);
    } else {
        // Guerra direta: vermelho pulsante
        let pulso = 0.7 + 0.3 * params.intensidade_pulso;
        alfa *= pulso;
        // Adiciona brilho no centro da linha
        let centro = 1.0 - abs(mesh.uv.y - 0.5) * 2.0;
        cor = mix(cor, vec3<f32>(1.0, 0.4, 0.4), centro * 0.3 * params.intensidade_pulso);
    }
    
    // Borda sutil
    let borda = 1.0 - abs(mesh.uv.y - 0.5) * 2.0;
    alfa *= smoothstep(0.05, 0.15, borda);
    
    return vec4<f32>(cor * tint.rgb, alfa * tint.a);
}