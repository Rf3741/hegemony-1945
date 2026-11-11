//! Gerador procedural de cenários sintéticos determinísticos para testes e benchmarks.

use crate::catalogo::CatalogoGuerra;
use crate::cenario::CenarioGuerra;
use crate::entidades::{
    Comandante, ConfigGuerra, Divisao, Exercito, Guerra, PaisMilitar, RegiaoMapa,
};
use crate::grafo::ArestaRegiao;
use crate::rng::RngDeterministico;
use crate::tipos::{DataJogo, PosturaOperacional};

/// Parâmetros para geração procedural de cenários sintéticos.
#[derive(Debug, Clone, Copy)]
pub struct ParametrosCenarioSintetico {
    pub num_regioes: usize,
    pub num_paises: usize,
    pub num_guerras: usize,
    pub divisoes_por_pais: usize,
    pub seed: u64,
}

impl Default for ParametrosCenarioSintetico {
    fn default() -> Self {
        Self {
            num_regioes: 50,
            num_paises: 2,
            num_guerras: 1,
            divisoes_por_pais: 6,
            seed: 42,
        }
    }
}

/// Gera um cenário de teste ou benchmark procedural e 100% determinístico.
pub fn gerar_cenario_sintetico(
    catalogo: CatalogoGuerra,
    params: ParametrosCenarioSintetico,
) -> CenarioGuerra {
    let mut rng = RngDeterministico::novo(params.seed);
    let n_regioes = params.num_regioes.max(4);
    let n_paises = params.num_paises.max(2).min(n_regioes / 2);

    // Gera lista de tags de 3 letras consistentes
    let mut tags_paises = Vec::with_capacity(n_paises);
    let tags_predefinidas = [
        "USA", "SOV", "GER", "GBR", "FRA", "JAP", "CHI", "PRK", "KOR", "VNM", "IND", "PAK", "BRA",
        "ARG", "ITA", "EGY", "ISR", "TUR", "POL", "CUB", "CAN", "AST", "SAF", "YUG",
    ];

    for i in 0..n_paises {
        if i < tags_predefinidas.len() {
            tags_paises.push(tags_predefinidas[i].to_string());
        } else {
            // Gera tag determinística tipo 'P01', 'P02', ...
            let k = i - tags_predefinidas.len();
            let tag: String = [k / 676 % 26, k / 26 % 26, k % 26]
                .iter()
                .map(|&c| (b'A' + c as u8) as char)
                .collect::<String>()
                .replacen('A', "X", 1);
            tags_paises.push(tag);
        }
    }

    // Topologia em grade regular aproximada (W x H) para conectar regiões realisticamente
    let largura = (n_regioes as f64).sqrt().ceil() as usize;
    let altura = n_regioes.div_ceil(largura);

    let mut regioes = Vec::with_capacity(n_regioes);
    let mut arestas = Vec::new();

    let tipos_terreno = if catalogo.terrenos.is_empty() {
        vec!["planicie".to_string()]
    } else {
        catalogo.terrenos.iter().map(|t| t.id.clone()).collect()
    };

    // Associa cada célula (x, y) a um país contiguamente
    for idx in 0..n_regioes {
        let x = idx % largura;
        let y = idx / largura;
        let _ = y; // y usado para layout

        let pais_idx = (x * n_paises) / largura;
        let dono = &tags_paises[pais_idx.min(n_paises - 1)];

        let terreno_id =
            &tipos_terreno[rng.interval_u32(0, (tipos_terreno.len() - 1) as u32) as usize];
        let eh_costeira = x == 0 || x == largura - 1 || y == 0 || y == altura - 1;
        let nivel_porto = if eh_costeira && rng.chance(0.40) {
            rng.interval_u32(10, 50)
        } else {
            0
        };

        let adm1_code = format!("{}-{:04}", dono, idx + 1);
        let nome_pt = format!("Região {:04}", idx + 1);

        regioes.push(RegiaoMapa {
            adm1_code,
            nome_pt,
            dono_tag: dono.clone(),
            controlador_tag: dono.clone(),
            terreno_id: terreno_id.clone(),
            infraestrutura: rng.interval_u32(20, 90),
            nivel_porto,
            nivel_fortificacao: if rng.chance(0.15) {
                rng.interval_u32(1, 3) as u8
            } else {
                0
            },
            pontos_vitoria: 1.0,
            populacao: rng.interval_u32(50_000, 1_500_000) as u64,
            destruicao_guerra: 0.0,
            resistencia_ocupacao: 0.0,
        });

        // Conecta com vizinho à esquerda
        if x > 0 {
            let esq_idx = idx - 1;
            arestas.push(ArestaRegiao {
                de: regioes[idx].adm1_code.clone(),
                para: regioes[esq_idx].adm1_code.clone(),
                distancia_km: rng.interval_f64(80.0, 150.0),
                travessia_rio: rng.chance(0.20),
                estreito_maritimo: false,
            });
        }

        // Conecta com vizinho acima
        if y > 0 {
            let cima_idx = idx - largura;
            if cima_idx < regioes.len() {
                arestas.push(ArestaRegiao {
                    de: regioes[idx].adm1_code.clone(),
                    para: regioes[cima_idx].adm1_code.clone(),
                    distancia_km: rng.interval_f64(80.0, 150.0),
                    travessia_rio: rng.chance(0.15),
                    estreito_maritimo: false,
                });
            }
        }

        // Conecta diagonal ocasional para grafos planares orgânicos
        if x > 0 && y > 0 && rng.chance(0.35) {
            let diag_idx = idx - largura - 1;
            if diag_idx < regioes.len() {
                arestas.push(ArestaRegiao {
                    de: regioes[idx].adm1_code.clone(),
                    para: regioes[diag_idx].adm1_code.clone(),
                    distancia_km: rng.interval_f64(110.0, 190.0),
                    travessia_rio: false,
                    estreito_maritimo: false,
                });
            }
        }
    }

    // Seleciona capitais para cada país e eleva seus pontos de vitória
    let mut capitais = Vec::with_capacity(n_paises);
    for tag in &tags_paises {
        let mut reg_pais: Vec<usize> = regioes
            .iter()
            .enumerate()
            .filter(|(_, r)| &r.dono_tag == tag)
            .map(|(i, _)| i)
            .collect();

        if reg_pais.is_empty() {
            reg_pais.push(0);
        }

        // Capital fica no centro do território do país
        let cap_idx = reg_pais[reg_pais.len() / 2];
        regioes[cap_idx].pontos_vitoria = 30.0;
        regioes[cap_idx].infraestrutura = 95;
        capitais.push(regioes[cap_idx].adm1_code.clone());
    }

    // Cria países militares
    let mut paises = Vec::with_capacity(n_paises);
    for (i, tag) in tags_paises.iter().enumerate() {
        paises.push(PaisMilitar {
            tag: tag.clone(),
            nome_pt: format!("País {tag}"),
            capital_adm1: capitais[i].clone(),
            doutrina_id: "padrao".to_string(),
            tecnologias_ativas: catalogo.tecnologias.iter().map(|t| t.id.clone()).collect(),
            cansaco_guerra: 0.0,
            baixas_totais: 0,
            populacao_mobilizavel: 1_000_000,
            estoque_suprimento: 1.0,
            eficiencia_naval: 1.0,
            data_criacao: None,
        });
    }

    // Template padrão para divisões
    let template_id = if let Some(tmpl) = catalogo.templates.first() {
        tmpl.id.clone()
    } else {
        "divisao_infantaria_padrao".to_string()
    };

    let mut divisoes = Vec::new();
    let mut exercitos = Vec::new();
    let mut comandantes = Vec::new();

    let mut div_id_counter = 1usize;

    for (p_idx, tag) in tags_paises.iter().enumerate() {
        let com_id = format!("comandante_{}", tag.to_lowercase());
        comandantes.push(Comandante {
            id: com_id.clone(),
            nome_pt: format!("General {tag}"),
            pais_tag: tag.clone(),
            habilidade_ataque: rng.interval_u32(3, 7) as u8,
            habilidade_defesa: rng.interval_u32(3, 7) as u8,
            habilidade_logistica: rng.interval_u32(3, 7) as u8,
            habilidade_planejamento: rng.interval_u32(3, 7) as u8,
            experiencia: 50.0,
            tracos: Vec::new(),
        });

        let mut reg_pais: Vec<String> = regioes
            .iter()
            .filter(|r| &r.dono_tag == tag)
            .map(|r| r.adm1_code.clone())
            .collect();

        if reg_pais.is_empty() {
            reg_pais.push(capitais[p_idx].clone());
        }

        let mut divisoes_exercito = Vec::new();
        let num_divs = params.divisoes_por_pais.max(1);

        for d in 0..num_divs {
            let reg_alocada = &reg_pais[d % reg_pais.len()];
            let d_id = format!("div_{}_{}", tag.to_lowercase(), div_id_counter);
            div_id_counter += 1;

            divisoes.push(Divisao {
                id: d_id.clone(),
                nome_pt: format!("{}ª Divisão {}", d + 1, tag),
                pais_tag: tag.clone(),
                template_id: template_id.clone(),
                regiao_adm1: reg_alocada.clone(),
                efetivos_atuais: 10_000,
                equipamento_fracao: 1.0,
                organizacao: 45.0,
                moral: 0.85,
                entrincheiramento: 5.0,
                suprimento_atual: 1.0,
                experiencia: 0.20,
            });

            divisoes_exercito.push(d_id);
        }

        exercitos.push(Exercito {
            id: format!("exercito_{}", tag.to_lowercase()),
            nome_pt: format!("1º Exército de {tag}"),
            pais_tag: tag.clone(),
            comandante_id: Some(com_id),
            divisoes_ids: divisoes_exercito,
            postura: PosturaOperacional::OfensivaEquilibrada,
            frente_id: None,
            regiao_objetivo_adm1: None,
        });
    }

    // Configura guerras solicitadas entre os primeiros países
    let mut guerras = Vec::new();
    let max_guerras = params.num_guerras.min(n_paises / 2);

    for g in 0..max_guerras {
        let atacante = &tags_paises[g * 2];
        let defensor = &tags_paises[g * 2 + 1];

        guerras.push(Guerra {
            id: format!(
                "guerra_{}_{}",
                atacante.to_lowercase(),
                defensor.to_lowercase()
            ),
            nome_pt: format!("Guerra de {atacante} contra {defensor}"),
            atacantes: vec![atacante.clone()],
            defensores: vec![defensor.clone()],
            data_inicio: DataJogo::default(),
            pontuacao_guerra: 0.0,
            baixas_atacantes: 0,
            baixas_defensores: 0,
            batalhas_vencidas_atacantes: 0,
            batalhas_vencidas_defensores: 0,
            capitulou_atacantes: false,
            capitulou_defensores: false,
            ativa: true,
            envolvimentos: Vec::new(),
        });
    }

    CenarioGuerra {
        data_inicial: DataJogo::default(),
        seed: params.seed,
        config: ConfigGuerra::default(),
        catalogo,
        regioes,
        arestas,
        paises,
        comandantes,
        divisoes,
        exercitos,
        guerras,
    }
}
