//! Ícones vetoriais desenhados com o `egui::Painter` (traço de cartaz 1940s).
//! Sem texturas: linhas, retângulos e polígonos com tinta do tema.
//! Usados nas abas de categoria, como fallback de PNGs ausentes (bens, prédios, leis)
//! e em cartões/chapas do HUD.

use bevy_egui::egui::{self, Color32, Rect, Shape, Stroke};

/// Desenha o glifo `chave` dentro de `rect` na cor `tinta` (com acento `sec`).
pub fn desenhar(painter: &egui::Painter, rect: Rect, chave: &str, tinta: Color32, sec: Color32) {
    let r = rect.shrink(rect.width() * 0.08);
    let s = (r.width() * 0.075).max(1.2);
    let e = Stroke::new(s, tinta);
    let p = |fx: f32, fy: f32| r.min + r.size() * Vec2x(fx, fy);
    macro_rules! seg {
        ($a:expr, $b:expr) => { painter.line_segment([$a, $b], e) };
        ($a:expr, $b:expr, $st:expr) => { painter.line_segment([$a, $b], $st) };
    }
    macro_rules! poly {
        ($pts:expr, $fill:expr) => { painter.add(Shape::convex_polygon($pts.to_vec(), $fill, Stroke::NONE)); };
        ($pts:expr, $fill:expr, $st:expr) => { painter.add(Shape::convex_polygon($pts.to_vec(), $fill, $st)); };
    }
    match chave {
        "caixa" => {
            painter.rect_stroke(Rect::from_min_max(p(0.06, 0.24), p(0.94, 0.94)), 0.0, e, egui::StrokeKind::Middle);
            seg!(p(0.5, 0.24), p(0.5, 0.94));
            seg!(p(0.06, 0.24), p(0.24, 0.04), Stroke::new(s, sec));
            seg!(p(0.94, 0.24), p(0.76, 0.04), Stroke::new(s, sec));
            seg!(p(0.24, 0.04), p(0.76, 0.04), Stroke::new(s, sec));
        }
        "trigo" => {
            seg!(p(0.5, 0.96), p(0.5, 0.16));
            for fy in [0.24, 0.42, 0.6] {
                seg!(p(0.5, fy + 0.1), p(0.22, fy), Stroke::new(s * 1.6, tinta));
                seg!(p(0.5, fy + 0.1), p(0.78, fy), Stroke::new(s * 1.6, tinta));
            }
            seg!(p(0.5, 0.26), p(0.5, 0.04), Stroke::new(s * 1.6, tinta));
        }
        "picareta" => {
            seg!(p(0.28, 0.96), p(0.66, 0.22));
            let head = Stroke::new(s * 1.5, tinta);
            seg!(p(0.12, 0.3), p(0.3, 0.1), head);
            seg!(p(0.3, 0.1), p(0.52, 0.04), head);
            seg!(p(0.52, 0.04), p(0.74, 0.1), head);
            seg!(p(0.74, 0.1), p(0.92, 0.34), head);
        }
        "raio" => {
            poly!(
                [p(0.58, 0.02), p(0.24, 0.54), p(0.46, 0.54), p(0.36, 0.98), p(0.76, 0.42), p(0.52, 0.42)],
                sec, e
            );
        }
        "fabrica" => {
            poly!([p(0.06, 0.96), p(0.06, 0.52), p(0.28, 0.36), p(0.28, 0.52), p(0.5, 0.36), p(0.5, 0.52), p(0.72, 0.36), p(0.72, 0.96)], Color32::TRANSPARENT, e);
            painter.rect_stroke(Rect::from_min_max(p(0.78, 0.12), p(0.92, 0.96)), 0.0, e, egui::StrokeKind::Middle);
            seg!(p(0.78, 0.26), p(0.92, 0.26), Stroke::new(s, sec));
            painter.rect_filled(Rect::from_min_max(p(0.16, 0.68), p(0.26, 0.86)), 0.0, sec);
        }
        "camisa" => {
            poly!(
                [p(0.3, 0.06), p(0.5, 0.16), p(0.7, 0.06), p(0.96, 0.26), p(0.82, 0.42), p(0.82, 0.96), p(0.18, 0.96), p(0.18, 0.42), p(0.04, 0.26)],
                Color32::TRANSPARENT, e
            );
        }
        "diamante" => {
            poly!([p(0.5, 0.04), p(0.94, 0.38), p(0.5, 0.96), p(0.06, 0.38)], sec.linear_multiply(0.25), e);
            seg!(p(0.06, 0.38), p(0.94, 0.38));
            seg!(p(0.3, 0.38), p(0.5, 0.04));
            seg!(p(0.7, 0.38), p(0.5, 0.04));
        }
        "canhao" => {
            painter.rect_filled(Rect::from_min_max(p(0.3, 0.2), p(0.94, 0.44)), 0.0, tinta);
            painter.rect_filled(Rect::from_min_max(p(0.06, 0.44), p(0.62, 0.72)), 0.0, tinta);
            painter.circle_stroke(p(0.28, 0.86), r.height() * 0.14, Stroke::new(s * 1.4, tinta));
            painter.circle_stroke(p(0.54, 0.86), r.height() * 0.14, Stroke::new(s * 1.4, tinta));
        }
        "engrenagem" => {
            let c = r.center();
            let ra = r.width() * 0.3;
            for k in 0..6 {
                let a = k as f32 * std::f32::consts::TAU / 6.0;
                let d = egui::vec2(a.cos(), a.sin());
                seg!(c + d * (ra - r.width() * 0.06), c + d * (ra + r.width() * 0.16), Stroke::new(s * 2.4, tinta));
            }
            painter.circle_stroke(c, ra, Stroke::new(s * 1.5, tinta));
            painter.circle_filled(c, ra * 0.42, sec);
        }
        "balanca" => {
            seg!(p(0.5, 0.06), p(0.5, 0.9));
            seg!(p(0.2, 0.94), p(0.8, 0.94), Stroke::new(s * 1.6, tinta));
            seg!(p(0.14, 0.2), p(0.86, 0.2), Stroke::new(s * 1.4, tinta));
            seg!(p(0.14, 0.2), p(0.06, 0.5));
            seg!(p(0.14, 0.2), p(0.22, 0.5));
            painter.add(Shape::convex_polygon([p(0.02, 0.5), p(0.26, 0.5), p(0.14, 0.68)].to_vec(), sec.linear_multiply(0.35), e));
            seg!(p(0.86, 0.2), p(0.78, 0.5));
            seg!(p(0.86, 0.2), p(0.94, 0.5));
            painter.add(Shape::convex_polygon([p(0.74, 0.5), p(0.98, 0.5), p(0.86, 0.68)].to_vec(), sec.linear_multiply(0.35), e));
        }
        "urna" => {
            painter.rect_stroke(Rect::from_min_max(p(0.1, 0.42), p(0.9, 0.94)), 0.0, e, egui::StrokeKind::Middle);
            seg!(p(0.3, 0.42), p(0.7, 0.42), Stroke::new(s * 2.0, tinta));
            painter.rect_filled(Rect::from_min_max(p(0.4, 0.06), p(0.62, 0.34)), 0.0, sec);
        }
        "coroa" => {
            poly!(
                [p(0.08, 0.78), p(0.04, 0.2), p(0.28, 0.44), p(0.5, 0.12), p(0.72, 0.44), p(0.96, 0.2), p(0.92, 0.78)],
                sec.linear_multiply(0.3), e
            );
        }
        "livro" => {
            painter.add(Shape::convex_polygon([p(0.06, 0.16), p(0.5, 0.24), p(0.5, 0.92), p(0.06, 0.84)].to_vec(), Color32::TRANSPARENT, e));
            painter.add(Shape::convex_polygon([p(0.94, 0.16), p(0.5, 0.24), p(0.5, 0.92), p(0.94, 0.84)].to_vec(), Color32::TRANSPARENT, e));
            seg!(p(0.14, 0.36), p(0.42, 0.42), Stroke::new(s * 0.8, sec));
            seg!(p(0.58, 0.42), p(0.86, 0.36), Stroke::new(s * 0.8, sec));
        }
        "trem" => {
            painter.rect_stroke(Rect::from_min_max(p(0.06, 0.3), p(0.62, 0.72)), 0.0, e, egui::StrokeKind::Middle);
            painter.rect_filled(Rect::from_min_max(p(0.62, 0.46), p(0.94, 0.72)), 0.0, tinta);
            painter.circle_filled(p(0.24, 0.84), r.height() * 0.11, tinta);
            painter.circle_filled(p(0.5, 0.84), r.height() * 0.11, tinta);
            painter.rect_filled(Rect::from_min_max(p(0.14, 0.08), p(0.26, 0.3)), 0.0, sec);
        }
        "navio" => {
            poly!([p(0.04, 0.66), p(0.96, 0.66), p(0.8, 0.9), p(0.18, 0.9)], Color32::TRANSPARENT, e);
            seg!(p(0.5, 0.66), p(0.5, 0.1));
            poly!([p(0.52, 0.12), p(0.84, 0.24), p(0.52, 0.36)], sec);
            seg!(p(0.02, 0.96), p(0.98, 0.96), Stroke::new(s, sec));
        }
        "aviao" => {
            poly!([p(0.5, 0.04), p(0.62, 0.5), p(0.5, 0.96), p(0.38, 0.5)], Color32::TRANSPARENT, e);
            poly!([p(0.04, 0.54), p(0.96, 0.54), p(0.66, 0.66), p(0.34, 0.66)], sec.linear_multiply(0.3), e);
            poly!([p(0.22, 0.14), p(0.78, 0.14), p(0.6, 0.24), p(0.4, 0.24)], Color32::TRANSPARENT, e);
        }
        "moeda" => {
            let c = r.center();
            painter.circle_stroke(c, r.width() * 0.44, Stroke::new(s * 1.5, tinta));
            painter.circle_stroke(c, r.width() * 0.34, Stroke::new(s * 0.8, sec));
            seg!(c - egui::vec2(0.0, r.height() * 0.22), c + egui::vec2(0.0, r.height() * 0.22), Stroke::new(s * 1.2, tinta));
        }
        "torre" => {
            seg!(p(0.3, 0.96), p(0.3, 0.06), Stroke::new(s * 1.6, tinta));
            seg!(p(0.3, 0.1), p(0.94, 0.3), Stroke::new(s * 1.4, tinta));
            seg!(p(0.78, 0.24), p(0.78, 0.56));
            painter.rect_filled(Rect::from_min_max(p(0.72, 0.56), p(0.86, 0.72)), 0.0, sec);
            seg!(p(0.12, 0.96), p(0.52, 0.96), Stroke::new(s * 1.6, tinta));
        }
        "gota" => {
            painter.add(Shape::convex_polygon(
                [p(0.5, 0.04), p(0.82, 0.56), p(0.5, 0.96), p(0.18, 0.56)].to_vec(),
                sec.linear_multiply(0.35), e,
            ));
        }
        "estrela" => {
            let c = r.center();
            let mut pts = Vec::with_capacity(10);
            for k in 0..10 {
                let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::TAU / 10.0;
                let rad = if k % 2 == 0 { r.width() * 0.48 } else { r.width() * 0.2 };
                pts.push(c + egui::vec2(a.cos(), a.sin()) * rad);
            }
            painter.add(Shape::convex_polygon(pts, sec.linear_multiply(0.4), e));
        }
        "escudo" => {
            poly!(
                [p(0.5, 0.02), p(0.94, 0.18), p(0.86, 0.62), p(0.5, 0.98), p(0.14, 0.62), p(0.06, 0.18)],
                Color32::TRANSPARENT, e
            );
            seg!(p(0.5, 0.1), p(0.5, 0.86), Stroke::new(s, sec));
            seg!(p(0.18, 0.36), p(0.82, 0.36), Stroke::new(s, sec));
        }
        "grafico" => {
            seg!(p(0.06, 0.94), p(0.98, 0.94));
            seg!(p(0.06, 0.94), p(0.06, 0.06));
            painter.rect_filled(Rect::from_min_max(p(0.2, 0.6), p(0.36, 0.9)), 0.0, tinta);
            painter.rect_filled(Rect::from_min_max(p(0.44, 0.36), p(0.6, 0.9)), 0.0, tinta);
            painter.rect_filled(Rect::from_min_max(p(0.68, 0.16), p(0.84, 0.9)), 0.0, sec);
        }
        "jornal" => {
            painter.rect_stroke(Rect::from_min_max(p(0.06, 0.08), p(0.94, 0.94)), 0.0, e, egui::StrokeKind::Middle);
            painter.rect_filled(Rect::from_min_max(p(0.14, 0.16), p(0.86, 0.3)), 0.0, tinta);
            for fy in [0.42, 0.56, 0.7, 0.84] {
                seg!(p(0.14, fy), p(0.6, fy), Stroke::new(s * 0.8, sec));
                seg!(p(0.68, fy), p(0.86, fy), Stroke::new(s * 0.8, sec));
            }
        }
        "pessoa" => {
            painter.circle_filled(p(0.5, 0.28), r.width() * 0.18, tinta);
            painter.add(Shape::convex_polygon(
                [p(0.14, 0.94), p(0.28, 0.56), p(0.5, 0.48), p(0.72, 0.56), p(0.86, 0.94)].to_vec(),
                tinta, Stroke::NONE,
            ));
        }
        "mais" => {
            seg!(p(0.5, 0.1), p(0.5, 0.9), Stroke::new(s * 1.8, tinta));
            seg!(p(0.1, 0.5), p(0.9, 0.5), Stroke::new(s * 1.8, tinta));
        }
        "lupa" => {
            let c = p(0.42, 0.4);
            painter.circle_stroke(c, r.width() * 0.28, e);
            seg!(p(0.62, 0.62), p(0.92, 0.92), Stroke::new(s * 1.6, sec));
        }
        "falante" => {
            painter.rect_stroke(Rect::from_min_max(p(0.14, 0.08), p(0.86, 0.94)), 0.0, e, egui::StrokeKind::Middle);
            painter.circle_stroke(r.center() + egui::vec2(0.0, r.height() * 0.16), r.width() * 0.22, Stroke::new(s * 1.4, tinta));
            painter.circle_filled(r.center() + egui::vec2(0.0, r.height() * 0.16), r.width() * 0.07, sec);
            painter.circle_filled(p(0.5, 0.26), r.width() * 0.08, tinta);
        }
        "monitor" => {
            painter.rect_stroke(Rect::from_min_max(p(0.06, 0.12), p(0.94, 0.7)), 0.0, e, egui::StrokeKind::Middle);
            painter.rect_filled(Rect::from_min_max(p(0.14, 0.2), p(0.86, 0.62)), 0.0, sec.linear_multiply(0.22));
            seg!(p(0.5, 0.7), p(0.5, 0.86));
            seg!(p(0.26, 0.9), p(0.74, 0.9), Stroke::new(s * 1.8, tinta));
        }
        "seta_cima" => {
            poly!([p(0.5, 0.14), p(0.86, 0.82), p(0.14, 0.82)], tinta);
        }
        "seta_baixo" => {
            poly!([p(0.14, 0.18), p(0.86, 0.18), p(0.5, 0.86)], tinta);
        }
        "relogio" => {
            let c = p(0.5, 0.54);
            painter.circle_stroke(c, r.width() * 0.40, e);
            seg!(c, p(0.5, 0.3), Stroke::new(s * 1.3, tinta));
            seg!(c, p(0.66, 0.54), Stroke::new(s * 1.3, tinta));
            painter.circle_filled(c, r.width() * 0.05, sec);
            seg!(p(0.36, 0.06), p(0.64, 0.06), Stroke::new(s * 1.6, sec));
        }
        _ => {
            painter.rect_stroke(r, 0.0, e, egui::StrokeKind::Middle);
            seg!(p(0.2, 0.2), p(0.8, 0.8), Stroke::new(s * 0.8, sec));
            seg!(p(0.8, 0.2), p(0.2, 0.8), Stroke::new(s * 0.8, sec));
        }
    }
}

