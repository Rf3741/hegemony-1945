//! Testes unitários abrangentes de todas as 9 mecânicas econômicas,
//! validação de esquema RON, determinismo e casos de borda.

use sim_economia::{
    CategoriaNecessidade, CenarioEconomia, ConfigSimulacao, DataJogo, ErroEconomia, Estrato,
    FormulaPreco, ModoCadencia, MundoEconomico, calcular_fatias_substituicao, calcular_preco_alvo,
    taxa_crescimento_anual_pop, validar_id_snake_case,
};
use std::path::Path;

fn carregar_cenario_fixtures() -> CenarioEconomia {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    CenarioEconomia::carregar_diretorio(&dir).expect("fixtures RON devem carregar com sucesso")
}

#[test]
fn teste_calendario_1945_e_ano_bissexto_1948() {
    let mut data = DataJogo::default();
    assert_eq!(data.to_string(), "01/01/1945");
    assert!(!DataJogo::eh_bissexto(1945));
    assert!(DataJogo::eh_bissexto(1948));
    assert!(!DataJogo::eh_bissexto(1900));
    assert!(DataJogo::eh_bissexto(2000));

    for _ in 0..365 {
        data.avancar_dia();
    }
    assert_eq!(data, DataJogo::nova(1946, 1, 1).unwrap());

    // Avança até fevereiro de 1948 (ano bissexto)
    let mut feb_48 = DataJogo::nova(1948, 2, 28).unwrap();
    feb_48.avancar_dia();
    assert_eq!(feb_48, DataJogo::nova(1948, 2, 29).unwrap());
    feb_48.avancar_dia();
    assert_eq!(feb_48, DataJogo::nova(1948, 3, 1).unwrap());
}

#[test]
fn teste_validacao_snake_case_ascii() {
    assert!(validar_id_snake_case("Bem", "graos").is_ok());
    assert!(validar_id_snake_case("Pais", "uniao_sovietica").is_ok());
    assert!(validar_id_snake_case("Provincia", "prov_123").is_ok());

    assert!(matches!(
        validar_id_snake_case("Bem", "Grãos"),
        Err(ErroEconomia::IdentificadorInvalido { .. })
    ));
    assert!(matches!(
        validar_id_snake_case("Bem", "meu-bem"),
        Err(ErroEconomia::IdentificadorInvalido { .. })
    ));
    assert!(matches!(
        validar_id_snake_case("Bem", "_inicio"),
        Err(ErroEconomia::IdentificadorInvalido { .. })
    ));
    assert!(matches!(
        validar_id_snake_case("Bem", "fim_"),
        Err(ErroEconomia::IdentificadorInvalido { .. })
    ));
    assert!(matches!(
        validar_id_snake_case("Bem", "duplo__sub"),
        Err(ErroEconomia::IdentificadorInvalido { .. })
    ));
    assert!(matches!(
        validar_id_snake_case("Bem", "1_numero"),
        Err(ErroEconomia::IdentificadorInvalido { .. })
    ));
    assert!(matches!(
        validar_id_snake_case("Bem", ""),
        Err(ErroEconomia::IdentificadorInvalido { .. })
    ));
}

#[test]
fn teste_validacao_referencias_e_duplicatas_ron() {
    let mut cenario = carregar_cenario_fixtures();
    let clone_bem = cenario.catalogo.bens[0].clone();
    cenario.catalogo.bens.push(clone_bem);
    assert!(matches!(
        MundoEconomico::novo(cenario),
        Err(ErroEconomia::IdentificadorDuplicado {
            entidade: "Bem",
            ..
        })
    ));

    let mut cenario2 = carregar_cenario_fixtures();
    cenario2.paises[0].mercado_id = "mercado_fantasma".into();
    assert!(matches!(
        MundoEconomico::novo(cenario2),
        Err(ErroEconomia::ReferenciaInexistente {
            alvo_entidade: "Mercado",
            ..
        })
    ));

    // Bem local (eletricidade) não pode ser usado em rota comercial entre mercados
    let mut cenario3 = carregar_cenario_fixtures();
    cenario3.rotas_comerciais[0].bem_id = "eletricidade".into();
    assert!(matches!(
        MundoEconomico::novo(cenario3),
        Err(ErroEconomia::ValorInvalido {
            entidade: "RotaComercial",
            ..
        })
    ));
}

