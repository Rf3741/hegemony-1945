//! Gerador de números pseudoaleatórios rápido, sem alocações e 100% determinístico.
//!
//! Utiliza o algoritmo Xoshiro256** inicializado via SplitMix64 para garantir
//! reprodutibilidade exata de combates, eventos e testes em qualquer plataforma.

use serde::{Deserialize, Serialize};

/// Gerador pseudoaleatório determinístico baseado em Xoshiro256**.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngDeterministico {
    seed_original: u64,
    s: [u64; 4],
}

impl RngDeterministico {
    /// Cria um novo gerador determinístico a partir de uma semente de 64 bits.
    pub fn novo(seed: u64) -> Self {
        let mut sm = seed;
        let mut splitmix = || {
            sm = sm.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        };

        let mut s = [splitmix(), splitmix(), splitmix(), splitmix()];
        if s == [0, 0, 0, 0] {
            s[0] = 1;
        }

        Self {
            seed_original: seed,
            s,
        }
    }

    /// Retorna a semente original utilizada para inicializar o gerador.
    #[inline]
    pub const fn seed(&self) -> u64 {
        self.seed_original
    }

    /// Gera o próximo inteiro pseudoaleatório de 64 bits via Xoshiro256**.
    #[inline]
    pub fn proximo_u64(&mut self) -> u64 {
        let res = (self.s[1].wrapping_mul(5)).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];

        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);

        res
    }

    /// Gera um ponto flutuante no intervalo semiaberto `[0.0, 1.0)`.
    #[inline]
    pub fn proximo_f64(&mut self) -> f64 {
        (self.proximo_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Gera um ponto flutuante dentro do intervalo `[min, max]`.
    #[inline]
    pub fn intervalo_f64(&mut self, min: f64, max: f64) -> f64 {
        if max <= min {
            return min;
        }
        min + self.proximo_f64() * (max - min)
    }

    /// Alias conveniente para `intervalo_f64`.
    #[inline]
    pub fn interval_f64(&mut self, min: f64, max: f64) -> f64 {
        self.intervalo_f64(min, max)
    }

    /// Gera um inteiro no intervalo inclusivo `[min, max]`.
    #[inline]
    pub fn intervalo_u32(&mut self, min: u32, max: u32) -> u32 {
        if max <= min {
            return min;
        }
        let range = (max - min) as u64 + 1;
        min + (self.proximo_u64() % range) as u32
    }

    /// Alias conveniente para `intervalo_u32`.
    #[inline]
    pub fn interval_u32(&mut self, min: u32, max: u32) -> u32 {
        self.intervalo_u32(min, max)
    }

    /// Avalia um teste de probabilidade (ex: `0.35` -> 35% de chance de true).
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
}
