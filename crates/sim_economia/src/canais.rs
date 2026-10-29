//! Calendário histórico de canais (Suez, Panamá, Kiel, Corinto, Volga-Don, São Lourenço).
//! Dados em `assets/data/economia/canais.ron`; o tráfego de navios consulta `canal_disponivel` e `dono_canal`.
use crate::tipos::DataJogo;
use serde::Deserialize;
use std::sync::OnceLock;

type Dt = (u16, u8, u8);

#[derive(Debug, Clone, Deserialize)]
pub struct DonoPeriodo {
    pub pais: String,
    pub desde: Dt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Fechamento {
    pub de: Dt,
    pub ate: Dt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CanalHistorico {
    pub id: String,
    pub nome: String,
    pub pedagio_por_navio: f64,
    pub abertura: Dt,
    pub donos: Vec<DonoPeriodo>,
    pub fechamentos: Vec<Fechamento>,
}

const RON_CANAIS: &str = include_str!("../../../assets/data/economia/canais.ron");

/// Todos os canais do calendário embutido.
pub fn canais() -> &'static [CanalHistorico] {
    static CANAIS: OnceLock<Vec<CanalHistorico>> = OnceLock::new();
    CANAIS.get_or_init(|| ron::from_str(RON_CANAIS).expect("canais.ron inválido"))
}

fn tupla(d: DataJogo) -> Dt {
    (d.ano, d.mes, d.dia)
}

/// Canal aberto e operando na data (já inaugurado e fora de qualquer fechamento). `false` se o id não existe.
pub fn canal_disponivel(id: &str, data: DataJogo) -> bool {
    let t = tupla(data);
    canais().iter().find(|c| c.id == id).is_some_and(|c| {
        t >= c.abertura && !c.fechamentos.iter().any(|f| t >= f.de && t <= f.ate)
    })
}

/// País dono (que cobra o pedágio) na data; `None` se não existe ou ainda não foi inaugurado.
/// Para canais binacionais devolve o primeiro dono listado.
pub fn dono_canal(id: &str, data: DataJogo) -> Option<&'static str> {
    let t = tupla(data);
    let c = canais().iter().find(|c| c.id == id)?;
    if t < c.abertura {
        return None;
    }
    c.donos.iter().filter(|d| d.desde <= t).max_by_key(|d| d.desde).map(|d| d.pais.as_str())
}

#[cfg(test)]
mod testes {
    use super::*;
    fn d(a: u16, m: u8, di: u8) -> DataJogo {
        DataJogo::nova(a, m, di).unwrap()
    }

    #[test]
    fn suez_historia() {
        assert!(canal_disponivel("suez", d(1945, 1, 1)));
        assert_eq!(dono_canal("suez", d(1945, 1, 1)), Some("gbr"));
        assert_eq!(dono_canal("suez", d(1956, 7, 25)), Some("gbr"));
        assert_eq!(dono_canal("suez", d(1956, 7, 26)), Some("egy"));
        assert!(!canal_disponivel("suez", d(1956, 11, 1)));
        assert!(!canal_disponivel("suez", d(1957, 4, 24)));
        assert!(canal_disponivel("suez", d(1957, 4, 25)));
        assert!(!canal_disponivel("suez", d(1967, 6, 5)));
        assert!(!canal_disponivel("suez", d(1975, 6, 5)));
        assert!(canal_disponivel("suez", d(1975, 6, 6)));
    }

    #[test]
    fn outros_canais() {
        assert_eq!(dono_canal("panama", d(1945, 1, 1)), Some("usa"));
        assert_eq!(dono_canal("kiel", d(1945, 1, 1)), Some("ger"));
        assert_eq!(dono_canal("corinto", d(1945, 1, 1)), Some("gre"));
        assert!(!canal_disponivel("volga_don", d(1945, 1, 1)));
        assert_eq!(dono_canal("volga_don", d(1945, 1, 1)), None);
        assert!(canal_disponivel("volga_don", d(1952, 7, 27)));
        assert!(!canal_disponivel("sao_lourenco", d(1958, 12, 31)));
        assert!(canal_disponivel("sao_lourenco", d(1959, 4, 25)));
        assert!(!canal_disponivel("gibraltar", d(1945, 1, 1)));
    }
}