#[test]
fn teste_formula_preco_classica_e_victoria_suavizada() {
    let cfg_classica = ConfigSimulacao {
        formula_preco: FormulaPreco::Classica,
        multiplicador_preco_minimo: 0.25,
        multiplicador_preco_maximo: 1.75,
        ..Default::default()
    };

    // Oferta == Demanda == 0 não pode gerar NaN
    let p_vazio = calcular_preco_alvo(40.0, 0.0, 0.0, &cfg_classica);
    assert!((p_vazio - 40.0).abs() < 1e-9);

    // Oferta == Demanda > 0 -> preço base
    assert!((calcular_preco_alvo(40.0, 100.0, 100.0, &cfg_classica) - 40.0).abs() < 1e-9);

    // Excesso extremo de demanda -> teto 1.75x
    assert!((calcular_preco_alvo(40.0, 10.0, 200.0, &cfg_classica) - 70.0).abs() < 1e-9);

    // Excesso extremo de oferta -> piso 0.25x
    assert!((calcular_preco_alvo(40.0, 200.0, 10.0, &cfg_classica) - 10.0).abs() < 1e-9);

    // Desequilíbrio parcial: D = 150, O = 100 -> 1 + 50/150 = 1.333333...
    let p_parcial = calcular_preco_alvo(30.0, 100.0, 150.0, &cfg_classica);
    assert!((p_parcial - 40.0).abs() < 1e-6);

    // Fórmula VictoriaSuavizada mapeia [-1, +1] com alcance 0.75 e respeita volume mínimo
    let cfg_vic = ConfigSimulacao {
        formula_preco: FormulaPreco::VictoriaSuavizada,
        volume_minimo_mercado: 50.0,
        ..Default::default()
    };
    // Com volume residual minúsculo (O = 0, D = 1, V_min = 50), o preço quase não oscila
    let p_micro = calcular_preco_alvo(40.0, 0.0, 1.0, &cfg_vic);
    assert!(p_micro > 40.0 && p_micro < 41.0);
    // Com escassez real (O = 0, D = 200), atinge exatamente 1.75 * 40 = 70
    let p_escassez = calcular_preco_alvo(40.0, 0.0, 200.0, &cfg_vic);
    assert!((p_escassez - 70.0).abs() < 1e-9);
}

#[test]
fn teste_producao_conforme_metodo_e_automacao() {
    let cenario = carregar_cenario_fixtures();
    let mut mundo = MundoEconomico::novo(cenario).unwrap();

    mundo.executar_tick_diario();
    let oferta_graos_trad = mundo.oferta_atual("mercado_belga", "graos").unwrap();

    // Troca a fazenda de Antuérpia de agricultura tradicional para mecanizada
    mundo
        .alterar_metodos_predio(
            "antuerpia",
            "fazenda_graos",
            vec![
                "agricultura_mecanizada".into(),
                "propriedade_aristocratica".into(),
            ],
        )
        .unwrap();

    mundo.executar_tick_diario();
    let oferta_graos_mec = mundo.oferta_atual("mercado_belga", "graos").unwrap();
    assert!(
        oferta_graos_mec > oferta_graos_trad,
        "agricultura mecanizada deve elevar a oferta de grãos ({oferta_graos_mec} vs {oferta_graos_trad})"
    );

    // Testa automação na mina de carvão de Charleroi reduzindo vagas de operários
    let vagas_antes = mundo
        .obter_provincia("charleroi")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_carvao")
        .unwrap()
        .total_vagas();

    mundo
        .alterar_metodos_predio(
            "charleroi",
            "mina_carvao",
            vec![
                "mineracao_carvao_vapor".into(),
                "automacao_esteiras_ferrovia".into(),
                "propriedade_privada".into(),
            ],
        )
        .unwrap();

    let vagas_depois = mundo
        .obter_provincia("charleroi")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_carvao")
        .unwrap()
        .total_vagas();

    assert!(
        vagas_depois < vagas_antes,
        "automação com esteiras deve reduzir a necessidade total de mão de obra braçal"
    );
}

