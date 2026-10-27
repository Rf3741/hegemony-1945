//! Gerador de números pseudoaleatórios determinístico e serializável (Xoshiro256**).
//!
//! Garante determinismo idêntico entre execuções a partir de uma mesma semente `u64`,
//! sem alocações dinâmicas e com serialização completa do estado interno em RON.

use serde::{Deserialize, Serialize};

/// Gerador pseudoaleatório determinístico baseado no algoritmo `Xoshiro256**`.
///
/// Inicializado via expansão `SplitMix64` de uma semente de 64 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngDeterministico {
    estado: [u64; 4],
}

impl Default for RngDeterministico {
    fn default() -> Self {
        Self::com_semente(19_450_101)
    }
}

impl RngDeterministico {
    /// Cria um novo gerador a partir de uma semente `u64`.
    pub fn com_semente(semente: u64) -> Self {
        let mut sm = semente;
        let mut proximo_splitmix = || {
            sm = sm.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        };

        let mut estado = [
            proximo_splitmix(),
            proximo_splitmix(),
            proximo_splitmix(),
            proximo_splitmix(),
        ];

        // Garante que o estado não seja todo zero
        if estado.iter().all(|&x| x == 0) {
            estado[0] = 0x853C49E6748FEA9B;
        }

        Self { estado }
    }

    /// Gera o próximo inteiro `u64` pseudoaleatório.
    #[inline]
    pub fn proximo_u64(&mut self) -> u64 {
        let resultado = (self.estado[1].wrapping_mul(5))
            .rotate_left(7)
            .wrapping_mul(9);

        let t = self.estado[1] << 17;

        self.estado[2] ^= self.estado[0];
        self.estado[3] ^= self.estado[1];
        self.estado[1] ^= self.estado[2];
        self.estado[0] ^= self.estado[3];

        self.estado[2] ^= t;
        self.estado[3] = self.estado[3].rotate_left(45);

        resultado
    }

    /// Gera um ponto flutuante no intervalo semiaberto `[0.0, 1.0)`.
    #[inline]
    pub fn proximo_f64(&mut self) -> f64 {
        // Usa os 53 bits superiores para preencher a mantissa de um f64 IEEE-754
        let bits = self.proximo_u64() >> 11;
        bits as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Retorna um `f64` contido no intervalo `[min, max]`.
    #[inline]
    pub fn faixa_f64(&mut self, min: f64, max: f64) -> f64 {
        if max <= min {
            return min;
        }
        min + self.proximo_f64() * (max - min)
    }

    /// Retorna um `u32` contido no intervalo inclusivo `min..=max`.
    #[inline]
    pub fn faixa_u32(&mut self, min: u32, max: u32) -> u32 {
        if max <= min {
            return min;
        }
        let amplitude = (max - min) as u64 + 1;
        let sorteado = self.proximo_u64() % amplitude;
        min + sorteado as u32
    }

    /// Avalia um teste booleano com dada probabilidade `0.0..=1.0`.
    #[inline]
    pub fn chance(&mut self, probabilidade: f64) -> bool {
        if probabilidade <= 0.0 {
            false
        } else if probabilidade >= 1.0 {
            true
        } else {
            self.proximo_f64() < probabilidade
        }
    }

    /// Sorteia o índice de um vetor com base em pesos relativos não negativos.
    pub fn escolher_indice_pesos(&mut self, pesos: &[f64]) -> Option<usize> {
        let soma: f64 = pesos.iter().copied().filter(|&p| p > 0.0).sum();
        if soma <= 0.0 || !soma.is_finite() {
            return None;
        }

        let mut r = self.proximo_f64() * soma;
        for (idx, &peso) in pesos.iter().enumerate() {
            if peso > 0.0 {
                if r <= peso {
                    return Some(idx);
                }
                r -= peso;
            }
        }
        pesos.iter().rposition(|&p| p > 0.0)
    }

    /// Embaralha uma fatia mutável in-place usando o algoritmo Fisher-Yates.
    pub fn embaralhar<T>(&mut self, slice: &mut [T]) {
        let n = slice.len();
        if n <= 1 {
            return;
        }
        for i in (1..n).rev() {
            let j = (self.proximo_u64() % ((i + 1) as u64)) as usize;
            slice.swap(i, j);
        }
    }
}
