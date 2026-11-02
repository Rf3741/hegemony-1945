//! Rótulos em português (com acentos) e formatação de valores para os efeitos de eventos.
//!
//! Cobre todas as chaves de efeito usadas em `assets/data/eventos/*.ron`. Chaves que nenhum
//! subsistema processa (narrações/marcos) são exibidas sem número, pois o valor não tem efeito.

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tratamento {
    /// Pontos num medidor (aprovação, legitimidade, tensão, tesouro).
    Pontos,
    /// Fator multiplicativo (mostrado como porcentagem).
    Fator,
    /// Nenhum subsistema trata a chave (narração).
    Nenhum,
}

fn tratamento(id: &str) -> Tratamento {
    match id {
        "tensao_guerra_fria" | "legitimidade_base" | "estabilidade_politica" | "autoridade_estatal"
        | "legitimidade_internacional" | "legitimidade_soberania_popular" | "tesouro" => Tratamento::Pontos,
        "crescimento_pib" | "investimento_privado_fator" | "arrecadacao_tributaria_comercial" | "custo_construcao_estatal" => {
            Tratamento::Fator
        }
        _ => {
            if id.starts_with("aprovacao_") {
                Tratamento::Pontos
            } else if id.starts_with("forca_") || id.starts_with("radicalismo_") {
                Tratamento::Fator
            } else {
                Tratamento::Nenhum
            }
        }
    }
}

/// Nome do grupo de interesse (sufixo das chaves `aprovacao_`, `forca_`, `radicalismo_`).
fn nome_gi(sufixo: &str) -> String {
    let nome = match sufixo {
        "militares" => "dos militares",
        "militares_franceses" => "dos militares franceses",
        "intelectuais" => "dos intelectuais",
        "industriais" => "dos industriais",
        "sindicatos_operarios" => "dos sindicatos operários",
        "trabalhadores_sindicalizados" => "dos trabalhadores sindicalizados",
        "trabalhadores" => "dos trabalhadores",
        "movimento_anticolonial" => "do movimento anticolonial",
        "anticolonial_anual" => "anticolonial anual",
        "camponeses" => "dos camponeses",
        "partido_comunista" => "do partido comunista",
        "populacao_rural" => "da população rural",
        "clero" => "do clero",
        "populacao_religiosa" => "da população religiosa",
        "nomenklatura_partidaria" => "da nomenklatura partidária",
        "latifundiarios" => "dos latifundiários",
        "proprietarios_rurais" => "dos proprietários rurais",
        "pequena_burguesia" => "da pequena burguesia",
        "burocracia_estatal" => "da burocracia estatal",
        "burocratas" => "dos burocratas",
        "veteranos_guerra" => "dos veteranos de guerra",
        "administracao_colonial" => "da administração colonial",
        "grupos_discriminados" => "dos grupos discriminados",
        "defesa_nacional" => "de defesa nacional",
        "industria_belica" => "da indústria bélica",
        "geral" => "geral",
        outro => return outro.replace('_', " "),
    };
    nome.to_string()
}