#[test]
fn teste_rotas_comerciais_importacao_exportacao_e_tarifas() {
    let cenario = carregar_cenario_fixtures();
    let mut mundo = MundoEconomico::novo(cenario).unwrap();

    // Bélgica não possui poços de petróleo domésticos; depende da importação americana
    mundo.avancar_dias(5);
    let oferta_petroleo_belgica = mundo.oferta_atual("mercado_belga", "petroleo").unwrap();
    assert!(
        oferta_petroleo_belgica > 5.0,
        "rota comercial EUA -> Bélgica deve injetar oferta de petróleo no mercado belga"
    );

    let belgica = mundo.obter_pais("belgica").unwrap();
    let eua = mundo.obter_pais("estados_unidos").unwrap();
    assert!(
        belgica.orcamento.receita_tarifas > 0.0 && eua.orcamento.receita_tarifas > 0.0,
        "comércio entre mercados deve gerar receita tarifária"
    );
}

#[test]
fn teste_salarios_lucros_dividendos_privados_e_estatais() {
    let cenario = carregar_cenario_fixtures();
    let mut mundo = MundoEconomico::novo(cenario).unwrap();
    mundo.avancar_dias(10);

    // Na Bélgica (capitalista/intervencionista), capitalistas e aristocratas auferem alta renda per capita
    let antuerpia = mundo.obter_provincia("antuerpia").unwrap();
    let pop_operarios = antuerpia
        .pops
        .iter()
        .find(|p| p.tipo_pop_id == "operarios")
        .unwrap();
    let pop_capitalistas = antuerpia
        .pops
        .iter()
        .find(|p| p.tipo_pop_id == "capitalistas")
        .unwrap();
    let pop_aristocratas = antuerpia
        .pops
        .iter()
        .find(|p| p.tipo_pop_id == "aristocratas")
        .unwrap();

    let renda_pc_op = pop_operarios.renda_ultimo_tick / pop_operarios.populacao;
    let renda_pc_cap = pop_capitalistas.renda_ultimo_tick / pop_capitalistas.populacao;
    let renda_pc_ari = pop_aristocratas.renda_ultimo_tick / pop_aristocratas.populacao;
    assert!(
        renda_pc_cap > renda_pc_op * 3.0,
        "capitalistas devem receber salários altos + dividendos ({renda_pc_cap} vs {renda_pc_op})"
    );
    assert!(
        renda_pc_ari > renda_pc_op * 2.0,
        "aristocratas devem receber dividendos fundiários ({renda_pc_ari} vs {renda_pc_op})"
    );

    // Na URSS (economia_planificada), dividendos das empresas estatais vão para o orçamento público
    let urss = mundo.obter_pais("uniao_sovietica").unwrap();
    assert!(
        urss.orcamento.receita_dividendos_estatais > 100.0,
        "empresas estatais soviéticas devem transferir excedente ao tesouro público"
    );
}

#[test]
fn teste_consumo_por_estrato_e_substituicao_de_bens() {
    // 1. Perfil por estrato e Lei de Engel
    let pesos_baixo = CategoriaNecessidade::pesos_orcamento(Estrato::Baixo, 9.0);
    let pesos_alto = CategoriaNecessidade::pesos_orcamento(Estrato::Alto, 30.0);
    assert!(
        pesos_baixo[CategoriaNecessidade::AlimentacaoBasica.indice()] > 0.45,
        "estrato baixo deve dedicar grande parte da renda à alimentação básica"
    );
    assert!(
        pesos_alto[CategoriaNecessidade::ArtigosLuxo.indice()]
            > pesos_baixo[CategoriaNecessidade::ArtigosLuxo.indice()] * 5.0,
        "estrato alto deve consumir proporção muito superior de bens de luxo"
    );

    // 2. Substituição de bens quando o preço relativo dos grãos dispara
    let cenario = carregar_cenario_fixtures();
    let cat = &cenario.catalogo;
    let num_bens = cat.bens.len();
    let idx_graos = cat.indice_bem("graos").unwrap();
    let idx_carne = cat.indice_bem("carne_peixe").unwrap();

    let ofertas = vec![200.0; num_bens];
    let impostos = vec![0.0; num_bens];
    let mut precos_iguais: Vec<f64> = cat.bens.iter().map(|b| b.preco_base).collect();

    let cesta_normal = calcular_fatias_substituicao(
        cat,
        CategoriaNecessidade::AlimentacaoBasica,
        &precos_iguais,
        &ofertas,
        &impostos,
        1.5,
    );
    let fatia_graos_normal = cesta_normal
        .iter()
        .find(|&&(g, _)| g == idx_graos)
        .unwrap()
        .1;
    let fatia_carne_normal = cesta_normal
        .iter()
        .find(|&&(g, _)| g == idx_carne)
        .unwrap()
        .1;

    // Grãos encarecem para 1.75x e carne barateia para 0.50x
    precos_iguais[idx_graos] *= 1.75;
    precos_iguais[idx_carne] *= 0.50;
    let cesta_choque = calcular_fatias_substituicao(
        cat,
        CategoriaNecessidade::AlimentacaoBasica,
        &precos_iguais,
        &ofertas,
        &impostos,
        1.5,
    );
    let fatia_graos_choque = cesta_choque
        .iter()
        .find(|&&(g, _)| g == idx_graos)
        .unwrap()
        .1;
    let fatia_carne_choque = cesta_choque
        .iter()
        .find(|&&(g, _)| g == idx_carne)
        .unwrap()
        .1;

    assert!(
        fatia_graos_choque < fatia_graos_normal && fatia_carne_choque > fatia_carne_normal,
        "pops devem substituir grãos caros por carne/alimentos mais baratos"
    );
}

