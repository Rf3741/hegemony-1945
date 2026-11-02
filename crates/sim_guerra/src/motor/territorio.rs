//! Cache compacto de posse territorial (dono e controlador de cada região como inteiros).
//!
//! Comparar `String`s de tag por região/vizinho a cada dia custa cache misses (cada tag é uma
//! alocação separada). Como a posse só muda em conquistas e tratados, o motor mantém estes
//! índices e só os reconstrói quando o território muda.
//!
//! Convenção de ids: o país na posição `i` de `paises` tem id `i`; tags desconhecidas
//! (ex.: "NEU") recebem ids `>= paises.len()`.

use crate::entidades::{PaisMilitar, RegiaoMapa};
use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct CacheTerritorial {
    /// `false` força reconstrução no próximo uso.
    pub valido: bool,
    pub ctrl: Vec<u32>,
    pub dono: Vec<u32>,
}

impl CacheTerritorial {
    pub fn invalidar(&mut self) {
        self.valido = false;
    }

    /// Reconstrói o cache se estiver inválido ou com tamanho incompatível.
    pub fn garantir(&mut self, regioes: &[RegiaoMapa], paises: &[PaisMilitar]) {
        if self.valido && self.ctrl.len() == regioes.len() && self.dono.len() == regioes.len() {
            #[cfg(debug_assertions)]
            self.conferir(regioes, paises);
            return;
        }
        self.reconstruir(regioes, paises);
    }

    pub fn reconstruir(&mut self, regioes: &[RegiaoMapa], paises: &[PaisMilitar]) {
        let mut ids: HashMap<&str, u32> = HashMap::with_capacity(paises.len() + 8);
        for (i, p) in paises.iter().enumerate() {
            ids.insert(p.tag.as_str(), i as u32);
        }
        let mut proximo = paises.len() as u32;
        self.ctrl.clear();
        self.dono.clear();
        let mut ult_c: (&str, u32) = ("", u32::MAX);
        let mut ult_d: (&str, u32) = ("", u32::MAX);
        for reg in regioes {
            let c = internar(&reg.controlador_tag, &mut ult_c, &mut ids, &mut proximo);
            let d = internar(&reg.dono_tag, &mut ult_d, &mut ids, &mut proximo);
            self.ctrl.push(c);
            self.dono.push(d);
        }
        self.valido = true;
    }

    /// Só em builds de debug/teste: garante que o cache não ficou obsoleto.
    #[cfg(debug_assertions)]
    fn conferir(&self, regioes: &[RegiaoMapa], paises: &[PaisMilitar]) {
        let mut novo = CacheTerritorial::default();
        novo.reconstruir(regioes, paises);
        assert_eq!(self.ctrl, novo.ctrl, "cache territorial (controlador) obsoleto");
        assert_eq!(self.dono, novo.dono, "cache territorial (dono) obsoleto");
    }
}

/// Id inteiro de uma tag, reaproveitando a consulta anterior quando a tag se repete.
fn internar<'a>(
    tag: &'a str,
    ultimo: &mut (&'a str, u32),
    ids: &mut HashMap<&'a str, u32>,
    proximo: &mut u32,
) -> u32 {
    if ultimo.1 != u32::MAX && ultimo.0 == tag {
        return ultimo.1;
    }
    let id = *ids.entry(tag).or_insert_with(|| {
        let i = *proximo;
        *proximo += 1;
        i
    });
    *ultimo = (tag, id);
    id
}
