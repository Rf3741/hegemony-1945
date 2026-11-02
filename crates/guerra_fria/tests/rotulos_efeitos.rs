//! Todo efeito escrito nos eventos RON precisa de um rótulo legível (acentos + formatação).
use eventos::Modificador;
use eventos::rotulos;

const MARCADOR: &str = "Modificador(id: \"";

/// IDs de efeito usados em `assets/data/eventos/*.ron` (cada `Modificador(id: "...")`).
fn chaves_dos_eventos() -> Vec<String> {
    let pasta = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/eventos");
    let mut chaves = Vec::new();
    let arquivos = std::fs::read_dir(&pasta)
        .unwrap_or_else(|e| panic!("{}: {e}", pasta.display()))
        .map(|e| e.unwrap().path());
    for caminho in arquivos.filter(|p| p.extension().is_some_and(|e| e == "ron")) {
        let texto = std::fs::read_to_string(&caminho).unwrap();
        let mut desde = 0usize;
        while let Some((chave, seguinte)) = proxima_chave(&texto, desde) {
            if !chaves.iter().any(|x: &String| x == &chave) {
                chaves.push(chave);
            }
            desde = seguinte;
        }
    }
    chaves.sort();
    chaves
}

/// Próxima `Modificador(id: "...")` a partir de `desde`: a chave e o índice seguinte.
fn proxima_chave(texto: &str, desde: usize) -> Option<(String, usize)> {
    let inicio = texto[desde..].find(MARCADOR)? + desde + MARCADOR.len();
    let fim = texto[inicio..].find('"')? + inicio;
    Some((texto[inicio..fim].to_string(), fim))
}

fn modificador(id: &str, valor: f32) -> Modificador {
    Modificador { id: id.to_string(), valor }
}

#[test]
fn toda_chave_de_evento_tem_rotulo() {
    let chaves = chaves_dos_eventos();
    assert!(chaves.len() > 100, "poucas chaves encontradas: {}", chaves.len());
    let sem_rotulo: Vec<&String> = chaves.iter().filter(|c| rotulos::rotulo(c).is_none()).collect();
    assert!(sem_rotulo.is_empty(), "chaves sem rótulo em português: {sem_rotulo:?}");
}

#[test]
fn formata_valor_por_tipo_de_efeito() {
    assert_eq!(
        eventos::descrever_efeito(&modificador("forca_defesa_nacional", -0.05)),
        "Força de defesa nacional -5%"
    );
    assert_eq!(
        eventos::descrever_efeito(&modificador("aprovacao_militares", 10.0)),
        "Aprovação dos militares +10"
    );
    assert_eq!(
        eventos::descrever_efeito(&modificador("autoridade_estatal", 0.10)),
        "Autoridade estatal +0,1"
    );
    assert_eq!(
        eventos::descrever_efeito(&modificador("crescimento_pib", 0.015)),
        "Crescimento do PIB +1,5%"
    );
    assert_eq!(
        eventos::descrever_efeito(&modificador("tensao_guerra_fria", 0.35)),
        "Tensão da Guerra Fria +3,5"
    );
}

#[test]
fn efeitos_de_marco_nao_mostram_numero() {
    let texto = eventos::descrever_efeito(&modificador("definir_flag:afeganistao_neutral", 1.0));
    assert_eq!(texto, "Definir Flag Nacional: afeganistao neutral");
    let marco = eventos::descrever_efeito(&modificador("colapso_militar_britanico_india", 1.0));
    assert!(marco.starts_with("Marco:"), "{marco}");
    assert!(!marco.contains('_'), "{marco}");
}
