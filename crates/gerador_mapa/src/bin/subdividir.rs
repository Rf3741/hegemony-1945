//! Subdivide cada região admin-1 (ESTADO) em PROVÍNCIAS menores (~40.000 km²).
//!
//! Entrada : `data/estados_mapa/provinces.{png,ron}` (saída do `gerador_mapa`, 1 cor por estado)
//!           + `ne_10m_populated_places.shp` (sementes; env `GF_CIDADES_SHP`).
//! Saída   : `assets/map/provinces.{png,ron}` (1 cor por província) e
//!           `data/scenario_1945/province_owners_1945.ron` (dono por província, herdado do estado).
//!
//! Método: por componente conexo de cada estado, sementes = cidades (por população, com espaçamento mínimo)
//! completadas por amostragem do ponto mais distante; atribuição pixel a pixel à semente mais próxima
//! (distância esférica) com distorção suave do domínio (fronteiras orgânicas); fragmentos soltos são
//! incorporados ao vizinho dominante.

use gf_mapa::mapa::{
    CatalogoProvincias, DonosProvincias1945, ProvinciaMapa, cor_unica_provincia, desprojetar_miller,
    id_de_cor_provincia, projetar_miller,
};
use image::{ImageBuffer, Rgb};
use shapefile::dbase::FieldValue;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

const W: usize = 5632;
const H: usize = 2048;
const RAIO_KM: f64 = 6371.0;

fn alvo_km2() -> f64 {
    std::env::var("GF_ALVO_KM2").ok().and_then(|s| s.parse().ok()).unwrap_or(36_000.0)
}

fn hash2(x: i64, y: i64, s: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ (y as u64).wrapping_mul(0xC2B2AE3D27D4EB4F) ^ s.wrapping_mul(0x165667B19E3779F9);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58476D1CE4E5B9);
    h ^= h >> 32;
    (h & 0xFFFFFF) as f32 / 16777216.0
}

