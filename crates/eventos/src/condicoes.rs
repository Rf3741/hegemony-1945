//! Condições de disparo de eventos históricos e políticos sistêmicos.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    Ge,
    Le,
    Gt,
    Lt,
    Eq,
}

/// Condição já interpretada a partir do texto do `.ron` ou da estrutura `CondicoesEvento`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Condicao {
    AnoMin(i32),
    PosEvento(String),
    Soberano(String),
    TemLei(String),
    NaoTemLei(String),
    LegitimidadeMax(f32),
    LegitimidadeMin(f32),
    TemFlag(String),
    NaoTemFlag(String),
    RelacaoMin(String, f32),
    RelacaoMax(String, f32),
    /// Variável numérica comparada a um valor (`tensao_guerra_fria >= 40`).
    Compara(String, Op, f32),
    /// Sem dado no cliente para avaliar: assume verdadeira e avisa no log.
    Desconhecida(String),
}

impl Condicao {
    pub fn parse(s: &str) -> Condicao {
        let s = s.trim();
        if let Some((k, v)) = s.split_once(':') {
            let k = k.trim();
            let v = v.trim();
            return match k {
                "pos_evento" => Condicao::PosEvento(v.to_string()),
                "soberano" => Condicao::Soberano(v.to_string()),
                "tem_lei" => Condicao::TemLei(v.to_string()),
                "nao_tem_lei" | "sem_lei" => Condicao::NaoTemLei(v.to_string()),
                "tem_flag" | "flag" => Condicao::TemFlag(v.to_string()),
                "nao_tem_flag" | "sem_flag" => Condicao::NaoTemFlag(v.to_string()),
                "legitimidade_max" | "legitimidade_maxima" => {
                    v.parse::<f32>().map(Condicao::LegitimidadeMax).unwrap_or_else(|_| Condicao::Desconhecida(s.to_string()))
                }
                "legitimidade_min" | "legitimidade_minima" => {
                    v.parse::<f32>().map(Condicao::LegitimidadeMin).unwrap_or_else(|_| Condicao::Desconhecida(s.to_string()))
                }
                _ => {
                    if k.starts_with("relacao_min_") {
                        let tag = k.trim_start_matches("relacao_min_");
                        if let Ok(n) = v.parse::<f32>() {
                            return Condicao::RelacaoMin(tag.to_string(), n);
                        }
                    } else if k.starts_with("relacao_max_") {
                        let tag = k.trim_start_matches("relacao_max_");
                        if let Ok(n) = v.parse::<f32>() {
                            return Condicao::RelacaoMax(tag.to_string(), n);
                        }
                    }
                    Condicao::Desconhecida(s.to_string())
                }
            };
        }
        for (txt, op) in [(">=", Op::Ge), ("<=", Op::Le), ("==", Op::Eq), (">", Op::Gt), ("<", Op::Lt)] {
            if let Some((a, b)) = s.split_once(txt)
                && let Ok(n) = b.trim().parse::<f32>()
            {
                let a = a.trim();
                if a == "ano" && op == Op::Ge {
                    return Condicao::AnoMin(n as i32);
                }
                if a == "legitimidade" || a == "legitimidade_maxima" {
                    if op == Op::Le || op == Op::Lt {
                        return Condicao::LegitimidadeMax(n);
                    } else if op == Op::Ge || op == Op::Gt {
                        return Condicao::LegitimidadeMin(n);
                    }
                }
                return Condicao::Compara(a.to_string(), op, n);
            }
        }
        Condicao::Desconhecida(s.to_string())
    }
}