#[test]
fn teste_padrao_de_vida_e_investimento_em_educacao() {
    let mut cenario_sem_escola = carregar_cenario_fixtures();
    let mut cenario_com_escola = carregar_cenario_fixtures();

    for p in &mut cenario_sem_escola.paises {
        if p.id == "brasil" {
            p.politica_fiscal.nivel_investimento_educacao = 0;
        }
    }
    for p in &mut cenario_com_escola.paises {
        if p.id == "brasil" {
            p.politica_fiscal.nivel_investimento_educacao = 5;
        }
    }

    let mut mundo_sem = MundoEconomico::novo(cenario_sem_escola).unwrap();
    let mut mundo_com = MundoEconomico::novo(cenario_com_escola).unwrap();

    mundo_sem.avancar_dias(250);
    mundo_com.avancar_dias(250);

    let alfa_sem = mundo_sem.alfabetizacao_media_pais("brasil").unwrap();
    let alfa_com = mundo_com.alfabetizacao_media_pais("brasil").unwrap();
    assert!(
        alfa_com > alfa_sem + 0.08,
        "investimento nível 5 em educação deve elevar significativamente a alfabetização ({alfa_com:.3} vs {alfa_sem:.3})"
    );
}

#[test]
fn teste_crescimento_demografico_e_migracao_interna() {
    // 1. Curva demográfica
    assert!(taxa_crescimento_anual_pop(2.5, 0.2) < 0.0);
    assert!(taxa_crescimento_anual_pop(15.0, 0.5) > 0.008);
    assert!(
        taxa_crescimento_anual_pop(35.0, 0.95) < taxa_crescimento_anual_pop(20.0, 0.50),
        "transição demográfica deve moderar crescimento em SoL e alfabetização muito altos"
    );

    // 2. Migração interna: cria grande disparidade entre Gante (deprimida) e Bruxelas (próspera com vagas)
    let mut cenario = carregar_cenario_fixtures();
    for prov in &mut cenario.provincias {
        if prov.id == "gante" {
            prov.predios.clear();
            prov.terra_aravel_total = 2;
            for pop in &mut prov.pops {
                pop.padrao_de_vida = 5.5;
            }
        } else if prov.id == "bruxelas" {
            for b in &mut prov.predios {
                b.nivel += 3;
            }
            for pop in &mut prov.pops {
                pop.padrao_de_vida = 22.0;
            }
        }
    }

    let mut mundo = MundoEconomico::novo(cenario).unwrap();
    let pop_bruxelas_ini = mundo.obter_provincia("bruxelas").unwrap().populacao_total();
    let pop_gante_ini = mundo.obter_provincia("gante").unwrap().populacao_total();

    mundo.avancar_dias(90);

    let pop_bruxelas_fim = mundo.obter_provincia("bruxelas").unwrap().populacao_total();
    let pop_gante_fim = mundo.obter_provincia("gante").unwrap().populacao_total();

    assert!(
        pop_bruxelas_fim - pop_bruxelas_ini > 500.0 && pop_gante_fim < pop_gante_ini,
        "migrantes desempregados devem fluir de Gante ({pop_gante_ini:.0}->{pop_gante_fim:.0}) para Bruxelas ({pop_bruxelas_ini:.0}->{pop_bruxelas_fim:.0})"
    );
}

