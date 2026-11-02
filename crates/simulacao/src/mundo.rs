use crate::economia::{
    BemId, Mercado, OrcamentoNacional, Pop, Predio, Profissao, TipoPredio,
    calcular_penalidade_escassez,
};
use crate::erro::{ErroSimulacao, ResultadoSimulacao};
use crate::mapa::{
    AlinhamentoGeopolitico, CatalogoProvincias, DefinicaoPais1945, DonosProvincias1945,
};
use crate::politica::{CategoriaLei, EstadoPolitico, construir_politica_inicial};
use crate::tempo::DataSimulacao;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

/// Estado completo de um país, colônia ou zona de ocupação durante a simulação.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoPais {
    pub tag: String,
    pub nome: String,
    pub cor: (u8, u8, u8),
    pub capital: String,
    pub suserano: Option<String>,
    pub alinhamento_inicial: AlinhamentoGeopolitico,
    pub ideologia: String,
    pub mercado_id: String,
    pub provincias_ids: Vec<u32>,
    pub pops: Vec<Pop>,
    pub predios: Vec<Predio>,
    pub orcamento: OrcamentoNacional,
    pub politica: EstadoPolitico,
    pub pib_anual_estimado: f64,
}

impl EstadoPais {
    pub fn populacao_total(&self) -> u64 {
        self.pops.iter().map(Pop::total_habitantes).sum()
    }

    pub fn padrao_vida_medio(&self) -> f32 {
        let mut soma_pond = 0.0_f64;
        let mut total_pop = 0_u64;
        for p in &self.pops {
            let hab = p.total_habitantes();
            soma_pond += (p.padrao_vida as f64) * (hab as f64);
            total_pop = total_pop.saturating_add(hab);
        }
        if total_pop == 0 {
            10.0
        } else {
            (soma_pond / (total_pop as f64)) as f32
        }
    }

    pub fn alfabetizacao_media(&self) -> f32 {
        let mut soma_pond = 0.0_f64;
        let mut total_pop = 0_u64;
        for p in &self.pops {
            let hab = p.total_habitantes();
            soma_pond += (p.alfabetizacao as f64) * (hab as f64);
            total_pop = total_pop.saturating_add(hab);
        }
        if total_pop == 0 {
            0.25
        } else {
            (soma_pond / (total_pop as f64)) as f32
        }
    }

    pub fn eh_grande_potencia(&self) -> bool {
        matches!(self.tag.as_str(), "USA" | "SOV" | "GBR" | "FRA" | "CHI")
    }
}

/// Estado global completo do mundo da Guerra Fria (100% puro, sem Bevy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MundoJogo {
    pub data: DataSimulacao,
    pub tensao_mundial: f32,
    pub paises: BTreeMap<String, EstadoPais>,
    pub mercados: BTreeMap<String, Mercado>,
    /// Dono (soberania) por PROVÍNCIA, herdado do estado em 1945.
    pub dono_por_provincia: HashMap<u32, String>,
    /// Controlador (ocupante atual) por PROVÍNCIA; começa igual ao dono. A guerra ocupa província a província.
    #[serde(default)]
    pub controlador_por_provincia: HashMap<u32, String>,
    pub populacao_por_provincia: HashMap<u32, u64>,
    pub ticks_executados: u64,
}

impl MundoJogo {
    /// Carrega os arquivos RON gerados pelo pipeline e inicializa o mundo em `01/01/1945`.
    pub fn carregar_de_arquivos(
        caminho_provincias: impl AsRef<Path>,
        caminho_paises_1945: impl AsRef<Path>,
        caminho_donos_1945: impl AsRef<Path>,
    ) -> ResultadoSimulacao<(CatalogoProvincias, Self)> {
        let catalogo: CatalogoProvincias = ler_ron(caminho_provincias.as_ref(), "provinces.ron")?;
        let paises_def: Vec<DefinicaoPais1945> =
            ler_ron(caminho_paises_1945.as_ref(), "countries_1945.ron")?;
        let donos: DonosProvincias1945 =
            ler_ron(caminho_donos_1945.as_ref(), "province_owners_1945.ron")?;

        let mundo = Self::construir_cenario_1945(&catalogo, &paises_def, &donos)?;
        Ok((catalogo, mundo))
    }

