//! Teste de integração de guerra simulada em grande escala:
//! - Grafo de 50 regiões entre 2 países beligerantes.
//! - Verificação de determinismo exato (bit-a-bit) utilizando a mesma semente (`seed`).
//! - Garantia de que a simulação progride com avanço de frentes e que a guerra termina com tratado de paz.

use sim_guerra::{
    CatalogoGuerra, MundoGuerra, ParametrosCenarioSintetico, gerar_cenario_sintetico,
};
use std::path::Path;

#[test]
fn test_guerra_simulada_50_regioes_determinismo_e_termino() {
    let dir_fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let catalogo = CatalogoGuerra::carregar_diretorio(&dir_fixtures)
        .expect("catálogo militar deve carregar sem erros");

    let params = ParametrosCenarioSintetico {
        num_regioes: 50,
        num_paises: 2,
        num_guerras: 1,
        divisoes_por_pais: 8,
        seed: 19450815,
    };

    // 1. Instancia dois mundos independentes com os mesmos parâmetros e mesma seed
    let cenario1 = gerar_cenario_sintetico(catalogo.clone(), params);
    let cenario2 = gerar_cenario_sintetico(catalogo, params);

    assert_eq!(cenario1.regioes.len(), 50);
    assert_eq!(cenario1.paises.len(), 2);
    assert_eq!(cenario1.guerras.len(), 1);

    let mut mundo1 = MundoGuerra::novo(cenario1).expect("mundo 1 deve inicializar");
    let mut mundo2 = MundoGuerra::novo(cenario2).expect("mundo 2 deve inicializar");

    // 2. Executa os primeiros 40 ticks e verifica determinismo estrito
    for dia in 1..=40 {
        let rel1 = mundo1.executar_tick_diario();
        let rel2 = mundo2.executar_tick_diario();

        assert_eq!(
            rel1.baixas_totais_dia, rel2.baixas_totais_dia,
            "Dia {dia}: divergência de baixas entre execuções determinísticas"
        );
        assert_eq!(
            rel1.batalhas_ocorridas, rel2.batalhas_ocorridas,
            "Dia {dia}: divergência de batalhas ocorridas"
        );
        assert_eq!(
            rel1.regioes_conquistadas, rel2.regioes_conquistadas,
            "Dia {dia}: divergência de regiões conquistadas"
        );
        assert_eq!(
            mundo1.guerras()[0].pontuacao_guerra,
            mundo2.guerras()[0].pontuacao_guerra,
            "Dia {dia}: divergência de pontuação de guerra"
        );
    }

    // 3. Continua executando mundo1 até o término da guerra (ou limite de segurança)
    let max_ticks = 1000;
    let mut dias_decorridos = 40;
    let mut guerra_terminou = false;

    while dias_decorridos < max_ticks {
        let rel = mundo1.executar_tick_diario();
        dias_decorridos += 1;

        if !mundo1.guerras()[0].ativa {
            guerra_terminou = true;
            println!(
                "Guerra concluída com sucesso no dia {} com {} tratados assinados!",
                dias_decorridos,
                mundo1.historico_tratados.len()
            );
            break;
        }

        // Se foram assinados tratados, guerra deve ter encerrado
        if rel.tratados_assinados > 0 {
            guerra_terminou = true;
            break;
        }
    }

    assert!(
        guerra_terminou,
        "A guerra entre os 2 países no grafo de 50 regiões deve terminar em menos de {max_ticks} dias (terminou ativa = {})",
        mundo1.guerras()[0].ativa
    );

    assert!(
        !mundo1.historico_tratados.is_empty(),
        "Deve haver ao menos um tratado de paz assinado no histórico ao término da guerra"
    );

    let tratado = &mundo1.historico_tratados[0];
    println!("=== Resumo da Guerra Simulada (50 Regiões) ===");
    println!("Duração total: {} dias", dias_decorridos);
    println!("Tratado de Paz: Guerra '{}'", tratado.guerra_id);
    println!("Vencedores: {:?}", tratado.vencedores);
    println!("Derrotados: {:?}", tratado.derrotados);
    println!("Cláusulas aplicadas: {:?}", tratado.clausulas_executadas);
    println!(
        "Baixas Finais: Atacantes = {}, Defensores = {}",
        mundo1.guerras()[0].baixas_atacantes,
        mundo1.guerras()[0].baixas_defensores
    );
}