/// Ruído de valor suave em [0,1), célula de `cel` pixels (x com período W).
fn ruido(x: f32, y: f32, cel: f32, s: u64) -> f32 {
    let (gx, gy) = (x / cel, y / cel);
    let (ix, iy) = (gx.floor(), gy.floor());
    let (fx, fy) = (gx - ix, gy - iy);
    let (fx, fy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let n = (W as f32 / cel).round() as i64;
    let (ix, iy) = (ix as i64, iy as i64);
    let v = |dx: i64, dy: i64| hash2((ix + dx).rem_euclid(n), iy + dy, s);
    let a = v(0, 0) + (v(1, 0) - v(0, 0)) * fx;
    let b = v(0, 1) + (v(1, 1) - v(0, 1)) * fx;
    a + (b - a) * fy
}

type V3 = [f64; 3];
fn vec3(lon_deg: f64, lat_deg: f64) -> V3 {
    let (lo, la) = (lon_deg.to_radians(), lat_deg.to_radians());
    [la.cos() * lo.cos(), la.cos() * lo.sin(), la.sin()]
}
fn dot(a: &V3, b: &V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn dist_km(a: &V3, b: &V3) -> f64 {
    dot(a, b).clamp(-1.0, 1.0).acos() * RAIO_KM
}

struct Cidade {
    nome: String,
    lon: f64,
    lat: f64,
    pop: f64,
}

fn texto(v: Option<&FieldValue>) -> String {
    match v {
        Some(FieldValue::Character(Some(s))) => s.trim().to_string(),
        _ => String::new(),
    }
}
fn numero(v: Option<&FieldValue>) -> f64 {
    match v {
        Some(FieldValue::Numeric(Some(n))) => *n,
        Some(FieldValue::Double(d)) => *d,
        Some(FieldValue::Float(Some(f))) => *f as f64,
        Some(FieldValue::Integer(i)) => *i as f64,
        _ => 0.0,
    }
}

fn carregar_cidades(caminho: &str) -> Result<Vec<Cidade>, Box<dyn std::error::Error>> {
    let mut leitor = shapefile::Reader::from_path(caminho)?;
    let mut v = Vec::new();
    for r in leitor.iter_shapes_and_records() {
        let (forma, reg) = r?;
        let shapefile::Shape::Point(p) = forma else { continue };
        let pt = texto(reg.get("NAME_PT"));
        let nome = if pt.is_empty() { texto(reg.get("NAME")) } else { pt };
        if nome.is_empty() {
            continue;
        }
        let pop = numero(reg.get("POP_MAX")).max(numero(reg.get("POP_MIN")));
        v.push(Cidade { nome, lon: p.x, lat: p.y, pop });
    }
    v.sort_by(|a, b| b.pop.partial_cmp(&a.pop).unwrap());
    Ok(v)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let raiz = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into())).join("../..").canonicalize()?;
    let alvo = alvo_km2();
    let shp = std::env::var("GF_CIDADES_SHP").unwrap_or_else(|_| {
        "/home/rafael/claude/guerra-fria-staging/territorio_dados/ne_cities/ne_10m_populated_places.shp".into()
    });

    // --- entrada: estados ---
    let cat_txt = fs::read_to_string(raiz.join("data/estados_mapa/provinces.ron"))?;
    let estados: CatalogoProvincias = ron::from_str(&cat_txt)?;
    let img = image::open(raiz.join("data/estados_mapa/provinces.png"))?.to_rgb8();
    assert_eq!(img.dimensions(), (W as u32, H as u32));
    let est: Vec<u32> = img.pixels().map(|p| id_de_cor_provincia((p[0], p[1], p[2])).unwrap_or(0)).collect();
    let cidades = carregar_cidades(&shp)?;
    println!("estados: {} | cidades: {} | leitura {:.1}s", estados.provincias.len(), cidades.len(), t0.elapsed().as_secs_f32());

    // --- tabelas por linha/coluna ---
    let lat_c: Vec<f64> = (0..H).map(|y| desprojetar_miller(0.0, y as f64 + 0.5, W as u32, H as u32).1).collect();
    let area_linha: Vec<f64> = (0..H)
        .map(|y| {
            let a = desprojetar_miller(0.0, y as f64, W as u32, H as u32).1.to_radians();
            let b = desprojetar_miller(0.0, (y + 1) as f64, W as u32, H as u32).1.to_radians();
            RAIO_KM * RAIO_KM * (std::f64::consts::TAU / W as f64) * (a.sin() - b.sin()).abs()
        })
        .collect();
    let lon_c: Vec<f64> = (0..W).map(|x| (x as f64 + 0.5) / W as f64 * 360.0 - 180.0).collect();

    // --- pixels por estado ---
    let nmax = estados.provincias.iter().map(|p| p.id).max().unwrap_or(0) as usize + 1;
    let mut pixels_est: Vec<Vec<u32>> = vec![Vec::new(); nmax];
    for (i, &e) in est.iter().enumerate() {
        if e != 0 {
            pixels_est[e as usize].push(i as u32);
        }
    }
    // cidades por pixel de estado (primeira = mais populosa por pixel)
    let mut cid_est: Vec<Vec<usize>> = vec![Vec::new(); nmax];
    for (ci, c) in cidades.iter().enumerate() {
        let (px, py) = projetar_miller(c.lon, c.lat, W as u32, H as u32);
        let (x, y) = ((px as usize).min(W - 1), (py as usize).min(H - 1));
        let e = est[y * W + x] as usize;
        if e != 0 {
            cid_est[e].push(ci);
        }
    }

    let mut prov: Vec<u32> = vec![0; W * H]; // rótulo provisório global (id final depois)
    let mut sementes_info: Vec<(u32, String, bool)> = Vec::new(); // (estado, nome, eh_cidade), índice = rótulo-1
    let mut comp: Vec<u32> = vec![0; W * H];
    let mut fila: Vec<u32> = Vec::new();
    let mut total_provs_estado: Vec<u32> = vec![0; nmax];

    for sid in 1..nmax {
        let pix = &pixels_est[sid];
        if pix.is_empty() {
            continue;
        }
        let area_total: f64 = pix.iter().map(|&i| area_linha[i as usize / W]).sum();

        // componentes conexos (4-vizinhança, wrap em x)
        let mut comps: Vec<Vec<u32>> = Vec::new();
        for &p0 in pix {
            if comp[p0 as usize] != 0 {
                continue;
            }
            let cid = comps.len() as u32 + 1;
            comp[p0 as usize] = cid;
            fila.clear();
            fila.push(p0);
            let mut k = 0;
            while k < fila.len() {
                let p = fila[k] as usize;
                k += 1;
                let (x, y) = (p % W, p / W);
                let viz = [y * W + (x + 1) % W, y * W + (x + W - 1) % W, if y + 1 < H { (y + 1) * W + x } else { usize::MAX }, if y > 0 { (y - 1) * W + x } else { usize::MAX }];
                for n in viz {
                    if n != usize::MAX && est[n] == sid as u32 && comp[n] == 0 {
                        comp[n] = cid;
                        fila.push(n as u32);
                    }
                }
            }
            comps.push(fila.clone());
        }
        let areas_c: Vec<f64> = comps.iter().map(|c| c.iter().map(|&i| area_linha[i as usize / W]).sum()).collect();
        let maior = (0..comps.len()).max_by(|&a, &b| areas_c[a].partial_cmp(&areas_c[b]).unwrap()).unwrap();

        // número alvo de províncias do estado
        let n_total = if area_total < alvo * 1.15 { 1 } else { (area_total / alvo).ceil().max(1.0) as usize };
        // distribui entre componentes "grandes" (>= 0.4 alvo) proporcionalmente à área
        let grandes: Vec<usize> = (0..comps.len()).filter(|&c| c == maior || areas_c[c] >= 0.4 * alvo).collect();
        let area_g: f64 = grandes.iter().map(|&c| areas_c[c]).sum();
        let mut sementes: Vec<(usize, V3, String, bool)> = Vec::new(); // (comp, vec, nome, cidade)
        for &c in &grandes {
            let n_c = if grandes.len() == 1 { n_total } else { ((n_total as f64 * areas_c[c] / area_g).round() as usize).max(1) };
            let n_c = if n_total == 1 { 1 } else { n_c };
            let pc = &comps[c];
            let mut sem_c: Vec<(V3, String, bool)> = Vec::new();
            let espac = 0.6 * (areas_c[c] / n_c as f64).sqrt();
            if n_c >= 1 {
                for &ci in &cid_est[sid] {
                    if sem_c.len() >= n_c {
                        break;
                    }
                    let cd = &cidades[ci];
                    let (px, py) = projetar_miller(cd.lon, cd.lat, W as u32, H as u32);
                    let (x, y) = ((px as usize).min(W - 1), (py as usize).min(H - 1));
                    if comp[y * W + x] != c as u32 + 1 {
                        continue;
                    }
                    let v = vec3(lon_c[x], lat_c[y]);
                    if sem_c.iter().all(|(s, _, _)| dist_km(s, &v) >= espac) {
                        sem_c.push((v, cd.nome.clone(), true));
                    }
                }
            }
            if sem_c.len() < n_c {
                // amostragem do ponto mais distante
                let passo = ((pc.len() as f64 / (8.0 * n_c as f64)).sqrt() as usize).clamp(1, 4);
                let amostra: Vec<(V3, u32)> = pc
                    .iter()
                    .filter(|&&i| (i as usize % W) % passo == 0 && (i as usize / W) % passo == 0)
                    .map(|&i| (vec3(lon_c[i as usize % W], lat_c[i as usize / W]), i))
                    .collect();
                let amostra = if amostra.is_empty() { vec![(vec3(lon_c[pc[0] as usize % W], lat_c[pc[0] as usize / W]), pc[0])] } else { amostra };
                let mut mind: Vec<f64> = vec![-2.0; amostra.len()]; // maior dot = mais perto
                for (s, _, _) in &sem_c {
                    for (k, (v, _)) in amostra.iter().enumerate() {
                        mind[k] = mind[k].max(dot(s, v));
                    }
                }
                while sem_c.len() < n_c {
                    let (k, _) = mind.iter().enumerate().min_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap();
                    let v = amostra[k].0;
                    sem_c.push((v, String::new(), false));
                    for (j, (w, _)) in amostra.iter().enumerate() {
                        mind[j] = mind[j].max(dot(&v, w));
                    }
                    if mind[k] > 0.999999999 && sem_c.len() > amostra.len() {
                        break;
                    }
                }
            }
            for (v, nm, cid) in sem_c {
                sementes.push((c, v, nm, cid));
            }
        }

        // atribuição por pixel (com distorção suave do domínio)
        let base = sementes_info.len() as u32;
        let n_sem = sementes.len();
        let semente_comp: Vec<usize> = sementes.iter().map(|s| s.0).collect();
        for (c, pc) in comps.iter().enumerate() {
            let candidatas: Vec<usize> = if grandes.contains(&c) { (0..n_sem).filter(|&k| semente_comp[k] == c).collect() } else { (0..n_sem).collect() };
            for &i in pc {
                let (x, y) = (i as usize % W, i as usize / W);
                let k_best = if candidatas.len() == 1 {
                    candidatas[0]
                } else {
                    let amp = 5.0;
                    let wx = x as f32 + 0.5 + amp * (ruido(x as f32, y as f32, 22.0, 1) - 0.5) * 2.0 + 2.0 * (ruido(x as f32, y as f32, 7.0, 3) - 0.5) * 2.0;
                    let wy = y as f32 + 0.5 + amp * (ruido(x as f32, y as f32, 22.0, 2) - 0.5) * 2.0 + 2.0 * (ruido(x as f32, y as f32, 7.0, 4) - 0.5) * 2.0;
                    let (lo, la) = desprojetar_miller(wx as f64, (wy as f64).clamp(0.0, H as f64), W as u32, H as u32);
                    let v = vec3(lo, la);
                    let mut best = candidatas[0];
                    let mut bd = -2.0;
                    for &k in &candidatas {
                        let d = dot(&sementes[k].1, &v);
                        if d > bd {
                            bd = d;
                            best = k;
                        }
                    }
                    best
                };
                prov[i as usize] = base + k_best as u32 + 1;
            }
        }
        for (_, _, nm, cid) in sementes.iter().map(|s| (s.0, s.1, s.2.clone(), s.3)) {
            sementes_info.push((sid as u32, nm, cid));
        }
        total_provs_estado[sid] = n_sem as u32;

        // limpeza: fragmentos desconectados de um mesmo rótulo -> vizinho dominante do mesmo estado
        if n_sem > 1 {
            limpar_fragmentos(pix, &mut prov, &est, sid as u32, &mut fila);
        }
        // reseta marcação de componentes
        for &i in pix {
            comp[i as usize] = 0;
        }
    }
    println!("sementes/rotulagem {:.1}s | provincias brutas: {}", t0.elapsed().as_secs_f32(), sementes_info.len());

    // --- renumeração final: por estado, a província que contém o centroide_px antigo vem primeiro ---
    let n_rot = sementes_info.len();
    let mut pix_rot: Vec<u32> = vec![0; n_rot + 1];
    for &r in &prov {
        pix_rot[r as usize] += 1;
    }
    let mut por_estado: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for r in 1..=n_rot as u32 {
        if pix_rot[r as usize] > 0 {
            por_estado.entry(sementes_info[r as usize - 1].0).or_default().push(r);
        }
    }
    let mut novo_id: Vec<u32> = vec![0; n_rot + 1];
    let mut ordem: Vec<(u32, u32)> = Vec::new(); // (rótulo, estado)
    for sid in 1..nmax as u32 {
        let Some(rots) = por_estado.get_mut(&sid) else { continue };
        let (cx, cy) = estados.provincias[sid as usize - 1].centroide_px;
        let r0 = prov[cy as usize * W + cx as usize];
        if let Some(pos) = rots.iter().position(|&r| r == r0) {
            rots.swap(0, pos);
        }
        for &r in rots.iter() {
            ordem.push((r, sid));
        }
    }
    for (k, (r, _)) in ordem.iter().enumerate() {
        novo_id[*r as usize] = k as u32 + 1;
    }
    let total = ordem.len();
    assert!(total < 65000, "ids estouram o limite de 16 bits");

    // --- estatísticas por província ---
    let mut area = vec![0f64; total + 1];
    let mut soma = vec![[0f64; 3]; total + 1];
    let mut npx = vec![0u32; total + 1];
    let mut raster: Vec<u16> = vec![0; W * H];
    for i in 0..W * H {
        let r = prov[i];
        if r == 0 {
            continue;
        }
        let id = novo_id[r as usize] as usize;
        raster[i] = id as u16;
        let (x, y) = (i % W, i / W);
        let v = vec3(lon_c[x], lat_c[y]);
        for k in 0..3 {
            soma[id][k] += v[k];
        }
        area[id] += area_linha[y];
        npx[id] += 1;
    }
    let centro_px = |id: usize| -> (f64, f64) {
        let s = soma[id];
        let lon = s[1].atan2(s[0]).to_degrees();
        let lat = s[2].atan2((s[0] * s[0] + s[1] * s[1]).sqrt()).to_degrees();
        (lon, lat)
    };
    let mut melhor: Vec<(f64, u32, u32)> = vec![(f64::MAX, 0, 0); total + 1];
    for i in 0..W * H {
        let id = raster[i] as usize;
        if id == 0 {
            continue;
        }
        let (lon, lat) = centro_px(id);
        let (mx, my) = projetar_miller(lon, lat, W as u32, H as u32);
        let (x, y) = (i % W, i / W);
        let dx = {
            let d = (x as f64 + 0.5 - mx).abs();
            d.min(W as f64 - d)
        };
        let d2 = dx * dx + (y as f64 + 0.5 - my).powi(2);
        if d2 < melhor[id].0 {
            melhor[id] = (d2, x as u32, y as u32);
        }
    }

    // --- vizinhos ---
    let mut viz: Vec<Vec<u32>> = vec![Vec::new(); total + 1];
    for y in 0..H {
        for x in 0..W {
            let a = raster[y * W + x] as usize;
            if a == 0 {
                continue;
            }
            let mut tenta = |b: usize| {
                if b != 0 && b != a {
                    viz[a].push(b as u32);
                    viz[b].push(a as u32);
                }
            };
            tenta(raster[y * W + (x + 1) % W] as usize);
            if y + 1 < H {
                tenta(raster[(y + 1) * W + x] as usize);
            }
        }
    }
    for v in &mut viz {
        v.sort_unstable();
        v.dedup();
    }

    // --- nomes únicos por estado ---
    let mut provincias: Vec<ProvinciaMapa> = Vec::with_capacity(total);
    let mut cont_estado: HashMap<u32, u32> = HashMap::new();
    let mut usados: HashMap<(u32, String), u32> = HashMap::new();
    for (k, &(r, sid)) in ordem.iter().enumerate() {
        let id = k + 1;
        let e = &estados.provincias[sid as usize - 1];
        let n = cont_estado.entry(sid).or_insert(0);
        *n += 1;
        let (_, nm, _) = &sementes_info[r as usize - 1];
        let base = if nm.is_empty() { format!("{} {}", e.nome, n) } else { nm.clone() };
        let c = usados.entry((sid, base.clone())).or_insert(0);
        *c += 1;
        let nome = if *c > 1 { format!("{base} {c}") } else { base };
        let (lon, lat) = centro_px(id);
        let (_, px, py) = melhor[id];
        provincias.push(ProvinciaMapa {
            id: id as u32,
            adm1_code: e.adm1_code.clone(),
            estado: e.nome.clone(),
            nome,
            cor: cor_unica_provincia(id as u32),
            centroide: (lon as f32, lat as f32),
            centroide_px: (px, py),
            area: area[id].max(1.0) as f32,
            vizinhos: std::mem::take(&mut viz[id]),
            pais_moderno: e.pais_moderno.clone(),
            adm0_moderno: e.adm0_moderno.clone(),
        });
    }
    // estados sem nenhum pixel (sobrescritos no mapa antigo): mantém 1 registro para preservar a cobertura adm1
    let tem: std::collections::HashSet<String> = provincias.iter().map(|p| p.adm1_code.clone()).collect();
    let mut sem_pixel = 0;
    for e in &estados.provincias {
        if !tem.contains(&e.adm1_code) {
            sem_pixel += 1;
            let id = provincias.len() as u32 + 1;
            let mut p = e.clone();
            p.id = id;
            p.cor = cor_unica_provincia(id);
            p.estado = e.nome.clone();
            p.vizinhos = Vec::new();
            provincias.push(p);
        }
    }
    println!("estados sem pixel (mantidos sem território): {sem_pixel}");

    // --- escrita ---
    let mut buf: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(W as u32, H as u32);
    for (i, p) in buf.pixels_mut().enumerate() {
        let c = cor_unica_provincia(raster[i] as u32);
        *p = Rgb([c.0, c.1, c.2]);
    }
    let saida = raiz.join("assets/map");
    fs::create_dir_all(&saida)?;
    buf.save(saida.join("provinces.png"))?;
    let cat = CatalogoProvincias { largura: W as u32, altura: H as u32, projecao: "miller".into(), provincias: provincias.clone() };
    fs::write(saida.join("provinces.ron"), ron::ser::to_string_pretty(&cat, ron::ser::PrettyConfig::default())?)?;

    // donos por província (herdados do estado, via por_adm1)
    let caminho_donos = raiz.join("data/scenario_1945/province_owners_1945.ron");
    let donos_ant: DonosProvincias1945 = ron::from_str(&fs::read_to_string(&caminho_donos)?)?;
    let mut por_id = BTreeMap::new();
    for p in &provincias {
        if let Some(t) = donos_ant.por_adm1.get(&p.adm1_code) {
            por_id.insert(p.id, t.clone());
        }
    }
    let donos = DonosProvincias1945 { por_id, por_adm1: donos_ant.por_adm1 };
    fs::write(&caminho_donos, ron::ser::to_string_pretty(&donos, ron::ser::PrettyConfig::default())?)?;

    let mut por_pais: BTreeMap<&str, usize> = BTreeMap::new();
    for p in &provincias {
        *por_pais.entry(p.adm0_moderno.as_str()).or_default() += 1;
    }
    println!("TOTAL provincias: {} | BRA: {} | USA: {} | RUS: {} | CHN: {}", provincias.len(), por_pais.get("BRA").unwrap_or(&0), por_pais.get("USA").unwrap_or(&0), por_pais.get("RUS").unwrap_or(&0), por_pais.get("CHN").unwrap_or(&0));
    println!("tempo total {:.1}s", t0.elapsed().as_secs_f32());
    Ok(())
}