    /// Constrói o estado inicial determinístico de 01/01/1945 a partir dos catálogos em memória.
    pub fn construir_cenario_1945(
        catalogo: &CatalogoProvincias,
        paises_def: &[DefinicaoPais1945],
        donos: &DonosProvincias1945,
    ) -> ResultadoSimulacao<Self> {
        if paises_def.is_empty() {
            return Err(ErroSimulacao::CenarioInvalido(
                "lista de países de 1945 está vazia".to_string(),
            ));
        }

        let mut provincias_por_tag: HashMap<String, Vec<u32>> = HashMap::new();
        let mut area_por_tag: HashMap<String, f64> = HashMap::new();
        let mut dono_por_provincia: HashMap<u32, String> =
            HashMap::with_capacity(catalogo.provincias.len());

        for prov in &catalogo.provincias {
            let tag = donos
                .por_id
                .get(&prov.id)
                .or_else(|| donos.por_adm1.get(&prov.adm1_code))
                .cloned()
                .unwrap_or_else(|| "ATA".to_string());
            dono_por_provincia.insert(prov.id, tag.clone());
            provincias_por_tag.entry(tag.clone()).or_default().push(prov.id);
            *area_por_tag.entry(tag).or_insert(0.0) += (prov.area.max(1.0)) as f64;
        }

        let mut mercados: BTreeMap<String, Mercado> = BTreeMap::new();
        let mut paises: BTreeMap<String, EstadoPais> = BTreeMap::new();
        let mut populacao_por_provincia: HashMap<u32, u64> =
            HashMap::with_capacity(catalogo.provincias.len());

        // Mapa rápido de áreas individuais para distribuir população entre províncias
        let area_prov_map: HashMap<u32, f64> = catalogo
            .provincias
            .iter()
            .map(|p| (p.id, (p.area.max(1.0)) as f64))
            .collect();

        for def in paises_def {
            let provs = provincias_por_tag
                .remove(&def.tag)
                .unwrap_or_default();

            // Países coloniais/domínios integram o mercado aduaneiro da metrópole
            let lider_mercado = def.suserano.as_deref().unwrap_or(&def.tag);
            let mercado_id = format!("MKT_{lider_mercado}");
            mercados.entry(mercado_id.clone()).or_insert_with(|| {
                Mercado::novo(
                    mercado_id.clone(),
                    format!("Mercado Comum ({lider_mercado})"),
                    lider_mercado.to_string(),
                )
            });

            // Distribui população nacional entre as províncias do país
            let pop_nac = def.populacao_1945;
            let area_total = area_por_tag.get(&def.tag).copied().unwrap_or(1.0).max(1.0);
            let mut acumulado = 0_u64;
            for (idx, &pid) in provs.iter().enumerate() {
                let hab = if pop_nac == 0 {
                    0
                } else if idx + 1 == provs.len() {
                    pop_nac.saturating_sub(acumulado)
                } else {
                    // Raiz quadrada da área evita concentrar tudo em desertos gigantescos (ex: Sibéria/Saara)
                    let peso_a = area_prov_map.get(&pid).copied().unwrap_or(1.0).sqrt();
                    let soma_sqrt: f64 = provs
                        .iter()
                        .map(|id| area_prov_map.get(id).copied().unwrap_or(1.0).sqrt())
                        .sum::<f64>()
                        .max(1.0);
                    let fatia = ((pop_nac as f64) * (peso_a / soma_sqrt)).round() as u64;
                    acumulado = acumulado.saturating_add(fatia);
                    fatia
                };
                populacao_por_provincia.insert(pid, hab);
                let _ = area_total;
            }

            let prov_capital_id = provs.first().copied().unwrap_or(0);
            let pib_per_capita = if pop_nac > 0 {
                ((def.pib_milhoes_usd as f64) * 1_000_000.0) / (pop_nac as f64)
            } else {
                500.0
            };

            let sol_base = (6.0 + (pib_per_capita / 450.0)).clamp(6.5, 26.0) as f32;
            let alfab_base = (0.12 + (pib_per_capita / 6500.0)).clamp(0.10, 0.96) as f32;
            let urbanizacao = (pib_per_capita / 8500.0).clamp(0.08, 0.72);

            let pops = gerar_pops_iniciais(
                prov_capital_id,
                pop_nac,
                urbanizacao,
                sol_base,
                alfab_base,
                &def.tag,
            );

            let dano_guerra = match def.tag.as_str() {
                "GER" => 0.62,
                "JAP" | "POL" => 0.48,
                "SOV" => 0.32,
                "FRA" | "ITA" | "NLD" | "BEL" | "YUG" | "GRE" | "CHI" => 0.25,
                "GBR" => 0.12,
                _ => 0.0,
            };

            let predios = gerar_predios_iniciais(pop_nac, def.pib_milhoes_usd, dano_guerra);

            let score_alinhamento = match def.tag.as_str() {
                "USA" => -100.0,
                "SOV" => 100.0,
                "GBR" | "CAN" | "AST" | "NZL" | "FRA" | "BEL" | "NLD" => -75.0,
                "POL" | "BUL" | "ROM" | "YUG" | "ALB" | "MON" | "CCP" => 70.0,
                _ => match def.alinhamento {
                    AlinhamentoGeopolitico::BlocoOcidental => -55.0,
                    AlinhamentoGeopolitico::BlocoSovietico => 65.0,
                    AlinhamentoGeopolitico::NaoAlinhado => 0.0,
                    AlinhamentoGeopolitico::OcupacaoAliada => -10.0,
                },
            };

            let politica =
                construir_politica_inicial(&def.tag, &def.leis_iniciais, score_alinhamento);

            let pib_estimado = (def.pib_milhoes_usd as f64).max(10.0) * 1_000.0;
            let orcamento = OrcamentoNacional {
                tesouro: pib_estimado * 0.04,
                limite_credito: pib_estimado * 0.50,
                fundo_investimento: pib_estimado * 0.015,
                divida_soberana: if matches!(def.tag.as_str(), "GBR" | "FRA" | "GER" | "JAP") {
                    pib_estimado * 0.18
                } else {
                    pib_estimado * 0.03
                },
                ..OrcamentoNacional::default()
            };

            paises.insert(
                def.tag.clone(),
                EstadoPais {
                    tag: def.tag.clone(),
                    nome: def.nome.clone(),
                    cor: def.cor,
                    capital: def.capital.clone(),
                    suserano: def.suserano.clone(),
                    alinhamento_inicial: def.alinhamento,
                    ideologia: def.ideologia.clone(),
                    mercado_id,
                    provincias_ids: provs,
                    pops,
                    predios,
                    orcamento,
                    politica,
                    pib_anual_estimado: pib_estimado,
                },
            );
        }

        // Alemanha e Áustria começam sob ocupação aliada: o dono segue sendo GER/AUT, mas o
        // controlador de cada província é o ocupante da sua zona (USA, GBR, FRA ou SOV).
        let mut controlador_por_provincia = dono_por_provincia.clone();
        for prov in &catalogo.provincias {
            let dono = dono_por_provincia.get(&prov.id).map(String::as_str).unwrap_or("");
            if let Some(ocupante) = zona_ocupacao_1945_com_centroid(
                &prov.adm1_code,
                dono,
                prov.centroide,
            ) {
                controlador_por_provincia.insert(prov.id, ocupante.to_string());
            }
        }
        let mut mundo = Self {
            data: DataSimulacao::INICIO_1945,
            tensao_mundial: 35.0,
            paises,
            mercados,
            controlador_por_provincia,
            dono_por_provincia,
            populacao_por_provincia,
            ticks_executados: 0,
        };
        // Executa uma rodada inicial de equilíbrio de mercado e fiscal para popular métricas no dia 1
        mundo.executar_tick_economia();
        Ok(mundo)
    }