/// Rótulo legível em português para uma chave de efeito (`None` para chaves com prefixo composto,
/// como `mudar_lei:` — quem exibe decide o texto).
pub fn rotulo(id: &str) -> Option<String> {
    if let Some(sufixo) = id.strip_prefix("aprovacao_") {
        return Some(format!("Aprovação {}", nome_gi(sufixo)));
    }
    if let Some(sufixo) = id.strip_prefix("forca_") {
        return Some(format!("Força {}", nome_gi(sufixo)));
    }
    if let Some(sufixo) = id.strip_prefix("radicalismo_") {
        return Some(format!("Radicalismo {}", nome_gi(sufixo)));
    }
    let rotulo = match id {
        // Medidores nacionais e do governo
        "tensao_guerra_fria" => "Tensão da Guerra Fria",
        "legitimidade_base" => "Legitimidade do governo",
        "legitimidade_internacional" => "Legitimidade internacional",
        "legitimidade_soberania_popular" => "Legitimidade de soberania popular",
        "legitimidade_internacional_onu" => "Legitimidade internacional (ONU)",
        "estabilidade_politica" => "Estabilidade política",
        "autoridade_estatal" => "Autoridade estatal",
        "capacidade_institucional_governo" => "Capacidade institucional do governo",
        "capacidade_diplomatica" => "Capacidade diplomática",
        "capacidade_intervencao_global" => "Capacidade de intervenção global",
        "capacidade_mobilizacao_global" => "Capacidade de mobilização global",
        "capacidade_destrutiva_estrategica" => "Capacidade destrutiva estratégica",
        "paralisia_burocratica_governo" => "Paralisia burocrática do governo",
        "corrupcao_administrativa_fator" => "Corrupção administrativa",
        "tempo_passagem_leis" => "Tempo de passagem de leis",
        "conformidade_ideologica_total" => "Conformidade ideológica total",
        "garantias_constitucionais_amplas" => "Garantias constitucionais amplas",
        "liberdades_locais_garantia" => "Garantia de liberdades locais",
        "taxa_alfabetizacao_anual" => "Taxa de alfabetização anual",
        "alfabetizacao_comunitaria_avanco" => "Avanço da alfabetização comunitária",
        // Economia
        "crescimento_pib" => "Crescimento do PIB",
        "inflacao" => "Inflação",
        "produtividade_industrial" => "Produtividade industrial",
        "disciplina_produtiva_estatal" => "Disciplina produtiva estatal",
        "investimento_privado_fator" => "Investimento privado",
        "arrecadacao_tributaria_comercial" => "Arrecadação tributária comercial",
        "eficiencia_tributaria_arrecadacao" => "Eficiência tributária na arrecadação",
        "custo_construcao_estatal" => "Custo de construção estatal",
        "renda_agricola_nacional" => "Renda agrícola nacional",
        "oferta_alimentos_mercado_interno" => "Oferta de alimentos no mercado interno",
        "exportacoes_americanas_expansao" => "Expansão das exportações americanas",
        "custo_manutencao_imperio" => "Custo de manutenção do império",
        "custo_manutencao_exercito" => "Custo de manutenção do exército",
        "custo_ocupacao_militar_japao" => "Custo da ocupação militar do Japão",
        "custo_operacao_contra_guerrilha" => "Custo de operação contraguerrilha",
        "extincao_custos_guerra_colonial" => "Extinção dos custos da guerra colonial",
        "orcamento_militar_expansao_maciça" => "Orçamento militar: expansão maciça",
        "fuga_de_cerebros_imigracao" => "Fuga de cérebros e imigração",
        "fuga_de_capitais_risco" => "Risco de fuga de capitais",
        "atracao_imigracao" => "Atração de imigração",
        "crise_economica_europeia" => "Crise econômica europeia",
        "colapso_industrial_alemao" => "Colapso industrial alemão",
        "colapso_militar_britanico_india" => "Colapso militar britânico na Índia",
        "recuperacao_industrial_europeia" => "Recuperação industrial europeia",
        "recuperacao_industrial_rfa" => "Recuperação industrial da RFA",
        "desenvolvimento_industrial_chines_lento" => "Desenvolvimento industrial chinês lento",
        "reparacoes_industriais_urss" => "Reparações industriais da URSS",
        "inovacao_tecnologica_fator" => "Inovação tecnológica",
        "velocidade_pesquisa" => "Velocidade de pesquisa",
        "transferencia_tecnologica_china" => "Transferência de tecnologia à China",
        "custo_vida_urbano_reducao" => "Redução do custo de vida urbano",
        "apropriacao_excedente_estatal" => "Apropriação do excedente pelo Estado",
        // Blocos e diplomacia
        "coesao_bloco_sovietico" => "Coesão do bloco soviético",
        "coesao_bloco_ocidental" => "Coesão do bloco ocidental",
        "coesao_bloco_oriental" => "Coesão do bloco oriental",
        "coesao_nacional_patriotismo" => "Coesão nacional e patriotismo",
        "polarizacao_ideologica_global" => "Polarização ideológica global",
        "hostilidade_eua_urss" => "Hostilidade EUA–URSS",
        "hostilidade_bloco_ocidental" => "Hostilidade ao bloco ocidental",
        "estabilidade_diplomatica_global" => "Estabilidade diplomática global",
        "inseguranca_aliados_europeus" => "Insegurança dos aliados europeus",
        "desmoralizacao_aliados_europeus" => "Desmoralização dos aliados europeus",
        "fraqueza_defensiva_europeia" => "Fraqueza defensiva europeia",
        "isolamento_diplomatico_pequim" => "Isolamento diplomático de Pequim",
        "distanciamento_precoce_moscou" => "Distanciamento precoce de Moscou",
        "divisao_movimento_comunista" => "Divisão do movimento comunista",
        "anarquia_internacional_fator" => "Anarquia internacional",
        "queda_prestigio_eua" => "Queda do prestígio dos EUA",
        "prestigio_americano_maximo" => "Prestígio americano máximo",
        "prestígio_onu_primeiro_sucesso" => "Prestígio da ONU: primeiro sucesso",
        "fundacao_formal_otan" => "Fundação formal da OTAN",
        "urgencia_criacao_otan" => "Urgência na criação da OTAN",
        "garantia_defesa_coletiva_art5" => "Garantia de defesa coletiva (Art. 5)",
        "alianca_militar_sino_sovietica" => "Aliança militar sino-soviética",
        "doutrina_contencao_ativa" => "Doutrina de contenção ativa",
        "monolito_sovietico_reforcado" => "Monolito soviético reforçado",
        "influencia_sovietica_oriente_medio" => "Influência soviética no Oriente Médio",
        "acesso_naval_sovietico_mediterraneo" => "Acesso naval soviético ao Mediterrâneo",
        "alinhamento_tchecoslovaco_sovietico" => "Alinhamento tchecoslovaco à URSS",
        "alinhamento_turquia_ocidente" => "Alinhamento da Turquia ao Ocidente",
        "expansao_bloco_comunista_asia" => "Expansão do bloco comunista na Ásia",
        "avanco_comunista_mediterraneo" => "Avanço comunista no Mediterrâneo",
        "autonomia_leste_europeu" => "Autonomia do Leste Europeu",
        "progresso_fundacao_onu" => "Progresso da fundação da ONU",
        // Guerra e armas nucleares
        "corrida_armamentista_nuclear" => "Corrida armamentista nuclear",
        "paridade_nuclear_estrategica" => "Paridade nuclear estratégica",
        "dissuasao_nuclear_na_europa" => "Dissuasão nuclear na Europa",
        "desilusao_desarmamento_nuclear" => "Desilusão com o desarmamento nuclear",
        "cooperacao_nuclear" => "Cooperação nuclear",
        "monopolio_nuclear_americano" => "Monopólio nuclear americano",
        "alarme_estrategico_pentagono" => "Alarme estratégico no Pentágono",
        "vulnerabilidade_estrategica_fator" => "Vulnerabilidade estratégica",
        "vulnerabilidade_militar_convencional" => "Vulnerabilidade militar convencional",
        "vulnerabilidade_militar_ocidente" => "Vulnerabilidade militar do Ocidente",
        "mobilizacao_militar_total" => "Mobilização militar total",
        "desmilitarizacao_alema" => "Desmilitarização alemã",
        "baixas_militares_extremas" => "Baixas militares extremas",
        "baixas_forcas_paz_onu" => "Baixas das forças de paz da ONU",
        "crise_militar_centro_europa" => "Crise militar na Europa Central",
        "risco_guerra_aberta_europa" => "Risco de guerra aberta na Europa",
        "risco_guerra_civil_italiana" => "Risco de guerra civil na Itália",
        "guerra_da_coreia_ativa" => "Guerra da Coreia em curso",
        "guerra_colonial_indochina" => "Guerra colonial na Indochina",
        "conflito_arabe_israelense_ativo" => "Conflito árabe-israelense ativo",
        "conflito_estreito_de_taiwan" => "Conflito no Estreito de Taiwan",
        "conflito_fronteira_caxemira" => "Conflito na fronteira da Caxemira",
        "tensao_regional_balcas" => "Tensão regional nos Bálcãs",
        "tensao_regional_oriente_medio" => "Tensão regional no Oriente Médio",
        "seguranca_bósforo_ocidental" => "Segurança ocidental do Bósforo",
        "seguranca_petroleo_oriente_medio" => "Segurança do petróleo no Oriente Médio",
        "anarquia_militar_palestina" => "Anarquia militar na Palestina",
        // Política interna, crises e movimentos
        "risco_golpe_de_estado_militar" => "Risco de golpe de Estado militar",
        "risco_golpe_comunista_grecia" => "Risco de golpe comunista na Grécia",
        "risco_revolucao_armada_popular" => "Risco de revolução armada popular",
        "conspiracao_golpista_apoio" => "Apoio à conspiração golpista",
        "greve_geral_probabilidade" => "Probabilidade de greve geral",
        "insurreicao_camponesa_revolta" => "Insurreição camponesa em revolta",
        "desobediencia_hierarquica_tropas" => "Desobediência hierárquica nas tropas",
        "agitacao_estudantil_protestos" => "Agitação estudantil e protestos",
        "agitacao_religiosa_protestos" => "Agitação religiosa e protestos",
        "paz_social_nas_fabricas" => "Paz social nas fábricas",
        "paz_social_conformidade" => "Paz social por conformidade",
        "estabilidade_rural_pacifica" => "Estabilidade rural pacífica",
        "estabilidade_urbana_ordem" => "Estabilidade urbana e ordem",
        "estabilidade_democratica_italia" => "Estabilidade democrática da Itália",
        "estabilidade_temporaria_grecia" => "Estabilidade temporária da Grécia",
        "mobilizacao_revolucionaria_maxima" => "Mobilização revolucionária máxima",
        "entusiasmo_construcao_nacional" => "Entusiasmo com a construção nacional",
        "entusiasmo_vitoria_aliada" => "Entusiasmo com a vitória aliada",
        "histeria_anticomunista_macarthismo" => "Histeria anticomunista (macarthismo)",
        "expurgos_titoistas_leste_europeu" => "Expurgos titóistas no Leste Europeu",
        // Eventos narrativos de 1945-1950
        "alemanha_dividida_fronteira_dupla" => "Alemanha dividida: fronteira dupla",
        "unificacao_comunista_coreia" => "Unificação comunista da Coreia",
        "colapso_credibilidade_eua_asia" => "Colapso da credibilidade dos EUA na Ásia",
        "fome_e_radicalismo_alemanha" => "Fome e radicalismo na Alemanha",
        "crise_refugiados_palestinos_nakba" => "Crise de refugiados palestinos (Nakba)",
        "descolonizacao_sul_asia" => "Descolonização do Sul da Ásia",
        "democratizacao_pacifica_japao" => "Democratização pacífica do Japão",
        "reforma_agraria_japonesa" => "Reforma agrária japonesa",
        "reforma_agraria_radical_china" => "Reforma agrária radical na China",
        "particao_sovietica_do_japao" => "Partição soviética do Japão",
        "independencia_vietna_pacifica" => "Independência pacífica do Vietnã",
        "debates_neutralidade_alema" => "Debates sobre a neutralidade alemã",
        "aceleracao_criacao_alemanha_ocidental" => "Aceleração da criação da Alemanha Ocidental",
        "perda_berlim_para_soviéticos" => "Perda de Berlim para os soviéticos",
        "vitoria_moral_berlim_ocidental" => "Vitória moral em Berlim Ocidental",
        "incerteza_geopolitica_bonn" => "Incerteza geopolítica em Bonn",
        "iugoslavia_socialismo_autogestionario" => "Iugoslávia: socialismo autogestionário",
        _ => return None,
    };
    Some(rotulo.to_string())
}

