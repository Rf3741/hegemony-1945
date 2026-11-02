//! Desocupações históricas: a notícia cita o OCUPANTE que se retira, não o dono do território.
use simulacao::formacao::Formacoes;
use simulacao::mundo::MundoJogo;
use simulacao::tempo::DataSimulacao;
use std::path::PathBuf;

#[test]
fn desocupacao_de_1946_cita_quem_se_retira() {
    let raiz = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let (catalogo, mut mundo) = MundoJogo::carregar_de_arquivos(
        raiz.join("assets/map/provinces.ron"),
        raiz.join("data/scenario_1945/countries_1945.ron"),
        raiz.join("data/scenario_1945/province_owners_1945.ron"),
    )
    .expect("cenário 1945");
    let mut form = Formacoes::carregar(raiz.join("data/formaveis"), &catalogo).expect("formáveis");

    mundo.data = DataSimulacao::nova(1946, 6, 1);
    let eventos = form.processar(&mut mundo);
    let desocupacoes: Vec<_> = eventos.iter().filter(|e| e.tipo == "desocupacao").collect();
    assert!(!desocupacoes.is_empty(), "nenhuma desocupação processada até 01/06/1946");

    for e in &desocupacoes {
        let soberano = &mundo.paises[&e.tag].nome;
        println!("{}: {}", e.nome, e.descricao);
        let retirada = e.descricao.split_once("(retirada ").expect("frase de retirada").1;
        assert!(!retirada.starts_with("de "), "{}: retirada deve citar o ocupante com artigo -> {}", e.nome, e.descricao);
        assert!(!retirada.contains(soberano.as_str()), "{}: a retirada cita o próprio soberano {soberano} -> {}", e.nome, e.descricao);
    }

    let texto = |t: &str| {
        desocupacoes
            .iter()
            .find(|e| e.nome == t)
            .unwrap_or_else(|| panic!("sem desocupação de {t}"))
            .descricao
            .clone()
    };
    assert!(texto("Manchúria").contains("retirada da União Soviética"), "{}", texto("Manchúria"));
    assert!(texto("Bornholm").contains("retirada da União Soviética"), "{}", texto("Bornholm"));
    let ira = texto("Irã");
    assert!(ira.contains("União Soviética") && ira.contains("Reino Unido"), "{ira}");
    let indochina = texto("Indochina");
    assert!(indochina.contains("China") && indochina.contains("Reino Unido"), "{indochina}");

    // Depois das retiradas, o ocupante não manda mais em nenhuma província desses países.
    for tag in ["IRN", "DEN", "CHI", "FIC"] {
        for (id, dono) in &mundo.dono_por_provincia {
            if *dono == tag {
                assert_eq!(mundo.controlador_por_provincia.get(id), Some(dono), "{tag}: província {id} segue ocupada");
            }
        }
    }
}