    /// Passo diário do motor econômico: produção, ordens de mercado, preços, salários e orçamento.
    pub fn executar_tick_economia(&mut self) {
        for mercado in self.mercados.values_mut() {
            mercado.zerar_ordens();
        }

        // 1. Coleta produção dos prédios e consumo das Pops por país -> alimenta o Mercado correspondente
        let mut transferencias_suseranos: HashMap<String, f64> = HashMap::new();

        for pais in self.paises.values_mut() {
            if pais.populacao_total() == 0 {
                continue;
            }
            let Some(mercado) = self.mercados.get_mut(&pais.mercado_id) else {
                continue;
            };

            let eh_planificado =
                pais.politica.lei_ativa(CategoriaLei::Economia) == "economia_planificada";
            let mult_imposto = pais.orcamento.nivel_imposto.multiplicador_aliquota();

            let mut cap_burocratica = 25.0_f64;
            let mut folha_publica = 0.0_f64;
            let mut folha_militar = 0.0_f64;
            let mut despesa_construcao = 0.0_f64;
            let mut lucros_privados = 0.0_f64;
            let mut lucros_estatais = 0.0_f64;
            let mut valor_adicionado_bruto = 0.0_f64;

            for predio in &mut pais.predios {
                predio.recuperar_dano_paz_diario();

                // Checa escassez máxima entre insumos do prédio usando preços correntes como proxy
                let mut pen_escassez = 0.0_f64;
                for &(bem_in, _) in predio.tipo.insumos_por_nivel() {
                    let razao_preco = mercado.preco(bem_in) / bem_in.preco_base();
                    if razao_preco > 1.55 {
                        pen_escassez = pen_escassez.max((razao_preco - 1.55) * 1.8);
                    }
                }
                let mult = predio.multiplicador_efetivo(pen_escassez.min(0.70));

                let mut custo_insumos = 0.0_f64;
                for &(bem_in, qtd) in predio.tipo.insumos_por_nivel() {
                    let volume = qtd * mult;
                    mercado.registrar_demanda(bem_in, volume);
                    custo_insumos += volume * mercado.preco(bem_in);
                }

                let mut receita_saidas = 0.0_f64;
                for &(bem_out, qtd) in predio.tipo.saidas_por_nivel() {
                    let volume = qtd * mult;
                    mercado.registrar_oferta(bem_out, volume);
                    receita_saidas += volume * mercado.preco(bem_out);
                }

                let salario_por_nivel = 120.0;
                let folha = (predio.nivel as f64) * salario_por_nivel * predio.taxa_ocupacao();

                match predio.tipo {
                    TipoPredio::Administracao => {
                        cap_burocratica += 55.0 * mult;
                        folha_publica += folha + custo_insumos;
                        predio.ultimo_lucro_diario = 0.0;
                    }
                    TipoPredio::Quartel => {
                        folha_militar += folha + custo_insumos;
                        predio.ultimo_lucro_diario = 0.0;
                    }
                    TipoPredio::SetorConstrucao => {
                        despesa_construcao += (folha + custo_insumos) * 0.55;
                        predio.ultimo_lucro_diario = 0.0;
                    }
                    _ => {
                        let lucro = receita_saidas - custo_insumos - folha;
                        predio.ultimo_lucro_diario = lucro;
                        predio.reserva_caixa = (predio.reserva_caixa + lucro * 0.15).max(0.0);
                        valor_adicionado_bruto += (receita_saidas - custo_insumos).max(0.0);
                        if lucro > 0.0 {
                            if eh_planificado {
                                lucros_estatais += lucro * 0.85;
                            } else {
                                lucros_privados += lucro * 0.85;
                            }
                        }
                    }
                }
            }

            // Demanda de consumo das Pops (Lei de Engel simplificada por estrato)
            let mut massa_salarial_civil = 0.0_f64;
            let mut gasto_consumo_total = 0.0_f64;
            for pop in &mut pais.pops {
                let escala_mil = (pop.total_habitantes() as f64) / 100_000.0;
                if escala_mil <= 0.0 {
                    continue;
                }
                let renda_pop = escala_mil * 420.0 * pop.profissao.multiplicador_salarial();
                massa_salarial_civil += renda_pop;

                let qtd_graos = escala_mil * 6.5;
                let qtd_roupas = escala_mil * 2.4;
                let qtd_servicos = escala_mil * 2.0;
                mercado.registrar_demanda(BemId::Graos, qtd_graos);
                mercado.registrar_demanda(BemId::Roupas, qtd_roupas);
                mercado.registrar_demanda(BemId::Servicos, qtd_servicos);

                let mut custo_cesta = qtd_graos * mercado.preco(BemId::Graos)
                    + qtd_roupas * mercado.preco(BemId::Roupas)
                    + qtd_servicos * mercado.preco(BemId::Servicos);

                if pop.padrao_vida >= 13.0 {
                    let qtd_alim = escala_mil * 2.2;
                    let qtd_moveis = escala_mil * 1.4;
                    mercado.registrar_demanda(BemId::AlimentosProc, qtd_alim);
                    mercado.registrar_demanda(BemId::Moveis, qtd_moveis);
                    custo_cesta += qtd_alim * mercado.preco(BemId::AlimentosProc)
                        + qtd_moveis * mercado.preco(BemId::Moveis);
                }
                if pop.padrao_vida >= 19.0 {
                    let qtd_eletro = escala_mil * 0.9;
                    let qtd_auto = escala_mil * 0.4;
                    mercado.registrar_demanda(BemId::Eletrodomesticos, qtd_eletro);
                    mercado.registrar_demanda(BemId::Automoveis, qtd_auto);
                    custo_cesta += qtd_eletro * mercado.preco(BemId::Eletrodomesticos)
                        + qtd_auto * mercado.preco(BemId::Automoveis);
                }
                gasto_consumo_total += custo_cesta;

                // Ajuste lento do Padrão de Vida pela relação renda / custo da cesta
                let razao_poder_compra = (renda_pop / custo_cesta.max(1.0)).clamp(0.4, 2.5);
                let alvo_sol = (9.0 * razao_poder_compra).clamp(3.0, 45.0) as f32;
                pop.padrao_vida += 0.01 * (alvo_sol - pop.padrao_vida);
            }

            // Eficiência burocrática e arrecadação fiscal
            let custo_buroc_pais = ((pais.populacao_total() as f64) / 1_200_000.0).max(10.0);
            let efic_fisco = (cap_burocratica / custo_buroc_pais).clamp(0.25, 1.0);
            pais.orcamento.eficiencia_arrecadacao = efic_fisco;

            let (aliq_renda, aliq_cons, aliq_div) =
                match pais.politica.lei_ativa(CategoriaLei::SistemaTributario) {
                    "tributacao_regressiva" => (0.08, 0.16, 0.05),
                    "tributacao_progressiva" => (0.18, 0.06, 0.28),
                    "tributo_colonial" => (0.12, 0.15, 0.10),
                    "requisicao_estatal" => (0.14, 0.08, 0.45),
                    _ => (0.13, 0.10, 0.15),
                };

            let rec_impostos = efic_fisco
                * mult_imposto
                * (massa_salarial_civil * aliq_renda
                    + gasto_consumo_total * aliq_cons
                    + lucros_privados * aliq_div);

            let taxa_reinvestimento = match pais.politica.lei_ativa(CategoriaLei::Economia) {
                "livre_mercado" => 0.40,
                "capitalismo_de_intervencao" => 0.25,
                "economia_planificada" => 0.0,
                _ => 0.12,
            };
            pais.orcamento.fundo_investimento += lucros_privados * taxa_reinvestimento;

            // Financiamento privado alivia despesa estatal de construção
            let coberto_privado =
                (despesa_construcao * 0.60).min(pais.orcamento.fundo_investimento * 0.05);
            pais.orcamento.fundo_investimento -= coberto_privado;

            pais.orcamento.receita_impostos_diaria = rec_impostos;
            pais.orcamento.receita_estatal_diaria = lucros_estatais * efic_fisco;
            pais.orcamento.despesa_administrativa_diaria = folha_publica * 0.35;
            pais.orcamento.despesa_militar_diaria = folha_militar * 0.35;
            pais.orcamento.despesa_construcao_diaria =
                (despesa_construcao - coberto_privado).max(0.0);
            pais.orcamento.transferencias_externas_diaria = 0.0;

            // Tributo colonial (15% da receita bruta vai para a metrópole)
            if let Some(sus) = &pais.suserano {
                let tributo = (rec_impostos + lucros_estatais) * 0.15;
                pais.orcamento.transferencias_externas_diaria = -tributo;
                *transferencias_suseranos.entry(sus.clone()).or_insert(0.0) += tributo;
            }

            pais.pib_anual_estimado = (valor_adicionado_bruto * 365.0).max(1_000.0);
            pais.orcamento.limite_credito = pais.pib_anual_estimado * 0.50;
        }

        for (tag_sus, valor) in transferencias_suseranos {
            if let Some(metropole) = self.paises.get_mut(&tag_sus) {
                metropole.orcamento.transferencias_externas_diaria += valor;
            }
        }

        let grande_pot_tags: Vec<(String, bool)> = self
            .paises
            .values()
            .map(|p| (p.tag.clone(), p.eh_grande_potencia()))
            .collect();
        for (tag, gp) in grande_pot_tags {
            if let Some(p) = self.paises.get_mut(&tag) {
                p.orcamento.consolidar_caixa_diario(gp);
            }
        }

        // 2. Atualiza todos os preços de mercado por oferta e demanda
        for mercado in self.mercados.values_mut() {
            let _ = calcular_penalidade_escassez(10.0, 10.0);
            mercado.atualizar_precos_diarios();
        }
    }

