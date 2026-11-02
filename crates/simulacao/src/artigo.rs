//! Concordância de artigo com o nome de um país.
//!
//! Em português o artigo segue o núcleo do nome, e não a última palavra: "União das Repúblicas
//! Socialistas Soviéticas" é feminino ("da União", nunca "do União das…"), "Estados Unidos" é
//! plural ("dos"). Os dados do cenário não têm campo de gênero, então o artigo é decidido pelo
//! primeiro termo do nome, com lista de núcleos; o que não estiver na lista cai na terminação.

/// Gênero e número do núcleo de um nome de país.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genero {
    Masculino,
    Feminino,
    Plural,
}

/// Núcleos femininos.
const FEMININOS: [&str; 7] = ["República", "União", "Confederação", "Federação", "Monarquia", "Junta", "Comunidade"];
/// Núcleos masculinos.
const MASCULINOS: [&str; 9] = ["Reino", "Império", "Estado", "Emirado", "Sultanato", "Principado", "Ducado", "Grão-Ducado", "Domínio"];
/// Nomes que já são plural ("dos Emirados Árabes Unidos").
const PLURAIS: [&str; 3] = ["Estados Unidos", "Países Baixos", "Emirados Árabes"];

/// Gênero do país pelo nome.
pub fn genero(nome: &str) -> Genero {
    if PLURAIS.iter().any(|p| nome.starts_with(p)) {
        return Genero::Plural;
    }
    let primeiro = nome.split_whitespace().next().unwrap_or("").trim_end_matches(['(', ',', ':']);
    if FEMININOS.iter().any(|f| *f == primeiro) {
        Genero::Feminino
    } else if MASCULINOS.iter().any(|m| *m == primeiro) {
        Genero::Masculino
    } else if primeiro.ends_with('s') {
        Genero::Plural // "Ilhas", "Filipinas", "Antilhas"
    } else if primeiro.ends_with('a') || nome.split_whitespace().next_back().is_some_and(|u| u.ends_with('a')) {
        Genero::Feminino // "França", "Costa do Marfim", "Nova Zelândia"
    } else {
        Genero::Masculino
    }
}

/// Artigo definido: "o", "a", "os", "as" ("a França", "os Países Baixos").
pub fn artigo_definido(nome: &str) -> &'static str {
    match genero(nome) {
        Genero::Masculino => "o",
        Genero::Feminino => "a",
        Genero::Plural => "os",
    }
}

/// Preposição "de" + artigo: "do", "da", "dos", "das" ("retirada da União Soviética").
pub fn contraido_de(nome: &str) -> &'static str {
    match genero(nome) {
        Genero::Masculino => "do",
        Genero::Feminino => "da",
        Genero::Plural => "dos",
    }
}

/// Preposição "a" + artigo: "ao", "à", "aos", "às" ("entrega à França").
pub fn contraido_a(nome: &str) -> &'static str {
    match genero(nome) {
        Genero::Masculino => "ao",
        Genero::Feminino => "à",
        Genero::Plural => "aos",
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn nucleo_feminino_mesmo_terminando_em_consonante() {
        for nome in [
            "União das Repúblicas Socialistas Soviéticas",
            "União Soviética",
            "República Popular da China",
            "Confederação Suíça",
            "Federação da Rodésia",
            "Junta Militar da Grécia",
        ] {
            assert_eq!(contraido_de(nome), "da", "{nome}");
            assert_eq!(artigo_definido(nome), "a", "{nome}");
            assert_eq!(contraido_a(nome), "à", "{nome}");
        }
    }

    #[test]
    fn nucleo_masculino() {
        for nome in ["Reino Unido", "Império do Irã", "Estado Livre da Irlanda", "Principado de Mônaco", "Grão-Ducado de Luxemburgo", "Sultanato de Omã"] {
            assert_eq!(contraido_de(nome), "do", "{nome}");
            assert_eq!(contraido_a(nome), "ao", "{nome}");
        }
        assert_eq!(contraido_de("França"), "da");
        assert_eq!(contraido_de("Congo"), "do");
    }

    #[test]
    fn nomes_plurais() {
        for nome in ["Estados Unidos da América", "Países Baixos", "Emirados Árabes Unidos", "Filipinas", "Ilhas Marshall"] {
            assert_eq!(genero(nome), Genero::Plural, "{nome}");
            assert_eq!(contraido_de(nome), "dos", "{nome}");
            assert_eq!(artigo_definido(nome), "os", "{nome}");
            assert_eq!(contraido_a(nome), "aos", "{nome}");
        }
    }
}
