//! Valores por PROVÍNCIA do mapa (7.740) para os modos de mapa temáticos.
//!
//! A economia conhece regiões (adm1). Cada região é repartida entre as províncias do mapa que
//! compartilham o mesmo `adm1_code`, por peso de cidades (população urbana de 1945) e área:
//! o PIB se concentra mais nas cidades (costa, capitais) do que a população.
use super::economia_sim::PerfilRegiao;
use gf_mapa::mapa::ProvinciaMapa;
use std::collections::HashMap;

/// Grandeza exibida num modo de mapa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metrica {
    Populacao,
    Densidade,
    PibTotal,
    PibPerCapita,
    PadraoVida,
    Urbanizacao,
    Industrializacao,
}

impl Metrica {
    /// Escala logarítmica (grandezas que variam por ordens de grandeza).
    pub fn logaritmica(self) -> bool {
        !matches!(self, Self::PadraoVida | Self::Urbanizacao)
    }

    pub fn formatar(self, v: f64) -> String {
        let g = |x: f64| {
            if x >= 1e9 {
                format!("{:.1} bi", x / 1e9)
            } else if x >= 1e6 {
                format!("{:.1} mi", x / 1e6)
            } else if x >= 1e3 {
                format!("{:.0} mil", x / 1e3)
            } else {
                format!("{x:.0}")
            }
        };
        match self {
            Self::Populacao => format!("{} hab.", g(v)),
            Self::Densidade => format!("{v:.1} hab./km²"),
            Self::PibTotal => format!("{}/ano", g(v)),
            Self::PibPerCapita | Self::Industrializacao => format!("{v:.0}/hab./ano"),
            Self::PadraoVida => format!("{v:.1}"),
            Self::Urbanizacao => format!("{:.0}%", v * 100.0),
        }
    }
}

/// Pesos fixos de cada província dentro da sua região (calculados uma vez).
pub struct PesosProvincias {
    adm1: Vec<String>,
    w_pop: Vec<f32>,
    w_pib: Vec<f32>,
    area: Vec<f32>,
    urbana: Vec<f64>,
}

impl PesosProvincias {
    /// `urbana_por_prov`: população urbana (cidades) já atribuída a cada id de província.
    pub fn novo(provs: &[ProvinciaMapa], urbana_por_prov: &HashMap<u32, f64>) -> Self {
        let n = provs.iter().map(|p| p.id as usize + 1).max().unwrap_or(0);
        let mut s = Self { adm1: vec![String::new(); n], w_pop: vec![0.0; n], w_pib: vec![0.0; n], area: vec![1.0; n], urbana: vec![0.0; n] };
        let mut soma: HashMap<&str, (f64, f64)> = HashMap::new();
        for p in provs {
            let i = p.id as usize;
            s.adm1[i] = p.adm1_code.clone();
            s.area[i] = p.area.max(1.0);
            s.urbana[i] = urbana_por_prov.get(&p.id).copied().unwrap_or(0.0);
            let e = soma.entry(p.adm1_code.as_str()).or_default();
            e.0 += s.urbana[i];
            e.1 += s.area[i] as f64;
        }
        for p in provs {
            let i = p.id as usize;
            let (su, sa) = soma[p.adm1_code.as_str()];
            let ca = s.area[i] as f64 / sa.max(1.0);
            if su > 0.0 {
                let cu = s.urbana[i] / su;
                s.w_pop[i] = (0.5 * cu + 0.5 * ca) as f32;
                s.w_pib[i] = (0.8 * cu + 0.2 * ca) as f32;
            } else {
                s.w_pop[i] = ca as f32;
                s.w_pib[i] = ca as f32;
            }
        }
        s
    }