    /// Passo diário do motor político e diplomático (leis, eleições, legitimidade e tensão mundial).
    pub fn executar_tick_politica(&mut self) {
        let semente_base = self.ticks_executados.wrapping_add(19450101);
        let mut soma_polarizacao = 0.0_f32;
        let mut num_paises = 0_u32;

        for (idx, pais) in self.paises.values_mut().enumerate() {
            if pais.populacao_total() == 0 {
                continue;
            }
            let semente = semente_base.wrapping_add((idx as u64) * 7919);
            pais.politica.avancar_dia(&pais.tag, semente);

            // Crescimento demográfico mensal (no dia 1 de cada mês)
            if self.data.eh_inicio_mes() {
                let bonus_saude = match pais.politica.lei_ativa(CategoriaLei::SaudeEEducacao) {
                    "sistema_publico_universal" => 0.006,
                    "sistema_publico_basico" => 0.003,
                    "escolas_religiosas" => 0.001,
                    _ => 0.0,
                };
                for pop in &mut pais.pops {
                    let taxa_mensal = pop.taxa_crescimento_anual(bonus_saude) / 12.0;
                    let novos_ativos = ((pop.ativos as f64) * taxa_mensal).round() as i64;
                    let novos_dep = ((pop.dependentes as f64) * taxa_mensal).round() as i64;
                    pop.ativos = (pop.ativos as i64 + novos_ativos).max(0) as u64;
                    pop.dependentes = (pop.dependentes as i64 + novos_dep).max(0) as u64;
                }
            }

            soma_polarizacao += pais.politica.influencia_bipolar.abs();
            num_paises += 1;
        }

        if num_paises > 0 {
            let media_pol = soma_polarizacao / (num_paises as f32);
            let alvo_tensao = (20.0 + media_pol * 0.55).clamp(10.0, 95.0);
            self.tensao_mundial += 0.005 * (alvo_tensao - self.tensao_mundial);
        }
    }

