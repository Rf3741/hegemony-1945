import bevy_sprite_render::mesh2d::vertex_output::VertexOutput;

struct AtmosferaMaterial {
    dados_a: vec4<f32>,
    dados_b: vec4<f32>,
    dados_c: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: AtmosferaMaterial;

fn hash21(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453);
}

fn ruido(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = smoothstep(vec2<f32>(0.0), vec2<f32>(1.0), fract(p));
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var valor = 0.0;
    var amplitude = 0.5;
    for (var i = 0; i < 4; i++) {
        valor += amplitude * ruido(p);
        p = p * 2.03 + vec2<f32>(17.0, 9.0);
        amplitude *= 0.5;
    }
    return valor;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let centro = uv - vec2<f32>(0.5);
    let borda = smoothstep(0.78, 0.35, length(centro * vec2<f32>(1.0, 1.18)));
    let vignette = (1.0 - borda) * material.dados_b.w;
    let papel = (1.0 - borda) * material.dados_b.y * 0.055;

    let nuvem_ruido = fbm(uv * vec2<f32>(5.0, 3.2) + vec2<f32>(material.dados_a.x * 0.012, material.dados_a.x * 0.006));
    let zoom_fade = smoothstep(0.45, 1.55, material.dados_a.y);
    let nuvens = smoothstep(0.58, 0.78, nuvem_ruido) * material.dados_a.z * zoom_fade;
    let sombra = nuvens * 0.075 * material.dados_c.z;

    let longitude = uv.x * 6.2831853;
    let distancia_solar = sin(longitude - material.dados_b.x);
    let noite_local = smoothstep(0.12, -0.12, distancia_solar) * material.dados_a.w * material.dados_c.x;

    let neblina = (1.0 - uv.y) * material.dados_b.z * 0.16;
    let rgb = vec3<f32>(
        0.055 * noite_local + sombra + papel,
        0.085 * noite_local + sombra + papel,
        0.14 * noite_local + neblina + papel
    );
    let alpha = max(max(vignette, nuvens * 0.10), max(noite_local * 0.28, neblina));
    return vec4<f32>(rgb, alpha);
}