/// Incorpora fragmentos desconectados (todos menos o maior componente de cada rótulo) ao rótulo vizinho dominante.
fn limpar_fragmentos(pix: &[u32], prov: &mut [u32], est: &[u32], sid: u32, fila: &mut Vec<u32>) {
    let mut visto: HashMap<u32, ()> = HashMap::new();
    let mut componentes: Vec<(u32, Vec<u32>)> = Vec::new();
    for &p0 in pix {
        if visto.contains_key(&p0) {
            continue;
        }
        let r = prov[p0 as usize];
        fila.clear();
        fila.push(p0);
        visto.insert(p0, ());
        let mut k = 0;
        while k < fila.len() {
            let p = fila[k] as usize;
            k += 1;
            let (x, y) = (p % W, p / W);
            let viz = [y * W + (x + 1) % W, y * W + (x + W - 1) % W, if y + 1 < H { (y + 1) * W + x } else { usize::MAX }, if y > 0 { (y - 1) * W + x } else { usize::MAX }];
            for n in viz {
                if n != usize::MAX && est[n] == sid && prov[n] == r && !visto.contains_key(&(n as u32)) {
                    visto.insert(n as u32, ());
                    fila.push(n as u32);
                }
            }
        }
        componentes.push((r, fila.clone()));
    }
    let mut maior: HashMap<u32, usize> = HashMap::new();
    for (r, c) in &componentes {
        let m = maior.entry(*r).or_insert(0);
        *m = (*m).max(c.len());
    }
    for (r, c) in &componentes {
        if c.len() == maior[r] {
            continue;
        }
        let mut votos: HashMap<u32, u32> = HashMap::new();
        for &p in c {
            let (x, y) = (p as usize % W, p as usize / W);
            let viz = [y * W + (x + 1) % W, y * W + (x + W - 1) % W, if y + 1 < H { (y + 1) * W + x } else { usize::MAX }, if y > 0 { (y - 1) * W + x } else { usize::MAX }];
            for n in viz {
                if n != usize::MAX && est[n] == sid && prov[n] != *r {
                    *votos.entry(prov[n]).or_default() += 1;
                }
            }
        }
        if let Some((&novo, _)) = votos.iter().max_by_key(|(k, v)| (**v, std::cmp::Reverse(**k))) {
            for &p in c {
                prov[p as usize] = novo;
            }
        }
    }
}