    /// Executa um tick diário completo (avança calendário, economia e política).
    pub fn executar_tick_diario(&mut self) {
        self.data.avancar_dia();
        self.ticks_executados += 1;
        self.executar_tick_economia();
        self.executar_tick_politica();
    }
}

pub(crate) fn gerar_pops_iniciais(
    provincia_id: u32,
    populacao_total: u64,
    urbanizacao: f64,
    sol_base: f32,
    alfab_base: f32,
    tag: &str,
) -> Vec<Pop> {
    if populacao_total == 0 {
        return Vec::new();
    }
    let cultura = format!("cultura_{}", tag.to_lowercase());
    let religiao = if tag == "SOV" {
        "secular_estatal".to_string()
    } else {
        "tradicional".to_string()
    };

    let distribuicao: &[(Profissao, f64, f32)] = &[
        (Profissao::Camponeses, (1.0 - urbanizacao) * 0.58, -2.5),
        (Profissao::TrabalhadoresRurais, (1.0 - urbanizacao) * 0.34, -1.5),
        (Profissao::Aristocratas, (1.0 - urbanizacao) * 0.03, 7.0),
        (Profissao::Clerigos, (1.0 - urbanizacao) * 0.05, 1.0),
        (Profissao::Operarios, urbanizacao * 0.44, 0.0),
        (Profissao::Maquinistas, urbanizacao * 0.16, 2.5),
        (Profissao::Escriturarios, urbanizacao * 0.14, 1.5),
        (Profissao::Burocratas, urbanizacao * 0.08, 3.0),
        (Profissao::Academicos, urbanizacao * 0.04, 4.5),
        (Profissao::Soldados, urbanizacao * 0.09, -0.5),
        (Profissao::Oficiais, urbanizacao * 0.02, 5.0),
        (Profissao::Capitalistas, urbanizacao * 0.03, 10.0),
    ];

    let mut pops = Vec::with_capacity(distribuicao.len());
    for &(prof, fatia, delta_sol) in distribuicao {
        let hab = ((populacao_total as f64) * fatia).round() as u64;
        if hab == 0 {
            continue;
        }
        let ativos = (hab * 2) / 5;
        let dependentes = hab.saturating_sub(ativos);
        let sol = (sol_base + delta_sol).clamp(4.0, 42.0);
        pops.push(Pop {
            provincia_id,
            profissao: prof,
            cultura: cultura.clone(),
            religiao: religiao.clone(),
            ativos,
            dependentes,
            alfabetizacao: alfab_base,
            padrao_vida: sol,
            riqueza: (sol as f64) * 25.0,
            radicais: hab / 20,
            leais: hab / 12,
        });
    }
    pops
}