#[test]
fn teste_contratacao_promocao_e_demissao_por_lucratividade() {
    // 1. Prédio lucrativo com 0 empregados iniciais contrata e promove camponeses alfabetizados
    let mut cenario = carregar_cenario_fixtures();
    for prov in &mut cenario.provincias {
        if prov.id == "liege" {
            for b in &mut prov.predios {
                if b.tipo_predio_id == "mina_ferro" {
                    b.empregados_por_tipo.clear();
                }
            }
        }
    }
    let mut mundo = MundoEconomico::novo(cenario).unwrap();
    let emp_ini = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_ferro")
        .unwrap()
        .total_empregados();
    assert_eq!(emp_ini, 0.0);

    mundo.avancar_dias(25);
    let emp_fim = mundo
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "mina_ferro")
        .unwrap()
        .total_empregados();
    assert!(
        emp_fim > 400.0,
        "mina lucrativa deve contratar rapidamente trabalhadores ({emp_fim})"
    );

    // 2. Prédio severamente deficitário (salário artificialmente inflado e sem caixa) demite e reduz salário
    let mut cenario_crise = carregar_cenario_fixtures();
    for prov in &mut cenario_crise.provincias {
        if prov.id == "liege" {
            for b in &mut prov.predios {
                if b.tipo_predio_id == "fabrica_armamentos" {
                    b.salario_base = 35.0; // Folha insustentável
                    b.reservas_caixa = 0.0;
                    b.subsidiado = false;
                }
            }
        }
    }
    let mut mundo_crise = MundoEconomico::novo(cenario_crise).unwrap();
    let fab_antes = mundo_crise
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "fabrica_armamentos")
        .unwrap()
        .clone();

    mundo_crise.avancar_dias(20);
    let fab_depois = mundo_crise
        .obter_provincia("liege")
        .unwrap()
        .predios
        .iter()
        .find(|b| b.tipo_predio_id == "fabrica_armamentos")
        .unwrap();

    assert!(
        fab_depois.salario_base < fab_antes.salario_base
            && fab_depois.total_empregados() < fab_antes.total_empregados(),
        "fábrica deficitária sem caixa deve reduzir salário ({:.2}->{:.2}) e demitir ({:.0}->{:.0})",
        fab_antes.salario_base,
        fab_depois.salario_base,
        fab_antes.total_empregados(),
        fab_depois.total_empregados()
    );
}

#[test]
fn teste_impostos_orcamento_divida_juros_e_promulgacao_lei() {
    let mut cenario = carregar_cenario_fixtures();
    // Força déficit inicial elevado na Bélgica aumentando gastos militares e zerando tesouro
    for p in &mut cenario.paises {
        if p.id == "belgica" {
            p.orcamento.tesouro = 0.0;
            p.orcamento.divida = 20_000.0;
            p.politica_fiscal.aliquota_imposto_renda_baixo = 0.01;
            p.politica_fiscal.aliquota_imposto_renda_medio = 0.01;
            p.politica_fiscal.aliquota_imposto_renda_alto = 0.01;
            p.politica_fiscal.aliquota_consumo_geral = 0.0;
            p.politica_fiscal.bens_tributados_consumo.clear();
            p.politica_fiscal.multiplicador_salario_governo = 1.5;
            p.politica_fiscal.multiplicador_salario_militar = 1.5;
        }
    }

    let mut mundo = MundoEconomico::novo(cenario).unwrap();
    mundo.avancar_dias(15);

    let orc_deficit = &mundo.obter_pais("belgica").unwrap().orcamento;
    assert!(
        orc_deficit.gasto_juros > 0.0,
        "dívida deve cobrar juros diários"
    );
    assert!(orc_deficit.gasto_burocracia > 0.0 && orc_deficit.gasto_militar > 0.0);
    assert!(
        orc_deficit.divida > 20_000.0,
        "déficit sem reserva no tesouro deve elevar a dívida pública"
    );

    // Reforma fiscal: promulga tributação progressiva + IVA geral e normaliza salários estatais
    mundo
        .promulgar_lei("belgica", "tributacao_progressiva")
        .unwrap();
    {
        let belgica = mundo.obter_pais_mut("belgica").unwrap();
        belgica.politica_fiscal.aliquota_consumo_geral = 0.12;
        belgica.politica_fiscal.imposto_sobre_terra_por_campones = 0.04;
        belgica.politica_fiscal.multiplicador_salario_governo = 0.85;
        belgica.politica_fiscal.multiplicador_salario_militar = 0.85;
        belgica.fila_construcao.clear();
    }
    mundo.atualizar_cache_fiscal_pais("belgica").unwrap();

    let divida_pico = mundo.obter_pais("belgica").unwrap().orcamento.divida;
    mundo.avancar_dias(25);
    let orc_superavit = &mundo.obter_pais("belgica").unwrap().orcamento;

    assert!(orc_superavit.receita_imposto_renda > 0.0);
    assert!(orc_superavit.receita_imposto_consumo > 0.0);
    assert!(orc_superavit.receita_imposto_terra > 0.0);
    assert!(
        orc_superavit.divida < divida_pico,
        "superávit primário deve amortizar a dívida soberana ({} < {})",
        orc_superavit.divida,
        divida_pico
    );
}

