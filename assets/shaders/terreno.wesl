// MaterialExtension2d do mapa. Bevy 0.20 chama esta variante modular de WESL
// (WGSL com imports e shader defs).
//
// A cor política NÃO vem mais de uma textura RGBA gigante repintada na CPU: o mapa guarda uma
// textura de IDs de província (gerada uma vez) e uma PALETA minúscula (uma cor/dono/estado por
// província). Repintar o mapa = atualizar só a paleta (~130 KB). Fronteiras, hachura de ocupação,
// relevo e oceano são calculados aqui, pixel a pixel, com as mesmas regras do pintor de referência
// em CPU (`colorir_com_hachura` em plugins/map.rs).
import bevy_sprite_render::{
    mesh2d::vertex_output::VertexOutput,
    sprite_mesh::functions as sprite_functions,
};

struct ParamsPaleta {
    // x = id de Berlim (setores de ocupação), y/z = centro em pixels, w = 1 se ativo.
    berlim: vec4<f32>,
    // Cores dos setores: URSS, França, Reino Unido, EUA.
    setores: array<vec4<f32>, 4>,
    // x = 1 se o relevo modula a cor política (mesmo tamanho da textura de IDs).
    flags: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(20) var<uniform> terrain_params: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(21) var relief_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(22) var relief_sampler: sampler;
// Rg8: r = id & 255; g = (id >> 8) & 31 | perto_de_fronteira << 5 | distancia_costa << 6.
@group(constants::MATERIAL_BIND_GROUP) @binding(23) var ids_texture: texture_2d<f32>;
// Rgba8, 4096 colunas; por id: linha 0 = cor, 1 = ocupante (a>0), 2 = dono (rg) e estado (ba).
@group(constants::MATERIAL_BIND_GROUP) @binding(24) var palette_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(25) var<uniform> paleta_params: ParamsPaleta;

const PAL_W: i32 = 4096;
const OCEANO_FUNDO: vec3<f32> = vec3<f32>(16.0, 38.0, 70.0);
const COR_COSTA: vec3<f32> = vec3<f32>(70.0, 130.0, 175.0);
const COR_FRONTEIRA: vec3<f32> = vec3<f32>(15.0, 15.0, 15.0);

fn b8(v: f32) -> u32 {
    return u32(v * 255.0 + 0.5);
}

fn pal(id: u32, linha: i32) -> vec4<f32> {
    let i = i32(id);
    return textureLoad(palette_texture, vec2<i32>(i % PAL_W, (i / PAL_W) * 4 + linha), 0);
}

// (dono, estado) de uma província.
fn dono_estado(id: u32) -> vec2<u32> {
    let p = pal(id, 2);
    return vec2<u32>(b8(p.r) | (b8(p.g) << 8u), b8(p.b) | (b8(p.a) << 8u));
}

fn raw_ids(x: i32, y: i32, d: vec2<i32>) -> vec2<u32> {
    let xx = ((x % d.x) + d.x) % d.x;
    let t = textureLoad(ids_texture, vec2<i32>(xx, y), 0);
    return vec2<u32>(b8(t.r), b8(t.g));
}

// Id da província no pixel (x com wrap horizontal); -1 fora do mapa na vertical.
fn pid(x: i32, y: i32, d: vec2<i32>) -> i32 {
    if (y < 0 || y >= d.y) {
        return -1;
    }
    let r = raw_ids(x, y, d);
    return i32(r.x | ((r.y & 31u) << 8u));
}

// Tipo de fronteira entre `a` (pixel corrente) e `b` (vizinho): 3 = país, 2 = estado, 1 = província.
fn tipo(a: u32, b: u32) -> u32 {
    let da = dono_estado(a);
    let db = dono_estado(b);
    if (b == 0u || db.x != da.x) {
        return 3u;
    }
    if (db.y != da.y) {
        return 2u;
    }
    return 1u;
}

// Espessura/tipo da borda do pixel: marca os DOIS lados (como o pintor em CPU, que percorre
// vizinhos à direita e abaixo), logo olhamos os quatro vizinhos com a direção certa.
fn borda(x: i32, y: i32, d: vec2<i32>) -> u32 {
    let id = u32(pid(x, y, d));
    var m = 0u;
    let dir = u32(pid(x + 1, y, d));
    if (dir != id) {
        m = max(m, tipo(id, dir));
    }
    let esq = u32(pid(x - 1, y, d));
    if (esq != id) {
        m = max(m, tipo(esq, id));
    }
    let bai = pid(x, y + 1, d);
    if (bai >= 0 && u32(bai) != id) {
        m = max(m, tipo(id, u32(bai)));
    }
    let cim = pid(x, y - 1, d);
    if (cim >= 0 && u32(cim) != id) {
        m = max(m, tipo(u32(cim), id));
    }
    return m;
}

fn borda_pais(x: i32, y: i32, d: vec2<i32>) -> bool {
    if (y < 0 || y >= d.y) {
        return false;
    }
    return borda(x, y, d) == 3u;
}

// Anel de brilho 1: terra encostada numa fronteira de país.
fn anel1(x: i32, y: i32, d: vec2<i32>) -> bool {
    if (y < 0 || y >= d.y) {
        return false;
    }
    if (pid(x, y, d) == 0 || borda(x, y, d) == 3u) {
        return false;
    }
    return borda_pais(x + 1, y, d) || borda_pais(x - 1, y, d) || borda_pais(x, y + 1, d) || borda_pais(x, y - 1, d);
}

fn anel_a_duas_casas(x: i32, y: i32, d: vec2<i32>) -> bool {
    if (pid(x, y, d) == 0 || borda(x, y, d) == 3u || anel1(x, y, d)) {
        return false;
    }
    return anel1(x + 1, y, d) || anel1(x - 1, y, d) || anel1(x, y + 1, d) || anel1(x, y - 1, d);
}

// Cor (0..255, espaço sRGB) e alfa do pixel (x, y), igual ao pintor de referência em CPU.
fn cor_do_pixel(x: i32, y: i32, d: vec2<i32>) -> vec4<f32> {
    let raw = raw_ids(x, y, d);
    let id = raw.x | ((raw.y & 31u) << 8u);
    let perto = ((raw.y >> 5u) & 1u) == 1u;
    let costa = (raw.y >> 6u) & 3u;
    var c = vec3<f32>(0.0);
    var alfa = 255.0;
    if (id == 0u) {
        let fx = f32(x);
        let fy = f32(y);
        let lat = abs(fy / f32(d.y) - 0.5) * 2.0;
        let ruido = sin(fx * 0.021 + fy * 0.013) * 0.5 + sin(fx * 0.047 - fy * 0.031) * 0.3 + sin(fx * 0.09 + fy * 0.07) * 0.2;
        let f = 1.0 + (0.22 - lat * 0.40) + ruido * 0.045;
        c = clamp(OCEANO_FUNDO * f, vec3<f32>(0.0), vec3<f32>(255.0));
        if (costa > 0u) {
            var t = 0.14;
            if (costa == 1u) {
                t = 0.55;
            } else if (costa == 2u) {
                t = 0.30;
            }
            c = c + (COR_COSTA - c) * t;
        }
    } else {
        let pc = pal(id, 0);
        c = floor(pc.rgb * 255.0 + 0.5);
        alfa = floor(pc.a * 255.0 + 0.5);
        let oc = pal(id, 1);
        if (oc.a > 0.5 && ((x + y) % 8) < 4) {
            c = floor(oc.rgb * 255.0 + 0.5);
            if (paleta_params.berlim.w > 0.5 && id == u32(paleta_params.berlim.x + 0.5)) {
                let cx = i32(paleta_params.berlim.y + 0.5);
                let cy = i32(paleta_params.berlim.z + 0.5);
                var k = 2;
                if (x >= cx) {
                    k = 0;
                } else if (y + 1 < cy) {
                    k = 1;
                } else if (y > cy + 1) {
                    k = 3;
                }
                c = floor(paleta_params.setores[k].rgb * 255.0 + 0.5);
            }
        }
        if (paleta_params.flags.x > 0.5) {
            // 60% cor do país + 40% cor do país modulada pelo relevo sombreado.
            let r = textureLoad(relief_texture, vec2<i32>(x, y), 0).r * 255.0;
            let k = clamp(((r - 185.0) / 45.0) + 1.0, 0.3, 1.6);
            c = floor(clamp(c * (0.6 + 0.4 * k), vec3<f32>(0.0), vec3<f32>(255.0)));
        }
    }
    if (perto) {
        let b = borda(x, y, d);
        if (b == 3u) {
            c = COR_FRONTEIRA;
            alfa = 255.0;
        } else if (b > 0u) {
            var f = 0.92;
            if (b == 2u) {
                f = 0.72;
            }
            c = floor(c * f);
        } else if (anel1(x, y, d)) {
            c = c + (vec3<f32>(255.0) - c) * 0.10;
        } else if (anel_a_duas_casas(x, y, d)) {
            c = c + (vec3<f32>(255.0) - c) * 0.04;
        }
    }
    return vec4<f32>(c, alfa);
}

fn srgb_para_linear(c: vec3<f32>) -> vec3<f32> {
    let baixo = c / 12.92;
    let alto = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(alto, baixo, c <= vec3<f32>(0.04045));
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // A textura do sprite é só um pixel branco: dela vem apenas o tingimento/alfa do sprite.
    let tint = sprite_functions::sample_final_color(mesh.uv, mesh.instance_index);
    let h = textureSample(relief_texture, relief_sampler, mesh.uv).r;
    let dx = textureSample(relief_texture, relief_sampler, mesh.uv + vec2<f32>(0.0012, 0.0)).r - h;
    let dy = textureSample(relief_texture, relief_sampler, mesh.uv + vec2<f32>(0.0, 0.0012)).r - h;

    // Pixel da textura de IDs sob este fragmento (o sprite cobre o mapa inteiro, sem wrap próprio).
    let d = vec2<i32>(textureDimensions(ids_texture));
    let q = clamp(vec2<i32>(floor(mesh.uv * vec2<f32>(d))), vec2<i32>(0), d - vec2<i32>(1));
    let pintura = cor_do_pixel(q.x, q.y, d);
    let base = vec4<f32>(srgb_para_linear(pintura.rgb / 255.0) * tint.rgb, pintura.a / 255.0 * tint.a);

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