pub(crate) fn gerar_predios_iniciais(pop_total: u64, pib_milhoes: u64, dano_guerra: f32) -> Vec<Predio> {
    if pop_total == 0 {
        return Vec::new();
    }
    let escala_pop = ((pop_total as f64) / 2_000_000.0).max(0.5);
    let escala_ind = ((pib_milhoes as f64) / 8_000.0).max(0.3);

    let candidatos: &[(TipoPredio, f64)] = &[
        (TipoPredio::FazendaSubsistencia, escala_pop * 2.2),
        (TipoPredio::Fazenda, escala_pop * 1.4),
        (TipoPredio::Serraria, escala_pop * 0.7),
        (TipoPredio::MinaCarvao, escala_ind * 1.1),
        (TipoPredio::MinaFerro, escala_ind * 0.9),
        (TipoPredio::MinaMetais, escala_ind * 0.5),
        (TipoPredio::CampoPetrolifero, escala_ind * 0.5),
        (TipoPredio::Siderurgica, escala_ind * 0.9),
        (TipoPredio::RefinariaQuimica, escala_ind * 0.6),
        (TipoPredio::FabricaMaquinas, escala_ind * 0.8),
        (TipoPredio::IndustriaAlimentos, escala_ind * 1.0),
        (TipoPredio::IndustriaConsumo, escala_ind * 1.1),
        (TipoPredio::IndustriaEletroAuto, escala_ind * 0.5),
        (TipoPredio::IndustriaBelica, escala_ind * 0.6),
        (TipoPredio::UsinaEletrica, escala_ind * 0.8),
        (TipoPredio::FerroviaPorto, escala_ind * 0.8),
        (TipoPredio::CentroUrbano, escala_pop * 1.0),
        (TipoPredio::Administracao, (escala_pop * 0.8).max(1.0)),
        (TipoPredio::Quartel, (escala_ind * 0.5).max(1.0)),
        (TipoPredio::SetorConstrucao, (escala_ind * 0.4).max(1.0)),
    ];

    let mut predios = Vec::new();
    for &(tipo, nivel_f) in candidatos {
        let nivel = (nivel_f.round() as u32).max(1);
        predios.push(Predio::novo(tipo, nivel, dano_guerra));
    }
    predios
}