struct Vec2x(f32, f32);
impl std::ops::Mul<Vec2x> for egui::Vec2 {
    type Output = egui::Vec2;
    fn mul(self, v: Vec2x) -> egui::Vec2 {
        egui::Vec2::new(self.x * v.0, self.y * v.1)
    }
}

/// Escolhe o glifo adequado para um id de bem/prédio/lei/população.
pub fn chave_para(id: &str) -> &'static str {
    let i = id.to_lowercase();
    let tem = |k: &str| i.contains(k);
    if tem("eletric") || tem("energia") || tem("eletron") && tem("gera") {
        "raio"
    } else if tem("petroleo") || tem("oleo") || tem("gas") || tem("combustiv") {
        "gota"
    } else if tem("carvao") {
        "picareta"
    } else if tem("mineri") || tem("minerio") || tem("cobre") || tem("bauxita") || tem("aluminio") || tem("uranio") || tem("chumbo") || tem("zinco") {
        "picareta"
    } else if tem("aco") || tem("quimic") || tem("ciment") || tem("maquin") || tem("turbina") || tem("industr") || tem("metalurg") || tem("auto") {
        "fabrica"
    } else if tem("trigo") || tem("graos") || tem("cereais") || tem("carne") || tem("leite") || tem("algodao") || tem("cafe") || tem("cha") || tem("acucar") || tem("borracha") || tem("madeira") || tem("toras") || tem("peixe") || tem("alimento") || tem("bebida") || tem("cereal") {
        "trigo"
    } else if tem("trem") || tem("ferrovia") || tem("ferroviari") || tem("transporte") || tem("logistica") || tem("caminh") {
        "trem"
    } else if tem("navio") || tem("marit") || tem("portuar") || tem("rota") || tem("costeiro") {
        "navio"
    } else if tem("aviao") || tem("aereo") || tem("aeronau") || tem("avia") {
        "aviao"
    } else if tem("arma") || tem("muni") || tem("militar") || tem("tanque") || tem("artilh") || tem("soldado") || tem("exercito") || tem("guerra") || tem("quartel") {
        "canhao"
    } else if tem("eleicao") || tem("voto") || tem("partido") || tem("urna") {
        "urna"
    } else if tem("lei") || tem("justica") || tem("direito") || tem("tribunal") || tem("constituic") {
        "balanca"
    } else if tem("governo") || tem("autoridade") || tem("monarqui") || tem("estado") || tem("diplomacia") {
        "coroa"
    } else if tem("administra") || tem("burocrat") || tem("escola") || tem("educ") || tem("univers") || tem("livro") || tem("imprensa") {
        "livro"
    } else if tem("banco") || tem("financ") || tem("credito") || tem("dinheiro") || tem("fundo") || tem("seguro") || tem("comercio") || tem("comerc") {
        "moeda"
    } else if tem("eletrodom") || tem("radio") || tem("roupa") || tem("text") || tem("vestuario") || tem("moveis") || tem("calcad") {
        "camisa"
    } else if tem("luxo") || tem("joalheri") || tem("joia") || tem("diamante") || tem("alta_costura") || tem("ouro") {
        "diamante"
    } else if tem("servic") || tem("telecom") || tem("telefone") || tem("ensaio") || tem("laborator") {
        "engrenagem"
    } else if tem("pop") || tem("demog") || tem("campones") || tem("aristocrat") || tem("capitalist") || tem("clero") {
        "pessoa"
    } else if tem("construc") || tem("obra") || tem("edific") || tem("habita") || tem("moradia") || tem("arranha") {
        "torre"
    } else if tem("pesquisa") || tem("tecn") || tem("nuclear") || tem("espacial") || tem("foguete") || tem("satelite") || tem("reato") {
        "estrela"
    } else if tem("defesa") || tem("fortalez") || tem("muralha") || tem("bunker") {
        "escudo"
    } else if tem("estat") || tem("grafico") || tem("censo") {
        "grafico"
    } else if tem("jornal") || tem("imprensa_esc") || tem("midia") {
        "jornal"
    } else {
        "caixa"
    }
}

/// Desenha o glifo de `id` num quadrado de `tam` px dentro de `ui` (não interativo).
pub fn glifo_ui(ui: &mut egui::Ui, chave: &str, tam: f32, tinta: Option<egui::Color32>) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(tam, tam), egui::Sense::hover());
    let (t, sec) = (tinta.unwrap_or(super::paleta::menu::TINTA), super::paleta::menu::VERMELHO);
    desenhar(ui.painter(), rect, chave, t, sec);
}