#[test]
fn teste_fila_construcao_conclusao_e_limites_recursos() {
    let cenario = carregar_cenario_fixtures();
    let mut mundo = MundoEconomico::novo(cenario).unwrap();

    // Em 01/01/1945 Antuérpia ainda não tem fabrica_texteis_consumo, mas ela está na fila da Bélgica (210 pts)
    assert!(
        !mundo
            .obter_provincia("antuerpia")
            .unwrap()
            .predios
            .iter()
            .any(|b| b.tipo_predio_id == "fabrica_texteis_consumo")
    );

    let rel = mundo.avancar_dias(15);
    assert!(
        rel.obras_concluidas >= 1,
        "obra na fila belga deve ser concluída"
    );
    assert!(
        mundo
            .obter_provincia("antuerpia")
            .unwrap()
            .predios
            .iter()
            .any(|b| b.tipo_predio_id == "fabrica_texteis_consumo" && b.nivel >= 1)
    );

    // Bruxelas não possui jazida de carvão em `limites_recursos`, mas construções são liberadas (com rendimento reduzido)
    let erro_mina = mundo.enfileirar_construcao("belgica", "bruxelas", "mina_carvao", false);
    assert!(erro_mina.is_ok(), "construções liberadas mesmo sem jazida mineral");
}

#[test]
fn teste_roundtrip_ron_determinismo_e_cadencia_semanal() {
    let cenario_a = carregar_cenario_fixtures();
    let cenario_b = cenario_a.clone();

    let mut mundo_a = MundoEconomico::novo(cenario_a).unwrap();
    let mut mundo_b = MundoEconomico::novo(cenario_b).unwrap();

    mundo_a.avancar_dias(60);
    mundo_b.avancar_dias(60);

    // Determinismo bit-a-bit entre duas execuções independentes
    assert_eq!(
        mundo_a.populacao_total_pais("belgica"),
        mundo_b.populacao_total_pais("belgica")
    );
    assert_eq!(
        mundo_a.preco_atual("mercado_belga", "aco"),
        mundo_b.preco_atual("mercado_belga", "aco")
    );
    assert_eq!(
        mundo_a.obter_pais("belgica").unwrap().orcamento.tesouro,
        mundo_b.obter_pais("belgica").unwrap().orcamento.tesouro
    );

    // Exportar para string RON e recarregar
    let cenario_exportado = mundo_a.exportar_cenario();
    let ron_str = cenario_exportado.para_ron_string().unwrap();
    let cenario_recarregado = CenarioEconomia::de_ron_str(&ron_str).unwrap();
    let mut mundo_recarregado = MundoEconomico::novo(cenario_recarregado).unwrap();
    mundo_recarregado.config.modo_cadencia = ModoCadencia::DiarioComAjusteSemanal;

    let rel_sem = mundo_recarregado.executar_tick_semanal();
    assert_eq!(rel_sem.dias_decorridos, 7);
    mundo_recarregado.auditar_estabilidade().unwrap();
}