fn ler_ron<T: serde::de::DeserializeOwned>(caminho: &Path, contexto: &str) -> ResultadoSimulacao<T> {
    let conteudo = fs::read_to_string(caminho).map_err(|fonte| ErroSimulacao::Io {
        caminho: caminho.display().to_string(),
        fonte,
    })?;
    ron::from_str(&conteudo).map_err(|fonte| ErroSimulacao::RonDe {
        contexto: contexto.to_string(),
        fonte: Box::new(fonte),
    })
}

/// Zona de ocupação aliada (tag do ocupante) de uma província da Alemanha (dono `GER`) ou da
/// Áustria (dono `AUT`) em 1945, a partir do código adm1. `None` para o resto do mundo;
/// outras ocupações históricas são aplicadas por seus eventos próprios.
///
/// Alemanha: USA (Baviera, Hesse, Bremen), GBR (Schleswig-Holstein, Hamburgo, Baixa Saxônia,
/// Renânia do Norte-Vestfália), FRA (Renânia-Palatinado, Sarre, Baden-Württemberg),
/// SOV (Saxônia, Turíngia, Brandemburgo, Mecklemburgo, Saxônia-Anhalt, Berlim, setor leste).
/// Áustria: SOV (Baixa Áustria, Burgenland, Viena), USA (Salzburgo, Alta Áustria),
/// GBR (Estíria, Caríntia), FRA (Tirol, Vorarlberg).
pub fn zona_ocupacao_1945(adm1_code: &str, dono: &str) -> Option<&'static str> {
    zona_ocupacao_1945_com_centroid(adm1_code, dono, (0.0, 0.0))
}