/// Número com sinal em estilo brasileiro ("+10", "+0,1", "-2,5").
fn pontos(v: f32) -> String {
    let arred = (v * 10.0).round() / 10.0;
    if arred == arred.trunc() { format!("{arred:+.0}") } else { format!("{arred:+.1}").replace('.', ",") }
}

/// Fator multiplicativo como porcentagem ("+15%", "-5%").
fn percentual(v: f32) -> String {
    pontos(v * 100.0) + "%"
}

/// Texto de tooltip para um modificador de efeito: rótulo legível + valor conforme o tipo.
pub fn descrever(m: &crate::Modificador) -> String {
    let id = m.id.as_str();
    let v = m.valor;
    if let Some(resto) = id.strip_prefix("mudar_lei:").or_else(|| id.strip_prefix("lei:")).or_else(|| id.strip_prefix("promulgar_lei:")) {
        let lei = resto.split_once(':').map(|(_, l)| l).unwrap_or(resto);
        return format!("Promulgar Lei: {}", lei.replace('_', " "));
    }
    if let Some(resto) = id.strip_prefix("temp:") {
        if let Some((nome, dias)) = resto.split_once(':') {
            return format!("Modificador temporário: {} {} ({} dias)", nome.replace('_', " "), pontos(v), dias);
        }
    }
    if let Some(f) = id.strip_prefix("flag:").or_else(|| id.strip_prefix("definir_flag:")) {
        return format!("Definir Flag Nacional: {}", f.replace('_', " "));
    }
    if let Some(f) = id.strip_prefix("remover_flag:") {
        return format!("Remover Flag Nacional: {}", f.replace('_', " "));
    }
    if let Some(vn) = id.strip_prefix("var:").or_else(|| id.strip_prefix("variavel:")) {
        return format!("Variável {}: {}", vn.replace('_', " "), pontos(v));
    }
    if let Some(r) = id.strip_prefix("relacao:") {
        return format!("Relações com {}: {}", r, pontos(v));
    }

    let nome = rotulo(id).unwrap_or_else(|| {
        let s = id.replace('_', " ");
        let mut c = s.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    });
    match tratamento(id) {
        Tratamento::Pontos => {
            let escala = if id == "tensao_guerra_fria" { v * 10.0 } else { v };
            format!("{nome} {}", pontos(escala))
        }
        Tratamento::Fator => format!("{nome} {}", percentual(v)),
        Tratamento::Nenhum => format!("Marco: {nome}"),
    }
}