#[test]
fn teste_potencial_regional_culturas_e_limites_estado() {
    let mut cenario = carregar_cenario_fixtures();
    
    // Configura o estado de São Paulo com apenas grãos permitidos (sem gado), e ferro limitado a 1 nível
    if let Some(est) = cenario.estados.iter_mut().find(|e| e.id == "estado_sao_paulo") {
        est.terra_aravel = 10;
        est.culturas_permitidas = vec!["graos".into()];
        est.limites_recursos.insert("ferro".into(), 1);
        est.limites_recursos.insert("carvao".into(), 0); // Sem carvão
        est.qualidade_recursos.insert("graos".into(), 1.25);
    }

    let mut mundo = MundoEconomico::novo(cenario).unwrap();

    // Província campinas_santos (pertence a estado_sao_paulo)
    let prov_sp = "campinas_santos";
    
    // Grãos são permitidos
    assert!(mundo.pode_construir(prov_sp, "fazenda_graos"), "grãos devem ser permitidos em SP");

    // Gado (pecuaria_pesca) é permitido construir mesmo sem aptidão climática (rendimento reduzido)
    assert!(mundo.pode_construir(prov_sp, "pecuaria_pesca"), "pecuária permitida mesmo onde cultura não consta");
    assert!(mundo.enfileirar_construcao("brasil", prov_sp, "pecuaria_pesca", false).is_ok());

    // Mina de carvão: liberada para construir mesmo sem jazida (rendimento reduzido)
    assert!(mundo.pode_construir(prov_sp, "mina_carvao"), "carvão liberado para construir mesmo com limite 0");
    assert!(mundo.enfileirar_construcao("brasil", prov_sp, "mina_carvao", false).is_ok());

    // Mina de ferro: limite de 1 nível
    assert!(mundo.pode_construir(prov_sp, "mina_ferro"), "ferro com limite 1 pode ser construído");
    mundo.enfileirar_construcao("brasil", prov_sp, "mina_ferro", false).unwrap();

    // Bônus de qualidade de recurso: grãos em SP tem qualidade 1.25 (+25% produtividade)
    let est_sp = mundo.obter_estado("estado_sao_paulo").unwrap();
    let def_graos = mundo.catalogo().tipos_predio.iter().find(|t| t.id == "fazenda_graos").unwrap();
    assert_eq!(est_sp.fator_qualidade_predio(def_graos), 1.25);
}

#[test]
fn teste_potencial_regional_cenario_real_1945_minas_sp_texas() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/data/economia");
    let cenario = CenarioEconomia::carregar_diretorio(&dir).expect("cenário real deve carregar");
    let mundo = MundoEconomico::novo(cenario).expect("mundo real deve ser válido");

    // 1. Minas Gerais deve ter ferro, ouro e café permitidos
    let est_mg = mundo.obter_estado("bra_minas_gerais").expect("Minas Gerais deve existir");
    assert!(est_mg.cultura_permitida("cafe"), "MG deve permitir café");
    assert!(est_mg.cultura_permitida("graos"), "MG deve permitir grãos");
    assert!(est_mg.limite_recurso("ferro") >= 180, "MG deve ter grande jazida de ferro");
    assert!(est_mg.limite_recurso("ouro_prata") >= 50, "MG deve ter ouro");
    assert_eq!(est_mg.limite_recurso("pesca"), 0, "MG não tem costa marítima");
    assert!(est_mg.qualidade_recursos.get("cafe").copied().unwrap_or(0.0) >= 1.20, "MG tem bônus de café");

    // 2. Centro-Oeste (Goiás) deve ter gado e grãos, e NÃO café
    let est_go = mundo.obter_estado("bra_goias").expect("Goiás deve existir");
    assert!(est_go.cultura_permitida("gado"), "Goiás deve permitir gado");
    assert!(est_go.cultura_permitida("graos"), "Goiás deve permitir grãos");
    assert!(!est_go.cultura_permitida("cafe"), "Goiás não deve permitir café");
    assert!(est_go.qualidade_recursos.get("gado").copied().unwrap_or(0.0) >= 1.20, "Goiás tem bônus de gado");

    // 3. Texas deve ter petróleo e gás
    let est_tx = mundo.obter_estado("usa_texas").expect("Texas deve existir");
    assert!(est_tx.limite_recurso("petroleo") >= 200, "Texas deve ter petróleo abundante");
    assert!(est_tx.limite_recurso("gas_natural") >= 150, "Texas deve ter gás natural");
    assert!(est_tx.qualidade_recursos.get("petroleo").copied().unwrap_or(0.0) >= 1.20, "Texas tem bônus de petróleo");

    // 4. Ruhr deve ter carvão
    let est_ruhr = mundo.obter_estado("ger_nordrhein_westfalen").expect("Ruhr deve existir");
    assert!(est_ruhr.limite_recurso("carvao") >= 300, "Ruhr deve ter muito carvão");
    assert!(est_ruhr.qualidade_recursos.get("carvao").copied().unwrap_or(0.0) >= 1.20, "Ruhr tem bônus de carvão");

    // 5. Katanga deve ter cobre e urânio
    let est_kat = mundo.obter_estado("cod_sul").expect("Katanga (cod_sul) deve existir");
    assert!(est_kat.limite_recurso("cobre") >= 150, "Katanga deve ter cobre abundante");
    assert!(est_kat.limite_recurso("uranio") >= 100, "Katanga deve ter urânio");
}