/// Variante usada pelo carregamento do mapa: as partilhas que seguem um paralelo precisam
/// do centroide da província, enquanto a função pública acima continua útil para consultas
/// por região.
pub fn zona_ocupacao_1945_com_centroid(
    adm1_code: &str,
    dono: &str,
    _centroide: (f32, f32),
) -> Option<&'static str> {
    // c-fix: proteção para não atribuir ocupação a países fora da lista conhecida
    let dono_permitido = matches!(
        dono,
        "GER" | "AUT"
    );
    if !dono_permitido {
        return None;
    }

    match (dono, adm1_code) {
        ("GER", "DEU-1591" | "DEU-1574" | "DEU-1575") => Some("USA"),
        ("GER", "DEU-1579" | "DEU-1578" | "DEU-1576" | "DEU-1572") => Some("GBR"),
        ("GER", "DEU-1580" | "DEU-1581" | "DEU-1573") => Some("FRA"),
        ("GER", "DEU-1601" | "DEU-1577" | "DEU-1600" | "DEU-1599" | "DEU-3487" | "DEU-3488") => {
            Some("SOV")
        }
        ("AUT", "AUT-2330" | "AUT-2322" | "AUT-2331") => Some("SOV"),
        ("AUT", "AUT-2327" | "AUT-2326") => Some("USA"),
        ("AUT", "AUT-2323" | "AUT-2325") => Some("GBR"),
        ("AUT", "AUT-2329" | "AUT-2320") => Some("FRA"),
        _ => None,
    }
}