    /// Valor da métrica em cada província (índice = id); `NaN` quando não há dado.
    pub fn valores(&self, m: Metrica, regioes: &[PerfilRegiao]) -> Vec<f64> {
        let por_adm1: HashMap<&str, &PerfilRegiao> = regioes.iter().map(|r| (r.adm1.as_str(), r)).collect();
        let mut out = vec![f64::NAN; self.adm1.len()];
        for (i, a) in self.adm1.iter().enumerate() {
            let Some(r) = por_adm1.get(a.as_str()) else { continue };
            let (wp, wb) = (self.w_pop[i] as f64, self.w_pib[i] as f64);
            let pop = r.populacao * wp;
            let pib = r.pib_anual * wb;
            if pop <= 0.0 {
                continue;
            }
            out[i] = match m {
                Metrica::Populacao => pop,
                Metrica::Densidade => pop / self.area[i] as f64,
                Metrica::PibTotal => pib,
                Metrica::PibPerCapita => pib / pop,
                Metrica::PadraoVida => r.padrao_vida * (1.0 + 0.1 * (wb / wp.max(1e-9)).ln().clamp(-1.0, 1.0)),
                Metrica::Urbanizacao => (self.urbana[i] / pop).clamp(0.0, 1.0),
                Metrica::Industrializacao => r.pib_industrial_anual * wb / pop,
            };
        }
        out
    }
}

/// Intervalo de cores (percentis 2%..98% dos valores positivos) e se é logarítmico.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Escala {
    pub min: f64,
    pub max: f64,
    pub log: bool,
}

impl Escala {
    pub fn de_valores(v: &[f64], log: bool) -> Self {
        let mut x: Vec<f64> = v.iter().copied().filter(|a| a.is_finite() && (!log || *a > 0.0)).collect();
        if x.is_empty() {
            return Self { min: 1.0, max: 2.0, log };
        }
        x.sort_by(|a, b| a.total_cmp(b));
        let q = |f: f64| x[((x.len() - 1) as f64 * f) as usize];
        let (min, mut max) = (q(0.02), q(0.98));
        if max <= min {
            max = min * 1.01 + 1e-9;
        }
        Self { min, max, log }
    }

    pub fn normalizar(&self, v: f64) -> f32 {
        if !v.is_finite() {
            return 0.0;
        }
        if self.log {
            super::hud_mapas::escala_log(v, self.min.max(1e-12), self.max)
        } else {
            ((v - self.min) / (self.max - self.min)).clamp(0.0, 1.0) as f32
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn prov(id: u32, adm1: &str, area: f32) -> ProvinciaMapa {
        ProvinciaMapa { id, adm1_code: adm1.into(), estado: String::new(), nome: String::new(), cor: (0, 0, 0), centroide: (0.0, 0.0), centroide_px: (0, 0), area, vizinhos: vec![], pais_moderno: String::new(), adm0_moderno: String::new() }
    }

    #[test]
    fn pib_concentra_na_cidade_e_soma_a_regiao() {
        let provs = vec![prov(1, "X", 100.0), prov(2, "X", 100.0)];
        let urb: HashMap<u32, f64> = [(1, 1_000_000.0)].into();
        let w = PesosProvincias::novo(&provs, &urb);
        let r = PerfilRegiao { adm1: "X".into(), populacao: 2e6, pib_anual: 1e9, padrao_vida: 10.0, pib_industrial_anual: 2e8 };
        let pib = w.valores(Metrica::PibTotal, std::slice::from_ref(&r));
        assert!(pib[1] > 3.0 * pib[2]);
        assert!(((pib[1] + pib[2]) - 1e9).abs() < 1e3);
        let pop = w.valores(Metrica::Populacao, std::slice::from_ref(&r));
        assert!(((pop[1] + pop[2]) - 2e6).abs() < 10.0);
        assert!(pop[1] > pop[2]);
    }

    #[test]
    fn escala_log_e_linear() {
        let e = Escala::de_valores(&[1.0, 10.0, 100.0, 1000.0, f64::NAN], true);
        assert!(e.normalizar(1000.0) > e.normalizar(10.0));
        assert_eq!(e.normalizar(f64::NAN), 0.0);
        let l = Escala::de_valores(&(0..=100).map(|i| i as f64 / 100.0).collect::<Vec<_>>(), false);
        assert!(l.normalizar(0.5) > 0.3 && l.normalizar(0.5) < 0.7);
    }
}