#[test]
fn teste_comercio_internacional_tarifas_acordos_e_blocos_comecon() {
    let cenario = carregar_cenario_fixtures();
    let mut mundo = MundoEconomico::novo(cenario).unwrap();

    // 1. Tarifas gerais, por bem e nível de abertura
    mundo.definir_tarifa_geral("belgica", 0.20, 0.05).unwrap();
    let bel = mundo.obter_pais("belgica").unwrap();
    assert_eq!(bel.politica_fiscal.tarifa_importacao, 0.20);
    assert_eq!(bel.politica_fiscal.tarifa_exportacao, 0.05);

    mundo.definir_tarifa_bem("belgica", "petroleo", 0.03, 0.01).unwrap();
    let bel = mundo.obter_pais("belgica").unwrap();
    assert_eq!(bel.politica_fiscal.tarifa_importacao_bem("petroleo"), 0.03);
    assert_eq!(bel.politica_fiscal.tarifa_importacao_bem("graos"), 0.20);

    mundo.definir_nivel_abertura("belgica", 0.85).unwrap();
    let bel = mundo.obter_pais("belgica").unwrap();
    assert!(bel.politica_fiscal.nivel_abertura() >= 0.80);

    // 2. Acordos comerciais bilaterais
    assert!(!mundo.tem_acordo_comercial("belgica", "estados_unidos"));
    mundo.assinar_acordo_comercial("belgica", "estados_unidos").unwrap();
    assert!(mundo.tem_acordo_comercial("belgica", "estados_unidos"));
    assert!(mundo.tem_acordo_comercial("estados_unidos", "belgica"));

    // 3. Controle e ciclo de vida de rotas comerciais
    let rota_id = mundo
        .criar_rota_comercial("belgica", "brasil", "graos", false, 50.0)
        .unwrap();
    assert!(mundo.rotas_comerciais().iter().any(|r| r.id == rota_id));

    let auto = mundo.alternar_rota_automatica(&rota_id).unwrap();
    assert!(!auto);

    mundo.ajustar_volume_rota(&rota_id, 75.0).unwrap();
    let rota = mundo.rotas_comerciais().iter().find(|r| r.id == rota_id).unwrap();
    assert_eq!(rota.volume_maximo, 75.0);

    mundo.remover_rota_comercial(&rota_id).unwrap();
    assert!(!mundo.rotas_comerciais().iter().any(|r| r.id == rota_id));

    // 4. Blocos e restrições: URSS fechada até COMECON
    let bloco_sov = mundo.bloco_do_pais("uniao_sovietica").unwrap();
    assert!(bloco_sov.fechado, "Mercado soviético começa fechado");
    assert!(!bloco_sov.comecon_ativo, "COMECON começa inativo em 1945");

    // Tentativa de rota externa sem acordo com país do bloco fechado deve falhar
    let tentativa_sem_acordo = mundo.criar_rota_comercial("belgica", "uniao_sovietica", "ferramentas", true, 40.0);
    assert!(
        tentativa_sem_acordo.is_err(),
        "Mercado fechado soviético deve impedir rota com o ocidente sem acordo bilateral"
    );

    // Com acordo comercial, rota é permitida
    mundo.assinar_acordo_comercial("belgica", "uniao_sovietica").unwrap();
    let rota_com_acordo = mundo.criar_rota_comercial("belgica", "uniao_sovietica", "ferramentas", true, 40.0);
    assert!(rota_com_acordo.is_ok(), "Acordo comercial deve autorizar rota");

    // 5. Ativação do COMECON
    let comecon = mundo.alternar_comecon().unwrap();
    assert!(comecon, "COMECON deve ser ativado");
    let bloco_pos = mundo.bloco_do_pais("uniao_sovietica").unwrap();
    assert!(bloco_pos.comecon_ativo, "COMECON ativo deve constar no bloco");

    // Resumo comercial do país
    let resumo = mundo.resumo_comercio_pais("belgica");
    assert!(resumo.acordos_comerciais.contains(&"estados_unidos".to_string()));
    assert!(resumo.acordos_comerciais.contains(&"uniao_sovietica".to_string()));
}
